// EOM Volumetric Auditor IPC commands.
//
// Adapter layer between the frontend's expected types and the actual
// metardu-core::mining API. The core library was recreated with slightly
// different field names after a session restart — this layer translates.

use metardu_core::mining::csf::CsfParams;
use metardu_core::mining::dem::DemParams;
use metardu_core::mining::dxf_import::{import_dxf_surface, rasterize_dxf_to_dem, DesignDem};
use metardu_core::mining::eom::{run_eom_pipeline, EomInput, EomOutput, EomPipelineError, EomProgress};
use metardu_core::mining::license::{
    check_status, compute_machine_fingerprint, generate_license_keypair,
    import_public_key_pem, sign_license, verify_license, LicenseClaims, LicenseFile,
    LicenseStatus, RsaPubKey,
};
use metardu_core::mining::report::{generate_pdf_report, ReportData};
use std::path::PathBuf;
use tauri::ipc::Channel;

const BUNDLED_PUBLIC_KEY_PEM: &str = include_str!("../keys/license_pub.pem");
static BUNDLED_PUBLIC_KEY: std::sync::OnceLock<Option<RsaPubKey>> = std::sync::OnceLock::new();

fn bundled_public_key() -> Result<RsaPubKey, String> {
    BUNDLED_PUBLIC_KEY
        .get_or_init(|| import_public_key_pem(BUNDLED_PUBLIC_KEY_PEM).ok())
        .clone()
        .ok_or_else(|| "bundled public key failed to parse".to_string())
}

fn get_machine_id() -> String {
    // Match the CLI's detect_machine_fingerprint logic exactly so that
    // a license signed by the CLI verifies in the app.
    use sha2::{Digest, Sha256};
    let mut input = String::new();
    input.push_str("metardu-eom-cli|");
    input.push_str("os=");
    input.push_str(std::env::consts::OS);
    input.push_str("|arch=");
    input.push_str(std::env::consts::ARCH);
    input.push_str("|hostname=");
    // Try HOSTNAME env var (Linux), then fall back to machine name
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    input.push_str(&hostname);
    // MAC addresses on Linux: read /sys/class/net/*/address
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        let mut macs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| std::fs::read_to_string(e.path().join("address")).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "00:00:00:00:00:00")
            .collect();
        macs.sort();
        for mac in macs {
            input.push_str("|mac=");
            input.push_str(&mac);
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

#[tauri::command]
pub async fn run_eom_pipeline_cmd(
    input: EomInputAdapter,
    on_progress: Channel<EomProgress>,
) -> Result<EomOutputAdapter, String> {
    let machine_id = get_machine_id();
    let core_input = EomInput {
        point_cloud_path: PathBuf::from(&input.current_las_path),
        csf_params: input.csf_params.clone(),
        dem_cell_size: input.dem_params.cell_size,
        bench_interval: input.bench_interval,
        max_points: input.max_points,
        license_id: String::new(),
        machine_id,
        site_id: String::new(),
        signed: false,
        custodian: String::new(),
        baseline_z: Some(input.reference_flat_elevation),
    };
    let label = input.current_las_path.clone();
    tokio::task::spawn_blocking(move || {
        let result = run_eom_pipeline(&core_input, |p| { let _ = on_progress.send(p); });
        result
            .map(|o| EomOutputAdapter::from(o))
            .map_err(|e| format!("EOM pipeline failed: {} — {}", label, e))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn generate_eom_report_cmd(
    eom_output: EomOutputAdapter,
    customer: String,
    site: String,
    surveyor: String,
    output_path: String,
    signed: bool,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let report = ReportData {
            title: "MetaRDU Industrial — EOM Volumetric Report".to_string(),
            subtitle: format!("{} — {}", customer, site),
            author: surveyor.clone(),
            project: customer,
            site,
            created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            signed,
            summary: format!(
                "Fill: +{:.2} m³\nCut: -{:.2} m³\nNet: {:+.2} m³\nAudit: {}",
                eom_output.fill_volume, eom_output.cut_volume, eom_output.net_volume, eom_output.audit_hash
            ),
            chain_of_custody: metardu_core::mining::report::ChainOfCustody {
                custody_id: format!("EOM-{}", &eom_output.audit_hash[..12]),
                created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                custodian: surveyor.clone(),
                source_file: eom_output.source_file,
                source_hash: eom_output.source_hash,
                point_count: eom_output.points_read,
                ground_count: eom_output.ground_points as u64,
                fill_volume: eom_output.fill_volume,
                cut_volume: eom_output.cut_volume,
                net_volume: eom_output.net_volume,
                dem_cell_size: eom_output.dem_cell_size,
                ..Default::default()
            },
            software_version: "0.1.0".to_string(),
        };
        let path = PathBuf::from(&output_path);
        generate_pdf_report(&path, &report)
            .map_err(|e| format!("PDF generation failed: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn detect_machine_fingerprint_cmd() -> Result<FingerprintAdapter, String> {
    let machine_id = get_machine_id();
    let fp = compute_machine_fingerprint(&machine_id, "");
    Ok(FingerprintAdapter {
        machine_id: fp.machine_id.clone(),
        site_id: fp.site_id,
        fingerprint_hash: fp.machine_id.clone(), // The machine_id IS the hash
    })
}

#[tauri::command]
pub async fn verify_eom_license_cmd(
    license: LicenseFile,
    _expected_product: Option<String>,
    _expected_tier: Option<String>,
) -> Result<LicenseClaims, String> {
    let pub_key = bundled_public_key()?;
    tokio::task::spawn_blocking(move || {
        verify_license(&license, &pub_key)
            .map_err(|e| format!("license verification failed: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn sign_eom_license_cmd(claims: LicenseClaims) -> Result<LicenseFile, String> {
    tokio::task::spawn_blocking(move || {
        let (priv_key, _pub_key) = generate_license_keypair()
            .map_err(|e| format!("keypair generation failed: {e}"))?;
        sign_license(&claims, &priv_key)
            .map_err(|e| format!("license signing failed: {e}"))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn check_license_status_cmd(
    license: Option<LicenseFile>,
) -> Result<LicenseStatus, String> {
    let pub_key = bundled_public_key()?;
    let machine_id = get_machine_id();
    tokio::task::spawn_blocking(move || {
        Ok(check_status(license.as_ref(), &pub_key, &machine_id, "", 3))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn consume_report_cmd(
    _license: Option<LicenseFile>,
) -> Result<LicenseStatus, String> {
    // Simplified: just check status (the ReportCounter lives in the core module)
    let pub_key = bundled_public_key()?;
    let machine_id = get_machine_id();
    tokio::task::spawn_blocking(move || {
        Ok(check_status(None, &pub_key, &machine_id, "", 3))
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
pub async fn import_dxf_surface_cmd(path: String, cell_size: f64) -> Result<DesignDem, String> {
    let label = path.clone();
    tokio::task::spawn_blocking(move || {
        let dxf_path = PathBuf::from(&path);
        let surface = import_dxf_surface(&dxf_path)
            .map_err(|e| format!("DXF import failed: {} — {}", label, e))?;
        let dem = rasterize_dxf_to_dem(&surface, cell_size, None)
            .map_err(|e| format!("DXF rasterize failed: {}", e))?;
        Ok(dem)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ── Watch folder ──

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EomWatchFolderConfig {
    pub path: String,
    #[serde(default = "default_poll")]
    pub poll_interval_secs: u64,
    #[serde(default)]
    pub csf_params: CsfParams,
    #[serde(default)]
    pub dem_params: DemParams,
    #[serde(default = "default_bench")]
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

fn default_poll() -> u64 { 5 }
fn default_bench() -> f64 { 5.0 }

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
        return Err("already running".to_string());
    }
    EOM_WATCH_RUNNING.store(true, Ordering::SeqCst);
    let path = config.path.clone();
    let poll = config.poll_interval_secs;
    let csf = config.csf_params.clone();
    let dem_size = config.dem_params.cell_size;
    let bench = config.bench_interval;
    let watch_path = PathBuf::from(&path);
    if !watch_path.is_dir() {
        EOM_WATCH_RUNNING.store(false, Ordering::SeqCst);
        return Err(format!("folder not found: {}", path));
    }
    let seen: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    tokio::task::spawn_blocking(move || {
        let wp = std::path::PathBuf::from(&path);
        while EOM_WATCH_RUNNING.load(Ordering::SeqCst) {
            let new_files = scan_dir(&wp, &seen);
            for fp in new_files {
                let _ = app.emit("eom://watch", EomWatchEvent {
                    kind: "started".into(), file_path: fp.clone(),
                    report_path: None, fill_volume: None, cut_volume: None,
                    net_volume: None, error: None, processing_time_ms: None,
                });
                let machine_id = get_machine_id();
                let input = EomInput {
                    point_cloud_path: PathBuf::from(&fp),
                    csf_params: csf.clone(),
                    dem_cell_size: dem_size,
                    bench_interval: bench,
                    max_points: 0,
                    license_id: String::new(),
                    machine_id,
                    site_id: String::new(),
                    signed: false,
                    custodian: String::new(),
                    baseline_z: None,
                };
                match run_eom_pipeline(&input, |_| {}) {
                    Ok(output) => {
                        let rp = format!("{}_eom_report.pdf",
                            fp.rsplit_once('.').map(|(b,_)| b).unwrap_or(&fp));
                        let rd = ReportData {
                            title: "MetaRDU EOM Report".into(),
                            subtitle: config.site.clone(),
                            author: config.surveyor.clone(),
                            project: config.customer.clone(),
                            site: config.site.clone(),
                            created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                            signed: true,
                            summary: format!("Fill: {:.1} m³, Cut: {:.1} m³", output.volumes.fill_volume, output.volumes.cut_volume),
                            chain_of_custody: output.chain_of_custody.clone(),
                            software_version: "0.1.0".to_string(),
                        };
                        match generate_pdf_report(std::path::Path::new(&rp), &rd) {
                            Ok(()) => { let _ = app.emit("eom://watch", EomWatchEvent {
                                kind: "completed".into(), file_path: fp.clone(),
                                report_path: Some(rp),
                                fill_volume: Some(output.volumes.fill_volume),
                                cut_volume: Some(output.volumes.cut_volume),
                                net_volume: Some(output.volumes.net_volume),
                                error: None, processing_time_ms: None,
                            });}
                            Err(e) => { let _ = app.emit("eom://watch", EomWatchEvent {
                                kind: "failed".into(), file_path: fp.clone(),
                                report_path: None, fill_volume: None, cut_volume: None,
                                net_volume: None, error: Some(format!("PDF: {e}")),
                                processing_time_ms: None,
                            });}
                        }
                    }
                    Err(e) => { let _ = app.emit("eom://watch", EomWatchEvent {
                        kind: "failed".into(), file_path: fp.clone(),
                        report_path: None, fill_volume: None, cut_volume: None,
                        net_volume: None, error: Some(format!("Pipeline: {e}")),
                        processing_time_ms: None,
                    });}
                }
            }
            std::thread::sleep(Duration::from_secs(poll));
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

fn scan_dir(
    path: &std::path::Path,
    seen: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
) -> Vec<String> {
    let mut new_files = Vec::new();
    let entries = match std::fs::read_dir(path) { Ok(e) => e, Err(_) => return new_files };
    let now = std::time::SystemTime::now();
    let mut s = seen.lock().unwrap();
    for entry in entries.flatten() {
        let ep = entry.path();
        if !ep.is_file() { continue; }
        let ext = ep.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_default();
        if ext != "las" && ext != "laz" { continue; }
        let key = ep.display().to_string();
        if s.contains(&key) { continue; }
        if let Ok(m) = entry.metadata() {
            if let Ok(modified) = m.modified() {
                if now.duration_since(modified).unwrap_or(Duration::from_secs(0)).as_secs() < 2 { continue; }
            }
        }
        s.insert(key.clone());
        new_files.push(key);
    }
    new_files
}

// ── Adapter types — translate between frontend expectations and core API ──

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EomInputAdapter {
    pub current_las_path: String,
    pub previous_las_path: Option<String>,
    pub reference_flat_elevation: f64,
    pub csf_params: CsfParams,
    pub dem_params: DemParams,
    pub bench_interval: f64,
    pub max_points: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EomOutputAdapter {
    pub audit_hash: String,
    pub points_read: u64,
    pub ground_points: usize,
    pub non_ground_points: usize,
    pub volumes: metardu_core::mining::VolumeResult,
    pub fill_volume: f64,
    pub cut_volume: f64,
    pub net_volume: f64,
    pub cell_area: f64,
    pub fill_cells: usize,
    pub cut_cells: usize,
    pub dem_cols: usize,
    pub dem_rows: usize,
    pub dem_cell_size: f64,
    pub source_file: String,
    pub source_hash: String,
    pub processing_time_ms: u64,
    pub warnings: Vec<String>,
}

impl From<EomOutput> for EomOutputAdapter {
    fn from(o: EomOutput) -> Self {
        let coc = &o.chain_of_custody;
        Self {
            audit_hash: o.audit_hash.clone(),
            points_read: o.points_read,
            ground_points: o.ground_points,
            non_ground_points: o.non_ground_points,
            fill_volume: o.volumes.fill_volume,
            cut_volume: o.volumes.cut_volume,
            net_volume: o.volumes.net_volume,
            cell_area: o.volumes.cell_area,
            fill_cells: o.volumes.fill_cells,
            cut_cells: o.volumes.cut_cells,
            volumes: o.volumes.clone(),
            dem_cols: o.dem.ncols,
            dem_rows: o.dem.nrows,
            dem_cell_size: o.dem.cell_size,
            source_file: coc.source_file.clone(),
            source_hash: coc.source_hash.clone(),
            processing_time_ms: 0, // TODO: pass from pipeline if available
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDataAdapter {
    pub customer: String,
    pub site: String,
    pub surveyor: String,
    pub report_date: String,
    pub software_version: String,
    pub signed: bool,
    // The frontend sends the EomOutput which contains the chain_of_custody
    // For now, we create a default CoC — the real one comes from the pipeline
    #[serde(skip)]
    pub _eom_output: Option<EomOutputAdapter>,
}

impl From<ReportDataAdapter> for ReportData {
    fn from(a: ReportDataAdapter) -> Self {
        use metardu_core::mining::report::ChainOfCustody;
        ReportData {
            title: "MetaRDU Industrial — EOM Volumetric Report".to_string(),
            subtitle: format!("{} — {}", a.customer, a.site),
            author: a.surveyor,
            project: a.customer,
            site: a.site,
            created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            signed: a.signed,
            summary: String::new(),
            chain_of_custody: ChainOfCustody::default(),
            software_version: a.software_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintAdapter {
    pub machine_id: String,
    pub site_id: String,
    pub fingerprint_hash: String,
}

// ── Mission Data Triage ──

use metardu_core::triage::{run_triage, TriageReport};

/// Run triage analysis on a directory of field data files.
#[tauri::command]
pub async fn run_triage_cmd(dir: String) -> Result<TriageReport, String> {
    let dir_label = dir.clone();
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&dir);
        run_triage(&path)
            .map_err(|e| format!("triage analysis failed: {} — {}", dir_label, e))
    })
    .await
    .map_err(|e| format!("run_triage_cmd: task join error: {e}"))?
}

// ── NTRIP/RTCM3 Client ──

use metardu_core::ntrip::{NtripClient, NtripConfig, NtripStatus};
use std::sync::Mutex;

static NTRIP_CLIENT: std::sync::OnceLock<Mutex<Option<NtripClient>>> = std::sync::OnceLock::new();

fn ntrip_state() -> &'static Mutex<Option<NtripClient>> {
    NTRIP_CLIENT.get_or_init(|| Mutex::new(None))
}

/// Start the NTRIP client — connects to a caster and begins streaming RTCM corrections.
#[tauri::command]
pub async fn start_ntrip_cmd(config: NtripConfig) -> Result<NtripStatus, String> {
    // Stop any existing client
    {
        let mut state = ntrip_state().lock().map_err(|e| e.to_string())?;
        if let Some(client) = state.take() {
            client.stop();
        }
    }

    let host = config.host.clone();
    let port = config.port;
    let client = tokio::task::spawn_blocking(move || {
        NtripClient::start(config)
            .map_err(|e| format!("NTRIP connection failed to {}:{} — {}", host, port, e))
    })
    .await
    .map_err(|e| format!("start_ntrip_cmd: task join error: {e}"))??;

    let status = client.get_status();
    let mut state = ntrip_state().lock().map_err(|e| e.to_string())?;
    *state = Some(client);
    Ok(status)
}

/// Stop the NTRIP client.
#[tauri::command]
pub async fn stop_ntrip_cmd() -> Result<(), String> {
    let mut state = ntrip_state().lock().map_err(|e| e.to_string())?;
    if let Some(client) = state.take() {
        client.stop();
    }
    Ok(())
}

/// Get the current NTRIP client status.
#[tauri::command]
pub async fn get_ntrip_status_cmd() -> Result<NtripStatus, String> {
    let state = ntrip_state().lock().map_err(|e| e.to_string())?;
    Ok(match state.as_ref() {
        Some(client) => client.get_status(),
        None => NtripStatus {
            connected: false,
            mountpoint: String::new(),
            messages_received: 0,
            bytes_received: 0,
            last_message_type: None,
            last_error: None,
            uptime_secs: 0,
        },
    })
}
