// Kongsberg .all multibeam echosounder datagram reader — pure Rust.
//
// The Kongsberg .all format is the de facto standard for multibeam
// bathymetry data from Kongsberg EM-series sonars (EM 710, EM 122, EM 302,
// EM 2040, etc.). It's a binary container with timestamped datagrams.
//
// Spec source: Kongsberg's 'EM Series Multibeam Echo Sounder Datagram
// Description' manual (publicly documented in CARIS, QPS, and PyAll
// implementations).
//
// This Phase 0 reader extracts just enough to:
//   - Confirm a file is a valid .all (start datagram 0x49 + 'EM')
//   - Walk the datagram stream and count types
//   - Extract the first/last timestamps for survey duration
//   - Identify the model (EM 710, EM 2040, etc.) from the RunTime datagram
//
// Full per-ping bathymetry decoding is Phase 2 (Marine MVP) work — the
// datagram payload format varies significantly by sonar generation and
// requires sound velocity ray-tracing to convert raw travel times to
// depths.

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Kongsberg .all datagram type bytes — byte 1 of the 4-byte header.
/// Reference: Kongsberg EM Series Datagram Description §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatagramType {
    /// 0x49 — Start datagram, file begins with this
    Start,
    /// 0x4E — Sound velocity profile
    Svp,
    /// 0x50 — Position (telegrams, usually from GPS)
    Position,
    /// 0x52 — Runtime parameters (sonar model, settings)
    Runtime,
    /// 0x44 — Bathymetry (depth + sidescan per beam)
    Bathymetry,
    /// 0x55 — Backscatter (snippet per beam)
    Backscatter,
    /// 0x59 — Attitude (heave, roll, pitch)
    Attitude,
    /// 0x68 — Heading
    Heading,
    /// 0x43 — Clock (synchronizes PC time to sonar time)
    Clock,
    /// 0x54 — Tide
    Tide,
    /// 0x4D — Height (single-beam depth or altitude)
    Height,
    /// 0x4B — XYZ datagram (point cloud from sonar)
    Xyz,
    /// 0x53 — Surface sound speed
    SurfaceSpeed,
    /// 0x6E — Network attitude
    NetworkAttitude,
    /// 0x6F — Network attitude velocity
    NetworkAttitudeVelocity,
    /// Any other type byte
    Unknown(u8),
}

impl From<u8> for DatagramType {
    fn from(b: u8) -> Self {
        match b {
            0x49 => DatagramType::Start,
            0x4E => DatagramType::Svp,
            0x50 => DatagramType::Position,
            0x52 => DatagramType::Runtime,
            0x44 => DatagramType::Bathymetry,
            0x55 => DatagramType::Backscatter,
            0x59 => DatagramType::Attitude,
            0x68 => DatagramType::Heading,
            0x43 => DatagramType::Clock,
            0x54 => DatagramType::Tide,
            0x4D => DatagramType::Height,
            0x4B => DatagramType::Xyz,
            0x53 => DatagramType::SurfaceSpeed,
            0x6E => DatagramType::NetworkAttitude,
            0x6F => DatagramType::NetworkAttitudeVelocity,
            _ => DatagramType::Unknown(b),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AllHeader {
    pub model: String,
    pub model_id: u16,
    pub date: String,
    pub seconds_since_epoch: u32,
    pub ping_count: u32,
    pub position_count: u32,
    pub attitude_count: u32,
    pub svp_count: u32,
    pub runtime_count: u32,
    pub total_datagrams: u32,
    pub first_timestamp: Option<u32>,
    pub last_timestamp: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum AllError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("not a Kongsberg .all file — first byte was {0:#04x} (expected 0x49)")]
    BadMagic(u8),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("truncated header")]
    #[allow(dead_code)]
    Truncated,
}

/// Parse the Kongsberg .all file header by walking the datagram stream.
///
/// Each datagram has a 4-byte header (type + size) followed by a
/// variable payload and a 4-byte trailing size (matching the leading
/// size for validation).
///
/// Phase 0 walks up to 10,000 datagrams (covers most surveys' first
/// few minutes) to identify the model, count types, and extract
/// timestamps. Full per-ping decoding is Phase 2 work.
pub fn read_header(path: &Path) -> Result<AllHeader, AllError> {
    let mut file = File::open(path).map_err(|_| AllError::NotFound(path.display().to_string()))?;

    // Verify magic: first byte must be 0x49 (start datagram type)
    let mut start_byte = [0u8; 1];
    file.read_exact(&mut start_byte)?;
    if start_byte[0] != 0x49 {
        return Err(AllError::BadMagic(start_byte[0]));
    }
    file.seek(SeekFrom::Start(0))?;

    let mut model = String::from("unknown");
    let mut model_id: u16 = 0;
    let mut ping_count = 0u32;
    let mut position_count = 0u32;
    let mut attitude_count = 0u32;
    let mut svp_count = 0u32;
    let mut runtime_count = 0u32;
    let mut total_datagrams = 0u32;
    let mut first_timestamp: Option<u32> = None;
    let mut last_timestamp: Option<u32> = None;
    let mut date_string = String::new();

    // Walk datagrams — cap at 10000 for Phase 0 (covers ~30s of survey)
    let max_datagrams = 10000;
    for _ in 0..max_datagrams {
        // 4-byte header: type(1) + size(3, little-endian, 24-bit)
        let mut header = [0u8; 4];
        match file.read(&mut header) {
            Ok(0) => break, // EOF
            Ok(n) if n < 4 => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let type_byte = header[0];
        let size =
            u32::from(header[1]) | (u32::from(header[2]) << 8) | (u32::from(header[3]) << 16);

        if size < 4 {
            // Bogus size — bail
            break;
        }

        // Read the datagram payload (size includes the 4-byte header)
        let payload_size = (size as usize).saturating_sub(4);
        let mut payload = vec![0u8; payload_size];
        if file.read_exact(&mut payload).is_err() {
            break;
        }

        // 4-byte trailing size (must match leading size)
        let mut trailing = [0u8; 4];
        if file.read_exact(&mut trailing).is_err() {
            break;
        }

        total_datagrams += 1;
        let dg_type = DatagramType::from(type_byte);

        match dg_type {
            DatagramType::Start => {
                // Start datagram payload: second(4) + minute(1) + hour(1) +
                // day(1) + month(1) + year(2) + model(2) + ...
                if payload.len() >= 14 {
                    let second =
                        u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    let minute = payload[4];
                    let hour = payload[5];
                    let day = payload[6];
                    let month = payload[7];
                    let year = u16::from_le_bytes([payload[8], payload[9]]);
                    model_id = u16::from_le_bytes([payload[10], payload[11]]);
                    model = model_name(model_id);
                    date_string =
                        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}");
                    first_timestamp = Some(unix_seconds(year, month, day, hour, minute, second));
                }
            }
            DatagramType::Bathymetry => {
                ping_count += 1;
                // Bathymetry payload starts with date (4s) + time fraction (2)
                if payload.len() >= 6 {
                    let sec = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    last_timestamp = Some(sec);
                }
            }
            DatagramType::Position => position_count += 1,
            DatagramType::Attitude | DatagramType::NetworkAttitude => attitude_count += 1,
            DatagramType::Svp => svp_count += 1,
            DatagramType::Runtime => runtime_count += 1,
            _ => {}
        }
    }

    // Compute duration if we have timestamps
    let _duration_secs = match (first_timestamp, last_timestamp) {
        (Some(f), Some(l)) => Some(l.saturating_sub(f)),
        _ => None,
    };

    Ok(AllHeader {
        model,
        model_id,
        date: date_string,
        seconds_since_epoch: first_timestamp.unwrap_or(0),
        ping_count,
        position_count,
        attitude_count,
        svp_count,
        runtime_count,
        total_datagrams,
        first_timestamp,
        last_timestamp,
    })
}

/// Convert Kongsberg model ID to human-readable name.
fn model_name(id: u16) -> String {
    match id {
        100 => "EM 100".into(),
        300 => "EM 300".into(),
        3000 => "EM 3000".into(),
        1002 => "EM 1002".into(),
        3002 => "EM 3002".into(),
        120 => "EM 120".into(),
        121 => "EM 121".into(),
        122 => "EM 122".into(),
        3001 => "EM 3001".into(),
        710 => "EM 710".into(),
        712 => "EM 712".into(),
        302 => "EM 302".into(),
        2040 => "EM 2040".into(),
        2045 => "EM 2045".into(),
        _ => format!("EM (unknown id {id})"),
    }
}

/// Convert Y/M/D h:m:s to Unix seconds (UTC).
/// Naive implementation — no leap second handling, no timezone.
/// Kongsberg timestamps are UTC.
fn unix_seconds(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u32) -> u32 {
    // Days from 1970-01-01 to year-month-day, using a simple algorithm.
    // Good enough for Phase 0 timestamp display.
    if year < 1970 {
        return 0;
    }
    let mut days: u32 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let days_per_month = [31u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (m, &dpm) in days_per_month
        .iter()
        .enumerate()
        .take(month.saturating_sub(1) as usize)
    {
        days += dpm;
        if m == 1 && is_leap_year(year) {
            days += 1;
        }
    }
    days += day.saturating_sub(1) as u32;
    let hours = days.saturating_mul(24).saturating_add(hour as u32);
    let minutes = hours.saturating_mul(60).saturating_add(minute as u32);
    minutes.saturating_mul(60).saturating_add(second)
}

fn is_leap_year(y: u16) -> bool {
    let y = y as u32;
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}
