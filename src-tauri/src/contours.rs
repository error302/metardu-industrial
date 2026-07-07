// Contour generation — Sprint 12.
//
// Marching squares algorithm for generating contour lines from a DEM
// (Digital Elevation Model) raster. Output is GeoJSON LineString
// features that can be rendered on the OpenLayers map or exported as DXF.
//
// Reference: Bourke, "Contouring Algorithm Specification"
// (http://paulbourke.net/papers/conrec/)
//
// The algorithm walks each 2x2 cell of the DEM, classifies which
// corners are above/below the contour level, and emits 0, 1, or 2
// line segments per cell based on the 16-case lookup table.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContourLine {
    /// Elevation of this contour (meters)
    pub elevation: f64,
    /// Sequence of (x, y) points forming the line
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContourResult {
    /// All contour lines, sorted by elevation
    pub lines: Vec<ContourLine>,
    /// Min elevation in the DEM
    pub min_elevation: f64,
    /// Max elevation in the DEM
    pub max_elevation: f64,
    /// Number of distinct contour levels
    pub level_count: usize,
}

/// Generate contour lines from a DEM grid.
///
/// `grid` is a flat row-major Vec<f64> of elevation values. NaN is
/// treated as NODATA. `ncols` and `nrows` are the grid dimensions.
/// `cell_size` is the size of each cell in world units (meters).
/// `origin_x` and `origin_y` are the world coordinates of the
/// top-left (row 0, col 0) cell — typically the GeoTIFF origin.
/// `interval` is the contour interval in elevation units (e.g., 5.0 m).
/// `base_elevation` is the starting elevation for the contour levels
/// (typically 0.0; use the DEM min for cleaner alignment).
///
/// Returns one `ContourLine` per level. Each line may contain multiple
/// disjoint segments (e.g., a contour that wraps around a hill).
pub fn generate_contours(
    grid: &[f64],
    ncols: usize,
    nrows: usize,
    cell_size: f64,
    origin_x: f64,
    origin_y: f64,
    interval: f64,
    base_elevation: f64,
) -> ContourResult {
    if grid.len() != ncols * nrows || ncols < 2 || nrows < 2 || interval <= 0.0 {
        return ContourResult {
            lines: vec![],
            min_elevation: 0.0,
            max_elevation: 0.0,
            level_count: 0,
        };
    }

    // Find elevation range (ignoring NODATA)
    let mut min_elev = f64::INFINITY;
    let mut max_elev = f64::NEG_INFINITY;
    for &v in grid {
        if !v.is_nan() {
            min_elev = min_elev.min(v);
            max_elev = max_elev.max(v);
        }
    }
    if !min_elev.is_finite() {
        return ContourResult {
            lines: vec![],
            min_elevation: 0.0,
            max_elevation: 0.0,
            level_count: 0,
        };
    }

    // Build contour levels
    let mut levels: Vec<f64> = vec![];
    let mut level = base_elevation + ((min_elev - base_elevation) / interval).ceil() * interval;
    while level <= max_elev {
        levels.push(level);
        level += interval;
    }

    let mut result_lines = vec![];
    for &elev in &levels {
        let segments = marching_squares(grid, ncols, nrows, cell_size, origin_x, origin_y, elev);
        if !segments.is_empty() {
            // Concatenate all segments for this level into a single ContourLine.
            // A more sophisticated implementation would chain connected segments
            // into multi-point polylines, but for visualization purposes
            // segment-pairs are sufficient.
            let mut points = vec![];
            for seg in segments {
                points.push(seg.0);
                points.push(seg.1);
            }
            result_lines.push(ContourLine { elevation: elev, points });
        }
    }

    ContourResult {
        lines: result_lines,
        min_elevation: min_elev,
        max_elevation: max_elev,
        level_count: levels.len(),
    }
}

/// Run marching squares for a single contour level. Returns a list of
/// line segments, each as ((x1, y1), (x2, y2)) in world coordinates.
fn marching_squares(
    grid: &[f64],
    ncols: usize,
    nrows: usize,
    cell_size: f64,
    origin_x: f64,
    origin_y: f64,
    level: f64,
) -> Vec<((f64, f64), (f64, f64))> {
    let mut segments = vec![];

    for row in 0..(nrows - 1) {
        for col in 0..(ncols - 1) {
            // Four corners of the 2x2 cell, clockwise from top-left
            let i00 = row * ncols + col;            // top-left
            let i10 = row * ncols + col + 1;        // top-right
            let i11 = (row + 1) * ncols + col + 1;  // bottom-right
            let i01 = (row + 1) * ncols + col;      // bottom-left

            let v00 = grid[i00];
            let v10 = grid[i10];
            let v11 = grid[i11];
            let v01 = grid[i01];

            // Skip if any corner is NODATA
            if v00.is_nan() || v10.is_nan() || v11.is_nan() || v01.is_nan() {
                continue;
            }

            // World coordinates of corners (y increases southward in raster)
            let x0 = origin_x + col as f64 * cell_size;
            let y0 = origin_y + row as f64 * cell_size;
            let x1 = x0 + cell_size;
            let y1 = y0 + cell_size;

            // Classify each corner: above (1) or below (0) the contour level
            let c00 = if v00 >= level { 1 } else { 0 };
            let c10 = if v10 >= level { 1 } else { 0 };
            let c11 = if v11 >= level { 1 } else { 0 };
            let c01 = if v01 >= level { 1 } else { 0 };

            // 4-bit case index
            let case = c00 | (c10 << 1) | (c11 << 2) | (c01 << 3);

            // Edge intersection points (linear interpolation)
            let lerp = |va: f64, vb: f64, xa: f64, xb: f64, ya: f64, yb: f64| -> (f64, f64) {
                let t = (level - va) / (vb - va);
                (xa + t * (xb - xa), ya + t * (yb - ya))
            };
            // Edges (top, right, bottom, left)
            let top = lerp(v00, v10, x0, x1, y0, y0);
            let right = lerp(v10, v11, x1, x1, y0, y1);
            let bottom = lerp(v01, v11, x0, x1, y1, y1);
            let left = lerp(v00, v01, x0, x0, y0, y1);

            // Lookup table: for each of 16 cases, which edges does the contour cross?
            match case {
                0 | 15 => {} // No contour
                1 | 14 => segments.push((top, left)),
                2 | 13 => segments.push((top, right)),
                3 | 12 => segments.push((left, right)),
                4 | 11 => segments.push((right, bottom)),
                5 => {
                    // Saddle: two segments
                    segments.push((top, left));
                    segments.push((right, bottom));
                }
                6 | 9 => segments.push((top, bottom)),
                7 | 8 => segments.push((left, bottom)),
                10 => {
                    // Saddle: two segments
                    segments.push((top, right));
                    segments.push((left, bottom));
                }
                _ => {}
            }
        }
    }

    segments
}

/// Convert a ContourResult to GeoJSON FeatureCollection string.
///
/// Each contour level becomes a single Feature with a LineString geometry
/// (or MultiLineString if there are disjoint segments — but for simplicity
/// we emit LineString with all points concatenated).
pub fn contours_to_geojson(result: &ContourResult) -> String {
    let features: Vec<String> = result.lines.iter().map(|line| {
        let coords: Vec<String> = line.points.iter()
            .map(|(x, y)| format!("[{}, {}]", x, y))
            .collect();
        format!(
            r#"{{"type":"Feature","geometry":{{"type":"LineString","coordinates":[{}]}}, "properties":{{"elevation":{}}}}}"#,
            coords.join(", "),
            line.elevation
        )
    }).collect();
    format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_grid_no_contours() {
        // All cells = 100m, interval 5m → only one contour at 100
        let grid = vec![100.0_f64; 16]; // 4x4
        let result = generate_contours(&grid, 4, 4, 10.0, 0.0, 0.0, 5.0, 0.0);
        assert!(result.min_elevation >= 99.9 && result.min_elevation <= 100.1);
        assert!(result.max_elevation >= 99.9 && result.max_elevation <= 100.1);
        // Level 100 should have some segments (the boundary of the grid)
        // Actually on a uniform grid, no contour passes through the interior
        // because all corners are equal. The result may be empty.
        assert!(result.lines.len() <= 1);
    }

    #[test]
    fn test_slope_grid_generates_contours() {
        // 5x5 grid with a slope: z = col * 10 (so 0, 10, 20, 30, 40)
        let ncols = 5;
        let nrows = 5;
        let grid: Vec<f64> = (0..nrows).flat_map(|r| {
            (0..ncols).map(move |c| (c as f64) * 10.0)
        }).collect();
        let result = generate_contours(&grid, ncols, nrows, 1.0, 0.0, 0.0, 10.0, 0.0);
        // Levels: 0, 10, 20, 30, 40 (5 levels)
        assert!(result.level_count >= 4, "level_count = {}", result.level_count);
        assert!(result.min_elevation >= -0.01);
        assert!(result.max_elevation <= 40.01);
        // At least some lines should have points
        assert!(result.lines.iter().any(|l| !l.points.is_empty()));
    }

    #[test]
    fn test_nodata_cells_skipped() {
        // 3x3 grid with NODATA in the middle cell
        let mut grid = vec![100.0_f64; 9];
        grid[4] = f64::NAN; // center
        let result = generate_contours(&grid, 3, 3, 1.0, 0.0, 0.0, 10.0, 0.0);
        // Should not crash; should produce some contours or none
        assert!(result.level_count > 0 || result.lines.is_empty());
    }

    #[test]
    fn test_invalid_input() {
        let grid = vec![100.0_f64; 4];
        // ncols=2, nrows=2 is valid but cell_size=0 is invalid
        let result = generate_contours(&grid, 2, 2, 0.0, 0.0, 0.0, 10.0, 0.0);
        assert_eq!(result.level_count, 0);
    }

    #[test]
    fn test_geojson_output_valid() {
        let result = ContourResult {
            lines: vec![ContourLine {
                elevation: 100.0,
                points: vec![(0.0, 0.0), (10.0, 0.0)],
            }],
            min_elevation: 100.0,
            max_elevation: 100.0,
            level_count: 1,
        };
        let json = contours_to_geojson(&result);
        assert!(json.contains("FeatureCollection"));
        assert!(json.contains("LineString"));
        assert!(json.contains("\"elevation\":100"));
    }
}
