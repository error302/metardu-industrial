// Module registry — central catalog of processing-core modules.
//
// Each module is identified by an id matching the frontend's MODULES list
// in src/screens/module-loading-screen.tsx. The registry is responsible
// for initializing them in dependency order and surfacing status to the
// frontend via IPC.
//
// In Phase 0, initialization is simulated with timing that matches the
// frontend's expectations. Real implementations will be added per the
// roadmap:
//   - geodesy   → proj crate (PROJ 9.4 bindings)
//   - raster    → gdal crate (GDAL 3.8 bindings)
//   - pointcloud → pdal-sys crate (PDAL 2.6 bindings)
//   - spatialite → rusqlite + libsqlite3-sys with spatialite feature
//   - coord-reg → ndarray + ndarray-linalg (least-squares)
//   - marine    → custom readers for .all / .s7k / .bsf
//   - mining    → custom UAV photogrammetry + ODM bindings
//   - reporting → printpdf + custom KML/DXF/S-57 writers

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::Instant;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub can_fail: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleStatus {
    Pending,
    Loading,
    Ok,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLoadResult {
    pub id: String,
    pub status: ModuleStatus,
    pub load_time_ms: u64,
    pub error: Option<String>,
}

pub struct ModuleRegistry {
    pub modules: Vec<ModuleInfo>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: vec![
                ModuleInfo {
                    id: "geodesy".into(),
                    name: "Geodesy engine".into(),
                    version: "PROJ 9.4".into(),
                    description: "Coordinate transforms, CRS management, datum shifts".into(),
                    can_fail: false,
                },
                ModuleInfo {
                    id: "raster".into(),
                    name: "Raster I/O".into(),
                    version: "GDAL 3.8".into(),
                    description: "GeoTIFF/COG read, warp, mosaic, reprojection".into(),
                    can_fail: false,
                },
                ModuleInfo {
                    id: "pointcloud".into(),
                    name: "Point cloud engine".into(),
                    version: "PDAL 2.6".into(),
                    description: "LAS/LAZ ingest, classification, ground extraction".into(),
                    can_fail: false,
                },
                ModuleInfo {
                    id: "spatialite".into(),
                    name: "Spatial index".into(),
                    version: "SpatiaLite 5.1".into(),
                    description: "Embedded local cache, project metadata, search".into(),
                    can_fail: false,
                },
                ModuleInfo {
                    id: "coord-reg".into(),
                    name: "Coordinate registry".into(),
                    version: "internal".into(),
                    description: "Least-squares adjustment, deformation tracking".into(),
                    can_fail: false,
                },
                ModuleInfo {
                    id: "marine".into(),
                    name: "Marine sonar readers".into(),
                    version: ".all / .s7k / .bsf".into(),
                    description: "Kongsberg, Reson, R2Sonic multibeam ingest".into(),
                    can_fail: true,
                },
                ModuleInfo {
                    id: "mining".into(),
                    name: "Mining drone pipelines".into(),
                    version: "DJI / SenseFly".into(),
                    description: "UAV photogrammetry ingest, ODM bindings".into(),
                    can_fail: true,
                },
                ModuleInfo {
                    id: "reporting".into(),
                    name: "Reporting engine".into(),
                    version: "internal".into(),
                    description: "PDF, KML, DXF, S-57, GeoTIFF export".into(),
                    can_fail: false,
                },
            ],
        }
    }

    /// Initialize a single module by id. Returns the load result.
    ///
    /// In Phase 0 this is a simulated load with timing close to the
    /// frontend's hardcoded durations. Real implementations will replace
    /// the match arms with actual library init code.
    pub async fn init(&self, id: &str) -> ModuleLoadResult {
        let start = Instant::now();
        let module = self.modules.iter().find(|m| m.id == id);

        let module = match module {
            Some(m) => m,
            None => {
                return ModuleLoadResult {
                    id: id.into(),
                    status: ModuleStatus::Fail,
                    load_time_ms: 0,
                    error: Some(format!("unknown module: {id}")),
                };
            }
        };

        // Simulated load times (ms) per module — matching frontend expectations.
        // Real implementations will replace these arms.
        let (load_ms, can_init): (u64, bool) = match module.id.as_str() {
            "geodesy" => (700, true),
            "raster" => (900, true),
            "pointcloud" => (800, true),
            "spatialite" => (350, true),
            "coord-reg" => (500, true),
            "marine" => (600, true),
            "mining" => (650, true),
            "reporting" => (400, true),
            _ => (0, false),
        };

        sleep(Duration::from_millis(load_ms)).await;

        let elapsed = start.elapsed().as_millis() as u64;

        if can_init {
            ModuleLoadResult {
                id: module.id.clone(),
                status: ModuleStatus::Ok,
                load_time_ms: elapsed,
                error: None,
            }
        } else {
            ModuleLoadResult {
                id: module.id.clone(),
                status: ModuleStatus::Fail,
                load_time_ms: elapsed,
                error: Some("module not yet implemented".into()),
            }
        }
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
