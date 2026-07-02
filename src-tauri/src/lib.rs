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

mod ar_companion;
mod automation;
mod commands;
#[allow(dead_code)]
mod distributed;
mod formats;
mod geodesy;
mod marine;
mod mining;
mod ml;
mod modules;
#[allow(dead_code)]
mod performance;
mod pipelines;
mod plugins;
mod report_engine;
#[allow(dead_code)]
mod streaming;
#[allow(dead_code)]
mod wasm_sandbox;

use commands::pipelines::OdmState;
use commands::{
    app_version, automation::add_scheduled_job, automation::add_watch_folder,
    automation::check_due_jobs, automation::list_scheduled_jobs, automation::list_watch_folders,
    automation::parse_pipeline_cmd, automation::remove_scheduled_job,
    automation::remove_watch_folder, automation::run_pipeline_cmd, automation::scan_watch_folders,
    automation::serialize_pipeline_cmd, get_settings, init_module, is_proj_available, list_modules,
    marine::check_s44_compliance_cmd, marine::compute_dredge_audit_cmd, marine::compute_tpu_batch,
    marine::export_s57, marine::generate_cube_surface_cmd, marine::parse_svp_cmd,
    mining::classify_ground, mining::compute_volumes_cmd,
    mining::parse_drone_manifest, ml::analyze_fragmentation_cmd, ml::classify_habitat_cmd,
    monitoring::compute_epoch_diff_cmd, monitoring::compute_progression_cmd, ping,
    pipelines::check_odm_availability, pipelines::get_odm_status, pipelines::run_odm_pipeline,
    probe_file, read_las_points_binary, read_las_points_cmd, sample_profile, save_settings,
    generate_report_cmd,
    streaming::enqueue_distributed_cube, streaming::get_coordinator_status_cmd,
    streaming::get_stream_status_cmd, streaming::merge_distributed_cube_results,
    streaming::start_coordinator_cmd, streaming::start_stream_cmd, streaming::stop_coordinator_cmd,
    streaming::stop_stream_cmd, transform_coords_cmd,
};
use modules::ModuleRegistry;
use plugins::get_supported_extensions;
use plugins::list_plugins;
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
            read_las_points_binary,
            read_las_points_cmd,
            sample_profile,
            generate_report_cmd,
            parse_drone_manifest,
            classify_ground,
            compute_volumes_cmd,
            check_odm_availability,
            run_odm_pipeline,
            get_odm_status,
            is_proj_available,
            transform_coords_cmd,
            generate_cube_surface_cmd,
            compute_tpu_batch,
            check_s44_compliance_cmd,
            export_s57,
            parse_svp_cmd,
            compute_dredge_audit_cmd,
            compute_epoch_diff_cmd,
            compute_progression_cmd,
            classify_habitat_cmd,
            analyze_fragmentation_cmd,
            list_plugins,
            get_supported_extensions,
            parse_pipeline_cmd,
            serialize_pipeline_cmd,
            run_pipeline_cmd,
            add_watch_folder,
            remove_watch_folder,
            list_watch_folders,
            scan_watch_folders,
            add_scheduled_job,
            remove_scheduled_job,
            list_scheduled_jobs,
            check_due_jobs,
            start_stream_cmd,
            stop_stream_cmd,
            get_stream_status_cmd,
            start_coordinator_cmd,
            stop_coordinator_cmd,
            get_coordinator_status_cmd,
            enqueue_distributed_cube,
            merge_distributed_cube_results,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MetaRDU Industrial application");
}
