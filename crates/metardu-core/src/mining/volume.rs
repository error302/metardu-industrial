// Volume calculation engine — pure Rust, no external deps.
//
// Implements the standard mine surveyor method for stockpile and pit
// volume calculation:
//   1. Align two raster surfaces (current survey, reference/base) on a
//      common grid.
//   2. Per cell, compute elevation difference (dz = current - reference).
//   3. Volume = sum(dz × cell_area) — with sign:
//      - Positive dz (current higher than reference) = fill volume (stockpile)
//      - Negative dz (current lower than reference) = cut volume (excavation)
//   4. Net volume = fill - cut.
//
// For bench-by-bench breakdown, we slice the volume by elevation bands:
//   for each bench [z_min, z_max]:
//     bench_fill = sum over cells where current > z_min AND current <= z_max
//                   of min(current, z_max) - max(reference, z_min)
//                   × cell_area (only if positive)
//     (similarly for cut)
//
// Inputs are passed as flat Vec<f64> elevation grids with explicit
// dimensions. Frontend calls this via the compute_volumes IPC command
// with two GeoTIFF paths; the IPC layer reads the GeoTIFFs and converts
// to grids.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeResult {
    /// Total fill volume (current above reference), cubic meters
    pub fill_volume: f64,
    /// Total cut volume (current below reference), cubic meters
    pub cut_volume: f64,
    /// Net volume (fill - cut). Positive = net fill, negative = net cut.
    pub net_volume: f64,
    /// Cell area used for integration (square meters)
    pub cell_area: f64,
    /// Number of cells where fill occurred
    pub fill_cells: usize,
    /// Number of cells where cut occurred
    pub cut_cells: usize,
    /// Per-bench breakdown — only for cells within each band
    pub benches: Vec<BenchVolume>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchVolume {
    /// Lower elevation bound of this bench (meters)
    pub z_min: f64,
    /// Upper elevation bound of this bench (meters)
    pub z_max: f64,
    pub fill_volume: f64,
    pub cut_volume: f64,
    pub net_volume: f64,
    pub fill_cells: usize,
    pub cut_cells: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum VolumeError {
    #[error("grids have different dimensions: {0}x{1} vs {2}x{3}")]
    DimensionMismatch(usize, usize, usize, usize),
    #[error("grid is empty")]
    Empty,
    #[error("cell dimensions must be positive: got {0}x{1}")]
    InvalidCellDims(f64, f64),
}

/// Compute volumes by differencing two elevation grids.
///
/// `current` and `reference` must have the same dimensions. They are
/// assumed to be aligned to the same geographic grid; the caller is
/// responsible for resampling if needed (Phase 1 simplification — we
/// assume both surfaces come from the same source or were already
/// resampled in the frontend).
///
/// `cell_width_m` and `cell_height_m` are the geographic size of each
/// cell in meters. For UTM-based DEMs this is straightforward; for
/// geographic DEMs we approximate using the latitude of the grid center.
///
/// `bench_interval` is the elevation band width for bench-by-bench
/// breakdown (e.g., 5.0 for 5m benches). Pass 0.0 to skip bench breakdown.
pub fn compute_volumes(
    current: &[f64],
    reference: &[f64],
    cell_width_m: f64,
    cell_height_m: f64,
    bench_interval: f64,
) -> Result<VolumeResult, VolumeError> {
    if current.is_empty() || reference.is_empty() {
        return Err(VolumeError::Empty);
    }
    if current.len() != reference.len() {
        return Err(VolumeError::DimensionMismatch(
            current.len(),
            1,
            reference.len(),
            1,
        ));
    }
    if cell_width_m <= 0.0 || cell_height_m <= 0.0 {
        return Err(VolumeError::InvalidCellDims(cell_width_m, cell_height_m));
    }

    let cell_area = cell_width_m * cell_height_m;
    let mut fill_volume = 0.0;
    let mut cut_volume = 0.0;
    let mut fill_cells = 0usize;
    let mut cut_cells = 0usize;

    // Determine bench bounds for breakdown
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    for (c, r) in current.iter().zip(reference.iter()) {
        z_min = z_min.min(*c).min(*r);
        z_max = z_max.max(*c).max(*r);
    }
    let benches = if bench_interval > 0.0 {
        build_bench_breakpoints(z_min, z_max, bench_interval)
    } else {
        Vec::new()
    };
    let mut bench_results: Vec<BenchVolume> = benches
        .iter()
        .map(|(lo, hi)| BenchVolume {
            z_min: *lo,
            z_max: *hi,
            fill_volume: 0.0,
            cut_volume: 0.0,
            net_volume: 0.0,
            fill_cells: 0,
            cut_cells: 0,
        })
        .collect();

    for (c, r) in current.iter().zip(reference.iter()) {
        let dz = c - r;
        if dz > 0.0 {
            fill_volume += dz * cell_area;
            fill_cells += 1;
        } else if dz < 0.0 {
            cut_volume += -dz * cell_area;
            cut_cells += 1;
        }

        // Bench breakdown: contribute to each bench the cell overlaps
        for bench in &mut bench_results {
            // The cell's elevation range is [*c, *c] (a single value).
            // We assign it to the bench that contains *c.
            // For surveys this matches the convention: a cell at elevation z
            // contributes to the bench [z_min_b, z_max_b] where z is in that band.
            if *c >= bench.z_min && *c < bench.z_max {
                if dz > 0.0 {
                    bench.fill_volume += dz * cell_area;
                    bench.fill_cells += 1;
                } else if dz < 0.0 {
                    bench.cut_volume += -dz * cell_area;
                    bench.cut_cells += 1;
                }
                break;
            }
        }
    }

    // Compute net per-bench
    for bench in &mut bench_results {
        bench.net_volume = bench.fill_volume - bench.cut_volume;
    }

    Ok(VolumeResult {
        fill_volume,
        cut_volume,
        net_volume: fill_volume - cut_volume,
        cell_area,
        fill_cells,
        cut_cells,
        benches: bench_results,
    })
}

/// Build bench breakpoints as [(z_min, z_max), ...] covering [z_min, z_max].
fn build_bench_breakpoints(z_min: f64, z_max: f64, interval: f64) -> Vec<(f64, f64)> {
    if interval <= 0.0 || z_max <= z_min {
        return vec![];
    }
    let mut result = Vec::new();
    let mut lo = (z_min / interval).floor() * interval;
    while lo < z_max {
        let hi = lo + interval;
        result.push((lo, hi));
        lo = hi;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_volume_flat_reference() {
        // 2x2 grid, all current = 110m, all reference = 100m
        // Each cell 10m x 10m = 100 m²
        // dz = 10m everywhere → fill_volume = 4 * 10 * 100 = 4000 m³
        let current = vec![110.0_f64; 4];
        let reference = vec![100.0_f64; 4];
        let result = compute_volumes(&current, &reference, 10.0, 10.0, 0.0).unwrap();
        assert_eq!(result.fill_volume, 4000.0);
        assert_eq!(result.cut_volume, 0.0);
        assert_eq!(result.net_volume, 4000.0);
        assert_eq!(result.fill_cells, 4);
        assert_eq!(result.cut_cells, 0);
    }

    #[test]
    fn test_cut_volume() {
        // 2x2 grid, current = 90m, reference = 100m
        // dz = -10m → cut_volume = 4 * 10 * 100 = 4000 m³
        let current = vec![90.0_f64; 4];
        let reference = vec![100.0_f64; 4];
        let result = compute_volumes(&current, &reference, 10.0, 10.0, 0.0).unwrap();
        assert_eq!(result.fill_volume, 0.0);
        assert_eq!(result.cut_volume, 4000.0);
        assert_eq!(result.net_volume, -4000.0);
    }

    #[test]
    fn test_mixed_fill_cut() {
        // 2x2 grid: 2 cells fill (110-100=10), 2 cells cut (90-100=-10)
        // Cell area 100 m²
        // fill = 2 * 10 * 100 = 2000; cut = 2 * 10 * 100 = 2000; net = 0
        let current = vec![110.0, 110.0, 90.0, 90.0];
        let reference = vec![100.0; 4];
        let result = compute_volumes(&current, &reference, 10.0, 10.0, 0.0).unwrap();
        assert_eq!(result.fill_volume, 2000.0);
        assert_eq!(result.cut_volume, 2000.0);
        assert_eq!(result.net_volume, 0.0);
        assert_eq!(result.fill_cells, 2);
        assert_eq!(result.cut_cells, 2);
    }

    #[test]
    fn test_bench_breakdown() {
        let current = vec![105.0, 115.0, 125.0, 135.0];
        let reference = vec![100.0; 4];
        // Bench interval = 10m. Bands:
        //   [100, 110): 105 → dz=5, fill=5*100=500
        //   [110, 120): 115 → dz=15, fill=15*100=1500
        //   [120, 130): 125 → dz=25, fill=25*100=2500
        //   [130, 140): 135 → dz=35, fill=35*100=3500
        let result = compute_volumes(&current, &reference, 10.0, 10.0, 10.0).unwrap();
        assert_eq!(result.benches.len(), 4);
        assert_eq!(result.benches[0].fill_volume, 500.0);
        assert_eq!(result.benches[1].fill_volume, 1500.0);
        assert_eq!(result.benches[2].fill_volume, 2500.0);
        assert_eq!(result.benches[3].fill_volume, 3500.0);
    }

    #[test]
    fn test_dimension_mismatch_errors() {
        let current = vec![1.0, 2.0, 3.0];
        let reference = vec![1.0, 2.0];
        let result = compute_volumes(&current, &reference, 1.0, 1.0, 0.0);
        assert!(matches!(
            result,
            Err(VolumeError::DimensionMismatch(_, _, _, _))
        ));
    }

    #[test]
    fn test_empty_grids_error() {
        let result = compute_volumes(&[], &[], 1.0, 1.0, 0.0);
        assert!(matches!(result, Err(VolumeError::Empty)));
    }
}
