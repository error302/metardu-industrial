// Autonomous Surface Vessel (USV) & Drone Swarm Real-Time Telemetry Gateway.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsvTelemetryFrame {
    pub vessel_id: String,
    pub vessel_name: String,
    pub timestamp_utc: String,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub depth_m: f64,
    pub speed_kts: f64,
    pub heading_deg: f64,
    pub battery_percent: u8,
    pub rtk_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetStatusReport {
    pub total_vessels_online: usize,
    pub active_telemetry: Vec<UsvTelemetryFrame>,
}

/// Parse incoming JSON or NMEA telemetry frame from remote USV over UDP / WebSockets.
pub fn parse_usv_telemetry(json_payload: &str) -> Option<UsvTelemetryFrame> {
    serde_json::from_str(json_payload).ok()
}

/// Summarize active USV fleet status.
pub fn summarize_usv_fleet(frames: &[UsvTelemetryFrame]) -> FleetStatusReport {
    FleetStatusReport {
        total_vessels_online: frames.len(),
        active_telemetry: frames.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usv_telemetry_parse() {
        let json = r#"{
            "vessel_id": "USV-01",
            "vessel_name": "Otter USV #1",
            "timestamp_utc": "2026-08-12T19:30:00Z",
            "latitude_deg": -1.285,
            "longitude_deg": 36.82,
            "depth_m": 14.5,
            "speed_kts": 3.8,
            "heading_deg": 182.0,
            "battery_percent": 88,
            "rtk_status": "RTK Fixed"
        }"#;

        let frame = parse_usv_telemetry(json).unwrap();
        assert_eq!(frame.vessel_id, "USV-01");
        assert_eq!(frame.battery_percent, 88);
    }
}
