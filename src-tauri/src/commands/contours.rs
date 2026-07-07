// Contour IPC commands — Sprint 12.

use crate::contours::{generate_contours, contours_to_geojson, ContourResult};

#[tauri::command]
pub fn generate_contours_cmd(
    grid: Vec<f64>,
    ncols: usize,
    nrows: usize,
    cell_size: f64,
    origin_x: f64,
    origin_y: f64,
    interval: f64,
    base_elevation: f64,
) -> ContourResult {
    generate_contours(&grid, ncols, nrows, cell_size, origin_x, origin_y, interval, base_elevation)
}

#[tauri::command]
pub fn contours_to_geojson_cmd(result: ContourResult) -> String {
    contours_to_geojson(&result)
}
