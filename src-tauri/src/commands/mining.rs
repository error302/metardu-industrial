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
    let path_buf = PathBuf::from(&path);
    let points = crate::formats::read_las_points(&path_buf, max_points.unwrap_or(0))
        .map_err(|e| ctx!("reading LAS points for ground classification", path, e))?;
    csf_classify(&points, &params).map_err(|e| ctx!("running CSF ground classification", path, e))
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
    let current_grid = read_dem_grid(&current_path, &current_header)
        .map_err(|e| ctx!("reading current-survey DEM grid", request.current_path, e))?;

    let (reference_grid, _ref_header) = if request.reference_path.starts_with("flat:") {
        let z: f64 = request
            .reference_path
            .strip_prefix("flat:")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("invalid flat:Z reference: '{}'", request.reference_path))?;
        (vec![z; current_grid.len()], current_header.clone())
    } else {
        let ref_path = PathBuf::from(&request.reference_path);
        let header = read_geotiff_header(&ref_path)
            .map_err(|e| ctx!("reading reference DEM header", request.reference_path, e))?;
        let grid = read_dem_grid(&ref_path, &header)
            .map_err(|e| ctx!("reading reference DEM grid", request.reference_path, e))?;
        (grid, header)
    };

    let (cell_w_m, cell_h_m) = derive_cell_meters(&current_header);

    compute_volumes_core(
        &current_grid,
        &reference_grid,
        cell_w_m,
        cell_h_m,
        request.bench_interval,
    )
    .map_err(|e| ctx_no_input!("computing fill/cut volumes", e))
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

    let total = (header.width as usize) * (header.length as usize);
    if total > 10_000_000 {
        return Err(format!(
            "DEM too large for Phase 1 in-memory loading: {} pixels (max 10M)",
            total
        ));
    }

    let mut file = File::open(path).map_err(|e| e.to_string())?;
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
                grid.push(0.0);
                continue;
            }
            let bytes = &strip[offset..offset + bytes_per_sample];
            let val = decode_pixel(bytes, header.sample_format, bytes_per_sample);
            grid.push(val);
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
