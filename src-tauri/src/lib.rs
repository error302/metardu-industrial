// MetaRDU Industrial — Tauri main entry
//
// Per ARCHITECTURE.md §2.1, the Rust core hosts all heavy processing
// (geodesy, point cloud, coordinate registration). Phase 0 keeps the core
// minimal — just the Tauri shell with the IPC bridge wired. Domain modules
// will be added incrementally per the roadmap (Phase 1: Mining MVP, etc.).

mod commands;
mod formats;
mod modules;

use commands::{
    app_version, get_settings, init_module, list_modules, ping, probe_file, save_settings,
};
use modules::ModuleRegistry;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Build the module registry — this is where Phase 1+ plugs in real
    // gdal/proj/pdal/rusqlite integrations. Today the registry returns
    // simulated status, but the IPC surface is already shaped correctly.
    let registry = Mutex::new(ModuleRegistry::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(registry)
        .invoke_handler(tauri::generate_handler![
            ping,
            app_version,
            init_module,
            list_modules,
            get_settings,
            save_settings,
            probe_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MetaRDU Industrial application");
}
