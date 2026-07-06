// COGO IPC commands — Sprint 12.
//
// Exposes the Coordinate Geometry functions to the frontend. Each COGO
// operation is a pure-math function that the dialog calls with user
// input and renders the result.

use crate::cogo::{
    circle_from_3_points, forward, intersect_bearing_bearing, intersect_bearing_circle,
    intersect_circle_circle, inverse, offset_from_line, perpendicular_foot, area_shoelace,
    area_dmd, split_polygon, snell_refract, Point2D,
};

#[tauri::command]
pub fn cogo_inverse_cmd(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> crate::cogo::InverseResult {
    inverse(Point2D::new(from_x, from_y), Point2D::new(to_x, to_y))
}

#[tauri::command]
pub fn cogo_forward_cmd(from_x: f64, from_y: f64, bearing_deg: f64, distance_m: f64) -> (f64, f64) {
    let p = forward(Point2D::new(from_x, from_y), bearing_deg, distance_m);
    (p.x, p.y)
}

#[tauri::command]
pub fn cogo_intersect_bearing_bearing_cmd(
    p1_x: f64, p1_y: f64, bearing1_deg: f64,
    p2_x: f64, p2_y: f64, bearing2_deg: f64,
) -> Option<(f64, f64)> {
    intersect_bearing_bearing(
        Point2D::new(p1_x, p1_y), bearing1_deg,
        Point2D::new(p2_x, p2_y), bearing2_deg,
    ).map(|p| (p.x, p.y))
}

#[tauri::command]
pub fn cogo_intersect_bearing_circle_cmd(
    px: f64, py: f64, bearing_deg: f64,
    cx: f64, cy: f64, radius_m: f64,
) -> Vec<(f64, f64)> {
    intersect_bearing_circle(
        Point2D::new(px, py), bearing_deg,
        Point2D::new(cx, cy), radius_m,
    ).into_iter().map(|p| (p.x, p.y)).collect()
}

#[tauri::command]
pub fn cogo_intersect_circle_circle_cmd(
    c1_x: f64, c1_y: f64, r1: f64,
    c2_x: f64, c2_y: f64, r2: f64,
) -> Vec<(f64, f64)> {
    intersect_circle_circle(
        Point2D::new(c1_x, c1_y), r1,
        Point2D::new(c2_x, c2_y), r2,
    ).into_iter().map(|p| (p.x, p.y)).collect()
}

#[tauri::command]
pub fn cogo_offset_from_line_cmd(
    ax: f64, ay: f64, bx: f64, by: f64, distance_m: f64,
) -> (f64, f64) {
    let p = offset_from_line(Point2D::new(ax, ay), Point2D::new(bx, by), distance_m);
    (p.x, p.y)
}

#[tauri::command]
pub fn cogo_perpendicular_foot_cmd(
    px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64,
) -> (f64, f64) {
    let p = perpendicular_foot(Point2D::new(px, py), Point2D::new(ax, ay), Point2D::new(bx, by));
    (p.x, p.y)
}

#[tauri::command]
pub fn cogo_circle_from_3_points_cmd(
    p1_x: f64, p1_y: f64, p2_x: f64, p2_y: f64, p3_x: f64, p3_y: f64,
) -> Option<((f64, f64), f64)> {
    circle_from_3_points(
        Point2D::new(p1_x, p1_y),
        Point2D::new(p2_x, p2_y),
        Point2D::new(p3_x, p3_y),
    ).map(|(center, radius)| ((center.x, center.y), radius))
}

#[tauri::command]
pub fn cogo_area_cmd(points: Vec<(f64, f64)>, method: String) -> f64 {
    let pts: Vec<Point2D> = points.into_iter().map(|(x, y)| Point2D::new(x, y)).collect();
    match method.as_str() {
        "dmd" => area_dmd(&pts),
        _ => area_shoelace(&pts), // default to shoelace
    }
}

#[tauri::command]
pub fn cogo_split_polygon_cmd(
    polygon: Vec<(f64, f64)>,
    ax: f64, ay: f64, bx: f64, by: f64,
) -> Option<(Vec<(f64, f64)>, Vec<(f64, f64)>)> {
    let poly: Vec<Point2D> = polygon.into_iter().map(|(x, y)| Point2D::new(x, y)).collect();
    split_polygon(&poly, Point2D::new(ax, ay), Point2D::new(bx, by))
        .map(|(left, right)| {
            (
                left.into_iter().map(|p| (p.x, p.y)).collect(),
                right.into_iter().map(|p| (p.x, p.y)).collect(),
            )
        })
}

#[tauri::command]
pub fn cogo_snell_refract_cmd(incident_deg: f64, v1: f64, v2: f64) -> Option<f64> {
    snell_refract(incident_deg, v1, v2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cogo_inverse_ipc() {
        let r = cogo_inverse_cmd(0.0, 0.0, 100.0, 100.0);
        assert!((r.bearing_deg - 45.0).abs() < 1e-6);
    }

    #[test]
    fn test_cogo_area_shoelace() {
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!((cogo_area_cmd(square, "shoelace".to_string()) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_cogo_area_dmd() {
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!((cogo_area_cmd(square, "dmd".to_string()) - 100.0).abs() < 1e-6);
    }
}
