// Backscatter mosaicking — process MBES backscatter into a seafloor mosaic.
//
// Backscatter is the acoustic intensity returned from the seafloor.
// It's used for seafloor classification (rock vs sand vs mud) and
// habitat mapping. The mosaic is a georeferenced raster image where
// each pixel's value represents the average backscatter intensity.
//
// Pipeline:
//   1. Raw backscatter snippets per beam → per-beam intensity (dB)
//   2. Correct for angle of incidence (Lambert's law)
//   3. Grid into a raster mosaic (IDW or mean)
//   4. Output as GeoTIFF or PNG with georeferencing

use serde::{Deserialize, Serialize};

/// A single backscatter sample from a multibeam beam.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackscatterSample {
    /// Across-track distance (meters, positive starboard)
    pub across_track: f64,
    /// Along-track distance (meters, positive forward)
    pub along_track: f64,
    /// Backscatter intensity (dB)
    pub intensity_db: f64,
    /// Beam angle from nadir (degrees)
    pub beam_angle: f64,
    /// Timestamp (Unix seconds)
    pub timestamp: f64,
}

/// Parameters for backscatter mosaicking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MosaicParams {
    /// Grid cell size (meters)
    pub cell_size: f64,
    /// Apply Lambert correction for angle of incidence
    pub apply_lambert_correction: bool,
    /// Mosaic method: "mean" or "max"
    pub method: String,
}

impl Default for MosaicParams {
    fn default() -> Self {
        Self {
            cell_size: 1.0,
            apply_lambert_correction: true,
            method: "mean".to_string(),
        }
    }
}

/// Result of backscatter mosaicking.
#[derive(Debug, Clone, Serialize)]
pub struct BackscatterMosaic {
    /// Number of columns
    pub ncols: usize,
    /// Number of rows
    pub nrows: usize,
    /// Cell size (meters)
    pub cell_size: f64,
    /// Geographic bounds (min_x, min_y, max_x, max_y)
    pub bounds: (f64, f64, f64, f64),
    /// Mosaic data in dB, row-major [row * ncols + col]
    pub data: Vec<f64>,
    /// NODATA value
    pub nodata: f64,
    /// Number of valid cells
    pub valid_cells: usize,
    /// Min/max intensity (dB)
    pub min_db: f64,
    pub max_db: f64,
}

/// Process raw backscatter samples into a georeferenced mosaic.
///
/// 1. Optionally applies Lambert's law correction: I_corrected = I_raw / cos(angle)
/// 2. Grids the corrected intensities using mean or max
/// 3. Returns a raster mosaic ready for rendering or export
pub fn create_mosaic(
    samples: &[BackscatterSample],
    params: &MosaicParams,
) -> Result<BackscatterMosaic, String> {
    if samples.is_empty() {
        return Err("no backscatter samples".to_string());
    }

    // Compute bounds
    let (min_at, max_at, min_al, max_al) = samples.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
        |(mn_at, mx_at, mn_al, mx_al), s| {
            (
                mn_at.min(s.across_track),
                mx_at.max(s.across_track),
                mn_al.min(s.along_track),
                mx_al.max(s.along_track),
            )
        },
    );

    let cell = params.cell_size;
    let ncols = (((max_at - min_at) / cell).ceil() as usize + 1).max(1);
    let nrows = (((max_al - min_al) / cell).ceil() as usize + 1).max(1);

    // Cap grid size
    const MAX_DIM: usize = 2000;
    let (ncols, nrows) = if ncols > MAX_DIM || nrows > MAX_DIM {
        let scale = (ncols.max(nrows) as f64 / MAX_DIM as f64).max(1.0);
        let new_cell = cell * scale;
        let new_ncols = (((max_at - min_at) / new_cell).ceil() as usize + 1).min(MAX_DIM);
        let new_nrows = (((max_al - min_al) / new_cell).ceil() as usize + 1).min(MAX_DIM);
        (new_ncols, new_nrows)
    } else {
        (ncols, nrows)
    };

    let nodata = -9999.0;
    let mut sum_grid = vec![0.0f64; ncols * nrows];
    let mut count_grid = vec![0u32; ncols * nrows];
    let mut max_grid = vec![f64::NEG_INFINITY; ncols * nrows];

    // Grid samples
    for s in samples {
        let col = (((s.across_track - min_at) / cell).floor() as usize).min(ncols - 1);
        let row = (((s.along_track - min_al) / cell).floor() as usize).min(nrows - 1);
        let idx = row * ncols + col;

        // Apply Lambert correction if requested
        let intensity = if params.apply_lambert_correction {
            let angle_rad = s.beam_angle.to_radians();
            let cos_a = angle_rad.cos().max(0.01); // avoid div by zero
            s.intensity_db + 10.0 * cos_a.log10() // Lambert correction in dB
        } else {
            s.intensity_db
        };

        sum_grid[idx] += intensity;
        count_grid[idx] += 1;
        if intensity > max_grid[idx] {
            max_grid[idx] = intensity;
        }
    }

    // Compute final mosaic
    let mut data = vec![nodata; ncols * nrows];
    let mut valid_cells = 0;
    let mut min_db = f64::INFINITY;
    let mut max_db = f64::NEG_INFINITY;

    for i in 0..ncols * nrows {
        if count_grid[i] > 0 {
            data[i] = if params.method == "max" {
                max_grid[i]
            } else {
                sum_grid[i] / count_grid[i] as f64
            };
            valid_cells += 1;
            min_db = min_db.min(data[i]);
            max_db = max_db.max(data[i]);
        }
    }

    if valid_cells == 0 {
        return Err("no valid grid cells after mosaicking".to_string());
    }

    Ok(BackscatterMosaic {
        ncols,
        nrows,
        cell_size: cell,
        bounds: (min_at, min_al, max_at, max_al),
        data,
        nodata,
        valid_cells,
        min_db,
        max_db,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mosaic_basic() {
        let samples = vec![
            BackscatterSample {
                across_track: 0.0, along_track: 0.0, intensity_db: -20.0,
                beam_angle: 0.0, timestamp: 0.0,
            },
            BackscatterSample {
                across_track: 5.0, along_track: 0.0, intensity_db: -25.0,
                beam_angle: 30.0, timestamp: 1.0,
            },
            BackscatterSample {
                across_track: 0.0, along_track: 5.0, intensity_db: -15.0,
                beam_angle: -30.0, timestamp: 2.0,
            },
        ];
        let params = MosaicParams {
            apply_lambert_correction: false,
            ..Default::default()
        };
        let mosaic = create_mosaic(&samples, &params).unwrap();
        assert!(mosaic.valid_cells >= 3);
        assert!(mosaic.min_db < 0.0);
    }

    #[test]
    fn test_empty_samples_error() {
        let result = create_mosaic(&[], &MosaicParams::default());
        assert!(result.is_err());
    }
}
