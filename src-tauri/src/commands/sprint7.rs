// Sprint 7 — License + Telemetry + Benchmark IPC commands.

use crate::benchmarks::{run_benchmark_suite, BenchmarkSuiteResult};
use crate::license::{load_license, parse_license, LicensePayload, LicenseStatus};
use crate::telemetry::{
    get_config, get_pending_crash_dumps, get_recent_events, get_stats, init_telemetry,
    mark_crash_submitted, record_crash, record_event, update_config, CrashDump, TelemetryConfig,
    TelemetryEvent, TelemetryStats,
};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::Manager; // for app.path()

// ──────────────────────────────────────────────────────────────────
// License Manager

/// Get the current license status. Called by the frontend on startup
/// to display the license badge + gate Pro/Enterprise features.
#[tauri::command]
pub fn get_license_status_cmd(license_path: Option<String>) -> Result<LicenseStatus, String> {
    let path = match license_path {
        Some(p) if !p.is_empty() => PathBuf::from(&p),
        _ => {
            // Default location: app data dir / metardu-license.json
            // For Phase 7 we check a few common locations
            let candidates = [
                PathBuf::from("metardu-license.json"),
                PathBuf::from("/tmp/metardu-license.json"),
                PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".metardu")
                    .join("license.json"),
            ];
            match candidates.iter().find(|p| p.exists()) {
                Some(p) => p.clone(),
                None => return Ok(LicenseStatus::default()),
            }
        }
    };

    match load_license(&path) {
        Ok(status) => Ok(status),
        Err(e) => Ok(LicenseStatus {
            valid: false,
            tier: crate::license::LicenseTier::Core,
            payload: None,
            days_remaining: None,
            expired: false,
            error: Some(e.to_string()),
            unlocked_features: LicenseStatus::core_features(),
        }),
    }
}

/// Activate a license from a pasted license string (alternative to file path).
#[tauri::command]
pub fn activate_license_cmd(
    app: tauri::AppHandle,
    license_content: String,
    save_path: Option<String>,
) -> Result<LicenseStatus, String> {
    let status =
        parse_license(&license_content).map_err(|e| ctx_no_input!("parsing license", e))?;

    // Optionally save the license to disk for future runs.
    //
    // Security: the save_path comes from the frontend and was previously
    // passed to std::fs::write with no validation — a compromised frontend
    // could overwrite ~/.bashrc, drop a binary in ~/.local/bin/, or plant
    // a .desktop autostart entry. Now we ignore the user-supplied path
    // and always write to the app's data directory (e.g.
    // ~/.local/share/metardu-industrial/license.json on Linux,
    // %APPDATA%\metardu-industrial\license.json on Windows).
    if let Some(_user_path) = save_path {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("failed to create app data dir: {e}"))?;
        let license_path = app_data_dir.join("license.json");
        std::fs::write(&license_path, &license_content)
            .map_err(|e| ctx!("saving license to app data dir", license_path.display(), e))?;
    }

    Ok(status)
}

/// Generate a license file (admin/sales tool — not for end users).
///
/// ⚠️ NOT exposed via IPC — it's a forge oracle that would let any
/// frontend code (or a compromised plugin) mint an Enterprise
/// license. Kept as a library function so the standalone
/// `metardu-license-tool` binary can call it. See SECURITY.md.
#[allow(dead_code)]
pub fn generate_license_cmd(payload: LicensePayload) -> String {
    crate::license::generate_license_file(&payload)
}

/// Check if a specific feature is unlocked by the current license.
#[tauri::command]
pub fn check_feature_cmd(feature: String, license_path: Option<String>) -> bool {
    match get_license_status_cmd(license_path) {
        Ok(status) => status.has_feature(&feature),
        Err(_) => false,
    }
}

// ──────────────────────────────────────────────────────────────────
// Telemetry + Crash Reporter

/// Initialize telemetry at app startup with the user's saved config.
#[tauri::command]
pub fn init_telemetry_cmd(config: TelemetryConfig) {
    init_telemetry(config);
}

/// Update the telemetry config (when user toggles opt-in in Settings).
#[tauri::command]
pub fn update_telemetry_config_cmd(config: TelemetryConfig) {
    update_config(config);
}

/// Get the current telemetry config.
#[tauri::command]
pub fn get_telemetry_config_cmd() -> TelemetryConfig {
    get_config()
}

/// Record a telemetry event (called by IPC wrappers + UI components).
#[tauri::command]
pub fn record_telemetry_event_cmd(
    event_type: String,
    event_name: String,
    duration_ms: Option<u64>,
    success: bool,
    error: Option<String>,
    license_tier: String,
) {
    record_event(
        &event_type,
        &event_name,
        duration_ms,
        success,
        error.as_deref(),
        &license_tier,
    );
}

/// Record a crash dump (called from panic handlers + IPC error paths).
#[tauri::command]
pub fn record_crash_cmd(
    command: String,
    message: String,
    stack_trace: String,
    license_tier: String,
) -> String {
    record_crash(&command, &message, &stack_trace, &license_tier)
}

/// Get aggregated telemetry stats for the Settings UI.
#[tauri::command]
pub fn get_telemetry_stats_cmd() -> TelemetryStats {
    get_stats()
}

/// Get recent telemetry events (for the Settings UI diagnostic panel).
#[tauri::command]
pub fn get_recent_events_cmd(limit: Option<usize>) -> Vec<TelemetryEvent> {
    get_recent_events(limit.unwrap_or(50))
}

/// Get all pending (unsubmitted) crash dumps.
#[tauri::command]
pub fn get_pending_crashes_cmd() -> Vec<CrashDump> {
    get_pending_crash_dumps()
}

/// Mark a crash dump as submitted (after successful upload).
#[tauri::command]
pub fn mark_crash_submitted_cmd(crash_id: String) {
    mark_crash_submitted(&crash_id);
}

// ──────────────────────────────────────────────────────────────────
// Performance Benchmark Suite

#[derive(Debug, Deserialize)]
pub struct BenchmarkRequest {
    /// Number of iterations per benchmark (default 5)
    pub iterations: Option<usize>,
}

/// Run the full performance benchmark suite.
///
/// Each benchmark runs `iterations` times and reports min/max/mean/p50/p95.
/// The frontend displays the results in a "Performance Benchmark" dialog
/// so users can verify their hardware meets recommended specs.
#[tauri::command]
pub async fn run_benchmarks_cmd(request: BenchmarkRequest) -> Result<BenchmarkSuiteResult, String> {
    let iterations = request.iterations.unwrap_or(5);
    // Benchmarks are CPU-intensive — run in blocking task
    tokio::task::spawn_blocking(move || run_benchmark_suite(iterations))
        .await
        .map_err(|e| format!("run_benchmarks_cmd: task join error: {e}"))
}
