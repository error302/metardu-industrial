// 4D Monitoring IPC commands — Phase 3 + Sprint 5 highwall.
//
// Exposes compute_epoch_diff, compute_progression, and the Sprint 5
// highwall deformation analysis (time-series + alerts + compliance).

use crate::commands::mining::read_dem_grid;
use crate::formats::read_geotiff_header;
use crate::mining::{
    analyze_highwall, compute_epoch_diff, compute_progression, HighwallThresholds,
    Monitoring4DParams,
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct EpochDiffRequest {
    #[serde(rename = "previousPath")]
    pub previous_path: String,
    #[serde(rename = "currentPath")]
    pub current_path: String,
    pub params: Monitoring4DParams,
}

#[tauri::command]
pub async fn compute_epoch_diff_cmd(
    request: EpochDiffRequest,
) -> Result<crate::mining::monitoring_4d::EpochDiff, String> {
    let prev_path = PathBuf::from(&request.previous_path);
    let curr_path = PathBuf::from(&request.current_path);

    let prev_header = read_geotiff_header(&prev_path).map_err(|e| {
        ctx!(
            "reading previous-survey DEM header",
            request.previous_path,
            e
        )
    })?;
    let prev_grid = read_dem_grid(&prev_path, &prev_header)
        .map_err(|e| ctx!("reading previous-survey DEM grid", request.previous_path, e))?;

    let curr_header = read_geotiff_header(&curr_path)
        .map_err(|e| ctx!("reading current-survey DEM header", request.current_path, e))?;
    let curr_grid = read_dem_grid(&curr_path, &curr_header)
        .map_err(|e| ctx!("reading current-survey DEM grid", request.current_path, e))?;

    compute_epoch_diff(&prev_grid, &curr_grid, &request.params)
        .map_err(|e| ctx_no_input!("computing 4D epoch difference", e))
}

#[derive(Debug, Deserialize)]
pub struct ProgressionRequest {
    pub paths: Vec<String>,
    pub params: Monitoring4DParams,
}

#[tauri::command]
pub async fn compute_progression_cmd(
    request: ProgressionRequest,
) -> Result<crate::mining::monitoring_4d::ProgressionReport, String> {
    if request.paths.len() < 2 {
        return Err(format!(
            "compute_progression_cmd: at least 2 surfaces required, got {}",
            request.paths.len()
        ));
    }

    let mut surfaces = Vec::with_capacity(request.paths.len());
    for (i, path) in request.paths.iter().enumerate() {
        let p = PathBuf::from(path);
        let header = read_geotiff_header(&p)
            .map_err(|e| ctx!("reading progression DEM header (epoch {}", i, e))?;
        let grid = read_dem_grid(&p, &header)
            .map_err(|e| ctx!("reading progression DEM grid (epoch {}", i, e))?;
        surfaces.push(grid);
    }

    compute_progression(&surfaces, &request.params)
        .map_err(|e| ctx_no_input!("computing N-epoch progression", e))
}

// ──────────────────────────────────────────────────────────────────
// Sprint 5 — Highwall deformation monitoring (Revenue Feature #6)

#[derive(Debug, Deserialize)]
pub struct HighwallRequest {
    /// Paths to epoch DEMs (GeoTIFFs) in chronological order. Min 2.
    pub paths: Vec<String>,
    /// ISO 8601 dates corresponding to each epoch (YYYY-MM-DD)
    #[serde(rename = "epochDates")]
    pub epoch_dates: Vec<String>,
    /// Cell area in square meters (from DEM pixel scale). Default 1.0.
    #[serde(rename = "cellAreaM2", default = "default_cell_area")]
    pub cell_area_m2: f64,
    /// Optional custom thresholds. If omitted, USACE defaults are used.
    #[serde(default)]
    pub thresholds: HighwallThresholds,
}

fn default_cell_area() -> f64 {
    1.0
}

#[tauri::command]
pub async fn analyze_highwall_cmd(
    request: HighwallRequest,
) -> Result<crate::mining::highwall::HighwallReport, String> {
    if request.paths.len() < 2 {
        return Err(format!(
            "analyze_highwall_cmd: at least 2 epochs required, got {}",
            request.paths.len()
        ));
    }
    if request.epoch_dates.len() != request.paths.len() {
        return Err(format!(
            "analyze_highwall_cmd: epoch_dates count ({}) must match paths count ({})",
            request.epoch_dates.len(),
            request.paths.len()
        ));
    }

    let mut surfaces = Vec::with_capacity(request.paths.len());
    for (i, path) in request.paths.iter().enumerate() {
        let p = PathBuf::from(path);
        let header = read_geotiff_header(&p)
            .map_err(|e| ctx!("reading highwall epoch DEM header (epoch {}", i, e))?;
        let grid = read_dem_grid(&p, &header)
            .map_err(|e| ctx!("reading highwall epoch DEM grid (epoch {}", i, e))?;
        surfaces.push(grid);
    }

    analyze_highwall(
        &surfaces,
        &request.epoch_dates,
        request.cell_area_m2,
        &request.thresholds,
    )
    .map_err(|e| ctx_no_input!("analyzing highwall deformation", e))
}
