// IPC command surface — functions exposed to the frontend via `invoke()`.
//
// Naming convention: snake_case in Rust, camelCase on the TS side via serde.

use crate::formats::{read_las_header, LasHeader};
use crate::modules::{ModuleLoadResult, ModuleRegistry};
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
    Geotiff {
        path: String,
        // TODO: parse GeoTIFF tags in Phase 1
        size_bytes: u64,
    },
    MbEs {
        path: String,
        vendor: String, // "kongsberg-all" | "reson-s7k" | "r2sonic-bsf"
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
    let path_buf = PathBuf::from(&path);
    let lower = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let size_bytes = std::fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0);

    match lower.as_str() {
        "las" => {
            let header = read_las_header(&path_buf).map_err(|e| e.to_string())?;
            Ok(FileProbeResult::Las {
                path,
                header: Box::new(header),
            })
        }
        "laz" => {
            // LAZ detection happens inside read_las_header (LasZip VLR scan)
            // but we surface a friendlier error here for the .laz extension
            Err("LAZ (compressed LAS) is not yet supported — coming in Phase 1".into())
        }
        "tif" | "tiff" => Ok(FileProbeResult::Geotiff { path, size_bytes }),
        "all" => Ok(FileProbeResult::MbEs {
            path,
            vendor: "kongsberg-all".into(),
            size_bytes,
        }),
        "s7k" => Ok(FileProbeResult::MbEs {
            path,
            vendor: "reson-s7k".into(),
            size_bytes,
        }),
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
