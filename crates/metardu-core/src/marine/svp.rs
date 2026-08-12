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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RayTraceResult {
    pub true_depth: f64,       // meters relative to transducer
    pub horizontal_offset: f64, // meters relative to transducer along beam azimuth
    pub true_beam_angle_deg: f64, // final angle at target depth
    pub path_distance: f64,     // total curved distance along ray path
}

/// Perform Snell's Law ray tracing through multi-layer sound velocity profile.
/// Given launch_angle_deg (0 deg = straight down, positive towards port/starboard)
/// and one_way_travel_time_sec, returns the exact (true_depth, horizontal_offset).
pub fn ray_trace_beam(
    profile: &SvpProfile,
    launch_angle_deg: f64,
    one_way_travel_time_sec: f64,
) -> RayTraceResult {
    if profile.points.len() < 2 || one_way_travel_time_sec <= 0.0 {
        let v = if profile.points.is_empty() {
            1500.0
        } else {
            profile.points[0].speed
        };
        let rad = launch_angle_deg.to_radians();
        let dist = v * one_way_travel_time_sec;
        return RayTraceResult {
            true_depth: dist * rad.cos(),
            horizontal_offset: dist * rad.sin(),
            true_beam_angle_deg: launch_angle_deg,
            path_distance: dist,
        };
    }

    let angle_rad = launch_angle_deg.abs().to_radians();
    let sign = if launch_angle_deg < 0.0 { -1.0 } else { 1.0 };
    let v0 = profile.points[0].speed;
    let p = angle_rad.sin() / v0; // Ray parameter (Snell's Law invariant)

    // Handle vertical beam (angle = 0)
    if p.abs() < 1e-12 {
        let mut time_rem = one_way_travel_time_sec;
        let mut curr_z = profile.points[0].depth;
        let mut total_dist = 0.0;

        for i in 0..profile.points.len() - 1 {
            let z1 = profile.points[i].depth;
            let z2 = profile.points[i + 1].depth;
            let v1 = profile.points[i].speed;
            let v2 = profile.points[i + 1].speed;
            let dz = z2 - z1;
            if dz <= 0.0 {
                continue;
            }

            let g = (v2 - v1) / dz;
            let dt = if g.abs() < 1e-6 {
                dz / v1
            } else {
                (v2 / v1).ln() / g
            };

            if time_rem <= dt {
                let actual_dz = if g.abs() < 1e-6 {
                    time_rem * v1
                } else {
                    v1 * ((g * time_rem).exp() - 1.0) / g
                };
                curr_z += actual_dz;
                total_dist += actual_dz;
                time_rem = 0.0;
                break;
            } else {
                time_rem -= dt;
                curr_z = z2;
                total_dist += dz;
            }
        }

        if time_rem > 0.0 {
            let last_v = profile.points.last().unwrap().speed;
            let extra_dz = time_rem * last_v;
            curr_z += extra_dz;
            total_dist += extra_dz;
        }

        return RayTraceResult {
            true_depth: curr_z - profile.points[0].depth,
            horizontal_offset: 0.0,
            true_beam_angle_deg: 0.0,
            path_distance: total_dist,
        };
    }

    let mut time_rem = one_way_travel_time_sec;
    let mut x_acc = 0.0;
    let mut z_acc = 0.0;
    let mut total_path = 0.0;
    let mut current_angle_rad = angle_rad;

    for i in 0..profile.points.len() - 1 {
        let z1 = profile.points[i].depth;
        let z2 = profile.points[i + 1].depth;
        let v1 = profile.points[i].speed;
        let v2 = profile.points[i + 1].speed;
        let dz = z2 - z1;
        if dz <= 0.0 {
            continue;
        }

        let sin_theta1 = (p * v1).min(1.0);
        let cos_theta1 = (1.0 - sin_theta1 * sin_theta1).sqrt();
        let sin_theta2 = (p * v2).min(1.0);
        let cos_theta2 = (1.0 - sin_theta2 * sin_theta2).sqrt();

        let g = (v2 - v1) / dz;
        let (dx, dt, actual_dz, next_angle) = if g.abs() < 1e-6 {
            let tan_th = sin_theta1 / cos_theta1;
            let dx_layer = dz * tan_th;
            let dt_layer = dz / (v1 * cos_theta1);
            (dx_layer, dt_layer, dz, angle_rad)
        } else {
            let dx_layer = (cos_theta1 - cos_theta2) / (p * g);
            let dt_layer = (v2 * (1.0 + cos_theta1) / (v1 * (1.0 + cos_theta2))).ln() / g;
            (dx_layer, dt_layer, dz, sin_theta2.asin())
        };

        if time_rem <= dt {
            let frac = time_rem / dt;
            x_acc += dx * frac;
            z_acc += actual_dz * frac;
            total_path += (dx * dx + actual_dz * actual_dz).sqrt() * frac;
            current_angle_rad = sin_theta1.asin();
            time_rem = 0.0;
            break;
        } else {
            x_acc += dx;
            z_acc += actual_dz;
            total_path += (dx * dx + actual_dz * actual_dz).sqrt();
            current_angle_rad = next_angle;
            time_rem -= dt;
        }
    }

    if time_rem > 0.0 {
        let last_v = profile.points.last().unwrap().speed;
        let sin_th = (p * last_v).min(1.0);
        let cos_th = (1.0 - sin_th * sin_th).sqrt();
        let dist = time_rem * last_v;
        x_acc += dist * sin_th;
        z_acc += dist * cos_th;
        total_path += dist;
        current_angle_rad = sin_th.asin();
    }

    RayTraceResult {
        true_depth: z_acc,
        horizontal_offset: x_acc * sign,
        true_beam_angle_deg: current_angle_rad.to_degrees() * sign,
        path_distance: total_path,
    }
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

    #[test]
    fn test_ray_tracing_vertical_and_oblique() {
        let profile = SvpProfile {
            source: "test_cast".into(),
            cast_count: 2,
            points: vec![
                SvpPoint {
                    depth: 0.0,
                    speed: 1500.0,
                },
                SvpPoint {
                    depth: 100.0,
                    speed: 1500.0,
                },
            ],
            min_depth: 0.0,
            max_depth: 100.0,
            min_speed: 1500.0,
            max_speed: 1500.0,
            surface_speed: 1500.0,
            bottom_speed: 1500.0,
        };

        // Vertical beam: 100m depth at 1500m/s takes 100/1500 = 0.066667s
        let vert = ray_trace_beam(&profile, 0.0, 100.0 / 1500.0);
        assert!((vert.true_depth - 100.0).abs() < 1e-4);
        assert!((vert.horizontal_offset - 0.0).abs() < 1e-4);

        // Oblique beam at 45 deg: 100m straight line distance takes 100/1500s
        let obl = ray_trace_beam(&profile, 45.0, 100.0 / 1500.0);
        let expected_depth = 100.0 * (45.0f64.to_radians().cos());
        let expected_offset = 100.0 * (45.0f64.to_radians().sin());
        assert!((obl.true_depth - expected_depth).abs() < 1e-3);
        assert!((obl.horizontal_offset - expected_offset).abs() < 1e-3);
    }
}

