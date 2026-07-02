// 4D Monitoring IPC commands — Phase 3.
//
// Exposes compute_epoch_diff and compute_progression to the frontend.

use crate::commands::mining::read_dem_grid;
use crate::formats::read_geotiff_header;
use crate::mining::{compute_epoch_diff, compute_progression, Monitoring4DParams};
use serde::{Deserialize, Serialize};
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

    let prev_header = read_geotiff_header(&prev_path).map_err(|e| e.to_string())?;
    let prev_grid = read_dem_grid(&prev_path, &prev_header).map_err(|e| e.to_string())?;

    let curr_header = read_geotiff_header(&curr_path).map_err(|e| e.to_string())?;
    let curr_grid = read_dem_grid(&curr_path, &curr_header).map_err(|e| e.to_string())?;

    compute_epoch_diff(&prev_grid, &curr_grid, &request.params).map_err(|e| e.to_string())
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
        return Err("at least 2 surfaces required".into());
    }

    let mut surfaces = Vec::with_capacity(request.paths.len());
    for path in &request.paths {
        let p = PathBuf::from(path);
        let header = read_geotiff_header(&p).map_err(|e| e.to_string())?;
        let grid = read_dem_grid(&p, &header).map_err(|e| e.to_string())?;
        surfaces.push(grid);
    }

    compute_progression(&surfaces, &request.params).map_err(|e| e.to_string())
}
