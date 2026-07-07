// Mining surveyor tools — setting out, markout, mine grid, underground survey.
//
// Based on the Mining Surveyor job description:
//   - "Setting out and marking the locations of new mining works"
//   - "Mapping and measuring underground and surface mining areas"
//   - "Assisting in the design and layout of mining infrastructure"
//   - "Updating existing mine maps and records"
//   - "Identifying potential hazards or obstacles within the mining area"
//
// These tools bridge the gap between raw coordinate data and the
// actionable outputs a mining surveyor needs: markout sheets, design
// setout coordinates, tunnel profiles, and compliance reports.

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────────
// Setting Out / Markout
// ──────────────────────────────────────────────────────────────────

/// A design point to be set out in the field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetoutPoint {
    /// Point ID (e.g., "P-001", "BH-0142")
    pub id: String,
    /// Design Easting (meters, mine grid)
    pub easting: f64,
    /// Design Northing (meters, mine grid)
    pub northing: f64,
    /// Design elevation (meters, mine datum)
    pub elevation: f64,
    /// Description (e.g., "Blast hole collar", "Peg", "Bench toe")
    pub description: String,
    /// Point type
    #[serde(rename = "pointType")]
    pub point_type: SetoutPointType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetoutPointType {
    /// Blast hole collar
    BlastHole,
    /// Survey peg/marker
    Peg,
    /// Bench toe line
    BenchToe,
    /// Bench crest line
    BenchCrest,
    /// Road centerline
    RoadCenterline,
    /// Road edge
    RoadEdge,
    /// Drill pattern
    DrillPattern,
    /// Infrastructure (conveyor, crusher, etc.)
    Infrastructure,
    /// Hazard boundary
    HazardBoundary,
    /// Custom
    Custom,
}

/// Result of a setout calculation — design coords + bearing/distance from a known point.
#[derive(Debug, Clone, Serialize)]
pub struct SetoutResult {
    pub point: SetoutPoint,
    /// Bearing from reference point (degrees, 0=N, clockwise)
    pub bearing_deg: f64,
    /// Horizontal distance from reference point (meters)
    pub distance_m: f64,
    /// Elevation difference from reference (meters)
    pub delta_z: f64,
    /// Slope distance (meters)
    pub slope_distance: f64,
    /// Slope angle (degrees, positive up)
    pub slope_angle_deg: f64,
}

/// Compute setout information for a list of design points from a known reference.
///
/// The surveyor stands at the reference point (known coordinate) and
/// needs the bearing + distance to each design point to set it out
/// with a total station or RTK GPS.
pub fn compute_setout(
    points: &[SetoutPoint],
    ref_easting: f64,
    ref_northing: f64,
    ref_elevation: f64,
) -> Vec<SetoutResult> {
    points
        .iter()
        .map(|p| {
            let delta_e = p.easting - ref_easting;
            let delta_n = p.northing - ref_northing;
            let delta_z = p.elevation - ref_elevation;

            // Bearing: 0=N, clockwise. atan2(east, north) gives radians from north.
            let bearing_rad = delta_e.atan2(delta_n);
            let bearing_deg = if bearing_rad < 0.0 {
                bearing_rad.to_degrees() + 360.0
            } else {
                bearing_rad.to_degrees()
            };

            let distance_m = (delta_e * delta_e + delta_n * delta_n).sqrt();
            let slope_distance = (delta_e * delta_e + delta_n * delta_n + delta_z * delta_z).sqrt();
            let slope_angle_deg = if distance_m > 0.001 {
                (delta_z / distance_m).atan().to_degrees()
            } else {
                0.0
            };

            SetoutResult {
                point: p.clone(),
                bearing_deg,
                distance_m,
                delta_z,
                slope_distance,
                slope_angle_deg,
            }
        })
        .collect()
}

// ──────────────────────────────────────────────────────────────────
// Mine Grid Setup
// ──────────────────────────────────────────────────────────────────

/// A mine grid definition — a local coordinate system tied to a
/// projected CRS (e.g., MGA Zone 55) via an origin shift + rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineGrid {
    /// Mine grid name (e.g., "NEWMONT-A")
    pub name: String,
    /// Origin easting in the parent CRS (meters)
    pub origin_easting: f64,
    /// Origin northing in the parent CRS (meters)
    pub origin_northing: f64,
    /// Rotation from grid north to true north (degrees, clockwise positive)
    pub rotation_deg: f64,
    /// Scale factor (usually 1.0)
    pub scale_factor: f64,
    /// Parent CRS (e.g., "EPSG:28355")
    pub parent_crs: String,
}

/// Convert mine grid coordinates to parent CRS coordinates.
pub fn mine_grid_to_crs(
    grid: &MineGrid,
    grid_easting: f64,
    grid_northing: f64,
) -> (f64, f64) {
    let theta = grid.rotation_deg.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // Apply rotation + scale, then translate to origin
    let crs_e = grid.origin_easting + grid.scale_factor * (grid_easting * cos_t - grid_northing * sin_t);
    let crs_n = grid.origin_northing + grid.scale_factor * (grid_easting * sin_t + grid_northing * cos_t);

    (crs_e, crs_n)
}

/// Convert parent CRS coordinates to mine grid coordinates.
pub fn crs_to_mine_grid(
    grid: &MineGrid,
    crs_easting: f64,
    crs_northing: f64,
) -> (f64, f64) {
    let theta = (-grid.rotation_deg).to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // Translate to origin, then inverse-rotate
    let de = (crs_easting - grid.origin_easting) / grid.scale_factor;
    let dn = (crs_northing - grid.origin_northing) / grid.scale_factor;

    let grid_e = de * cos_t - dn * sin_t;
    let grid_n = de * sin_t + dn * cos_t;

    (grid_e, grid_n)
}

// ──────────────────────────────────────────────────────────────────
// Underground Survey — Tunnel Profile
// ──────────────────────────────────────────────────────────────────

/// A tunnel profile cross-section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelProfile {
    /// Chainage along the drive (meters)
    pub chainage: f64,
    /// Profile points (width, height) relative to the drive centerline
    /// at floor level. Width positive = right wall, negative = left wall.
    /// Height positive = above floor.
    pub points: Vec<(f64, f64)>,
    /// Design profile (if available) for overbreak/underbreak comparison
    #[serde(default)]
    pub design_profile: Option<Vec<(f64, f64)>>,
}

/// Result of tunnel profile analysis.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelProfileResult {
    /// Cross-sectional area (square meters)
    pub area: f64,
    /// Design area (if available)
    pub design_area: Option<f64>,
    /// Overbreak area (positive = excavated more than design)
    pub overbreak: Option<f64>,
    /// Underbreak area (positive = excavated less than design)
    pub underbreak: Option<f64>,
    /// Maximum width (meters)
    pub max_width: f64,
    /// Maximum height (meters)
    pub max_height: f64,
}

/// Analyze a tunnel profile: compute area, compare against design.
pub fn analyze_tunnel_profile(profile: &TunnelProfile) -> Result<TunnelProfileResult, String> {
    if profile.points.len() < 3 {
        return Err("tunnel profile needs at least 3 points".to_string());
    }

    // Compute area using the shoelace formula
    let area = shoelace_area(&profile.points);

    // Compute max width and height
    let max_width = profile
        .points
        .iter()
        .map(|(w, _)| w.abs())
        .fold(0.0f64, |a, b| a.max(b))
        * 2.0; // width is from center to wall, so *2 for full width
    let max_height = profile
        .points
        .iter()
        .map(|(_, h)| *h)
        .fold(0.0f64, |a, b| a.max(b));

    // Compare against design if available
    let (design_area, overbreak, underbreak) = if let Some(ref design) = profile.design_profile {
        let d_area = shoelace_area(design);
        let diff = area - d_area;
        let overbreak = if diff > 0.0 { diff } else { 0.0 };
        let underbreak = if diff < 0.0 { -diff } else { 0.0 };
        (Some(d_area), Some(overbreak), Some(underbreak))
    } else {
        (None, None, None)
    };

    Ok(TunnelProfileResult {
        area,
        design_area,
        overbreak,
        underbreak,
        max_width,
        max_height,
    })
}

/// Compute the area of a polygon using the shoelace formula.
fn shoelace_area(points: &[(f64, f64)]) -> f64 {
    let n = points.len();
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += points[i].0 * points[j].1;
        sum -= points[j].0 * points[i].1;
    }
    (sum / 2.0).abs()
}

// ──────────────────────────────────────────────────────────────────
// Compliance & Safety Reporting
// ──────────────────────────────────────────────────────────────────

/// A safety/hazard inspection record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyInspection {
    /// Inspection date (ISO 8601)
    pub date: String,
    /// Inspector name
    pub inspector: String,
    /// Area inspected (e.g., "Pit A — Bench 1050")
    pub area: String,
    /// List of hazards identified
    pub hazards: Vec<Hazard>,
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hazard {
    /// Hazard type
    pub hazard_type: HazardType,
    /// Location (easting, northing, elevation)
    pub location: (f64, f64, f64),
    /// Description
    pub description: String,
    /// Severity (1=low, 5=critical)
    pub severity: u8,
    /// Status (open/resolved/mitigated)
    pub status: HazardStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardType {
    WallInstability,
    Rockfall,
    WaterInflow,
    Equipment,
    BlastMisfire,
    SlopeFailure,
    Subsidence,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardStatus {
    Open,
    Mitigated,
    Resolved,
}

/// Generate a safety inspection report from inspection data.
pub fn generate_safety_report(inspection: &SafetyInspection) -> String {
    let mut report = String::new();
    report.push_str(&format!("SAFETY INSPECTION REPORT\n"));
    report.push_str(&format!("========================\n\n"));
    report.push_str(&format!("Date: {}\n", inspection.date));
    report.push_str(&format!("Inspector: {}\n", inspection.inspector));
    report.push_str(&format!("Area: {}\n", inspection.area));
    report.push_str(&format!("Risk Level: {:?}\n\n", inspection.risk_level));

    if !inspection.hazards.is_empty() {
        report.push_str(&format!("HAZARDS ({}):\n", inspection.hazards.len()));
        for (i, h) in inspection.hazards.iter().enumerate() {
            report.push_str(&format!(
                "  {}. {:?} — Severity {} — {:?}\n     Location: {:.1}, {:.1}, {:.1}\n     {}\n",
                i + 1,
                h.hazard_type,
                h.severity,
                h.status,
                h.location.0,
                h.location.1,
                h.location.2,
                h.description
            ));
        }
        report.push_str("\n");
    }

    if !inspection.recommendations.is_empty() {
        report.push_str("RECOMMENDATIONS:\n");
        for (i, r) in inspection.recommendations.iter().enumerate() {
            report.push_str(&format!("  {}. {}\n", i + 1, r));
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_setout() {
        let points = vec![SetoutPoint {
            id: "P-001".to_string(),
            easting: 100.0,
            northing: 100.0,
            elevation: 50.0,
            description: "Test peg".to_string(),
            point_type: SetoutPointType::Peg,
        }];
        let results = compute_setout(&points, 0.0, 0.0, 0.0);
        assert_eq!(results.len(), 1);
        // Point at (100, 100) from origin → bearing = 45° (NE)
        assert!((results[0].bearing_deg - 45.0).abs() < 0.1);
        assert!((results[0].distance_m - 141.42).abs() < 0.1);
    }

    #[test]
    fn test_mine_grid_round_trip() {
        let grid = MineGrid {
            name: "TEST".to_string(),
            origin_easting: 500000.0,
            origin_northing: 6000000.0,
            rotation_deg: 10.0,
            scale_factor: 1.0,
            parent_crs: "EPSG:28355".to_string(),
        };
        let (crs_e, crs_n) = mine_grid_to_crs(&grid, 100.0, 200.0);
        let (grid_e, grid_n) = crs_to_mine_grid(&grid, crs_e, crs_n);
        assert!((grid_e - 100.0).abs() < 1e-6);
        assert!((grid_n - 200.0).abs() < 1e-6);
    }

    #[test]
    fn test_tunnel_profile() {
        // Simple 4m × 4m rectangular tunnel profile
        let profile = TunnelProfile {
            chainage: 100.0,
            points: vec![
                (-2.0, 0.0),
                (2.0, 0.0),
                (2.0, 4.0),
                (-2.0, 4.0),
            ],
            design_profile: Some(vec![
                (-1.8, 0.0),
                (1.8, 0.0),
                (1.8, 3.8),
                (-1.8, 3.8),
            ]),
        };
        let result = analyze_tunnel_profile(&profile).unwrap();
        assert!((result.area - 16.0).abs() < 0.1); // 4×4 = 16 m²
        assert!(result.overbreak.unwrap() > 0.0); // actual > design
    }

    #[test]
    fn test_shoelace_area() {
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!((shoelace_area(&square) - 100.0).abs() < 1e-6);
    }
}
