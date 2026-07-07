// Real-time tide gauge ingest — NOAA CO-OPS API + local TCP socket.
//
// The NOAA CO-OPS API (https://api.tidesandcurrents.noaa.gov/api/prod/datagetter)
// returns 6-minute water level observations for any of ~200 US tide stations.
// Free, no auth, returns JSON or CSV. This module fetches observations for a
// station + date range, parses them into a `TideSeries`, and exposes a
// spline-interpolation function that gives the tide height at any timestamp.
//
// For non-US waters (or vessels with their own gauge), the TCP socket mode
// reads ASCII tide lines like "2026-07-07T12:34:56Z,1.234" and builds the
// same `TideSeries`.
//
// The frontend uses this to:
//   1. Show a live tide graph (last 24 hours + next 6 hours predicted)
//   2. Apply tide correction to loaded bathymetry soundings in real time,
//      so the QC dashboard shows corrected depths during the survey

use serde::{Deserialize, Serialize};

/// A single tide observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TideObservation {
    /// Unix timestamp (seconds, UTC)
    pub timestamp: f64,
    /// Water level (meters, relative to station datum — usually MLLW)
    pub level_m: f64,
    /// Quality flag: 'o' = verified observation, 'p' = prediction
    pub quality: char,
}

/// A time-ordered series of tide observations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TideSeries {
    pub station_id: String,
    pub station_name: String,
    pub datum: String, // "MLLW", "MSL", "NAVD88", etc.
    pub observations: Vec<TideObservation>,
}

impl TideSeries {
    /// Linearly interpolate the tide level at the given Unix timestamp.
    ///
    /// Returns `None` if the timestamp is outside the observation range
    /// or if there are fewer than 2 observations.
    pub fn level_at(&self, t: f64) -> Option<f64> {
        let obs = &self.observations;
        if obs.len() < 2 {
            return None;
        }
        // Before first observation
        if t < obs[0].timestamp {
            return None;
        }
        // After last observation
        if t > obs[obs.len() - 1].timestamp {
            return None;
        }
        // Binary search for the bracketing pair
        let mut lo = 0usize;
        let mut hi = obs.len() - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if obs[mid].timestamp <= t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let t0 = obs[lo].timestamp;
        let t1 = obs[hi].timestamp;
        let v0 = obs[lo].level_m;
        let v1 = obs[hi].level_m;
        if t1 == t0 {
            Some(v0)
        } else {
            let alpha = (t - t0) / (t1 - t0);
            Some(v0 + alpha * (v1 - v0))
        }
    }

    /// Apply tide correction to a list of (timestamp, depth) pairs.
    ///
    /// `depths` are raw soundings relative to the transducer. The
    /// corrected depth is `depth + tide_level(t)` — assuming the
    /// soundings are measured downward from the surface and the tide
    /// level is positive upward. Returns the corrected depths.
    ///
    /// Soundings whose timestamp falls outside the tide range are
    /// returned unchanged (with a flag in the second return value).
    pub fn apply_to_soundings(
        &self,
        depths: &[(f64, f64)],
    ) -> (Vec<f64>, Vec<bool>) {
        depths
            .iter()
            .map(|(t, d)| match self.level_at(*t) {
                Some(level) => (d + level, true),
                None => (*d, false),
            })
            .unzip()
    }

    /// Min/max/mean stats for the series.
    pub fn stats(&self) -> Option<TideStats> {
        if self.observations.is_empty() {
            return None;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        for o in &self.observations {
            if o.level_m < min {
                min = o.level_m;
            }
            if o.level_m > max {
                max = o.level_m;
            }
            sum += o.level_m;
        }
        let mean = sum / self.observations.len() as f64;
        let range = max - min;
        Some(TideStats { min, max, mean, range })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TideStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub range: f64,
}

/// Build the NOAA CO-OPS API URL for a 6-minute water level request.
///
/// `station_id` is the 7-character CO-OPS station ID (e.g., "8454000"
/// for Providence, RI). `start_date` and `end_date` are `YYYYMMDD`.
/// `datum` is one of "MLLW", "MSL", "NAVD88", "STND".
pub fn build_noaa_url(station_id: &str, start_date: &str, end_date: &str, datum: &str) -> String {
    format!(
        "https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?product=water_level&application=metardu&begin_date={}&end_date={}&datum={}&station={}&time_zone=gmt&units=metric&format=json",
        start_date, end_date, datum, station_id
    )
}

/// Parse the JSON response from the CO-OPS API.
///
/// Example response:
/// ```json
/// {
///   "metadata": { "id": "8454000", "name": "Providence", "lat": 41.8071, "lon": -71.4012 },
///   "data": [
///     { "t": "2026-07-07 12:00", "v": "1.234", "q": "v" },
///     ...
///   ]
/// }
/// ```
///
/// The `t` field is `YYYY-MM-DD HH:MM` in GMT.
pub fn parse_noaa_response(json: &str) -> Result<TideSeries, String> {
    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        metadata: Option<NoaaMetadata>,
        #[serde(default)]
        data: Vec<NoaaObservation>,
    }
    #[derive(Deserialize)]
    struct NoaaMetadata {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: String,
    }
    #[derive(Deserialize)]
    struct NoaaObservation {
        t: String,           // "2026-07-07 12:00"
        #[serde(default)]
        v: String,           // "1.234" or empty
        #[serde(default)]
        q: String,           // "v" verified, "p" prediction
    }

    let resp: Response = serde_json::from_str(json)
        .map_err(|e| format!("parsing NOAA JSON: {}", e))?;

    let station_id = resp.metadata.as_ref().map(|m| m.id.clone()).unwrap_or_default();
    let station_name = resp.metadata.as_ref().map(|m| m.name.clone()).unwrap_or_default();

    let observations = resp
        .data
        .iter()
        .filter_map(|o| {
            // Parse "2026-07-07 12:00" → Unix timestamp
            let ts = parse_noaa_timestamp(&o.t)?;
            // Skip empty values (common during gauge outage)
            if o.v.is_empty() {
                return None;
            }
            let level = o.v.parse::<f64>().ok()?;
            let quality = o.q.chars().next().unwrap_or('p');
            Some(TideObservation {
                timestamp: ts,
                level_m: level,
                quality,
            })
        })
        .collect();

    Ok(TideSeries {
        station_id,
        station_name,
        datum: "MLLW".to_string(), // NOAA default — overridden by caller if needed
        observations,
    })
}

/// Parse "2026-07-07 12:00" → Unix timestamp (UTC).
fn parse_noaa_timestamp(s: &str) -> Option<f64> {
    // Expected: "YYYY-MM-DD HH:MM"
    if s.len() < 16 {
        return None;
    }
    let year: u32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let minute: u32 = s[14..16].parse().ok()?;
    Some(unix_seconds(year, month, day, hour, minute, 0))
}

/// Convert a UTC date to Unix seconds. Proleptic Gregorian, no leap seconds.
fn unix_seconds(year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> f64 {
    // Days since 1970-01-01 (proleptic Gregorian)
    let mut days: i64 = 0;
    for y in 1970..year as i64 {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        let d = month_days[(m - 1) as usize];
        days += if m == 2 && is_leap_year(year as i64) { 29 } else { d as i64 };
    }
    days += (day - 1) as i64;
    (days * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + second as i64) as f64
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Parse a tide TCP stream line into a `TideObservation`.
///
/// Accepted formats (one observation per line):
///   - `2026-07-07T12:34:56Z,1.234`       (ISO 8601 + comma + level)
///   - `1751888096,1.234`                  (Unix seconds + comma + level)
///   - `2026-07-07T12:34:56Z 1.234`       (whitespace separator)
pub fn parse_tide_line(line: &str) -> Option<TideObservation> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    // Try comma first, then whitespace
    let (ts_str, level_str) = line.split_once(',').or_else(|| {
        // Find the first whitespace separator
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() == 2 {
            Some((parts[0], parts[1]))
        } else {
            None
        }
    })?;

    let level = level_str.trim().parse::<f64>().ok()?;
    let timestamp = if ts_str.starts_with(char::is_numeric) && !ts_str.contains('-') {
        // Pure number → Unix timestamp
        ts_str.parse::<f64>().ok()?
    } else {
        parse_iso_timestamp(ts_str.trim())?
    };

    Some(TideObservation {
        timestamp,
        level_m: level,
        quality: 'o',
    })
}

/// Parse a limited subset of ISO 8601: `2026-07-07T12:34:56Z` or
/// `2026-07-07T12:34:56+00:00`. Returns Unix seconds.
fn parse_iso_timestamp(s: &str) -> Option<f64> {
    if s.len() < 19 {
        return None;
    }
    let year: u32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let minute: u32 = s[14..16].parse().ok()?;
    let second: u32 = s[17..19].parse().ok()?;
    Some(unix_seconds(year, month, day, hour, minute, second))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolation_within_range() {
        let series = TideSeries {
            station_id: "TEST".to_string(),
            station_name: "Test".to_string(),
            datum: "MLLW".to_string(),
            observations: vec![
                TideObservation { timestamp: 100.0, level_m: 1.0, quality: 'o' },
                TideObservation { timestamp: 200.0, level_m: 2.0, quality: 'o' },
                TideObservation { timestamp: 300.0, level_m: 1.5, quality: 'o' },
            ],
        };
        assert!((series.level_at(100.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((series.level_at(150.0).unwrap() - 1.5).abs() < 1e-9);
        assert!((series.level_at(200.0).unwrap() - 2.0).abs() < 1e-9);
        assert!((series.level_at(250.0).unwrap() - 1.75).abs() < 1e-9);
        assert!((series.level_at(300.0).unwrap() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_interpolation_outside_range() {
        let series = TideSeries {
            station_id: "TEST".to_string(),
            station_name: "Test".to_string(),
            datum: "MLLW".to_string(),
            observations: vec![
                TideObservation { timestamp: 100.0, level_m: 1.0, quality: 'o' },
                TideObservation { timestamp: 200.0, level_m: 2.0, quality: 'o' },
            ],
        };
        assert!(series.level_at(50.0).is_none());
        assert!(series.level_at(250.0).is_none());
        assert!(series.level_at(99.9).is_none());
        assert!(series.level_at(200.1).is_none());
    }

    #[test]
    fn test_apply_to_soundings() {
        let series = TideSeries {
            station_id: "TEST".to_string(),
            station_name: "Test".to_string(),
            datum: "MLLW".to_string(),
            observations: vec![
                TideObservation { timestamp: 100.0, level_m: 1.0, quality: 'o' },
                TideObservation { timestamp: 200.0, level_m: 2.0, quality: 'o' },
            ],
        };
        let depths = vec![(150.0, 10.0), (300.0, 20.0)];
        let (corrected, applied) = series.apply_to_soundings(&depths);
        assert!((corrected[0] - 11.5).abs() < 1e-9);
        assert!(applied[0]);
        // 300 is outside range → no correction
        assert!((corrected[1] - 20.0).abs() < 1e-9);
        assert!(!applied[1]);
    }

    #[test]
    fn test_parse_noaa_response() {
        let json = r#"{
            "metadata": {"id": "8454000", "name": "Providence", "lat": 41.8071, "lon": -71.4012},
            "data": [
                {"t": "2026-07-07 12:00", "v": "1.234", "q": "v"},
                {"t": "2026-07-07 12:06", "v": "1.245", "q": "v"},
                {"t": "2026-07-07 12:12", "v": "1.256", "q": "v"}
            ]
        }"#;
        let series = parse_noaa_response(json).unwrap();
        assert_eq!(series.station_id, "8454000");
        assert_eq!(series.station_name, "Providence");
        assert_eq!(series.observations.len(), 3);
        assert!((series.observations[0].level_m - 1.234).abs() < 1e-9);
        assert_eq!(series.observations[0].quality, 'v');
        // Verify timestamp parsing — 2026-07-07 12:00 UTC
        // (we just check it's a sensible positive number)
        assert!(series.observations[0].timestamp > 1.7e9);
        // 6-minute interval
        assert!((series.observations[1].timestamp - series.observations[0].timestamp - 360.0).abs() < 1e-9);
    }

    #[test]
    fn test_parse_tide_line_iso() {
        let obs = parse_tide_line("2026-07-07T12:34:56Z,1.234").unwrap();
        assert!((obs.level_m - 1.234).abs() < 1e-9);
        assert!(obs.timestamp > 1.7e9);
    }

    #[test]
    fn test_parse_tide_line_unix() {
        let obs = parse_tide_line("1751888096,1.234").unwrap();
        assert!((obs.level_m - 1.234).abs() < 1e-9);
        assert!((obs.timestamp - 1751888096.0).abs() < 1e-9);
    }

    #[test]
    fn test_parse_tide_line_whitespace() {
        let obs = parse_tide_line("2026-07-07T12:34:56Z 1.234").unwrap();
        assert!((obs.level_m - 1.234).abs() < 1e-9);
    }

    #[test]
    fn test_parse_tide_line_invalid() {
        assert!(parse_tide_line("").is_none());
        assert!(parse_tide_line("garbage").is_none());
        assert!(parse_tide_line("2026-07-07T12:34:56Z,not_a_number").is_none());
    }

    #[test]
    fn test_stats() {
        let series = TideSeries {
            station_id: "TEST".to_string(),
            station_name: "Test".to_string(),
            datum: "MLLW".to_string(),
            observations: vec![
                TideObservation { timestamp: 100.0, level_m: 1.0, quality: 'o' },
                TideObservation { timestamp: 200.0, level_m: 3.0, quality: 'o' },
                TideObservation { timestamp: 300.0, level_m: 2.0, quality: 'o' },
            ],
        };
        let stats = series.stats().unwrap();
        assert!((stats.min - 1.0).abs() < 1e-9);
        assert!((stats.max - 3.0).abs() < 1e-9);
        assert!((stats.mean - 2.0).abs() < 1e-9);
        assert!((stats.range - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_unix_seconds_basic() {
        // 1970-01-01 00:00:00 UTC = 0
        assert_eq!(unix_seconds(1970, 1, 1, 0, 0, 0), 0.0);
        // 1970-01-02 00:00:00 UTC = 86400
        assert_eq!(unix_seconds(1970, 1, 2, 0, 0, 0), 86400.0);
        // 1971-01-01 00:00:00 UTC = 365 days (1970 not leap)
        assert_eq!(unix_seconds(1971, 1, 1, 0, 0, 0), 365.0 * 86400.0);
        // 2000-01-01 — known timestamp
        assert!((unix_seconds(2000, 1, 1, 0, 0, 0) - 946684800.0).abs() < 1e-6);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(!is_leap_year(2026));
    }
}
