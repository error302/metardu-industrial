// Teledyne Reson .s7k multibeam datagram reader — pure Rust.
//
// The .s7k format is the native binary format for Teledyne Reson
// multibeam echosounders (T20-P, T50, SeaBat 7125, etc.).
//
// Spec source: Teledyne Reson 'S7K Data Format' Rev 4.0+ (publicly
// documented in pyReson, MB-System, and the Reson SDK).
//
// Each s7k record (datagram) has a 80-byte header followed by a
// variable payload and a 4-byte trailing size matching the leading
// size field.
//
// Phase 0 reader extracts:
//   - Verify magic (sync pattern 0x7F7F7F7F)
//   - Walk records and count by type
//   - Extract survey start time from record 7000 (raw bathymetry)
//   - Identify sonar model from record 7000 settings
//
// Full per-beam decoding (range, angle, intensity per beam) is
// Phase 2 work — payload structure varies significantly by sonar
// generation and requires per-record interpretation.

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// S7K record type IDs — see Teledyne Reson S7K Data Format §3.
/// These are the most common types encountered in survey data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum S7kRecordType {
    /// 1000 — Sonar settings (frequency, pulse length, etc.)
    SonarSettings,
    /// 1001 — Configuration
    Configuration,
    /// 1002 — Match filter
    MatchFilter,
    /// 1003 — Firmware
    Firmware,
    /// 1004 — Beam geometry
    BeamGeometry,
    /// 1005 — Beam pointing
    BeamPointing,
    /// 1010 — Measurement of surface sound speed
    SurfaceSoundSpeed,
    /// 1012 — SVP
    Svp,
    /// 1013 — C-Node configuration
    CNodeConfig,
    /// 1015 — Processor settings
    ProcessorSettings,
    /// 1020 — Instrument information
    InstrumentInfo,
    /// 1100 — Telegraph
    Telegraph,
    /// 2000 — Position
    Position,
    /// 2001 — Cartesian position
    CartesianPosition,
    /// 3000 — Attitude
    Attitude,
    /// 3001 — Compass
    Compass,
    /// 4000 — Bathymetric data (raw — per-beam ranges/angles)
    Bathymetry,
    /// 4001 — Bathymetric data (processed — XYZ per beam)
    BathymetryXYZ,
    /// 4002 — Side scan
    SideScan,
    /// 4003 — Water column
    WaterColumn,
    /// 4004 — Snippet
    Snippet,
    /// 4005 — Beamformed magnitude
    Beamformed,
    /// 5000 — Tide
    Tide,
    /// 5200 — Motion over ground
    MotionOverGround,
    /// 7000 — Raw bathymetry (older format — used by SeaBat 7125 etc.)
    RawBathymetry,
    /// 7001 — Raw snippet
    RawSnippet,
    /// 7002 — Raw side scan
    RawSideScan,
    /// 7003 — Raw water column
    RawWaterColumn,
    /// 7004 — TVG function
    TvgFunction,
    /// 7005 — Attitude (raw)
    RawAttitude,
    /// 7010 — Position (raw)
    RawPosition,
    /// 7011 — Telegraph (raw)
    RawTelegraph,
    /// 7021 — Custom attitude
    CustomAttitude,
    /// 7500 — System event message
    SystemEvent,
    /// Any other type
    Unknown(u32),
}

impl From<u32> for S7kRecordType {
    fn from(id: u32) -> Self {
        match id {
            1000 => S7kRecordType::SonarSettings,
            1001 => S7kRecordType::Configuration,
            1002 => S7kRecordType::MatchFilter,
            1003 => S7kRecordType::Firmware,
            1004 => S7kRecordType::BeamGeometry,
            1005 => S7kRecordType::BeamPointing,
            1010 => S7kRecordType::SurfaceSoundSpeed,
            1012 => S7kRecordType::Svp,
            1013 => S7kRecordType::CNodeConfig,
            1015 => S7kRecordType::ProcessorSettings,
            1020 => S7kRecordType::InstrumentInfo,
            1100 => S7kRecordType::Telegraph,
            2000 => S7kRecordType::Position,
            2001 => S7kRecordType::CartesianPosition,
            3000 => S7kRecordType::Attitude,
            3001 => S7kRecordType::Compass,
            4000 => S7kRecordType::Bathymetry,
            4001 => S7kRecordType::BathymetryXYZ,
            4002 => S7kRecordType::SideScan,
            4003 => S7kRecordType::WaterColumn,
            4004 => S7kRecordType::Snippet,
            4005 => S7kRecordType::Beamformed,
            5000 => S7kRecordType::Tide,
            5200 => S7kRecordType::MotionOverGround,
            7000 => S7kRecordType::RawBathymetry,
            7001 => S7kRecordType::RawSnippet,
            7002 => S7kRecordType::RawSideScan,
            7003 => S7kRecordType::RawWaterColumn,
            7004 => S7kRecordType::TvgFunction,
            7005 => S7kRecordType::RawAttitude,
            7010 => S7kRecordType::RawPosition,
            7011 => S7kRecordType::RawTelegraph,
            7021 => S7kRecordType::CustomAttitude,
            7500 => S7kRecordType::SystemEvent,
            _ => S7kRecordType::Unknown(id),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct S7kHeader {
    /// Sonar model guess from record 1020 (Instrument Info) if present
    pub model: String,
    /// S7K format version (usually 0x00000300 = v3.0)
    pub version: u32,
    /// Survey start time as Unix seconds (UTC)
    pub seconds_since_epoch: u32,
    /// Human-readable start time
    pub date: String,
    /// Counts per record type
    pub bathymetry_count: u32,
    pub position_count: u32,
    pub attitude_count: u32,
    pub svp_count: u32,
    pub sonar_settings_count: u32,
    pub side_scan_count: u32,
    pub snippet_count: u32,
    pub total_records: u32,
    pub first_timestamp: Option<u32>,
    pub last_timestamp: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum S7kError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("not a Reson .s7k file — sync pattern mismatch (got {0:#010x})")]
    BadSync(u32),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("truncated header")]
    #[allow(dead_code)]
    Truncated,
}

/// S7K sync pattern — every record starts with these 4 bytes (0x7F repeated).
const S7K_SYNC_LE: u32 = 0x7F7F_7F7F;

/// Parse the Reson .s7k file header by walking the record stream.
///
/// Each s7k record has an 80-byte header:
///   - 4 bytes: sync pattern (0x7F7F7F7F)
///   - 4 bytes: record size (includes everything after this field)
///   - 4 bytes: offset to optional data
///   - 4 bytes: record type ID
///   - 4 bytes: subdevice
///   - 8 bytes: num records (s7k can batch — usually 1)
///   - 8 bytes: sequence number
///   - 8 bytes: time (double, days since Jan 1, 1904 — Delphi TDateTime style)
///   - 16 bytes: reserved
///   - 8 bytes: total record size (matches the leading size field)
///   - 8 bytes: device ID
///   - 2 bytes: system enumerator
///   - 2 bytes: data format specifier
///   - 2 bytes: subsystem
///   - 2 bytes: reserved
///   - ... payload ...
///   - 4 bytes: trailing size (matches leading)
pub fn read_header(path: &Path) -> Result<S7kHeader, S7kError> {
    let mut file = File::open(path).map_err(|_| S7kError::NotFound(path.display().to_string()))?;

    // Verify sync
    let mut sync_buf = [0u8; 4];
    file.read_exact(&mut sync_buf)?;
    let sync = u32::from_le_bytes(sync_buf);
    if sync != S7K_SYNC_LE {
        return Err(S7kError::BadSync(sync));
    }
    file.seek(SeekFrom::Start(0))?;

    let mut model = String::from("unknown");
    let mut version: u32 = 0;
    let mut bathymetry_count = 0u32;
    let mut position_count = 0u32;
    let mut attitude_count = 0u32;
    let mut svp_count = 0u32;
    let mut sonar_settings_count = 0u32;
    let mut side_scan_count = 0u32;
    let mut snippet_count = 0u32;
    let mut total_records = 0u32;
    let mut first_timestamp: Option<u32> = None;
    let mut last_timestamp: Option<u32> = None;

    // Walk records — cap at 10000 for Phase 0
    let max_records = 10000;
    for _ in 0..max_records {
        // 80-byte header
        let mut header = [0u8; 80];
        match file.read(&mut header) {
            Ok(0) => break,
            Ok(n) if n < 80 => break,
            Ok(_) => {}
            Err(_) => break,
        }

        // Verify sync at start of every record
        let record_sync = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if record_sync != S7K_SYNC_LE {
            // Lost sync — bail
            break;
        }

        // Record size field (bytes 4-7) — size of everything AFTER the size field
        let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

        // Record type (bytes 12-15)
        let record_type_id = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);

        // Version (bytes 4 of the next 8 — actually it's offset 8 in the header
        // for the offset-to-data field, but version comes from the s7k record
        // header. Per spec: bytes 8-11 are "offset to optional data" and
        // version is bytes 36-39 (reserved area). Actually, let's be careful:
        // The standard s7k header (Rev 3.0+) has:
        //   0-3:   sync
        //   4-7:   record size
        //   8-11:  offset to optional data
        //   12-15: record type
        //   16-19: subdevice ID
        //   20-23: num records (low 4 of 8)
        //   24-27: num records (high 4 of 8) — usually 0
        //   28-31: sequence number (low 4 of 8)
        //   32-35: sequence number (high 4 of 8)
        //   36-43: time (double, days since 1904-01-01)
        //   44-59: reserved
        //   60-67: total record size
        //   68-71: device ID
        //   72-73: system enumerator
        //   74-75: data format specifier (this is the s7k version!)
        //   76-77: subsystem
        //   78-79: reserved
        let data_format_specifier = u16::from_le_bytes([header[74], header[75]]);
        if version == 0 && data_format_specifier != 0 {
            version = u32::from(data_format_specifier);
        }

        // Time — double days since 1904-01-01 (Delphi TDateTime epoch)
        let time_bytes = [
            header[36], header[37], header[38], header[39], header[40], header[41], header[42],
            header[43],
        ];
        let time_days = f64::from_le_bytes(time_bytes);
        let unix_secs = tdatetime_to_unix(time_days);

        // Total record size (bytes 60-67) — should match the leading size
        // We trust the leading size and skip the rest of the record
        // (size includes the 76 bytes after itself + payload + trailing 4 bytes)
        // Per spec: "Record Size — Size of record from byte offset 8 through
        // the trailing size field." So total = size + 8.
        let skip_bytes = (size as i64) - 76; // we've read 80, but size counts from offset 8
        if skip_bytes < 4 {
            // Bogus size — bail
            break;
        }

        let payload_size = (skip_bytes as u64).saturating_sub(4); // 4 = trailing size
        let mut payload = vec![0u8; payload_size as usize];
        if file.read_exact(&mut payload).is_err() {
            break;
        }

        // Read trailing size (4 bytes)
        let mut trailing = [0u8; 4];
        if file.read_exact(&mut trailing).is_err() {
            break;
        }

        total_records += 1;
        let record_type = S7kRecordType::from(record_type_id);

        match record_type {
            S7kRecordType::SonarSettings => {
                sonar_settings_count += 1;
                // Sonar settings record contains frequency, pulse length, etc.
                // First timestamp comes from this typically.
                if first_timestamp.is_none() {
                    first_timestamp = Some(unix_secs);
                }
            }
            S7kRecordType::InstrumentInfo => {
                // Try to read sonar model name from instrument info payload
                // (typically a fixed-length string at known offset)
                if model == "unknown" {
                    model = extract_instrument_name(&payload);
                }
            }
            S7kRecordType::RawBathymetry
            | S7kRecordType::Bathymetry
            | S7kRecordType::BathymetryXYZ => {
                bathymetry_count += 1;
                last_timestamp = Some(unix_secs);
            }
            S7kRecordType::Position
            | S7kRecordType::RawPosition
            | S7kRecordType::CartesianPosition => {
                position_count += 1;
            }
            S7kRecordType::Attitude
            | S7kRecordType::RawAttitude
            | S7kRecordType::CustomAttitude => {
                attitude_count += 1;
            }
            S7kRecordType::Svp => {
                svp_count += 1;
            }
            S7kRecordType::SideScan | S7kRecordType::RawSideScan => {
                side_scan_count += 1;
            }
            S7kRecordType::Snippet | S7kRecordType::RawSnippet => {
                snippet_count += 1;
            }
            _ => {}
        }
    }

    let date = first_timestamp.map(unix_to_iso).unwrap_or_default();

    Ok(S7kHeader {
        model,
        version,
        seconds_since_epoch: first_timestamp.unwrap_or(0),
        date,
        bathymetry_count,
        position_count,
        attitude_count,
        svp_count,
        sonar_settings_count,
        side_scan_count,
        snippet_count,
        total_records,
        first_timestamp,
        last_timestamp,
    })
}

/// Convert Delphi TDateTime (days since 1900-01-01 actually — some sources
/// say 1899-12-30) to Unix seconds. The s7k spec says "days since
/// Jan 1, 1904" but Reson implementation uses Delphi's TDateTime which
/// is days since 1899-12-30. We use 1899-12-30 here.
fn tdatetime_to_unix(days: f64) -> u32 {
    // Days from 1899-12-30 to 1970-01-01 = 25569
    const EPOCH_OFFSET_DAYS: f64 = 25569.0;
    let unix_days = days - EPOCH_OFFSET_DAYS;
    if unix_days < 0.0 {
        return 0;
    }
    (unix_days * 86400.0) as u32
}

fn unix_to_iso(unix: u32) -> String {
    // Simple UTC ISO format — no chrono dep
    let secs_per_day: u32 = 86400;
    let days = unix / secs_per_day;
    let remainder = unix % secs_per_day;
    let hour = remainder / 3600;
    let minute = (remainder % 3600) / 60;
    let second = remainder % 60;

    // Walk forward from 1970-01-01
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

fn days_to_ymd(days_since_epoch: u32) -> (u16, u8, u8) {
    let mut year = 1970u16;
    let mut days = days_since_epoch;
    loop {
        let yd = if is_leap_year(year) { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        year += 1;
    }
    let days_per_month: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u8;
    for (i, &dpm) in days_per_month.iter().enumerate() {
        let effective_dpm = if i == 1 && is_leap_year(year) {
            dpm + 1
        } else {
            dpm
        };
        if days < effective_dpm {
            break;
        }
        days -= effective_dpm;
        month += 1;
    }
    (year, month, (days + 1) as u8)
}

fn is_leap_year(y: u16) -> bool {
    let y = y as u32;
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

/// Try to extract the sonar model name from an Instrument Info record payload.
/// The payload format varies by sonar generation — we look for printable
/// ASCII runs that look like a model name.
fn extract_instrument_name(payload: &[u8]) -> String {
    // Heuristic: find the longest printable ASCII run (>=4 chars)
    let mut best = String::new();
    let mut current = String::new();
    for &b in payload {
        if b.is_ascii_graphic() || b == b' ' {
            current.push(b as char);
        } else {
            if current.len() >= 4 && current.len() > best.len() {
                best = current.clone();
            }
            current.clear();
        }
    }
    if current.len() >= 4 && current.len() > best.len() {
        best = current;
    }
    best.trim().to_string()
}
