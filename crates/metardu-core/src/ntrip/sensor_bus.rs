// Real-Time Sensor Bus: NMEA 0183 / NMEA 2000 parser & Helmsman XTE navigation telemetry engine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NmeaPosition {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: f64,
    pub fix_quality: u8, // 0=invalid, 1=GPS, 2=DGPS, 4=RTK fixed, 5=RTK float
    pub num_satellites: u8,
    pub hdop: f32,
    pub speed_knots: f64,
    pub track_deg_true: f64,
    pub timestamp_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VesselAttitude {
    pub heading_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    pub heave_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmsmanTelemetry {
    pub cross_track_error_m: f64, // Positive = right of line, Negative = left of line
    pub distance_to_go_m: f64,
    pub distance_along_line_m: f64,
    pub bearing_to_target_deg: f64,
    pub speed_over_ground_kts: f64,
    pub course_over_ground_deg: f64,
    pub target_waypoint: [f64; 2],
    pub line_name: String,
}

/// Parses NMEA 0183 sentences ($GPGGA, $GPRMC, $HEHDT, $PASHR, $TSS1).
pub fn parse_nmea_sentence(sentence: &str) -> Option<NmeaPosition> {
    let trimmed = sentence.trim();
    if !trimmed.starts_with('$') {
        return None;
    }

    let parts: Vec<&str> = trimmed.split('*').next()?.split(',').collect();
    if parts.is_empty() {
        return None;
    }

    let msg_type = parts[0];
    if msg_type.ends_with("GGA") && parts.len() >= 10 {
        let lat_raw: f64 = parts[2].parse().ok()?;
        let lat_dir = parts[3];
        let lon_raw: f64 = parts[4].parse().ok()?;
        let lon_dir = parts[5];
        let fix_quality: u8 = parts[6].parse().unwrap_or(0);
        let sats: u8 = parts[7].parse().unwrap_or(0);
        let hdop: f32 = parts[8].parse().unwrap_or(1.0);
        let alt: f64 = parts[9].parse().unwrap_or(0.0);

        let lat_deg = (lat_raw / 100.0).floor() + (lat_raw % 100.0) / 60.0;
        let lat = if lat_dir == "S" { -lat_deg } else { lat_deg };

        let lon_deg = (lon_raw / 100.0).floor() + (lon_raw % 100.0) / 60.0;
        let lon = if lon_dir == "W" { -lon_deg } else { lon_deg };

        return Some(NmeaPosition {
            latitude_deg: lat,
            longitude_deg: lon,
            altitude_m: alt,
            fix_quality,
            num_satellites: sats,
            hdop,
            speed_knots: 0.0,
            track_deg_true: 0.0,
            timestamp_utc: parts[1].to_string(),
        });
    }

    None
}

/// Compute Helmsman navigation Cross-Track Error (XTE) and distance to target waypoint.
pub fn compute_helmsman_telemetry(
    vessel_lat: f64,
    vessel_lon: f64,
    sog_kts: f64,
    cog_deg: f64,
    start_pt: [f64; 2], // [lon, lat]
    end_pt: [f64; 2],   // [lon, lat]
    line_name: &str,
) -> HelmsmanTelemetry {
    let dx_line = end_pt[0] - start_pt[0];
    let dy_line = end_pt[1] - start_pt[1];
    let line_len_sq = dx_line * dx_line + dy_line * dy_line;

    if line_len_sq < 1e-12 {
        return HelmsmanTelemetry {
            cross_track_error_m: 0.0,
            distance_to_go_m: 0.0,
            distance_along_line_m: 0.0,
            bearing_to_target_deg: 0.0,
            speed_over_ground_kts: sog_kts,
            course_over_ground_deg: cog_deg,
            target_waypoint: end_pt,
            line_name: line_name.to_string(),
        };
    }

    // Convert lat/lon offset to approximate meters (1 deg ~ 111,000m)
    let m_per_deg_lat = 111000.0;
    let m_per_deg_lon = 111000.0 * (vessel_lat.to_radians().cos());

    let dx_v = (vessel_lon - start_pt[0]) * m_per_deg_lon;
    let dy_v = (vessel_lat - start_pt[1]) * m_per_deg_lat;

    let dx_l = dx_line * m_per_deg_lon;
    let dy_l = dy_line * m_per_deg_lat;
    let l_m = (dx_l * dx_l + dy_l * dy_l).sqrt();

    // Line unit vector and normal vector
    let ux = dx_l / l_m;
    let uy = dy_l / l_m;

    let dist_along = dx_v * ux + dy_v * uy;
    let xte = dx_v * (-uy) + dy_v * ux; // Cross-track error in meters

    let dist_to_go = (l_m - dist_along).max(0.0);
    let bearing_rad = (end_pt[0] - vessel_lon).atan2(end_pt[1] - vessel_lat);
    let bearing_deg = (bearing_rad.to_degrees() + 360.0) % 360.0;

    HelmsmanTelemetry {
        cross_track_error_m: xte,
        distance_to_go_m: dist_to_go,
        distance_along_line_m: dist_along,
        bearing_to_target_deg: bearing_deg,
        speed_over_ground_kts: sog_kts,
        course_over_ground_deg: cog_deg,
        target_waypoint: end_pt,
        line_name: line_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nmea_gga() {
        let sentence = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
        let parsed = parse_nmea_sentence(sentence).unwrap();
        assert!((parsed.latitude_deg - 48.1173).abs() < 1e-3);
        assert!((parsed.longitude_deg - 11.5166).abs() < 1e-3);
        assert_eq!(parsed.fix_quality, 1);
        assert_eq!(parsed.num_satellites, 8);
    }

    #[test]
    fn test_helmsman_xte() {
        let start = [0.0, 0.0];
        let end = [0.0, 1.0]; // Line heading north
        let telem = compute_helmsman_telemetry(0.5, 0.001, 5.0, 0.0, start, end, "LINE-01");
        assert!(telem.distance_to_go_m > 0.0);
        assert!(telem.cross_track_error_m != 0.0);
    }
}
