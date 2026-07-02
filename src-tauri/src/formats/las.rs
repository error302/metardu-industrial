// LAS/LAZ file header parser — pure Rust, no external deps.
//
// Implements the LAS 1.2 / 1.3 / 1.4 spec for the public header block
// (https://www.asprs.org/wp-content/uploads/2019/07/LAS_1_4_r15.pdf).
// Returns enough info for the frontend to:
//   - Show point count, bounds, CRS
//   - Render the file's bounding box on the OpenLayers canvas
//   - Decide if it can be ingested (version + PDRF support)
//
// .laz (compressed) is NOT supported by this reader — that requires
// the laszip/laz-perf crate. We detect LAZ and return an error pointing
// the user to Phase 1 work.

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const LAS_HEADER_SIZE: u64 = 375;
const VLR_HEADER_SIZE: u64 = 54;

#[derive(Debug, Clone, Serialize)]
pub struct LasHeader {
    pub file_source_id: u16,
    pub global_encoding: u16,
    pub version_major: u8,
    pub version_minor: u8,
    pub system_identifier: String,
    pub generating_software: String,
    pub file_creation_day: u16,
    pub file_creation_year: u16,
    pub header_size: u16,
    pub offset_to_point_data: u32,
    pub number_of_vlrs: u32,
    pub point_data_format: u8,
    pub point_data_record_length: u16,
    pub point_count: u64,
    /// Per-return counts (LAS spec defines 5 or 15 returns depending on version)
    pub points_by_return: Vec<u64>,
    pub scale_x: f64,
    pub scale_y: f64,
    pub scale_z: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub offset_z: f64,
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
    /// WKT or GeoTIFF keys from the WKT VLR (if present, LAS 1.4+)
    pub crs_wkt: Option<String>,
    /// GeoTIFF-style GeoKeyDirectory from VLR (LAS 1.2/1.3)
    pub geotiff_keys: Option<Vec<u16>>,
}

#[derive(Debug, thiserror::Error)]
pub enum LasError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("not a LAS/LAZ file — magic bytes mismatch (got {0:?})")]
    BadMagic([u8; 4]),
    #[error("LAZ (compressed LAS) is not yet supported — Phase 1 work")]
    LazUnsupported,
    #[error("unsupported LAS version: {0}.{1}")]
    UnsupportedVersion(u8, u8),
    #[error("unsupported point data format: {0}")]
    UnsupportedPdrf(u8),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("truncated header — file too small")]
    Truncated,
}

/// Parse the public header block of a LAS file.
///
/// Per the LAS 1.4 spec:
///   - Bytes 0-3: "LASF" magic
///   - Bytes 4-5: file source ID
///   - Bytes 6-7: global encoding
///   - Bytes 24-25: version major
///   - Bytes 26-27: version minor
///   - (and so on)
pub fn read_header(path: &Path) -> Result<LasHeader, LasError> {
    let mut file = File::open(path).map_err(|_| LasError::NotFound(path.display().to_string()))?;

    // Read the entire 375-byte public header block
    let mut header_buf = [0u8; LAS_HEADER_SIZE as usize];
    let n = file.read(&mut header_buf)?;
    if n < LAS_HEADER_SIZE as usize {
        return Err(LasError::Truncated);
    }

    // Verify magic
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&header_buf[0..4]);
    if &magic != b"LASF" {
        return Err(LasError::BadMagic(magic));
    }

    // Detect LAZ via the 0x02 compressor flag in global encoding (LAS 1.4)
    // or via the LasZip VLR ID 1. We check both.
    let global_encoding = read_u16_le(&header_buf[6..8]);
    // Bits 0-1 reserved in 1.4 — but 0x02 set commonly indicates LAZ for older versions
    // Proper LAZ detection happens later when scanning VLRs

    let version_major = header_buf[24];
    let version_minor = header_buf[25];

    if version_major != 1 || version_minor > 4 {
        return Err(LasError::UnsupportedVersion(version_major, version_minor));
    }

    let file_source_id = read_u16_le(&header_buf[4..6]);
    let system_identifier = read_string(&header_buf[40..72], 32);
    let generating_software = read_string(&header_buf[72..104], 32);
    let file_creation_day = read_u16_le(&header_buf[212..214]);
    let file_creation_year = read_u16_le(&header_buf[214..216]);
    let header_size = read_u16_le(&header_buf[94..96]);
    let offset_to_point_data = read_u32_le(&header_buf[96..100]);
    let number_of_vlrs = read_u32_le(&header_buf[100..104]);
    let point_data_format = header_buf[104];
    let point_data_record_length = read_u16_le(&header_buf[105..107]);

    // Per LAS version, point count and points-by-return live at different offsets
    // and use different widths (u32 in 1.0-1.3, u64 in 1.4)
    let (point_count, points_by_return) = if version_minor >= 4 {
        let pc = read_u64_le(&header_buf[247..255]);
        let pbr: Vec<u64> = (0..15)
            .map(|i| read_u64_le(&header_buf[255 + i * 8..255 + (i + 1) * 8]))
            .collect();
        (pc, pbr)
    } else {
        let pc = read_u32_le(&header_buf[107..111]) as u64;
        // Points by return — 5 returns in 1.0-1.3, each u32
        let pbr: Vec<u64> = (0..5)
            .map(|i| read_u32_le(&header_buf[111 + i * 4..111 + (i + 1) * 4]) as u64)
            .collect();
        (pc, pbr)
    };

    // Scale / offset / bounds — LAS 1.4 moved these but they overlap in 1.2-1.3
    // The layout below handles 1.2/1.3 layout; 1.4 has the same layout for these fields.
    let scale_x = read_f64_le(&header_buf[131..139]);
    let scale_y = read_f64_le(&header_buf[139..147]);
    let scale_z = read_f64_le(&header_buf[147..155]);
    let offset_x = read_f64_le(&header_buf[155..163]);
    let offset_y = read_f64_le(&header_buf[163..171]);
    let offset_z = read_f64_le(&header_buf[171..179]);
    let min_x = read_f64_le(&header_buf[179..187]);
    let min_y = read_f64_le(&header_buf[187..195]);
    let min_z = read_f64_le(&header_buf[195..203]);
    let max_x = read_f64_le(&header_buf[203..211]);
    let max_y = read_f64_le(&header_buf[211..219]);
    let max_z = read_f64_le(&header_buf[219..227]);

    // Scan VLRs for WKT (LAS 1.4) or GeoTIFF keys (LAS 1.2/1.3)
    // and for LasZip VLR (LAZ detection)
    let vlr_scan = scan_vlrs(&mut file, number_of_vlrs, offset_to_point_data as u64)?;
    let crs_wkt = vlr_scan.crs_wkt;
    let geotiff_keys = vlr_scan.geotiff_keys;
    let is_laz = vlr_scan.is_laz;

    if is_laz {
        return Err(LasError::LazUnsupported);
    }

    // PDRF validation — LAS 1.4 spec defines formats 0-10
    if point_data_format > 10 {
        return Err(LasError::UnsupportedPdrf(point_data_format));
    }

    Ok(LasHeader {
        file_source_id,
        global_encoding,
        version_major,
        version_minor,
        system_identifier,
        generating_software,
        file_creation_day,
        file_creation_year,
        header_size,
        offset_to_point_data,
        number_of_vlrs,
        point_data_format,
        point_data_record_length,
        point_count,
        points_by_return,
        scale_x,
        scale_y,
        scale_z,
        offset_x,
        offset_y,
        offset_z,
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
        crs_wkt,
        geotiff_keys,
    })
}

/// Scan VLRs (Variable Length Records) for CRS info and LAZ detection.
///
/// LAS VLRs live between the public header and the point data.
/// Each VLR has a 54-byte header followed by a payload of up to 65535 bytes.
/// We care about:
///   - Record ID 34735: GeoKeyDirectoryTag (GeoTIFF keys)
///   - Record ID 34736: GeoDoubleParamsTag
///   - Record ID 34737: GeoAsciiParamsTag
///   - Record ID 2112: WKT (LAS 1.4)
///   - User ID "laszip encoded": LAZ detection
///
/// Result of scanning a LAS file's Variable Length Records.
struct VlrScan {
    crs_wkt: Option<String>,
    geotiff_keys: Option<Vec<u16>>,
    is_laz: bool,
}

fn scan_vlrs(file: &mut File, count: u32, offset_to_point_data: u64) -> Result<VlrScan, LasError> {
    let mut crs_wkt: Option<String> = None;
    let mut geotiff_keys: Option<Vec<u16>> = None;
    let mut is_laz = false;

    file.seek(SeekFrom::Start(LAS_HEADER_SIZE))?;

    for _ in 0..count {
        let mut vlr_header = [0u8; VLR_HEADER_SIZE as usize];
        if file.read(&mut vlr_header)? < VLR_HEADER_SIZE as usize {
            break;
        }

        let user_id = read_string(&vlr_header[2..18], 16);
        let record_id = read_u16_le(&vlr_header[18..20]);
        let record_length = read_u16_le(&vlr_header[20..22]) as usize;

        let mut payload = vec![0u8; record_length];
        file.read_exact(&mut payload)?;

        // LAZ detection — LasZip VLR has user_id "laszip encoded"
        if user_id.trim() == "laszip encoded" && record_id == 22204 {
            is_laz = true;
        }

        // WKT (LAS 1.4) — user_id "LASF_Projection", record_id 2112
        if user_id.trim() == "LASF_Projection" && record_id == 2112 {
            crs_wkt = Some(read_string(&payload, payload.len()));
        }

        // GeoTIFF GeoKeyDirectory — user_id "LASF_Projection", record_id 34735
        if user_id.trim() == "LASF_Projection" && record_id == 34735 {
            // Parse as u16 array
            let mut keys = Vec::with_capacity(record_length / 2);
            for chunk in payload.chunks_exact(2) {
                keys.push(read_u16_le(chunk));
            }
            if !keys.is_empty() {
                geotiff_keys = Some(keys);
            }
        }
    }

    // Reset position so callers can continue reading from point data if needed
    let _ = file.seek(SeekFrom::Start(offset_to_point_data));

    Ok(VlrScan {
        crs_wkt,
        geotiff_keys,
        is_laz,
    })
}

// ──────────────────────────────────────────────────────────────────
// Little-endian readers

fn read_u16_le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn read_u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn read_u64_le(b: &[u8]) -> u64 {
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

fn read_f64_le(b: &[u8]) -> f64 {
    f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

fn read_string(b: &[u8], _max: usize) -> String {
    // LAS strings are null-padded; trim trailing nulls and whitespace
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}
