// Project File Format (.metardu) — Sprint 8 Production Distribution.
//
// Saves and loads full project state so surveyors can close the app,
// reopen it tomorrow, and pick up exactly where they left off.
//
// Without this, nothing else matters — users can't persist their work.
//
// File format: JSON with .metardu extension. Human-readable so users
// can inspect/diff/debug. Versioned for forward compatibility.
//
// Saved state:
//   - Project name + creation/modification timestamps
//   - Default CRS (EPSG code)
//   - Active domain (mining / marine / both)
//   - Loaded files (path, kind, display name, layer visibility, color)
//   - Map view state (center lon/lat, zoom, rotation)
//   - CSF classification results (per file)
//   - CUBE surface parameters + last result
//   - Recent report paths (for quick re-open)
//   - Layout profile (default / data_ingest / bathymetry_clean / volume_reporting)
//   - License tier at save time (for display consistency)
//   - Pipeline definitions (user's saved YAML pipelines)
//
// NOT saved (privacy + freshness):
//   - Telemetry config (per-installation, not per-project)
//   - License file content (only tier label)
//   - Crash dumps
//
// Auto-save: the frontend calls save_project every 30 seconds + on close.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Current .metardu file format version. Bump when the schema changes.
pub const PROJECT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetarduProject {
    /// File format version — for forward compatibility
    pub format_version: u32,
    /// Project name (shown in title bar)
    pub name: String,
    /// ISO 8601 creation timestamp
    pub created: String,
    /// ISO 8601 last-modified timestamp
    pub modified: String,
    /// Default EPSG code (e.g., "EPSG:28355")
    pub default_epsg: String,
    /// Active domain: "mining" | "marine" | "both"
    pub domain: String,
    /// Loaded files (in load order)
    pub files: Vec<ProjectFile>,
    /// Map view state
    pub view_state: ViewState,
    /// CSF classification results, keyed by file path
    #[serde(default)]
    pub csf_results: HashMap<String, CsfResultSummary>,
    /// CUBE surface parameters last used
    #[serde(default)]
    pub cube_params: Option<CubeParamsSummary>,
    /// Recent report output paths (most recent first)
    #[serde(default)]
    pub recent_reports: Vec<String>,
    /// Active layout profile
    #[serde(default)]
    pub layout: String,
    /// License tier label at save time
    #[serde(default)]
    pub license_tier: String,
    /// User's saved pipeline definitions (YAML strings)
    #[serde(default)]
    pub pipelines: Vec<String>,
    /// Theme: "dark" | "light"
    #[serde(default)]
    pub theme: String,
    /// Custom metadata (key-value pairs the user can add)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    /// Absolute path to the file on disk
    pub path: String,
    /// File kind: "las" | "geotiff" | "kongsberg_all" | "reson_s7k" | "xtf"
    pub kind: String,
    /// Display name (usually filename without extension)
    pub name: String,
    /// File size in bytes (for display)
    pub size_bytes: u64,
    /// Layer visibility toggle
    pub visible: bool,
    /// Layer color (hex, e.g., "#FFA500")
    #[serde(default)]
    pub color: Option<String>,
    /// Layer opacity (0.0 to 1.0)
    #[serde(default = "default_opacity")]
    pub opacity: f64,
}

fn default_opacity() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewState {
    /// Center longitude (WGS84 degrees)
    pub center_lon: f64,
    /// Center latitude (WGS84 degrees)
    pub center_lat: f64,
    /// Zoom level (OpenLayers scale)
    pub zoom: f64,
    /// Rotation in degrees (0 = north up)
    #[serde(default)]
    pub rotation: f64,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            center_lon: 0.0,
            center_lat: 0.0,
            zoom: 2.0,
            rotation: 0.0,
        }
    }
}

/// CSF result summary (not the full per-point classification — that's
/// recomputed on load if needed). We save enough to restore the
/// ground/non-ground coloring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsfResultSummary {
    /// File path this result applies to
    pub file_path: String,
    /// CSF parameters used
    pub cloth_resolution: f64,
    pub classifications: f64,
    /// Point count
    pub point_count: u64,
    /// Ground point count
    pub ground_count: u64,
    /// Time to compute (ms)
    pub elapsed_ms: u64,
    /// Slope (radians) used in CSF
    pub slope: f64,
}

/// CUBE surface parameters (enough to re-run with same settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeParamsSummary {
    pub cell_size: f64,
    pub iho_order: String,
    pub hypothesis_distance: f64,
    pub soundings_count: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project file format version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
    #[error("project file is empty or invalid")]
    Empty,
}

/// Save a project to a .metardu file.
pub fn save_project(project: &MetarduProject, path: &Path) -> Result<(), ProjectError> {
    let json = serde_json::to_string_pretty(project)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a project from a .metardu file.
pub fn load_project(path: &Path) -> Result<MetarduProject, ProjectError> {
    let content = std::fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Err(ProjectError::Empty);
    }
    let project: MetarduProject = serde_json::from_str(&content)?;

    // Version check — for now we only support v1. Future versions will
    // migrate old formats.
    if project.format_version != PROJECT_FORMAT_VERSION {
        return Err(ProjectError::VersionMismatch {
            expected: PROJECT_FORMAT_VERSION,
            actual: project.format_version,
        });
    }

    Ok(project)
}

/// Create a new empty project with sensible defaults.
pub fn new_project(name: &str, default_epsg: &str, domain: &str) -> MetarduProject {
    let now = now_iso();
    MetarduProject {
        format_version: PROJECT_FORMAT_VERSION,
        name: name.into(),
        created: now.clone(),
        modified: now,
        default_epsg: default_epsg.into(),
        domain: domain.into(),
        files: Vec::new(),
        view_state: ViewState::default(),
        csf_results: HashMap::new(),
        cube_params: None,
        recent_reports: Vec::new(),
        layout: "default".into(),
        license_tier: "core".into(),
        pipelines: Vec::new(),
        theme: "dark".into(),
        metadata: HashMap::new(),
    }
}

/// Add a file to the project.
pub fn add_file_to_project(project: &mut MetarduProject, file: ProjectFile) {
    // Replace if same path already exists
    project.files.retain(|f| f.path != file.path);
    project.files.push(file);
    project.modified = now_iso();
}

/// Remove a file from the project by path.
pub fn remove_file_from_project(project: &mut MetarduProject, path: &str) {
    project.files.retain(|f| f.path != path);
    project.csf_results.remove(path);
    project.modified = now_iso();
}

/// Update the view state.
pub fn update_view_state(project: &mut MetarduProject, view: ViewState) {
    project.view_state = view;
    project.modified = now_iso();
}

/// Add a report path to the recent list (most recent first, max 10).
pub fn add_recent_report(project: &mut MetarduProject, report_path: &str) {
    project.recent_reports.retain(|p| p != report_path);
    project.recent_reports.insert(0, report_path.into());
    if project.recent_reports.len() > 10 {
        project.recent_reports.truncate(10);
    }
    project.modified = now_iso();
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = ((day_of_year / 30) as u8).min(11) + 1;
    let day = ((day_of_year % 30) as u8) + 1;
    format!("{:04}-{:02}-{:02}T00:00:00Z", year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_project_defaults() {
        let p = new_project("Test Project", "EPSG:4326", "both");
        assert_eq!(p.format_version, PROJECT_FORMAT_VERSION);
        assert_eq!(p.name, "Test Project");
        assert_eq!(p.default_epsg, "EPSG:4326");
        assert_eq!(p.domain, "both");
        assert!(p.files.is_empty());
        assert_eq!(p.theme, "dark");
        assert_eq!(p.layout, "default");
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("metardu_test_project.metardu");
        let mut project = new_project("Roundtrip Test", "EPSG:28355", "mining");
        add_file_to_project(
            &mut project,
            ProjectFile {
                path: "/tmp/survey.tif".into(),
                kind: "geotiff".into(),
                name: "survey".into(),
                size_bytes: 1024,
                visible: true,
                color: Some("#FFA500".into()),
                opacity: 0.8,
            },
        );
        add_recent_report(&mut project, "/tmp/report.html");

        save_project(&project, &tmp).unwrap();
        let loaded = load_project(&tmp).unwrap();

        assert_eq!(loaded.name, "Roundtrip Test");
        assert_eq!(loaded.default_epsg, "EPSG:28355");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].path, "/tmp/survey.tif");
        assert_eq!(loaded.files[0].color, Some("#FFA500".into()));
        assert!((loaded.files[0].opacity - 0.8).abs() < 0.001);
        assert_eq!(loaded.recent_reports.len(), 1);
        assert_eq!(loaded.recent_reports[0], "/tmp/report.html");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_add_file_replaces_existing() {
        let mut p = new_project("Test", "EPSG:4326", "both");
        add_file_to_project(
            &mut p,
            ProjectFile {
                path: "/tmp/a.tif".into(),
                kind: "geotiff".into(),
                name: "a".into(),
                size_bytes: 100,
                visible: true,
                color: None,
                opacity: 1.0,
            },
        );
        add_file_to_project(
            &mut p,
            ProjectFile {
                path: "/tmp/a.tif".into(),
                kind: "geotiff".into(),
                name: "a_updated".into(),
                size_bytes: 200,
                visible: false,
                color: Some("#FF0000".into()),
                opacity: 0.5,
            },
        );
        assert_eq!(p.files.len(), 1);
        assert_eq!(p.files[0].name, "a_updated");
        assert!(!p.files[0].visible);
    }

    #[test]
    fn test_remove_file() {
        let mut p = new_project("Test", "EPSG:4326", "both");
        add_file_to_project(
            &mut p,
            ProjectFile {
                path: "/tmp/a.tif".into(),
                kind: "geotiff".into(),
                name: "a".into(),
                size_bytes: 100,
                visible: true,
                color: None,
                opacity: 1.0,
            },
        );
        remove_file_from_project(&mut p, "/tmp/a.tif");
        assert!(p.files.is_empty());
    }

    #[test]
    fn test_recent_reports_max_10() {
        let mut p = new_project("Test", "EPSG:4326", "both");
        for i in 0..15 {
            add_recent_report(&mut p, &format!("/tmp/report_{}.html", i));
        }
        assert_eq!(p.recent_reports.len(), 10);
        // Most recent should be first
        assert_eq!(p.recent_reports[0], "/tmp/report_14.html");
    }

    #[test]
    fn test_recent_reports_dedup() {
        let mut p = new_project("Test", "EPSG:4326", "both");
        add_recent_report(&mut p, "/tmp/r1.html");
        add_recent_report(&mut p, "/tmp/r2.html");
        add_recent_report(&mut p, "/tmp/r1.html"); // duplicate
        assert_eq!(p.recent_reports.len(), 2);
        assert_eq!(p.recent_reports[0], "/tmp/r1.html");
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_project(std::path::Path::new("/nonexistent.metardu"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_empty_file() {
        let tmp = std::env::temp_dir().join("metardu_test_empty.metardu");
        std::fs::write(&tmp, "").unwrap();
        let result = load_project(&tmp);
        assert!(matches!(result, Err(ProjectError::Empty)));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_version_mismatch() {
        let tmp = std::env::temp_dir().join("metardu_test_v2.metardu");
        let json = r#"{"format_version":999,"name":"Test","created":"2026-01-01","modified":"2026-01-01","default_epsg":"EPSG:4326","domain":"both","files":[],"view_state":{"center_lon":0,"center_lat":0,"zoom":2,"rotation":0}}"#;
        std::fs::write(&tmp, json).unwrap();
        let result = load_project(&tmp);
        assert!(matches!(result, Err(ProjectError::VersionMismatch { .. })));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_update_view_state() {
        let mut p = new_project("Test", "EPSG:4326", "both");
        update_view_state(
            &mut p,
            ViewState {
                center_lon: 144.0,
                center_lat: -37.0,
                zoom: 12.0,
                rotation: 45.0,
            },
        );
        assert!((p.view_state.center_lon - 144.0).abs() < 0.001);
        assert!((p.view_state.zoom - 12.0).abs() < 0.001);
    }
}
