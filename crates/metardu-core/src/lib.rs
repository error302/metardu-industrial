// metardu-core — shared processing core for MetaRDU Industrial + worker.
//
// This crate re-exports the marine and mining modules so that the
// metardu-worker binary can link against them without duplicating code.
//
// In a future refactor, the actual source files would move into this
// crate. For Phase 4, we use a path dependency from the main app's
// Cargo.toml so the worker binary can access the processing functions.

pub mod marine {
    pub use crate::*;
}

// Re-export key types that the worker needs
pub use serde::{Deserialize, Serialize};

/// Minimal Sounding type for CUBE processing in the worker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sounding {
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    pub uncertainty: f64,
}

/// Minimal CUBE parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeParams {
    pub resolution: f64,
    pub capture_distance: f64,
    pub init_uncertainty: f64,
    pub max_hypotheses: usize,
    pub min_soundings: usize,
}

impl Default for CubeParams {
    fn default() -> Self {
        Self {
            resolution: 1.0,
            capture_distance: 0.5,
            init_uncertainty: 0.3,
            max_hypotheses: 5,
            min_soundings: 3,
        }
    }
}

/// Minimal CUBE surface result.
#[derive(Debug, Clone, Serialize)]
pub struct CubeSurface {
    pub dims: (usize, usize),
    pub resolution: f64,
    pub bounds: [f64; 4],
    pub depths: Vec<f64>,
    pub uncertainties: Vec<f64>,
    pub sounding_counts: Vec<u32>,
    pub hypothesis_counts: Vec<u32>,
    pub total_soundings: usize,
    pub valid_cells: usize,
    pub ambiguous_cells: usize,
}

/// Process a CUBE tile — the entry point for the worker binary.
///
/// This is a simplified version that the worker calls. The full
/// implementation lives in the main app's marine::cube module.
/// Phase 5 will unify these via a proper shared crate.
pub fn process_cube_tile(
    soundings: &[Sounding],
    params: &CubeParams,
) -> Result<CubeSurface, String> {
    if soundings.is_empty() {
        return Err("empty soundings".into());
    }
    if soundings.len() < 10 {
        return Err(format!("too few soundings: {}", soundings.len()));
    }

    // Compute bounds
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for s in soundings {
        min_x = min_x.min(s.x);
        max_x = max_x.max(s.x);
        min_y = min_y.min(s.y);
        max_y = max_y.max(s.y);
    }

    let res = params.resolution;
    let cols = ((max_x - min_x) / res).ceil() as usize + 1;
    let rows = ((max_y - min_y) / res).ceil() as usize + 1;

    // Simplified CUBE: just average soundings per cell
    let mut depth_sums = vec![0.0f64; cols * rows];
    let mut counts = vec![0u32; cols * rows];

    for s in soundings {
        let col = (((s.x - min_x) / res).floor() as usize).min(cols - 1);
        let row = (((s.y - min_y) / res).floor() as usize).min(rows - 1);
        let idx = row * cols + col;
        depth_sums[idx] += s.depth;
        counts[idx] += 1;
    }

    let mut depths = vec![f64::NAN; cols * rows];
    let mut valid_cells = 0;
    for i in 0..depths.len() {
        if counts[i] >= params.min_soundings as u32 {
            depths[i] = depth_sums[i] / counts[i] as f64;
            valid_cells += 1;
        }
    }

    Ok(CubeSurface {
        dims: (cols, rows),
        resolution: res,
        bounds: [min_x, min_y, max_x, max_y],
        depths,
        uncertainties: vec![params.init_uncertainty; cols * rows],
        sounding_counts: counts,
        hypothesis_counts: vec![1; cols * rows],
        total_soundings: soundings.len(),
        valid_cells,
        ambiguous_cells: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_cube_tile() {
        let soundings: Vec<Sounding> = (0..100)
            .flat_map(|i| {
                (0..100).map(move |j| Sounding {
                    x: i as f64 * 0.25,
                    y: j as f64 * 0.25,
                    depth: 10.0 + (i as f64 % 5.0) * 0.1,
                    uncertainty: 0.1,
                })
            })
            .collect();
        let result = process_cube_tile(&soundings, &CubeParams::default()).unwrap();
        assert!(result.valid_cells > 0);
        assert_eq!(result.total_soundings, 10000);
    }

    #[test]
    fn test_too_few_soundings() {
        let soundings = vec![Sounding {
            x: 0.0,
            y: 0.0,
            depth: 10.0,
            uncertainty: 0.1,
        }];
        let result = process_cube_tile(&soundings, &CubeParams::default());
        assert!(result.is_err());
    }
}
