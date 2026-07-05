// DEM rasterization — Inverse Distance Weighting (IDW) interpolation of
// ground-classified points onto a regular grid.
//
// For each DEM cell, IDW computes the cell's elevation as a weighted
// average of all input points within `search_radius` (or all points if
// no radius is set):
//
//     z(x, y) = Σ wᵢ zᵢ / Σ wᵢ
//     wᵢ = 1 / dᵢ^p
//
// where dᵢ is the planimetric distance from the cell centre to point i,
// and p is the IDW power (default 2.0). Cells with fewer than
// `min_points` contributing points are marked as NODATA.
//
// The outer loop (per cell) is parallelised with `rayon::par_iter` over
// the cell index range — this is the most expensive stage of the EOM
// pipeline and benefits directly from multi-core machines.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Parameters controlling DEM rasterization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemParams {
    /// Grid cell edge length in metres.
    pub cell_size: f64,
    /// Optional bounds override: (min_x, min_y, max_x, max_y). When `None`
    /// the bounds are computed from the input points.
    #[serde(default)]
    pub bounds: Option<(f64, f64, f64, f64)>,
    /// IDW power (default 2.0).
    #[serde(default = "default_idw_power")]
    pub idw_power: f64,
    /// Optional search radius in metres. Points outside this radius do
    /// not contribute to a cell. `None` means all points contribute.
    #[serde(default)]
    pub search_radius: Option<f64>,
    /// Minimum number of contributing points required for a cell to be
    /// considered valid (otherwise NODATA). Default 1.
    #[serde(default = "default_min_points")]
    pub min_points: usize,
    /// NODATA sentinel value written into the grid for empty cells.
    #[serde(default = "default_nodata")]
    pub nodata_value: f64,
}

fn default_idw_power() -> f64 {
    2.0
}
fn default_min_points() -> usize {
    1
}
fn default_nodata() -> f64 {
    -9999.0
}

impl Default for DemParams {
    fn default() -> Self {
        Self {
            cell_size: 1.0,
            bounds: None,
            idw_power: default_idw_power(),
            search_radius: None,
            min_points: default_min_points(),
            nodata_value: default_nodata(),
        }
    }
}

/// A regular-grid DEM raster in row-major order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemGrid {
    /// Number of columns.
    pub ncols: usize,
    /// Number of rows.
    pub nrows: usize,
    /// Cell edge length in metres.
    pub cell_size: f64,
    /// Geographic bounds: (min_x, min_y, max_x, max_y).
    pub bounds: (f64, f64, f64, f64),
    /// Elevation values, row-major: `[row * ncols + col]`.
    pub data: Vec<f64>,
    /// NODATA sentinel value.
    pub nodata_value: f64,
    /// Number of cells with valid data.
    pub valid_cells: usize,
}

impl DemGrid {
    /// Return the elevation at column `col`, row `row`.
    pub fn get(&self, col: usize, row: usize) -> Option<f64> {
        if col >= self.ncols || row >= self.nrows {
            return None;
        }
        let v = self.data[row * self.ncols + col];
        if v == self.nodata_value {
            None
        } else {
            Some(v)
        }
    }

    /// Iterate over (col, row, value) tuples for all valid cells.
    pub fn iter_valid(&self) -> impl Iterator<Item = (usize, usize, f64)> + '_ {
        (0..self.nrows).flat_map(move |row| {
            (0..self.ncols).filter_map(move |col| self.get(col, row).map(|v| (col, row, v)))
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DemError {
    #[error("cell size must be positive (got {0})")]
    InvalidCellSize(f64),
    #[error("input points are empty")]
    Empty,
    #[error("IDW power must be positive (got {0})")]
    InvalidPower(f64),
    #[error("computed grid is degenerate (zero rows or columns)")]
    DegenerateGrid,
}

/// Rasterize a set of ground points to a DEM grid using IDW interpolation.
///
/// `points` is a flat slice of `(x, y, z)` tuples — typically the ground
/// subset produced by `csf::classify_ground`. The grid layout is
/// determined by `params.cell_size` and `params.bounds` (or computed
/// from the point bounds if `bounds` is `None`).
pub fn rasterize_ground_to_dem(
    points: &[(f64, f64, f64)],
    params: &DemParams,
) -> Result<DemGrid, DemError> {
    if points.is_empty() {
        return Err(DemError::Empty);
    }
    if params.cell_size <= 0.0 {
        return Err(DemError::InvalidCellSize(params.cell_size));
    }
    if params.idw_power <= 0.0 {
        return Err(DemError::InvalidPower(params.idw_power));
    }

    let (min_x, min_y, max_x, max_y) = params.bounds.unwrap_or_else(|| {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for &(x, y, _) in points {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        (min_x, min_y, max_x, max_y)
    });

    let cell = params.cell_size;
    // `+1` matches the csf convention so that a point sitting exactly on the
    // max boundary is in its own cell rather than spilling outside the grid.
    let ncols = (((max_x - min_x) / cell).ceil() as usize + 1).max(1);
    let nrows = (((max_y - min_y) / cell).ceil() as usize + 1).max(1);
    if ncols == 0 || nrows == 0 {
        return Err(DemError::DegenerateGrid);
    }

    let power = params.idw_power;
    let radius = params.search_radius;
    let min_points = params.min_points.max(1);
    let nodata = params.nodata_value;

    // Pre-extract (x, y, z) into separate slices for cache-friendly access
    // inside the parallel loop.
    let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
    let zs: Vec<f64> = points.iter().map(|p| p.2).collect();

    let total_cells = ncols * nrows;
    let data: Vec<f64> = (0..total_cells)
        .into_par_iter()
        .map(|idx| {
            let row = idx / ncols;
            let col = idx % ncols;
            // Cell centre in geographic coordinates. Row 0 = bottom (min_y).
            let cx = min_x + (col as f64 + 0.5) * cell;
            let cy = min_y + (row as f64 + 0.5) * cell;

            let mut weight_sum = 0.0f64;
            let mut value_sum = 0.0f64;
            let mut contributor_count = 0usize;

            for i in 0..xs.len() {
                let dx = xs[i] - cx;
                let dy = ys[i] - cy;
                let d2 = dx * dx + dy * dy;
                if d2 == 0.0 {
                    // Exact hit — use this point's elevation directly.
                    return zs[i];
                }
                let d = d2.sqrt();
                if let Some(r) = radius {
                    if d > r {
                        continue;
                    }
                }
                let w = 1.0 / d.powf(power);
                weight_sum += w;
                value_sum += w * zs[i];
                contributor_count += 1;
            }

            if contributor_count < min_points || weight_sum <= 0.0 {
                nodata
            } else {
                value_sum / weight_sum
            }
        })
        .collect();

    let valid_cells = data.iter().filter(|v| **v != nodata).count();

    Ok(DemGrid {
        ncols,
        nrows,
        cell_size: cell,
        bounds: (min_x, min_y, max_x, max_y),
        data,
        nodata_value: nodata,
        valid_cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_grid_interpolates_to_input_z() {
        // 4 points at z=100 on a 1m grid → every cell should be ≈100.
        let points = vec![
            (0.0, 0.0, 100.0),
            (10.0, 0.0, 100.0),
            (0.0, 10.0, 100.0),
            (10.0, 10.0, 100.0),
        ];
        let params = DemParams {
            cell_size: 1.0,
            ..DemParams::default()
        };
        let grid = rasterize_ground_to_dem(&points, &params).unwrap();
        assert_eq!(grid.ncols, 11);
        assert_eq!(grid.nrows, 11);
        // Every cell should be near 100 (interpolation of constant field).
        for v in &grid.data {
            assert!((v - 100.0).abs() < 1e-3, "expected ~100, got {}", v);
        }
        assert_eq!(grid.valid_cells, grid.ncols * grid.nrows);
    }

    #[test]
    fn test_search_radius_creates_nodata() {
        // Two clusters of points far apart with a search radius smaller
        // than the gap → cells in the middle should be NODATA.
        let mut points = Vec::new();
        for i in 0..5 {
            points.push((i as f64, 0.0, 50.0));
        }
        for i in 0..5 {
            points.push((100.0 + i as f64, 0.0, 150.0));
        }
        let params = DemParams {
            cell_size: 1.0,
            search_radius: Some(3.0),
            min_points: 1,
            ..DemParams::default()
        };
        let grid = rasterize_ground_to_dem(&points, &params).unwrap();
        // The grid spans 0..105 in X, so ~105 cols x 1 row.
        // Cells around x=50 should be NODATA (no points within 3m).
        let middle_col = 50usize;
        let row = 0;
        let v = grid.data[row * grid.ncols + middle_col];
        assert_eq!(v, params.nodata_value);
        // Cells at x=2 should be ≈50.
        let near_col = 2usize;
        let v = grid.data[row * grid.ncols + near_col];
        assert!((v - 50.0).abs() < 1e-3, "expected ~50, got {}", v);
    }

    #[test]
    fn test_invalid_cell_size_errors() {
        let points = vec![(0.0, 0.0, 1.0), (1.0, 1.0, 2.0)];
        let params = DemParams {
            cell_size: 0.0,
            ..DemParams::default()
        };
        assert!(matches!(
            rasterize_ground_to_dem(&points, &params),
            Err(DemError::InvalidCellSize(0.0))
        ));
    }

    #[test]
    fn test_empty_points_errors() {
        let params = DemParams::default();
        assert!(matches!(
            rasterize_ground_to_dem(&[], &params),
            Err(DemError::Empty)
        ));
    }
}
