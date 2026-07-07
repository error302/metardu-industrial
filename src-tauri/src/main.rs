// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Install crash recovery panic hook before anything else.
    // This ensures that if ANY panic occurs during startup or operation,
    // a crash dump is saved to app_data_dir/recovery/ for later analysis.
    metardu_industrial_lib::recovery::install_panic_hook();

    metardu_industrial_lib::run()
}
