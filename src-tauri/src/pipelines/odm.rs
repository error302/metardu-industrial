// ODM (OpenDroneMap) subprocess manager — Phase 1.
//
// MetaRDU doesn't bundle ODM. The user installs ODM locally (Docker image
// `opendronemap/odm` is the canonical distribution) and MetaRDU shells
// out to it via `docker run`. This keeps the binary small and lets users
// pick their ODM version.
//
// Pipeline:
//   1. Validate the input image directory exists and has JPEGs/TIFFs
//   2. Validate Docker is installed and the ODM image is pulled
//   3. Construct the docker run command with project path mounted
//   4. Spawn the subprocess, stream stdout/stderr line-by-line via a
//      Tauri event channel
//   5. On completion, verify the output LAS exists at the expected path
//   6. Return the LAS path so the frontend can probe_file() it
//
// The user can configure:
//   - ODM Docker image name (default: opendronemap/odm:latest)
//   - ODM parameters (max-concurrency, feature-quality, etc.)
//   - Output path (default: <images_dir>/odm_results)
//
// We use tokio::process for async subprocess management so the Tauri
// command handler doesn't block the UI thread.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdmConfig {
    /// Docker image to use — default "opendronemap/odm:latest"
    #[serde(default = "default_image")]
    pub image: String,
    /// Path to the directory of JPEG/TIFF images
    pub images_dir: String,
    /// Where to write ODM results (default: <images_dir>/odm_results)
    pub output_dir: Option<String>,
    /// Max CPU cores ODM can use (default: 4)
    #[serde(default = "default_concurrency")]
    pub max_concurrency: u32,
    /// Feature quality: ultra, high, medium, low, lowest
    #[serde(default = "default_quality")]
    pub feature_quality: String,
    /// Skip 3D model generation (saves time, we only need point cloud)
    #[serde(default = "default_skip_3dmodel")]
    pub skip_3dmodel: bool,
    /// PC-Type: las laz ply csv
    #[serde(default = "default_pc_type")]
    pub pc_type: String,
}

fn default_image() -> String {
    "opendronemap/odm:latest".into()
}
fn default_concurrency() -> u32 {
    4
}
fn default_quality() -> String {
    "high".into()
}
fn default_skip_3dmodel() -> bool {
    true
}
fn default_pc_type() -> String {
    "las".into()
}

impl Default for OdmConfig {
    fn default() -> Self {
        Self {
            image: default_image(),
            images_dir: String::new(),
            output_dir: None,
            max_concurrency: default_concurrency(),
            feature_quality: default_quality(),
            skip_3dmodel: default_skip_3dmodel(),
            pc_type: default_pc_type(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OdmStatus {
    pub phase: String,
    pub progress: f64, // 0.0–1.0, estimated from log keywords
    pub last_log_line: String,
    pub elapsed_seconds: u64,
    pub output_las_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OdmError {
    #[error("docker not found — install Docker Desktop or docker engine")]
    DockerNotFound,
    #[error("images directory not found: {0}")]
    ImagesDirNotFound(String),
    #[error("no images found in {0} (expected .jpg/.jpeg/.tif/.tiff)")]
    NoImages(String),
    #[error("ODM image not pulled — run `docker pull {0}` first")]
    ImageNotPulled(String),
    #[error("ODM process failed: {0}")]
    ProcessFailed(String),
    #[error("ODM completed but output LAS not found at {0}")]
    OutputNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Check that Docker is installed and the configured ODM image is available.
pub async fn check_odm(image: &str) -> Result<bool, OdmError> {
    // Check docker is on PATH
    let docker_check = Command::new("docker")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    if docker_check.is_err() {
        return Err(OdmError::DockerNotFound);
    }

    // Check image is pulled
    let output = Command::new("docker")
        .args(["image", "inspect", image])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await?;
    Ok(output.status.success())
}

/// Count the number of supported image files in a directory.
pub fn count_images(dir: &Path) -> Result<usize, OdmError> {
    if !dir.is_dir() {
        return Err(OdmError::ImagesDirNotFound(dir.display().to_string()));
    }
    let mut count = 0usize;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_lowercase();
            if matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") {
                count += 1;
            }
        }
    }
    if count == 0 {
        return Err(OdmError::NoImages(dir.display().to_string()));
    }
    Ok(count)
}

/// Run the ODM pipeline. Spawns `docker run` with the config, streams
/// stdout/stderr line-by-line via the on_log callback, and returns the
/// path to the resulting LAS file on completion.
///
/// The on_log callback is invoked for every line of output — the caller
/// is responsible for forwarding to a Tauri event channel.
pub async fn run_odm<F>(config: &OdmConfig, mut on_log: F) -> Result<PathBuf, OdmError>
where
    F: FnMut(&str),
{
    let images_dir = PathBuf::from(&config.images_dir);
    if !images_dir.is_dir() {
        return Err(OdmError::ImagesDirNotFound(config.images_dir.clone()));
    }
    let _image_count = count_images(&images_dir)?;

    let output_dir = config
        .output_dir
        .clone()
        .unwrap_or_else(|| format!("{}/odm_results", config.images_dir));
    let output_dir_path = PathBuf::from(&output_dir);
    std::fs::create_dir_all(&output_dir_path)?;

    // Docker run command — mount images dir as /datasets/code, output as /odm_results
    // ODM expects: docker run --rm -v <images>:/datasets/code -v <output>:/outputs
    //              opendronemap/odm [options]
    let images_mount = format!("{}:/datasets/code", images_dir.display());
    let output_mount = format!("{}:/outputs", output_dir_path.display());

    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--rm",
        "-v",
        &images_mount,
        "-v",
        &output_mount,
        &config.image,
    ]);

    // ODM options
    cmd.arg("--max-concurrency")
        .arg(config.max_concurrency.to_string());
    cmd.arg("--feature-quality").arg(&config.feature_quality);
    cmd.arg("--pc-type").arg(&config.pc_type);
    if config.skip_3dmodel {
        cmd.arg("--skip-3dmodel");
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            OdmError::DockerNotFound
        } else {
            OdmError::ProcessFailed(format!("failed to spawn docker: {e}"))
        }
    })?;

    // Take stdout + stderr, merge, stream line-by-line
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // We can't easily await both in parallel without select!, so we
    // alternate non-blocking reads. For Phase 1 simplicity, we drain
    // stderr first then stdout — ODM writes most output to stdout anyway.
    loop {
        let line = stdout_reader.next_line().await?;
        if let Some(l) = line {
            on_log(&l);
        } else {
            break;
        }
    }
    // Drain any remaining stderr
    while let Some(l) = stderr_reader.next_line().await? {
        on_log(&format!("[stderr] {l}"));
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(OdmError::ProcessFailed(format!(
            "ODM exited with status {}",
            status.code().unwrap_or(-1)
        )));
    }

    // ODM writes the point cloud to:
    //   <output>/odm_georeferencing/odm_georeferenced_model.las (or laz/ply/csv)
    let las_path = output_dir_path
        .join("odm_georeferencing")
        .join(format!("odm_georeferenced_model.{}", config.pc_type));
    if !las_path.exists() {
        return Err(OdmError::OutputNotFound(las_path.display().to_string()));
    }

    Ok(las_path)
}

/// Estimate progress (0.0–1.0) from a log line by matching ODM's phase keywords.
pub fn estimate_progress(log_line: &str) -> Option<f64> {
    let line = log_line.to_lowercase();
    // ODM phases in order — rough percentages based on typical runtime
    if line.contains("running opencv") {
        return Some(0.02);
    }
    if line.contains("loading dataset") || line.contains("dataset loaded") {
        return Some(0.05);
    }
    if line.contains("extracting features") {
        return Some(0.15);
    }
    if line.contains("matching features") || line.contains("matching") {
        return Some(0.30);
    }
    if line.contains("running bundle adjustment") {
        return Some(0.45);
    }
    if line.contains("reconstruction") || line.contains("reconstructing") {
        return Some(0.55);
    }
    if line.contains("dense reconstruction") || line.contains("dense point cloud") {
        return Some(0.70);
    }
    if line.contains("georeferencing") {
        return Some(0.85);
    }
    if line.contains("exporting point cloud") || line.contains("odm_georeferenced_model") {
        return Some(0.95);
    }
    if line.contains("pipeline finished") || line.contains("odom finished") {
        return Some(1.0);
    }
    None
}
