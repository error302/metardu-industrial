// AR companion app scaffold — Phase 5.
//
// Per ARCHITECTURE.md §9.10 — an iPad companion app for stakeout
// with AR overlays. The surveyor holds up the iPad, sees the design
// points overlaid on the real world through the camera, and verifies
// placement.
//
// Phase 5 scaffold: defines the AR data model, the stakeout protocol,
// and the IPC commands. The actual AR rendering uses ARKit (iOS) or
// ARCore (Android) via Tauri's mobile entry point.
//
// Data flow:
//   1. Desktop MetaRDU Industrial exports a "stakeout package" — a JSON
//      file with design points, control points, and the mine grid CRS
//   2. The companion app loads the package on the iPad
//   3. The app uses the iPad's GPS + compass + camera to determine
//      position and heading
//   4. Design points within the camera's field of view are rendered
//      as AR overlays at their real-world positions
//   5. The surveyor taps a point to mark it as "staked" or "verified"
//   6. Results sync back to the desktop app

use serde::{Deserialize, Serialize};

/// A stakeout package — exported from the desktop app, loaded on the iPad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeoutPackage {
    pub name: String,
    pub created: String,
    pub crs: String,
    /// Design points to stake
    pub points: Vec<DesignPoint>,
    /// Known control points for orientation
    pub control_points: Vec<ControlPoint>,
    /// Mine grid parameters (for local CRS transforms)
    #[serde(default)]
    pub mine_grid: Option<MineGrid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignPoint {
    pub id: String,
    pub label: String,
    pub point_type: DesignPointType,
    /// Easting in mine grid (meters)
    pub easting: f64,
    /// Northing in mine grid (meters)
    pub northing: f64,
    /// Elevation (meters)
    pub elevation: f64,
    /// Whether this point has been staked
    #[serde(default)]
    pub staked: bool,
    /// Whether this point has been verified
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignPointType {
    /// Drill hole collar
    DrillHole,
    /// Survey peg
    Peg,
    /// Blast hole
    BlastHole,
    /// Benchmark
    Benchmark,
    /// Toe of batter
    Toe,
    /// Crest of batter
    Crest,
    /// Custom point
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlPoint {
    pub id: String,
    pub easting: f64,
    pub northing: f64,
    pub elevation: f64,
    /// WGS84 position for GPS matching
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineGrid {
    pub epsg: String,
    pub false_easting: f64,
    pub false_northing: f64,
    pub scale_factor: f64,
    pub central_meridian: f64,
    pub latitude_of_origin: f64,
}

/// AR view state — what the iPad camera sees.
#[derive(Debug, Clone, Serialize)]
pub struct ArViewState {
    /// Current GPS position (WGS84)
    pub latitude: f64,
    pub longitude: f64,
    /// Current compass heading (degrees, 0=north)
    pub heading: f64,
    /// Device tilt (degrees, 0=pointing at horizon)
    pub tilt: f64,
    /// Points currently visible in the AR view
    pub visible_points: Vec<VisiblePoint>,
    /// Total points in the package
    pub total_points: usize,
    /// Points staked so far
    pub staked_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisiblePoint {
    pub id: String,
    pub label: String,
    /// Distance from device (meters)
    pub distance: f64,
    /// Bearing from device (degrees, relative to heading)
    pub relative_bearing: f64,
    /// Vertical angle (degrees, 0=horizon, positive=up)
    pub vertical_angle: f64,
    /// Screen position (0.0-1.0, for fallback 2D rendering)
    pub screen_x: f64,
    pub screen_y: f64,
    pub point_type: DesignPointType,
    pub staked: bool,
}

/// Compute which design points are visible given the AR view state.
///
/// Uses the device's GPS + heading to determine which points fall within
/// the camera's field of view (default 60° horizontal).
pub fn compute_visible_points(
    device_lat: f64,
    device_lon: f64,
    heading: f64,
    field_of_view_deg: f64,
    package: &StakeoutPackage,
) -> Vec<VisiblePoint> {
    let mut visible = Vec::new();

    for point in &package.points {
        // Find the nearest control point to get WGS84 position
        // (Phase 5 simplified: use control points as reference)
        if package.control_points.is_empty() {
            continue;
        }

        // Find closest control point
        let nearest = package
            .control_points
            .iter()
            .min_by(|a, b| {
                let dist_a =
                    (a.easting - point.easting).powi(2) + (a.northing - point.northing).powi(2);
                let dist_b =
                    (b.easting - point.easting).powi(2) + (b.northing - point.northing).powi(2);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or_else(|| package.control_points.first().cloned().unwrap_or_default());

        // Offset from control point to design point
        let de = point.easting - nearest.easting;
        let dn = point.northing - nearest.northing;

        // Convert offset to lat/lon delta (approximate)
        let lat_per_m = 1.0 / 111320.0;
        let lon_per_m = 1.0 / (111320.0 * device_lat.to_radians().cos());

        let point_lat = nearest.latitude + dn * lat_per_m;
        let point_lon = nearest.longitude + de * lon_per_m;

        // Compute distance and bearing from device to point
        let distance = haversine(device_lat, device_lon, point_lat, point_lon);
        let bearing = bearing(device_lat, device_lon, point_lat, point_lon);

        // Relative bearing (0 = straight ahead)
        let relative = (bearing - heading + 360.0) % 360.0;
        let relative_normalized = if relative > 180.0 {
            relative - 360.0
        } else {
            relative
        };

        // Check if within field of view
        if relative.abs() <= field_of_view_deg / 2.0 && distance < 500.0 {
            // Compute screen position (simplified: linear projection)
            let screen_x = 0.5 + (relative_normalized / (field_of_view_deg / 2.0)) * 0.5;
            let screen_y = 0.5; // Simplified — would use tilt for real AR

            visible.push(VisiblePoint {
                id: point.id.clone(),
                label: point.label.clone(),
                distance,
                relative_bearing: relative_normalized,
                vertical_angle: 0.0, // Simplified
                screen_x: screen_x.clamp(0.0, 1.0),
                screen_y,
                point_type: point.point_type,
                staked: point.staked,
            });
        }
    }

    // Sort by distance (closest first)
    visible.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    visible
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0;
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dphi = (lat2 - lat1).to_radians();
    let dlambda = (lon2 - lon1).to_radians();
    let h = (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlambda / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dlambda = (lon2 - lon1).to_radians();
    let y = dlambda.sin() * phi2.cos();
    let x = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * dlambda.cos();
    let bearing = y.atan2(x).to_degrees();
    (bearing + 360.0) % 360.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stakeout_package_serialization() {
        let pkg = StakeoutPackage {
            name: "Test Survey".into(),
            created: "2026-07-03".into(),
            crs: "EPSG:28355".into(),
            points: vec![DesignPoint {
                id: "P1".into(),
                label: "DH-001".into(),
                point_type: DesignPointType::DrillHole,
                easting: 100.0,
                northing: 200.0,
                elevation: 150.0,
                staked: false,
                verified: false,
            }],
            control_points: vec![ControlPoint {
                id: "CP1".into(),
                easting: 0.0,
                northing: 0.0,
                elevation: 100.0,
                latitude: -37.81,
                longitude: 144.96,
            }],
            mine_grid: None,
        };
        let json = serde_json::to_string(&pkg).unwrap();
        let reparsed: StakeoutPackage = serde_json::from_str(&json).unwrap();
        assert_eq!(pkg.name, reparsed.name);
        assert_eq!(pkg.points.len(), reparsed.points.len());
    }

    #[test]
    fn test_compute_visible_points() {
        let pkg = StakeoutPackage {
            name: "Test".into(),
            created: "2026".into(),
            crs: "EPSG:4326".into(),
            points: vec![
                DesignPoint {
                    id: "P1".into(),
                    label: "Point 1".into(),
                    point_type: DesignPointType::Peg,
                    easting: 10.0,
                    northing: 0.0,
                    elevation: 100.0,
                    staked: false,
                    verified: false,
                },
                DesignPoint {
                    id: "P2".into(),
                    label: "Point 2".into(),
                    point_type: DesignPointType::Peg,
                    easting: 0.0,
                    northing: 100.0,
                    elevation: 100.0,
                    staked: false,
                    verified: false,
                },
            ],
            control_points: vec![ControlPoint {
                id: "CP1".into(),
                easting: 0.0,
                northing: 0.0,
                elevation: 100.0,
                latitude: -37.81,
                longitude: 144.96,
            }],
            mine_grid: None,
        };

        // Device at control point, heading north (0°), 60° FOV
        let visible = compute_visible_points(-37.81, 144.96, 0.0, 60.0, &pkg);
        // P2 is directly north (bearing 0°) → should be visible
        // P1 is directly east (bearing 90°) → should NOT be visible with 60° FOV
        assert!(visible.iter().any(|p| p.id == "P2"));
        assert!(!visible.iter().any(|p| p.id == "P1"));
    }

    #[test]
    fn test_haversine() {
        let d = haversine(-37.81, 144.96, -37.81, 144.97);
        // ~0.01° longitude at lat -37.81 ≈ 880m
        assert!(d > 800.0 && d < 950.0, "distance {d}");
    }
}
