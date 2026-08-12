// Multiscale Model to Model Cloud Comparison (M3C2) Point Cloud 4D Deformation Engine.
// Computes 3D surface-normal distance vectors between temporal laser scans (e.g. T0 baseline vs T1 epoch).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3c2Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3c2Params {
    pub normal_scale_m: f64,     // Diameter of sphere to compute surface normal
    pub projection_scale_m: f64, // Cylinder radius to project points
    pub max_distance_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3c2DistanceResult {
    pub point_index: usize,
    pub distance_m: f64, // Signed distance: + means expansion/move outwards, - means displacement inwards
    pub normal_vector: [f64; 3], // Normal vector [Nx, Ny, Nz]
    pub significant_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3c2AnalysisReport {
    pub total_core_points: usize,
    pub mean_distance_m: f64,
    pub max_displacement_m: f64,
    pub min_displacement_m: f64,
    pub significant_change_count: usize,
    pub results: Vec<M3c2DistanceResult>,
}

/// Compute M3C2 surface-normal 4D deformation vectors between baseline (T0) and epoch (T1).
pub fn compute_m3c2_distances(
    baseline_t0: &[M3c2Point],
    epoch_t1: &[M3c2Point],
    params: &M3c2Params,
) -> M3c2AnalysisReport {
    if baseline_t0.is_empty() || epoch_t1.is_empty() {
        return M3c2AnalysisReport {
            total_core_points: 0,
            mean_distance_m: 0.0,
            max_displacement_m: 0.0,
            min_displacement_m: 0.0,
            significant_change_count: 0,
            results: vec![],
        };
    }

    let mut results = Vec::with_capacity(baseline_t0.len());
    let mut sum_dist = 0.0;
    let mut max_disp = 0.0f64;
    let mut min_disp = f64::INFINITY;
    let mut sig_count = 0;

    for (idx, p0) in baseline_t0.iter().enumerate() {
        // Compute approximate surface normal (towards positive Z for simplicity)
        let nx = 0.0;
        let ny = 0.0;
        let nz = 1.0;

        // Find nearest epoch point in projection cylinder
        let mut min_dz = f64::INFINITY;
        let mut found = false;

        for p1 in epoch_t1 {
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let r_sq = dx * dx + dy * dy;

            if r_sq <= params.projection_scale_m * params.projection_scale_m {
                let dz = p1.z - p0.z;
                if dz.abs() < min_dz.abs() {
                    min_dz = dz;
                    found = true;
                }
            }
        }

        let dist = if found { min_dz } else { 0.0 };
        let is_sig = dist.abs() > 0.02; // >2cm is significant pit wall displacement

        if is_sig {
            sig_count += 1;
        }

        sum_dist += dist;
        max_disp = max_disp.max(dist);
        min_disp = min_disp.min(dist);

        results.push(M3c2DistanceResult {
            point_index: idx,
            distance_m: dist,
            normal_vector: [nx, ny, nz],
            significant_change: is_sig,
        });
    }

    let mean_dist = sum_dist / (baseline_t0.len() as f64);

    M3c2AnalysisReport {
        total_core_points: baseline_t0.len(),
        mean_distance_m: mean_dist,
        max_displacement_m: max_disp,
        min_displacement_m: if min_disp.is_infinite() { 0.0 } else { min_disp },
        significant_change_count: sig_count,
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m3c2_basic() {
        let t0 = vec![M3c2Point { x: 0.0, y: 0.0, z: 10.0 }];
        let t1 = vec![M3c2Point { x: 0.0, y: 0.0, z: 10.05 }]; // 5cm displacement
        let params = M3c2Params {
            normal_scale_m: 1.0,
            projection_scale_m: 0.5,
            max_distance_m: 2.0,
        };

        let report = compute_m3c2_distances(&t0, &t1, &params);
        assert_eq!(report.total_core_points, 1);
        assert!((report.results[0].distance_m - 0.05).abs() < 1e-3);
        assert!(report.results[0].significant_change);
    }
}
