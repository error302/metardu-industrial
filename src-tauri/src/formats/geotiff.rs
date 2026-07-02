// GeoTIFF reader — pure Rust, no external deps.
//
// Implements enough of the TIFF 6.0 spec + GeoTIFF 1.0 to extract:
//   - Image dimensions (width, length, bits per sample)
//   - Compression detection (we only fully support uncompressed for now)
//   - GeoTIFF keys: ModelPixelScale (33550), ModelTiepoint (33922),
//     GeoKeyDirectory (34735), GeoAsciiParams (34737), GeoDoubleParams (34736)
//   - Derived geographic bounds from pixel scale + tiepoint
//
// Spec references:
//   - TIFF 6.0: https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf
//   - GeoTIFF 1.0: https://earthdata.nasa.gov/files/STD-REF-v001.3.pdf
//
// Limitations in Phase 0:
//   - Only uncompressed (compression=1) or LZW (compression=5) — no JPEG/DEFLATE
//   - BigTIFF not yet supported (offsets >4GB)
//   - Tile offsets parsed but tile data not yet read — only metadata extraction
//   - CRS extraction from GeoKeyDirectory is partial: we get the EPSG code
//     from GeographicTypeGeoKey (2048) or ProjectedCSTypeGeoKey (3072)
//     when present, but full GeoKey interpretation is Phase 1 work.

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct GeoTiffHeader {
    pub width: u32,
    pub length: u32,
    pub bits_per_sample: u16,
    pub samples_per_pixel: u16,
    pub compression: u16,
    pub photometric: u16,
    /// Strips or tiles — surveyors mostly get strips from orthomosaics
    pub is_tiled: bool,
    pub strip_count: u32,
    /// GeoTIFF: model pixel scale (sx, sy, sz)
    pub model_pixel_scale: Option<[f64; 3]>,
    /// GeoTIFF: model tiepoint (i, j, k, x, y, z)
    pub model_tiepoint: Option<[f64; 6]>,
    /// Derived EPSG code if we can extract one from the GeoKeyDirectory
    pub epsg: Option<u16>,
    /// Raw ASCII GeoTIFF params (citation, CRS name, etc.)
    pub geo_ascii: Option<String>,
    /// Derived geographic bounds (min_x, min_y, max_x, max_y)
    /// None if pixel scale + tiepoint aren't both present
    pub bounds: Option<[f64; 4]>,
}

#[derive(Debug, thiserror::Error)]
pub enum GeoTiffError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("not a TIFF file — magic bytes mismatch (got {0:?})")]
    BadMagic([u8; 2]),
    #[error("BigTIFF not yet supported (Phase 1 work)")]
    BigTiffUnsupported,
    #[error("unsupported compression: {0} (only 1=none, 5=LZW supported)")]
    UnsupportedCompression(u16),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("truncated header")]
    Truncated,
    #[error("invalid IFD entry at offset {0}")]
    InvalidIFDEntry(u64),
}

// TIFF tag IDs we care about
const TAG_IMAGE_WIDTH: u16 = 256;
const TAG_IMAGE_LENGTH: u16 = 257;
const TAG_BITS_PER_SAMPLE: u16 = 258;
const TAG_COMPRESSION: u16 = 259;
const TAG_PHOTOMETRIC: u16 = 262;
const TAG_STRIP_OFFSETS: u16 = 273;
const TAG_SAMPLES_PER_PIXEL: u16 = 277;
const TAG_TILE_OFFSETS: u16 = 324;

// GeoTIFF tags
const TAG_MODEL_PIXEL_SCALE: u16 = 33550;
const TAG_MODEL_TIEPOINT: u16 = 33922;
const TAG_GEO_KEY_DIRECTORY: u16 = 34735;
const TAG_GEO_ASCII_PARAMS: u16 = 34737;

// GeoTIFF key IDs for CRS extraction
const GEOKEY_GEOGRAPHIC_TYPE: u16 = 2048;
const GEOKEY_PROJECTED_CSTYPE: u16 = 3072;

// TIFF data type sizes
const TYPE_SIZES: [u32; 13] = [0, 1, 1, 2, 4, 8, 1, 1, 2, 4, 8, 4, 8];

pub fn read_header(path: &Path) -> Result<GeoTiffHeader, GeoTiffError> {
    let mut file =
        File::open(path).map_err(|_| GeoTiffError::NotFound(path.display().to_string()))?;

    // TIFF header: 8 bytes
    //   bytes 0-1: byte order ("II" = little-endian, "MM" = big-endian)
    //   bytes 2-3: magic (42 for TIFF, 43 for BigTIFF)
    //   bytes 4-7: offset to first IFD
    let mut header = [0u8; 8];
    if file.read(&mut header)? < 8 {
        return Err(GeoTiffError::Truncated);
    }

    let little_endian = match &header[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err(GeoTiffError::BadMagic([header[0], header[1]])),
    };

    let magic = read_u16(&header[2..4], little_endian);
    if magic == 43 {
        return Err(GeoTiffError::BigTiffUnsupported);
    }
    if magic != 42 {
        return Err(GeoTiffError::BadMagic([header[2], header[3]]));
    }

    let first_ifd_offset = read_u32(&header[4..8], little_endian) as u64;
    file.seek(SeekFrom::Start(first_ifd_offset))?;

    // IFD: 2-byte count + count * 12-byte entries + 4-byte next IFD offset
    let mut count_buf = [0u8; 2];
    file.read_exact(&mut count_buf)?;
    let entry_count = read_u16(&count_buf, little_endian);

    let mut width = 0u32;
    let mut length = 0u32;
    let mut bits_per_sample = 1u16;
    let mut samples_per_pixel = 1u16;
    let mut compression = 1u16;
    let mut photometric = 0u16;
    let mut strip_count = 0u32;
    let mut is_tiled = false;
    let mut model_pixel_scale: Option<[f64; 3]> = None;
    let mut model_tiepoint: Option<[f64; 6]> = None;
    let mut geo_key_directory: Option<Vec<u16>> = None;
    let mut geo_ascii: Option<String> = None;

    for i in 0..entry_count {
        let mut entry = [0u8; 12];
        if file.read(&mut entry)? < 12 {
            return Err(GeoTiffError::InvalidIFDEntry(i as u64));
        }
        let tag = read_u16(&entry[0..2], little_endian);
        let type_id = read_u16(&entry[2..4], little_endian);
        let count = read_u32(&entry[4..8], little_endian);
        let value_offset_bytes = &entry[8..12];

        // For values that fit in 4 bytes, the value is stored inline.
        // Otherwise, value_offset_bytes is a pointer to the actual data.
        let type_size = if (type_id as usize) < TYPE_SIZES.len() {
            TYPE_SIZES[type_id as usize]
        } else {
            1
        };
        let total_bytes = count * type_size;
        let inline = total_bytes <= 4;

        match tag {
            TAG_IMAGE_WIDTH => {
                width = read_ifd_value_u32(value_offset_bytes, type_id, little_endian);
            }
            TAG_IMAGE_LENGTH => {
                length = read_ifd_value_u32(value_offset_bytes, type_id, little_endian);
            }
            TAG_BITS_PER_SAMPLE => {
                bits_per_sample = read_ifd_value_u16(value_offset_bytes, type_id, little_endian);
            }
            TAG_COMPRESSION => {
                compression = read_ifd_value_u16(value_offset_bytes, type_id, little_endian);
            }
            TAG_PHOTOMETRIC => {
                photometric = read_ifd_value_u16(value_offset_bytes, type_id, little_endian);
            }
            TAG_SAMPLES_PER_PIXEL => {
                samples_per_pixel = read_ifd_value_u16(value_offset_bytes, type_id, little_endian);
            }
            TAG_STRIP_OFFSETS => {
                strip_count = count;
                is_tiled = false;
            }
            TAG_TILE_OFFSETS => {
                strip_count = count;
                is_tiled = true;
            }
            TAG_MODEL_PIXEL_SCALE => {
                if count >= 3 && type_id == 12 {
                    // f64 array
                    let bytes = read_value_data(
                        &mut file,
                        value_offset_bytes,
                        inline,
                        total_bytes,
                        little_endian,
                    )?;
                    if bytes.len() >= 24 {
                        model_pixel_scale = Some([
                            read_f64(&bytes[0..8], little_endian),
                            read_f64(&bytes[8..16], little_endian),
                            read_f64(&bytes[16..24], little_endian),
                        ]);
                    }
                }
            }
            TAG_MODEL_TIEPOINT => {
                if count >= 6 && type_id == 12 {
                    let bytes = read_value_data(
                        &mut file,
                        value_offset_bytes,
                        inline,
                        total_bytes,
                        little_endian,
                    )?;
                    if bytes.len() >= 48 {
                        model_tiepoint = Some([
                            read_f64(&bytes[0..8], little_endian),
                            read_f64(&bytes[8..16], little_endian),
                            read_f64(&bytes[16..24], little_endian),
                            read_f64(&bytes[24..32], little_endian),
                            read_f64(&bytes[32..40], little_endian),
                            read_f64(&bytes[40..48], little_endian),
                        ]);
                    }
                }
            }
            TAG_GEO_KEY_DIRECTORY => {
                if type_id == 3 {
                    // u16 array — first 4 values are KeyDirectoryVersion, KeyRevision,
                    // MinorRevision, NumberOfKeys; then 4-tuples per key
                    let bytes = read_value_data(
                        &mut file,
                        value_offset_bytes,
                        inline,
                        total_bytes,
                        little_endian,
                    )?;
                    let mut keys = Vec::with_capacity(count as usize);
                    for chunk in bytes.chunks_exact(2) {
                        keys.push(read_u16(chunk, little_endian));
                    }
                    if !keys.is_empty() {
                        geo_key_directory = Some(keys);
                    }
                }
            }
            TAG_GEO_ASCII_PARAMS => {
                if type_id == 2 {
                    let bytes = read_value_data(
                        &mut file,
                        value_offset_bytes,
                        inline,
                        total_bytes,
                        little_endian,
                    )?;
                    // ASCII params use '|' as field terminator
                    let s = String::from_utf8_lossy(&bytes)
                        .trim_end_matches('|')
                        .trim()
                        .to_string();
                    if !s.is_empty() {
                        geo_ascii = Some(s);
                    }
                }
            }
            _ => {} // ignore tags we don't need for Phase 0
        }
    }

    if compression != 1 && compression != 5 {
        return Err(GeoTiffError::UnsupportedCompression(compression));
    }

    // Extract EPSG code from GeoKeyDirectory
    let epsg = extract_epsg_from_geokeys(&geo_key_directory);

    // Derive geographic bounds from pixel scale + tiepoint
    // ModelTiepoint = (i, j, k, x, y, z) — pixel (i,j) maps to geo (x,y)
    // ModelPixelScale = (sx, sy, sz) — pixel size in geo units
    // Bounds: min_x = x, max_x = x + width * sx
    //         max_y = y, min_y = y - length * sy  (y decreases downward in raster)
    let bounds = match (model_pixel_scale, model_tiepoint) {
        (Some(scale), Some(tiepoint)) => {
            let x = tiepoint[3];
            let y = tiepoint[4];
            let sx = scale[0];
            let sy = scale[1];
            Some([x, y - length as f64 * sy, x + width as f64 * sx, y])
        }
        _ => None,
    };

    Ok(GeoTiffHeader {
        width,
        length,
        bits_per_sample,
        samples_per_pixel,
        compression,
        photometric,
        is_tiled,
        strip_count,
        model_pixel_scale,
        model_tiepoint,
        epsg,
        geo_ascii,
        bounds,
    })
}

/// Extract the EPSG code from the GeoKeyDirectory if present.
///
/// GeoKeyDirectory format: [KeyDirVersion, KeyRevision, MinorRevision, NumberOfKeys,
///   KeyId1, TIFFTagLocation1, Count1, Value_Offset1,
///   KeyId2, TIFFTagLocation2, Count2, Value_Offset2,
///   ...]
///
/// We look for:
///   - GeographicTypeGeoKey (2048) — value is the EPSG geographic CRS code
///   - ProjectedCSTypeGeoKey (3072) — value is the EPSG projected CRS code
fn extract_epsg_from_geokeys(geo_keys: &Option<Vec<u16>>) -> Option<u16> {
    let keys = geo_keys.as_ref()?;
    if keys.len() < 4 {
        return None;
    }
    // Skip header (4 values), then iterate 4-tuples
    let mut i = 4;
    while i + 3 < keys.len() {
        let key_id = keys[i];
        let tiff_tag_location = keys[i + 1];
        let _count = keys[i + 2];
        let value_offset = keys[i + 3];

        // When tiff_tag_location == 0, the value is stored inline in value_offset
        if tiff_tag_location == 0 {
            match key_id {
                GEOKEY_GEOGRAPHIC_TYPE | GEOKEY_PROJECTED_CSTYPE => {
                    if value_offset > 0 {
                        return Some(value_offset);
                    }
                }
                _ => {}
            }
        }
        i += 4;
    }
    None
}

// ──────────────────────────────────────────────────────────────────
// Helpers

fn read_u16(b: &[u8], le: bool) -> u16 {
    if le {
        u16::from_le_bytes([b[0], b[1]])
    } else {
        u16::from_be_bytes([b[0], b[1]])
    }
}

fn read_u32(b: &[u8], le: bool) -> u32 {
    if le {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    } else {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }
}

fn read_f64(b: &[u8], le: bool) -> f64 {
    if le {
        f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    } else {
        f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }
}

fn read_ifd_value_u16(value_offset_bytes: &[u8], type_id: u16, le: bool) -> u16 {
    match type_id {
        1 | 7 => value_offset_bytes[0] as u16, // BYTE
        3 => read_u16(value_offset_bytes, le), // SHORT
        _ => read_u16(value_offset_bytes, le),
    }
}

fn read_ifd_value_u32(value_offset_bytes: &[u8], type_id: u16, le: bool) -> u32 {
    match type_id {
        1 | 7 => value_offset_bytes[0] as u32,        // BYTE
        3 => read_u16(value_offset_bytes, le) as u32, // SHORT
        4 => read_u32(value_offset_bytes, le),        // LONG
        _ => read_u32(value_offset_bytes, le),
    }
}

/// Read value data either inline (from the 4-byte IFD entry slot) or
/// by seeking to the offset stored in those 4 bytes.
fn read_value_data(
    file: &mut File,
    value_offset_bytes: &[u8],
    inline: bool,
    total_bytes: u32,
    le: bool,
) -> Result<Vec<u8>, GeoTiffError> {
    if inline {
        // Value fits in 4 bytes — copy from the IFD entry directly
        let bytes_to_copy = total_bytes as usize;
        Ok(value_offset_bytes[..bytes_to_copy].to_vec())
    } else {
        let offset = read_u32(value_offset_bytes, le) as u64;
        let mut buf = vec![0u8; total_bytes as usize];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buf)?;
        Ok(buf)
    }
}
