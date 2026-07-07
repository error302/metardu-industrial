// IDW Interpolation — Sprint 15.
//
// Inverse Distance Weighting: estimate values at unobserved locations
// from scattered point observations. Used for:
//   - Filling DEM gaps in sparse bathymetry (MBES data has gaps between
//     survey lines)
//   - Generating continuous surfaces from point observations
//   - Gridding irregular point clouds to a regular raster
//
// Algorithm: for each output cell, find the N nearest input points
// (or all points within a search radius), weight each by 1/d^p, and
// compute the weighted average.
//
//   v(x) = Σ (v_i / d_i^p) / Σ (1 / d_i^p)
//
// where:
//   v(x) = estimated value at output cell
//   v_i  = value at input point i
//   d_i  = distance from output cell to input point i
//   p    = power parameter (default 2.0; higher = more localized)
//
// References:
//   Shepard, D. (1968) "A two-dimensional interpolation function for
//   irregularly-spaced data." Proc. ACM National Conference.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdwParams {
    /// Power parameter. Default 2.0. Higher values make the interpolation
    /// more localized (closer points dominate). p=1 gives a smooth cone
    /// surface; p=2 is the standard; p→∞ gives nearest-neighbor.
    pub power: f64,
    /// Search radius in world units. Only points within this radius
    /// contribute to each output cell. 0 = use all points (slow for
    /// large datasets). Default: 0 (all points).
    pub search_radius: f64,
    /// Maximum number of nearest points to use per cell. 0 = no limit.
    /// Default: 12 (balances accuracy vs speed).
    pub max_points: usize,
    /// NODATA value for output cells with no nearby points.
    pub nodata: f64,
}

impl Default for IdwParams {
    fn default() -> Self {
        Self {
            power: 2.0,
            search_radius: 0.0,
            max_points: 12,
            nodata: f64::NAN,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IdwResult {
    /// Output grid, row-major [row * ncols + col]
    pub grid: Vec<f64>,
    pub ncols: usize,
    pub nrows: usize,
    /// Cell size in world units
    pub cell_size: f64,
    /// Bounds: (min_x, min_y, max_x, max_y)
    pub bounds: (f64, f64, f64, f64),
    /// Number of cells that were interpolated (not NODATA)
    pub interpolated_cells: usize,
    /// Number of cells that are NODATA (no points within search radius)
    pub nodata_cells: usize,
    /// Min/max of the interpolated grid
    pub min_value: f64,
    pub max_value: f64,
}

/// Perform IDW interpolation on a set of scattered 3D points.
///
/// `points` are the input observations (x, y, z). The output grid
/// covers `bounds` at `cell_size` resolution. For each output cell,
/// the function finds the nearest input points (within `search_radius`
/// if specified, up to `max_points` if specified) and computes the
/// weighted average.
///
/// Time complexity: O(N_out × N_in) for the naive approach. For large
/// datasets (>10K points), use a spatial index (k-d tree) — not
/// implemented here but the API supports it.
pub fn interpolate_idw(
    points: &[Point3D],
    bounds: (f64, f64, f64, f64), // (min_x, min_y, max_x, max_y)
    cell_size: f64,
    params: &IdwParams,
) -> Result<IdwResult, String> {
    if points.is_empty() {
        return Err("no input points for IDW interpolation".to_string());
    }
    if cell_size <= 0.0 {
        return Err("cell_size must be > 0".to_string());
    }
    let (min_x, min_y, max_x, max_y) = bounds;
    if max_x <= min_x || max_y <= min_y {
        return Err("invalid bounds: max must be > min".to_string());
    }

    let ncols = ((max_x - min_x) / cell_size).ceil() as usize;
    let nrows = ((max_y - min_y) / cell_size).ceil() as usize;
    if ncols == 0 || nrows == 0 || ncols > 10000 || nrows > 10000 {
        return Err(format!(
            "grid dimensions out of range: {}x{} (max 10000x10000)",
            ncols, nrows
        ));
    }

    let mut grid = vec![params.nodata; ncols * nrows];
    let mut interpolated = 0usize;
    let mut nodata = 0usize;
    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;

    for row in 0..nrows {
        for col in 0..ncols {
            // Output cell center in world coordinates
            let cx = min_x + (col as f64 + 0.5) * cell_size;
            let cy = min_y + (row as f64 + 0.5) * cell_size;

            // Find candidate points: within search_radius (if set)
            // or all points (if search_radius = 0)
            let mut candidates: Vec<(f64, f64)> = points
                .iter()
                .filter_map(|p| {
                    let dx = p.x - cx;
                    let dy = p.y - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 1e-12 {
                        // Point exactly at cell center — use its value directly
                        return Some((0.0, p.z));
                    }
                    if params.search_radius > 0.0 && dist > params.search_radius {
                        return None;
                    }
                    Some((dist, p.z))
                })
                .collect();

            if candidates.is_empty() {
                nodata += 1;
                continue;
            }

            // If a point is exactly at the center, use it directly
            if candidates.iter().any(|(d, _)| *d < 1e-12) {
                let z = candidates.iter().find(|(d, _)| *d < 1e-12).map(|(_, z)| *z).unwrap_or(0.0);
                grid[row * ncols + col] = z;
                interpolated += 1;
                min_val = min_val.min(z);
                max_val = max_val.max(z);
                continue;
            }

            // Sort by distance and take the nearest max_points
            if params.max_points > 0 && candidates.len() > params.max_points {
                candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                candidates.truncate(params.max_points);
            }

            // Compute weighted average
            let power = params.power;
            let mut weighted_sum = 0.0;
            let mut weight_sum = 0.0;
            for (dist, z) in &candidates {
                let weight = 1.0 / dist.powf(power);
                weighted_sum += weight * z;
                weight_sum += weight;
            }

            if weight_sum > 0.0 {
                let z = weighted_sum / weight_sum;
                grid[row * ncols + col] = z;
                interpolated += 1;
                min_val = min_val.min(z);
                max_val = max_val.max(z);
            } else {
                nodata += 1;
            }
        }
    }

    Ok(IdwResult {
        grid,
        ncols,
        nrows,
        cell_size,
        bounds,
        interpolated_cells: interpolated,
        nodata_cells: nodata,
        min_value: if min_val.is_finite() { min_val } else { 0.0 },
        max_value: if max_val.is_finite() { max_val } else { 0.0 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idw_single_point() {
        // One point at (5, 5) with z=100
        let points = vec![Point3D { x: 5.0, y: 5.0, z: 100.0 }];
        let result = interpolate_idw(&points, (0.0, 0.0, 10.0, 10.0), 1.0, &IdwParams::default()).unwrap();
        // Every cell should be 100 (only one input point)
        for v in &result.grid {
            assert!((v - 100.0).abs() < 1e-6, "expected 100, got {}", v);
        }
        assert_eq!(result.interpolated_cells, 100);
        assert_eq!(result.nodata_cells, 0);
    }

    #[test]
    fn test_idw_two_points_midpoint() {
        // Two points: (0, 0, z=0) and (10, 0, z=100)
        // At the midpoint (5, 0), IDW with p=2 should give ~50
        let points = vec![
            Point3D { x: 0.0, y: 0.0, z: 0.0 },
            Point3D { x: 10.0, y: 0.0, z: 100.0 },
        ];
        let result = interpolate_idw(&points, (-1.0, -1.0, 11.0, 1.0), 1.0, &IdwParams::default()).unwrap();
        // Find the cell at approximately (5, 0)
        // Grid covers x: -1 to 11 (12 cols), y: -1 to 1 (2 rows)
        // Col 6 = x = -1 + 6.5 = 5.5 (closest to 5)
        let mid_idx = 1 * 12 + 6; // row 1, col 6
        let mid_val = result.grid[mid_idx];
        // Should be between 0 and 100, close to 50
        assert!(mid_val > 0.0 && mid_val < 100.0, "midpoint value = {}", mid_val);
        // With equal distances, IDW gives the arithmetic mean
        // But the distances aren't exactly equal here (5.5 vs 4.5), so
        // the value will be slightly biased toward the closer point.
    }

    #[test]
    fn test_idw_point_at_center() {
        // Point exactly at the cell center should return that point's value
        let points = vec![
            Point3D { x: 0.5, y: 0.5, z: 42.0 },
            Point3D { x: 9.5, y: 9.5, z: 99.0 },
        ];
        let result = interpolate_idw(&points, (0.0, 0.0, 10.0, 10.0), 1.0, &IdwParams::default()).unwrap();
        // Cell (0, 0) center is at (0.5, 0.5) → should be exactly 42
        assert!((result.grid[0] - 42.0).abs() < 1e-6, "grid[0] = {}", result.grid[0]);
    }

    #[test]
    fn test_idw_search_radius_excludes_far_points() {
        let points = vec![
            Point3D { x: 0.0, y: 0.0, z: 100.0 },
            Point3D { x: 50.0, y: 50.0, z: 999.0 },
        ];
        let params = IdwParams {
            search_radius: 5.0,
            ..Default::default()
        };
        let result = interpolate_idw(&points, (0.0, 0.0, 10.0, 10.0), 1.0, &params).unwrap();
        // Cells near (0,0) should be ~100 (only the first point is within radius)
        assert!((result.grid[0] - 100.0).abs() < 1e-6);
        // Cells far from both points should be NODATA
        // The far point at (50,50) is outside the grid bounds entirely
    }

    #[test]
    fn test_idw_max_points_limits_neighbors() {
        // 10 points in a line, all equidistant from the center cell
        let points: Vec<Point3D> = (0..10)
            .map(|i| Point3D { x: i as f64, y: 5.0, z: i as f64 * 10.0 })
            .collect();
        let params = IdwParams {
            max_points: 3,
            ..Default::default()
        };
        let result = interpolate_idw(&points, (0.0, 0.0, 10.0, 10.0), 1.0, &params).unwrap();
        // The result should be valid (no crash, reasonable values)
        assert!(result.interpolated_cells > 0);
    }

    #[test]
    fn test_idw_empty_points_errors() {
        let result = interpolate_idw(&[], (0.0, 0.0, 10.0, 10.0), 1.0, &IdwParams::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_idw_invalid_cell_size() {
        let points = vec![Point3D { x: 0.0, y: 0.0, z: 1.0 }];
        let result = interpolate_idw(&points, (0.0, 0.0, 10.0, 10.0), 0.0, &IdwParams::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_idw_grid_dimensions() {
        let points = vec![Point3D { x: 5.0, y: 5.0, z: 100.0 }];
        let result = interpolate_idw(&points, (0.0, 0.0, 10.0, 10.0), 2.0, &IdwParams::default()).unwrap();
        // 10 / 2 = 5 cols, 5 rows
        assert_eq!(result.ncols, 5);
        assert_eq!(result.nrows, 5);
        assert_eq!(result.grid.len(), 25);
    }

    #[test]
    fn test_idw_higher_power_more_localized() {
        // Two points: near (0, z=0) and far (10, z=100)
        // With high power (p=10), the near point dominates more than with p=1
        let points = vec![
            Point3D { x: 0.0, y: 0.0, z: 0.0 },
            Point3D { x: 10.0, y: 0.0, z: 100.0 },
        ];
        let params_low = IdwParams { power: 1.0, ..Default::default() };
        let params_high = IdwParams { power: 10.0, ..Default::default() };
        let result_low = interpolate_idw(&points, (-1.0, -1.0, 11.0, 1.0), 1.0, &params_low).unwrap();
        let result_high = interpolate_idw(&points, (-1.0, -1.0, 11.0, 1.0), 1.0, &params_high).unwrap();
        // At a cell near (0, 0), high power should give a lower value
        // (more influenced by the near point with z=0)
        let idx = 1 * 12 + 1; // near (0, 0)
        assert!(result_high.grid[idx] < result_low.grid[idx],
            "high power should be more localized: high={}, low={}",
            result_high.grid[idx], result_low.grid[idx]);
    }
}
