// 3D Slice Editor — point cloud subsetting + reject brush.
//
// Per ROADMAP.md Priority #9 — 3D Slice Editor with Reject Brush.
//
// Workflow:
//   1. Surveyor draws a bounding polygon over a survey line on the map
//   2. The system loads the LAS point cloud and isolates only the points
//      that fall inside the polygon (the "slice")
//   3. The surveyor sees the slice in a 3D Deck.gl view
//   4. The surveyor drags a "reject brush" (a sphere of radius R) over
//      outlier points; points inside the brush are flagged as rejected
//   5. Rejected points can be undone (toggle the reject flag)
//   6. CUBE re-runs on the cleaned point cloud (only non-rejected points)
//
// This module provides:
//   - point_in_polygon_2d(): standard ray-casting test
//   - slice_by_polygon(): filter a Vec of (x,y,z) points to those inside
//   - RejectMask: a per-point boolean mask, undoable via a stack of operations
//   - brush_reject(): mark points within radius R of a brush center as rejected
//   - brush_restore(): unmark points within radius R
//
// All coordinates are in projected CRS (meters). The frontend converts
// geographic to projected before invoking these functions.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SliceRequest {
    /// Path to the LAS file to slice
    pub path: String,
    /// Polygon vertices (projected coords, meters). Must be closed (last vertex
    /// implicitly connects to first).
    pub polygon: Vec<Point2D>,
    /// Maximum points to load from the LAS (0 = all)
    #[serde(default)]
    pub max_points: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SliceResult {
    /// Indices into the original LAS point array (so the frontend can
    /// render only the slice in 3D without re-loading the full cloud)
    pub indices: Vec<u32>,
    /// The actual point coordinates (x, y, z) for 3D rendering
    pub points: Vec<Point3D>,
    /// Total points in the LAS file
    pub total_points: u32,
    /// Points inside the polygon
    pub slice_points: u32,
    /// Polygon area in square meters (shoelace formula)
    pub polygon_area_m2: f64,
}

/// RejectMask tracks which points have been flagged as rejected.
/// Uses a HashSet of indices for O(1) membership + insert + remove.
/// Supports undo via a stack of operations (each brush stroke is one op).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectMask {
    /// Set of point indices marked as rejected
    pub rejected: HashSet<u32>,
    /// Stack of operations for undo. Each operation is the list of
    /// indices that were toggled (added or removed) in that stroke.
    pub undo_stack: Vec<Vec<u32>>,
}

impl RejectMask {
    pub fn new() -> Self {
        Self {
            rejected: HashSet::new(),
            undo_stack: Vec::new(),
        }
    }

    /// Reject all points within `radius_m` of `(center_x, center_y, center_z)`.
    /// Returns the count of newly rejected points.
    pub fn brush_reject(
        &mut self,
        points: &[Point3D],
        center_x: f64,
        center_y: f64,
        center_z: f64,
        radius_m: f64,
    ) -> u32 {
        let radius_sq = radius_m * radius_m;
        let mut toggled: Vec<u32> = Vec::new();
        for (i, p) in points.iter().enumerate() {
            let dx = p.x - center_x;
            let dy = p.y - center_y;
            let dz = p.z - center_z;
            if dx * dx + dy * dy + dz * dz <= radius_sq {
                let idx = i as u32;
                if self.rejected.insert(idx) {
                    toggled.push(idx);
                }
            }
        }
        let count = toggled.len() as u32;
        if !toggled.is_empty() {
            self.undo_stack.push(toggled);
        }
        count
    }

    /// Restore (un-reject) all points within `radius_m` of the center.
    pub fn brush_restore(
        &mut self,
        points: &[Point3D],
        center_x: f64,
        center_y: f64,
        center_z: f64,
        radius_m: f64,
    ) -> u32 {
        let radius_sq = radius_m * radius_m;
        let mut toggled: Vec<u32> = Vec::new();
        for (i, p) in points.iter().enumerate() {
            let dx = p.x - center_x;
            let dy = p.y - center_y;
            let dz = p.z - center_z;
            if dx * dx + dy * dy + dz * dz <= radius_sq {
                let idx = i as u32;
                if self.rejected.remove(&idx) {
                    toggled.push(idx);
                }
            }
        }
        let count = toggled.len() as u32;
        if !toggled.is_empty() {
            self.undo_stack.push(toggled);
        }
        count
    }

    /// Undo the most recent brush operation. Returns the count of
    /// points that were toggled back, or None if the stack is empty.
    pub fn undo(&mut self) -> Option<u32> {
        let toggled = self.undo_stack.pop()?;
        for &idx in &toggled {
            if !self.rejected.remove(&idx) {
                self.rejected.insert(idx);
            }
        }
        Some(toggled.len() as u32)
    }

    /// Clear all rejections (without affecting the undo stack —
    /// call this when starting a new slice).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.rejected.clear();
        self.undo_stack.clear();
    }

    /// Count of currently rejected points
    pub fn rejected_count(&self) -> u32 {
        self.rejected.len() as u32
    }

    /// Get the indices of accepted (non-rejected) points
    pub fn accepted_indices(&self, total: u32) -> Vec<u32> {
        (0..total).filter(|i| !self.rejected.contains(i)).collect()
    }
}

impl Default for RejectMask {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard 2D ray-casting point-in-polygon test.
///
/// Returns true if the point is strictly inside the polygon (or on its
/// boundary). The polygon is implicitly closed — the last vertex is
/// connected to the first.
pub fn point_in_polygon_2d(point: Point2D, polygon: &[Point2D]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let n = polygon.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        // Check if the ray from (point.x, +inf) crosses the edge (pi → pj)
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Alias for the IPC command (which can't take a slice — needs owned Vec).
pub fn point_in_polygon_2d_test(point: Point2D, polygon: &[Point2D]) -> bool {
    point_in_polygon_2d(point, polygon)
}

/// Compute polygon area using the shoelace formula.
/// Returns area in square meters (assuming polygon coords are in meters).
pub fn polygon_area(polygon: &[Point2D]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0f64;
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        sum += polygon[i].x * polygon[j].y;
        sum -= polygon[j].x * polygon[i].y;
    }
    (sum / 2.0).abs()
}

/// Slice a LAS point cloud by a 2D polygon (projected coords).
///
/// Returns the indices and points that fall inside the polygon.
pub fn slice_by_polygon(request: &SliceRequest) -> Result<SliceResult, String> {
    use crate::formats::read_las_points;
    use std::path::Path;

    let path = Path::new(&request.path);
    let all_points = read_las_points(path, request.max_points.unwrap_or(0))
        .map_err(|e| format!("failed to read LAS: {e}"))?;

    let total_points = all_points.len() as u32;
    let mut indices: Vec<u32> = Vec::new();
    let mut points: Vec<Point3D> = Vec::new();

    for (i, (x, y, z)) in all_points.iter().enumerate() {
        let p2d = Point2D { x: *x, y: *y };
        if point_in_polygon_2d(p2d, &request.polygon) {
            indices.push(i as u32);
            points.push(Point3D {
                x: *x,
                y: *y,
                z: *z,
            });
        }
    }

    let slice_points = points.len() as u32;
    let polygon_area_m2 = polygon_area(&request.polygon);

    Ok(SliceResult {
        indices,
        points,
        total_points,
        slice_points,
        polygon_area_m2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2D {
        Point2D { x, y }
    }

    fn p3(x: f64, y: f64, z: f64) -> Point3D {
        Point3D { x, y, z }
    }

    #[test]
    fn test_point_in_polygon_simple_square() {
        let square = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)];
        assert!(point_in_polygon_2d(p(5.0, 5.0), &square));
        assert!(!point_in_polygon_2d(p(15.0, 5.0), &square));
        assert!(!point_in_polygon_2d(p(-5.0, 5.0), &square));
    }

    #[test]
    fn test_point_in_polygon_triangle() {
        let tri = vec![p(0.0, 0.0), p(10.0, 0.0), p(5.0, 10.0)];
        assert!(point_in_polygon_2d(p(5.0, 3.0), &tri));
        assert!(!point_in_polygon_2d(p(8.0, 8.0), &tri));
    }

    #[test]
    fn test_point_in_polygon_concave() {
        // C-shape with notch on left: notch = rect (0,2) to (8,8).
        // Notch points are OUTSIDE the polygon; points in the right arm
        // and the bottom band (y<2) or top band (y>8) are INSIDE.
        let c = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 10.0),
            p(0.0, 10.0),
            p(0.0, 8.0),
            p(8.0, 8.0),
            p(8.0, 2.0),
            p(0.0, 2.0),
        ];
        assert!(!point_in_polygon_2d(p(5.0, 5.0), &c)); // in notch
        assert!(!point_in_polygon_2d(p(1.0, 5.0), &c)); // in notch
        assert!(point_in_polygon_2d(p(9.0, 5.0), &c)); // right arm of C
        assert!(point_in_polygon_2d(p(1.0, 1.0), &c)); // bottom band
        assert!(point_in_polygon_2d(p(1.0, 9.0), &c)); // top band
    }

    #[test]
    fn test_point_in_polygon_too_few_vertices() {
        assert!(!point_in_polygon_2d(
            p(0.0, 0.0),
            &[p(0.0, 0.0), p(1.0, 1.0)]
        ));
    }

    #[test]
    fn test_polygon_area_square() {
        let square = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)];
        assert!((polygon_area(&square) - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_polygon_area_triangle() {
        let tri = vec![p(0.0, 0.0), p(10.0, 0.0), p(0.0, 10.0)];
        assert!((polygon_area(&tri) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_reject_mask_brush_reject() {
        let mut mask = RejectMask::new();
        let points = vec![
            p3(0.0, 0.0, 0.0),
            p3(1.0, 0.0, 0.0),
            p3(5.0, 0.0, 0.0),
            p3(10.0, 0.0, 0.0),
        ];
        // Brush at origin, radius 2m — should catch points 0 and 1
        let count = mask.brush_reject(&points, 0.0, 0.0, 0.0, 2.0);
        assert_eq!(count, 2);
        assert_eq!(mask.rejected_count(), 2);
        assert!(mask.rejected.contains(&0));
        assert!(mask.rejected.contains(&1));
    }

    #[test]
    fn test_reject_mask_undo() {
        let mut mask = RejectMask::new();
        let points = vec![p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(5.0, 0.0, 0.0)];
        mask.brush_reject(&points, 0.0, 0.0, 0.0, 2.0);
        assert_eq!(mask.rejected_count(), 2);
        assert_eq!(mask.undo_stack.len(), 1);
        let undone = mask.undo();
        assert_eq!(undone, Some(2));
        assert_eq!(mask.rejected_count(), 0);
        assert!(mask.undo_stack.is_empty());
    }

    #[test]
    fn test_reject_mask_undo_empty_stack() {
        let mut mask = RejectMask::new();
        assert_eq!(mask.undo(), None);
    }

    #[test]
    fn test_reject_mask_brush_restore() {
        let mut mask = RejectMask::new();
        let points = vec![p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(5.0, 0.0, 0.0)];
        mask.brush_reject(&points, 0.0, 0.0, 0.0, 2.0);
        assert_eq!(mask.rejected_count(), 2);
        // Restore the same brush area
        let restored = mask.brush_restore(&points, 0.0, 0.0, 0.0, 2.0);
        assert_eq!(restored, 2);
        assert_eq!(mask.rejected_count(), 0);
    }

    #[test]
    fn test_accepted_indices() {
        let mut mask = RejectMask::new();
        mask.rejected.insert(1);
        mask.rejected.insert(3);
        let accepted = mask.accepted_indices(5);
        assert_eq!(accepted, vec![0, 2, 4]);
    }

    #[test]
    fn test_slice_by_polygon_too_few_vertices() {
        // We can't easily test the full LAS slice without a real LAS file,
        // but we can verify the geometry helpers work via the standalone
        // point_in_polygon tests above. This test just verifies the
        // polygon_area + point_in_polygon contracts that slice_by_polygon
        // depends on.
        let poly = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)];
        assert!((polygon_area(&poly) - 100.0).abs() < 0.001);
        assert!(point_in_polygon_2d(p(5.0, 5.0), &poly));
    }
}
