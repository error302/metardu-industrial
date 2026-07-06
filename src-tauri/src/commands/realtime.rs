// Real-time IPC commands — Sprint 11.
//
// Wires the rover NMEA TCP stream and the tide gauge (NOAA CO-OPS or
// TCP) to the frontend. Each command is a thin wrapper around the
// shared `RoverState` and `TideSeries` types.

use crate::realtime::rover::{RoverStatus, RoverState};
use crate::realtime::tide::{TideObservation, TideSeries};
use crate::realtime::RoverPosition;
use std::sync::OnceLock;

/// Singleton rover state — survives across IPC calls.
static ROVER_STATE: OnceLock<RoverState> = OnceLock::new();

fn rover() -> &'static RoverState {
    ROVER_STATE.get_or_init(RoverState::new)
}

// ──────────────────────────────────────────────────────────────────
// Rover
// ──────────────────────────────────────────────────────────────────

/// Start streaming NMEA sentences from a TCP source.
#[tauri::command]
pub async fn start_rover_stream_cmd(host: String, port: u16) -> Result<(), String> {
    rover().start(host, port)
}

/// Stop the active rover stream.
#[tauri::command]
pub fn stop_rover_stream_cmd() {
    rover().stop();
}

/// Get the latest merged rover position.
#[tauri::command]
pub fn get_rover_position_cmd() -> RoverPosition {
    rover().position()
}

/// Get the position trail (oldest first).
#[tauri::command]
pub fn get_rover_trail_cmd() -> Vec<RoverPosition> {
    rover().trail()
}

/// Get connection status and sentence counters.
#[tauri::command]
pub fn get_rover_status_cmd() -> RoverStatus {
    rover().status()
}

// ──────────────────────────────────────────────────────────────────
// Tide gauge — NOAA CO-OPS fetch (HTTP) + TCP line-streaming mode
// ──────────────────────────────────────────────────────────────────

/// Fetch a tide series from NOAA CO-OPS for a station + date range.
///
/// `station_id` is the 7-character CO-OPS station ID (e.g., "8454000").
/// `begin_date` and `end_date` are `YYYYMMDD HH:MM` strings (NOAA format).
/// `datum` is one of MLLW / MSL / NAVD88 / STND.
///
/// Performs an HTTP GET using reqwest (already a dependency for the
/// updater) and parses the JSON response.
#[tauri::command]
pub async fn fetch_noaa_tide_cmd(
    station_id: String,
    begin_date: String,
    end_date: String,
    datum: String,
) -> Result<TideSeries, String> {
    let url = crate::realtime::tide::build_noaa_url(&station_id, &begin_date, &end_date, &datum);
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("NOAA HTTP request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("reading NOAA response: {e}"))?;
    let mut series = crate::realtime::tide::parse_noaa_response(&body)
        .map_err(|e| format!("parsing NOAA tide JSON: {e}"))?;
    series.datum = datum;
    Ok(series)
}

/// Parse a tide TCP stream chunk into observations.
///
/// The frontend opens a TCP socket to the gauge (or NTRIP-style
/// server), reads lines, and calls this command with each chunk.
/// Returns the observations parsed from complete lines in the chunk;
/// incomplete trailing lines are returned as the second element so
/// the frontend can prepend them to the next chunk.
#[tauri::command]
pub fn parse_tide_tcp_chunk_cmd(chunk: String) -> (Vec<TideObservation>, String) {
    let mut observations = Vec::new();
    let mut remaining = String::new();

    let mut lines = chunk.split('\n').peekable();
    while let Some(line) = lines.next() {
        if lines.peek().is_none() && !line.ends_with('\n') {
            // Last partial line — return for the next chunk
            remaining = line.to_string();
        } else if let Some(obs) = crate::realtime::tide::parse_tide_line(line) {
            observations.push(obs);
        }
    }

    (observations, remaining)
}

/// Apply tide correction to a list of (timestamp, depth) soundings.
///
/// Returns a list of (corrected_depth, applied) tuples. `applied=false`
/// for soundings whose timestamp falls outside the tide observation range.
#[tauri::command]
pub fn apply_tide_correction_cmd(
    series: TideSeries,
    soundings: Vec<(f64, f64)>,
) -> Vec<(f64, bool)> {
    let (corrected, applied) = series.apply_to_soundings(&soundings);
    corrected.into_iter().zip(applied.into_iter()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rover_singleton() {
        let r1 = rover();
        let r2 = rover();
        // Same instance
        assert!(std::ptr::eq(r1, r2));
    }

    #[test]
    fn test_parse_tide_tcp_chunk_complete() {
        let chunk = "2026-07-07T12:34:56Z,1.234\n2026-07-07T12:40:00Z,1.245\n";
        let (obs, remaining) = parse_tide_tcp_chunk_cmd(chunk.to_string());
        assert_eq!(obs.len(), 2);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_parse_tide_tcp_chunk_partial() {
        let chunk = "2026-07-07T12:34:56Z,1.234\n2026-07-07T12:40:00Z,1.24";
        let (obs, remaining) = parse_tide_tcp_chunk_cmd(chunk.to_string());
        assert_eq!(obs.len(), 1);
        assert_eq!(remaining, "2026-07-07T12:40:00Z,1.24");
    }
}
