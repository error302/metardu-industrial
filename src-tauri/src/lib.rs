// MetaRDU Industrial — Tauri main entry
//
// Per ARCHITECTURE.md §2.1, the Rust core hosts all heavy processing
// (geodesy, point cloud, coordinate registration). Phase 0 keeps the core
// minimal — just the Tauri shell with the IPC bridge wired. Domain modules
// will be added incrementally per the roadmap (Phase 1: Mining MVP, etc.).

// unknown_lints must be allowed first so never_type_fallback (which only
// exists on newer Rust) doesn't cause a hard error on older toolchains.
#![allow(unknown_lints)]
#![allow(never_type_fallback)]

mod commands;
mod formats;
mod geodesy;
mod mining;
mod modules;
mod pipelines;

use commands::pipelines::OdmState;
use commands::{
    app_version, get_settings, init_module, is_proj_available, list_modules,
    mining::classify_ground, mining::compute_volumes_cmd, mining::parse_drone_manifest, ping,
    pipelines::check_odm_availability, pipelines::get_odm_status, pipelines::run_odm_pipeline,
    probe_file, sample_profile, save_settings, transform_coords_cmd,
};
use modules::ModuleRegistry;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Build the module registry — this is where Phase 1+ plugs in real
    // gdal/proj/pdal/rusqlite integrations. Today the registry returns
    // simulated status, but the IPC surface is already shaped correctly.
    let registry = Mutex::new(ModuleRegistry::new());
    let odm_state = Mutex::new(OdmState::default());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(registry)
        .manage(odm_state)
        .invoke_handler(tauri::generate_handler![
            ping,
            app_version,
            init_module,
            list_modules,
            get_settings,
            save_settings,
            probe_file,
            sample_profile,
            parse_drone_manifest,
            classify_ground,
            compute_volumes_cmd,
            check_odm_availability,
            run_odm_pipeline,
            get_odm_status,
            is_proj_available,
            transform_coords_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MetaRDU Industrial application");
}
