// SVP (Sound Velocity Profile) model + parser — Phase 3 / Sprint 3.
//
// Sound speed in water varies with temperature, salinity, and depth.
// Surveyors import SVP casts to correct multibeam ray tracing.
//
// Supported formats:
//   - .svp (Kongsberg format): depth(m) speed(m/s) per line
//   - .asvp (ASCII SVP): same but with header
//   - CSV: depth,speed per line

use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpProfile {
    pub source: String,
    pub cast_count: usize,
    pub points: Vec<SvpPoint>,
    pub min_depth: f64,
    pub max_depth: f64,
    pub min_speed: f64,
    pub max_speed: f64,
    pub surface_speed: f64,
    pub bottom_speed: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SvpPoint {
    pub depth: f64, // meters
    pub speed: f64, // m/s
}

pub fn parse_svp(path: &Path) -> Result<SvpProfile, SvpError> {
    let file =
        std::fs::File::open(path).map_err(|_| SvpError::NotFound(path.display().to_string()))?;
    let reader = std::io::BufReader::new(file);
    let mut points = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        // Try comma-separated first, then whitespace
        let parts: Vec<&str> = if trimmed.contains(',') {
            trimmed.split(',').collect()
        } else {
            trimmed.split_whitespace().collect()
        };

        if parts.len() < 2 {
            continue;
        }

        // Try to parse — skip header lines that don't parse as numbers
        let depth: f64 = match parts[0].parse() {
            Ok(d) => d,
            Err(_) => continue, // Header line
        };
        let speed: f64 = match parts[1].parse() {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Sanity check: depth should be 0-12000m, speed 1400-1600 m/s
        if depth < 0.0 || depth > 12000.0 || speed < 1300.0 || speed > 1700.0 {
            continue;
        }

        points.push(SvpPoint { depth, speed });
    }

    if points.len() < 2 {
        return Err(SvpError::TooFewPoints(points.len()));
    }

    let min_depth = points.iter().map(|p| p.depth).fold(f64::INFINITY, f64::min);
    let max_depth = points
        .iter()
        .map(|p| p.depth)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_speed = points.iter().map(|p| p.speed).fold(f64::INFINITY, f64::min);
    let max_speed = points
        .iter()
        .map(|p| p.speed)
        .fold(f64::NEG_INFINITY, f64::max);

    Ok(SvpProfile {
        source: path.display().to_string(),
        cast_count: points.len(),
        points,
        min_depth,
        max_depth,
        min_speed,
        max_speed,
        surface_speed: min_speed, // speed at shallowest depth
        bottom_speed: max_speed,  // speed at deepest
    })
}

/// Interpolate sound speed at a given depth using linear interpolation.
pub fn interpolate_speed(profile: &SvpProfile, depth: f64) -> f64 {
    if profile.points.is_empty() {
        return 1500.0; // Default seawater speed
    }
    if depth <= profile.points[0].depth {
        return profile.points[0].speed;
    }
    if depth >= profile.points[profile.points.len() - 1].depth {
        return profile.points[profile.points.len() - 1].speed;
    }

    for i in 0..profile.points.len() - 1 {
        let p0 = &profile.points[i];
        let p1 = &profile.points[i + 1];
        if depth >= p0.depth && depth <= p1.depth {
            let t = (depth - p0.depth) / (p1.depth - p0.depth);
            return p0.speed + t * (p1.speed - p0.speed);
        }
    }

    1500.0
}

#[derive(Debug, thiserror::Error)]
pub enum SvpError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("too few valid points: {0} (need at least 2)")]
    TooFewPoints(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate() {
        let profile = SvpProfile {
            source: "test".into(),
            cast_count: 3,
            points: vec![
                SvpPoint {
                    depth: 0.0,
                    speed: 1505.0,
                },
                SvpPoint {
                    depth: 10.0,
                    speed: 1500.0,
                },
                SvpPoint {
                    depth: 20.0,
                    speed: 1495.0,
                },
            ],
            min_depth: 0.0,
            max_depth: 20.0,
            min_speed: 1495.0,
            max_speed: 1505.0,
            surface_speed: 1505.0,
            bottom_speed: 1495.0,
        };

        assert!((interpolate_speed(&profile, 0.0) - 1505.0).abs() < 0.01);
        assert!((interpolate_speed(&profile, 5.0) - 1502.5).abs() < 0.01);
        assert!((interpolate_speed(&profile, 10.0) - 1500.0).abs() < 0.01);
        assert!((interpolate_speed(&profile, 15.0) - 1497.5).abs() < 0.01);
        assert!((interpolate_speed(&profile, 20.0) - 1495.0).abs() < 0.01);
        assert!((interpolate_speed(&profile, 25.0) - 1495.0).abs() < 0.01); // extrapolate
    }

    #[test]
    fn test_empty_profile_returns_default() {
        let profile = SvpProfile {
            source: "empty".into(),
            cast_count: 0,
            points: vec![],
            min_depth: 0.0,
            max_depth: 0.0,
            min_speed: 0.0,
            max_speed: 0.0,
            surface_speed: 0.0,
            bottom_speed: 0.0,
        };
        assert!((interpolate_speed(&profile, 10.0) - 1500.0).abs() < 0.01);
    }
}
