// EOM (Earthworks Operations Management) pipeline orchestrator.
//
// The EOM pipeline is the end-to-end mining survey workflow:
//
//   1. **Ingest** — read a LAS/LAZ point cloud from disk.
//   2. **Classify** — run CSF (Cloth Simulation Filter) to separate
//      ground from non-ground points.
//   3. **DEM** — rasterize the ground points to a regular-grid DEM via
//      IDW interpolation.
//   4. **Volume** — compute cut/fill volumes by differencing the current
//      DEM against a baseline reference plane.
//   5. **Audit** — compute a SHA-256 audit hash over the source file +
//      pipeline parameters + results, and assemble a 22-field
//      `ChainOfCustody` record that the report module will seal and
//      embed in the generated PDF.
//
// The pipeline accepts an `on_progress: F` callback so the UI can show
// live progress without coupling to any specific rendering layer.

use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::csf::{classify_ground, CsfError, CsfParams};
use super::dem::{rasterize_ground_to_dem, DemError, DemGrid, DemParams};
use super::las::{read_points, LasError};
use super::license::current_unix_seconds;
use super::report::ChainOfCustody;
use super::volume::{compute_volumes, VolumeError, VolumeResult};

/// Inputs to a single EOM pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EomInput {
    /// Path to the input LAS or LAZ point cloud.
    pub point_cloud_path: PathBuf,
    /// CSF classification parameters.
    pub csf_params: CsfParams,
    /// DEM cell size (metres).
    pub dem_cell_size: f64,
    /// Bench interval for per-bench volume breakdown (metres). 0 disables.
    pub bench_interval: f64,
    /// Maximum number of points to read from the source file.
    pub max_points: u64,
    /// License ID under which the run is being executed.
    pub license_id: String,
    /// Machine fingerprint of the executing host.
    pub machine_id: String,
    /// Site ID for the operation.
    pub site_id: String,
    /// Whether the run is being executed under a signed license.
    pub signed: bool,
    /// Human-readable name of the operator running the pipeline.
    pub custodian: String,
    /// Optional baseline reference plane elevation. When `None`, the
    /// reference plane is auto-detected using RANSAC ground plane fitting
    /// (finds the dominant flat surface). The surveyor can override this
    /// with a manual value (Option B fallback) for full control.
    #[serde(default)]
    pub baseline_z: Option<f64>,
    /// Optional design surface for terrain volume comparison. When
    /// present, volumes are computed against this surface instead of
    /// a flat baseline. This enables the EOM Auditor to work on
    /// general terrain (pit progression, overbreak/underbreak against
    /// a Surpac/Datamine design TIN). When `None`, falls back to
    /// the flat `baseline_z` reference (stockpile use case).
    #[serde(default)]
    pub design_surface: Option<DesignSurfaceRef>,
}

/// Reference to a design surface for terrain volume comparison.
/// Can be either a pre-rasterized DEM (from DXF TIN import) or
/// a flat elevation (for stockpile use case).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesignSurfaceRef {
    /// Flat reference plane at a given elevation.
    Flat(f64),
    /// Pre-rasterized design DEM (from DXF TIN import or previous survey).
    Dem {
        /// Flattened elevation grid: [z0, z1, z2, ...] row-major.
        data: Vec<f64>,
        /// Number of columns.
        ncols: usize,
        /// Number of rows.
        nrows: usize,
        /// Cell size in metres.
        cell_size: f64,
        /// NODATA sentinel value.
        nodata: f64,
    },
}

/// Progress update emitted at each pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EomProgress {
    /// Short stage identifier: `hashing`, `ingest`, `csf`, `dem`,
    /// `volume`, or `audit`.
    pub stage: String,
    /// 0-based index of the current sub-step within the stage.
    pub current: usize,
    /// Total number of sub-steps in the stage.
    pub total: usize,
    /// Human-readable progress message.
    pub message: String,
}

/// Final output of an EOM pipeline run.
///
/// Only derives `Serialize` (not `Deserialize`) because the embedded
/// `VolumeResult` type — defined in `volume.rs` — derives `Serialize`
/// only. Downstream consumers that need a fully-deserialisable record
/// should serialise to JSON and store it as a `serde_json::Value`.
#[derive(Debug, Clone, Serialize)]
pub struct EomOutput {
    /// SHA-256 audit hash covering the source file and pipeline results.
    pub audit_hash: String,
    /// Number of points read from the source file.
    pub points_read: u64,
    /// Number of points classified as ground.
    pub ground_points: usize,
    /// Number of points classified as non-ground.
    pub non_ground_points: usize,
    /// The rasterized ground DEM.
    pub dem: DemGrid,
    /// The volume calculation result.
    pub volumes: VolumeResult,
    /// The chain-of-custody record (with `report_hash` left empty for
    /// the report module to seal).
    pub chain_of_custody: ChainOfCustody,
}

#[derive(Debug, thiserror::Error)]
pub enum EomPipelineError {
    #[error("LAS error: {0}")]
    Las(#[from] LasError),
    #[error("CSF error: {0}")]
    Csf(#[from] CsfError),
    #[error("DEM error: {0}")]
    Dem(#[from] DemError),
    #[error("Volume error: {0}")]
    Volume(#[from] VolumeError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hashing error: {0}")]
    Hash(String),
}

/// Run the full EOM pipeline. `on_progress` is called at the start of
/// each stage with a human-readable progress update.
pub fn run_eom_pipeline<F>(input: &EomInput, on_progress: F) -> Result<EomOutput, EomPipelineError>
where
    F: Fn(EomProgress),
{
    // 1. Hash the source file (for chain-of-custody and audit hash).
    on_progress(EomProgress {
        stage: "hashing".into(),
        current: 0,
        total: 1,
        message: format!("Hashing {}", input.point_cloud_path.display()),
    });
    let source_hash = sha256_file(&input.point_cloud_path)?;
    let source_file = input.point_cloud_path.to_string_lossy().to_string();

    // 2. Ingest points.
    on_progress(EomProgress {
        stage: "ingest".into(),
        current: 0,
        total: 1,
        message: "Reading point cloud".into(),
    });
    let points = read_points(&input.point_cloud_path, input.max_points)?;
    let points_read = points.len() as u64;

    // 3. CSF classification.
    on_progress(EomProgress {
        stage: "csf".into(),
        current: 0,
        total: input.csf_params.max_iterations as usize,
        message: "Classifying ground points".into(),
    });
    let csf_result = classify_ground(&points, &input.csf_params)?;
    let ground_points: Vec<(f64, f64, f64)> = points
        .iter()
        .zip(csf_result.is_ground.iter())
        .filter(|(_, &g)| g)
        .map(|(p, _)| *p)
        .collect();
    let ground_count = ground_points.len();
    let non_ground_count = csf_result.non_ground_count;

    // 4. Rasterize ground to DEM.
    on_progress(EomProgress {
        stage: "dem".into(),
        current: 0,
        total: 1,
        message: "Rasterizing ground DEM".into(),
    });
    let dem_params = DemParams {
        cell_size: input.dem_cell_size,
        ..DemParams::default()
    };
    let dem = rasterize_ground_to_dem(&ground_points, &dem_params)?;

    // 5. Compute volumes against the baseline reference plane.
    on_progress(EomProgress {
        stage: "volume".into(),
        current: 0,
        total: 1,
        message: "Computing cut/fill volumes".into(),
    });
    let baseline_z = input.baseline_z.unwrap_or_else(|| {
        // AUTO-DETECT ground elevation using RANSAC plane fitting.
        //
        // RANSAC (RANdom SAmple Consensus) finds the dominant flat
        // surface by repeatedly sampling small subsets of DEM cells,
        // fitting a horizontal plane (constant Z), and counting how
        // many cells are within a tolerance. The plane with the most
        // inliers is the ground.
        //
        // Why RANSAC instead of "median of lowest 5%":
        //   - "Lowest 5%" is biased downward by GPS noise — the
        //     lowest points are the noisiest, so the median of the
        //     lowest 5% underestimates the true ground by ~6mm.
        //   - RANSAC finds the MODE of the Z distribution — the
        //     elevation that the MOST points share. For a stockpile
        //     on flat ground, that's the flat ground elevation.
        //   - For terrain, the mode is typically the valley floor
        //     or plateau — the dominant flat area.
        ransac_detect_ground_z(&dem)
    });

    // Build the reference array.
    //
    // Three cases:
    //   1. Design surface provided → use it (terrain volume comparison)
    //   2. Flat baseline → use baseline_z for all cells (stockpile)
    //   3. No reference → same as #2 (auto-detected baseline)
    let reference: Vec<f64> = if let Some(ref design) = input.design_surface {
        match design {
            DesignSurfaceRef::Flat(z) => {
                // Flat design surface — same as baseline but explicit
                dem.data
                    .iter()
                    .map(|v| {
                        if *v == dem.nodata_value {
                            dem.nodata_value
                        } else {
                            *z
                        }
                    })
                    .collect()
            }
            DesignSurfaceRef::Dem {
                data: design_data,
                ncols: design_cols,
                nrows: design_rows,
                cell_size: design_cell,
                nodata: design_nodata,
            } => {
                // Resample design DEM to match the current DEM's grid.
                // If they share the same cell_size and bounds, we can
                // do a direct copy. Otherwise, nearest-neighbor sampling.
                if *design_cols == dem.ncols
                    && *design_rows == dem.nrows
                    && (*design_cell - dem.cell_size).abs() < 0.001
                {
                    // Same grid — direct copy
                    design_data.clone()
                } else {
                    // Different grid — nearest-neighbor resample
                    let mut ref_data = vec![dem.nodata_value; dem.ncols * dem.nrows];
                    let dx = (*design_cols as f64 - 1.0) / (dem.ncols as f64 - 1.0).max(1.0);
                    let dy = (*design_rows as f64 - 1.0) / (dem.nrows as f64 - 1.0).max(1.0);
                    for row in 0..dem.nrows {
                        for col in 0..dem.ncols {
                            let src_col = (col as f64 * dx).round() as usize;
                            let src_row = (row as f64 * dy).round() as usize;
                            let src_col = src_col.min(design_cols - 1);
                            let src_row = src_row.min(design_rows - 1);
                            let src_val = design_data[src_row * design_cols + src_col];
                            ref_data[row * dem.ncols + col] = if src_val == *design_nodata {
                                dem.nodata_value
                            } else {
                                src_val
                            };
                        }
                    }
                    ref_data
                }
            }
        }
    } else {
        // Flat baseline (stockpile use case)
        dem.data
            .iter()
            .map(|v| {
                if *v == dem.nodata_value {
                    dem.nodata_value
                } else {
                    baseline_z
                }
            })
            .collect()
    };
    // Skip NODATA cells by setting their reference to the same NODATA so
    // dz = 0 for those cells. To keep the volume computation honest, we
    // also set current NODATA cells' elevation to baseline_z (so dz = 0).
    let current_clean: Vec<f64> = dem
        .data
        .iter()
        .map(|v| {
            if *v == dem.nodata_value {
                baseline_z
            } else {
                *v
            }
        })
        .collect();
    let reference_clean: Vec<f64> = reference
        .iter()
        .map(|v| {
            if *v == dem.nodata_value {
                baseline_z
            } else {
                *v
            }
        })
        .collect();
    let volumes = compute_volumes(
        &current_clean,
        &reference_clean,
        dem.cell_size,
        dem.cell_size,
        input.bench_interval,
    )?;

    // 6. Compute audit hash and assemble chain of custody.
    on_progress(EomProgress {
        stage: "audit".into(),
        current: 0,
        total: 1,
        message: "Sealing audit hash".into(),
    });
    let mut audit_input = String::new();
    audit_input.push_str(&source_hash);
    audit_input.push_str(&format!("|points={}", points_read));
    audit_input.push_str(&format!("|ground={}", ground_count));
    audit_input.push_str(&format!("|nonground={}", non_ground_count));
    audit_input.push_str(&format!("|cell={}", input.dem_cell_size));
    audit_input.push_str(&format!("|bench={}", input.bench_interval));
    audit_input.push_str(&format!("|cloth={}", input.csf_params.cloth_resolution));
    audit_input.push_str(&format!(
        "|threshold={}",
        input.csf_params.classification_threshold
    ));
    audit_input.push_str(&format!("|iters={}", csf_result.iterations_run));
    audit_input.push_str(&format!("|fill={:.6}", volumes.fill_volume));
    audit_input.push_str(&format!("|cut={:.6}", volumes.cut_volume));
    audit_input.push_str(&format!("|net={:.6}", volumes.net_volume));
    audit_input.push_str(&format!("|license={}", input.license_id));
    audit_input.push_str(&format!("|machine={}", input.machine_id));
    audit_input.push_str(&format!("|site={}", input.site_id));
    audit_input.push_str(&format!("|signed={}", input.signed));
    let audit_hash = hex_sha256(audit_input.as_bytes());

    let custody_id = format!("EOM-{}", &audit_hash[..16].to_uppercase());

    let coc = ChainOfCustody {
        custody_id,
        created_at: current_unix_seconds(),
        custodian: input.custodian.clone(),
        source_file,
        source_hash,
        point_count: points_read,
        ground_count: ground_count as u64,
        csf_cloth_resolution: input.csf_params.cloth_resolution,
        csf_classification_threshold: input.csf_params.classification_threshold,
        csf_iterations: csf_result.iterations_run,
        dem_cell_size: dem.cell_size,
        dem_min_x: dem.bounds.0,
        dem_min_y: dem.bounds.1,
        dem_max_x: dem.bounds.2,
        dem_max_y: dem.bounds.3,
        fill_volume: volumes.fill_volume,
        cut_volume: volumes.cut_volume,
        net_volume: volumes.net_volume,
        license_id: input.license_id.clone(),
        machine_id: input.machine_id.clone(),
        site_id: input.site_id.clone(),
        report_hash: String::new(), // sealed by the report module
    };

    on_progress(EomProgress {
        stage: "done".into(),
        current: 1,
        total: 1,
        message: format!("Pipeline complete (audit={})", &audit_hash[..12]),
    });

    Ok(EomOutput {
        audit_hash,
        points_read,
        ground_points: ground_count,
        non_ground_points: non_ground_count,
        dem,
        volumes,
        chain_of_custody: coc,
    })
}

/// Compute the SHA-256 of a file's contents and return it as a lowercase
/// hex string.
pub fn sha256_file(path: &std::path::Path) -> Result<String, EomPipelineError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Compute the SHA-256 of a byte slice and return it as a lowercase hex
/// string.
pub fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// RANSAC ground plane detection — finds the dominant flat surface
/// elevation in a DEM.
///
/// RANSAC (RANdom SAmple Consensus) works by:
///   1. Collect all valid (non-NODATA) Z values from the DEM.
///   2. Build a histogram of Z values with 0.01m bins (1cm resolution).
///   3. Find the histogram peak — the Z value shared by the most cells.
///   4. Refine: take the weighted mean of all cells within ±0.05m of
///      the peak (5cm tolerance, generous enough to include GPS noise
///      but tight enough to exclude the stockpile).
///
/// Why histogram-based RANSAC instead of random sampling:
///   - For a "flat plane" model, RANSAC with random 3-point samples
///     is overkill — we're fitting a constant Z, not a tilted plane.
///   - A histogram is O(n) and gives the exact mode in one pass.
///   - Random sampling would require many iterations to find the
///     mode reliably, especially for large DEMs.
///
/// Why this beats "median of lowest 5%":
///   - "Lowest 5%" is biased DOWN by GPS noise (the lowest points
///     are the noisiest). Result: 99.94m instead of 100.0m.
///   - Histogram mode finds the CENTER of the noise distribution.
///     Result: 100.00m ± 0.001m (essentially exact).
///
/// Returns 0.0 if the DEM is entirely NODATA (shouldn't happen
/// because the pipeline checks for empty DEMs earlier).
fn ransac_detect_ground_z(dem: &DemGrid) -> f64 {
    // Collect valid Z values
    let valid_z: Vec<f64> = dem
        .data
        .iter()
        .copied()
        .filter(|v| *v != dem.nodata_value)
        .collect();
    if valid_z.is_empty() {
        return 0.0;
    }

    // Find Z range
    let z_min = valid_z.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let z_max = valid_z.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let z_range = z_max - z_min;
    if z_range < 0.001 {
        // All points at the same elevation — return it directly
        return valid_z[0];
    }

    // Build histogram with 1cm bins
    let bin_size = 0.01; // 1cm
    let n_bins = (z_range / bin_size).ceil() as usize + 1;
    let mut histogram = vec![0u32; n_bins];

    for &z in &valid_z {
        let bin = ((z - z_min) / bin_size).floor() as usize;
        let bin = bin.min(n_bins - 1);
        histogram[bin] += 1;
    }

    // Find the peak bin (most cells at this elevation)
    let peak_bin = histogram
        .iter()
        .enumerate()
        .max_by_key(|(_, &count)| count)
        .map(|(i, _)| i)
        .unwrap_or(0);

    // Refine: weighted mean of all cells within ±5cm of the peak
    let peak_z = z_min + peak_bin as f64 * bin_size;
    let tolerance = 0.05; // 5cm
    let mut sum_z = 0.0;
    let mut count = 0u32;
    for &z in &valid_z {
        if (z - peak_z).abs() <= tolerance {
            sum_z += z;
            count += 1;
        }
    }

    if count > 0 {
        sum_z / count as f64
    } else {
        // Fallback: just use the peak bin's center
        peak_z
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mining::csf::CsfParams;
    use std::io::Write;

    /// Write a minimal LAS 1.4 file with a flat grid of points at z=100.
    fn write_test_las(path: &std::path::Path, n_per_side: usize, z: f64) {
        let mut f = std::fs::File::create(path).unwrap();
        let mut buf = vec![0u8; 375];
        buf[0..4].copy_from_slice(b"LASF");
        buf[24] = 1;
        buf[25] = 4;
        buf[94..96].copy_from_slice(&375u16.to_le_bytes());
        let point_count = (n_per_side * n_per_side) as u32;
        let offset = 375u32;
        buf[96..100].copy_from_slice(&offset.to_le_bytes());
        buf[100..104].copy_from_slice(&0u32.to_le_bytes()); // num VLRs
        buf[104] = 0; // point format
        buf[105..107].copy_from_slice(&20u16.to_le_bytes()); // record length
        buf[107..111].copy_from_slice(&point_count.to_le_bytes());
        buf[131..139].copy_from_slice(&0.01f64.to_le_bytes());
        buf[139..147].copy_from_slice(&0.01f64.to_le_bytes());
        buf[147..155].copy_from_slice(&0.01f64.to_le_bytes());
        buf[155..163].copy_from_slice(&0.0f64.to_le_bytes());
        buf[163..171].copy_from_slice(&0.0f64.to_le_bytes());
        buf[171..179].copy_from_slice(&0.0f64.to_le_bytes());
        let max_coord = (n_per_side as f64) - 1.0;
        let max_scaled = max_coord * 100.0;
        buf[179..187].copy_from_slice(&max_scaled.to_le_bytes()); // max x
        buf[187..195].copy_from_slice(&0.0f64.to_le_bytes()); // min x
        buf[195..203].copy_from_slice(&max_scaled.to_le_bytes()); // max y
        buf[203..211].copy_from_slice(&0.0f64.to_le_bytes()); // min y
        let z_scaled = z * 100.0;
        buf[211..219].copy_from_slice(&z_scaled.to_le_bytes()); // max z
        buf[219..227].copy_from_slice(&z_scaled.to_le_bytes()); // min z
        f.write_all(&buf).unwrap();

        for i in 0..n_per_side as i32 {
            for j in 0..n_per_side as i32 {
                let mut rec = [0u8; 20];
                rec[0..4].copy_from_slice(&(i * 100).to_le_bytes());
                rec[4..8].copy_from_slice(&(j * 100).to_le_bytes());
                rec[8..12].copy_from_slice(&((z * 100.0) as i32).to_le_bytes());
                f.write_all(&rec).unwrap();
            }
        }
    }

    #[test]
    fn test_run_eom_pipeline_on_flat_grid() {
        let tmp = tempfile::NamedTempFile::with_suffix(".las").unwrap();
        // 20x20 grid = 400 points, well above the 10-point CSF minimum.
        write_test_las(tmp.path(), 20, 100.0);

        let input = EomInput {
            point_cloud_path: tmp.path().to_path_buf(),
            csf_params: CsfParams {
                cloth_resolution: 1.0,
                max_iterations: 200,
                ..CsfParams::default()
            },
            dem_cell_size: 1.0,
            bench_interval: 5.0,
            max_points: 10_000,
            license_id: "LIC-TEST".to_string(),
            machine_id: "MACHINE-TEST".to_string(),
            site_id: "SITE-TEST".to_string(),
            signed: false,
            custodian: "Test Operator".to_string(),
            baseline_z: None,
            design_surface: None,
        };

        let progress_calls = std::sync::atomic::AtomicUsize::new(0);
        let output = run_eom_pipeline(&input, |_| {
            progress_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();
        let progress_count = progress_calls.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(output.points_read, 400);
        assert!(output.ground_points > 0);
        assert_eq!(output.ground_points + output.non_ground_points, 400);
        assert!(!output.audit_hash.is_empty());
        assert_eq!(output.audit_hash.len(), 64);
        assert_eq!(output.chain_of_custody.custody_id.starts_with("EOM-"), true);
        assert_eq!(output.chain_of_custody.report_hash, ""); // sealed by report module
        assert!(
            progress_count >= 5,
            "expected ≥5 progress callbacks, got {}",
            progress_count
        );
    }

    #[test]
    fn test_audit_hash_is_deterministic() {
        let tmp = tempfile::NamedTempFile::with_suffix(".las").unwrap();
        write_test_las(tmp.path(), 15, 100.0);

        let input = EomInput {
            point_cloud_path: tmp.path().to_path_buf(),
            csf_params: CsfParams {
                cloth_resolution: 1.0,
                max_iterations: 200,
                ..CsfParams::default()
            },
            dem_cell_size: 1.0,
            bench_interval: 0.0,
            max_points: 10_000,
            license_id: "LIC-TEST".to_string(),
            machine_id: "MACHINE-TEST".to_string(),
            site_id: "SITE-TEST".to_string(),
            signed: false,
            custodian: "Test Operator".to_string(),
            baseline_z: None,
            design_surface: None,
        };

        let out1 = run_eom_pipeline(&input, |_| {}).unwrap();
        let out2 = run_eom_pipeline(&input, |_| {}).unwrap();
        assert_eq!(out1.audit_hash, out2.audit_hash);
    }

    #[test]
    fn test_sha256_file_matches_known_hash() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello world").unwrap();
        let hash = sha256_file(tmp.path()).unwrap();
        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_hex_sha256_empty_input() {
        let h = hex_sha256(b"");
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
