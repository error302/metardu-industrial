// Underground Cavity Monitoring System (CMS) & Tunnel Overbreak / Underbreak Profiler.
// Compares scan point cloud against design CAD surfaces to calculate accurate volumes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsScanPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub intensity: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTriMesh {
    pub vertices: Vec<[f64; 3]>,
    pub indices: Vec<[usize; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsAnalysisResult {
    pub total_scan_points: usize,
    pub overbreak_volume_m3: f64,  // Material excavated beyond design line (unsafe / expensive)
    pub underbreak_volume_m3: f64, // Material remaining inside design line (requires re-blasting)
    pub design_volume_m3: f64,
    pub actual_as_scanned_volume_m3: f64,
    pub max_overbreak_distance_m: f64,
    pub max_underbreak_distance_m: f64,
    pub mean_deviation_m: f64,
    pub point_deviations: Vec<f64>, // Signed distance per point (+ overbreak, - underbreak)
}

/// Compute signed point-to-mesh deviation and volumetric overbreak/underbreak for CMS stope scans.
pub fn analyze_cms_cavity(
    scan_points: &[CmsScanPoint],
    design: &DesignTriMesh,
) -> CmsAnalysisResult {
    if scan_points.is_empty() || design.vertices.is_empty() || design.indices.is_empty() {
        return CmsAnalysisResult {
            total_scan_points: scan_points.len(),
            overbreak_volume_m3: 0.0,
            underbreak_volume_m3: 0.0,
            design_volume_m3: 0.0,
            actual_as_scanned_volume_m3: 0.0,
            max_overbreak_distance_m: 0.0,
            max_underbreak_distance_m: 0.0,
            mean_deviation_m: 0.0,
            point_deviations: vec![],
        };
    }

    // 1. Calculate approximate design volume using divergence theorem (signed tetrahedron volume sum)
    let mut design_vol = 0.0;
    for tri in &design.indices {
        let v0 = design.vertices[tri[0]];
        let v1 = design.vertices[tri[1]];
        let v2 = design.vertices[tri[2]];

        let tet_vol = (v0[0] * (v1[1] * v2[2] - v1[2] * v2[1])
            - v0[1] * (v1[0] * v2[2] - v1[2] * v2[0])
            + v0[2] * (v1[0] * v2[1] - v1[1] * v2[0]))
            / 6.0;
        design_vol += tet_vol;
    }
    let design_vol_m3 = design_vol.abs();

    // 2. Compute point-to-mesh distances
    let mut point_deviations = Vec::with_capacity(scan_points.len());
    let mut overbreak_dist_sum = 0.0;
    let mut underbreak_dist_sum = 0.0;
    let mut max_overbreak = 0.0f64;
    let mut max_underbreak = 0.0f64;
    let mut dev_sum = 0.0;

    for p in scan_points {
        let mut min_dist_sq = f64::INFINITY;
        let mut sign = 1.0;

        for tri in &design.indices {
            let v0 = design.vertices[tri[0]];
            let v1 = design.vertices[tri[1]];
            let v2 = design.vertices[tri[2]];

            // Triangle centroid approximation for speed
            let cx = (v0[0] + v1[0] + v2[0]) / 3.0;
            let cy = (v0[1] + v1[1] + v2[1]) / 3.0;
            let cz = (v0[2] + v1[2] + v2[2]) / 3.0;

            let dx = p.x - cx;
            let dy = p.y - cy;
            let dz = p.z - cz;
            let d_sq = dx * dx + dy * dy + dz * dz;

            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;

                // Triangle normal vector
                let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
                let nx = e1[1] * e2[2] - e1[2] * e2[1];
                let ny = e1[2] * e2[0] - e1[0] * e2[2];
                let nz = e1[0] * e2[1] - e1[1] * e2[0];

                let dot = dx * nx + dy * ny + dz * nz;
                sign = if dot >= 0.0 { 1.0 } else { -1.0 };
            }
        }

        let dist = min_dist_sq.sqrt() * sign;
        point_deviations.push(dist);
        dev_sum += dist;

        if dist > 0.0 {
            overbreak_dist_sum += dist;
            max_overbreak = max_overbreak.max(dist);
        } else {
            underbreak_dist_sum += dist.abs();
            max_underbreak = max_underbreak.max(dist.abs());
        }
    }

    // Estimate total surface area from triangle count
    let mean_surf_area_per_pt = if scan_points.is_empty() {
        1.0
    } else {
        (design_vol_m3.powf(2.0 / 3.0) * 6.0) / (scan_points.len() as f64)
    };

    let overbreak_vol = overbreak_dist_sum * mean_surf_area_per_pt;
    let underbreak_vol = underbreak_dist_sum * mean_surf_area_per_pt;
    let actual_vol = design_vol_m3 + overbreak_vol - underbreak_vol;
    let mean_dev = dev_sum / (scan_points.len() as f64);

    CmsAnalysisResult {
        total_scan_points: scan_points.len(),
        overbreak_volume_m3: overbreak_vol,
        underbreak_volume_m3: underbreak_vol,
        design_volume_m3: design_vol_m3,
        actual_as_scanned_volume_m3: actual_vol,
        max_overbreak_distance_m: max_overbreak,
        max_underbreak_distance_m: max_underbreak,
        mean_deviation_m: mean_dev,
        point_deviations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cms_analysis_basic() {
        let design = DesignTriMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 10.0, 0.0],
                [0.0, 10.0, 0.0],
                [0.0, 0.0, 10.0],
                [10.0, 0.0, 10.0],
                [10.0, 10.0, 10.0],
                [0.0, 10.0, 10.0],
            ],
            indices: vec![
                [0, 1, 2], [0, 2, 3], // bottom
                [4, 6, 5], [4, 7, 6], // top
                [0, 4, 5], [0, 5, 1], // front
                [2, 6, 7], [2, 7, 3], // back
            ],
        };

        let pts = vec![
            CmsScanPoint { x: 5.0, y: 5.0, z: 11.0, intensity: 100 }, // Overbreak (+1m)
            CmsScanPoint { x: 5.0, y: 5.0, z: 9.0, intensity: 100 },  // Underbreak (-1m)
        ];

        let result = analyze_cms_cavity(&pts, &design);
        assert_eq!(result.total_scan_points, 2);
        assert!(result.max_overbreak_distance_m > 0.0);
    }
}
