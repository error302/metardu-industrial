// Stockpile change detection — compare two LAS surveys of the same area
// from different epochs and compute per-cell cut/fill volumes.
//
// Use cases:
//   - Monthly inventory reconciliation: this month's stockpile drone
//     survey vs last month's → net change in m³ (and tonnage with a
//     density factor).
//   - Progress claims: excavation volume since the previous claim
//     baseline.
//   - Hotspot detection: regions where |Δz| exceeds a threshold,
//     flagging potential data errors, theft, or unexpected material
//     movement.
//
// Algorithm:
//   1. Read both LAS files and extract XYZ points.
//   2. Compute the union bounds (min/max X and Y across both files).
//   3. Rasterize each LAS into a regular grid at `cell_size` resolution
//      using a nearest-neighbour or median-of-cell strategy. Cells with
//      no data become NaN.
//   4. For each cell where both grids have data, compute Δz = current −
//      previous. Positive Δz = fill (material added). Negative Δz = cut
//      (material removed).
//   5. Integrate cut and fill volumes: sum(Δz * cell_area) for cut cells
//      and fill cells separately. Net change = fill − cut.
//   6. Identify hotspots: cells where |Δz| > hotspot_threshold_m.

use serde::Serialize;
use std::path::Path;

/// Result of a stockpile change-detection analysis.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeDetectionResult {
    /// Total cut volume (material removed), in cubic meters
    pub cut_volume_m3: f64,
    /// Total fill volume (material added), in cubic meters
    pub fill_volume_m3: f64,
    /// Net volume change (fill − cut), in cubic meters
    pub net_change_m3: f64,
    /// Number of cells where material was cut
    pub cut_cells: usize,
    /// Number of cells where material was filled
    pub fill_cells: usize,
    /// Number of cells where both surveys had data
    pub compared_cells: usize,
    /// Number of cells where only one survey had data
    pub no_overlap_cells: usize,
    /// Grid cell size used for the analysis (meters)
    pub cell_size_m: f64,
    /// Grid dimensions: (ncols, nrows)
    pub grid_dims: (usize, usize),
    /// Bounds of the analysis grid: (min_x, min_y, max_x, max_y)
    pub bounds: (f64, f64, f64, f64),
    /// Per-cell Δz values, row-major. NaN where no overlap. Length = ncols × nrows.
    pub delta_grid: Vec<f64>,
    /// Hotspot cells (row, col, delta_z) where |Δz| > threshold
    pub hotspots: Vec<(usize, usize, f64)>,
    /// Mean Δz (overlapping cells only)
    pub mean_delta: f64,
    /// Std-dev of Δz (overlapping cells only)
    pub std_delta: f64,
    /// Max positive Δz (largest fill)
    pub max_fill: f64,
    /// Max negative Δz (largest cut, as a negative number)
    pub max_cut: f64,
}

/// Compare two LAS files and produce a per-cell change-detection report.
///
/// `cell_size_m` controls the analysis resolution. Smaller = more
/// detailed but slower; 0.5 m is typical for drone surveys, 1.0 m for
/// stockpile yards, 2.0 m for pit-wide comparisons.
///
/// `hotspot_threshold_m` flags cells where |Δz| exceeds this value.
/// Useful for spotting data errors (e.g., a single 5 m spike in one
/// survey that wasn't in the other).
pub fn detect_stockpile_change(
    current_las: &Path,
    previous_las: &Path,
    cell_size_m: f64,
    hotspot_threshold_m: f64,
) -> Result<ChangeDetectionResult, String> {
    if cell_size_m <= 0.0 {
        return Err("cell_size_m must be > 0".to_string());
    }

    // Read points from both LAS files
    let current_points = crate::formats::las::read_points(current_las, 0)
        .map_err(|e| format!("reading current LAS: {e}"))?;
    let previous_points = crate::formats::las::read_points(previous_las, 0)
        .map_err(|e| format!("reading previous LAS: {e}"))?;

    if current_points.is_empty() {
        return Err("current LAS has no points".to_string());
    }
    if previous_points.is_empty() {
        return Err("previous LAS has no points".to_string());
    }

    // Compute union bounds
    let (min_x, max_x, min_y, max_y) = union_bounds(&current_points, &previous_points);

    // Grid dimensions
    let ncols = ((max_x - min_x) / cell_size_m).ceil() as usize;
    let nrows = ((max_y - min_y) / cell_size_m).ceil() as usize;
    if ncols == 0 || nrows == 0 || ncols > 5000 || nrows > 5000 {
        return Err(format!(
            "grid dimensions out of range: {ncols}×{nrows} (cell_size={cell_size_m}m, bounds={max_x}-{min_x} × {max_y}-{min_y})"
        ));
    }

    // Rasterize both point clouds to the grid using median-of-cell
    let current_grid = rasterize_median(&current_points, ncols, nrows, cell_size_m, min_x, min_y);
    let previous_grid = rasterize_median(&previous_points, ncols, nrows, cell_size_m, min_x, min_y);

    // Compute per-cell Δz and integrate
    let cell_area = cell_size_m * cell_size_m;
    let mut delta_grid = vec![f64::NAN; ncols * nrows];
    let mut cut_volume = 0.0;
    let mut fill_volume = 0.0;
    let mut cut_cells = 0usize;
    let mut fill_cells = 0usize;
    let mut compared_cells = 0usize;
    let mut no_overlap_cells = 0usize;
    let mut deltas: Vec<f64> = Vec::new();
    let mut max_fill = 0.0f64;
    let mut max_cut = 0.0f64;
    let mut hotspots: Vec<(usize, usize, f64)> = Vec::new();

    for row in 0..nrows {
        for col in 0..ncols {
            let idx = row * ncols + col;
            let c = current_grid[idx];
            let p = previous_grid[idx];
            if c.is_nan() || p.is_nan() {
                no_overlap_cells += 1;
                continue;
            }
            let delta = c - p;
            delta_grid[idx] = delta;
            compared_cells += 1;
            deltas.push(delta);

            if delta > 0.0 {
                fill_volume += delta * cell_area;
                fill_cells += 1;
                if delta > max_fill {
                    max_fill = delta;
                }
            } else if delta < 0.0 {
                cut_volume += (-delta) * cell_area;
                cut_cells += 1;
                if delta < max_cut {
                    max_cut = delta;
                }
            }

            if delta.abs() > hotspot_threshold_m {
                hotspots.push((row, col, delta));
            }
        }
    }

    let mean_delta = if deltas.is_empty() {
        0.0
    } else {
        let sum: f64 = deltas.iter().sum();
        sum / deltas.len() as f64
    };
    let std_delta = if deltas.is_empty() {
        0.0
    } else {
        let variance: f64 = deltas.iter().map(|d| (d - mean_delta).powi(2)).sum::<f64>()
            / deltas.len() as f64;
        variance.sqrt()
    };

    Ok(ChangeDetectionResult {
        cut_volume_m3: cut_volume,
        fill_volume_m3: fill_volume,
        net_change_m3: fill_volume - cut_volume,
        cut_cells,
        fill_cells,
        compared_cells,
        no_overlap_cells,
        cell_size_m,
        grid_dims: (ncols, nrows),
        bounds: (min_x, min_y, max_x, max_y),
        delta_grid,
        hotspots,
        mean_delta,
        std_delta,
        max_fill,
        max_cut,
    })
}

/// Compute the union bounds (min_x, max_x, min_y, max_y) across two
/// point sets.
fn union_bounds(
    a: &[(f64, f64, f64)],
    b: &[(f64, f64, f64)],
) -> (f64, f64, f64, f64) {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for (x, y, _) in a.iter().chain(b.iter()) {
        if *x < min_x { min_x = *x; }
        if *x > max_x { max_x = *x; }
        if *y < min_y { min_y = *y; }
        if *y > max_y { max_y = *y; }
    }
    (min_x, max_x, min_y, max_y)
}

/// Rasterize points into a grid using the median Z value per cell.
///
/// Median is preferred over mean for stockpile data because it's
/// robust to outliers (single noise points that fall inside a cell).
/// Cells with no points get NaN.
fn rasterize_median(
    points: &[(f64, f64, f64)],
    ncols: usize,
    nrows: usize,
    cell_size: f64,
    min_x: f64,
    min_y: f64,
) -> Vec<f64> {
    use std::collections::BTreeMap;
    // Bucket points by cell index
    let mut buckets: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
    for (x, y, z) in points {
        let col = (((*x - min_x) / cell_size) as usize).min(ncols - 1);
        // Y axis: LAS Y increases northward; grid row 0 = south
        let row = (((*y - min_y) / cell_size) as usize).min(nrows - 1);
        let idx = row * ncols + col;
        buckets.entry(idx).or_default().push(*z);
    }
    let mut grid = vec![f64::NAN; ncols * nrows];
    for (idx, zs) in buckets.iter() {
        if zs.is_empty() {
            continue;
        }
        let mut sorted = zs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        let median = if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        };
        grid[*idx] = median;
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_las(path: &Path, points: &[(f64, f64, f64)]) -> std::io::Result<()> {
        // Minimal LAS 1.2 writer — header + point data record format 0
        use std::io::Write;
        let n = points.len() as u32;
        let header_size: u32 = 227;
        let point_size: u32 = 20;
        let mut f = std::fs::File::create(path)?;
        // 4 bytes signature "LASF"
        f.write_all(b"LASF")?;
        // 4 bytes source ID
        f.write_all(&0u16.to_le_bytes())?;
        f.write_all(&0u16.to_le_bytes())?;
        // 4 bytes project ID GUID
        f.write_all(&[0u8; 16])?;
        // 1 byte version major, 1 byte minor
        f.write_all(&1u8)?;
        f.write_all(&2u8)?;
        // 32 bytes system identifier
        f.write_all(&[0u8; 32])?;
        // 32 bytes generating software
        f.write_all(&[0u8; 32])?;
        // 2 bytes file creation day of year
        f.write_all(&1u16.to_le_bytes())?;
        // 2 bytes file creation year
        f.write_all(&2024u16.to_le_bytes())?;
        // 2 bytes header size
        f.write_all(&header_size.to_le_bytes())?;
        // 4 bytes offset to point data
        let offset = header_size;
        f.write_all(&offset.to_le_bytes())?;
        // 4 bytes number of VLRs
        f.write_all(&0u32.to_le_bytes())?;
        // 1 byte point data record format
        f.write_all(&0u8)?;
        // 2 bytes point data record length
        f.write_all(&point_size.to_le_bytes())?;
        // 4 bytes legacy point count
        f.write_all(&n.to_le_bytes())?;
        // 4 bytes legacy points by return (5 × 4 bytes)
        f.write_all(&[0u8; 20])?;
        // 8 bytes scale factors X, Y, Z
        f.write_all(&0.001f64.to_le_bytes())?; // X scale
        f.write_all(&0.001f64.to_le_bytes())?; // Y scale
        f.write_all(&0.001f64.to_le_bytes())?; // Z scale
        // 8 bytes offset X, Y, Z
        f.write_all(&0f64.to_le_bytes())?;
        f.write_all(&0f64.to_le_bytes())?;
        f.write_all(&0f64.to_le_bytes())?;
        // 8 bytes max/min X
        let max_x = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let min_x = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_y = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_z = points.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);
        let min_z = points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        f.write_all(&max_x.to_le_bytes())?;
        f.write_all(&min_x.to_le_bytes())?;
        f.write_all(&max_y.to_le_bytes())?;
        f.write_all(&min_y.to_le_bytes())?;
        f.write_all(&max_z.to_le_bytes())?;
        f.write_all(&min_z.to_le_bytes())?;

        // Points — record format 0: 20 bytes each
        for (x, y, z) in points {
            let xi = (x / 0.001) as i32;
            let yi = (y / 0.001) as i32;
            let zi = (z / 0.001) as i32;
            f.write_all(&xi.to_le_bytes())?;
            f.write_all(&yi.to_le_bytes())?;
            f.write_all(&zi.to_le_bytes())?;
            f.write_all(&0u16.to_le_bytes())?; // intensity
            f.write_all(&0u8)?; // flags
            f.write_all(&0u8.to_le_bytes())?; // classification
            f.write_all(&0i8.to_le_bytes())?; // scan angle
            f.write_all(&0u8.to_le_bytes())?; // user data
            f.write_all(&0u16.to_le_bytes())?; // point source ID
        }
        Ok(())
    }

    #[test]
    fn test_change_detection_pure_fill() {
        let dir = std::env::temp_dir();
        let current = dir.join("sc_current.las");
        let previous = dir.join("sc_previous.las");

        // Previous: flat at z=0
        let prev_pts: Vec<(f64, f64, f64)> = (0..10).flat_map(|i| {
            (0..10).map(move |j| (i as f64, j as f64, 0.0))
        }).collect();
        // Current: flat at z=1 (pure fill of 1 m × 100 m² = 100 m³)
        let cur_pts: Vec<(f64, f64, f64)> = (0..10).flat_map(|i| {
            (0..10).map(move |j| (i as f64, j as f64, 1.0))
        }).collect();

        write_test_las(&current, &cur_pts).unwrap();
        write_test_las(&previous, &prev_pts).unwrap();

        let result = detect_stockpile_change(&current, &previous, 1.0, 0.5).unwrap();
        assert!((result.fill_volume_m3 - 100.0).abs() < 5.0, "fill volume should be ~100 m³, got {}", result.fill_volume_m3);
        assert!(result.cut_volume_m3 < 1.0, "cut should be ~0, got {}", result.cut_volume_m3);
        assert!((result.net_change_m3 - 100.0).abs() < 5.0);

        let _ = std::fs::remove_file(&current);
        let _ = std::fs::remove_file(&previous);
    }

    #[test]
    fn test_change_detection_pure_cut() {
        let dir = std::env::temp_dir();
        let current = dir.join("sc_cut_current.las");
        let previous = dir.join("sc_cut_prev.las");

        let prev_pts: Vec<(f64, f64, f64)> = (0..10).flat_map(|i| {
            (0..10).map(move |j| (i as f64, j as f64, 5.0))
        }).collect();
        let cur_pts: Vec<(f64, f64, f64)> = (0..10).flat_map(|i| {
            (0..10).map(move |j| (i as f64, j as f64, 2.0))
        }).collect();

        write_test_las(&current, &cur_pts).unwrap();
        write_test_las(&previous, &prev_pts).unwrap();

        let result = detect_stockpile_change(&current, &previous, 1.0, 0.5).unwrap();
        assert!((result.cut_volume_m3 - 300.0).abs() < 5.0, "cut volume should be ~300 m³ (3 m × 100 m²), got {}", result.cut_volume_m3);
        assert!(result.fill_volume_m3 < 1.0);
        assert!((result.net_change_m3 - (-300.0)).abs() < 5.0);

        let _ = std::fs::remove_file(&current);
        let _ = std::fs::remove_file(&previous);
    }

    #[test]
    fn test_hotspots() {
        let dir = std::env::temp_dir();
        let current = dir.join("sc_hot_current.las");
        let previous = dir.join("sc_hot_prev.las");

        // Previous: all flat
        let prev_pts: Vec<(f64, f64, f64)> = (0..10).flat_map(|i| {
            (0..10).map(move |j| (i as f64, j as f64, 0.0))
        }).collect();
        // Current: one spike at (5, 5) — 10 m tall
        let mut cur_pts: Vec<(f64, f64, f64)> = (0..10).flat_map(|i| {
            (0..10).map(move |j| (i as f64, j as f64, 0.0))
        }).collect();
        cur_pts.push((5.0, 5.0, 10.0));

        write_test_las(&current, &cur_pts).unwrap();
        write_test_las(&previous, &prev_pts).unwrap();

        let result = detect_stockpile_change(&current, &previous, 1.0, 2.0).unwrap();
        assert!(!result.hotspots.is_empty(), "should detect at least one hotspot");
        assert!(result.max_fill >= 9.0, "max_fill should be ~10, got {}", result.max_fill);

        let _ = std::fs::remove_file(&current);
        let _ = std::fs::remove_file(&previous);
    }

    #[test]
    fn test_invalid_cell_size() {
        let dir = std::env::temp_dir();
        let path = dir.join("sc_invalid.las");
        write_test_las(&path, &[(0.0, 0.0, 0.0)]).unwrap();
        let result = detect_stockpile_change(&path, &path, 0.0, 1.0);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&path);
    }
}
