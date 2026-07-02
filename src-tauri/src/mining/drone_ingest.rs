// UAV drone manifest parser — Phase 1 Mining MVP scaffolding.
//
// Supports DJI FlightHub and DJI MMC exports (JSON), and a generic
// image-list CSV format for non-DJI drones (SenseFly eMotion, PX4,
// Mission Planner). The Phase 1 goal is to extract enough metadata
// to:
//   - Display image footprints on the map
//   - Compute approximate ground sampling distance (GSD)
//   - Trigger an external ODM (OpenDroneMap) run via subprocess
//
// The actual SfM (structure-from-motion) processing is delegated to
// an external ODM Docker container — MetaRDU doesn't bundle ODM. The
// user installs ODM locally and MetaRDU shells out to it. This keeps
// the binary small and avoids licensing complexity.
//
// Spec references:
//   - DJI MMC format: documented in DJI Pilot 2 / GS Pro manuals
//   - DJI FlightHub: REST API + JSON exports
//   - ODM: https://github.com/OpenDroneMap/ODM

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneImage {
    pub filename: String,
    /// WGS84 longitude
    pub longitude: f64,
    /// WGS84 latitude
    pub latitude: f64,
    /// Altitude above mean sea level (meters)
    pub altitude: f64,
    /// Camera yaw (deg) — aircraft heading when photo was taken
    pub yaw: f64,
    /// Camera pitch (deg) — usually -90 for nadir
    pub pitch: f64,
    /// Camera roll (deg)
    pub roll: f64,
    /// Capture timestamp (Unix seconds UTC) — 0 if unknown
    pub timestamp: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DroneManifest {
    /// Source manifest file path
    pub source: String,
    /// Detected manifest format
    pub format: String,
    /// Number of images parsed
    pub image_count: usize,
    /// WGS84 bounds of all image centers
    pub bounds: Option<[f64; 4]>, // min_lon, min_lat, max_lon, max_lat
    /// Min/max altitude across all images
    pub min_altitude: f64,
    pub max_altitude: f64,
    /// Drone model if identifiable from the manifest
    pub drone_model: Option<String>,
    /// Camera model if identifiable
    pub camera_model: Option<String>,
    /// Images
    pub images: Vec<DroneImage>,
}

/// Internal result of parsing a manifest — used before computing bounds.
struct ManifestParse {
    format: String,
    images: Vec<DroneImage>,
    drone_model: Option<String>,
    camera_model: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DroneIngestError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported manifest format: {0}")]
    UnsupportedFormat(String),
    #[error("no images found in manifest")]
    NoImages,
}

/// Parse a drone manifest file. Format is auto-detected by file extension
/// and content sniffing.
///
/// Supported formats:
///   - .mrk (DJI MMC marker file) — line-oriented text format
///   - .json (DJI FlightHub export) — JSON with array of photo objects
///   - .csv (generic) — columns: filename, longitude, latitude, altitude[, yaw, pitch, roll]
pub fn parse_manifest(path: &Path) -> Result<DroneManifest, DroneIngestError> {
    if !path.exists() {
        return Err(DroneIngestError::NotFound(path.display().to_string()));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let parsed = match ext.as_str() {
        "mrk" => parse_dji_mmc(path)?,
        "json" => parse_dji_flighthub_json(path)?,
        "csv" => parse_generic_csv(path)?,
        _ => {
            // Sniff content
            let content = fs::read_to_string(path)?;
            let trimmed = content.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                parse_dji_flighthub_json(path)?
            } else if trimmed.contains("DJIFlightRecord") || trimmed.contains("Camera") {
                parse_dji_mmc(path)?
            } else {
                return Err(DroneIngestError::UnsupportedFormat(ext));
            }
        }
    };

    let ManifestParse {
        format,
        images,
        drone_model,
        camera_model,
    } = parsed;

    if images.is_empty() {
        return Err(DroneIngestError::NoImages);
    }

    let mut min_lon = f64::INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut min_alt = f64::INFINITY;
    let mut max_alt = f64::NEG_INFINITY;
    for img in &images {
        min_lon = min_lon.min(img.longitude);
        max_lon = max_lon.max(img.longitude);
        min_lat = min_lat.min(img.latitude);
        max_lat = max_lat.max(img.latitude);
        min_alt = min_alt.min(img.altitude);
        max_alt = max_alt.max(img.altitude);
    }

    Ok(DroneManifest {
        source: path.display().to_string(),
        format,
        image_count: images.len(),
        bounds: Some([min_lon, min_lat, max_lon, max_lat]),
        min_altitude: min_alt,
        max_altitude: max_alt,
        drone_model,
        camera_model,
        images,
    })
}

/// Parse a DJI MMC marker file. Each line is a record with comma-separated
/// or whitespace-separated fields. The format varies by drone generation
/// but typically includes: index, longitude, latitude, altitude, yaw, pitch,
/// roll, timestamp, filename.
fn parse_dji_mmc(path: &Path) -> Result<ManifestParse, DroneIngestError> {
    let content = fs::read_to_string(path)?;
    let mut images = Vec::new();
    let mut drone_model: Option<String> = None;
    let mut camera_model: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Try to extract drone/camera model from header lines
        if line.contains("DroneModel:") {
            drone_model = line
                .split("DroneModel:")
                .nth(1)
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string());
            continue;
        }
        if line.contains("CameraModel:") {
            camera_model = line
                .split("CameraModel:")
                .nth(1)
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string());
            continue;
        }
        // Skip non-numeric lines
        let first_char = line.chars().next();
        if first_char.is_none() || !first_char.unwrap().is_ascii_digit() {
            continue;
        }

        // Split on commas OR whitespace
        let fields: Vec<&str> = if line.contains(',') {
            line.split(',').map(|s| s.trim()).collect()
        } else {
            line.split_whitespace().collect()
        };

        if fields.len() < 4 {
            continue;
        }

        // MMC format is notoriously inconsistent. We try common layouts:
        //   Layout A: idx, lon, lat, alt, yaw, pitch, roll, time, filename
        //   Layout B: idx, lat, lon, alt, yaw, pitch, roll, time, filename
        // Detect by checking whether field[1] is in lon range (-180..180) AND
        // field[2] is in lat range (-90..90) (Layout A), OR vice versa.
        let f1: f64 = match fields[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let f2: f64 = match fields[2].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let f3: f64 = match fields[3].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let (lon, lat, alt) = if f1.abs() <= 180.0 && f2.abs() <= 90.0 {
            (f1, f2, f3) // Layout A
        } else if f2.abs() <= 180.0 && f1.abs() <= 90.0 {
            (f2, f1, f3) // Layout B
        } else {
            continue;
        };

        let yaw = fields.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let pitch = fields.get(5).and_then(|s| s.parse().ok()).unwrap_or(-90.0);
        let roll = fields.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let timestamp = fields.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
        let filename = fields
            .get(8)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("IMG_{:04}.JPG", images.len() + 1));

        images.push(DroneImage {
            filename,
            longitude: lon,
            latitude: lat,
            altitude: alt,
            yaw,
            pitch,
            roll,
            timestamp,
        });
    }

    Ok(ManifestParse {
        format: "dji-mmc".into(),
        images,
        drone_model,
        camera_model,
    })
}

/// Parse a DJI FlightHub JSON export. The format is an array of photo
/// objects with fields like latitude, longitude, altitude, etc.
fn parse_dji_flighthub_json(path: &Path) -> Result<ManifestParse, DroneIngestError> {
    let content = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;

    // Try to extract drone/camera model from top-level metadata if present
    let drone_model = value
        .get("droneModel")
        .or_else(|| value.get("drone_model"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let camera_model = value
        .get("cameraModel")
        .or_else(|| value.get("camera_model"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Find the photos array — could be at root or under "photos" / "photos"
    let photos = value
        .as_array()
        .or_else(|| value.get("photos").and_then(|v| v.as_array()))
        .or_else(|| value.get("images").and_then(|v| v.as_array()))
        .ok_or_else(|| DroneIngestError::UnsupportedFormat("JSON without photos array".into()))?;

    let mut images = Vec::with_capacity(photos.len());
    for photo in photos {
        let longitude = photo
            .get("longitude")
            .or_else(|| photo.get("lon"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let latitude = photo
            .get("latitude")
            .or_else(|| photo.get("lat"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let altitude = photo
            .get("altitude")
            .or_else(|| photo.get("alt"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let yaw = photo
            .get("yaw")
            .or_else(|| photo.get("heading"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let pitch = photo.get("pitch").and_then(|v| v.as_f64()).unwrap_or(-90.0);
        let roll = photo.get("roll").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let timestamp = photo
            .get("timestamp")
            .or_else(|| photo.get("time"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let filename = photo
            .get("filename")
            .or_else(|| photo.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("IMG_{:04}.JPG", images.len() + 1));

        images.push(DroneImage {
            filename,
            longitude,
            latitude,
            altitude,
            yaw,
            pitch,
            roll,
            timestamp,
        });
    }

    Ok(ManifestParse {
        format: "dji-flighthub-json".into(),
        images,
        drone_model,
        camera_model,
    })
}

/// Parse a generic CSV manifest. Expected columns:
///   filename, longitude, latitude, altitude[, yaw, pitch, roll, timestamp]
fn parse_generic_csv(path: &Path) -> Result<ManifestParse, DroneIngestError> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();
    let header = lines.next().unwrap_or("").to_lowercase();
    let has_header = header.contains("filename") || header.contains("lon");

    let mut images = Vec::new();

    let iter: Box<dyn Iterator<Item = &str>> = if has_header {
        Box::new(lines)
    } else {
        Box::new(content.lines())
    };

    for line in iter {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if fields.len() < 4 {
            continue;
        }
        let filename = fields[0].to_string();
        let longitude: f64 = match fields[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let latitude: f64 = match fields[2].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let altitude: f64 = match fields[3].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let yaw = fields.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let pitch = fields.get(5).and_then(|s| s.parse().ok()).unwrap_or(-90.0);
        let roll = fields.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let timestamp = fields.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
        images.push(DroneImage {
            filename,
            longitude,
            latitude,
            altitude,
            yaw,
            pitch,
            roll,
            timestamp,
        });
    }

    Ok(ManifestParse {
        format: "generic-csv".into(),
        images,
        drone_model: None,
        camera_model: None,
    })
}
