// IPC command surface — functions exposed to the frontend via `invoke()`.
//
// Naming convention: snake_case in Rust, camelCase on the TS side via serde.

use crate::modules::{ModuleLoadResult, ModuleRegistry};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

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
    // The Mutex here is fine because init is a short-lived operation;
    // for true parallel init across modules we'd use a RwLock or a
    // dedicated actor. Phase 0 simplicity wins.
    let registry = registry.lock().map_err(|e| e.to_string())?;
    Ok(registry.init(&id).await)
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
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("settings.json");
    let raw = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())?;
    Ok(())
}
