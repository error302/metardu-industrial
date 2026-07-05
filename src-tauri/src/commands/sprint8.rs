// Sprint 8 — Project + Updater + i18n + Marketplace IPC commands.

use crate::i18n::{self, Language};
use crate::plugin_marketplace::{self, InstalledPlugin, PluginRegistry, RegistryPlugin};
use crate::project::{
    add_file_to_project, add_recent_report, load_project, new_project, remove_file_from_project,
    save_project, update_view_state, MetarduProject, ProjectFile, ViewState,
};
use crate::updater::{self, UpdateInfo, UpdateStatus};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::Manager;

// ──────────────────────────────────────────────────────────────────
// Project File Format (.metardu)

#[derive(Debug, Deserialize)]
pub struct NewProjectRequest {
    pub name: String,
    #[serde(rename = "defaultEpsg")]
    pub default_epsg: String,
    pub domain: String,
}

#[tauri::command]
pub fn new_project_cmd(request: NewProjectRequest) -> MetarduProject {
    new_project(&request.name, &request.default_epsg, &request.domain)
}

#[tauri::command]
pub fn save_project_cmd(project: MetarduProject, path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    save_project(&project, &path_buf).map_err(|e| ctx!("saving project", path, e))?;
    Ok(path)
}

#[tauri::command]
pub fn load_project_cmd(path: String) -> Result<MetarduProject, String> {
    let path_buf = PathBuf::from(&path);
    load_project(&path_buf).map_err(|e| ctx!("loading project", path, e))
}

#[tauri::command]
pub fn add_file_to_project_cmd(
    mut project: MetarduProject,
    file: ProjectFile,
) -> Result<MetarduProject, String> {
    add_file_to_project(&mut project, file);
    Ok(project)
}

#[tauri::command]
pub fn remove_file_from_project_cmd(
    mut project: MetarduProject,
    path: String,
) -> Result<MetarduProject, String> {
    remove_file_from_project(&mut project, &path);
    Ok(project)
}

#[tauri::command]
pub fn update_view_state_cmd(
    mut project: MetarduProject,
    view: ViewState,
) -> Result<MetarduProject, String> {
    update_view_state(&mut project, view);
    Ok(project)
}

#[tauri::command]
pub fn add_recent_report_cmd(
    mut project: MetarduProject,
    report_path: String,
) -> Result<MetarduProject, String> {
    add_recent_report(&mut project, &report_path);
    Ok(project)
}

// ──────────────────────────────────────────────────────────────────
// Auto-Updater — uses tauri-plugin-updater for real signed updates.

/// Check for updates. The endpoint is configured in tauri.conf.json
/// (plugins.updater.endpoints), not passed from the frontend — the
/// `endpoint` parameter is accepted for backward compatibility but
/// ignored. The plugin uses the configured endpoints automatically.
#[tauri::command]
pub async fn check_for_updates_cmd(
    app: tauri::AppHandle,
    _endpoint: Option<String>,
) -> Result<UpdateInfo, String> {
    updater::check_for_updates(&app)
        .await
        .map_err(|e| e.to_string())
}

/// Download and install the latest update (if available). The plugin
/// verifies the Ed25519 signature against the configured pubkey before
/// installing. Returns Ok(()) on success — the frontend should then
/// prompt the user to restart.
#[tauri::command]
pub async fn download_and_install_update_cmd(app: tauri::AppHandle) -> Result<(), String> {
    updater::download_and_install_update(&app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_update_status_cmd() -> UpdateStatus {
    UpdateStatus::default()
}

#[tauri::command]
pub fn get_current_version_cmd() -> String {
    env!("CARGO_PKG_VERSION").into()
}

// ──────────────────────────────────────────────────────────────────
// i18n

#[tauri::command]
pub fn translate_cmd(key: String, lang_code: String) -> String {
    let lang = Language::from_code(&lang_code).unwrap_or(Language::En);
    i18n::translate(&key, lang)
}

#[tauri::command]
pub fn get_available_languages_cmd() -> Vec<(String, String)> {
    i18n::available_languages()
        .iter()
        .map(|l| (l.code().to_string(), l.label().to_string()))
        .collect()
}

// ──────────────────────────────────────────────────────────────────
// Plugin Marketplace

#[tauri::command]
pub fn fetch_plugin_registry_cmd(source: String) -> Result<PluginRegistry, String> {
    plugin_marketplace::fetch_registry(&source).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_installed_plugins_cmd(app: tauri::AppHandle) -> Result<Vec<InstalledPlugin>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;
    plugin_marketplace::list_installed_plugins(&app_data_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_plugin_cmd(
    app: tauri::AppHandle,
    registry: PluginRegistry,
    plugin_id: String,
) -> Result<InstalledPlugin, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;
    plugin_marketplace::install_plugin(&registry, &plugin_id, &app_data_dir)
        .map_err(|e| ctx_no_input!("installing plugin", e))
}

#[tauri::command]
pub fn uninstall_plugin_cmd(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;
    plugin_marketplace::uninstall_plugin(&plugin_id, &app_data_dir)
        .map_err(|e| ctx_no_input!("uninstalling plugin", e))
}

#[tauri::command]
pub fn search_registry_cmd(registry: PluginRegistry, query: String) -> Vec<RegistryPlugin> {
    plugin_marketplace::search_registry(&registry, &query)
        .into_iter()
        .cloned()
        .collect()
}
