// Sprint 15 GIS gap feature IPC commands — IDW interpolation, Shapefile, topology.
// Sprint 16: orthomosaic reader.

use crate::interpolation::{interpolate_idw, IdwParams, IdwResult, Point3D};
use crate::topology::{validate_lines, validate_polygons, TopologyParams, TopologyReport};
use crate::formats::shapefile::{read_shapefile, write_shapefile, Shapefile, ShapefileFeature};
use crate::formats::orthomosaic::{read_orthomosaic, Orthomosaic};

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

// ── Orthomosaic (Sprint 16) ──

#[tauri::command]
pub async fn read_orthomosaic_cmd(path: String) -> Result<Orthomosaic, String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating orthomosaic path", path, e))?;
    let label = path.clone();
    tokio::task::spawn_blocking(move || {
        read_orthomosaic(&path_buf).map_err(|e| ctx!("reading orthomosaic", label, e))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ── Map layout composer (Sprint 16) ──

#[tauri::command]
pub async fn generate_map_layout_cmd(
    request: crate::map_layout::MapLayoutRequest,
) -> Result<crate::map_layout::MapLayoutResult, String> {
    tokio::task::spawn_blocking(move || {
        crate::map_layout::generate_map_layout(&request)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ── GeoJSON + KML export (Sprint 17) ──

#[tauri::command]
pub async fn export_geojson_cmd(shp_path: String, output_path: String) -> Result<(), String> {
    let shp_path_buf = crate::path_validation::validate_path(&shp_path)
        .map_err(|e| ctx!("validating shapefile path", shp_path, e))?;
    let out_path = crate::path_validation::validate_path(&output_path)
        .map_err(|e| ctx!("validating output path", output_path, e))?;
    let shp_label = shp_path.clone();
    tokio::task::spawn_blocking(move || {
        let shp = crate::formats::shapefile::read_shapefile(&shp_path_buf)
            .map_err(|e| ctx!("reading shapefile", shp_label, e))?;
        crate::export_formats::export_geojson(&shp, &out_path)
            .map_err(|e| format!("exporting GeoJSON: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn export_kml_cmd(shp_path: String, output_path: String, document_name: String) -> Result<(), String> {
    let shp_path_buf = crate::path_validation::validate_path(&shp_path)
        .map_err(|e| ctx!("validating shapefile path", shp_path, e))?;
    let out_path = crate::path_validation::validate_path(&output_path)
        .map_err(|e| ctx!("validating output path", output_path, e))?;
    let shp_label = shp_path.clone();
    tokio::task::spawn_blocking(move || {
        let shp = crate::formats::shapefile::read_shapefile(&shp_path_buf)
            .map_err(|e| ctx!("reading shapefile", shp_label, e))?;
        crate::export_formats::export_kml(&shp, &out_path, &document_name)
            .map_err(|e| format!("exporting KML: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ── CRS consistency audit (Sprint 17) ──

#[derive(serde::Serialize)]
pub struct CrsAuditResult {
    pub files: Vec<CrsAuditEntry>,
    pub unique_crs: Vec<String>,
    pub has_mismatch: bool,
    pub warning: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CrsAuditEntry {
    pub path: String,
    pub detected_crs: String,
    pub file_type: String,
}

#[tauri::command]
pub async fn audit_crs_consistency_cmd(file_paths: Vec<String>) -> Result<CrsAuditResult, String> {
    let mut entries = Vec::new();
    for path_str in &file_paths {
        let path = std::path::PathBuf::from(path_str);
        let (crs, file_type) = if path_str.ends_with(".tif") || path_str.ends_with(".tiff") {
            match crate::formats::read_geotiff_header(&path) {
                Ok(h) => {
                    let crs = h.epsg.map(|e| format!("EPSG:{}", e)).unwrap_or_else(|| "unknown".to_string());
                    (crs, "GeoTIFF".to_string())
                }
                Err(_) => ("error".to_string(), "GeoTIFF".to_string()),
            }
        } else if path_str.ends_with(".las") || path_str.ends_with(".laz") {
            match crate::formats::read_las_header(&path) {
                Ok(h) => {
                    let crs = h.crs_wkt.unwrap_or_else(|| "unknown".to_string());
                    (crs, "LAS".to_string())
                }
                Err(_) => ("error".to_string(), "LAS".to_string()),
            }
        } else if path_str.ends_with(".shp") {
            ("shapefile-no-crs".to_string(), "Shapefile".to_string())
        } else {
            ("unknown".to_string(), "unknown".to_string())
        };
        entries.push(CrsAuditEntry {
            path: path_str.clone(),
            detected_crs: crs,
            file_type,
        });
    }

    let unique_crs: Vec<String> = entries
        .iter()
        .map(|e| e.detected_crs.clone())
        .filter(|c| c != "unknown" && c != "error" && c != "shapefile-no-crs")
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let has_mismatch = unique_crs.len() > 1;
    let warning = if has_mismatch {
        Some(format!(
            "Project has files in {} different CRSs: {}. Reproject all files to a common CRS before computing volumes or areas.",
            unique_crs.len(),
            unique_crs.join(", ")
        ))
    } else {
        None
    };

    Ok(CrsAuditResult {
        files: entries,
        unique_crs,
        has_mismatch,
        warning,
    })
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
