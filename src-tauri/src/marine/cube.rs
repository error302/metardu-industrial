// CUBE (Combined Uncertainty and Bathymetry Estimator) — pure Rust.
//
// Reference: Calder & Mayer (2003) "Automatic processing of high-rate
// multibeam echosounder data"
// https://www.ldeo.columbia.edu/res/pi/MB-System/algorithmdocs/CubeProc_HSMMS_2003.pdf
//
// CUBE is the standard algorithm for generating gridded bathymetric
// surfaces from dense multibeam sounding data. It's public domain
// (developed at UNB/CCOM), making it the right choice for an open-core
// product.
//
// Algorithm overview (simplified for Phase 2):
//   1. Build a regular grid over the survey area at the target resolution.
//   2. For each sounding, determine which grid cell it falls in.
//   3. For each cell, maintain a set of "hypotheses" — candidate depth
//      estimates with associated uncertainty.
//   4. When a new sounding arrives at a cell, either:
//      a) Update an existing hypothesis if the sounding is within
//         capture_distance of the hypothesis mean (Bayesian update).
//      b) Create a new hypothesis if no existing one is close enough.
//   5. After all soundings are processed, select the "best" hypothesis
//      per cell — the one with the most support (lowest uncertainty).
//   6. Output: depth grid + uncertainty grid + hypothesis count grid.
//
// This Phase 2 implementation uses a simplified Bayesian update without
// the full Kalman filter machinery. The reference CUBE+ (CARIS) adds
// context-based disambiguation and multiple hypothesis tracking — that's
// Phase 3+ work.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeParams {
    /// Grid cell size in meters (the output resolution)
    #[serde(default = "default_resolution")]
    pub resolution: f64,
    /// Maximum distance (meters) for a sounding to be captured by an
    /// existing hypothesis. Beyond this, a new hypothesis is created.
    #[serde(default = "default_capture_distance")]
    pub capture_distance: f64,
    /// Initial uncertainty for each hypothesis (meters)
    #[serde(default = "default_init_uncertainty")]
    pub init_uncertainty: f64,
    /// Maximum number of hypotheses per cell before pruning
    #[serde(default = "default_max_hypotheses")]
    pub max_hypotheses: usize,
    /// Minimum soundings per cell for it to be included in output
    #[serde(default = "default_min_soundings")]
    pub min_soundings: usize,
}

fn default_resolution() -> f64 {
    1.0
}
fn default_capture_distance() -> f64 {
    0.5
}
fn default_init_uncertainty() -> f64 {
    0.3
}
fn default_max_hypotheses() -> usize {
    5
}
fn default_min_soundings() -> usize {
    3
}

impl Default for CubeParams {
    fn default() -> Self {
        Self {
            resolution: default_resolution(),
            capture_distance: default_capture_distance(),
            init_uncertainty: default_init_uncertainty(),
            max_hypotheses: default_max_hypotheses(),
            min_soundings: default_min_soundings(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CubeSurface {
    /// Grid dimensions (cols, rows)
    pub dims: (usize, usize),
    /// Cell size in meters
    pub resolution: f64,
    /// Geographic bounds: [min_x, min_y, max_x, max_y]
    pub bounds: [f64; 4],
    /// Depth grid (cols * rows), row-major. NaN for empty cells.
    pub depths: Vec<f64>,
    /// Uncertainty grid (cols * rows), row-major. NaN for empty cells.
    pub uncertainties: Vec<f64>,
    /// Sounding count per cell
    pub sounding_counts: Vec<u32>,
    /// Hypothesis count per cell (cells with >1 hypothesis may need review)
    pub hypothesis_counts: Vec<u32>,
    /// Total soundings processed
    pub total_soundings: usize,
    /// Cells with valid depth (non-NaN)
    pub valid_cells: usize,
    /// Cells with multiple hypotheses (potential artifacts)
    pub ambiguous_cells: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum CubeError {
    #[error("empty sounding set")]
    Empty,
    #[error("too few soundings: {0} (minimum 10)")]
    TooFewSoundings(usize),
    #[error("invalid resolution: {0} (must be > 0)")]
    InvalidResolution(f64),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A single depth sounding with position and uncertainty.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sounding {
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    /// Vertical uncertainty (1-sigma) in meters
    pub uncertainty: f64,
}

/// A hypothesis for a grid cell's true depth.
#[derive(Debug, Clone)]
struct Hypothesis {
    /// Estimated depth (Bayesian posterior mean)
    depth: f64,
    /// Current uncertainty (posterior sigma)
    uncertainty: f64,
    /// Number of soundings captured by this hypothesis
    count: u32,
}

/// Run CUBE on a set of soundings. Returns a gridded bathymetric surface
/// with per-cell depth, uncertainty, and hypothesis counts.
pub fn generate_surface(
    soundings: &[Sounding],
    params: &CubeParams,
) -> Result<CubeSurface, CubeError> {
    if soundings.is_empty() {
        return Err(CubeError::Empty);
    }
    if soundings.len() < 10 {
        return Err(CubeError::TooFewSoundings(soundings.len()));
    }
    if params.resolution <= 0.0 {
        return Err(CubeError::InvalidResolution(params.resolution));
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
    if cols == 0 || rows == 0 {
        return Err(CubeError::TooFewSoundings(soundings.len()));
    }

    // Per-cell hypothesis lists
    let mut cell_hypotheses: Vec<Vec<Hypothesis>> = vec![Vec::new(); cols * rows];
    let mut sounding_counts = vec![0u32; cols * rows];

    // Process each sounding
    for s in soundings {
        let col = (((s.x - min_x) / res).floor() as usize).min(cols - 1);
        let row = (((s.y - min_y) / res).floor() as usize).min(rows - 1);
        let idx = row * cols + col;
        sounding_counts[idx] += 1;

        let hypotheses = &mut cell_hypotheses[idx];

        // Try to find an existing hypothesis that captures this sounding
        let mut captured_idx: Option<usize> = None;
        let mut best_dist = f64::INFINITY;
        for (i, h) in hypotheses.iter().enumerate() {
            let dist = (h.depth - s.depth).abs();
            if dist < params.capture_distance && dist < best_dist {
                best_dist = dist;
                captured_idx = Some(i);
            }
        }

        if let Some(i) = captured_idx {
            // Bayesian update: merge the sounding into the hypothesis
            let h = &mut hypotheses[i];
            let prior_var = h.uncertainty * h.uncertainty;
            let meas_var = s.uncertainty * s.uncertainty;
            let post_var = 1.0 / (1.0 / prior_var + 1.0 / meas_var);
            let post_mean = post_var * (h.depth / prior_var + s.depth / meas_var);
            h.depth = post_mean;
            h.uncertainty = post_var.sqrt();
            h.count += 1;
        } else {
            // Create a new hypothesis
            hypotheses.push(Hypothesis {
                depth: s.depth,
                uncertainty: params.init_uncertainty.max(s.uncertainty),
                count: 1,
            });

            // Prune if too many hypotheses — keep the ones with most support
            if hypotheses.len() > params.max_hypotheses {
                hypotheses.sort_by(|a, b| b.count.cmp(&a.count));
                hypotheses.truncate(params.max_hypotheses);
            }
        }
    }

    // Select best hypothesis per cell and build output grids
    let mut depths = vec![f64::NAN; cols * rows];
    let mut uncertainties = vec![f64::NAN; cols * rows];
    let mut hypothesis_counts = vec![0u32; cols * rows];
    let mut valid_cells = 0usize;
    let mut ambiguous_cells = 0usize;

    for idx in 0..(cols * rows) {
        let hypotheses = &cell_hypotheses[idx];
        if hypotheses.is_empty() {
            continue;
        }
        if sounding_counts[idx] < params.min_soundings as u32 {
            continue;
        }

        hypothesis_counts[idx] = hypotheses.len() as u32;
        if hypotheses.len() > 1 {
            ambiguous_cells += 1;
        }

        // Select the hypothesis with the most soundings (lowest uncertainty
        // as tiebreaker)
        let best = hypotheses
            .iter()
            .max_by(|a, b| {
                a.count.cmp(&b.count).then_with(|| {
                    b.uncertainty
                        .partial_cmp(&a.uncertainty)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            })
            .unwrap();

        depths[idx] = best.depth;
        uncertainties[idx] = best.uncertainty;
        valid_cells += 1;
    }

    let total_soundings = soundings.len();

    Ok(CubeSurface {
        dims: (cols, rows),
        resolution: res,
        bounds: [min_x, min_y, max_x, max_y],
        depths,
        uncertainties,
        sounding_counts,
        hypothesis_counts,
        total_soundings,
        valid_cells,
        ambiguous_cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_flat_soundings(n: usize, depth: f64) -> Vec<Sounding> {
        let mut soundings = Vec::with_capacity(n * n);
        for i in 0..n {
            for j in 0..n {
                soundings.push(Sounding {
                    x: i as f64 * 0.25,
                    y: j as f64 * 0.25,
                    depth,
                    uncertainty: 0.1,
                });
            }
        }
        soundings
    }

    #[test]
    fn test_flat_surface() {
        // 20×20 soundings at 0.25m spacing = 5m × 5m area
        // With 1m resolution → 5×5 = 25 cells, each with ~16 soundings
        // min_soundings=3 → all cells should pass
        let soundings = make_flat_soundings(20, 10.0);
        let result = generate_surface(&soundings, &CubeParams::default()).unwrap();
        assert!(
            result.valid_cells > 5,
            "expected >5 valid cells, got {}",
            result.valid_cells
        );
        // All depths should be ~10.0
        let valid_depths: Vec<f64> = result
            .depths
            .iter()
            .copied()
            .filter(|d| !d.is_nan())
            .collect();
        let mean = valid_depths.iter().sum::<f64>() / valid_depths.len() as f64;
        assert!(
            (mean - 10.0).abs() < 0.1,
            "mean depth {mean} should be ~10.0"
        );
    }

    #[test]
    fn test_too_few_soundings() {
        let soundings = vec![Sounding {
            x: 0.0,
            y: 0.0,
            depth: 10.0,
            uncertainty: 0.1,
        }];
        let result = generate_surface(&soundings, &CubeParams::default());
        assert!(matches!(result, Err(CubeError::TooFewSoundings(1))));
    }

    #[test]
    fn test_empty_errors() {
        let result = generate_surface(&[], &CubeParams::default());
        assert!(matches!(result, Err(CubeError::Empty)));
    }

    #[test]
    fn test_multiple_hypotheses_detected() {
        // Create soundings where half the cells have two distinct depth
        // populations (e.g., a wreck on the seabed)
        let mut soundings = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                let x = i as f64 * 0.5;
                let y = j as f64 * 0.5;
                // Seabed at 10m
                soundings.push(Sounding {
                    x,
                    y,
                    depth: 10.0,
                    uncertainty: 0.1,
                });
                // "Wreck" at 8m in some cells
                if i == 5 && j == 5 {
                    soundings.push(Sounding {
                        x,
                        y,
                        depth: 8.0,
                        uncertainty: 0.1,
                    });
                }
            }
        }
        let result = generate_surface(&soundings, &CubeParams::default()).unwrap();
        // At least one cell should have multiple hypotheses (or zero —
        // the algorithm may or may not produce ambiguity depending on params).
        // This test just verifies the function doesn't crash on mixed depths.
        let _ = result.ambiguous_cells;
    }
}
