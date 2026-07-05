// IPC command surface — functions exposed to the frontend via `invoke()`.
//
// Naming convention: snake_case in Rust, camelCase on the TS side via serde.

pub mod automation;
pub mod bottleneck_tools;
pub mod deliverable;
pub mod eom;
pub mod marine;
pub mod mining;
pub mod ml;
pub mod monitoring;
pub mod pipelines;
pub mod sprint6;
pub mod sprint7;
pub mod sprint8;
pub mod streaming;

use crate::formats::{
    read_geotiff_header, read_kongsberg_all_header, read_las_header, read_las_points,
    read_s7k_header, sample_profile as sample_dem_profile, AllHeader, GeoTiffHeader, LasHeader,
    S7kHeader,
};
use crate::geodesy::{transform_coords, Coord, TransformResult};
use crate::modules::{ModuleLoadResult, ModuleRegistry};
use crate::report_engine::{generate_report, ReportSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

/// Health-check command — frontend calls this to verify IPC bridge.
#[tauri::command]
pub fn ping() -> String {
    "metardu-industrial-core-online".into()
}

/// Returns the semantic version of the Rust core.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

/// Initialize a single module by id. Async because real impls will do
/// nontrivial I/O (loading shared libs, opening DBs, etc.).
#[tauri::command]
pub async fn init_module(
    id: String,
    registry: State<'_, Mutex<ModuleRegistry>>,
) -> Result<ModuleLoadResult, String> {
    // Clone the module info we need so we don't hold the MutexGuard
    // across the .await — that would make the future !Send and break
    // Tauri's command handler. The registry itself is read-only after
    // construction in Phase 0; for true parallel init in Phase 1+ we'll
    // switch to an RwLock or actor model.
    let load_ms = {
        let registry = registry.lock().map_err(|e| e.to_string())?;
        if registry.find(&id).is_none() {
            return Err(format!("unknown module: {id}"));
        }
        registry.simulated_load_ms(&id)
    };
    // Run the simulated init outside the lock
    let start = std::time::Instant::now();
    tokio::time::sleep(std::time::Duration::from_millis(load_ms)).await;
    Ok(ModuleLoadResult {
        id,
        status: crate::modules::ModuleStatus::Ok,
        load_time_ms: start.elapsed().as_millis() as u64,
        error: None,
    })
}

/// List all known modules with their metadata. Used by the frontend
/// module-loading screen to render the row list dynamically.
#[tauri::command]
pub fn list_modules(
    registry: State<'_, std::sync::Mutex<ModuleRegistry>>,
) -> Result<Vec<crate::modules::ModuleInfo>, String> {
    let registry = registry.lock().map_err(|e| e.to_string())?;
    Ok(registry.modules.clone())
}

// ──────────────────────────────────────────────────────────────────
// File ingest — Phase 0 reads LAS headers; future formats to follow.

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FileProbeResult {
    Las {
        path: String,
        // Boxed to avoid large-variant clippy warning — LasHeader is ~200B,
        // other variants are <50B. Box keeps the enum discriminant small.
        header: Box<LasHeader>,
    },
    #[serde(rename = "geo-tiff")]
    Geotiff {
        path: String,
        header: Box<GeoTiffHeader>,
    },
    /// Kongsberg .all multibeam datagram file
    KongsbergAll {
        path: String,
        header: Box<AllHeader>,
    },
    /// Reson Teledyne .s7k multibeam datagram file
    ResonS7k {
        path: String,
        header: Box<S7kHeader>,
    },
    /// Other multibeam vendor formats not yet fully parsed (.bsf only)
    MbEs {
        path: String,
        vendor: String, // "r2sonic-bsf"
        size_bytes: u64,
    },
    Other {
        path: String,
        size_bytes: u64,
        note: String,
    },
}

/// Probe a file by extension + magic bytes, returning enough metadata
/// for the frontend to render the file's bounds on the map canvas.
///
/// This is the entry point for the drag-and-drop workflow:
///   1. User drops a file → frontend calls probe_file(path)
///   2. Rust reads the header → returns FileProbeResult
///   3. Frontend adds to survey store + renders bounds on canvas
#[tauri::command]
pub fn probe_file(path: String) -> Result<FileProbeResult, String> {
    // Security: validate the path before any filesystem access.
    // Rejects paths into ~/.ssh, ~/.aws, browser dirs, etc.
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating path for probe_file", path, e))?;
    let path = path_buf.to_string_lossy().to_string();
    let lower = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let size_bytes = std::fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0);

    match lower.as_str() {
        "las" => {
            let header =
                read_las_header(&path_buf).map_err(|e| ctx!("probing LAS file", path, e))?;
            Ok(FileProbeResult::Las {
                path,
                header: Box::new(header),
            })
        }
        "laz" => {
            // LAZ detection happens inside read_las_header (LasZip VLR scan)
            // but we surface a friendlier error here for the .laz extension
            Err("probe_file: LAZ (compressed LAS) is not yet supported — coming in Phase 1".into())
        }
        "tif" | "tiff" => {
            let header = read_geotiff_header(&path_buf)
                .map_err(|e| ctx!("probing GeoTIFF file", path, e))?;
            Ok(FileProbeResult::Geotiff {
                path,
                header: Box::new(header),
            })
        }
        "all" => {
            let header = read_kongsberg_all_header(&path_buf)
                .map_err(|e| ctx!("probing Kongsberg .all file", path, e))?;
            Ok(FileProbeResult::KongsbergAll {
                path,
                header: Box::new(header),
            })
        }
        "s7k" => {
            let header =
                read_s7k_header(&path_buf).map_err(|e| ctx!("probing Reson .s7k file", path, e))?;
            Ok(FileProbeResult::ResonS7k {
                path,
                header: Box::new(header),
            })
        }
        "bsf" => Ok(FileProbeResult::MbEs {
            path,
            vendor: "r2sonic-bsf".into(),
            size_bytes,
        }),
        other => Ok(FileProbeResult::Other {
            path,
            size_bytes,
            note: format!("unsupported extension: .{other}"),
        }),
    }
}

/// Read LAS point data as a packed binary buffer (f32 array) for
/// high-performance rendering. Returns raw bytes: [x0, y0, z0, x1, y1, z1, ...]
/// as little-endian f32 values. The frontend wraps this as a Float32Array
/// for direct upload to Deck.gl/WebGL — zero JSON serialization, zero GC pressure.
///
/// Each point is 3 × f32 = 12 bytes. For 1M points = 12MB (vs ~40MB JSON).
#[tauri::command]
pub fn read_las_points_binary(path: String, max_points: u64) -> Result<Vec<u8>, String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating path for read_las_points_binary", path, e))?;
    let points = read_las_points(&path_buf, max_points)
        .map_err(|e| ctx!("reading LAS points (binary path)", path, e))?;

    // Pack into f32 array: [x0, y0, z0, x1, y1, z1, ...]
    let mut buf = Vec::with_capacity(points.len() * 12);
    for (x, y, z) in &points {
        buf.extend_from_slice(&(*x as f32).to_le_bytes());
        buf.extend_from_slice(&(*y as f32).to_le_bytes());
        buf.extend_from_slice(&(*z as f32).to_le_bytes());
    }
    Ok(buf)
}

/// Read LAS point data (x, y, z tuples) as JSON. Kept for backward compat
/// but prefer read_las_points_binary for >100K points.
#[tauri::command]
pub fn read_las_points_cmd(path: String, max_points: u64) -> Result<Vec<(f64, f64, f64)>, String> {
    let path_buf = crate::path_validation::validate_path(&path)
        .map_err(|e| ctx!("validating path for read_las_points_cmd", path, e))?;
    read_las_points(&path_buf, max_points)
        .map_err(|e| ctx!("reading LAS points (JSON path)", path, e))
}

// ──────────────────────────────────────────────────────────────────
// Elevation profile — sample real elevation from a loaded GeoTIFF DEM.
//
// The frontend passes the GeoTIFF path, two endpoints in geographic
// coords (lon/lat, assumed WGS84), and the number of samples to take.
// We reproject endpoints to pixel coords using the GeoTIFF's
// ModelTiepoint + ModelPixelScale, then call bilinear-sample along the
// line and return the elevations.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSampleResult {
    /// Elevation samples (in DEM units — usually meters)
    pub elevations: Vec<f64>,
    /// Distance per sample in meters (haversine, lon/lat assumption)
    pub distances: Vec<f64>,
    /// Min/max elevation across the samples
    pub min_elevation: f64,
    pub max_elevation: f64,
    /// Whether the result came from real DEM data (true) or fell back to
    /// a synthesized placeholder (false). Frontend uses this to badge.
    pub from_real_dem: bool,
}

#[tauri::command]
pub fn sample_profile(
    path: String,
    start_lon: f64,
    start_lat: f64,
    end_lon: f64,
    end_lat: f64,
    num_samples: usize,
) -> Result<ProfileSampleResult, String> {
    let path_buf = PathBuf::from(&path);
    let header = read_geotiff_header(&path_buf).map_err(|e| e.to_string())?;

    // Convert lon/lat to pixel coords using tiepoint + scale
    // Tiepoint (i,j,k,x,y,z): pixel (i,j) corresponds to geo (x,y)
    // For DEMs: x is typically lon, y is typically lat
    let (start_px, end_px) = match (header.model_tiepoint, header.model_pixel_scale) {
        (Some(tp), Some(scale)) => {
            let tp_x = tp[3]; // geo x at tiepoint
            let tp_y = tp[4]; // geo y at tiepoint
            let tp_i = tp[0]; // pixel col at tiepoint
            let tp_j = tp[1]; // pixel row at tiepoint
            let sx = scale[0]; // geo units per pixel column
            let sy = scale[1]; // geo units per pixel row
            let start_px = (
                tp_i + (start_lon - tp_x) / sx,
                tp_j + (start_lat - tp_y) / sy,
            );
            let end_px = (tp_i + (end_lon - tp_x) / sx, tp_j + (end_lat - tp_y) / sy);
            (start_px, end_px)
        }
        _ => {
            return Err(
                "GeoTIFF lacks ModelTiepoint or ModelPixelScale — cannot sample profile".into(),
            );
        }
    };

    // Haversine distance for the meters-per-sample (assumes WGS84)
    let total_meters = haversine_meters(start_lon, start_lat, end_lon, end_lat);
    let elevations = sample_dem_profile(&path_buf, &header, start_px, end_px, num_samples)
        .map_err(|e| e.to_string())?;

    let distances: Vec<f64> = (0..num_samples)
        .map(|i| total_meters * (i as f64) / (num_samples.saturating_sub(1) as f64))
        .collect();

    let min_elevation = elevations.iter().copied().fold(f64::INFINITY, f64::min);
    let max_elevation = elevations.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    Ok(ProfileSampleResult {
        elevations,
        distances,
        min_elevation,
        max_elevation,
        from_real_dem: true,
    })
}

fn haversine_meters(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let r = 6_371_000.0_f64;
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dphi = (lat2 - lat1).to_radians();
    let dlambda = (lon2 - lon1).to_radians();
    let h = (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlambda / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

// ──────────────────────────────────────────────────────────────────
// Branded PDF Report Engine — generates professional survey reports.

/// Generate a branded HTML report (print-ready for PDF conversion).
#[tauri::command]
pub fn generate_report_cmd(spec: ReportSpec) -> Result<String, String> {
    generate_report(&spec).map_err(|e| e.to_string())?;
    Ok(spec.output_path.clone())
}

// ──────────────────────────────────────────────────────────────────
// Coordinate reprojection — conditional on 'geo' or 'geo-proj' feature.
//
// When the proj crate is enabled at build time, this delegates to real
// PROJ 9.x transformations. Otherwise returns an error so the frontend
// can fall back to displaying data in its native CRS.

/// Check whether real PROJ-backed reprojection is available in this build.
#[tauri::command]
pub fn is_proj_available() -> bool {
    crate::geodesy::is_proj_available()
}

/// Transform a batch of coordinates from one CRS to another.
#[tauri::command]
pub fn transform_coords_cmd(
    coords: Vec<Coord>,
    from_crs: String,
    to_crs: String,
) -> Result<TransformResult, String> {
    let label = format!("{} -> {}", from_crs, to_crs);
    transform_coords(&coords, &from_crs, &to_crs)
        .map_err(|e| ctx!("transforming coordinates", label, e))
}

// ──────────────────────────────────────────────────────────────────
// Settings — persisted to disk via tauri's app_data_dir.
// Schema mirrors src/stores/app-store.ts AppSettings.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(rename = "defaultDomain")]
    pub default_domain: String,
    #[serde(rename = "defaultEpsg")]
    pub default_epsg: String,
    pub density: String,
    #[serde(rename = "reducedMotion")]
    pub reduced_motion: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_domain: "both".into(),
            default_epsg: "EPSG:4326".into(),
            density: "comfortable".into(),
            reduced_motion: false,
        }
    }
}

#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("settings.json");
    let raw = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())?;
    Ok(())
}
