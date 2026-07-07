// Real-time QC dashboard data — coverage, density, uncertainty statistics.
//
// During a hydrographic survey, the surveyor needs live feedback on:
//   1. Coverage — are all planned survey lines covered?
//   2. Density — is the sounding density sufficient for S-44?
//   3. Uncertainty — are the TPU values within specification?
//   4. Quality flags — how many beams are rejected vs accepted?
//
// This module computes these statistics from a set of soundings.

use serde::{Deserialize, Serialize};

/// QC statistics for a set of soundings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcStats {
    /// Total number of soundings
    pub total_soundings: usize,
    /// Number of soundings that passed quality check
    pub accepted_soundings: usize,
    /// Number rejected (quality flag too high)
    pub rejected_soundings: usize,
    /// Coverage area (square meters) — convex hull area approximation
    pub coverage_area_m2: f64,
    /// Average sounding density (soundings per square meter)
    pub avg_density_per_m2: f64,
    /// Minimum depth (meters)
    pub min_depth: f64,
    /// Maximum depth (meters)
    pub max_depth: f64,
    /// Mean depth (meters)
    pub mean_depth: f64,
    /// Standard deviation of depth (meters)
    pub std_depth: f64,
    /// Mean across-track beam angle (degrees)
    pub mean_beam_angle: f64,
    /// Maximum across-track beam angle (degrees)
    pub max_beam_angle: f64,
    /// Number of beams per ping (average)
    pub avg_beams_per_ping: usize,
    /// Number of pings
    pub ping_count: usize,
    /// S-44 order compliance (1a, 1b, 2, Special)
    pub s44_order: String,
    /// Percentage of cells meeting S-44 density requirement
    pub density_compliance_pct: f64,
    /// Percentage of soundings meeting S-44 uncertainty requirement
    pub uncertainty_compliance_pct: f64,
}

/// Compute QC statistics from sounding data.
///
/// `soundings` is a flat array of (x, y, depth, quality, beam_angle, uncertainty).
/// `cell_size` is the grid cell size for density computation (meters).
/// `s44_order` is the target S-44 order for compliance checking.
pub fn compute_qc_stats(
    soundings: &[(f64, f64, f64, u8, f64, f64)],
    cell_size: f64,
    s44_order: &str,
) -> Result<QcStats, String> {
    if soundings.is_empty() {
        return Err("no soundings".to_string());
    }

    let total = soundings.len();
    let accepted = soundings.iter().filter(|s| s.3 <= 3).count();
    let rejected = total - accepted;

    // Depth statistics
    let depths: Vec<f64> = soundings.iter().map(|s| s.2).collect();
    let min_depth = depths.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_depth = depths.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let mean_depth = depths.iter().sum::<f64>() / total as f64;
    let variance = depths.iter().map(|d| (d - mean_depth).powi(2)).sum::<f64>() / total as f64;
    let std_depth = variance.sqrt();

    // Beam angle statistics
    let beam_angles: Vec<f64> = soundings.iter().map(|s| s.4.abs()).collect();
    let mean_beam_angle = beam_angles.iter().sum::<f64>() / total as f64;
    let max_beam_angle = beam_angles.iter().fold(0.0f64, |a, &b| a.max(b));

    // Coverage area (bounding box approximation — real implementation
    // would use convex hull, but bbox is good enough for live QC)
    let (min_x, max_x, min_y, max_y) = soundings.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
        |(mnx, mxx, mny, mxy), s| {
            (mnx.min(s.0), mxx.max(s.0), mny.min(s.1), mxy.max(s.1))
        },
    );
    let coverage_area_m2 = (max_x - min_x) * (max_y - min_y);
    let avg_density_per_m2 = total as f64 / coverage_area_m2.max(1.0);

    // Ping count (unique ping numbers — we approximate by counting
    // unique timestamps within 0.01s)
    let mut ping_count = 0usize;
    let mut last_time = f64::NEG_INFINITY;
    for s in soundings {
        if (s.2 - last_time).abs() > 0.01 {
            ping_count += 1;
            last_time = s.2;
        }
    }
    let avg_beams_per_ping = if ping_count > 0 { total / ping_count } else { 0 };

    // S-44 compliance
    let (density_req, uncertainty_req) = match s44_order {
        "Special" => (25.0, 0.57), // 25 soundings/cell, 0.57m max uncertainty at 10m
        "1a" => (9.0, 0.75),
        "1b" => (4.0, 0.75),
        "2" => (1.0, 1.5),
        _ => (1.0, f64::MAX),
    };

    // Density compliance: grid the soundings and count cells meeting req
    let mut density_cells = 0;
    let mut total_cells = 0;
    if coverage_area_m2 > 0.0 && cell_size > 0.0 {
        let ncols = ((max_x - min_x) / cell_size).ceil() as usize + 1;
        let nrows = ((max_y - min_y) / cell_size).ceil() as usize + 1;
        let mut counts = vec![0u32; ncols * nrows];
        for s in soundings {
            let col = (((s.0 - min_x) / cell_size).floor() as usize).min(ncols - 1);
            let row = (((s.1 - min_y) / cell_size).floor() as usize).min(nrows - 1);
            counts[row * ncols + col] += 1;
        }
        for &c in &counts {
            if c > 0 {
                total_cells += 1;
                if c as f64 >= density_req {
                    density_cells += 1;
                }
            }
        }
    }
    let density_compliance_pct = if total_cells > 0 {
        density_cells as f64 / total_cells as f64 * 100.0
    } else {
        0.0
    };

    // Uncertainty compliance
    let uncertainty_compliant = soundings.iter().filter(|s| s.5 <= uncertainty_req).count();
    let uncertainty_compliance_pct = uncertainty_compliant as f64 / total as f64 * 100.0;

    Ok(QcStats {
        total_soundings: total,
        accepted_soundings: accepted,
        rejected_soundings: rejected,
        coverage_area_m2,
        avg_density_per_m2,
        min_depth,
        max_depth,
        mean_depth,
        std_depth,
        mean_beam_angle,
        max_beam_angle,
        avg_beams_per_ping,
        ping_count,
        s44_order: s44_order.to_string(),
        density_compliance_pct,
        uncertainty_compliance_pct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_qc_stats() {
        let soundings = vec![
            (0.0, 0.0, 10.0, 0, 0.0, 0.1),
            (5.0, 0.0, 12.0, 0, 15.0, 0.2),
            (10.0, 0.0, 15.0, 1, 30.0, 0.3),
            (0.0, 5.0, 11.0, 5, -15.0, 0.5), // rejected (quality=5)
            (5.0, 5.0, 13.0, 0, 0.0, 0.15),
        ];
        let stats = compute_qc_stats(&soundings, 5.0, "1a").unwrap();
        assert_eq!(stats.total_soundings, 5);
        assert_eq!(stats.accepted_soundings, 4);
        assert_eq!(stats.rejected_soundings, 1);
        assert!(stats.min_depth == 10.0);
        assert!(stats.max_depth == 15.0);
    }

    #[test]
    fn test_empty_soundings_error() {
        let result = compute_qc_stats(&[], 1.0, "1a");
        assert!(result.is_err());
    }
}
