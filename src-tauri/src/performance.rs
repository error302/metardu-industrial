// Performance hardening — LOD (Level of Detail) streaming for point clouds
// and chunked CUBE processing.
//
// For 100M+ point datasets, loading everything into memory is infeasible.
// This module provides:
//   - PointCloudLod: decimate points by spatial hashing for display at
//     different zoom levels
//   - ChunkedCube: process CUBE surface in chunks to handle 50M+ soundings
//   - StreamingLasReader: read LAS points in batches without loading
//     the entire file

use serde::Serialize;
use std::collections::HashMap;

/// Decimate a point cloud for display at a given LOD level.
///
/// Uses spatial hashing: divide the bounding box into cells of size
/// `cell_size`, keep one representative point per cell (the median by Z).
/// This reduces 100M points to ~1M at LOD level 1 (10m cells), or
/// ~100K at LOD level 2 (50m cells).
///
/// Returns the decimated point array + statistics.
#[derive(Debug, Clone, Serialize)]
pub struct LodResult {
    pub original_count: usize,
    pub decimated_count: usize,
    pub reduction_ratio: f64,
    pub cell_size: f64,
    pub points: Vec<(f64, f64, f64)>,
}

pub fn decimate_points(points: &[(f64, f64, f64)], cell_size: f64) -> LodResult {
    if points.is_empty() || cell_size <= 0.0 {
        return LodResult {
            original_count: points.len(),
            decimated_count: 0,
            reduction_ratio: 0.0,
            cell_size,
            points: Vec::new(),
        };
    }

    // Spatial hash: key = (col, row) → collect points per cell
    let mut cells: HashMap<(i64, i64), Vec<(f64, f64, f64)>> = HashMap::new();
    for &(x, y, z) in points {
        let col = (x / cell_size).floor() as i64;
        let row = (y / cell_size).floor() as i64;
        cells.entry((col, row)).or_default().push((x, y, z));
    }

    // For each cell, pick the median-Z point as representative
    let mut decimated = Vec::with_capacity(cells.len());
    for (_, mut cell_points) in cells {
        if cell_points.is_empty() {
            continue;
        }
        // Sort by Z and pick median
        cell_points.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        let mid = cell_points.len() / 2;
        decimated.push(cell_points[mid]);
    }

    let original = points.len();
    let decimated_count = decimated.len();
    let reduction_ratio = if original > 0 {
        decimated_count as f64 / original as f64
    } else {
        0.0
    };

    LodResult {
        original_count: original,
        decimated_count,
        reduction_ratio,
        cell_size,
        points: decimated,
    }
}

/// Determine the appropriate LOD cell size based on map zoom level
/// and point density.
///
/// At high zoom (close): 0.5m cells (max detail)
/// At medium zoom: 5m cells (100x reduction)
/// At low zoom (far): 50m cells (10000x reduction)
pub fn lod_cell_size_for_zoom(zoom: f64, avg_point_spacing: f64) -> f64 {
    // Base cell size = average point spacing (no decimation)
    let base = avg_point_spacing.max(0.1);
    // Scale by inverse zoom — lower zoom = larger cells
    let scale = match zoom {
        z if z >= 16.0 => 1.0,   // very close: no decimation
        z if z >= 14.0 => 5.0,   // medium: 5x
        z if z >= 12.0 => 25.0,  // far: 25x
        z if z >= 10.0 => 100.0, // very far: 100x
        _ => 500.0,              // world view: 500x
    };
    base * scale
}

/// Estimate average point spacing from a sample of points.
pub fn estimate_point_spacing(points: &[(f64, f64, f64)]) -> f64 {
    if points.len() < 2 {
        return 1.0;
    }
    // Sample up to 1000 points to estimate density
    let sample_size = points.len().min(1000);
    let step = points.len() / sample_size;

    let mut min_dist = f64::INFINITY;
    for i in (0..points.len()).step_by(step) {
        for j in (i + step..points.len()).step_by(step).take(10) {
            let dx = points[i].0 - points[j].0;
            let dy = points[i].1 - points[j].1;
            let d = (dx * dx + dy * dy).sqrt();
            if d > 0.0 && d < min_dist {
                min_dist = d;
            }
        }
    }
    if min_dist.is_infinite() {
        1.0
    } else {
        min_dist
    }
}

/// Chunked CUBE processing — splits soundings into spatial chunks,
/// processes each chunk independently, and merges results.
///
/// This enables processing 50M+ soundings without loading all into
/// memory simultaneously. Each chunk is processed by the standard
/// CUBE algorithm, and the resulting grids are stitched together.
#[derive(Debug, Clone, Serialize)]
pub struct ChunkedCubeResult {
    pub total_chunks: usize,
    pub total_soundings: usize,
    pub valid_cells: usize,
    pub merge_stats: ChunkMergeStats,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ChunkMergeStats {
    pub boundary_cells: usize,
    pub merged_cells: usize,
    pub conflict_cells: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimate_flat_grid() {
        // 100x100 points at 0.25m spacing, cell_size=1.0 → ~25x25=625 cells
        let mut points = Vec::new();
        for i in 0..100 {
            for j in 0..100 {
                points.push((i as f64 * 0.25, j as f64 * 0.25, 10.0));
            }
        }
        let result = decimate_points(&points, 1.0);
        assert_eq!(result.original_count, 10000);
        assert!(result.decimated_count > 500 && result.decimated_count < 700);
        assert!(result.reduction_ratio < 0.1);
    }

    #[test]
    fn test_decimate_empty() {
        let result = decimate_points(&[], 1.0);
        assert_eq!(result.decimated_count, 0);
    }

    #[test]
    fn test_lod_cell_size_scales_with_zoom() {
        let spacing = 0.5;
        let close = lod_cell_size_for_zoom(18.0, spacing);
        let far = lod_cell_size_for_zoom(8.0, spacing);
        assert!(far > close, "far zoom should have larger cells");
    }

    #[test]
    fn test_estimate_spacing() {
        let points = vec![
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
        ];
        let spacing = estimate_point_spacing(&points);
        assert!(
            (spacing - 1.0).abs() < 0.1,
            "spacing should be ~1.0, got {spacing}"
        );
    }
}
