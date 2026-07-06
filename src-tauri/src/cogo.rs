// Coordinate Geometry (COGO) — Sprint 12.
//
// Pure-math COGO functions used by mining and marine surveyors:
//   - Inverse: bearing + distance between two known points
//   - Intersection: bearing-bearing, bearing-distance, distance-distance
//   - Offset: point perpendicular to a line at a fixed distance
//   - Curve fitting: radius from 3 points
//   - Area: shoelace + DMD (Double Meridian Distance) cross-check
//   - Subdivision: split a polygon along a line
//
// All angles in degrees (0=N, clockwise). All distances in meters.
// References:
//   - Davis, Foote, Anderson "Surveying Theory and Practice"
//   - Buckner, "A Treatise on Coordinate Geometry"
//   - standard surveying textbooks

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────────
// 2D point
// ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

// ──────────────────────────────────────────────────────────────────
// Inverse — bearing + distance between two points
// ──────────────────────────────────────────────────────────────────

/// Result of an inverse calculation: bearing (degrees, 0=N, clockwise)
/// and horizontal distance (meters) between two points.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct InverseResult {
    /// Bearing from `from` to `to`, degrees 0-360
    pub bearing_deg: f64,
    /// Horizontal distance, meters
    pub distance_m: f64,
}

/// Compute the bearing and distance from `from` to `to`.
///
/// Bearing: 0 = North, 90 = East, 180 = South, 270 = West.
/// Uses atan2(Δx, Δy) for quadrant correctness.
pub fn inverse(from: Point2D, to: Point2D) -> InverseResult {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let distance_m = (dx * dx + dy * dy).sqrt();
    let bearing_rad = dx.atan2(dy);
    let mut bearing_deg = bearing_rad.to_degrees();
    if bearing_deg < 0.0 {
        bearing_deg += 360.0;
    }
    InverseResult { bearing_deg, distance_m }
}

// ──────────────────────────────────────────────────────────────────
// Forward — compute a point from bearing + distance
// ──────────────────────────────────────────────────────────────────

/// Compute the point at `distance_m` from `from` on bearing `bearing_deg`.
pub fn forward(from: Point2D, bearing_deg: f64, distance_m: f64) -> Point2D {
    let theta = bearing_deg.to_radians();
    let dx = distance_m * theta.sin();
    let dy = distance_m * theta.cos();
    Point2D::new(from.x + dx, from.y + dy)
}

// ──────────────────────────────────────────────────────────────────
// Intersections
// ──────────────────────────────────────────────────────────────────

/// Intersection of two lines, each defined by a point + bearing.
///
/// Returns `None` if the lines are parallel (no intersection) or
/// coincident (infinite intersections).
pub fn intersect_bearing_bearing(
    p1: Point2D,
    bearing1_deg: f64,
    p2: Point2D,
    bearing2_deg: f64,
) -> Option<Point2D> {
    // Convert bearings to direction vectors
    let t1 = bearing1_deg.to_radians();
    let t2 = bearing2_deg.to_radians();
    // Direction vectors (dx, dy) where dy is north
    let d1x = t1.sin();
    let d1y = t1.cos();
    let d2x = t2.sin();
    let d2y = t2.cos();

    // Solve: p1 + s*d1 = p2 + t*d2
    // → s*d1 - t*d2 = p2 - p1
    // → [d1x  -d2x] [s]   [p2x - p1x]
    //   [d1y  -d2y] [t] = [p2y - p1y]
    let det = d1x * (-d2y) - (-d2x) * d1y;
    if det.abs() < 1e-12 {
        return None; // Parallel
    }
    let rhs_x = p2.x - p1.x;
    let rhs_y = p2.y - p1.y;
    let s = (rhs_x * (-d2y) - (-d2x) * rhs_y) / det;
    Some(Point2D::new(p1.x + s * d1x, p1.y + s * d1y))
}

/// Intersection of a line (point + bearing) and a circle (center + radius).
///
/// Returns up to two intersection points. Empty if no intersection.
pub fn intersect_bearing_circle(
    p: Point2D,
    bearing_deg: f64,
    center: Point2D,
    radius_m: f64,
) -> Vec<Point2D> {
    let theta = bearing_deg.to_radians();
    let dx = theta.sin();
    let dy = theta.cos();
    // Parametric: P = p + t * (dx, dy), t >= 0
    // |P - center|² = radius²
    // (p.x + t*dx - center.x)² + (p.y + t*dy - center.y)² = radius²
    let fx = p.x - center.x;
    let fy = p.y - center.y;
    let a = dx * dx + dy * dy; // = 1 for unit vector
    let b = 2.0 * (fx * dx + fy * dy);
    let c = fx * fx + fy * fy - radius_m * radius_m;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return vec![];
    }
    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    let mut result = vec![];
    if t1 >= 0.0 {
        result.push(Point2D::new(p.x + t1 * dx, p.y + t1 * dy));
    }
    if t2 >= 0.0 && (t1 - t2).abs() > 1e-9 {
        result.push(Point2D::new(p.x + t2 * dx, p.y + t2 * dy));
    }
    result
}

/// Intersection of two circles (center1, r1) and (center2, r2).
///
/// Returns 0, 1, or 2 intersection points.
pub fn intersect_circle_circle(
    c1: Point2D,
    r1: f64,
    c2: Point2D,
    r2: f64,
) -> Vec<Point2D> {
    let d = ((c2.x - c1.x).powi(2) + (c2.y - c1.y).powi(2)).sqrt();
    if d > r1 + r2 || d < (r1 - r2).abs() || d == 0.0 {
        return vec![];
    }
    // Distance from c1 to the chord midpoint
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    // Half chord length
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    // Chord midpoint
    let mx = c1.x + a * (c2.x - c1.x) / d;
    let my = c1.y + a * (c2.y - c1.y) / d;
    // Perpendicular offset
    let ox = h * (c2.y - c1.y) / d;
    let oy = -h * (c2.x - c1.x) / d;
    if h < 1e-9 {
        // Tangent — single point
        return vec![Point2D::new(mx, my)];
    }
    vec![
        Point2D::new(mx + ox, my + oy),
        Point2D::new(mx - ox, my - oy),
    ]
}

// ──────────────────────────────────────────────────────────────────
// Offset — point perpendicular to a line
// ──────────────────────────────────────────────────────────────────

/// Compute the point offset `distance_m` perpendicular to the line
/// from `a` to `b`. Positive distance = right side of line (looking
/// from a to b), negative = left.
pub fn offset_from_line(a: Point2D, b: Point2D, distance_m: f64) -> Point2D {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 {
        return a;
    }
    // Perpendicular unit vector (right side)
    // Right perpendicular of (dx, dy) is (dy, -dx) / len
    let perp_x = dy / len;
    let perp_y = -dx / len;
    // Offset from the midpoint for symmetry
    let mid = Point2D::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    Point2D::new(mid.x + distance_m * perp_x, mid.y + distance_m * perp_y)
}

/// Compute the foot of the perpendicular from `p` to the line through
/// `a` and `b` (the closest point on the line).
pub fn perpendicular_foot(p: Point2D, a: Point2D, b: Point2D) -> Point2D {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return a;
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq;
    Point2D::new(a.x + t * dx, a.y + t * dy)
}

// ──────────────────────────────────────────────────────────────────
// Curve fitting — radius from 3 points
// ──────────────────────────────────────────────────────────────────

/// Fit a circle through 3 points and return the center + radius.
///
/// Returns `None` if the points are collinear (no circle fits).
pub fn circle_from_3_points(p1: Point2D, p2: Point2D, p3: Point2D) -> Option<(Point2D, f64)> {
    // Perpendicular bisector method
    let ax = (p1.x + p2.x) / 2.0;
    let ay = (p1.y + p2.y) / 2.0;
    let bx = (p2.x + p3.x) / 2.0;
    let by = (p2.y + p3.y) / 2.0;

    // Direction of bisector 1: perpendicular to p1→p2
    let d1x = -(p2.y - p1.y);
    let d1y = p2.x - p1.x;
    let d2x = -(p3.y - p2.y);
    let d2y = p3.x - p2.x;

    // Intersect the two bisectors
    let det = d1x * (-d2y) - (-d2x) * d1y;
    if det.abs() < 1e-12 {
        return None; // Collinear
    }
    let rhs_x = bx - ax;
    let rhs_y = by - ay;
    let s = (rhs_x * (-d2y) - (-d2x) * rhs_y) / det;
    let center = Point2D::new(ax + s * d1x, ay + s * d1y);
    let radius = ((center.x - p1.x).powi(2) + (center.y - p1.y).powi(2)).sqrt();
    Some((center, radius))
}

// ──────────────────────────────────────────────────────────────────
// Area — shoelace + DMD cross-check
// ──────────────────────────────────────────────────────────────────

/// Compute polygon area using the shoelace formula.
pub fn area_shoelace(points: &[Point2D]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += points[i].x * points[j].y;
        sum -= points[j].x * points[i].y;
    }
    (sum / 2.0).abs()
}

/// Compute polygon area using the Double Meridian Distance (DMD) method.
///
/// This is the classical surveyor's method, used as a cross-check on
/// the shoelace formula. The two methods should agree to <1e-9 on the
/// same polygon.
pub fn area_dmd(points: &[Point2D]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    // Compute departures (ΔE) and latitudes (ΔN) for each course
    let mut courses = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let departure = points[j].x - points[i].x; // ΔE
        let latitude = points[j].y - points[i].y;  // ΔN
        courses.push((departure, latitude));
    }
    // DMD of first course = departure of first course / 2
    // DMD of subsequent course = DMD_prev + departure_prev + departure_current/2
    let mut dmd = courses[0].0 / 2.0;
    let mut double_area = dmd * courses[0].1;
    for i in 1..n {
        dmd = dmd + courses[i - 1].0 + courses[i].0 / 2.0;
        double_area += dmd * courses[i].1;
    }
    double_area.abs() / 2.0
}

// ──────────────────────────────────────────────────────────────────
// Subdivision — split a polygon along a line
// ──────────────────────────────────────────────────────────────────

/// Split a polygon along a line (defined by two points) into two parts.
///
/// Returns `(left_polygon, right_polygon)` where "left" is the part on
/// the left side of the line a→b and "right" is the other part.
/// Returns `None` if the line doesn't intersect the polygon.
pub fn split_polygon(
    polygon: &[Point2D],
    a: Point2D,
    b: Point2D,
) -> Option<(Vec<Point2D>, Vec<Point2D>)> {
    // Find all intersection points of the line with the polygon edges
    let n = polygon.len();
    if n < 3 {
        return None;
    }
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 {
        return None;
    }
    // Normal direction (perpendicular to line, pointing left)
    let nx = -dy / len;
    let ny = dx / len;

    // For each polygon vertex, classify as left (positive) or right (negative)
    let side = |p: Point2D| -> f64 {
        ((p.x - a.x) * nx + (p.y - a.y) * ny).signum()
    };

    let mut left = vec![];
    let mut right = vec![];

    for i in 0..n {
        let p1 = polygon[i];
        let p2 = polygon[(i + 1) % n];
        let s1 = side(p1);
        let s2 = side(p2);

        // Add p1 to the appropriate side
        if s1 >= 0.0 {
            left.push(p1);
        }
        if s1 <= 0.0 {
            right.push(p1);
        }

        // If the edge crosses the line, compute the intersection point
        if s1 * s2 < 0.0 {
            // Solve: p1 + t * (p2 - p1) lies on line a→b
            // Perpendicular distance from p1+t*(p2-p1) to line = 0
            let edge_dx = p2.x - p1.x;
            let edge_dy = p2.y - p1.y;
            let t = -((p1.x - a.x) * nx + (p1.y - a.y) * ny)
                / (edge_dx * nx + edge_dy * ny);
            let ix = p1.x + t * edge_dx;
            let iy = p1.y + t * edge_dy;
            let intersection = Point2D::new(ix, iy);
            left.push(intersection);
            right.push(intersection);
        }
    }

    if left.len() < 3 || right.len() < 3 {
        return None;
    }
    Some((left, right))
}

// ──────────────────────────────────────────────────────────────────
// Snell's law for refraction (used in hydrographic ray tracing)
// ──────────────────────────────────────────────────────────────────

/// Refract a ray at a Snell boundary.
///
/// `incident_deg` is the angle from the normal (degrees).
/// `v1` is the wave speed in medium 1, `v2` in medium 2.
/// Returns the refracted angle from the normal (degrees), or `None`
/// for total internal reflection.
pub fn snell_refract(incident_deg: f64, v1: f64, v2: f64) -> Option<f64> {
    let inc_rad = incident_deg.to_radians();
    let sin_refracted = (inc_rad.sin() * v2 / v1).abs();
    if sin_refracted > 1.0 {
        return None; // Total internal reflection
    }
    Some(sin_refracted.asin().to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverse_north() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(0.0, 100.0);
        let r = inverse(a, b);
        assert!((r.bearing_deg - 0.0).abs() < 1e-6);
        assert!((r.distance_m - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_inverse_east() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(100.0, 0.0);
        let r = inverse(a, b);
        assert!((r.bearing_deg - 90.0).abs() < 1e-6);
        assert!((r.distance_m - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_inverse_northeast() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(100.0, 100.0);
        let r = inverse(a, b);
        assert!((r.bearing_deg - 45.0).abs() < 1e-6);
        assert!((r.distance_m - 141.4214).abs() < 1e-3);
    }

    #[test]
    fn test_inverse_southwest() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(-100.0, -100.0);
        let r = inverse(a, b);
        assert!((r.bearing_deg - 225.0).abs() < 1e-6);
    }

    #[test]
    fn test_forward_round_trip() {
        let a = Point2D::new(1000.0, 2000.0);
        let bearing = 127.5;
        let distance = 350.0;
        let b = forward(a, bearing, distance);
        let r = inverse(a, b);
        assert!((r.bearing_deg - bearing).abs() < 1e-6);
        assert!((r.distance_m - distance).abs() < 1e-6);
    }

    #[test]
    fn test_intersect_bearing_bearing_perpendicular() {
        // Line 1: from (0,0) bearing 90° (East)
        // Line 2: from (100,0) bearing 180° (South)
        // Should intersect at (100, 0)... wait, line 1 goes through (0,0) east,
        // line 2 starts at (100, 0) going south. They intersect at (100, 0).
        let p1 = Point2D::new(0.0, 0.0);
        let p2 = Point2D::new(100.0, 50.0);
        let i = intersect_bearing_bearing(p1, 90.0, p2, 180.0).unwrap();
        assert!((i.x - 100.0).abs() < 1e-6);
        assert!((i.y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_intersect_bearing_bearing_parallel() {
        let p1 = Point2D::new(0.0, 0.0);
        let p2 = Point2D::new(0.0, 100.0);
        // Both bearings = 90° (East) → parallel
        assert!(intersect_bearing_bearing(p1, 90.0, p2, 90.0).is_none());
    }

    #[test]
    fn test_intersect_bearing_circle_two_points() {
        // Line from (0,0) bearing 0° (North), circle center (50, 50) r=50
        // The line x=0; circle (x-50)² + (y-50)² = 2500
        // 2500 + (y-50)² = 2500 → y = 50 (tangent, single point)
        let pts = intersect_bearing_circle(Point2D::new(0.0, 0.0), 0.0, Point2D::new(50.0, 50.0), 50.0);
        // Wait — actually the line x=0 doesn't reach the circle (which is tangent at (0,50)?)
        // Let me redo: circle center (50,50), r=50 → passes through (0,50) and (100,50)
        // Line x=0 (bearing 0° from origin) goes up the y-axis. It passes through (0,50).
        // (0-50)² + (y-50)² = 2500 → 2500 + (y-50)² = 2500 → y=50 (single tangent point)
        assert_eq!(pts.len(), 1);
        assert!((pts[0].x - 0.0).abs() < 1e-6);
        assert!((pts[0].y - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_intersect_bearing_circle_two_solutions() {
        // Line from (0,0) bearing 0° (North along y-axis), circle center (0, 100) r=50
        // (0-0)² + (y-100)² = 2500 → y = 100±50 → y=50 and y=150
        let pts = intersect_bearing_circle(Point2D::new(0.0, 0.0), 0.0, Point2D::new(0.0, 100.0), 50.0);
        assert_eq!(pts.len(), 2);
        // Both should be on the y-axis (x=0)
        for p in &pts {
            assert!(p.x.abs() < 1e-6);
        }
        // y values: 50 and 150
        let ys: Vec<f64> = pts.iter().map(|p| p.y).collect();
        assert!(ys.iter().any(|&y| (y - 50.0).abs() < 1e-6));
        assert!(ys.iter().any(|&y| (y - 150.0).abs() < 1e-6));
    }

    #[test]
    fn test_intersect_circle_circle_two_points() {
        // Circle 1: center (0,0) r=5
        // Circle 2: center (8,0) r=5
        // Distance = 8, both r=5 → intersect at (4, ±3)
        let pts = intersect_circle_circle(Point2D::new(0.0, 0.0), 5.0, Point2D::new(8.0, 0.0), 5.0);
        assert_eq!(pts.len(), 2);
        for p in &pts {
            assert!((p.x - 4.0).abs() < 1e-6);
            assert!((p.y.abs() - 3.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_intersect_circle_circle_no_intersection() {
        // Too far apart
        let pts = intersect_circle_circle(Point2D::new(0.0, 0.0), 1.0, Point2D::new(10.0, 0.0), 1.0);
        assert!(pts.is_empty());
    }

    #[test]
    fn test_intersect_circle_circle_tangent() {
        // Externally tangent
        let pts = intersect_circle_circle(Point2D::new(0.0, 0.0), 5.0, Point2D::new(10.0, 0.0), 5.0);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].x - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_offset_from_line_right() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(0.0, 100.0); // North line
        let p = offset_from_line(a, b, 50.0); // 50m to the right
        // Right of a north-pointing line is east (+x)
        assert!((p.x - 50.0).abs() < 1e-6);
        assert!((p.y - 50.0).abs() < 1e-6); // midpoint y
    }

    #[test]
    fn test_perpendicular_foot() {
        let p = Point2D::new(50.0, 100.0);
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(100.0, 0.0); // x-axis
        let foot = perpendicular_foot(p, a, b);
        assert!((foot.x - 50.0).abs() < 1e-6);
        assert!(foot.y.abs() < 1e-6);
    }

    #[test]
    fn test_circle_from_3_points() {
        // Three points on a unit circle centered at origin
        let p1 = Point2D::new(1.0, 0.0);
        let p2 = Point2D::new(0.0, 1.0);
        let p3 = Point2D::new(-1.0, 0.0);
        let (center, radius) = circle_from_3_points(p1, p2, p3).unwrap();
        assert!(center.x.abs() < 1e-6);
        assert!(center.y.abs() < 1e-6);
        assert!((radius - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_circle_from_3_collinear_points() {
        let p1 = Point2D::new(0.0, 0.0);
        let p2 = Point2D::new(1.0, 0.0);
        let p3 = Point2D::new(2.0, 0.0);
        assert!(circle_from_3_points(p1, p2, p3).is_none());
    }

    #[test]
    fn test_area_shoelace_square() {
        let square = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 0.0),
            Point2D::new(10.0, 10.0),
            Point2D::new(0.0, 10.0),
        ];
        assert!((area_shoelace(&square) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_area_dmd_matches_shoelace() {
        // Irregular polygon
        let polygon = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(50.0, 10.0),
            Point2D::new(80.0, 60.0),
            Point2D::new(30.0, 90.0),
            Point2D::new(-20.0, 50.0),
        ];
        let shoelace = area_shoelace(&polygon);
        let dmd = area_dmd(&polygon);
        assert!((shoelace - dmd).abs() < 1e-6, "shoelace={}, dmd={}", shoelace, dmd);
    }

    #[test]
    fn test_split_polygon() {
        // Square split horizontally
        let square = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(100.0, 0.0),
            Point2D::new(100.0, 100.0),
            Point2D::new(0.0, 100.0),
        ];
        // Split along y=50 (line from (0,50) to (100,50))
        let (left, right) = split_polygon(&square, Point2D::new(0.0, 50.0), Point2D::new(100.0, 50.0)).unwrap();
        // "Left" of a line going east (positive x) is north (positive y)
        // Actually: left of vector (100,0) is positive y. So left = upper half.
        let left_area = area_shoelace(&left);
        let right_area = area_shoelace(&right);
        // Each half should be 5000 m² (100 × 50)
        assert!((left_area - 5000.0).abs() < 1.0, "left_area = {}", left_area);
        assert!((right_area - 5000.0).abs() < 1.0, "right_area = {}", right_area);
    }

    #[test]
    fn test_snell_refract_normal_incidence() {
        // At normal incidence (0° from normal), refracted angle = 0°
        let r = snell_refract(0.0, 1500.0, 1600.0).unwrap();
        assert!(r.abs() < 1e-6);
    }

    #[test]
    fn test_snell_refract_total_internal_reflection() {
        // Going from slow (1500) to fast (3000) at high angle → TIR
        let r = snell_refract(60.0, 1500.0, 3000.0);
        assert!(r.is_none());
    }
}
