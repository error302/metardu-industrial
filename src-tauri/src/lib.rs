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

// error_context MUST be declared first because it uses #[macro_use] to
// export the ctx!() and ctx_no_input!() macros. In Rust, #[macro_use]
// only makes macros visible to modules declared TEXTUALLY AFTER the
// #[macro_use] module. All command modules use ctx!() so this must
// come before `mod commands;` and any other module that wraps errors.
#[macro_use]
mod error_context;

#[allow(dead_code)]
mod ar_companion;
mod automation;
#[allow(dead_code)]
mod benchmarks;
mod commands;
#[allow(dead_code)]
mod deliverable;
#[allow(dead_code)]
mod dem_render;
#[allow(dead_code)]
mod distributed;
mod formats;
mod geodesy;
mod i18n;
mod license;
mod marine;
mod mining;
mod ml;
mod modules;
#[allow(dead_code)]
mod performance;
mod pipelines;
mod plugin_marketplace;
#[allow(dead_code)]
mod plugins;
mod project;
mod report_engine;
mod slice_editor;
#[allow(dead_code)]
mod streaming;
mod telemetry;
#[allow(dead_code)]
mod updater;
#[allow(dead_code)]
mod wasm_sandbox;

use commands::pipelines::OdmState;
use commands::{
    app_version, automation::add_scheduled_job, automation::add_watch_folder,
    automation::check_due_jobs, automation::list_scheduled_jobs, automation::list_watch_folders,
    automation::parse_pipeline_cmd, automation::remove_scheduled_job,
    automation::remove_watch_folder, automation::run_pipeline_cmd, automation::scan_watch_folders,
    automation::serialize_pipeline_cmd, bottleneck_tools::compile_machine_control_cmd,
    bottleneck_tools::render_dem_cmd, bottleneck_tools::run_density_gates_cmd,
    bottleneck_tools::run_tidal_correction_cmd, deliverable::generate_deliverable_package_cmd,
    eom::check_license_status_cmd, eom::consume_report_cmd, eom::detect_machine_fingerprint_cmd,
    eom::generate_eom_report_cmd, eom::get_ntrip_status_cmd, eom::import_dxf_surface_cmd,
    eom::is_eom_watch_folder_running, eom::run_eom_pipeline_cmd, eom::run_triage_cmd,
    eom::start_eom_watch_folder, eom::start_ntrip_cmd, eom::stop_eom_watch_folder,
    eom::stop_ntrip_cmd, eom::verify_eom_license_cmd, generate_report_cmd, get_settings,
    init_module, is_proj_available, list_modules, marine::check_s44_compliance_cmd,
    marine::compute_cross_sections_cmd, marine::compute_dredge_audit_cmd,
    marine::compute_tpu_batch, marine::export_s57, marine::generate_cube_surface_cmd,
    marine::parse_svp_cmd, mining::classify_ground, mining::compute_volumes_cmd,
    mining::parse_drone_manifest, ml::analyze_fragmentation_cmd, ml::classify_habitat_cmd,
    monitoring::analyze_highwall_cmd, monitoring::compute_epoch_diff_cmd,
    monitoring::compute_progression_cmd, ping, pipelines::check_odm_availability,
    pipelines::get_odm_status, pipelines::run_odm_pipeline, probe_file, read_las_points_binary,
    read_las_points_cmd, sample_profile, save_settings, sprint6::accepted_indices_cmd,
    sprint6::brush_reject_cmd, sprint6::compute_target_height_cmd, sprint6::point_in_polygon_cmd,
    sprint6::read_sss_pings_cmd, sprint6::slice_by_polygon_cmd, sprint6::undo_brush_cmd,
    sprint7::activate_license_cmd, sprint7::check_feature_cmd, sprint7::get_license_status_cmd,
    sprint7::get_pending_crashes_cmd, sprint7::get_recent_events_cmd,
    sprint7::get_telemetry_config_cmd, sprint7::get_telemetry_stats_cmd,
    sprint7::init_telemetry_cmd, sprint7::mark_crash_submitted_cmd, sprint7::record_crash_cmd,
    sprint7::record_telemetry_event_cmd, sprint7::run_benchmarks_cmd,
    sprint7::update_telemetry_config_cmd, sprint8::add_file_to_project_cmd,
    sprint8::add_recent_report_cmd, sprint8::check_for_updates_cmd,
    sprint8::fetch_plugin_registry_cmd, sprint8::get_available_languages_cmd,
    sprint8::get_current_version_cmd, sprint8::get_update_status_cmd, sprint8::install_plugin_cmd,
    sprint8::list_installed_plugins_cmd, sprint8::load_project_cmd, sprint8::new_project_cmd,
    sprint8::remove_file_from_project_cmd, sprint8::save_project_cmd, sprint8::search_registry_cmd,
    sprint8::translate_cmd, sprint8::uninstall_plugin_cmd, sprint8::update_view_state_cmd,
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
        .plugin(tauri_plugin_dialog::init())
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
            compute_cross_sections_cmd,
            analyze_highwall_cmd,
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
            generate_deliverable_package_cmd,
            // Sprint 6 — SSS + 3D slice editor
            read_sss_pings_cmd,
            compute_target_height_cmd,
            slice_by_polygon_cmd,
            brush_reject_cmd,
            undo_brush_cmd,
            accepted_indices_cmd,
            point_in_polygon_cmd,
            // Sprint 7 — License + Telemetry + Benchmarks
            get_license_status_cmd,
            activate_license_cmd,
            // ⚠️ generate_license_cmd is NOT exposed via IPC — it's a
            // forge oracle that would let any frontend code (or a
            // compromised plugin) mint an Enterprise license. The
            // function is still callable from the standalone
            // metardu-license-tool binary for the sales team.
            check_feature_cmd,
            init_telemetry_cmd,
            update_telemetry_config_cmd,
            get_telemetry_config_cmd,
            record_telemetry_event_cmd,
            record_crash_cmd,
            get_telemetry_stats_cmd,
            get_recent_events_cmd,
            get_pending_crashes_cmd,
            mark_crash_submitted_cmd,
            run_benchmarks_cmd,
            // Sprint 8 — Project + Updater + i18n + Marketplace
            new_project_cmd,
            save_project_cmd,
            load_project_cmd,
            add_file_to_project_cmd,
            remove_file_from_project_cmd,
            update_view_state_cmd,
            add_recent_report_cmd,
            check_for_updates_cmd,
            get_update_status_cmd,
            get_current_version_cmd,
            translate_cmd,
            get_available_languages_cmd,
            fetch_plugin_registry_cmd,
            list_installed_plugins_cmd,
            install_plugin_cmd,
            uninstall_plugin_cmd,
            search_registry_cmd,
            // Bottleneck tools — high-value surveyor tools
            run_density_gates_cmd,
            run_tidal_correction_cmd,
            compile_machine_control_cmd,
            render_dem_cmd,
            // EOM Volumetric Auditor (commercial module v1)
            run_eom_pipeline_cmd,
            generate_eom_report_cmd,
            detect_machine_fingerprint_cmd,
            verify_eom_license_cmd,
            // ⚠️ sign_eom_license_cmd is NOT exposed via IPC — it's a
            // signing oracle + CPU-DoS vector (RSA-2048 keygen per call).
            // License signing belongs on the issuing authority only.
            check_license_status_cmd,
            consume_report_cmd,
            // EOM Watch Folder (zero-touch ingest)
            start_eom_watch_folder,
            stop_eom_watch_folder,
            is_eom_watch_folder_running,
            // DXF Design Surface Import
            import_dxf_surface_cmd,
            // Mission Data Triage
            run_triage_cmd,
            // NTRIP/RTCM3 Client
            start_ntrip_cmd,
            stop_ntrip_cmd,
            get_ntrip_status_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MetaRDU Industrial application");
}
