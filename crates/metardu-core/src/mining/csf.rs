// CSF (Cloth Simulation Filter) — pure-Rust ground extraction.
//
// Reference: Zhang et al. (2016) "An Easy-to-Use Airborne LiDAR Data
// Filtering Method Based on Cloth Simulation"
// https://doi.org/10.3390/rs8060501
//
// Algorithm overview:
//   1. Invert the point cloud (negate Z) — the "terrain" becomes a
//      bowl-shaped surface pointing up.
//   2. Generate a regular grid of "cloth particles" above the highest
//      inverted point. Each particle has position, velocity, and is
//      connected to its 8 neighbors by springs.
//   3. Simulate gravity pulling particles down. When a particle hits
//      the inverted terrain, it becomes "unmovable" and stays at the
//      terrain height.
//   4. Iterate: for each movable particle, apply gravity, spring force
//      from neighbors, integrate position (Verlet). Mark collisions.
//   5. After convergence (max iterations or position delta < threshold),
//      classify original points: those within `classification_threshold`
//      of the cloth surface are ground; others are non-ground.
//
// Parameters:
//   - cloth_resolution: grid spacing in meters (smaller = finer but slower)
//   - classification_threshold: max distance from cloth for ground (m)
//   - max_iterations: cap on simulation steps (default 500)
//   - rigidness: 1=gentle terrain, 2=sloped, 3=cliff
//   - time_step: simulation dt (default 0.65)
//
// This is a simplified implementation. The reference paper has additional
// features (post-processing of steep slopes, handling of buildings) that
// are Phase 2 work.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsfParams {
    /// Grid spacing for cloth particles, in meters
    #[serde(default = "default_cloth_resolution")]
    pub cloth_resolution: f64,
    /// Max distance from cloth for a point to be classified as ground, in meters
    #[serde(default = "default_classification_threshold")]
    pub classification_threshold: f64,
    /// Max simulation iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Terrain rigidness: 1=gentle, 2=sloped, 3=cliff
    #[serde(default = "default_rigidness")]
    pub rigidness: u32,
    /// Simulation time step
    #[serde(default = "default_time_step")]
    pub time_step: f64,
    /// Initial cloth height offset above max terrain (meters)
    #[serde(default = "default_cloth_init_offset")]
    pub cloth_init_offset: f64,
}

fn default_cloth_resolution() -> f64 {
    0.5
}
fn default_classification_threshold() -> f64 {
    0.5
}
fn default_max_iterations() -> u32 {
    500
}
fn default_rigidness() -> u32 {
    2
}
fn default_time_step() -> f64 {
    0.65
}
fn default_cloth_init_offset() -> f64 {
    10.0
}

impl Default for CsfParams {
    fn default() -> Self {
        Self {
            cloth_resolution: default_cloth_resolution(),
            classification_threshold: default_classification_threshold(),
            max_iterations: default_max_iterations(),
            rigidness: default_rigidness(),
            time_step: default_time_step(),
            cloth_init_offset: default_cloth_init_offset(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CsfResult {
    /// Total points processed
    pub point_count: usize,
    /// Points classified as ground
    pub ground_count: usize,
    /// Points classified as non-ground
    pub non_ground_count: usize,
    /// Per-point classification: true = ground, false = non-ground
    pub is_ground: Vec<bool>,
    /// Elapsed simulation iterations
    pub iterations_run: u32,
    /// Cloth grid dimensions (cols, rows)
    pub cloth_dims: (usize, usize),
    /// Final cloth height range
    pub cloth_z_min: f64,
    pub cloth_z_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum CsfError {
    #[error("empty point cloud")]
    Empty,
    #[error("point cloud has fewer than 10 points (got {0})")]
    TooFewPoints(usize),
    #[error("cloth resolution must be positive (got {0})")]
    InvalidResolution(f64),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Run CSF on a flat array of (x, y, z) points. Returns per-point
/// ground classification.
pub fn classify_ground(
    points: &[(f64, f64, f64)],
    params: &CsfParams,
) -> Result<CsfResult, CsfError> {
    if points.is_empty() {
        return Err(CsfError::Empty);
    }
    if points.len() < 10 {
        return Err(CsfError::TooFewPoints(points.len()));
    }
    if params.cloth_resolution <= 0.0 {
        return Err(CsfError::InvalidResolution(params.cloth_resolution));
    }

    // Compute bounds
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(x, y, z) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }

    // Build cloth grid
    let res = params.cloth_resolution;
    let mut cols = ((max_x - min_x) / res).ceil() as usize + 1;
    let mut rows = ((max_y - min_y) / res).ceil() as usize + 1;
    if cols == 0 || rows == 0 {
        return Err(CsfError::TooFewPoints(points.len()));
    }

    // Safety: cap the cloth grid size to prevent OOM/hang on large
    // extents. A 0.5m cloth resolution on a 3.3km × 4.6km survey
    // (common for airborne lidar) would create 6600 × 9200 = 60.7M
    // cloth particles × 500 iterations = 30 billion operations —
    // the app would appear frozen for minutes.
    //
    // The cap is 500×500 = 250K particles (reasonable simulation
    // time of <1s). If the requested grid exceeds this, we
    // automatically increase the resolution so the grid fits.
    // This trades accuracy for responsiveness on large extents —
    // the surveyor can always crop the point cloud to a smaller
    // area for finer-grained classification.
    const MAX_CLOTH_DIM: usize = 500;
    let max_dim = cols.max(rows);
    if max_dim > MAX_CLOTH_DIM {
        let scale = max_dim as f64 / MAX_CLOTH_DIM as f64;
        let new_res = res * scale;
        cols = ((max_x - min_x) / new_res).ceil() as usize + 1;
        rows = ((max_y - min_y) / new_res).ceil() as usize + 1;
        // Clamp again to be safe
        cols = cols.min(MAX_CLOTH_DIM);
        rows = rows.min(MAX_CLOTH_DIM);
    }

    // Inverted terrain: max_z - z, so high points become low and vice versa.
    // Cloth starts at (max_z + offset) inverted = (min_z - offset) in inverted coords.
    // For simplicity we work in inverted z = -z so terrain points down.
    let inverted_max_z = -min_z; // highest inverted z (lowest original z = ground)
    let cloth_init_z = inverted_max_z + params.cloth_init_offset;

    // Build height grid: for each cloth cell, store the highest inverted
    // terrain z (lowest original z) — i.e., the "ceiling" the cloth falls onto.
    let mut terrain_height = vec![f64::NEG_INFINITY; cols * rows];
    for &(x, y, z) in points {
        let col = (((x - min_x) / res).floor() as usize).min(cols - 1);
        let row = (((y - min_y) / res).floor() as usize).min(rows - 1);
        let idx = row * cols + col;
        let inv_z = -z;
        if inv_z > terrain_height[idx] {
            terrain_height[idx] = inv_z;
        }
    }

    // Initialize cloth particles
    let mut cloth_z = vec![cloth_init_z; cols * rows];
    let mut cloth_z_prev = vec![cloth_init_z; cols * rows];
    let mut movable = vec![true; cols * rows];

    // Rigidness controls the spring constant multiplier
    let spring_k = match params.rigidness {
        1 => 1.0,
        2 => 2.0,
        3 => 4.0,
        _ => 2.0,
    };

    let dt = params.time_step;
    let gravity = -0.5 * dt * dt; // per-step gravity in inverted z (pulls down)

    // Iterate
    let mut iterations_run = 0u32;
    for iter in 0..params.max_iterations {
        iterations_run = iter + 1;
        let mut max_delta = 0.0f64;

        // For each cloth particle, apply gravity + spring forces from neighbors
        let mut new_z = cloth_z.clone();
        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                if !movable[idx] {
                    continue;
                }

                // Gravity
                let mut force = gravity;

                // Spring forces from neighbors (8-connected)
                let mut neighbor_sum = 0.0;
                let mut neighbor_count = 0;
                for dr in 0..3i32 {
                    for dc in 0..3i32 {
                        if dr == 1 && dc == 1 {
                            continue;
                        }
                        let nr = row as i32 + dr - 1;
                        let nc = col as i32 + dc - 1;
                        if nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32 {
                            continue;
                        }
                        let n_idx = nr as usize * cols + nc as usize;
                        neighbor_sum += cloth_z[n_idx] - cloth_z[idx];
                        neighbor_count += 1;
                    }
                }
                if neighbor_count > 0 {
                    force += spring_k * (neighbor_sum / neighbor_count as f64) * dt * dt;
                }

                // Verlet integration: new = pos + (pos - prev) + force
                let vel = cloth_z[idx] - cloth_z_prev[idx];
                let candidate = cloth_z[idx] + vel * 0.99 + force;

                // Collision with terrain: if candidate z (inverted) goes below
                // terrain height, snap to terrain and mark unmovable
                let t_h = terrain_height[idx];
                if candidate <= t_h {
                    new_z[idx] = t_h;
                    movable[idx] = false;
                } else {
                    new_z[idx] = candidate;
                }

                let delta = (new_z[idx] - cloth_z[idx]).abs();
                if delta > max_delta {
                    max_delta = delta;
                }
            }
        }

        cloth_z_prev = cloth_z.clone();
        cloth_z = new_z;

        // Convergence check — if max position delta is tiny, stop
        if max_delta < 1e-6 && iter > 50 {
            break;
        }
    }

    // Classify points: a point is ground if its distance from the cloth
    // surface (interpolated to the point's xy) is within threshold.
    let threshold = params.classification_threshold;
    let mut is_ground = Vec::with_capacity(points.len());
    let mut ground_count = 0usize;

    for &(x, y, z) in points {
        let col = ((x - min_x) / res).floor() as i64;
        let row = ((y - min_y) / res).floor() as i64;
        // Bilinear interpolate cloth height at (col_frac, row_frac)
        let col_f = (x - min_x) / res - col as f64;
        let row_f = (y - min_y) / res - row as f64;
        let c0 = col.clamp(0, (cols - 1) as i64) as usize;
        let c1 = ((col + 1).clamp(0, (cols - 1) as i64)) as usize;
        let r0 = row.clamp(0, (rows - 1) as i64) as usize;
        let r1 = ((row + 1).clamp(0, (rows - 1) as i64)) as usize;

        let z00 = cloth_z[r0 * cols + c0];
        let z10 = cloth_z[r0 * cols + c1];
        let z01 = cloth_z[r1 * cols + c0];
        let z11 = cloth_z[r1 * cols + c1];
        let z0 = z00 * (1.0 - col_f) + z10 * col_f;
        let z1 = z01 * (1.0 - col_f) + z11 * col_f;
        let cloth_height_inverted = z0 * (1.0 - row_f) + z1 * row_f;
        // Convert back to original z (un-invert)
        let cloth_height = -cloth_height_inverted;

        let dist = (z - cloth_height).abs();
        let ground = dist <= threshold;
        if ground {
            ground_count += 1;
        }
        is_ground.push(ground);
    }

    // Compute cloth z range (un-inverted)
    let cloth_z_min = cloth_z.iter().copied().fold(f64::INFINITY, f64::min);
    let cloth_z_max = cloth_z.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    Ok(CsfResult {
        point_count: points.len(),
        ground_count,
        non_ground_count: points.len() - ground_count,
        is_ground,
        iterations_run,
        cloth_dims: (cols, rows),
        cloth_z_min: -cloth_z_max, // un-invert
        cloth_z_max: -cloth_z_min,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_terrain_all_ground() {
        // 100 points on a flat plane at z=100
        let mut points = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                points.push((i as f64, j as f64, 100.0));
            }
        }
        let result = classify_ground(&points, &CsfParams::default()).unwrap();
        assert_eq!(result.point_count, 100);
        // Flat terrain should classify most points as ground
        assert!(
            result.ground_count > 90,
            "expected >90 ground, got {}",
            result.ground_count
        );
    }

    #[test]
    fn test_buildings_above_terrain() {
        // Dense grid (0.25m spacing) with a building column at z=130 (30m tall).
        // Using cloth_resolution=0.5 means multiple ground points share each
        // cloth cell, so the terrain_height per cell is the ground (not the
        // building). The building point ends up in a cell where terrain_height
        // = ground level, and the cloth falls to ground, classifying the
        // building as non-ground.
        let mut points = Vec::new();
        for i in 0..20 {
            for j in 0..20 {
                let x = i as f64 * 0.25;
                let y = j as f64 * 0.25;
                // Building is a 2x2 column of points at z=130
                let z = if (i == 10 || i == 11) && (j == 10 || j == 11) {
                    130.0
                } else {
                    100.0
                };
                points.push((x, y, z));
            }
        }
        let params = CsfParams {
            cloth_resolution: 0.5,
            cloth_init_offset: 50.0,
            max_iterations: 1000,
            classification_threshold: 0.5,
            ..CsfParams::default()
        };
        let result = classify_ground(&points, &params).unwrap();
        // The 4 building points (z=130, 30m above ground) should be non-ground
        assert!(
            result.non_ground_count >= 1,
            "expected >=1 non-ground, got {}",
            result.non_ground_count
        );
        assert!(
            result.ground_count >= 390,
            "expected >=390 ground, got {}",
            result.ground_count
        );
    }

    #[test]
    fn test_too_few_points_errors() {
        let points = vec![(0.0, 0.0, 0.0); 5];
        let result = classify_ground(&points, &CsfParams::default());
        assert!(matches!(result, Err(CsfError::TooFewPoints(5))));
    }

    #[test]
    fn test_empty_errors() {
        let result = classify_ground(&[], &CsfParams::default());
        assert!(matches!(result, Err(CsfError::Empty)));
    }
}
