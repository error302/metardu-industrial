// Real-time field operations — RTK rover position, live tide gauge.
//
// This module provides the data-ingest side of the real-time field
// features introduced in Sprint 11. The frontend renders the rover
// position on the OpenLayers map and shows the live tide graph; this
// module is responsible for getting the bytes off the wire and into
// typed Rust structs.
//
// Two submodules:
//   - nmea: pure-Rust NMEA 0183 sentence parser (no_std-friendly, no
//     allocations on the hot path). Supports GGA, RMC, GLL, GSA, VTG.
//   - rover: TCP client that streams NMEA sentences from a serial-to-TCP
//     bridge or a base-station TCP port, decodes them, and updates a
//     shared position struct. The frontend polls via IPC.

pub mod nmea;
pub mod rover;
pub mod tide;

use serde::{Deserialize, Serialize};

/// A single position fix from the rover.
///
/// All fields are optional because different NMEA sentence types
/// populate different subsets — GGA gives lat/lon/alt/quality/sats/HDOP,
/// RMC gives speed/course/date, GLL gives lat/lon only. The rover
/// module merges them into a single `RoverPosition` per fix interval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoverPosition {
    /// Unix timestamp (seconds, UTC)
    pub timestamp: f64,
    /// Latitude (decimal degrees, WGS84). None if no fix.
    pub latitude: Option<f64>,
    /// Longitude (decimal degrees, WGS84). None if no fix.
    pub longitude: Option<f64>,
    /// Altitude above ellipsoid (meters)
    pub altitude_m: Option<f64>,
    /// Fix quality: 0=no fix, 1=GPS, 2=DGPS, 4=RTK fixed, 5=RTK float, 6=dead reckoning
    pub fix_quality: Option<u8>,
    /// Number of satellites in use
    pub satellites: Option<u8>,
    /// Horizontal dilution of precision
    pub hdop: Option<f64>,
    /// Speed over ground (m/s)
    pub speed_mps: Option<f64>,
    /// Course over ground (degrees true)
    pub course_deg: Option<f64>,
    /// Age of differential correction (seconds)
    pub age_of_diff_s: Option<f64>,
    /// Station ID of the differential reference station
    pub diff_station_id: Option<u16>,
}

/// Fix-quality labels for the UI.
pub fn fix_quality_label(q: u8) -> &'static str {
    match q {
        0 => "No fix",
        1 => "GPS",
        2 => "DGPS",
        3 => "PPS",
        4 => "RTK Fixed",
        5 => "RTK Float",
        6 => "Dead Reckoning",
        7 => "Manual",
        8 => "Simulation",
        _ => "Unknown",
    }
}
