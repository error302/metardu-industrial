// Side-Scan Sonar (SSS) data parser — pure Rust.
//
// Supports XTF (eXtended Triton Format) — the de-facto standard for
// side-scan sonar data from EdgeTech, Klein, Marine Sonic, and others.
// Also handles a generic ping-array format for in-memory data passed
// from the frontend (when the surveyor has already pre-extracted pings).
//
// Per ROADMAP.md Priority #8 — SSS Waterfall Viewer.
//
// XTF format reference: Triton Imaging 'XTF Data Format Rev 26'.
// High-level structure:
//   - 1024-byte file header (magic 'XGTF' + channel count + offsets)
//   - N channel definitions (one per sonar channel — typically port + starboard)
//   - Ping packets (CHIRP sonar data + bathymetry + sidescan backscatter)
//   - Each ping has a 256-byte ping header followed by the backscatter samples
//
// Phase 1 reader extracts:
//   - Verify XTF magic
//   - Walk ping packets
//   - Extract port + starboard backscatter amplitude per ping
//   - Per-ping navigation (lat/lon) + heading + altitude (fish height)
//   - Time stamps for waterfall scrolling
//
// The frontend renders this as a Canvas2D scrolling waterfall:
//   - X axis = across-track samples (port on left, starboard on right)
//   - Y axis = ping index (scrolls as new pings arrive)
//   - Pixel intensity = backscatter amplitude (log-scaled)

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// XTF file header — the first 1024 bytes of an XTF file.
/// Magic is "XGTF" (0x58 0x47 0x54 0x46).
#[derive(Debug, Clone, Serialize)]
pub struct XtfHeader {
    pub magic: String,
    pub file_format_version: u8,
    pub system_type: u8,
    pub sonar_name: String,
    pub n_channels: u8,
    pub total_ping_count_hint: u32,
}

/// One ping of side-scan data. Port and starboard are separate Vecs
/// because SSS fish typically have two transducers angled outward.
#[derive(Debug, Clone, Serialize)]
pub struct SssPing {
    /// Sequence number (0-based, monotonically increasing per channel)
    pub ping_number: u32,
    /// Unix epoch seconds when this ping was acquired
    pub timestamp_secs: f64,
    /// Latitude of the SSS fish (degrees WGS84)
    pub latitude: f64,
    /// Longitude of the SSS fish (degrees WGS84)
    pub longitude: f64,
    /// Heading (degrees, 0=N, 90=E, clockwise)
    pub heading_deg: f32,
    /// Altitude of the fish above the seabed (meters) — used to compute
    /// slant-range correction for the waterfall
    pub altitude_m: f32,
    /// Speed of sound at transducer (m/s)
    pub sound_speed_mps: f32,
    /// Port-side backscatter samples (amplitude, 0-255 typical)
    pub port_samples: Vec<u8>,
    /// Starboard-side backscatter samples
    pub starboard_samples: Vec<u8>,
    /// Sample interval (seconds between samples) — used to compute
    /// across-track distance = sample_index × sound_speed × sample_interval / 2
    pub sample_interval_secs: f32,
}

/// Result of parsing an XTF file. Contains the file header + all
/// extracted pings. The frontend uses this to render the waterfall.
#[derive(Debug, Clone, Serialize)]
pub struct SssData {
    pub header: XtfHeader,
    pub pings: Vec<SssPing>,
    /// Maximum samples per channel across all pings (for waterfall width)
    pub max_samples_per_channel: usize,
    /// Total ping count
    pub total_pings: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SssError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not an XTF file: missing 'XGTF' magic, got: {0}")]
    NotXtf(String),
    #[error("XTF file truncated at offset {0}")]
    Truncated(u64),
    #[error("unsupported XTF packet type: {0}")]
    UnsupportedPacketType(u16),
    #[error("no sonar data found in file")]
    NoSonarData,
}

/// Read the XTF file header (first 1024 bytes).
pub fn read_xtf_header(path: &Path) -> Result<XtfHeader, SssError> {
    let mut file = File::open(path)?;
    let mut header_buf = [0u8; 1024];
    file.read_exact(&mut header_buf)?;

    // Magic — bytes 0-3: "XGTF"
    let magic = String::from_utf8_lossy(&header_buf[0..4]).to_string();
    if magic != "XGTF" {
        // Maybe it's a generic format — check if it's actually XTF
        let hex: String = header_buf[0..4].iter().map(|b| format!("{:02X}", b)).collect();
        return Err(SssError::NotXtf(format!("'{}' (hex: {})", magic, hex)));
    }

    let file_format_version = header_buf[4];
    let system_type = header_buf[5];
    let sonar_name = String::from_utf8_lossy(&header_buf[6..38])
        .trim_end_matches('\0')
        .to_string();
    // Number of channels is at offset 64 (u8) per XTF spec
    let n_channels = header_buf[64];
    // Total ping count hint at offset 100 (u32 little-endian) — may be 0 if unknown
    let total_ping_count_hint = u32::from_le_bytes([
        header_buf[100], header_buf[101], header_buf[102], header_buf[103],
    ]);

    Ok(XtfHeader {
        magic,
        file_format_version,
        system_type,
        sonar_name,
        n_channels,
        total_ping_count_hint,
    })
}

/// Read all pings from an XTF file.
///
/// Walks the ping packets, extracting port + starboard backscatter
/// samples, navigation, heading, and altitude. Pings without nav data
/// are still included (lat/lon = 0,0).
pub fn read_xtf_pings(path: &Path, max_pings: usize) -> Result<SssData, SssError> {
    let header = read_xtf_header(path)?;
    let mut file = File::open(path)?;

    // Skip the 1024-byte file header
    file.seek(SeekFrom::Start(1024))?;

    // Skip channel definition packets. Each channel def is 1024 bytes,
    // and there are `n_channels` of them.
    let chan_def_size = 1024u64 * (header.n_channels as u64);
    file.seek(SeekFrom::Start(1024 + chan_def_size))?;

    let mut pings: Vec<SssPing> = Vec::new();
    let mut max_samples_per_channel = 0usize;
    let max_pings = if max_pings == 0 { usize::MAX } else { max_pings };

    // Walk ping packets. Each ping packet starts with a 256-byte ping header.
    // Byte 0-1: packet type (0x00 = sidescan, 0x01 = bathymetry, 0x02 = raw)
    // We only care about 0x00 for the waterfall.
    while pings.len() < max_pings {
        let mut ping_header = [0u8; 256];
        match file.read_exact(&mut ping_header) {
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(SssError::Io(e)),
        }

        // Packet type at bytes 0-1 (little-endian u16)
        let packet_type = u16::from_le_bytes([ping_header[0], ping_header[1]]);
        if packet_type != 0x0000 {
            // Skip non-sidescan packets — read the data size and skip
            let data_size = u32::from_le_bytes([
                ping_header[4], ping_header[5], ping_header[6], ping_header[7],
            ]) as u64;
            file.seek(SeekFrom::Current(data_size as i64))
                .map_err(|_| SssError::Truncated(file.stream_position().unwrap_or(0)))?;
            continue;
        }

        // Ping number at bytes 10-13 (u32 LE)
        let ping_number = u32::from_le_bytes([
            ping_header[10], ping_header[11], ping_header[12], ping_header[13],
        ]);

        // Timestamp — XTF uses a Windows FILETIME (100ns ticks since 1601-01-01)
        // stored at bytes 14-21 as u64 LE. Convert to Unix epoch seconds.
        let filetime_ticks = u64::from_le_bytes([
            ping_header[14], ping_header[15], ping_header[16], ping_header[17],
            ping_header[18], ping_header[19], ping_header[20], ping_header[21],
        ]);
        let timestamp_secs = filetime_to_unix_secs(filetime_ticks);

        // Latitude at bytes 22-29 (f64 LE)
        let latitude = f64::from_le_bytes([
            ping_header[22], ping_header[23], ping_header[24], ping_header[25],
            ping_header[26], ping_header[27], ping_header[28], ping_header[29],
        ]);
        // Longitude at bytes 30-37 (f64 LE)
        let longitude = f64::from_le_bytes([
            ping_header[30], ping_header[31], ping_header[32], ping_header[33],
            ping_header[34], ping_header[35], ping_header[36], ping_header[37],
        ]);
        // Heading at bytes 56-59 (f32 LE, degrees)
        let heading_deg = f32::from_le_bytes([
            ping_header[56], ping_header[57], ping_header[58], ping_header[59],
        ]);
        // Altitude (fish height) at bytes 84-87 (f32 LE, meters)
        let altitude_m = f32::from_le_bytes([
            ping_header[84], ping_header[85], ping_header[86], ping_header[87],
        ]);
        // Sound speed at bytes 88-91 (f32 LE)
        let sound_speed_mps = f32::from_le_bytes([
            ping_header[88], ping_header[89], ping_header[90], ping_header[91],
        ]);
        // Sample interval at bytes 96-99 (f32 LE, seconds)
        let sample_interval_secs = f32::from_le_bytes([
            ping_header[96], ping_header[97], ping_header[98], ping_header[99],
        ]);

        // Each ping has data for multiple channels. The number of channels
        // per ping is at byte 9 of the ping header (u8).
        let n_chan_in_ping = ping_header[9] as usize;
        if n_chan_in_ping == 0 {
            continue;
        }

        // Read the data section. Bytes 4-7 give the total data size for all channels.
        let total_data_size = u32::from_le_bytes([
            ping_header[4], ping_header[5], ping_header[6], ping_header[7],
        ]) as usize;

        let mut data_buf = vec![0u8; total_data_size];
        file.read_exact(&mut data_buf)
            .map_err(|_| SssError::Truncated(file.stream_position().unwrap_or(0)))?;

        // The data is laid out as: per channel: [u32 samples_per_channel] [u16 bytes_per_sample] [samples...]
        // Channel 1 = port, Channel 2 = starboard (XTF convention).
        // Each channel also has a 64-byte channel info prefix per the spec, but most
        // sonar vendors simplify this — we'll be tolerant.
        let mut port_samples = Vec::new();
        let mut starboard_samples = Vec::new();
        let mut offset = 0usize;
        for chan_idx in 0..n_chan_in_ping {
            if offset + 8 > data_buf.len() {
                break;
            }
            // First 4 bytes: samples per channel for this chan
            let n_samples = u32::from_le_bytes([
                data_buf[offset], data_buf[offset + 1], data_buf[offset + 2], data_buf[offset + 3],
            ]) as usize;
            // Next 4 bytes: bytes per sample (typically 1 for u8 backscatter, 2 for u16)
            let bytes_per_sample = u16::from_le_bytes([
                data_buf[offset + 4], data_buf[offset + 5],
            ]) as usize;
            offset += 8;
            // Some vendors insert a 64-byte channel info block here. Detect: if
            // offset + 64 + n_samples * bytes_per_sample > total_data_size, skip the 64-byte block.
            let needed = n_samples * bytes_per_sample;
            if offset + needed > data_buf.len() {
                break;
            }

            // Extract samples as u8 (1 byte/sample) or downsample u16 → u8
            let samples: Vec<u8> = if bytes_per_sample == 1 {
                data_buf[offset..offset + n_samples.min(data_buf.len() - offset)].to_vec()
            } else if bytes_per_sample == 2 {
                let mut out = Vec::with_capacity(n_samples);
                for i in 0..n_samples {
                    let pos = offset + i * 2;
                    if pos + 1 < data_buf.len() {
                        let v = u16::from_le_bytes([data_buf[pos], data_buf[pos + 1]]);
                        out.push((v >> 8) as u8); // take high byte
                    }
                }
                out
            } else {
                Vec::new()
            };

            if chan_idx == 0 {
                port_samples = samples;
            } else if chan_idx == 1 {
                starboard_samples = samples;
            }

            offset += needed;
            // Skip 64-byte channel info block if present (typical for XTF Rev 26+)
            if offset + 64 <= data_buf.len() && offset + 64 + needed > data_buf.len() {
                offset += 64;
            }
        }

        if port_samples.len() > max_samples_per_channel {
            max_samples_per_channel = port_samples.len();
        }
        if starboard_samples.len() > max_samples_per_channel {
            max_samples_per_channel = starboard_samples.len();
        }

        pings.push(SssPing {
            ping_number,
            timestamp_secs,
            latitude,
            longitude,
            heading_deg,
            altitude_m,
            sound_speed_mps,
            port_samples,
            starboard_samples,
            sample_interval_secs,
        });
    }

    if pings.is_empty() {
        return Err(SssError::NoSonarData);
    }

    let total_pings = pings.len();
    Ok(SssData {
        header,
        pings,
        max_samples_per_channel,
        total_pings,
    })
}

/// Convert a Windows FILETIME (100ns ticks since 1601-01-01) to Unix
/// epoch seconds (since 1970-01-01).
///
/// The difference between 1601-01-01 and 1970-01-01 is 11644473600 seconds.
fn filetime_to_unix_secs(filetime: u64) -> f64 {
    const FILETIME_EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    let unix_100ns = filetime.saturating_sub(FILETIME_EPOCH_DIFF_SECS * 10_000_000);
    unix_100ns as f64 / 10_000_000.0
}

/// Compute target height from shadow length using the similar-triangles method.
///
/// When a side-scan sonar ensonifies a target proud of the seabed, the
/// target casts an acoustic shadow behind it. The shadow length, fish
/// altitude, and slant range to the target form similar triangles:
///
///   target_height / shadow_length = fish_altitude / (slant_range + shadow_length)
///
/// Therefore:
///   target_height = fish_altitude × shadow_length / (slant_range + shadow_length)
///
/// All inputs in meters. Returns target height in meters.
pub fn compute_target_height_from_shadow(
    fish_altitude_m: f64,
    slant_range_to_target_m: f64,
    shadow_length_m: f64,
) -> f64 {
    if slant_range_to_target_m + shadow_length_m <= 0.0 {
        return 0.0;
    }
    fish_altitude_m * shadow_length_m / (slant_range_to_target_m + shadow_length_m)
}

/// Convert across-track sample index to slant range distance in meters.
///
/// slant_range = sample_index × sound_speed × sample_interval / 2
/// (factor of 2 because sound travels out and back)
pub fn sample_index_to_slant_range(
    sample_index: usize,
    sound_speed_mps: f64,
    sample_interval_secs: f64,
) -> f64 {
    sample_index as f64 * sound_speed_mps * sample_interval_secs / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filetime_to_unix_secs_known_value() {
        // 2026-01-01 00:00:00 UTC = 1767225600 Unix epoch seconds
        // Windows FILETIME = (Unix_secs + 11644473600) × 10_000_000
        //                  = (1767225600 + 11644473600) × 10_000_000
        //                  = 13411699200 × 10_000_000
        //                  = 134116992000000000 (100ns ticks since 1601)
        let filetime: u64 = 134_116_992_000_000_000;
        let unix = filetime_to_unix_secs(filetime);
        assert!((unix - 1767225600.0).abs() < 1.0, "got: {}", unix);
    }

    #[test]
    fn test_filetime_to_unix_secs_zero() {
        // FILETIME 0 = 1601-01-01 → should saturate to 0 (before Unix epoch)
        let unix = filetime_to_unix_secs(0);
        assert_eq!(unix, 0.0);
    }

    #[test]
    fn test_compute_target_height_simple() {
        // Fish at 5m altitude, target at 10m slant range, 2m shadow
        // height = 5 × 2 / (10 + 2) = 10/12 = 0.833m
        let h = compute_target_height_from_shadow(5.0, 10.0, 2.0);
        assert!((h - 0.8333).abs() < 0.01, "got: {}", h);
    }

    #[test]
    fn test_compute_target_height_zero_shadow() {
        let h = compute_target_height_from_shadow(5.0, 10.0, 0.0);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_sample_index_to_slant_range() {
        // 1500 m/s sound, 50µs sample interval, sample 100
        // = 100 × 1500 × 0.00005 / 2 = 3.75 m
        let r = sample_index_to_slant_range(100, 1500.0, 0.00005);
        assert!((r - 3.75).abs() < 0.001, "got: {}", r);
    }

    #[test]
    fn test_read_xtf_header_not_xtf() {
        // Create a fake file with wrong magic
        let tmp = std::env::temp_dir().join("metardu_test_not_xtf.bin");
        std::fs::write(&tmp, b"NOT_XTF_FILE_DATA_PADDING_TO_1024_BYTES".to_vec()).unwrap();
        // Pad to 1024 bytes
        let mut buf = std::fs::read(&tmp).unwrap();
        buf.resize(1024, 0);
        std::fs::write(&tmp, &buf).unwrap();
        let r = read_xtf_header(&tmp);
        assert!(r.is_err());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_xtf_header_valid_magic() {
        let tmp = std::env::temp_dir().join("metardu_test_xtf_header.bin");
        let mut buf = vec![0u8; 1024];
        // Magic "XGTF"
        buf[0..4].copy_from_slice(b"XGTF");
        buf[4] = 26; // file format version
        buf[5] = 1; // system type
        // Sonar name at bytes 6..38
        let name = b"EdgeTech 4125";
        buf[6..6 + name.len()].copy_from_slice(name);
        buf[64] = 2; // 2 channels (port + starboard)
        let count: u32 = 1000;
        buf[100..104].copy_from_slice(&count.to_le_bytes());
        std::fs::write(&tmp, &buf).unwrap();

        let header = read_xtf_header(&tmp).unwrap();
        assert_eq!(header.magic, "XGTF");
        assert_eq!(header.file_format_version, 26);
        assert_eq!(header.n_channels, 2);
        assert_eq!(header.sonar_name, "EdgeTech 4125");
        assert_eq!(header.total_ping_count_hint, 1000);

        let _ = std::fs::remove_file(&tmp);
    }
}
