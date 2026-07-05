// LAS 1.2 / 1.3 / 1.4 reader with LAZ decompression support.
//
// Implements the ASPRS LAS specification (Public Header Block, VLR scanning,
// point record parsing for formats 0–10) and transparent LAZ decompression
// via the `laz` crate's `LasZipDecompressor`.
//
// Only the X / Y / Z coordinates are decoded from each point record — all
// point formats share the same layout for the first 12 bytes (three i32
// values that, when multiplied by the per-axis scale and added to the
// per-axis offset, yield geographic coordinates in metres).
//
// LAZ files are detected by scanning the Variable Length Records for a
// record with user_id == "laszip encoded" (record_id == 222). The 52-byte
// payload of that VLR is the LAZ description record needed to seed
// `LasZipDecompressor::new`.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

const LASF_SIGNATURE: &[u8; 4] = b"LASF";
const LASZIP_USER_ID: &str = "laszip encoded";
const LASZIP_RECORD_ID: u16 = 222;

/// Parsed LAS public header block plus a flag indicating whether the file
/// is LAZ-compressed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LasHeader {
    /// 4-byte file signature — always "LASF" for a valid LAS file.
    pub file_signature: [u8; 4],
    /// File source ID (LAS 1.2+).
    pub file_source_id: u16,
    /// Global encoding bitfield (LAS 1.3+).
    pub global_encoding: u16,
    /// Project ID GUID (raw 16 bytes).
    pub project_id: [u8; 16],
    /// LAS format major version (always 1).
    pub version_major: u8,
    /// LAS format minor version (2, 3, or 4).
    pub version_minor: u8,
    /// System identifier (32-byte ASCII, null-padded).
    pub system_identifier: String,
    /// Generating software (32-byte ASCII, null-padded).
    pub generating_software: String,
    /// File creation day-of-year (1–366).
    pub file_creation_day: u16,
    /// File creation year (4-digit, e.g. 2024).
    pub file_creation_year: u16,
    /// Size of the public header block in bytes (227 / 235 / 375).
    pub header_size: u16,
    /// Byte offset from the start of file to the first point record.
    pub offset_to_point_data: u64,
    /// Number of Variable Length Records between the header and the points.
    pub num_vlrs: u32,
    /// Point data format ID (0–10).
    pub point_data_format_id: u8,
    /// Size of a single point data record in bytes.
    pub point_data_record_length: u16,
    /// Total number of point records (uses the LAS 1.4 extended field when
    /// the legacy 32-bit field is zero).
    pub num_point_records: u64,
    /// Number of points by return (5 entries for LAS 1.2/1.3, 15 for 1.4 —
    /// padded with zeros for 1.2/1.3).
    pub num_points_by_return: [u64; 15],
    /// X scale factor.
    pub x_scale: f64,
    /// Y scale factor.
    pub y_scale: f64,
    /// Z scale factor.
    pub z_scale: f64,
    /// X offset.
    pub x_offset: f64,
    /// Y offset.
    pub y_offset: f64,
    /// Z offset.
    pub z_offset: f64,
    /// Maximum X.
    pub max_x: f64,
    /// Minimum X.
    pub min_x: f64,
    /// Maximum Y.
    pub max_y: f64,
    /// Minimum Y.
    pub min_y: f64,
    /// Maximum Z.
    pub max_z: f64,
    /// Minimum Z.
    pub min_z: f64,
    /// True if a LAZ VLR was found in the header.
    pub is_laz: bool,
    /// Raw LAZ VLR payload (the bytes that follow the 54-byte VLR header).
    /// Skipped during Serde round-trips because it is meaningless without
    /// the surrounding file context.
    #[serde(skip)]
    pub laz_vlr_payload: Option<Vec<u8>>,
}

#[derive(Debug, thiserror::Error)]
pub enum LasError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("file is too short for a LAS header (read {0} bytes, need ≥227)")]
    HeaderTooShort(usize),
    #[error("invalid LAS file signature — expected 'LASF'")]
    InvalidSignature,
    #[error("unsupported LAS version: {0}.{1}")]
    UnsupportedVersion(u8, u8),
    #[error("LAZ file is missing its laszip VLR")]
    MissingLazVlr,
    #[error("LAZ decompression error: {0}")]
    LazDecompression(String),
    #[error("point record is shorter than 12 bytes ({0})")]
    PointRecordTooShort(u16),
}

/// Read and parse the LAS public header block, including VLR scanning for
/// LAZ detection.
pub fn read_header(path: &Path) -> Result<LasHeader, LasError> {
    let mut file = File::open(path)?;
    let mut header_buf = [0u8; 375];
    let n = read_exact_or_short(&mut file, &mut header_buf)?;
    if n < 227 {
        return Err(LasError::HeaderTooShort(n));
    }
    if &header_buf[0..4] != LASF_SIGNATURE {
        return Err(LasError::InvalidSignature);
    }

    let version_major = header_buf[24];
    let version_minor = header_buf[25];
    if version_major != 1 || !matches!(version_minor, 2 | 3 | 4) {
        return Err(LasError::UnsupportedVersion(version_major, version_minor));
    }

    let file_source_id = u16::from_le_bytes(header_buf[4..6].try_into().unwrap());
    let global_encoding = u16::from_le_bytes(header_buf[6..8].try_into().unwrap());
    let mut project_id = [0u8; 16];
    project_id.copy_from_slice(&header_buf[8..24]);

    let system_identifier = trim_ascii(&header_buf[26..58]);
    let generating_software = trim_ascii(&header_buf[58..90]);
    let file_creation_day = u16::from_le_bytes(header_buf[90..92].try_into().unwrap());
    let file_creation_year = u16::from_le_bytes(header_buf[92..94].try_into().unwrap());
    let header_size = u16::from_le_bytes(header_buf[94..96].try_into().unwrap());
    let offset_to_point_data = u32::from_le_bytes(header_buf[96..100].try_into().unwrap()) as u64;
    let num_vlrs = u32::from_le_bytes(header_buf[100..104].try_into().unwrap());
    let point_data_format_id = header_buf[104];
    let point_data_record_length = u16::from_le_bytes(header_buf[105..107].try_into().unwrap());

    // Legacy number of point records (LAS 1.2/1.3) and per-return counts.
    let legacy_num_points = u32::from_le_bytes(header_buf[107..111].try_into().unwrap()) as u64;
    let mut num_points_by_return = [0u64; 15];
    for i in 0..5usize {
        let v = u32::from_le_bytes(header_buf[111 + i * 4..115 + i * 4].try_into().unwrap()) as u64;
        num_points_by_return[i] = v;
    }

    let x_scale = f64::from_le_bytes(header_buf[131..139].try_into().unwrap());
    let y_scale = f64::from_le_bytes(header_buf[139..147].try_into().unwrap());
    let z_scale = f64::from_le_bytes(header_buf[147..155].try_into().unwrap());
    let x_offset = f64::from_le_bytes(header_buf[155..163].try_into().unwrap());
    let y_offset = f64::from_le_bytes(header_buf[163..171].try_into().unwrap());
    let z_offset = f64::from_le_bytes(header_buf[171..179].try_into().unwrap());
    let max_x = f64::from_le_bytes(header_buf[179..187].try_into().unwrap());
    let min_x = f64::from_le_bytes(header_buf[187..195].try_into().unwrap());
    let max_y = f64::from_le_bytes(header_buf[195..203].try_into().unwrap());
    let min_y = f64::from_le_bytes(header_buf[203..211].try_into().unwrap());
    let max_z = f64::from_le_bytes(header_buf[211..219].try_into().unwrap());
    let min_z = f64::from_le_bytes(header_buf[219..227].try_into().unwrap());

    // LAS 1.4 extended fields: extended number of point records (offset 247)
    // and the 15-entry extended per-return array (offset 255).
    let mut num_point_records = legacy_num_points;
    if version_minor == 4 && n >= 255 {
        let ext = u64::from_le_bytes(header_buf[247..255].try_into().unwrap());
        if ext > 0 {
            num_point_records = ext;
        }
        for i in 0..15usize {
            let off = 255 + i * 8;
            if off + 8 > n {
                break;
            }
            let v = u64::from_le_bytes(header_buf[off..off + 8].try_into().unwrap());
            if v > 0 {
                num_points_by_return[i] = v;
            }
        }
    }

    // Scan Variable Length Records for the LAZ VLR. Each VLR is 54 bytes of
    // header + `record_length_after_header` bytes of payload.
    let mut is_laz = false;
    let mut laz_vlr_payload: Option<Vec<u8>> = None;
    if num_vlrs > 0 {
        let mut vlr_pos = header_size as u64;
        for _ in 0..num_vlrs {
            if vlr_pos + 54 > offset_to_point_data {
                break;
            }
            file.seek(SeekFrom::Start(vlr_pos))?;
            let mut vlr_header = [0u8; 54];
            if file.read_exact(&mut vlr_header).is_err() {
                break;
            }
            let user_id = trim_ascii(&vlr_header[2..18]);
            let record_id = u16::from_le_bytes(vlr_header[18..20].try_into().unwrap());
            let record_length = u16::from_le_bytes(vlr_header[20..22].try_into().unwrap()) as usize;
            let mut payload = vec![0u8; record_length];
            if file.read_exact(&mut payload).is_err() {
                break;
            }
            if user_id == LASZIP_USER_ID || record_id == LASZIP_RECORD_ID {
                is_laz = true;
                laz_vlr_payload = Some(payload);
            }
            vlr_pos += 54 + record_length as u64;
        }
    }

    Ok(LasHeader {
        file_signature: *LASF_SIGNATURE,
        file_source_id,
        global_encoding,
        project_id,
        version_major,
        version_minor,
        system_identifier,
        generating_software,
        file_creation_day,
        file_creation_year,
        header_size,
        offset_to_point_data,
        num_vlrs,
        point_data_format_id,
        point_data_record_length,
        num_point_records,
        num_points_by_return,
        x_scale,
        y_scale,
        z_scale,
        x_offset,
        y_offset,
        z_offset,
        max_x,
        min_x,
        max_y,
        min_y,
        max_z,
        min_z,
        is_laz,
        laz_vlr_payload,
    })
}

/// Read up to `buf.len()` bytes from `file`, returning the number of bytes
/// actually read (which may be less than `buf.len()` at EOF).
fn read_exact_or_short(file: &mut File, buf: &mut [u8]) -> Result<usize, LasError> {
    let mut filled = 0usize;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(LasError::Io(e)),
        }
    }
    Ok(filled)
}

/// Decode an ASCII byte slice into a `String`, trimming trailing NULs and
/// whitespace.
fn trim_ascii(bytes: &[u8]) -> String {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == 0 || bytes[end - 1].is_ascii_whitespace()) {
        end -= 1;
    }
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Read up to `max_points` point records from a LAS/LAZ file and return
/// their (x, y, z) coordinates in geographic metres.
///
/// **Special case:** `max_points == 0` means "read all points" (no
/// limit). This matches the convention used by every caller in the
/// IPC layer (`classify_ground`, `slice_by_polygon`, `run_eom_pipeline`,
/// the watch folder). The previous implementation did
/// `header.num_point_records.min(0)` which returned 0 — silently
/// reading zero points and causing "empty point cloud" errors
/// everywhere `max_points` defaulted to 0.
pub fn read_points(path: &Path, max_points: u64) -> Result<Vec<(f64, f64, f64)>, LasError> {
    let header = read_header(path)?;
    if header.point_data_record_length < 12 {
        return Err(LasError::PointRecordTooShort(
            header.point_data_record_length,
        ));
    }
    // 0 means "all" — every caller in the IPC layer relies on this.
    let count = if max_points == 0 {
        header.num_point_records
    } else {
        header.num_point_records.min(max_points)
    };

    // Security: clamp the allocation against malicious headers. A 100-byte
    // LAS file with point_count = u64::MAX in the header would cause
    // Vec::with_capacity(usize::MAX) → instant OOM/abort. We cap the
    // allocation at what the file could actually contain: file_size /
    // record_length. For a legitimate 100M-point LAS (1.2 GB), this
    // still allows the full read; for a malicious 100-byte file claiming
    // 18 quintillion points, it caps at ~8 points.
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let max_feasible = if header.point_data_record_length > 0 {
        file_size / header.point_data_record_length as u64
    } else {
        0
    };
    let safe_count = count.min(max_feasible);

    let mut points = Vec::with_capacity(safe_count as usize);
    if safe_count == 0 {
        return Ok(points);
    }

    let point_size = header.point_data_record_length as usize;
    let mut buf = vec![0u8; point_size];

    if header.is_laz {
        let payload = header
            .laz_vlr_payload
            .clone()
            .ok_or(LasError::MissingLazVlr)?;
        let vlr = laz::LazVlr::from_buffer(&payload)
            .map_err(|e| LasError::LazDecompression(e.to_string()))?;
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(header.offset_to_point_data))?;
        let mut decompressor = laz::LasZipDecompressor::new(file, vlr)
            .map_err(|e| LasError::LazDecompression(e.to_string()))?;
        for _ in 0..count {
            decompressor
                .decompress_one(&mut buf)
                .map_err(|e| LasError::LazDecompression(e.to_string()))?;
            points.push(decode_xyz(&buf, &header));
        }
    } else {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(header.offset_to_point_data))?;
        for _ in 0..count {
            file.read_exact(&mut buf)?;
            points.push(decode_xyz(&buf, &header));
        }
    }

    Ok(points)
}

/// Decode the (x, y, z) coordinates from a single point record buffer.
fn decode_xyz(buf: &[u8], header: &LasHeader) -> (f64, f64, f64) {
    let x_raw = i32::from_le_bytes(buf[0..4].try_into().unwrap());
    let y_raw = i32::from_le_bytes(buf[4..8].try_into().unwrap());
    let z_raw = i32::from_le_bytes(buf[8..12].try_into().unwrap());
    let x = header.x_offset + x_raw as f64 * header.x_scale;
    let y = header.y_offset + y_raw as f64 * header.y_scale;
    let z = header.z_offset + z_raw as f64 * header.z_scale;
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build an in-memory LAS 1.4 file (header + 3 trivial point records)
    /// and write it to `path`.
    fn write_minimal_las(path: &Path) {
        let mut f = File::create(path).unwrap();
        let mut buf = vec![0u8; 375];

        // Signature
        buf[0..4].copy_from_slice(b"LASF");
        // Version 1.4
        buf[24] = 1;
        buf[25] = 4;
        // Header size = 375
        buf[94..96].copy_from_slice(&375u16.to_le_bytes());
        // Offset to point data = 375 (no VLRs)
        buf[96..100].copy_from_slice(&375u32.to_le_bytes());
        // num VLRs = 0
        buf[100..104].copy_from_slice(&0u32.to_le_bytes());
        // Point format 0, record length 20
        buf[104] = 0;
        buf[105..107].copy_from_slice(&20u16.to_le_bytes());
        // Number of point records = 3
        buf[107..111].copy_from_slice(&3u32.to_le_bytes());
        // Scales
        buf[131..139].copy_from_slice(&0.01f64.to_le_bytes());
        buf[139..147].copy_from_slice(&0.01f64.to_le_bytes());
        buf[147..155].copy_from_slice(&0.01f64.to_le_bytes());
        // Offsets
        buf[155..163].copy_from_slice(&500_000.0f64.to_le_bytes());
        buf[163..171].copy_from_slice(&4_000_000.0f64.to_le_bytes());
        buf[171..179].copy_from_slice(&100.0f64.to_le_bytes());
        // Bounds (max_x, min_x, max_y, min_y, max_z, min_z)
        buf[179..187].copy_from_slice(&500_010.0f64.to_le_bytes());
        buf[187..195].copy_from_slice(&500_000.0f64.to_le_bytes());
        buf[195..203].copy_from_slice(&4_000_010.0f64.to_le_bytes());
        buf[203..211].copy_from_slice(&4_000_000.0f64.to_le_bytes());
        buf[211..219].copy_from_slice(&110.0f64.to_le_bytes());
        buf[219..227].copy_from_slice(&100.0f64.to_le_bytes());

        f.write_all(&buf).unwrap();

        // Write 3 point records (only X/Y/Z matter; remaining 8 bytes are zeros).
        for i in 0..3i32 {
            let mut rec = [0u8; 20];
            rec[0..4].copy_from_slice(&(i * 100).to_le_bytes()); // x
            rec[4..8].copy_from_slice(&(i * 100).to_le_bytes()); // y
            rec[8..12].copy_from_slice(&(i * 100).to_le_bytes()); // z
            f.write_all(&rec).unwrap();
        }
    }

    #[test]
    fn test_read_header_parses_minimal_las_1_4() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_minimal_las(tmp.path());
        let header = read_header(tmp.path()).unwrap();
        assert_eq!(header.version_major, 1);
        assert_eq!(header.version_minor, 4);
        assert_eq!(header.num_point_records, 3);
        assert_eq!(header.point_data_format_id, 0);
        assert_eq!(header.point_data_record_length, 20);
        assert!(!header.is_laz);
        assert!(header.laz_vlr_payload.is_none());
    }

    #[test]
    fn test_read_points_returns_decoded_coordinates() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_minimal_las(tmp.path());
        let points = read_points(tmp.path(), 1000).unwrap();
        assert_eq!(points.len(), 3);
        // x = 500_000 + i*100*0.01 = 500_000 + i
        assert!((points[0].0 - 500_000.0).abs() < 1e-6);
        assert!((points[1].0 - 500_001.0).abs() < 1e-6);
        assert!((points[2].0 - 500_002.0).abs() < 1e-6);
        // z = 100 + i
        assert!((points[0].2 - 100.0).abs() < 1e-6);
        assert!((points[2].2 - 102.0).abs() < 1e-6);
    }

    #[test]
    fn test_invalid_signature_errors() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut payload: Vec<u8> = b"NOPE".to_vec();
        payload.extend_from_slice(&[0u8; 400]);
        std::fs::write(tmp.path(), &payload).unwrap();
        let result = read_header(tmp.path());
        assert!(matches!(result, Err(LasError::InvalidSignature)));
    }

    #[test]
    fn test_short_file_errors() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut payload: Vec<u8> = b"LASF".to_vec();
        payload.extend_from_slice(&[0u8; 50]);
        std::fs::write(tmp.path(), &payload).unwrap();
        let result = read_header(tmp.path());
        assert!(matches!(result, Err(LasError::HeaderTooShort(_))));
    }

    #[test]
    fn test_max_points_limits_output() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_minimal_las(tmp.path());
        let points = read_points(tmp.path(), 2).unwrap();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_max_points_zero_means_all() {
        // Regression test: max_points=0 used to silently read 0 points
        // (header.num_point_records.min(0) = 0). Now it means "all".
        // This is the convention every IPC caller relies on — if it
        // regresses, the EOM watch folder, slice editor, and CSF
        // classifier all break with "empty point cloud" errors.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_minimal_las(tmp.path());
        let header = read_header(tmp.path()).unwrap();
        let points = read_points(tmp.path(), 0).unwrap();
        assert_eq!(
            points.len(),
            header.num_point_records as usize,
            "max_points=0 must read ALL points, not zero"
        );
    }

    #[test]
    fn test_header_serde_round_trip() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        write_minimal_las(tmp.path());
        let header = read_header(tmp.path()).unwrap();
        let json = serde_json::to_string(&header).unwrap();
        // The serde-skipped `laz_vlr_payload` must not appear in the JSON.
        assert!(!json.contains("laz_vlr_payload"));
        let back: LasHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version_minor, header.version_minor);
        assert_eq!(back.num_point_records, header.num_point_records);
    }
}
