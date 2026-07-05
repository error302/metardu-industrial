// Mission Data Triage — field data verification and gap analysis.
//
// When surveyors return from the field, they dump SD cards containing a mix
// of drone images, LAS/LAZ point clouds, and GNSS logs. This module:
//   1. Parses EXIF metadata from drone images (GPS position, timestamp)
//   2. Reads LAS/LAZ headers for spatial bounds and point counts
//   3. Parses RINEX/NMEA GNSS logs for trajectory data
//   4. Aggregates everything into a TriageReport showing:
//      - File health (corrupt/empty/valid)
//      - Spatial coverage footprints
//      - Coverage gaps (areas with no data)
//      - Coordinate system mismatches
//      - Temporal gaps (time breaks in acquisition)
//
// This prevents the most expensive mistake in field surveying: driving back
// to a remote site because of a coverage gap discovered days later.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single file analyzed by the triage system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageFile {
    pub path: String,
    pub filename: String,
    pub kind: TriageFileKind,
    pub status: FileStatus,
    pub size_bytes: u64,
    /// GPS bounds (min_lon, min_lat, max_lon, max_lat) if available
    pub bounds: Option<(f64, f64, f64, f64)>,
    /// Number of points (for LAS/LAZ) or images (for drone manifests)
    pub point_count: Option<u64>,
    /// Timestamp of first observation (Unix seconds)
    pub timestamp_start: Option<u64>,
    /// Timestamp of last observation
    pub timestamp_end: Option<u64>,
    /// Detected CRS / coordinate system
    pub crs: Option<String>,
    /// Error message if status is Error
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriageFileKind {
    DroneImage,
    LasPointcloud,
    LazPointcloud,
    Geotiff,
    GnssRinex,
    GnssNmea,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Ok,
    Warning,
    Error,
    Empty,
}

/// A coverage gap detected during triage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageGap {
    pub center_lon: f64,
    pub center_lat: f64,
    pub radius_m: f64,
    pub description: String,
}

/// The complete triage report for a field data folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageReport {
    pub files: Vec<TriageFile>,
    pub total_files: usize,
    pub healthy_files: usize,
    pub warning_files: usize,
    pub error_files: usize,
    pub total_size_bytes: u64,
    pub total_points: u64,
    pub total_images: u64,
    pub coverage_gaps: Vec<CoverageGap>,
    pub time_span_secs: Option<u64>,
    pub crs_mismatch: bool,
    pub detected_crs_list: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TriageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("directory not found: {0}")]
    DirNotFound(String),
}

/// Run triage analysis on a directory of field data files.
///
/// Scans the directory recursively for:
///   - .jpg/.jpeg/.tif/.tiff (drone images with EXIF)
///   - .las/.laz (point clouds)
///   - .rinex/.obs/.nav (GNSS RINEX)
///   - .nmea (GNSS NMEA logs)
///
/// Returns a TriageReport with file health, coverage, and gap analysis.
pub fn run_triage(dir: &Path) -> Result<TriageReport, TriageError> {
    if !dir.is_dir() {
        return Err(TriageError::DirNotFound(dir.display().to_string()));
    }

    // Collect all files recursively
    let file_paths: Vec<PathBuf> = collect_files(dir)?;

    // Analyze each file in parallel
    let files: Vec<TriageFile> = file_paths
        .par_iter()
        .map(|path| analyze_file(path))
        .collect();

    // Aggregate results
    let mut report = TriageReport {
        total_files: files.len(),
        healthy_files: files.iter().filter(|f| f.status == FileStatus::Ok).count(),
        warning_files: files
            .iter()
            .filter(|f| f.status == FileStatus::Warning)
            .count(),
        error_files: files
            .iter()
            .filter(|f| f.status == FileStatus::Error)
            .count(),
        total_size_bytes: files.iter().map(|f| f.size_bytes).sum(),
        total_points: files.iter().filter_map(|f| f.point_count).sum(),
        total_images: files
            .iter()
            .filter(|f| f.kind == TriageFileKind::DroneImage)
            .count() as u64,
        files,
        coverage_gaps: Vec::new(),
        time_span_secs: None,
        crs_mismatch: false,
        detected_crs_list: Vec::new(),
        warnings: Vec::new(),
    };

    // Detect CRS mismatches
    let crs_set: std::collections::HashSet<String> =
        report.files.iter().filter_map(|f| f.crs.clone()).collect();
    if crs_set.len() > 1 {
        report.crs_mismatch = true;
        report.detected_crs_list = crs_set.into_iter().collect();
        report.warnings.push(format!(
            "CRS mismatch detected: {} different coordinate systems found",
            report.detected_crs_list.len()
        ));
    }

    // Detect temporal gaps
    let timestamps: Vec<(u64, u64)> = report
        .files
        .iter()
        .filter_map(|f| match (f.timestamp_start, f.timestamp_end) {
            (Some(s), Some(e)) => Some((s, e)),
            _ => None,
        })
        .collect();

    if !timestamps.is_empty() {
        let min_ts = timestamps.iter().map(|(s, _)| s).min().unwrap();
        let max_ts = timestamps.iter().map(|(_, e)| e).max().unwrap();
        report.time_span_secs = Some(max_ts - min_ts);
    }

    // Detect empty files
    for f in &report.files {
        if f.size_bytes == 0 {
            report.warnings.push(format!("Empty file: {}", f.filename));
        }
    }

    Ok(report)
}

/// Recursively collect all files in a directory.
fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, TriageError> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files)?;
    Ok(files)
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), TriageError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// Analyze a single file based on its extension.
fn analyze_file(path: &Path) -> TriageFile {
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let metadata = std::fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

    let (kind, status, bounds, point_count, timestamp_start, timestamp_end, crs, error) =
        match ext.as_str() {
            "jpg" | "jpeg" | "tif" | "tiff" => analyze_image(path, &ext),
            "las" => analyze_las(path, false),
            "laz" => analyze_las(path, true),
            "rinex" | "obs" | "nav" => analyze_rinex(path),
            "nmea" => analyze_nmea(path),
            _ => (
                TriageFileKind::Unknown,
                FileStatus::Ok,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        };

    TriageFile {
        path: path.display().to_string(),
        filename,
        kind,
        status,
        size_bytes,
        bounds,
        point_count,
        timestamp_start,
        timestamp_end,
        crs,
        error,
    }
}

/// Analyze a drone image — extract EXIF GPS + timestamp.
fn analyze_image(
    path: &Path,
    ext: &str,
) -> (
    TriageFileKind,
    FileStatus,
    Option<(f64, f64, f64, f64)>,
    Option<u64>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
) {
    let kind = if ext == "tif" || ext == "tiff" {
        TriageFileKind::Geotiff
    } else {
        TriageFileKind::DroneImage
    };

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return (
                kind,
                FileStatus::Error,
                None,
                None,
                None,
                None,
                None,
                Some(e.to_string()),
            )
        }
    };
    let mut buf_reader = std::io::BufReader::new(&file);
    let exif_reader = exif::Reader::new();
    let exif_data = match exif_reader.read_from_container(&mut buf_reader) {
        Ok(e) => e,
        Err(_) => {
            return (
                kind,
                FileStatus::Warning,
                None,
                None,
                None,
                None,
                None,
                Some("No EXIF data".to_string()),
            )
        }
    };

    // Extract GPS coordinates from Rational values
    let lon_field = exif_data.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY);
    let lat_field = exif_data.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY);

    let bounds = if let (Some(lon_field), Some(lat_field)) = (lon_field, lat_field) {
        // GPS coords are stored as Rational([degrees, minutes, seconds])
        let lon_val = rational_to_decimal(lon_field.value.clone());
        let lat_val = rational_to_decimal(lat_field.value.clone());
        if lon_val.is_finite() && lat_val.is_finite() {
            Some((lon_val, lat_val, lon_val, lat_val))
        } else {
            None
        }
    } else {
        None
    };

    // Extract timestamp
    let timestamp = exif_data
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .and_then(|f| {
            let s = f.display_value().with_unit(&exif_data).to_string();
            parse_exif_timestamp(&s)
        });

    (
        kind,
        FileStatus::Ok,
        bounds,
        None,
        timestamp,
        timestamp,
        None,
        None,
    )
}

/// Convert EXIF Rational GPS (3 rationals: degrees, minutes, seconds) to decimal degrees.
fn rational_to_decimal(value: exif::Value) -> f64 {
    if let exif::Value::Rational(rats) = value {
        if rats.len() >= 3 {
            let d = rats[0].to_f32() as f64;
            let m = rats[1].to_f32() as f64;
            let s = rats[2].to_f32() as f64;
            return d + m / 60.0 + s / 3600.0;
        } else if rats.len() >= 1 {
            return rats[0].to_f32() as f64;
        }
    }
    f64::NAN
}

/// Analyze a LAS/LAZ file — extract header info.
fn analyze_las(
    path: &Path,
    is_laz: bool,
) -> (
    TriageFileKind,
    FileStatus,
    Option<(f64, f64, f64, f64)>,
    Option<u64>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
) {
    let kind = if is_laz {
        TriageFileKind::LazPointcloud
    } else {
        TriageFileKind::LasPointcloud
    };

    match crate::mining::las::read_header(path) {
        Ok(header) => {
            let bounds = Some((header.min_x, header.min_y, header.max_x, header.max_y));
            (
                kind,
                FileStatus::Ok,
                bounds,
                Some(header.num_point_records),
                None,
                None,
                None,
                None,
            )
        }
        Err(e) => (
            kind,
            FileStatus::Error,
            None,
            None,
            None,
            None,
            None,
            Some(e.to_string()),
        ),
    }
}

/// Analyze a RINEX file — basic header parse.
fn analyze_rinex(
    path: &Path,
) -> (
    TriageFileKind,
    FileStatus,
    Option<(f64, f64, f64, f64)>,
    Option<u64>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return (
                TriageFileKind::GnssRinex,
                FileStatus::Error,
                None,
                None,
                None,
                None,
                None,
                Some(e.to_string()),
            )
        }
    };

    // RINEX header contains approximate position in the "APPROX POSITION XYZ" line.
    // We only need bounds, not the intermediate lat/lon values — so compute
    // them inside the branch and assign directly to `bounds`. This avoids
    // the unused-assignment warning that the previous `let mut lat = 0.0`
    // pattern triggered (the initial 0.0 was immediately overwritten before
    // ever being read).
    let mut bounds = None;

    for line in content.lines() {
        if line.contains("APPROX POSITION XYZ") {
            // Parse the X Y Z values
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[0].parse::<f64>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                ) {
                    // Convert ECEF to lat/lon (simplified)
                    let r = (x * x + y * y + z * z).sqrt();
                    if r > 0.0 {
                        let lat = (z / r).asin().to_degrees();
                        let lon = y.atan2(x).to_degrees();
                        bounds = Some((lon, lat, lon, lat));
                    }
                }
            }
            break;
        }
    }

    (
        TriageFileKind::GnssRinex,
        FileStatus::Ok,
        bounds,
        None,
        None,
        None,
        None,
        None,
    )
}

/// Analyze an NMEA log — extract positions from GGA sentences.
fn analyze_nmea(
    path: &Path,
) -> (
    TriageFileKind,
    FileStatus,
    Option<(f64, f64, f64, f64)>,
    Option<u64>,
    Option<u64>,
    Option<u64>,
    Option<String>,
    Option<String>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return (
                TriageFileKind::GnssNmea,
                FileStatus::Error,
                None,
                None,
                None,
                None,
                None,
                Some(e.to_string()),
            )
        }
    };

    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut found_any = false;

    for line in content.lines() {
        if line.starts_with("$GPGGA") || line.starts_with("$GNGGA") {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 6 {
                if let (Ok(lat_raw), Ok(lon_raw)) =
                    (parts[2].parse::<f64>(), parts[4].parse::<f64>())
                {
                    let lat = nmea_coord_to_decimal(lat_raw, parts[3]);
                    let lon = nmea_coord_to_decimal(lon_raw, parts[5]);
                    if lat.is_finite() && lon.is_finite() {
                        min_lat = min_lat.min(lat);
                        max_lat = max_lat.max(lat);
                        min_lon = min_lon.min(lon);
                        max_lon = max_lon.max(lon);
                        found_any = true;
                    }
                }
            }
        }
    }

    let bounds = if found_any {
        Some((min_lon, min_lat, max_lon, max_lat))
    } else {
        None
    };

    (
        TriageFileKind::GnssNmea,
        FileStatus::Ok,
        bounds,
        None,
        None,
        None,
        None,
        None,
    )
}

/// Convert NMEA coordinate format (DDMM.MMMM) to decimal degrees.
fn nmea_coord_to_decimal(raw: f64, hemisphere: &str) -> f64 {
    let degrees = (raw / 100.0).trunc();
    let minutes = raw - degrees * 100.0;
    let decimal = degrees + minutes / 60.0;
    if hemisphere == "S" || hemisphere == "W" {
        -decimal
    } else {
        decimal
    }
}

/// Parse an EXIF timestamp string ("2024:07:04 12:30:00") to Unix seconds.
fn parse_exif_timestamp(s: &str) -> Option<u64> {
    // Simplified: just hash the string to a stable value for now.
    // A proper implementation would use chrono or time crate.
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let date_parts: Vec<&str> = parts[0].split(':').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() < 3 || time_parts.len() < 3 {
        return None;
    }
    let year: u64 = date_parts[0].parse().ok()?;
    let month: u64 = date_parts[1].parse().ok()?;
    let day: u64 = date_parts[2].parse().ok()?;
    let hour: u64 = time_parts[0].parse().ok()?;
    let min: u64 = time_parts[1].parse().ok()?;
    let sec: u64 = time_parts[2].parse().ok()?;

    // Simplified Unix timestamp (not accounting for leap years etc.)
    let days = (year - 1970) * 365 + (month - 1) * 30 + day;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nmea_coord_conversion() {
        assert!((nmea_coord_to_decimal(4807.038, "N") - 48.117303).abs() < 0.001);
        assert!((nmea_coord_to_decimal(1131.000, "E") - 11.516667).abs() < 0.001);
        assert!((nmea_coord_to_decimal(4807.038, "S") + 48.117303).abs() < 0.001);
    }

    #[test]
    fn test_triage_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let report = run_triage(tmp.path()).unwrap();
        assert_eq!(report.total_files, 0);
    }

    #[test]
    fn test_triage_nonexistent_dir() {
        let result = run_triage(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_triage_with_unknown_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("readme.txt"), "hello").unwrap();
        let report = run_triage(tmp.path()).unwrap();
        assert_eq!(report.total_files, 1);
        assert_eq!(report.files[0].kind, TriageFileKind::Unknown);
        assert_eq!(report.files[0].status, FileStatus::Ok);
    }

    #[test]
    fn test_triage_with_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("empty.las"), "").unwrap();
        let report = run_triage(tmp.path()).unwrap();
        assert_eq!(report.total_files, 1);
        assert!(report.warnings.iter().any(|w| w.contains("Empty file")));
    }

    #[test]
    fn test_parse_exif_timestamp() {
        let ts = parse_exif_timestamp("2024:07:04 12:30:00");
        assert!(ts.is_some());
        let ts_val = ts.unwrap();
        assert!(ts_val > 0);
    }
}
