// Marine IPC commands — Phase 2 Marine MVP + Sprint 4 dredge audit.
//
// Exposes CUBE surface generation, TPU, S-44 compliance, SVP parsing,
// and the Sprint 4 dredge pay-volume audit to the frontend.

use crate::commands::mining::{derive_cell_meters, read_dem_grid};
use crate::formats::read_geotiff_header;
use crate::marine::cross_section::{
    compute_cross_sections, CrossSectionReport, CrossSectionRequest,
};
use crate::marine::dredge::{compute_dredge_volumes, DredgeVolumeResult};
use crate::marine::svp::{parse_svp, SvpProfile};
use crate::marine::{
    check_s44_compliance, compute_tpu, generate_cube_surface, write_s57, CubeParams, S44CheckInput,
    S44Order, S57Feature, Sounding, SoundingTpuInput,
};
use serde::Deserialize;
use std::path::PathBuf;

/// Generate a CUBE surface from a batch of soundings.
#[tauri::command]
pub async fn generate_cube_surface_cmd(
    soundings: Vec<Sounding>,
    params: CubeParams,
) -> Result<crate::marine::CubeSurface, String> {
    // Sprint 21: save recovery snapshot before long operation
    let _snapshot = crate::recovery::save_recovery_snapshot(
        &format!("{{\"operation\":\"generate_cube_surface\",\"soundings\":{}}}", soundings.len()),
        "generate_cube_surface",
    );

    let result = generate_cube_surface(&soundings, &params)
        .map_err(|e| ctx_no_input!("generating CUBE surface", e))?;

    if let Ok(ref snap) = _snapshot {
        crate::recovery::clear_recovery_snapshot(snap);
    }
    Ok(result)
}

/// Compute TPU for a batch of soundings.
#[tauri::command]
pub async fn compute_tpu_batch(
    soundings: Vec<SoundingTpuInput>,
) -> Result<Vec<crate::marine::TpuResult>, String> {
    soundings
        .iter()
        .map(compute_tpu)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ctx_no_input!("computing TPU batch", e))
}

/// Check S-44 compliance for a batch of soundings.
#[derive(Debug, Deserialize)]
pub struct S44CheckRequest {
    pub soundings: Vec<S44CheckInput>,
    #[serde(rename = "targetOrder")]
    pub target_order: S44Order,
}

#[tauri::command]
pub async fn check_s44_compliance_cmd(
    request: S44CheckRequest,
) -> Result<crate::marine::S44ComplianceResult, String> {
    check_s44_compliance(&request.soundings, request.target_order)
        .map_err(|e| ctx_no_input!("checking S-44 compliance", e))
}

/// Export features to an S-57 .000 file.
#[tauri::command]
pub fn export_s57(features: Vec<S57Feature>, path: String) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(&path);
    write_s57(&path_buf, &features).map_err(|e| ctx!("exporting S-57 .000 file", path, e))
}

/// Parse an SVP (Sound Velocity Profile) file.
#[tauri::command]
pub fn parse_svp_cmd(path: String) -> Result<SvpProfile, String> {
    let path_buf = std::path::PathBuf::from(&path);
    parse_svp(&path_buf).map_err(|e| ctx!("parsing SVP file", path, e))
}

// ──────────────────────────────────────────────────────────────────
// Sprint 4 — Dredge pay-volume audit (Revenue Feature #2)

/// Request payload for the dredge pay-volume audit.
///
/// `design_path` accepts either a GeoTIFF path (for variable-depth
/// design templates) or `flat:Z` syntax (e.g., `flat:15.0`) for a
/// constant design depth across the whole channel/berth.
#[derive(Debug, Deserialize)]
pub struct DredgeAuditRequest {
    /// Path to post-dredge survey GeoTIFF (seabed as surveyed after dredging)
    #[serde(rename = "postPath")]
    pub post_path: String,
    /// Path to pre-dredge survey GeoTIFF (baseline seabed before dredging)
    #[serde(rename = "prePath")]
    pub pre_path: String,
    /// Either a GeoTIFF path or "flat:Z" for a constant design depth (m)
    #[serde(rename = "designPath")]
    pub design_path: String,
    /// Allowable overdredge tolerance (m). Material removed below
    /// design + tolerance is unpaid "excessive overdredge".
    #[serde(rename = "toleranceM")]
    pub tolerance_m: f64,
}

/// Compute the four-bucket dredge pay-volume breakdown.
///
/// All three grids are read from GeoTIFF and resampled to the post-dredge
/// grid's dimensions (Phase 1 simplification — assumes surveys were
/// captured on the same grid; future Phase 2+ will add proper resampling).
#[tauri::command]
pub async fn compute_dredge_audit_cmd(
    request: DredgeAuditRequest,
) -> Result<DredgeVolumeResult, String> {
    // Sprint 21: save recovery snapshot before long operation
    let _snapshot = crate::recovery::save_recovery_snapshot(
        &format!("{{\"operation\":\"dredge_audit\",\"post_path\":\"{}\"}}", request.post_path),
        "dredge_audit",
    );

    // Read post-dredge grid (the reference grid)
    let post_path = crate::path_validation::validate_path(&request.post_path)
        .map_err(|e| ctx!("validating post-dredge path", request.post_path, e))?;
    let post_header = read_geotiff_header(&post_path)
        .map_err(|e| ctx!("reading post-dredge DEM header", request.post_path, e))?;
    let post_grid = read_dem_grid(&post_path, &post_header)
        .map_err(|e| ctx!("reading post-dredge DEM grid", request.post_path, e))?;
    let (cell_w_m, cell_h_m) = derive_cell_meters(&post_header);

    // Read pre-dredge grid
    let pre_path = crate::path_validation::validate_path(&request.pre_path)
        .map_err(|e| ctx!("validating pre-dredge path", request.pre_path, e))?;
    let pre_header = read_geotiff_header(&pre_path)
        .map_err(|e| ctx!("reading pre-dredge DEM header", request.pre_path, e))?;
    let pre_grid = read_dem_grid(&pre_path, &pre_header)
        .map_err(|e| ctx!("reading pre-dredge DEM grid", request.pre_path, e))?;

    // Design grid: either GeoTIFF or flat:Z
    let (design_grid, _design_header) = if request.design_path.starts_with("flat:") {
        let z: f64 = request
            .design_path
            .strip_prefix("flat:")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("invalid flat:Z design: '{}'", request.design_path))?;
        (vec![z; post_grid.len()], post_header.clone())
    } else {
        let d_path = PathBuf::from(&request.design_path);
        let header = read_geotiff_header(&d_path)
            .map_err(|e| ctx!("reading design DEM header", request.design_path, e))?;
        let grid = read_dem_grid(&d_path, &header)
            .map_err(|e| ctx!("reading design DEM grid", request.design_path, e))?;
        (grid, header)
    };

    compute_dredge_volumes(
        &post_grid,
        &pre_grid,
        &design_grid,
        cell_w_m,
        cell_h_m,
        request.tolerance_m,
    )
    .map_err(|e| ctx_no_input!("computing dredge pay-volume breakdown", e))
}

// ──────────────────────────────────────────────────────────────────
// Sprint 5 — Cross-section profiler (Revenue Feature #8)

/// Compute cross-sections perpendicular to a centerline.
///
/// All coordinates must be in a projected CRS (meters). The frontend is
/// responsible for converting from geographic (lon/lat) to projected before
/// invoking this command — use the `transform_coords` IPC.
#[tauri::command]
pub async fn compute_cross_sections_cmd(
    request: CrossSectionRequest,
) -> Result<CrossSectionReport, String> {
    compute_cross_sections(&request).map_err(|e| ctx_no_input!("computing cross-sections", e))
}

// ──────────────────────────────────────────────────────────────────
// Phase 2 — MBES datagram parsing, tidal datums, backscatter, QC
// ──────────────────────────────────────────────────────────────────

use crate::formats::kongsberg_all::{read_all_survey, AllSurveyData, WaterColumnSummary, extract_water_column_summary};

/// Read a Kongsberg .all file and extract all bathymetry, position, and
/// attitude data. Returns soundings with depth, across/along-track,
/// beam angle, quality, and interpolated attitude (roll/pitch/heave/heading).
///
/// `max_pings=0` means read all pings.
#[tauri::command]
pub async fn read_all_survey_cmd(
    path: String,
    max_pings: u32,
) -> Result<AllSurveyData, String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating path for MBES survey", path, e))?;
    let label = path.clone();
    tokio::task::spawn_blocking(move || {
        read_all_survey(&path_buf, max_pings)
            .map_err(|e| ctx!("reading Kongsberg .all survey", label, e))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

/// Extract water-column datagram summary statistics from a Kongsberg .all
/// file. Returns counts only — raw amplitude samples can be tens of
/// millions and are not shipped over IPC. Use this to populate the
/// MBES Survey Reader's "Water Column" tab.
#[tauri::command]
pub async fn extract_water_column_summary_cmd(
    path: String,
    max_pings: u32,
) -> Result<WaterColumnSummary, String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating path for water column extraction", path, e))?;
    let label = path.clone();
    tokio::task::spawn_blocking(move || {
        extract_water_column_summary(&path_buf, max_pings)
            .map_err(|e| ctx!("extracting water column summary", label, e))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

/// Apply a tidal datum conversion to an array of depths.
#[tauri::command]
pub fn convert_tidal_datum_cmd(
    depths: Vec<f64>,
    offset_m: f64,
) -> Vec<f64> {
    use crate::marine::tidal_datums::TidalDatumConversion;
    let conversion = TidalDatumConversion {
        from: crate::marine::tidal_datums::TidalDatum::Mllw,
        to: crate::marine::tidal_datums::TidalDatum::Cd,
        offset_m,
        source: "user-specified".to_string(),
    };
    crate::marine::tidal_datums::convert_depths(&depths, &conversion)
}

/// Create a backscatter mosaic from MBES backscatter samples.
#[tauri::command]
pub async fn create_backscatter_mosaic_cmd(
    samples: Vec<crate::marine::backscatter::BackscatterSample>,
    params: crate::marine::backscatter::MosaicParams,
) -> Result<crate::marine::backscatter::BackscatterMosaic, String> {
    tokio::task::spawn_blocking(move || {
        crate::marine::backscatter::create_mosaic(&samples, &params)
            .map_err(|e| format!("backscatter mosaic failed: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

/// Compute QC statistics for a set of soundings.
/// Returns S-44 compliance, density, coverage, and quality stats.
#[tauri::command]
pub fn compute_qc_stats_cmd(
    soundings: Vec<(f64, f64, f64, u8, f64, f64)>,
    cell_size: f64,
    s44_order: String,
) -> Result<crate::marine::qc_dashboard::QcStats, String> {
    crate::marine::qc_dashboard::compute_qc_stats(&soundings, cell_size, &s44_order)
        .map_err(|e| format!("QC stats failed: {e}"))
}
