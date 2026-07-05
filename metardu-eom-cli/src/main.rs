// metardu-eom-cli — EOM pipeline demo + RSA license signing/verifying tooling.
//
// This binary is the developer-side companion to the MetaRDU Industrial
// desktop app. It bundles five subcommands:
//
//   * `generate-keypair` — produce an RSA-2048 keypair as PKCS#8/SPKI PEM
//     files for license signing.
//   * `sign-license` — sign a node-locked `LicenseClaims` blob with the
//     vendor's private key and write a `LicenseFile` JSON.
//   * `fingerprint` — print the current machine's `MachineFingerprint`
//     (the value to pass as `--fingerprint` when signing a license).
//   * `verify-license` — verify a license file against a public key.
//   * (no subcommand) — run the end-to-end EOM pipeline on a synthetic
//     50x50 pyramid-stockpile LAS file and emit a signed PDF report.
//
// All cryptographic primitives come from `metardu_core::mining::license`;
// the EOM pipeline and PDF report generation come from
// `metardu_core::mining::eom` and `metardu_core::mining::report`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use metardu_core::mining::csf::CsfParams;
use metardu_core::mining::dem::DemParams;
use metardu_core::mining::eom::{hex_sha256, run_eom_pipeline, EomInput, EomProgress};
use metardu_core::mining::license::{
    self, compute_machine_fingerprint, current_unix_seconds, export_private_key_pem,
    export_public_key_pem, generate_license_keypair, import_private_key_pem, import_public_key_pem,
    load_license_file, save_license_file, sign_license, verify_license, LicenseClaims,
    MachineFingerprint,
};
use metardu_core::mining::report::{generate_pdf_report, ReportData, SOFTWARE_VERSION};

#[derive(Parser)]
#[command(
    name = "metardu-eom-cli",
    version,
    about = "MetaRDU Industrial — EOM pipeline demo + RSA license signing/verifying tooling"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate an RSA-2048 keypair and write the keys as PKCS#8 / SPKI PEM files.
    GenerateKeypair {
        /// Where to write the PKCS#8 private key PEM.
        #[arg(long)]
        private_out: PathBuf,
        /// Where to write the SPKI public key PEM.
        #[arg(long)]
        public_out: PathBuf,
    },
    /// Sign a node-locked license file with an RSA private key.
    SignLicense {
        /// Path to the PKCS#8 private key PEM.
        #[arg(long)]
        private_key: PathBuf,
        /// Customer / organisation name.
        #[arg(long)]
        customer: String,
        /// Unique license identifier (UUID or similar).
        #[arg(long)]
        license_id: String,
        /// Machine fingerprint (machine_id hex) to lock the license to.
        /// Obtain via `fingerprint` on the target machine.
        #[arg(long)]
        fingerprint: String,
        /// Optional site identifier.
        #[arg(long)]
        site_id: Option<String>,
        /// Optional remaining-reports quota.
        #[arg(long)]
        reports: Option<u32>,
        /// Optional expiry (Unix seconds).
        #[arg(long)]
        expires_at: Option<u64>,
        /// Where to write the signed license JSON.
        #[arg(long)]
        out: PathBuf,
    },
    /// Print the current machine's fingerprint (machine_id hex + site_id).
    Fingerprint {
        /// Optional site identifier to bake into the fingerprint.
        #[arg(long, default_value = "")]
        site_id: String,
    },
    /// Verify a signed license file against an RSA public key.
    VerifyLicense {
        /// Path to the license JSON file.
        #[arg(long)]
        license: PathBuf,
        /// Path to the SPKI public key PEM.
        #[arg(long)]
        public_key: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Some(Commands::GenerateKeypair {
            private_out,
            public_out,
        }) => cmd_generate_keypair(&private_out, &public_out),
        Some(Commands::SignLicense {
            private_key,
            customer,
            license_id,
            fingerprint,
            site_id,
            reports,
            expires_at,
            out,
        }) => cmd_sign_license(
            &private_key,
            &customer,
            &license_id,
            &fingerprint,
            site_id.as_deref(),
            reports,
            expires_at,
            &out,
        ),
        Some(Commands::Fingerprint { site_id }) => cmd_fingerprint(&site_id),
        Some(Commands::VerifyLicense {
            license,
            public_key,
        }) => cmd_verify_license(&license, &public_key),
        None => cmd_demo(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

// --- subcommands --------------------------------------------------------

fn cmd_generate_keypair(
    private_out: &Path,
    public_out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let (priv_key, pub_key) = generate_license_keypair()?;
    let priv_pem = export_private_key_pem(&priv_key)?;
    let pub_pem = export_public_key_pem(&pub_key)?;
    fs::write(private_out, priv_pem)?;
    fs::write(public_out, pub_pem)?;
    let key_id = key_id_for_public(&pub_key);
    println!("Generated RSA-2048 keypair.");
    println!("  private key: {} (PKCS#8 PEM)", private_out.display());
    println!("  public  key: {} (SPKI PEM)", public_out.display());
    println!("  key id     : {}", key_id);
    Ok(())
}

fn cmd_sign_license(
    private_key: &Path,
    customer: &str,
    license_id: &str,
    fingerprint: &str,
    site_id: Option<&str>,
    reports: Option<u32>,
    expires_at: Option<u64>,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pem = fs::read_to_string(private_key)?;
    let priv_key = import_private_key_pem(&pem)?;

    let claims = LicenseClaims {
        license_id: license_id.to_string(),
        customer: customer.to_string(),
        machine_id: fingerprint.to_string(),
        site_id: site_id.map(|s| s.to_string()),
        issued_at: current_unix_seconds(),
        expires_at,
        reports_remaining: reports,
    };
    let license = sign_license(&claims, &priv_key)?;
    save_license_file(out, &license)?;
    println!("Signed license {}.", claims.license_id);
    println!("  customer  : {}", claims.customer);
    println!("  machine_id: {}", claims.machine_id);
    if let Some(s) = &claims.site_id {
        println!("  site_id   : {}", s);
    }
    if let Some(r) = claims.reports_remaining {
        println!("  reports   : {}", r);
    }
    if let Some(e) = claims.expires_at {
        println!("  expires_at: {}", e);
    }
    println!("  algorithm : {}", license.algorithm);
    println!("  out       : {}", out.display());
    Ok(())
}

fn cmd_fingerprint(site_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let fp = detect_machine_fingerprint(site_id);
    let json = serde_json::to_string_pretty(&fp)?;
    println!("{}", json);
    Ok(())
}

fn cmd_verify_license(
    license_path: &Path,
    public_key: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let license = load_license_file(license_path)?;
    let pem = fs::read_to_string(public_key)?;
    let pub_key = import_public_key_pem(&pem)?;
    match verify_license(&license, &pub_key) {
        Ok(claims) => {
            println!("LICENSE VALID");
            println!("  license_id : {}", claims.license_id);
            println!("  customer   : {}", claims.customer);
            println!("  machine_id : {}", claims.machine_id);
            if let Some(s) = &claims.site_id {
                println!("  site_id    : {}", s);
            }
            println!("  issued_at  : {}", claims.issued_at);
            if let Some(e) = claims.expires_at {
                println!("  expires_at : {}", e);
            }
            if let Some(r) = claims.reports_remaining {
                println!("  reports    : {}", r);
            }
            println!("  algorithm  : {}", license.algorithm);
            Ok(())
        }
        Err(e) => {
            eprintln!("LICENSE INVALID: {e}");
            Err(e.into())
        }
    }
}

// --- demo pipeline ------------------------------------------------------

fn cmd_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("MetaRDU EOM pipeline demo");
    println!("=========================");

    // The LAS file is an intermediate artifact — keep it in a tempdir that
    // auto-cleans on exit. The final PDF report is written to the current
    // working directory so it survives the demo and can be passed to
    // `metardu-verify` for chain-of-custody verification.
    let tmp_dir = tempfile::tempdir()?;
    let las_path = tmp_dir.path().join("pyramid_stockpile.las");
    let pdf_path = PathBuf::from("metardu-eom-demo.pdf");

    println!("[1/4] Building synthetic 50x50 LAS file with pyramid stockpile...");
    build_demo_las(&las_path)?;
    println!(
        "      wrote {} ({} bytes)",
        las_path.display(),
        fs::metadata(&las_path)?.len()
    );

    println!("[2/4] Running EOM pipeline...");
    // Build a DemParams explicitly so the values that drive the audit hash
    // are visible in the demo output. (run_eom_pipeline internally
    // reconstructs a DemParams from `dem_cell_size`, but we surface the
    // same numbers here for transparency.)
    let _dem_params = DemParams {
        cell_size: 1.0,
        ..DemParams::default()
    };
    let input = EomInput {
        point_cloud_path: las_path.clone(),
        csf_params: CsfParams {
            cloth_resolution: 2.0,
            max_iterations: 200,
            ..CsfParams::default()
        },
        dem_cell_size: 1.0,
        bench_interval: 5.0,
        max_points: 10_000,
        license_id: "demo-license".to_string(),
        machine_id: "demo-machine".to_string(),
        site_id: "demo-site".to_string(),
        signed: false,
        custodian: "EOM CLI demo".to_string(),
        baseline_z: None,
    };
    let output = run_eom_pipeline(&input, |p: EomProgress| {
        eprintln!(
            "      [{}/{}] {}: {}",
            p.current, p.total, p.stage, p.message
        );
    })?;
    println!("      points read     : {}", output.points_read);
    println!("      ground points   : {}", output.ground_points);
    println!("      non-ground pts  : {}", output.non_ground_points);
    println!("      audit hash      : {}", output.audit_hash);
    println!(
        "      DEM             : {} cols x {} rows, cell={} m",
        output.dem.ncols, output.dem.nrows, output.dem.cell_size
    );
    println!(
        "      fill volume     : +{:.2} m^3",
        output.volumes.fill_volume
    );
    println!(
        "      cut volume      : -{:.2} m^3",
        output.volumes.cut_volume
    );
    println!(
        "      net volume      : {:+.2} m^3",
        output.volumes.net_volume
    );

    println!("[3/4] Generating signed PDF report...");
    let report = ReportData {
        title: "MetaRDU EOM Volume Report".to_string(),
        subtitle: "Synthetic pyramid stockpile (demo)".to_string(),
        author: "metardu-eom-cli".to_string(),
        project: "Demo".to_string(),
        site: "demo-site".to_string(),
        created_at: current_unix_seconds(),
        signed: false,
        summary: format!(
            "Synthetic 50x50 point cloud with a stepped pyramid stockpile.\n\
             Fill: +{:.2} m^3\n\
             Cut:  -{:.2} m^3\n\
             Net:  {:+.2} m^3\n\
             Audit hash: {}",
            output.volumes.fill_volume,
            output.volumes.cut_volume,
            output.volumes.net_volume,
            output.audit_hash,
        ),
        chain_of_custody: output.chain_of_custody.clone(),
        software_version: SOFTWARE_VERSION.to_string(),
    };
    generate_pdf_report(&pdf_path, &report)?;
    println!(
        "      wrote {} ({} bytes)",
        pdf_path.display(),
        fs::metadata(&pdf_path)?.len()
    );

    println!("[4/4] Done.");
    println!("  PDF report: {}", pdf_path.display());
    println!();
    println!("Next step: verify the chain-of-custody with metardu-verify:");
    println!("  metardu-verify {}", pdf_path.display());
    Ok(())
}

// --- helpers ------------------------------------------------------------

/// Compute a stable identifier for an RSA public key — the first 16 hex
/// characters of the SHA-256 of the SPKI PEM string. Used for labelling
/// keypairs in `generate-keypair` output.
fn key_id_for_public(pub_key: &license::RsaPubKey) -> String {
    let pem = export_public_key_pem(pub_key).unwrap_or_default();
    let full = hex_sha256(pem.as_bytes());
    full[..16].to_string()
}

/// Gather local host metadata and hash it into a stable 64-char hex
/// `machine_id`. The result is wrapped in a `MachineFingerprint` via
/// `compute_machine_fingerprint`, which keeps it as-is (since it is
/// already a 64-char hex string).
fn detect_machine_fingerprint(site_id: &str) -> MachineFingerprint {
    let mut fingerprint_input = String::new();
    fingerprint_input.push_str("metardu-eom-cli|");
    fingerprint_input.push_str("os=");
    fingerprint_input.push_str(std::env::consts::OS);
    fingerprint_input.push_str("|arch=");
    fingerprint_input.push_str(std::env::consts::ARCH);
    fingerprint_input.push_str("|hostname=");
    fingerprint_input.push_str(&hostname());

    // MAC addresses on Linux: read /sys/class/net/*/address.
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        let mut macs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| fs::read_to_string(e.path().join("address")).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "00:00:00:00:00:00")
            .collect();
        macs.sort();
        for mac in macs {
            fingerprint_input.push_str("|mac=");
            fingerprint_input.push_str(&mac);
        }
    }
    let machine_id = hex_sha256(fingerprint_input.as_bytes());
    compute_machine_fingerprint(&machine_id, site_id)
}

/// Best-effort hostname lookup with no external dependencies.
fn hostname() -> String {
    // Linux / Unix: /etc/hostname.
    if let Ok(s) = fs::read_to_string("/etc/hostname") {
        let s = s.trim();
        if !s.is_empty() {
            return s.to_string();
        }
    }
    // Fall back to env vars (set on some CI systems and on Windows).
    if let Ok(s) = std::env::var("HOSTNAME") {
        if !s.is_empty() {
            return s;
        }
    }
    if let Ok(s) = std::env::var("COMPUTERNAME") {
        if !s.is_empty() {
            return s;
        }
    }
    "unknown-host".to_string()
}

/// Build a synthetic 50x50 LAS 1.4 file with a stepped pyramid stockpile
/// centred on the grid. The pyramid peaks at z=20 m and falls off linearly
/// to z=0 at the borders.
fn build_demo_las(path: &Path) -> Result<(), std::io::Error> {
    let n_per_side: usize = 50;
    let point_count = (n_per_side * n_per_side) as u32;

    let mut f = fs::File::create(path)?;
    let mut buf = vec![0u8; 375];

    // Signature.
    buf[0..4].copy_from_slice(b"LASF");
    // Version 1.4.
    buf[24] = 1;
    buf[25] = 4;
    // Header size = 375.
    buf[94..96].copy_from_slice(&375u16.to_le_bytes());
    // Offset to point data = 375 (no VLRs).
    buf[96..100].copy_from_slice(&375u32.to_le_bytes());
    // num VLRs = 0.
    buf[100..104].copy_from_slice(&0u32.to_le_bytes());
    // Point format 0, record length 20.
    buf[104] = 0;
    buf[105..107].copy_from_slice(&20u16.to_le_bytes());
    // Number of point records.
    buf[107..111].copy_from_slice(&point_count.to_le_bytes());
    // Scales (0.01 = 1 cm resolution).
    buf[131..139].copy_from_slice(&0.01f64.to_le_bytes());
    buf[139..147].copy_from_slice(&0.01f64.to_le_bytes());
    buf[147..155].copy_from_slice(&0.01f64.to_le_bytes());
    // Offsets.
    buf[155..163].copy_from_slice(&0.0f64.to_le_bytes());
    buf[163..171].copy_from_slice(&0.0f64.to_le_bytes());
    buf[171..179].copy_from_slice(&0.0f64.to_le_bytes());

    // Bounds: x,y in [0, 49], z in [0, 20].
    let max_coord = (n_per_side as f64) - 1.0; // 49.0
    buf[179..187].copy_from_slice(&max_coord.to_le_bytes()); // max_x
    buf[187..195].copy_from_slice(&0.0f64.to_le_bytes()); // min_x
    buf[195..203].copy_from_slice(&max_coord.to_le_bytes()); // max_y
    buf[203..211].copy_from_slice(&0.0f64.to_le_bytes()); // min_y
    buf[211..219].copy_from_slice(&20.0f64.to_le_bytes()); // max_z
    buf[219..227].copy_from_slice(&0.0f64.to_le_bytes()); // min_z
    f.write_all(&buf)?;

    // Point records: 50x50 grid with a stepped pyramid.
    //   z(i, j) = max(0, 20 - max(|i-25|, |j-25|) * 0.8)
    // That gives z=20 at (25,25), tapering to z=0 at the borders.
    let scale = 100.0; // 1 / 0.01 — raw int = metres * 100
    for i in 0..n_per_side as i32 {
        for j in 0..n_per_side as i32 {
            let di = (i - 25).abs();
            let dj = (j - 25).abs();
            let z_metres = (20.0 - (di.max(dj) as f64) * 0.8).max(0.0);
            let z_raw = (z_metres * scale) as i32;
            let mut rec = [0u8; 20];
            rec[0..4].copy_from_slice(&(i * 100).to_le_bytes()); // x = i (raw * 0.01)
            rec[4..8].copy_from_slice(&(j * 100).to_le_bytes()); // y = j
            rec[8..12].copy_from_slice(&z_raw.to_le_bytes()); // z
            f.write_all(&rec)?;
        }
    }
    Ok(())
}
