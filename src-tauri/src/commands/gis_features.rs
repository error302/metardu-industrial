// Sprint 15 GIS gap feature IPC commands — IDW interpolation, Shapefile, topology.

use crate::interpolation::{interpolate_idw, IdwParams, IdwResult, Point3D};
use crate::topology::{validate_lines, validate_polygons, TopologyParams, TopologyReport};
use crate::formats::shapefile::{read_shapefile, write_shapefile, Shapefile, ShapefileFeature};

// ── IDW Interpolation ──

#[tauri::command]
pub fn interpolate_idw_cmd(
    points: Vec<(f64, f64, f64)>, // (x, y, z)
    bounds: (f64, f64, f64, f64), // (min_x, min_y, max_x, max_y)
    cell_size: f64,
    params: IdwParams,
) -> Result<IdwResult, String> {
    let pts: Vec<Point3D> = points.into_iter().map(|(x, y, z)| Point3D { x, y, z }).collect();
    interpolate_idw(&pts, bounds, cell_size, &params).map_err(|e| format!("IDW interpolation failed: {e}"))
}

// ── Shapefile ──

#[tauri::command]
pub async fn read_shapefile_cmd(path: String) -> Result<Shapefile, String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating shapefile path", path, e))?;
    let label = path.clone();
    tokio::task::spawn_blocking(move || {
        read_shapefile(&path_buf).map_err(|e| ctx!("reading shapefile", label, e))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn write_shapefile_cmd(path: String, features: Vec<ShapefileFeature>) -> Result<(), String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating shapefile write path", path, e))?;
    let label = path.clone();
    tokio::task::spawn_blocking(move || {
        write_shapefile(&path_buf, &features).map_err(|e| ctx!("writing shapefile", label, e))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ── Topology validation ──

#[tauri::command]
pub fn validate_polygons_cmd(
    polygons: Vec<Vec<Vec<(f64, f64)>>>,
    params: TopologyParams,
) -> TopologyReport {
    validate_polygons(&polygons, &params)
}

#[tauri::command]
pub fn validate_lines_cmd(
    lines: Vec<Vec<(f64, f64)>>,
    params: TopologyParams,
) -> TopologyReport {
    validate_lines(&lines, &params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_idw_cmd() {
        let points = vec![(5.0, 5.0, 100.0)];
        let result = interpolate_idw_cmd(points, (0.0, 0.0, 10.0, 10.0), 1.0, IdwParams::default()).unwrap();
        assert_eq!(result.ncols, 10);
        assert_eq!(result.nrows, 10);
    }

    #[test]
    fn test_validate_polygons_cmd() {
        let polygons = vec![vec![vec![
            (0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0),
        ]]];
        let report = validate_polygons_cmd(polygons, TopologyParams::default());
        assert!(report.passed);
    }
}
