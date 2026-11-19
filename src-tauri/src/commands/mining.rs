// Mining IPC commands — Phase 1 Mining MVP.
//
// Exposes the drone_ingest, csf, and volume modules to the frontend.

use crate::mining::{
    classify_ground as csf_classify, compute_volumes as compute_volumes_core, parse_manifest,
    CsfParams, CsfResult, DroneManifest, VolumeResult,
};
use serde::Deserialize;
use std::path::PathBuf;

/// Parse a drone manifest (.mrk / .json / .csv) and return image metadata.
#[tauri::command]
pub fn parse_drone_manifest(path: String) -> Result<DroneManifest, String> {
    let path_buf = PathBuf::from(&path);
    parse_manifest(&path_buf).map_err(|e| ctx!("parsing drone manifest", path, e))
}

/// Run CSF (Cloth Simulation Filter) ground extraction on a LAS point cloud.
///
/// Reads point data via the LAS module, runs CSF, returns per-point
/// classification. The frontend can use this to render ground vs non-ground
/// in different colors, or to filter the point cloud for DEM generation.
#[tauri::command]
pub async fn classify_ground(
    path: String,
    params: CsfParams,
    max_points: Option<u64>,
) -> Result<CsfResult, String> {
    // Sprint 21: save recovery snapshot before long operation
    let _snapshot = crate::recovery::save_recovery_snapshot(
        &format!("{{\"operation\":\"classify_ground\",\"path\":\"{}\"}}", path),
        "classify_ground",
    );

    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating LAS path for CSF", path, e))?;
    let points = crate::formats::read_las_points(&path_buf, max_points.unwrap_or(0))
        .map_err(|e| ctx!("reading LAS points for ground classification", path, e))?;
    let result = csf_classify(&points, &params).map_err(|e| ctx!("running CSF ground classification", path, e))?;

    // Clear recovery snapshot on success
    if let Ok(ref snap) = _snapshot {
        crate::recovery::clear_recovery_snapshot(snap);
    }
    Ok(result)
}

#[derive(Debug, Deserialize)]
pub struct ComputeVolumesRequest {
    /// Path to the current-survey GeoTIFF DEM
    pub current_path: String,
    /// Path to the reference-survey GeoTIFF DEM (or "flat:Z" for a flat plane at elevation Z)
    pub reference_path: String,
    /// Bench interval for bench-by-bench breakdown (meters). 0 = no breakdown.
    #[serde(rename = "benchInterval")]
    pub bench_interval: f64,
}

/// Compute fill/cut volumes by differencing two DEM surfaces.
///
/// Phase 1 limitation: both DEMs must be GeoTIFFs with the same dimensions
/// and geographic extent. Phase 2+ will add resampling for mismatched grids.
#[tauri::command]
pub async fn compute_volumes_cmd(request: ComputeVolumesRequest) -> Result<VolumeResult, String> {
    use crate::formats::read_geotiff_header;

    let current_path = PathBuf::from(&request.current_path);
    let current_header = read_geotiff_header(&current_path)
        .map_err(|e| ctx!("reading current-survey DEM header", request.current_path, e))?;
    let (cell_w_m, cell_h_m) = checked_metric_grid(&current_header)?;
    let current_grid = read_dem_grid(&current_path, &current_header)
        .map_err(|e| ctx!("reading current-survey DEM grid", request.current_path, e))?;

    let (reference_grid, _ref_header) = if request.reference_path.starts_with("flat:") {
        let z: f64 = request
            .reference_path
            .strip_prefix("flat:")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("invalid flat:Z reference: '{}'", request.reference_path))?;
        if !z.is_finite() { return Err("Flat reference elevation must be finite".into()); }
        (vec![z; current_grid.len()], current_header.clone())
    } else if request.reference_path.starts_with("dxf:") {
        return Err("DXF surface comparison is unavailable until exact grid alignment and NoData handling are verified. Use an aligned GeoTIFF reference.".into());
    } else {
        let ref_path = PathBuf::from(&request.reference_path);
        let header = read_geotiff_header(&ref_path)
            .map_err(|e| ctx!("reading reference DEM header", request.reference_path, e))?;
        validate_aligned_grids(&current_header, &header)?;
        let grid = read_dem_grid(&ref_path, &header)
            .map_err(|e| ctx!("reading reference DEM grid", request.reference_path, e))?;
        (grid, header)
    };

    compute_volumes_core(
        &current_grid,
        &reference_grid,
        cell_w_m,
        cell_h_m,
        request.bench_interval,
    )
    .map_err(|e| ctx_no_input!("computing fill/cut volumes", e))
}

fn checked_metric_grid(header: &crate::formats::GeoTiffHeader) -> Result<(f64, f64), String> {
    // EPSG UTM north/south and WGS84 / UTM are metre-based. Other CRSs
    // require explicit unit metadata, which this reader does not yet expose.
    match header.epsg {
        Some(32601..=32660 | 32701..=32760) => {}
        _ => return Err("Volume calculations require a recognized metre-based WGS 84 UTM CRS (EPSG:32601-32660 or 32701-32760). Convert the DEM before calculating.".into()),
    }
    let scale = header.model_pixel_scale.ok_or("DEM pixel scale is missing")?;
    if !scale[0].is_finite() || !scale[1].is_finite() || scale[0] <= 0.0 || scale[1] <= 0.0 {
        return Err("DEM pixel dimensions must be finite and positive".into());
    }
    let tie = header.model_tiepoint.ok_or("DEM georeferencing tiepoint is missing")?;
    if !tie.iter().all(|v| v.is_finite()) {
        return Err("DEM tiepoint contains non-finite coordinates".into());
    }
    Ok((scale[0], scale[1]))
}

fn validate_aligned_grids(current: &crate::formats::GeoTiffHeader, reference: &crate::formats::GeoTiffHeader) -> Result<(), String> {
    checked_metric_grid(reference)?;
    if current.epsg != reference.epsg || current.width != reference.width || current.length != reference.length
        || current.model_pixel_scale != reference.model_pixel_scale || current.model_tiepoint != reference.model_tiepoint {
        return Err("Survey and reference DEMs must use the same CRS, dimensions, pixel scale and tiepoint. Align them before calculating.".into());
    }
    Ok(())
}

#[cfg(test)]
mod grid_validation_tests {
    use super::*;
    use crate::formats::GeoTiffHeader;

    fn header() -> GeoTiffHeader {
        GeoTiffHeader {
            width: 2, length: 2, bits_per_sample: 32, samples_per_pixel: 1,
            compression: 1, photometric: 1, is_tiled: false, strip_count: 1,
            model_pixel_scale: Some([2.0, 2.0, 0.0]),
            model_tiepoint: Some([0.0, 0.0, 0.0, 500_000.0, 9_900_000.0, 0.0]),
            epsg: Some(32737), geo_ascii: None, nodata: None, bounds: None, sample_format: 3,
            rows_per_strip: 2, strip_offsets: vec![8], strip_byte_counts: vec![16],
        }
    }

    #[test]
    fn accepts_aligned_utm_grids() {
        let current = header();
        assert_eq!(checked_metric_grid(&current).unwrap(), (2.0, 2.0));
        assert!(validate_aligned_grids(&current, &current.clone()).is_ok());
    }

    #[test]
    fn rejects_equal_size_but_shifted_or_incompatible_grids() {
        let current = header();
        let mut reference = current.clone();
        reference.model_tiepoint.as_mut().unwrap()[3] += 1.0;
        assert!(validate_aligned_grids(&current, &reference).is_err());
        reference = current.clone();
        reference.epsg = Some(4326);
        assert!(validate_aligned_grids(&current, &reference).is_err());
        reference = current.clone();
        reference.model_pixel_scale = None;
        assert!(validate_aligned_grids(&current, &reference).is_err());
    }

    #[test]
    fn gdal_nodata_tag_is_excluded_from_dem_samples() {
        // Minimal little-endian TIFF: one GDAL_NODATA tag followed by four f32 cells.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"II");
        bytes.extend_from_slice(&42u16.to_le_bytes());
        bytes.extend_from_slice(&8u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&42113u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&6u32.to_le_bytes());
        bytes.extend_from_slice(&26u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(b"-9999\0");
        for value in [1.0_f32, -9999.0, 2.0, 3.0] { bytes.extend_from_slice(&value.to_le_bytes()); }
        let suffix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("metardu-nodata-{}-{suffix}.tif", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        let parsed = crate::formats::read_geotiff_header(&path).unwrap();
        assert_eq!(parsed.nodata.as_deref(), Some("-9999"));
        let mut raster = header();
        raster.nodata = parsed.nodata;
        raster.strip_offsets = vec![32];
        let values = read_dem_grid(&path, &raster).unwrap();
        assert_eq!(values[0], 1.0);
        assert!(values[1].is_nan());
        assert_eq!(values[2], 2.0);
        let volume = compute_volumes_core(&values, &[0.0; 4], 1.0, 1.0, 0.0).unwrap();
        assert_eq!(volume.nodata_cells, 1);
        assert_eq!(volume.fill_volume, 6.0);
        std::fs::remove_file(path).unwrap();
    }
}

/// Read a GeoTIFF DEM into a flat Vec<f64> elevation grid.
///
/// Phase 1: supports uncompressed strips with float32/float64/uint16/uint32
/// sample formats. Errors out for tiled or compressed DEMs.
pub fn read_dem_grid(
    path: &std::path::Path,
    header: &crate::formats::GeoTiffHeader,
) -> Result<Vec<f64>, String> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    if header.is_tiled || header.compression != 1 || header.samples_per_pixel != 1 || header.rows_per_strip == 0 {
        return Err("DEM reader supports only uncompressed, single-band strip GeoTIFFs".into());
    }
    if !matches!((header.sample_format, header.bits_per_sample), (1 | 2, 8 | 16 | 32) | (3, 32 | 64)) {
        return Err("Unsupported DEM sample format or bit depth".into());
    }
    let nodata = header.nodata.as_deref().map(|value| value.parse::<f64>().map_err(|_| "Invalid DEM NoData value".to_string())).transpose()?
        .map(|value| if header.sample_format == 3 && header.bits_per_sample == 32 { value as f32 as f64 } else { value });

    let total = (header.width as usize) * (header.length as usize);
    if total > 10_000_000 {
        return Err(format!(
            "DEM too large for Phase 1 in-memory loading: {} pixels (max 10M)",
            total
        ));
    }

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut byte_order = [0u8; 2];
    file.read_exact(&mut byte_order).map_err(|e| e.to_string())?;
    if &byte_order != b"II" { return Err("DEM reader supports only little-endian GeoTIFFs".into()); }
    let bytes_per_sample = (header.bits_per_sample as usize) / 8;
    let row_stride = header.width as usize * bytes_per_sample * header.samples_per_pixel as usize;

    let mut strip_data: Vec<Vec<u8>> = Vec::with_capacity(header.strip_offsets.len());
    for (i, &offset) in header.strip_offsets.iter().enumerate() {
        let size = header.strip_byte_counts.get(i).copied().unwrap_or(0) as usize;
        if size == 0 {
            strip_data.push(Vec::new());
            continue;
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; size];
        file.read_exact(&mut buf).map_err(|e| e.to_string())?;
        strip_data.push(buf);
    }

    let mut grid = Vec::with_capacity(total);
    for row in 0..header.length as usize {
        for col in 0..header.width as usize {
            let strip_idx = row / (header.rows_per_strip as usize);
            let row_in_strip = row % (header.rows_per_strip as usize);
            let strip = strip_data
                .get(strip_idx)
                .ok_or("strip index out of range")?;
            let offset = row_in_strip * row_stride + col * bytes_per_sample;
            if offset + bytes_per_sample > strip.len() {
                return Err(format!("Truncated DEM strip at row {row}, column {col}"));
            }
            let bytes = &strip[offset..offset + bytes_per_sample];
            let val = decode_pixel(bytes, header.sample_format, bytes_per_sample);
            grid.push(if nodata.is_some_and(|missing| val == missing || (missing.is_nan() && val.is_nan())) { f64::NAN } else { val });
        }
    }

    Ok(grid)
}

fn decode_pixel(bytes: &[u8], sample_format: u16, bytes_per_sample: usize) -> f64 {
    match (sample_format, bytes_per_sample) {
        (1, 1) => u8::from_le_bytes([bytes[0]]) as f64,
        (1, 2) => u16::from_le_bytes([bytes[0], bytes[1]]) as f64,
        (1, 4) => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
        (2, 1) => i8::from_le_bytes([bytes[0]]) as f64,
        (2, 2) => i16::from_le_bytes([bytes[0], bytes[1]]) as f64,
        (2, 4) => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
        (3, 4) => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
        (3, 8) => f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        _ => 0.0,
    }
}

/// Derive cell dimensions in meters from a GeoTIFF header's pixel scale.
///
/// For projected DEMs (UTM, MGA, etc.), the pixel scale is already in meters.
/// For geographic DEMs (WGS84), we'd need to multiply by the cosine of the
/// latitude at the grid center. Phase 1 assumes the DEM is projected —
/// geographic DEMs get an approximate scale based on the equator.
pub fn derive_cell_meters(header: &crate::formats::GeoTiffHeader) -> (f64, f64) {
    if let Some(scale) = header.model_pixel_scale {
        (scale[0].abs(), scale[1].abs())
    } else {
        (1.0, 1.0)
    }
}

// ──────────────────────────────────────────────────────────────────
// Mining surveyor tools — setting out, mine grid, tunnel profiles
// ──────────────────────────────────────────────────────────────────

/// Compute setout information (bearing, distance, slope) from a reference
/// point to a list of design points. Used for field markout with a
/// total station or RTK GPS.
#[tauri::command]
pub fn compute_setout_cmd(
    points: Vec<crate::mining::survey_tools::SetoutPoint>,
    ref_easting: f64,
    ref_northing: f64,
    ref_elevation: f64,
) -> Vec<crate::mining::survey_tools::SetoutResult> {
    crate::mining::survey_tools::compute_setout(&points, ref_easting, ref_northing, ref_elevation)
}

/// Convert mine grid coordinates to parent CRS coordinates.
#[tauri::command]
pub fn mine_grid_to_crs_cmd(
    grid: crate::mining::survey_tools::MineGrid,
    grid_easting: f64,
    grid_northing: f64,
) -> (f64, f64) {
    crate::mining::survey_tools::mine_grid_to_crs(&grid, grid_easting, grid_northing)
}

/// Convert parent CRS coordinates to mine grid coordinates.
#[tauri::command]
pub fn crs_to_mine_grid_cmd(
    grid: crate::mining::survey_tools::MineGrid,
    crs_easting: f64,
    crs_northing: f64,
) -> (f64, f64) {
    crate::mining::survey_tools::crs_to_mine_grid(&grid, crs_easting, crs_northing)
}

/// Analyze a tunnel profile: compute area, overbreak/underbreak vs design.
#[tauri::command]
pub fn analyze_tunnel_profile_cmd(
    profile: crate::mining::survey_tools::TunnelProfile,
) -> Result<crate::mining::survey_tools::TunnelProfileResult, String> {
    crate::mining::survey_tools::analyze_tunnel_profile(&profile)
        .map_err(|e| format!("tunnel profile analysis failed: {e}"))
}

/// Generate a safety inspection report.
#[tauri::command]
pub fn generate_safety_report_cmd(
    inspection: crate::mining::survey_tools::SafetyInspection,
) -> String {
    crate::mining::survey_tools::generate_safety_report(&inspection)
}

/// Compare two LAS surveys of the same stockpile and produce a per-cell
/// cut/fill change-detection report.
///
/// `current_path` is the newer survey; `previous_path` is the baseline.
/// Positive Δz (fill) means material was added; negative Δz (cut) means
/// material was removed. The hotspot threshold flags cells where |Δz|
/// exceeds the value, useful for spotting data errors or unexpected
/// movement.
#[tauri::command]
pub async fn compute_stockpile_change_cmd(
    current_path: String,
    previous_path: String,
    cell_size_m: f64,
    hotspot_threshold_m: f64,
) -> Result<crate::mining::change_detection::ChangeDetectionResult, String> {
    let cur_path = crate::path_validation::validate_path(&current_path)
        .map_err(|e| ctx!("validating current LAS path", current_path, e))?;
    let prev_path = crate::path_validation::validate_path(&previous_path)
        .map_err(|e| ctx!("validating previous LAS path", previous_path, e))?;
    let cur_label = current_path.clone();
    let prev_label = previous_path.clone();
    tokio::task::spawn_blocking(move || {
        crate::mining::change_detection::detect_stockpile_change(
            &cur_path,
            &prev_path,
            cell_size_m,
            hotspot_threshold_m,
        )
        .map_err(|e| {
            ctx!(
                "computing stockpile change",
                format!("current={} previous={}", cur_label, prev_label),
                e
            )
        })
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

/// Compute volumes with uncertainty propagation + grid/TIN cross-check.
///
/// Wraps `compute_volumes_verified` so the frontend can request a
/// defensible volume result that carries ± uncertainty and a
/// verification flag.
#[tauri::command]
pub async fn compute_volumes_verified_cmd(
    current_path: String,
    reference_path: String,
    sigma_z_m: f64,
) -> Result<crate::mining::volume::VerifiedVolumeResult, String> {
    use crate::formats::read_geotiff_header;
    let cur_path = PathBuf::from(&current_path);
    let ref_path = PathBuf::from(&reference_path);
    let cur_label = current_path.clone();
    let ref_label = reference_path.clone();

    tokio::task::spawn_blocking(move || -> Result<crate::mining::volume::VerifiedVolumeResult, String> {
        let cur_header = read_geotiff_header(&cur_path)
            .map_err(|e| ctx!("reading current DEM header", cur_label, e))?;
        let (cell_w, cell_h) = checked_metric_grid(&cur_header)?;
        let cur_grid = read_dem_grid(&cur_path, &cur_header)
            .map_err(|e| ctx!("reading current DEM grid", cur_label, e))?;
        let ref_grid = if reference_path.starts_with("flat:") {
            let z: f64 = reference_path.strip_prefix("flat:").and_then(|s| s.parse().ok())
                .ok_or_else(|| format!("invalid flat:Z reference: '{}'", reference_path))?;
            if !z.is_finite() { return Err("Flat reference elevation must be finite".into()); }
            vec![z; cur_grid.len()]
        } else {
            let ref_header = read_geotiff_header(&ref_path)
                .map_err(|e| ctx!("reading reference DEM header", ref_label, e))?;
            validate_aligned_grids(&cur_header, &ref_header)?;
            read_dem_grid(&ref_path, &ref_header)
                .map_err(|e| ctx!("reading reference DEM grid", ref_label, e))?
        };
        crate::mining::volume::compute_volumes_verified(
            &cur_grid, &ref_grid, cell_w, cell_h, 0.0, sigma_z_m,
        ).map_err(|e| format!("verified volume failed: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

/// Compute volumes with uncertainty + cross-check, wrapped in a 5-minute
/// timeout (Sprint 14 Backend Architect audit fix #3).
///
/// Returns `MetarduError::Timeout` if the operation doesn't complete
/// within `timeout_secs` seconds (default 300 = 5 minutes).
#[tauri::command]
pub async fn compute_volumes_verified_timed_cmd(
    current_path: String,
    reference_path: String,
    sigma_z_m: f64,
    timeout_secs: Option<u64>,
) -> Result<crate::mining::volume::VerifiedVolumeResult, crate::error_types::MetarduError> {
    use crate::error_types::{with_timeout, MetarduError, DEFAULT_TIMEOUT_SECS};
    use crate::formats::read_geotiff_header;
    let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let cur_path = PathBuf::from(&current_path);
    let ref_path = PathBuf::from(&reference_path);
    let cur_label = current_path.clone();
    let ref_label = reference_path.clone();

    with_timeout("compute_volumes_verified", timeout, async {
        tokio::task::spawn_blocking(move || -> Result<crate::mining::volume::VerifiedVolumeResult, MetarduError> {
            let cur_header = read_geotiff_header(&cur_path)
                .map_err(|e| MetarduError::parse_error("GeoTIFF", format!("reading header: {e}")))?;
            let (cell_w, cell_h) = checked_metric_grid(&cur_header)
                .map_err(|e| MetarduError::calculation_error("compute_volumes_verified", e))?;
            let cur_grid = read_dem_grid(&cur_path, &cur_header)
                .map_err(|e| MetarduError::io_error(format!("reading current DEM grid: {e}")))?;
            let ref_grid = if reference_path.starts_with("flat:") {
                let z: f64 = reference_path.strip_prefix("flat:").and_then(|s| s.parse().ok())
                    .ok_or_else(|| MetarduError::invalid_input("reference_path", ref_label.clone(), "invalid flat:Z reference"))?;
                if !z.is_finite() { return Err(MetarduError::invalid_input("reference_path", ref_label.clone(), "flat elevation must be finite")); }
                vec![z; cur_grid.len()]
            } else {
                let ref_header = read_geotiff_header(&ref_path)
                    .map_err(|e| MetarduError::parse_error("GeoTIFF", format!("reading reference header: {e}")))?;
                validate_aligned_grids(&cur_header, &ref_header)
                    .map_err(|e| MetarduError::calculation_error("compute_volumes_verified", e))?;
                read_dem_grid(&ref_path, &ref_header)
                    .map_err(|e| MetarduError::io_error(format!("reading reference DEM grid: {e}")))?
            };
            crate::mining::volume::compute_volumes_verified(
                &cur_grid, &ref_grid, cell_w, cell_h, 0.0, sigma_z_m,
            ).map_err(|e| MetarduError::calculation_error("compute_volumes_verified", e.to_string()))
        })
        .await
        .map_err(|e| MetarduError::internal(format!("task join error: {e}")))?
    }).await
}

/// Compute cut/fill volumes using the average end-area method.
///
/// Takes a list of cross-sections (chainage + cut area + fill area)
/// and computes per-segment + total volumes. Used for haul roads,
/// ramps, dredge channels, tailings dams.
#[tauri::command]
pub fn compute_end_area_volumes_cmd(
    sections: Vec<crate::mining::volume::CrossSection>,
) -> Result<crate::mining::volume::EndAreaVolumeResult, String> {
    crate::mining::volume::compute_end_area_volumes(&sections)
        .map_err(|e| format!("end-area volume failed: {e}"))
}
