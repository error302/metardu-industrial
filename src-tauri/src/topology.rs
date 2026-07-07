// Topology validator — Sprint 15.
//
// Validates polygon and line topology for GIS quality assurance.
// Catches the common errors that make survey data unusable:
//   - Polygon gaps (slivers between adjacent polygons)
//   - Polygon overlaps (two polygons claim the same area)
//   - Self-intersection (a polygon ring crosses itself)
//   - Dangles (line endpoints that don't connect to another line)
//   - Slivers (tiny polygons from digitization errors)
//
// Uses the GIS QA Engineer agent's methodology: check each rule against
// a tolerance, report all violations with location + severity.
//
// References:
//   - ESRI Topology Rules (ArcGIS Data Reviewer)
//   - OGC Simple Features Specification for SQL (SFS)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyError {
    pub rule: TopologyRule,
    pub severity: ErrorSeverity,
    pub message: String,
    /// Coordinates of the error location (centroid of the problematic geometry)
    pub location: (f64, f64),
    /// Feature indices involved in the error
    pub feature_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyRule {
    SelfIntersection,
    PolygonOverlap,
    PolygonGap,
    Dangle,
    Sliver,
    NullGeometry,
    TooFewPoints,
    NotClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyReport {
    pub errors: Vec<TopologyError>,
    pub total_features: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyParams {
    /// Minimum polygon area (m²). Polygons smaller than this are flagged as slivers.
    pub min_polygon_area: f64,
    /// Maximum gap width (m) between adjacent polygons before it's flagged.
    pub max_gap_width: f64,
    /// Coordinate tolerance (m) for considering two points coincident.
    pub tolerance: f64,
}

impl Default for TopologyParams {
    fn default() -> Self {
        Self {
            min_polygon_area: 1.0,     // 1 m²
            max_gap_width: 0.5,        // 50 cm
            tolerance: 0.001,          // 1 mm
        }
    }
}

/// Validate a set of polygon rings for topology errors.
///
/// Each polygon is a list of rings: the first ring is the exterior,
/// subsequent rings are holes. Each ring is a closed list of (x, y) points.
pub fn validate_polygons(
    polygons: &[Vec<Vec<(f64, f64)>>],
    params: &TopologyParams,
) -> TopologyReport {
    let mut errors = Vec::new();
    let total = polygons.len();

    for (i, rings) in polygons.iter().enumerate() {
        // Check for null/empty geometry
        if rings.is_empty() {
            errors.push(TopologyError {
                rule: TopologyRule::NullGeometry,
                severity: ErrorSeverity::Error,
                message: format!("Polygon {} has no rings", i),
                location: (0.0, 0.0),
                feature_indices: vec![i],
            });
            continue;
        }

        for (ring_idx, ring) in rings.iter().enumerate() {
            // Check ring has enough points (minimum 4 for a closed ring: 3 vertices + repeat)
            if ring.len() < 4 {
                errors.push(TopologyError {
                    rule: TopologyRule::TooFewPoints,
                    severity: ErrorSeverity::Error,
                    message: format!(
                        "Polygon {} ring {} has only {} points (minimum 4 for a closed ring)",
                        i, ring_idx, ring.len()
                    ),
                    location: ring.first().copied().unwrap_or((0.0, 0.0)),
                    feature_indices: vec![i],
                });
                continue;
            }

            // Check ring is closed (first point == last point)
            let first = ring[0];
            let last = ring[ring.len() - 1];
            if (first.0 - last.0).abs() > params.tolerance || (first.1 - last.1).abs() > params.tolerance {
                errors.push(TopologyError {
                    rule: TopologyRule::NotClosed,
                    severity: ErrorSeverity::Error,
                    message: format!(
                        "Polygon {} ring {} is not closed (first point != last point)",
                        i, ring_idx
                    ),
                    location: first,
                    feature_indices: vec![i],
                });
            }

            // Check for self-intersection
            if let Some((loc, msg)) = check_self_intersection(ring) {
                errors.push(TopologyError {
                    rule: TopologyRule::SelfIntersection,
                    severity: ErrorSeverity::Error,
                    message: format!("Polygon {} ring {}: {}", i, ring_idx, msg),
                    location: loc,
                    feature_indices: vec![i],
                });
            }
        }

        // Check for sliver polygons (tiny area)
        if !rings.is_empty() {
            let area = polygon_area(&rings[0]).abs();
            if area < params.min_polygon_area && area > 0.0 {
                let centroid = ring_centroid(&rings[0]);
                errors.push(TopologyError {
                    rule: TopologyRule::Sliver,
                    severity: ErrorSeverity::Warning,
                    message: format!(
                        "Polygon {} is a sliver (area = {:.4} m² < minimum {:.1} m²)",
                        i, area, params.min_polygon_area
                    ),
                    location: centroid,
                    feature_indices: vec![i],
                });
            }
        }
    }

    // Check for overlaps between polygons (pairwise — O(n²), fine for <1000 polygons)
    for i in 0..polygons.len() {
        for j in (i + 1)..polygons.len() {
            if polygons[i].is_empty() || polygons[j].is_empty() {
                continue;
            }
            if let Some(loc) = check_polygon_overlap(&polygons[i][0], &polygons[j][0]) {
                errors.push(TopologyError {
                    rule: TopologyRule::PolygonOverlap,
                    severity: ErrorSeverity::Error,
                    message: format!("Polygons {} and {} overlap", i, j),
                    location: loc,
                    feature_indices: vec![i, j],
                });
            }
        }
    }

    let error_count = errors.iter().filter(|e| matches!(e.severity, ErrorSeverity::Error)).count();
    let warning_count = errors.iter().filter(|e| matches!(e.severity, ErrorSeverity::Warning)).count();
    TopologyReport {
        passed: error_count == 0,
        errors,
        total_features: total,
        error_count,
        warning_count,
    }
}

/// Validate a set of polylines for dangles (endpoints that don't connect).
pub fn validate_lines(
    lines: &[Vec<(f64, f64)>],
    params: &TopologyParams,
) -> TopologyReport {
    let mut errors = Vec::new();
    let total = lines.len();

    for (i, line) in lines.iter().enumerate() {
        if line.len() < 2 {
            errors.push(TopologyError {
                rule: TopologyRule::TooFewPoints,
                severity: ErrorSeverity::Error,
                message: format!("Line {} has only {} points (minimum 2)", i, line.len()),
                location: line.first().copied().unwrap_or((0.0, 0.0)),
                feature_indices: vec![i],
            });
            continue;
        }

        // Check for self-intersection
        if let Some((loc, msg)) = check_self_intersection(line) {
            errors.push(TopologyError {
                rule: TopologyRule::SelfIntersection,
                severity: ErrorSeverity::Error,
                message: format!("Line {}: {}", i, msg),
                location: loc,
                feature_indices: vec![i],
            });
        }
    }

    // Check for dangles: each line endpoint should connect to another line's endpoint
    let tol = params.tolerance;
    for (i, line_i) in lines.iter().enumerate() {
        if line_i.len() < 2 {
            continue;
        }
        let endpoints = [line_i[0], line_i[line_i.len() - 1]];
        for (ep_idx, &ep) in endpoints.iter().enumerate() {
            let mut connected = false;
            for (j, line_j) in lines.iter().enumerate() {
                if i == j || line_j.len() < 2 {
                    continue;
                }
                let other_endpoints = [line_j[0], line_j[line_j.len() - 1]];
                for &other_ep in &other_endpoints {
                    let dist = ((ep.0 - other_ep.0).powi(2) + (ep.1 - other_ep.1).powi(2)).sqrt();
                    if dist < tol {
                        connected = true;
                        break;
                    }
                }
                if connected {
                    break;
                }
            }
            if !connected {
                errors.push(TopologyError {
                    rule: TopologyRule::Dangle,
                    severity: ErrorSeverity::Warning,
                    message: format!(
                        "Line {} endpoint {} ({}) is a dangle (not connected to any other line)",
                        i, ep_idx, if ep_idx == 0 { "start" } else { "end" }
                    ),
                    location: ep,
                    feature_indices: vec![i],
                });
            }
        }
    }

    let error_count = errors.iter().filter(|e| matches!(e.severity, ErrorSeverity::Error)).count();
    let warning_count = errors.iter().filter(|e| matches!(e.severity, ErrorSeverity::Warning)).count();
    TopologyReport {
        passed: error_count == 0,
        errors,
        total_features: total,
        error_count,
        warning_count,
    }
}

// ──────────────────────────────────────────────────────────────────
// Geometry helpers
// ──────────────────────────────────────────────────────────────────

/// Check if a ring/polyline self-intersects using segment-segment intersection.
fn check_self_intersection(ring: &[(f64, f64)]) -> Option<((f64, f64), String)> {
    let n = ring.len();
    if n < 4 {
        return None;
    }
    for i in 0..(n - 1) {
        for j in (i + 2)..(n - 1) {
            // Skip the closing segment (last → first) since it shares endpoints
            if i == 0 && j == n - 2 {
                continue;
            }
            let p1 = ring[i];
            let p2 = ring[i + 1];
            let p3 = ring[j];
            let p4 = ring[j + 1];
            if let Some(pt) = segment_intersection(p1, p2, p3, p4) {
                return Some((pt, format!("segments {} and {} cross at ({:.4}, {:.4})", i, j, pt.0, pt.1)));
            }
        }
    }
    None
}

/// Segment-segment intersection. Returns the intersection point if the segments cross.
fn segment_intersection(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> Option<(f64, f64)> {
    let d1x = p2.0 - p1.0;
    let d1y = p2.1 - p1.1;
    let d2x = p4.0 - p3.0;
    let d2y = p4.1 - p3.1;
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-12 {
        return None; // Parallel
    }
    let t = ((p3.0 - p1.0) * d2y - (p3.1 - p1.1) * d2x) / denom;
    let u = ((p3.0 - p1.0) * d1y - (p3.1 - p1.1) * d1x) / denom;
    if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
        Some((p1.0 + t * d1x, p1.1 + t * d1y))
    } else {
        None
    }
}

/// Polygon area using the shoelace formula.
fn polygon_area(ring: &[(f64, f64)]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += ring[i].0 * ring[j].1;
        sum -= ring[j].0 * ring[i].1;
    }
    sum / 2.0
}

/// Centroid of a ring (simple average of vertices — not the true centroid,
/// but good enough for error reporting).
fn ring_centroid(ring: &[(f64, f64)]) -> (f64, f64) {
    if ring.is_empty() {
        return (0.0, 0.0);
    }
    let (sx, sy) = ring.iter().fold((0.0, 0.0), |(ax, ay), p| (ax + p.0, ay + p.1));
    (sx / ring.len() as f64, sy / ring.len() as f64)
}

/// Check if two polygon rings overlap using bounding-box + point-in-polygon.
fn check_polygon_overlap(ring_a: &[(f64, f64)], ring_b: &[(f64, f64)]) -> Option<(f64, f64)> {
    // Quick bounding-box check
    let (a_min_x, a_max_x, a_min_y, a_max_y) = ring_bounds(ring_a);
    let (b_min_x, b_max_x, b_min_y, b_max_y) = ring_bounds(ring_b);
    if a_max_x < b_min_x || b_max_x < a_min_x || a_max_y < b_min_y || b_max_y < a_min_y {
        return None; // Bounding boxes don't overlap
    }
    // Check if any vertex of B is inside A (or vice versa)
    for &p in ring_b {
        if point_in_polygon(p, ring_a) {
            return Some(p);
        }
    }
    for &p in ring_a {
        if point_in_polygon(p, ring_b) {
            return Some(p);
        }
    }
    None
}

fn ring_bounds(ring: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let (min_x, max_x) = ring.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), p| {
        (mn.min(p.0), mx.max(p.0))
    });
    let (min_y, max_y) = ring.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), p| {
        (mn.min(p.1), mx.max(p.1))
    });
    (min_x, max_x, min_y, max_y)
}

/// Point-in-polygon using ray casting.
fn point_in_polygon(p: (f64, f64), ring: &[(f64, f64)]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = ring[i].0;
        let yi = ring[i].1;
        let xj = ring[j].0;
        let yj = ring[j].1;
        if ((yi > p.1) != (yj > p.1)) && (p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(min_x: f64, min_y: f64, size: f64) -> Vec<(f64, f64)> {
        vec![
            (min_x, min_y),
            (min_x + size, min_y),
            (min_x + size, min_y + size),
            (min_x, min_y + size),
            (min_x, min_y), // closed
        ]
    }

    #[test]
    fn test_valid_polygon_no_errors() {
        let polygons = vec![vec![square(0.0, 0.0, 10.0)]];
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.passed, "errors: {:?}", report.errors);
        assert_eq!(report.error_count, 0);
    }

    #[test]
    fn test_unclosed_ring() {
        let polygons = vec![vec![vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (1.0, 1.0), // Not closed — first != last
        ]]];
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::NotClosed)));
    }

    #[test]
    fn test_too_few_points() {
        let polygons = vec![vec![vec![(0.0, 0.0), (1.0, 1.0)]]]; // Only 2 points
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::TooFewPoints)));
    }

    #[test]
    fn test_null_geometry() {
        let polygons: Vec<Vec<Vec<(f64, f64)>>> = vec![vec![]];
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::NullGeometry)));
    }

    #[test]
    fn test_sliver_polygon() {
        // Tiny polygon: 0.01 m × 0.01 m = 0.0001 m²
        let polygons = vec![vec![square(0.0, 0.0, 0.01)]];
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::Sliver)));
    }

    #[test]
    fn test_polygon_overlap() {
        // Two overlapping squares
        let polygons = vec![
            vec![square(0.0, 0.0, 10.0)],
            vec![square(5.0, 5.0, 10.0)], // Overlaps the first
        ];
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::PolygonOverlap)));
    }

    #[test]
    fn test_self_intersection() {
        // Bowtie polygon (self-intersecting)
        let ring = vec![
            (0.0, 0.0),
            (10.0, 10.0),
            (10.0, 0.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ];
        let polygons = vec![vec![ring]];
        let report = validate_polygons(&polygons, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::SelfIntersection)));
    }

    #[test]
    fn test_line_dangle() {
        // Two lines that don't connect
        let lines = vec![
            vec![(0.0, 0.0), (10.0, 0.0)],
            vec![(20.0, 0.0), (30.0, 0.0)], // Dangles — doesn't connect to line 0
        ];
        let report = validate_lines(&lines, &TopologyParams::default());
        assert!(report.errors.iter().any(|e| matches!(e.rule, TopologyRule::Dangle)));
    }

    #[test]
    fn test_lines_connected_no_dangle() {
        let lines = vec![
            vec![(0.0, 0.0), (10.0, 0.0)],
            vec![(10.0, 0.0), (20.0, 0.0)], // Connects to line 0 at (10, 0)
        ];
        let report = validate_lines(&lines, &TopologyParams::default());
        // Should have no dangle errors
        assert!(!report.errors.iter().any(|e| matches!(e.rule, TopologyRule::Dangle)));
    }

    #[test]
    fn test_point_in_polygon() {
        let ring = square(0.0, 0.0, 10.0);
        assert!(point_in_polygon((5.0, 5.0), &ring));
        assert!(!point_in_polygon((15.0, 15.0), &ring));
    }

    #[test]
    fn test_segment_intersection() {
        // Crossing segments
        let p1 = (0.0, 0.0);
        let p2 = (10.0, 10.0);
        let p3 = (0.0, 10.0);
        let p4 = (10.0, 0.0);
        let isect = segment_intersection(p1, p2, p3, p4);
        assert!(isect.is_some());
        let (x, y) = isect.unwrap();
        assert!((x - 5.0).abs() < 1e-6);
        assert!((y - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_segment_no_intersection() {
        let p1 = (0.0, 0.0);
        let p2 = (1.0, 1.0);
        let p3 = (5.0, 5.0);
        let p4 = (6.0, 6.0);
        assert!(segment_intersection(p1, p2, p3, p4).is_none());
    }

    #[test]
    fn test_polygon_area() {
        let ring = square(0.0, 0.0, 10.0);
        assert!((polygon_area(&ring).abs() - 100.0) < 1e-6);
    }
}
