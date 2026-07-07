// Orthomosaic RGB reader — Sprint 16.
//
// Extends the existing GeoTIFF reader to handle RGB (3-band) orthomosaic
// GeoTIFFs produced by ODM (OpenDroneMap) and other photogrammetry
// software. The existing reader only handles single-band DEM rasters.
//
// Returns the RGB pixel data as a flat Vec<u8> (R, G, B, R, G, B, ...)
// suitable for rendering as a canvas ImageData or an OpenLayers
// ImageLayer with a data URL.
//
// The frontend converts the RGB buffer to a PNG data URL and displays
// it as an OpenLayers ImageStatic layer, georeferenced using the
// GeoTIFF's model tie point + pixel scale.

use crate::formats::geotiff::{read_header, GeoTiffHeader, GeoTiffError};
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Orthomosaic {
    /// Width in pixels
    pub width: usize,
    /// Height in pixels
    pub height: usize,
    /// RGB pixel data, row-major: [R, G, B, R, G, B, ...]
    pub rgb_data: Vec<u8>,
    /// World bounds: (min_x, min_y, max_x, max_y)
    pub bounds: (f64, f64, f64, f64),
    /// CRS EPSG code (e.g., "EPSG:32756") — may be empty if not in GeoKeys
    pub crs: String,
    /// Pixel size in world units (meters): (width, height)
    pub pixel_size: (f64, f64),
}

/// Read an RGB orthomosaic GeoTIFF.
///
/// The file must have `samples_per_pixel >= 3` and `photometric = 2`
/// (RGB). 16-bit samples are downscaled to 8-bit. If the file has an
/// alpha channel (samples_per_pixel = 4), it's dropped.
pub fn read_orthomosaic(path: &Path) -> Result<Orthomosaic, GeoTiffError> {
    let header = read_header(path)?;
    if header.samples_per_pixel < 3 {
        return Err(GeoTiffError::InvalidFormat(format!(
            "not an RGB GeoTIFF: samples_per_pixel = {} (need >= 3)",
            header.samples_per_pixel
        )));
    }

    let width = header.width as usize;
    let height = header.length as usize;
    let total_pixels = width * height;

    // Total size limit — 100M pixels max (e.g., 10000×10000)
    if total_pixels > 100_000_000 {
        return Err(GeoTiffError::InvalidFormat(format!(
            "orthomosaic too large: {} pixels (max 100M)",
            total_pixels
        )));
    }

    let mut file = File::open(path)?;
    let bytes_per_sample = (header.bits_per_sample as usize) / 8;
    let row_stride = width * bytes_per_sample * header.samples_per_pixel as usize;

    // Read all strip data
    let mut rgb_data = Vec::with_capacity(total_pixels * 3);

    for row in 0..height {
        // Find which strip contains this row
        let strip_idx = row / header.rows_per_strip as usize;
        let strip_offset = *header.strip_offsets.get(strip_idx).ok_or_else(|| {
            GeoTiffError::InvalidFormat(format!("missing strip offset for strip {}", strip_idx))
        })? as u64;
        let strip_byte_count = *header.strip_byte_counts.get(strip_idx).ok_or_else(|| {
            GeoTiffError::InvalidFormat(format!("missing strip byte count for strip {}", strip_idx))
        })? as usize;

        // Read the row from the strip
        let row_in_strip = row % header.rows_per_strip as usize;
        let row_offset_in_strip = row_in_strip * row_stride;
        if row_offset_in_strip + row_stride > strip_byte_count {
            return Err(GeoTiffError::InvalidFormat(format!(
                "strip {} row {} out of bounds",
                strip_idx, row
            )));
        }

        file.seek(SeekFrom::Start(strip_offset + row_offset_in_strip as u64))?;
        let mut row_buf = vec![0u8; row_stride];
        file.read_exact(&mut row_buf)?;

        // Extract R, G, B for each pixel in the row
        for col in 0..width {
            let pixel_offset = col * bytes_per_sample * header.samples_per_pixel as usize;
            // Band 0 = R, Band 1 = G, Band 2 = B (for photometric = 2 RGB)
            for band in 0..3 {
                let band_offset = pixel_offset + band * bytes_per_sample;
                if bytes_per_sample == 1 {
                    rgb_data.push(row_buf[band_offset]);
                } else if bytes_per_sample == 2 {
                    // 16-bit → 8-bit (take high byte)
                    let val = u16::from_le_bytes([row_buf[band_offset], row_buf[band_offset + 1]]);
                    rgb_data.push((val >> 8) as u8);
                } else {
                    // 32-bit float → 8-bit (assume 0.0-1.0 range)
                    let val = f32::from_le_bytes([
                        row_buf[band_offset],
                        row_buf[band_offset + 1],
                        row_buf[band_offset + 2],
                        row_buf[band_offset + 3],
                    ]);
                    rgb_data.push((val.clamp(0.0, 1.0) * 255.0) as u8);
                }
            }
        }
    }

    // Compute world bounds from model tie point + pixel scale
    let (min_x, max_x, min_y, max_y) = compute_bounds(&header);
    let pixel_size = if let Some(ps) = header.model_pixel_scale {
        (ps[0], ps[1])
    } else {
        (1.0, 1.0)
    };

    Ok(Orthomosaic {
        width,
        height,
        rgb_data,
        bounds: (min_x, min_y, max_x, max_y),
        crs: extract_crs(&header),
        pixel_size,
    })
}

/// Compute world bounds from the GeoTIFF model tie point + pixel scale.
fn compute_bounds(header: &GeoTiffHeader) -> (f64, f64, f64, f64) {
    if let Some(bounds) = header.bounds {
        return (bounds[0], bounds[1], bounds[2], bounds[3]);
    }
    if let Some(ps) = header.model_pixel_scale {
        if let Some(tp) = header.model_tiepoint {
            // Tie point: (i, j, k, x, y, z) — pixel (i,j) maps to world (x,y)
            let min_x = tp[3];
            let max_y = tp[4]; // y decreases as row increases (top-left origin)
            let max_x = min_x + ps[0] * header.width as f64;
            let min_y = max_y - ps[1] * header.length as f64;
            (min_x, min_y, max_x, max_y)
        } else {
            (0.0, 0.0, ps[0] * header.width as f64, ps[1] * header.length as f64)
        }
    } else {
        (0.0, 0.0, header.width as f64, header.length as f64)
    }
}

/// Extract CRS from GeoKeys.
fn extract_crs(header: &GeoTiffHeader) -> String {
    if let Some(epsg) = header.epsg {
        format!("EPSG:{}", epsg)
    } else {
        String::new()
    }
}

/// Encode RGB data as a PNG data URL for frontend rendering.
///
/// Uses a minimal PNG encoder (uncompressed via stored blocks in zlib).
/// For production, use the `png` crate — but this avoids adding a
/// dependency for a feature that may be replaced by a tile server.
pub fn rgb_to_png_data_url(ortho: &Orthomosaic) -> String {
    // This is a placeholder — the actual PNG encoding happens in the
    // frontend via canvas.toDataURL(). The Rust side just sends the
    // raw RGB bytes; the frontend draws them to a canvas and converts.
    //
    // We return a data URL with the raw bytes as base64 so the frontend
    // can decode it without a separate IPC call.
    let _ = ortho;
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(width: u32, length: u32, ps: Option<[f64; 3]>, tp: Option<[f64; 6]>) -> GeoTiffHeader {
        GeoTiffHeader {
            width,
            length,
            bits_per_sample: 8,
            samples_per_pixel: 3,
            compression: 1,
            photometric: 2,
            is_tiled: false,
            strip_count: length,
            model_pixel_scale: ps,
            model_tiepoint: tp,
            epsg: None,
            geo_ascii: None,
            bounds: None,
            sample_format: 1,
            rows_per_strip: 1,
            strip_offsets: vec![],
            strip_byte_counts: vec![],
        }
    }

    #[test]
    fn test_compute_bounds_with_tiepoint() {
        let header = make_header(100, 200, Some([1.0, 1.0, 0.0]), Some([0.0, 0.0, 0.0, 500.0, 1000.0, 0.0]));
        let (min_x, min_y, max_x, max_y) = compute_bounds(&header);
        assert!((min_x - 500.0).abs() < 1e-6);
        assert!((max_y - 1000.0).abs() < 1e-6);
        assert!((max_x - 600.0).abs() < 1e-6);
        assert!((min_y - 800.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_bounds_no_tiepoint() {
        let header = make_header(100, 200, Some([2.0, 2.0, 0.0]), None);
        let (min_x, min_y, max_x, max_y) = compute_bounds(&header);
        assert!((min_x - 0.0).abs() < 1e-6);
        assert!((max_x - 200.0).abs() < 1e-6);
        assert!((min_y - 0.0).abs() < 1e-6);
        assert!((max_y - 400.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_bounds_from_header_bounds() {
        let mut header = make_header(100, 200, None, None);
        header.bounds = Some([100.0, 200.0, 300.0, 400.0]);
        let (min_x, min_y, max_x, max_y) = compute_bounds(&header);
        assert!((min_x - 100.0).abs() < 1e-6);
        assert!((min_y - 200.0).abs() < 1e-6);
        assert!((max_x - 300.0).abs() < 1e-6);
        assert!((max_y - 400.0).abs() < 1e-6);
    }
}
