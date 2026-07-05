// DEM Rendering — read GeoTIFF grid + compute hillshade + color ramp.
//
// Returns a packed RGBA buffer that the frontend renders as an OpenLayers
// ImageLayer overlay. The Rust side does all the heavy lifting (hillshade
// computation, color ramp mapping) so the frontend just blits pixels.
//
// Hillshade algorithm: standard 3×3 window slope/aspect computation.
// Color ramp: terrain (blue → green → yellow → brown → white).

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct DemRenderRequest {
    pub path: String,
    /// Hillshade azimuth in degrees (0-360, default 315 = NW)
    #[serde(default = "default_azimuth")]
    pub azimuth: f64,
    /// Hillshade altitude in degrees (0-90, default 45)
    #[serde(default = "default_altitude")]
    pub altitude: f64,
    /// Color ramp: "terrain" | "bathy" | "grayscale"
    #[serde(default = "default_ramp")]
    pub color_ramp: String,
    /// Z-scale multiplier (for exaggerated terrain)
    #[serde(default = "default_z_scale")]
    pub z_scale: f64,
}

fn default_azimuth() -> f64 {
    315.0
}
fn default_altitude() -> f64 {
    45.0
}
fn default_ramp() -> String {
    "terrain".into()
}
fn default_z_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize)]
pub struct DemRenderResult {
    /// Width of the rendered image
    pub width: u32,
    /// Height of the rendered image
    pub height: u32,
    /// Geographic bounds: [min_x, min_y, max_x, max_y]
    pub bounds: [f64; 4],
    /// Packed RGBA bytes (4 × width × height)
    pub rgba: Vec<u8>,
    /// Min elevation found
    pub min_z: f64,
    /// Max elevation found
    pub max_z: f64,
    /// EPSG code if detected
    pub epsg: Option<u16>,
}

/// Read a GeoTIFF DEM and render it as a hillshaded color-ramp RGBA image.
///
/// The frontend creates an OpenLayers ImageLayer with this image as the
/// source, positioned at the geographic bounds.
pub fn render_dem(request: &DemRenderRequest) -> Result<DemRenderResult, String> {
    use crate::commands::mining::{derive_cell_meters, read_dem_grid};
    use crate::formats::read_geotiff_header;

    let path = Path::new(&request.path);
    let header =
        read_geotiff_header(path).map_err(|e| ctx!("reading DEM header", request.path, e))?;
    let grid =
        read_dem_grid(path, &header).map_err(|e| ctx!("reading DEM grid", request.path, e))?;

    let width = header.width;
    let height = header.length;
    let n = (width as usize) * (height as usize);

    if grid.len() < n {
        return Err(format!(
            "grid too small: expected {} cells, got {}",
            n,
            grid.len()
        ));
    }

    // Find min/max elevation (skipping nodata)
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for &v in &grid {
        if !v.is_nan() && v > -9999.0 {
            min_z = min_z.min(v);
            max_z = max_z.max(v);
        }
    }
    if min_z == f64::INFINITY {
        return Err("DEM contains no valid elevation data".into());
    }

    // Compute geographic bounds from tiepoint + pixel scale
    let bounds = match (&header.model_tiepoint, &header.model_pixel_scale) {
        (Some(tp), Some(scale)) => {
            let min_x = tp[3];
            let max_y = tp[4];
            let max_x = min_x + scale[0] * width as f64;
            let min_y = max_y - scale[1] * height as f64;
            [min_x, min_y, max_x, max_y]
        }
        _ => return Err("GeoTIFF lacks tiepoint or pixel scale — cannot determine bounds".into()),
    };

    // Compute cell size in meters (for hillshade slope calculation)
    let (cell_w_m, cell_h_m) = derive_cell_meters(&header);

    // Render: for each cell, compute hillshade + apply color ramp.
    //
    // Parallelized with rayon across rows. Each row is independent
    // (the 3×3 window reads from row-1, row, row+1 but never writes),
    // so we can compute all rows in parallel and collect the RGBA
    // bytes at the end. For a 5000×5000 DEM this cuts the render from
    // ~3s (single-threaded) to ~500ms-1s on an 8-core machine.
    let z_scale = request.z_scale;
    let azimuth_rad = request.azimuth.to_radians();
    let altitude_rad = request.altitude.to_radians();
    let cos_altitude = altitude_rad.cos();
    let sin_altitude = altitude_rad.sin();
    let cell_w_m_f64 = cell_w_m;
    let cell_h_m_f64 = cell_h_m;
    let color_ramp = request.color_ramp.clone();
    let min_z_f64 = min_z;
    let max_z_f64 = max_z;
    let width_usize = width as usize;
    let height_usize = height as usize;

    use rayon::prelude::*;
    let row_pixels: Vec<Vec<u8>> = (0..height_usize)
        .into_par_iter()
        .map(|row| {
            let mut row_rgba = Vec::with_capacity(width_usize * 4);
            for col in 0..width_usize {
                let idx = row * width_usize + col;
                let z = grid[idx];

                // Skip nodata — render transparent
                if z.is_nan() || z <= -9999.0 {
                    row_rgba.extend_from_slice(&[0, 0, 0, 0]);
                    continue;
                }

                // ── Hillshade computation ──
                // Standard 3×3 window:
                //   [NW] [N ] [NE]
                //   [W ] [C ] [E ]
                //   [SW] [S ] [SE]
                let get_z = |r: i32, c: i32| -> f64 {
                    let r = r.clamp(0, height as i32 - 1) as usize;
                    let c = c.clamp(0, width as i32 - 1) as usize;
                    let v = grid[r * width_usize + c];
                    if v.is_nan() || v <= -9999.0 {
                        z
                    } else {
                        v
                    }
                };

                let z_nw = get_z(row as i32 - 1, col as i32 - 1) * z_scale;
                let z_n = get_z(row as i32 - 1, col as i32) * z_scale;
                let z_ne = get_z(row as i32 - 1, col as i32 + 1) * z_scale;
                let z_w = get_z(row as i32, col as i32 - 1) * z_scale;
                let z_e = get_z(row as i32, col as i32 + 1) * z_scale;
                let z_sw = get_z(row as i32 + 1, col as i32 - 1) * z_scale;
                let z_s = get_z(row as i32 + 1, col as i32) * z_scale;
                let z_se = get_z(row as i32 + 1, col as i32 + 1) * z_scale;

                // Slope in x and y directions (dz/dx and dz/dy)
                let dz_dx =
                    ((z_ne + 2.0 * z_e + z_se) - (z_nw + 2.0 * z_w + z_sw)) / (8.0 * cell_w_m_f64);
                let dz_dy =
                    ((z_sw + 2.0 * z_s + z_se) - (z_nw + 2.0 * z_n + z_ne)) / (8.0 * cell_h_m_f64);

                // Slope and aspect
                let slope = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt().atan();
                let aspect = if dz_dx != 0.0 {
                    dz_dy.atan2(-dz_dx)
                } else if dz_dy > 0.0 {
                    std::f64::consts::FRAC_PI_2
                } else {
                    -std::f64::consts::FRAC_PI_2
                };

                // Hillshade value (0-255)
                let shade = {
                    let cos_term = cos_altitude * slope.cos();
                    let sin_term = sin_altitude * slope.sin() * (azimuth_rad - aspect).cos();
                    let value = ((cos_term + sin_term).max(0.0) * 255.0) as u8;
                    value
                };

                // ── Color ramp ──
                let (r, g, b) = match color_ramp.as_str() {
                    "bathy" => color_ramp_bathy(z, min_z_f64, max_z_f64),
                    "grayscale" => {
                        let normalized =
                            ((z - min_z_f64) / (max_z_f64 - min_z_f64).max(0.001) * 255.0) as u8;
                        (normalized, normalized, normalized)
                    }
                    _ => color_ramp_terrain(z, min_z_f64, max_z_f64),
                };

                // Apply hillshade as a brightness multiplier (0.3 to 1.0)
                let brightness = 0.3 + 0.7 * (shade as f64 / 255.0);
                let r = ((r as f64) * brightness).min(255.0) as u8;
                let g = ((g as f64) * brightness).min(255.0) as u8;
                let b = ((b as f64) * brightness).min(255.0) as u8;

                row_rgba.extend_from_slice(&[r, g, b, 255]);
            }
            row_rgba
        })
        .collect();

    // Flatten the per-row vectors into a single RGBA buffer
    let mut rgba = Vec::with_capacity(n * 4);
    for row_pixels in row_pixels {
        rgba.extend_from_slice(&row_pixels);
    }

    Ok(DemRenderResult {
        width,
        height,
        bounds,
        rgba,
        min_z,
        max_z,
        epsg: header.epsg,
    })
}

/// Terrain color ramp: blue → green → yellow → brown → white
fn color_ramp_terrain(z: f64, min_z: f64, max_z: f64) -> (u8, u8, u8) {
    let range = (max_z - min_z).max(0.001);
    let t = ((z - min_z) / range).clamp(0.0, 1.0);

    // 5-stop gradient
    let stops = [
        (0.0, (50, 100, 200)),  // deep water blue
        (0.2, (80, 180, 100)),  // green (lowlands)
        (0.5, (200, 200, 80)),  // yellow (mid)
        (0.75, (160, 110, 60)), // brown (high)
        (1.0, (240, 240, 240)), // white (peaks)
    ];

    interpolate_ramp(t, &stops)
}

/// Bathymetric color ramp: dark blue → light blue → cyan → yellow → red
fn color_ramp_bathy(z: f64, min_z: f64, max_z: f64) -> (u8, u8, u8) {
    let range = (max_z - min_z).max(0.001);
    let t = ((z - min_z) / range).clamp(0.0, 1.0);

    let stops = [
        (0.0, (20, 40, 100)),    // deep
        (0.25, (40, 100, 180)),  // mid blue
        (0.5, (80, 180, 220)),   // shallow blue
        (0.75, (200, 220, 100)), // shallow yellow
        (1.0, (220, 100, 60)),   // red (shoal)
    ];

    interpolate_ramp(t, &stops)
}

/// Linear interpolation between color ramp stops
fn interpolate_ramp(t: f64, stops: &[(f64, (u8, u8, u8))]) -> (u8, u8, u8) {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }

    for i in 0..stops.len() - 1 {
        if t >= stops[i].0 && t <= stops[i + 1].0 {
            let local_t = (t - stops[i].0) / (stops[i + 1].0 - stops[i].0);
            let (r1, g1, b1) = stops[i].1;
            let (r2, g2, b2) = stops[i + 1].1;
            return (
                (r1 as f64 + (r2 as f64 - r1 as f64) * local_t) as u8,
                (g1 as f64 + (g2 as f64 - g1 as f64) * local_t) as u8,
                (b1 as f64 + (b2 as f64 - b1 as f64) * local_t) as u8,
            );
        }
    }

    stops[stops.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_ramp_terrain_extremes() {
        let (r, g, b) = color_ramp_terrain(0.0, 0.0, 100.0);
        assert_eq!((r, g, b), (50, 100, 200)); // min → blue

        let (r, g, b) = color_ramp_terrain(100.0, 0.0, 100.0);
        assert_eq!((r, g, b), (240, 240, 240)); // max → white
    }

    #[test]
    fn test_color_ramp_bathy_extremes() {
        let (r, g, b) = color_ramp_bathy(0.0, 0.0, 100.0);
        assert_eq!((r, g, b), (20, 40, 100)); // deep → dark blue

        let (r, g, b) = color_ramp_bathy(100.0, 0.0, 100.0);
        assert_eq!((r, g, b), (220, 100, 60)); // shallow → red
    }

    #[test]
    fn test_interpolate_ramp_midpoint() {
        let stops = [(0.0, (0, 0, 0)), (1.0, (100, 100, 100))];
        let (r, g, b) = interpolate_ramp(0.5, &stops);
        assert_eq!((r, g, b), (50, 50, 50));
    }

    #[test]
    fn test_interpolate_ramp_clamp() {
        let stops = [(0.0, (10, 20, 30)), (1.0, (100, 200, 255))];
        assert_eq!(interpolate_ramp(-0.5, &stops), (10, 20, 30));
        assert_eq!(interpolate_ramp(1.5, &stops), (100, 200, 255));
    }
}
