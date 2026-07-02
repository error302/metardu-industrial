// MetaRDU Industrial — Tauri main entry
//
// Per ARCHITECTURE.md §2.1, the Rust core hosts all heavy processing
// (geodesy, point cloud, coordinate registration). Phase 0 keeps the core
// minimal — just the Tauri shell with the IPC bridge wired. Domain modules
// will be added incrementally per the roadmap (Phase 1: Mining MVP, etc.).

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MetaRDU Industrial application");
}

/// Health-check command — frontend calls this to verify IPC bridge.
#[tauri::command]
fn ping() -> String {
    "metardu-industrial-core-online".into()
}

/// Returns the semantic version of the Rust core.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}
