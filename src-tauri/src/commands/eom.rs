// EOM Volumetric Auditor IPC commands.
//
// Wires the metardu-core::mining::eom pipeline to the frontend, with
// Tauri 2 Channel<T> streaming for live progress updates.

use metardu_core::mining::csf::CsfParams;
use metardu_core::mining::dem::DemParams;
use metardu_core::mining::dxf_import::{import_dxf_surface, rasterize_dxf_to_dem, DesignDem};
use metardu_core::mining::eom::{
    run_eom_pipeline, EomInput, EomOutput, EomPipelineError, EomProgress,
};
use metardu_core::mining::license::{
    check_status, generate_license_keypair, public_key_from_pem, sign_license, verify_license,
    LicenseClaims, LicenseFile, LicenseStatus, MachineFingerprint, RsaPubKey,
};
use metardu_core::mining::report::{generate_pdf_report, ReportData};
use metardu_core::mining::report_counter::{ReportCounter, TRIAL_REPORT_QUOTA};
use std::path::PathBuf;
use tauri::ipc::Channel;

/// The vendor's RSA public key, bundled into the binary at build time.
const BUNDLED_PUBLIC_KEY_PEM: &str = include_str!("../keys/license_pub.pem");

static BUNDLED_PUBLIC_KEY: std::sync::OnceLock<Option<RsaPubKey>> = std::sync::OnceLock::new();

fn bundled_public_key() -> Result<&'static RsaPubKey, String> {
    let opt = BUNDLED_PUBLIC_KEY.get_or_init(|| public_key_from_pem(BUNDLED_PUBLIC_KEY_PEM).ok());
    opt.as_ref().ok_or_else(|| "bundled public key failed to parse".to_string())
}

/// Run the full EOM volumetric audit pipeline. Streams progress via Channel.
#[tauri::command]
pub async fn run_eom_pipeline_cmd(
    input: EomInput,
    on_progress: Channel<EomProgress>,
) -> Result<EomOutput, String> {
    let current_label = input.current_las_path.clone();
    tokio::task::spawn_blocking(move || {
        let result = run_eom_pipeline(&input, |progress| {
            let _ = on_progress.send(progress);
        });
        result.map_err(|e| ctx!("running EOM pipeline", current_label, e))
    })
    .await
    .map_err(|e| format!("run_eom_pipeline_cmd: task join error: {e}"))?
}

/// Generate the signed PDF report from an EomOutput.
#[tauri::command]
pub async fn generate_eom_report_cmd(
    report: ReportData,
    output_path: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&output_path);
        generate_pdf_report(&path, &report)
            .map_err(|e| ctx!("generating EOM PDF report", output_path, e))
    })
    .await
    .map_err(|e| format!("generate_eom_report_cmd: task join error: {e}"))?
}

/// Detect the current machine's fingerprint.
#[tauri::command]
pub async fn detect_machine_fingerprint_cmd() -> Result<MachineFingerprint, String> {
    tokio::task::spawn_blocking(MachineFingerprint::detect)
        .await
        .map_err(|e| format!("detect_machine_fingerprint_cmd: task join error: {e}"))?
}

/// Verify a license file against the bundled public key.
#[tauri::command]
pub async fn verify_eom_license_cmd(
    license: LicenseFile,
    expected_product: Option<String>,
    expected_tier: Option<String>,
) -> Result<LicenseClaims, String> {
    let pub_key = bundled_public_key()?.clone();
    let fp = MachineFingerprint::detect();
    let expected_fp = fp.fingerprint_hash.clone();
    tokio::task::spawn_blocking(move || {
        verify_license(&license, &pub_key, expected_product.as_deref(), expected_tier.as_deref(), &expected_fp)
            .map_err(|e| format!("license verification failed: {e}"))
    })
    .await
    .map_err(|e| format!("verify_eom_license_cmd: task join error: {e}"))?
}

/// Vendor-only: sign a license file (dev convenience).
#[tauri::command]
pub async fn sign_eom_license_cmd(claims: LicenseClaims) -> Result<LicenseFile, String> {
    tokio::task::spawn_blocking(move || {
        let (priv_key, _pub_key) = generate_license_keypair();
        sign_license(claims, &priv_key).map_err(|e| format!("sign_eom_license_cmd: {e}"))
    })
    .await
    .map_err(|e| format!("sign_eom_license_cmd: task join error: {e}"))?
}

/// Check the current license status for the EOM Auditor.
#[tauri::command]
pub async fn check_license_status_cmd(license: Option<LicenseFile>) -> Result<LicenseStatus, String> {
    let pub_key = bundled_public_key()?.clone();
    let fp = MachineFingerprint::detect();
    let expected_fp = fp.fingerprint_hash.clone();
    let status: LicenseStatus = tokio::task::spawn_blocking(move || {
        let counter = ReportCounter::load().unwrap_or_default();
        let status = check_status(license.as_ref(), &pub_key, "eom-volumetric-auditor", &expected_fp, TRIAL_REPORT_QUOTA);
        match status {
            LicenseStatus::Active { customer, license_id, tier, expires_at, reports_remaining } => {
                let actual_remaining = reports_remaining.map(|n| {
                    let consumed = counter.consumed_for(&license_id);
                    n.saturating_sub(consumed)
                });
                if let Some(0) = actual_remaining {
                    LicenseStatus::Exhausted { customer, license_id }
                } else {
                    LicenseStatus::Active { customer, license_id, tier, expires_at, reports_remaining: actual_remaining }
                }
            }
            LicenseStatus::Trial { .. } => {
                let consumed = counter.consumed_for("trial");
                let remaining = TRIAL_REPORT_QUOTA.saturating_sub(consumed);
                LicenseStatus::Trial { trial_reports_remaining: remaining }
            }
            other => other,
        }
    })
    .await
    .map_err(|e| format!("check_license_status_cmd: task join error: {e}"))?;
    Ok(status)
}

/// Consume one report against the current license (or trial quota).
#[tauri::command]
pub async fn consume_report_cmd(license: Option<LicenseFile>) -> Result<LicenseStatus, String> {
    let pub_key = bundled_public_key()?.clone();
    let fp = MachineFingerprint::detect();
    let expected_fp = fp.fingerprint_hash.clone();
    let status: LicenseStatus = tokio::task::spawn_blocking(move || {
        let license_id = license.as_ref()
            .map(|l| l.claims.license_id.clone())
            .unwrap_or_else(|| "trial".to_string());
        if let Ok(mut counter) = ReportCounter::load() {
            let _ = counter.increment(&license_id);
        }
        let counter = ReportCounter::load().unwrap_or_default();
        let status = check_status(license.as_ref(), &pub_key, "eom-volumetric-auditor", &expected_fp, TRIAL_REPORT_QUOTA);
        match status {
            LicenseStatus::Active { customer, license_id, tier, expires_at, reports_remaining } => {
                let actual_remaining = reports_remaining.map(|n| {
                    let consumed = counter.consumed_for(&license_id);
                    n.saturating_sub(consumed)
                });
                if let Some(0) = actual_remaining {
                    LicenseStatus::Exhausted { customer, license_id }
                } else {
                    LicenseStatus::Active { customer, license_id, tier, expires_at, reports_remaining: actual_remaining }
                }
            }
            LicenseStatus::Trial { .. } => {
                let consumed = counter.consumed_for("trial");
                let remaining = TRIAL_REPORT_QUOTA.saturating_sub(consumed);
                LicenseStatus::Trial { trial_reports_remaining: remaining }
            }
            other => other,
        }
    })
    .await
    .map_err(|e| format!("consume_report_cmd: task join error: {e}"))?;
    Ok(status)
}

/// Import a DXF design surface and rasterize it to a DEM grid.
#[tauri::command]
pub async fn import_dxf_surface_cmd(path: String, cell_size: f64) -> Result<DesignDem, String> {
    let path_label = path.clone();
    tokio::task::spawn_blocking(move || {
        let dxf_path = PathBuf::from(&path);
        let surface = import_dxf_surface(&dxf_path)
            .map_err(|e| ctx!("importing DXF surface", path_label, e))?;
        let dem = rasterize_dxf_to_dem(&surface, cell_size, None)
            .map_err(|e| ctx_no_input!("rasterizing DXF surface", e))?;
        Ok(dem)
    })
    .await
    .map_err(|e| format!("import_dxf_surface_cmd: task join error: {e}"))?
}

// ── EOM Watch Folder — zero-touch ingest ──

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EomWatchFolderConfig {
    pub path: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default)]
    pub csf_params: CsfParams,
    #[serde(default)]
    pub dem_params: DemParams,
    #[serde(default = "default_bench_interval")]
    pub bench_interval: f64,
    #[serde(default)]
    pub reference_flat_elevation: f64,
    #[serde(default)]
    pub customer: String,
    #[serde(default)]
    pub site: String,
    #[serde(default)]
    pub surveyor: String,
}

fn default_poll_interval() -> u64 { 5 }
fn default_bench_interval() -> f64 { 5.0 }

#[derive(Debug, Clone, Serialize)]
pub struct EomWatchEvent {
    pub kind: String,
    pub file_path: String,
    pub report_path: Option<String>,
    pub fill_volume: Option<f64>,
    pub cut_volume: Option<f64>,
    pub net_volume: Option<f64>,
    pub error: Option<String>,
    pub processing_time_ms: Option<u64>,
}

static EOM_WATCH_RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub async fn start_eom_watch_folder(app: AppHandle, config: EomWatchFolderConfig) -> Result<(), String> {
    if EOM_WATCH_RUNNING.load(Ordering::SeqCst) {
        return Err("EOM watch folder is already running".to_string());
    }
    EOM_WATCH_RUNNING.store(true, Ordering::SeqCst);

    let path = config.path.clone();
    let poll_interval = config.poll_interval_secs;
    let csf_params = config.csf_params.clone();
    let dem_params = config.dem_params.clone();
    let bench_interval = config.bench_interval;
    let reference_elevation = config.reference_flat_elevation;
    let customer = config.customer.clone();
    let site = config.site.clone();
    let surveyor = config.surveyor.clone();

    let watch_path = PathBuf::from(&path);
    if !watch_path.is_dir() {
        EOM_WATCH_RUNNING.store(false, Ordering::SeqCst);
        return Err(format!("watch folder does not exist: {}", path));
    }

    let seen_files: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));

    tokio::task::spawn_blocking(move || {
        let watch_path = std::path::PathBuf::from(&path);
        while EOM_WATCH_RUNNING.load(Ordering::SeqCst) {
            let new_files = scan_for_new_files(&watch_path, &seen_files);
            for file_path in new_files {
                let _ = app.emit("eom://watch", EomWatchEvent {
                    kind: "started".to_string(), file_path: file_path.clone(),
                    report_path: None, fill_volume: None, cut_volume: None,
                    net_volume: None, error: None, processing_time_ms: None,
                });
                let input = EomInput {
                    current_las_path: file_path.clone(),
                    previous_las_path: None,
                    reference_flat_elevation: reference_elevation,
                    csf_params: csf_params.clone(),
                    dem_params: dem_params.clone(),
                    bench_interval, max_points: 0,
                };
                match run_eom_pipeline(&input, |_| {}) {
                    Ok(output) => {
                        let report_path = format!("{}_eom_report.pdf",
                            file_path.rsplit_once('.').map(|(base, _)| base).unwrap_or(&file_path));
                        let report_data = ReportData {
                            eom_output: output.clone(),
                            customer: customer.clone(), site: site.clone(),
                            surveyor: surveyor.clone(),
                            report_date: chrono_today_iso(),
                            software_version: "0.1.0".to_string(),
                            signed: true,
                        };
                        match generate_pdf_report(std::path::Path::new(&report_path), &report_data) {
                            Ok(()) => { let _ = app.emit("eom://watch", EomWatchEvent {
                                kind: "completed".to_string(), file_path: file_path.clone(),
                                report_path: Some(report_path),
                                fill_volume: Some(output.volumes.fill_volume),
                                cut_volume: Some(output.volumes.cut_volume),
                                net_volume: Some(output.volumes.net_volume),
                                error: None, processing_time_ms: Some(output.processing_time_ms),
                            });}
                            Err(e) => { let _ = app.emit("eom://watch", EomWatchEvent {
                                kind: "failed".to_string(), file_path: file_path.clone(),
                                report_path: None, fill_volume: None, cut_volume: None,
                                net_volume: None, error: Some(format!("PDF generation failed: {e}")),
                                processing_time_ms: None,
                            });}
                        }
                    }
                    Err(e) => { let _ = app.emit("eom://watch", EomWatchEvent {
                        kind: "failed".to_string(), file_path: file_path.clone(),
                        report_path: None, fill_volume: None, cut_volume: None,
                        net_volume: None, error: Some(format!("Pipeline failed: {e}")),
                        processing_time_ms: None,
                    });}
                }
            }
            std::thread::sleep(Duration::from_secs(poll_interval));
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn stop_eom_watch_folder() -> Result<(), String> {
    EOM_WATCH_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn is_eom_watch_folder_running() -> Result<bool, String> {
    Ok(EOM_WATCH_RUNNING.load(Ordering::SeqCst))
}

fn scan_for_new_files(
    watch_path: &std::path::Path,
    seen_files: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
) -> Vec<String> {
    let mut new_files = Vec::new();
    let entries = match std::fs::read_dir(watch_path) { Ok(e) => e, Err(_) => return new_files };
    let now = std::time::SystemTime::now();
    let mut seen = seen_files.lock().unwrap();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_file() { continue; }
        let ext = entry_path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_default();
        if ext != "las" && ext != "laz" { continue; }
        let file_key = entry_path.display().to_string();
        if seen.contains(&file_key) { continue; }
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if now.duration_since(modified).unwrap_or(Duration::from_secs(0)).as_secs() < 2 { continue; }
            }
        }
        seen.insert(file_key.clone());
        new_files.push(file_key);
    }
    new_files
}

fn chrono_today_iso() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = epoch_to_ymd(secs);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn epoch_to_ymd(secs: u64) -> (u32, u32, u32) {
    let days = (secs / 86400) as i64 + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

#[allow(dead_code)]
pub type EomCmdError = EomPipelineError;
