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
    /// 0x57 — Water column (WC) datagram — raw water column samples per beam
    /// Used for object detection (fish schools, wrecks, gas plumes)
    WaterColumn,
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
            0x57 => DatagramType::WaterColumn,
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

// ──────────────────────────────────────────────────────────────────
// Phase 2: Full bathymetry datagram parsing
// ──────────────────────────────────────────────────────────────────

/// A single sounding from a Kongsberg .all bathymetry datagram.
#[derive(Debug, Clone, Serialize)]
pub struct KongsbergSounding {
    /// Unix timestamp (seconds)
    pub timestamp: f64,
    /// Ping number within the file
    pub ping_number: u32,
    /// Beam number (0 = portmost, increases to starboard)
    pub beam_number: u16,
    /// Depth below transducer (meters, positive down)
    pub depth: f64,
    /// Across-track distance from nadir (meters, positive starboard)
    pub across_track: f64,
    /// Along-track distance from transducer (meters, positive forward)
    pub along_track: f64,
    /// Beam pointing angle (degrees, 0 = nadir)
    pub beam_angle: f64,
    /// Heave at transducer (meters, positive up)
    pub heave: f64,
    /// Roll at ping time (degrees, positive starboard down)
    pub roll: f64,
    /// Pitch at ping time (degrees, positive bow up)
    pub pitch: f64,
    /// Heading at ping time (degrees)
    pub heading: f64,
    /// Sound speed at transducer (m/s)
    pub sound_speed: f64,
    /// Quality flag (0=good, higher=poorer)
    pub quality: u8,
    /// Detection info (0=normal, 1=phase detection, etc.)
    pub detection_info: u8,
}

/// Position record from a .all position datagram.
#[derive(Debug, Clone, Serialize)]
pub struct KongsbergPosition {
    /// Unix timestamp (seconds)
    pub timestamp: f64,
    /// Latitude (decimal degrees, WGS84)
    pub latitude: f64,
    /// Longitude (decimal degrees, WGS84)
    pub longitude: f64,
    /// Ellipsoidal height (meters)
    pub height: f64,
    /// Position quality flag
    pub quality: u8,
}

/// Attitude record from a .all attitude datagram.
#[derive(Debug, Clone, Serialize)]
pub struct KongsbergAttitude {
    /// Unix timestamp (seconds)
    pub timestamp: f64,
    /// Roll (degrees, positive starboard down)
    pub roll: f64,
    /// Pitch (degrees, positive bow up)
    pub pitch: f64,
    /// Heave (meters, positive up)
    pub heave: f64,
    /// Heading (degrees)
    pub heading: f64,
}

/// Full survey data extracted from a .all file.
#[derive(Debug, Clone, Serialize)]
pub struct AllSurveyData {
    pub header: AllHeader,
    pub soundings: Vec<KongsbergSounding>,
    pub positions: Vec<KongsbergPosition>,
    pub attitudes: Vec<KongsbergAttitude>,
    pub bounds: Option<(f64, f64, f64, f64)>, // min_lon, min_lat, max_lon, max_lat
}

/// Parse a Kongsberg .all file and extract all bathymetry, position, and
/// attitude data. This is the full Phase 2 parser — it walks every datagram
/// and decodes the payload.
///
/// `max_pings` limits the number of bathymetry pings to process (0 = all).
/// Each ping can contain 100-400+ beams, so 1000 pings = ~300K soundings.
pub fn read_all_survey(
    path: &Path,
    max_pings: u32,
) -> Result<AllSurveyData, AllError> {
    let header = read_header(path)?;
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(0))?;

    let mut soundings: Vec<KongsbergSounding> = Vec::new();
    let mut positions: Vec<KongsbergPosition> = Vec::new();
    let mut attitudes: Vec<KongsbergAttitude> = Vec::new();
    let mut ping_count = 0u32;

    // Interpolated attitude (nearest-neighbor lookup by timestamp)
    let mut attitude_buffer: Vec<KongsbergAttitude> = Vec::new();
    let mut attitude_idx = 0usize;

    // Interpolated position
    let mut position_buffer: Vec<KongsbergPosition> = Vec::new();
    let mut position_idx = 0usize;

    // Skip the start datagram
    let mut start_header = [0u8; 4];
    let _ = file.read(&mut start_header);
    if start_header[0] != 0x49 {
        file.seek(SeekFrom::Start(0))?;
    } else {
        let size = u32::from(start_header[1])
            | (u32::from(start_header[2]) << 8)
            | (u32::from(start_header[3]) << 16);
        let payload_size = (size as usize).saturating_sub(4);
        file.seek(SeekFrom::Current((payload_size + 4) as i64))?;
    }

    loop {
        let mut header_buf = [0u8; 4];
        match file.read(&mut header_buf) {
            Ok(0) => break,
            Ok(n) if n < 4 => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let type_byte = header_buf[0];
        let size = u32::from(header_buf[1])
            | (u32::from(header_buf[2]) << 8)
            | (u32::from(header_buf[3]) << 16);

        if size < 4 {
            break;
        }

        let payload_size = (size as usize).saturating_sub(4);
        let mut payload = vec![0u8; payload_size];
        if file.read_exact(&mut payload).is_err() {
            break;
        }

        // Skip trailing 4 bytes
        let mut trailing = [0u8; 4];
        if file.read_exact(&mut trailing).is_err() {
            break;
        }

        let dg_type = DatagramType::from(type_byte);

        match dg_type {
            DatagramType::Position => {
                if let Some(pos) = parse_position_datagram(&payload) {
                    positions.push(pos.clone());
                    position_buffer.push(pos);
                }
            }
            DatagramType::Attitude | DatagramType::NetworkAttitude => {
                if let Some(att) = parse_attitude_datagram(&payload, dg_type == DatagramType::NetworkAttitude) {
                    attitudes.push(att.clone());
                    attitude_buffer.push(att);
                }
            }
            DatagramType::Bathymetry => {
                if max_pings > 0 && ping_count >= max_pings {
                    continue;
                }
                ping_count += 1;

                // Find nearest attitude and position for this ping
                let ping_time = if payload.len() >= 6 {
                    u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64
                        + u16::from_le_bytes([payload[4], payload[5]]) as f64 / 1000.0
                } else {
                    continue;
                };

                let nearest_att = find_nearest(&attitude_buffer, &mut attitude_idx, ping_time);
                let nearest_pos = find_nearest(&position_buffer, &mut position_idx, ping_time);

                if let Some(pings) = parse_bathymetry_datagram(&payload, ping_count, nearest_att.as_ref(), nearest_pos.as_ref()) {
                    soundings.extend(pings);
                }
            }
            _ => {}
        }
    }

    // Compute geographic bounds from positions
    let bounds = if !positions.is_empty() {
        let (min_lon, max_lon, min_lat, max_lat) = positions.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
            |(mnx, mxx, mny, mxy), p| {
                (mnx.min(p.longitude), mxx.max(p.longitude), mny.min(p.latitude), mxy.max(p.latitude))
            },
        );
        Some((min_lon, min_lat, max_lon, max_lat))
    } else {
        None
    };

    Ok(AllSurveyData {
        header,
        soundings,
        positions,
        attitudes,
        bounds,
    })
}

/// Walk a Kongsberg .all file and extract water-column datagram summary
/// statistics without materializing every sample (a single .all file can
/// contain tens of millions of WC samples — too much to ship over IPC).
///
/// Returns a [`WaterColumnSummary`] with ping count, total samples, max
/// samples per beam, and beams per ping (averaged). Use this for the
/// real-time MBES reader UI; if the operator wants raw samples, they
/// should call [`extract_water_column_samples`] with a ping range.
pub fn extract_water_column_summary(path: &Path, max_pings: u32) -> Result<WaterColumnSummary, AllError> {
    let mut file = File::open(path)?;
    let mut start_header = [0u8; 4];
    if file.read_exact(&mut start_header).is_err() {
        return Err(AllError::Truncated);
    }
    if start_header[0] != 0x49 {
        return Err(AllError::BadMagic(start_header[0]));
    }
    // Rewind so the datagram walker sees the start datagram
    file.seek(SeekFrom::Start(0))?;

    let mut ping_count: u32 = 0;
    let mut total_samples: u64 = 0;
    let mut max_samples_per_beam: u32 = 0;
    let mut beams_total: u64 = 0;

    loop {
        // Each datagram: 4-byte length, 1-byte type, payload, 4-byte trailing length
        let mut len_buf = [0u8; 4];
        if file.read_exact(&mut len_buf).is_err() {
            break; // EOF
        }
        let dg_len = u32::from_le_bytes(len_buf) as usize;
        if dg_len < 5 || dg_len > 16 * 1024 * 1024 {
            // Sanity check — datagrams are at most a few MB
            break;
        }
        let mut type_and_payload = vec![0u8; dg_len];
        if file.read_exact(&mut type_and_payload).is_err() {
            break;
        }
        // Skip trailing 4-byte length
        let mut trailing = [0u8; 4];
        if file.read_exact(&mut trailing).is_err() {
            break;
        }

        let type_byte = type_and_payload[0];
        let payload = &type_and_payload[1..];
        let dg_type = DatagramType::from(type_byte);

        if dg_type == DatagramType::WaterColumn {
            if max_pings > 0 && ping_count >= max_pings {
                continue;
            }
            ping_count += 1;

            // Parse just the header counts — don't materialize samples
            if payload.len() >= 18 {
                let n_beams = u16::from_le_bytes([payload[14], payload[15]]) as u32;
                let samples_per_beam = u16::from_le_bytes([payload[16], payload[17]]) as u32;
                if n_beams > 0 && n_beams <= 1024 && samples_per_beam > 0 && samples_per_beam <= 10000 {
                    let ping_samples = (n_beams as u64) * (samples_per_beam as u64);
                    total_samples = total_samples.saturating_add(ping_samples);
                    if samples_per_beam > max_samples_per_beam {
                        max_samples_per_beam = samples_per_beam;
                    }
                    beams_total = beams_total.saturating_add(n_beams as u64);
                }
            }
        }
    }

    let beams_per_ping = if ping_count > 0 {
        (beams_total / ping_count as u64) as u32
    } else {
        0
    };

    Ok(WaterColumnSummary {
        ping_count,
        total_samples,
        max_samples_per_beam,
        beams_per_ping,
    })
}

/// Summary statistics for the water-column datagrams in a .all file.
///
/// Used by the MBES Survey Reader UI to show whether the file contains
/// water-column data and how much, without shipping gigabytes of
/// raw amplitude samples over IPC.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WaterColumnSummary {
    /// Number of water-column pings found
    pub ping_count: u32,
    /// Total number of samples across all pings and beams
    pub total_samples: u64,
    /// Maximum samples per beam observed in any ping
    pub max_samples_per_beam: u32,
    /// Average beams per ping
    pub beams_per_ping: u32,
}

/// Parse a Kongsberg .all position datagram (type 0x50).
///
/// Format (simplified — real format has more fields):
///   - Date (4 bytes, seconds since epoch)
///   - Time fraction (2 bytes, 1/1000s)
///   - Position system ID (2 bytes)
///   - Latitude (4 bytes, double, decimal degrees)
///   - Longitude (4 bytes, double, decimal degrees)
///   - Ellipsoidal height (4 bytes, float, meters)
///   - Quality (1 byte)
///   - ... (more fields)
fn parse_position_datagram(payload: &[u8]) -> Option<KongsbergPosition> {
    if payload.len() < 30 {
        return None;
    }

    let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64
        + u16::from_le_bytes([payload[4], payload[5]]) as f64 / 1000.0;

    // Position datagram format varies by system. The most common format
    // (EM series) stores lat/lon as doubles starting at byte 8.
    // Some systems use integer formats — we handle the double case.
    let latitude = f64::from_le_bytes([
        payload[8], payload[9], payload[10], payload[11],
        payload[12], payload[13], payload[14], payload[15],
    ]);

    let longitude = f64::from_le_bytes([
        payload[16], payload[17], payload[18], payload[19],
        payload[20], payload[21], payload[22], payload[23],
    ]);

    let height = if payload.len() >= 28 {
        f32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]) as f64
    } else {
        0.0
    };

    let quality = if payload.len() >= 29 {
        payload[28]
    } else {
        0
    };

    // Sanity check — reject obviously bad positions
    if latitude.abs() > 90.0 || longitude.abs() > 180.0 {
        return None;
    }

    Some(KongsbergPosition {
        timestamp,
        latitude,
        longitude,
        height,
        quality,
    })
}

/// Parse a Kongsberg .all attitude datagram (type 0x59 or 0x6E).
///
/// Format:
///   - Date (4 bytes, seconds since epoch)
///   - Number of entries (1 byte for 0x59, varies for 0x6E)
///   - Per-entry: delta time (2 bytes), roll (2), pitch (2), heave (2), heading (2)
///
/// All angles are stored as centi-degrees (1/100 degree).
/// Heave is stored as centi-meters (1/100 m).
fn parse_attitude_datagram(payload: &[u8], is_network: bool) -> Option<KongsbergAttitude> {
    if payload.len() < 6 {
        return None;
    }

    let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64;

    if is_network {
        // Network attitude (0x6E) — different format, typically has
        // multiple entries. We take the first.
        if payload.len() < 14 {
            return None;
        }
        let roll = i16::from_le_bytes([payload[4], payload[5]]) as f64 / 100.0;
        let pitch = i16::from_le_bytes([payload[6], payload[7]]) as f64 / 100.0;
        let heave = i16::from_le_bytes([payload[8], payload[9]]) as f64 / 100.0;
        let heading = u16::from_le_bytes([payload[10], payload[11]]) as f64 / 100.0;

        Some(KongsbergAttitude {
            timestamp: timestamp + payload[12] as f64 / 1000.0,
            roll,
            pitch,
            heave,
            heading,
        })
    } else {
        // Standard attitude (0x59)
        let n_entries = payload[4] as usize;
        if n_entries == 0 || payload.len() < 5 + n_entries * 10 {
            return None;
        }

        // Take the first entry
        let offset = 5;
        let roll = i16::from_le_bytes([payload[offset], payload[offset + 1]]) as f64 / 100.0;
        let pitch = i16::from_le_bytes([payload[offset + 2], payload[offset + 3]]) as f64 / 100.0;
        let heave = i16::from_le_bytes([payload[offset + 4], payload[offset + 5]]) as f64 / 100.0;
        let heading = u16::from_le_bytes([payload[offset + 6], payload[offset + 7]]) as f64 / 100.0;
        let delta_time = u16::from_le_bytes([payload[offset + 8], payload[offset + 9]]) as f64 / 1000.0;

        Some(KongsbergAttitude {
            timestamp: timestamp + delta_time,
            roll,
            pitch,
            heave,
            heading,
        })
    }
}

/// Parse a Kongsberg .all bathymetry datagram (type 0x44).
///
/// Format (EM series):
///   - Date (4 bytes, seconds since epoch)
///   - Time fraction (2 bytes, 1/1000s)
///   - Ping counter (2 bytes)
///   - System serial (2 bytes)
///   - Sound speed at transducer (2 bytes, m/s * 10)
///   - Transducer depth (2 bytes, m * 100)
///   - Number of valid beams (2 bytes)
///   - Number of valid pixels (2 bytes)
///   - Per-beam data (variable length depending on sample format)
fn parse_bathymetry_datagram(
    payload: &[u8],
    ping_number: u32,
    att: Option<&KongsbergAttitude>,
    _pos: Option<&KongsbergPosition>,
) -> Option<Vec<KongsbergSounding>> {
    if payload.len() < 20 {
        return None;
    }

    let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64
        + u16::from_le_bytes([payload[4], payload[5]]) as f64 / 1000.0;

    let sound_speed = u16::from_le_bytes([payload[10], payload[11]]) as f64 / 10.0;
    let n_beams = u16::from_le_bytes([payload[14], payload[15]]) as usize;

    if n_beams == 0 || n_beams > 1024 {
        // Sanity check — modern MBES have 100-880 beams
        return None;
    }

    // The per-beam data format varies. The most common format (used by
    // EM 710, EM 2040, EM 302) stores per-beam:
    //   - Beam number (2 bytes)
    //   - Depth (2 bytes, cm below transducer * 10 — or 4 bytes float for newer)
    //   - Across-track (2 bytes, cm * 10)
    //   - Along-track (2 bytes, cm * 10)
    //   - Beam angle (2 bytes, centi-degrees)
    //   - Quality (1 byte)
    //   - Detection info (1 byte)
    //
    // Total: ~12 bytes per beam for the basic format.
    // Some newer formats use 4-byte floats — we detect by checking
    // if the payload has enough bytes for the float format.

    let beam_data_offset = 16; // After the fixed header
    let remaining = payload.len().saturating_sub(beam_data_offset);

    // Try to determine the per-beam record size
    // Standard format: 12 bytes/beam (2+2+2+2+2+1+1)
    // Float format: 28 bytes/beam (2+4+4+4+2+4+4+1+1+2+2+2 — varies)
    // We'll use the standard 12-byte format as default, and fall back
    // to a larger format if the bytes don't add up.
    let record_size = if n_beams * 12 <= remaining {
        12
    } else if n_beams * 28 <= remaining {
        28
    } else if n_beams > 0 && remaining / n_beams >= 10 {
        remaining / n_beams
    } else {
        return None;
    };

    let mut soundings = Vec::with_capacity(n_beams);

    for i in 0..n_beams {
        let offset = beam_data_offset + i * record_size;
        if offset + record_size > payload.len() {
            break;
        }

        let beam_number = u16::from_le_bytes([payload[offset], payload[offset + 1]]);

        let (depth, across_track, along_track, beam_angle, quality, detection_info);

        if record_size >= 28 {
            // Float format (newer EM series)
            depth = f32::from_le_bytes([
                payload[offset + 2], payload[offset + 3], payload[offset + 4], payload[offset + 5],
            ]) as f64;
            across_track = f32::from_le_bytes([
                payload[offset + 6], payload[offset + 7], payload[offset + 8], payload[offset + 9],
            ]) as f64;
            along_track = f32::from_le_bytes([
                payload[offset + 10], payload[offset + 11], payload[offset + 12], payload[offset + 13],
            ]) as f64;
            beam_angle = i16::from_le_bytes([payload[offset + 14], payload[offset + 15]]) as f64 / 100.0;
            quality = payload[offset + 16];
            detection_info = payload[offset + 17];
        } else {
            // Standard 12-byte format (integer, scaled)
            depth = i16::from_le_bytes([payload[offset + 2], payload[offset + 3]]) as f64 / 10.0;
            across_track = i16::from_le_bytes([payload[offset + 4], payload[offset + 5]]) as f64 / 10.0;
            along_track = i16::from_le_bytes([payload[offset + 6], payload[offset + 7]]) as f64 / 10.0;
            beam_angle = i16::from_le_bytes([payload[offset + 8], payload[offset + 9]]) as f64 / 100.0;
            quality = payload[offset + 10];
            detection_info = payload[offset + 11];
        }

        soundings.push(KongsbergSounding {
            timestamp,
            ping_number,
            beam_number,
            depth,
            across_track,
            along_track,
            beam_angle,
            heave: att.map(|a| a.heave).unwrap_or(0.0),
            roll: att.map(|a| a.roll).unwrap_or(0.0),
            pitch: att.map(|a| a.pitch).unwrap_or(0.0),
            heading: att.map(|a| a.heading).unwrap_or(0.0),
            sound_speed,
            quality,
            detection_info,
        });
    }

    if soundings.is_empty() {
        None
    } else {
        Some(soundings)
    }
}

/// Find the nearest entry in a time-sorted buffer by binary search.
/// Uses a simple linear scan from the last position (sequential access
/// pattern makes this O(1) amortized).
fn find_nearest<T: Timestamped>(buffer: &[T], last_idx: &mut usize, target_time: f64) -> Option<T> {
    if buffer.is_empty() {
        return None;
    }

    // Advance the index until we pass the target time
    while *last_idx < buffer.len() - 1
        && buffer[*last_idx].timestamp() < target_time
    {
        *last_idx += 1;
    }

    // Check if previous entry is closer
    if *last_idx > 0 {
        let prev = buffer[*last_idx - 1].timestamp();
        let curr = buffer[*last_idx].timestamp();
        if (target_time - prev).abs() < (curr - target_time).abs() {
            return Some(buffer[*last_idx - 1].clone());
        }
    }

    Some(buffer[*last_idx].clone())
}

/// Trait for types with a timestamp — used by find_nearest.
trait Timestamped {
    fn timestamp(&self) -> f64;
}

impl Timestamped for KongsbergAttitude {
    fn timestamp(&self) -> f64 {
        self.timestamp
    }
}

impl Timestamped for KongsbergPosition {
    fn timestamp(&self) -> f64 {
        self.timestamp
    }
}

// ──────────────────────────────────────────────────────────────────
// Water Column datagram parsing — object detection in the water column
// ──────────────────────────────────────────────────────────────────

/// A water column sample — raw acoustic amplitude at a specific depth
/// in a specific beam. Used for detecting objects in the water column:
/// fish schools, wrecks, gas plumes, underwater structures.
#[derive(Debug, Clone, Serialize)]
pub struct WaterColumnSample {
    /// Timestamp (Unix seconds)
    pub timestamp: f64,
    /// Ping number
    pub ping_number: u32,
    /// Beam number
    pub beam_number: u16,
    /// Range from transducer (meters)
    pub range: f64,
    /// Acoustic amplitude (dB)
    pub amplitude_db: f64,
}

/// An object detected in the water column.
#[derive(Debug, Clone, Serialize)]
pub struct WaterColumnObject {
    /// Object type
    pub object_type: WaterColumnObjectType,
    /// Center position (easting, northing) — approximate
    pub position: (f64, f64),
    /// Depth of the object center (meters, positive down)
    pub depth: f64,
    /// Approximate size (meters)
    pub size_m: f64,
    /// Detection confidence (0-1)
    pub confidence: f64,
    /// Ping number where detected
    pub ping_number: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterColumnObjectType {
    /// Fish school — diffuse, mid-water, high backscatter
    FishSchool,
    /// Wreck/obstruction — solid, on or near seafloor
    Wreck,
    /// Gas plume — rising from seafloor
    GasPlume,
    /// Underwater structure — cable, pipeline, etc.
    Structure,
    /// Unknown — needs manual review
    Unknown,
}

/// Parse a Kongsberg .all water column datagram (type 0x57).
///
/// Water column datagrams contain raw acoustic amplitude samples
/// for each beam at each range bin. This is the raw data that can
/// be used for object detection.
///
/// Returns a vector of (beam_number, range, amplitude) samples.
fn parse_water_column_datagram(
    payload: &[u8],
    ping_number: u32,
) -> Option<Vec<WaterColumnSample>> {
    if payload.len() < 20 {
        return None;
    }

    let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64
        + u16::from_le_bytes([payload[4], payload[5]]) as f64 / 1000.0;

    // Water column datagram format (simplified):
    //   - Date (4 bytes)
    //   - Time fraction (2 bytes)
    //   - Ping counter (2 bytes)
    //   - System serial (2 bytes)
    //   - Sound speed (2 bytes)
    //   - Sample rate (2 bytes)
    //   - Number of beams (2 bytes)
    //   - Samples per beam (2 bytes)
    //   - Per-beam: beam number (2) + range to first sample (2) + samples (N bytes)

    let n_beams = u16::from_le_bytes([payload[14], payload[15]]) as usize;
    let samples_per_beam = u16::from_le_bytes([payload[16], payload[17]]) as usize;

    if n_beams == 0 || n_beams > 1024 || samples_per_beam == 0 || samples_per_beam > 10000 {
        return None;
    }

    let mut samples = Vec::new();
    let mut offset = 18; // After the fixed header

    for beam_idx in 0..n_beams {
        if offset + 4 > payload.len() {
            break;
        }

        let beam_number = u16::from_le_bytes([payload[offset], payload[offset + 1]]);
        let range_start = u16::from_le_bytes([payload[offset + 2], payload[offset + 3]]) as f64 / 10.0; // decimeters → meters
        offset += 4;

        // Each sample is 1 byte (8-bit amplitude) or 2 bytes (16-bit)
        // We detect by checking remaining bytes vs expected count
        let bytes_per_sample = if n_beams * samples_per_beam * 2 + offset <= payload.len() {
            2
        } else {
            1
        };

        for sample_idx in 0..samples_per_beam {
            if offset + bytes_per_sample > payload.len() {
                break;
            }

            let amplitude_raw = if bytes_per_sample == 2 {
                u16::from_le_bytes([payload[offset], payload[offset + 1]])
            } else {
                u16::from(payload[offset])
            };

            // Convert raw count to dB (approximate — real conversion needs
            // TVG correction and system gain, which varies by sonar model)
            let amplitude_db = if amplitude_raw > 0 {
                20.0 * (amplitude_raw as f64 / 65535.0).log10()
            } else {
                -100.0 // floor for zero amplitude
            };

            let range = range_start + sample_idx as f64 * 0.1; // 10cm per sample (typical)

            samples.push(WaterColumnSample {
                timestamp,
                ping_number,
                beam_number,
                range,
                amplitude_db,
            });

            offset += bytes_per_sample;
        }
    }

    if samples.is_empty() {
        None
    } else {
        Some(samples)
    }
}

/// Detect objects in water column data using simple thresholding.
///
/// Algorithm: for each ping, find contiguous range bins where
/// amplitude exceeds a threshold. If the contiguous block is
/// more than `min_size_m` meters in range, classify it as an object.
pub fn detect_water_column_objects(
    samples: &[WaterColumnSample],
    threshold_db: f64,
    min_size_m: f64,
) -> Vec<WaterColumnObject> {
    let mut objects = Vec::new();
    let mut current_pings: std::collections::HashMap<u32, Vec<&WaterColumnSample>> =
        std::collections::HashMap::new();

    // Group samples by ping
    for s in samples {
        current_pings.entry(s.ping_number).or_default().push(s);
    }

    // For each ping, find contiguous amplitude peaks
    for (ping_number, ping_samples) in &current_pings {
        let mut in_peak = false;
        let mut peak_start = 0.0;
        let mut peak_max = -100.0;
        let mut peak_count = 0usize;
        let mut peak_beam = 0u16;

        for s in ping_samples.iter() {
            if s.amplitude_db > threshold_db {
                if !in_peak {
                    in_peak = true;
                    peak_start = s.range;
                    peak_max = s.amplitude_db;
                    peak_count = 1;
                    peak_beam = s.beam_number;
                } else {
                    peak_max = peak_max.max(s.amplitude_db);
                    peak_count += 1;
                }
            } else if in_peak {
                // End of peak — check if it's big enough
                let peak_size = s.range - peak_start;
                if peak_size >= min_size_m {
                    let object_type = classify_water_column_object(peak_max, peak_size, peak_start);
                    objects.push(WaterColumnObject {
                        object_type,
                        position: (0.0, 0.0), // Would need position integration for real coords
                        depth: peak_start,
                        size_m: peak_size,
                        confidence: ((peak_max - threshold_db) / 20.0).min(1.0),
                        ping_number: *ping_number,
                    });
                }
                in_peak = false;
            }
        }
    }

    objects
}

/// Classify a water column object based on its characteristics.
fn classify_water_column_object(
    peak_amplitude: f64,
    size_m: f64,
    depth_m: f64,
) -> WaterColumnObjectType {
    // Heuristic classification:
    // - Mid-water (depth < 50m), large (>5m), moderate amplitude → fish school
    // - Near seafloor (depth > 30m), compact (<5m), high amplitude → wreck
    // - Rising from seafloor, diffuse → gas plume
    // - Otherwise → unknown

    if depth_m < 50.0 && size_m > 5.0 && peak_amplitude < -10.0 {
        WaterColumnObjectType::FishSchool
    } else if depth_m > 30.0 && size_m < 5.0 && peak_amplitude > -5.0 {
        WaterColumnObjectType::Wreck
    } else if depth_m > 20.0 && size_m > 3.0 {
        WaterColumnObjectType::GasPlume
    } else {
        WaterColumnObjectType::Unknown
    }
}
