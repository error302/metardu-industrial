// Marine IPC commands — Phase 2 Marine MVP + Sprint 4 dredge audit.
//
// Exposes CUBE surface generation, TPU, S-44 compliance, SVP parsing,
// and the Sprint 4 dredge pay-volume audit to the frontend.

use crate::commands::mining::{derive_cell_meters, read_dem_grid};
use crate::formats::read_geotiff_header;
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
    generate_cube_surface(&soundings, &params).map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())
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
    check_s44_compliance(&request.soundings, request.target_order).map_err(|e| e.to_string())
}

/// Export features to an S-57 .000 file.
#[tauri::command]
pub fn export_s57(features: Vec<S57Feature>, path: String) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(&path);
    write_s57(&path_buf, &features).map_err(|e| e.to_string())
}

/// Parse an SVP (Sound Velocity Profile) file.
#[tauri::command]
pub fn parse_svp_cmd(path: String) -> Result<SvpProfile, String> {
    let path_buf = std::path::PathBuf::from(&path);
    parse_svp(&path_buf).map_err(|e| e.to_string())
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
    // Read post-dredge grid (the reference grid)
    let post_path = PathBuf::from(&request.post_path);
    let post_header = read_geotiff_header(&post_path).map_err(|e| e.to_string())?;
    let post_grid = read_dem_grid(&post_path, &post_header).map_err(|e| e.to_string())?;
    let (cell_w_m, cell_h_m) = derive_cell_meters(&post_header);

    // Read pre-dredge grid
    let pre_path = PathBuf::from(&request.pre_path);
    let pre_header = read_geotiff_header(&pre_path).map_err(|e| e.to_string())?;
    let pre_grid = read_dem_grid(&pre_path, &pre_header).map_err(|e| e.to_string())?;

    // Design grid: either GeoTIFF or flat:Z
    let (design_grid, _design_header) = if request.design_path.starts_with("flat:") {
        let z: f64 = request
            .design_path
            .strip_prefix("flat:")
            .and_then(|s| s.parse().ok())
            .ok_or("flat:Z design must be flat:<number>")?;
        (vec![z; post_grid.len()], post_header.clone())
    } else {
        let d_path = PathBuf::from(&request.design_path);
        let header = read_geotiff_header(&d_path).map_err(|e| e.to_string())?;
        let grid = read_dem_grid(&d_path, &header).map_err(|e| e.to_string())?;
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
    .map_err(|e| e.to_string())
}
