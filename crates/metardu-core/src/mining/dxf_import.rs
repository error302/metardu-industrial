// DXF TIN surface import and DEM rasterization.
//
// Reads a DXF file containing `3DFACE` entities and converts them into a
// triangulated irregular network (TIN). The TIN can then be rasterized to
// a regular-grid DEM by sampling each cell's centre against the triangle
// that contains it (barycentric interpolation).
//
// Uses the `dxf` crate's `Drawing::load_file` for parsing. The `3DFACE`
// entity's `first_corner` / `second_corner` / `third_corner` /
// `fourth_corner` are `dxf::Point` values (x, y, z: f64). A 3DFACE with
// `fourth_corner == third_corner` is a triangle; otherwise it's a quad
// that gets split into two triangles along the (p1, p3) diagonal.

use std::path::Path;

use dxf::Drawing;
use dxf::entities::EntityType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// A 3D triangle in geographic coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Triangle {
    pub p1: (f64, f64, f64),
    pub p2: (f64, f64, f64),
    pub p3: (f64, f64, f64),
}

impl Triangle {
    /// Planimetric (x, y) bounding box of the triangle.
    pub fn bbox_xy(&self) -> (f64, f64, f64, f64) {
        let xs = [self.p1.0, self.p2.0, self.p3.0];
        let ys = [self.p1.1, self.p2.1, self.p3.1];
        let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
        let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (min_x, min_y, max_x, max_y)
    }

    /// Barycentric interpolation of z at planimetric point (x, y).
    ///
    /// Returns `None` if (x, y) is outside the triangle (one of the
    /// barycentric weights is negative).
    pub fn interpolate_z(&self, x: f64, y: f64) -> Option<f64> {
        let (x1, y1, z1) = self.p1;
        let (x2, y2, z2) = self.p2;
        let (x3, y3, z3) = self.p3;
        let det = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3);
        if det.abs() < 1e-12 {
            return None;
        }
        let w1 = ((y2 - y3) * (x - x3) + (x3 - x2) * (y - y3)) / det;
        let w2 = ((y3 - y1) * (x - x3) + (x1 - x3) * (y - y3)) / det;
        let w3 = 1.0 - w1 - w2;
        // Allow a small tolerance for points exactly on an edge.
        if w1 < -1e-9 || w2 < -1e-9 || w3 < -1e-9 {
            return None;
        }
        Some(w1 * z1 + w2 * z2 + w3 * z3)
    }
}

/// A TIN surface extracted from a DXF file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfSurface {
    /// Triangles making up the TIN.
    pub triangles: Vec<Triangle>,
    /// 3D bounding box: (min_x, min_y, min_z, max_x, max_y, max_z).
    pub bounds: (f64, f64, f64, f64, f64, f64),
    /// Number of `3DFACE` entities in the source DXF.
    pub source_face_count: usize,
}

/// A regular-grid DEM rasterized from a `DxfSurface`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignDem {
    pub ncols: usize,
    pub nrows: usize,
    pub cell_size: f64,
    /// Geographic bounds: (min_x, min_y, max_x, max_y).
    pub bounds: (f64, f64, f64, f64),
    /// Elevation values, row-major: `[row * ncols + col]`.
    pub data: Vec<f64>,
    /// NODATA sentinel value.
    pub nodata_value: f64,
    /// Number of cells with valid data.
    pub valid_cells: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DxfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("DXF parse error: {0}")]
    Parse(#[from] dxf::DxfError),
    #[error("DXF file contains no 3DFACE entities")]
    NoFaces,
    #[error("cell size must be positive (got {0})")]
    InvalidCellSize(f64),
    #[error("computed grid is degenerate (zero rows or columns)")]
    DegenerateGrid,
}

/// Import a DXF file and extract its 3DFACE entities as a TIN surface.
pub fn import_dxf_surface(path: &Path) -> Result<DxfSurface, DxfError> {
    let drawing = Drawing::load_file(path)?;

    let mut triangles: Vec<Triangle> = Vec::new();
    let mut face_count = 0usize;

    for entity in drawing.entities() {
        let face = match &entity.specific {
            EntityType::Face3D(f) => f,
            _ => continue,
        };
        face_count += 1;
        let p1 = (face.first_corner.x, face.first_corner.y, face.first_corner.z);
        let p2 = (face.second_corner.x, face.second_corner.y, face.second_corner.z);
        let p3 = (face.third_corner.x, face.third_corner.y, face.third_corner.z);
        let p4 = (face.fourth_corner.x, face.fourth_corner.y, face.fourth_corner.z);

        // First triangle: p1, p2, p3
        triangles.push(Triangle { p1, p2, p3 });
        // If the fourth corner differs from the third, the 3DFACE is a
        // quad — add the second triangle along the (p1, p3) diagonal.
        let is_quad = (p4.0 - p3.0).abs() > 1e-9
            || (p4.1 - p3.1).abs() > 1e-9
            || (p4.2 - p3.2).abs() > 1e-9;
        if is_quad {
            triangles.push(Triangle { p1, p2: p3, p3: p4 });
        }
    }

    if triangles.is_empty() {
        return Err(DxfError::NoFaces);
    }

    // Compute 3D bounds.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for t in &triangles {
        for p in [t.p1, t.p2, t.p3] {
            min_x = min_x.min(p.0);
            max_x = max_x.max(p.0);
            min_y = min_y.min(p.1);
            max_y = max_y.max(p.1);
            min_z = min_z.min(p.2);
            max_z = max_z.max(p.2);
        }
    }

    Ok(DxfSurface {
        triangles,
        bounds: (min_x, min_y, min_z, max_x, max_y, max_z),
        source_face_count: face_count,
    })
}

/// Rasterize a `DxfSurface` to a regular-grid DEM using barycentric
/// interpolation.
///
/// For each grid cell, the cell's centre is tested against every triangle
/// whose planimetric bounding box contains it; the first hit wins and the
/// cell's elevation is set via barycentric interpolation. Cells covered
/// by no triangle are set to `nodata_value` (default -9999.0).
///
/// The outer loop over triangles is parallelised with `rayon` — each
/// triangle writes only to its own bounding-box cell range, so there are
/// no cross-thread write conflicts.
pub fn rasterize_dxf_to_dem(
    surface: &DxfSurface,
    cell_size: f64,
    bounds: Option<(f64, f64, f64, f64)>,
) -> Result<DesignDem, DxfError> {
    if cell_size <= 0.0 {
        return Err(DxfError::InvalidCellSize(cell_size));
    }
    if surface.triangles.is_empty() {
        return Err(DxfError::NoFaces);
    }

    let nodata = -9999.0;
    let (min_x, min_y, max_x, max_y) = bounds.unwrap_or_else(|| {
        (
            surface.bounds.0,
            surface.bounds.1,
            surface.bounds.3,
            surface.bounds.4,
        )
    });

    let ncols = (((max_x - min_x) / cell_size).ceil() as usize).max(1);
    let nrows = (((max_y - min_y) / cell_size).ceil() as usize).max(1);
    if ncols == 0 || nrows == 0 {
        return Err(DxfError::DegenerateGrid);
    }

    // Use a Mutex-protected accumulator per cell? No — we instead do two
    // passes: first pass (parallel) writes per-triangle contributions to
    // a thread-local Vec<(cell_idx, z)>; second pass (sequential) merges.
    // For typical TIN sizes (≤ a few thousand triangles) this is fast and
    // avoids locking overhead.
    let contributions: Vec<(usize, f64)> = surface
        .triangles
        .par_iter()
        .flat_map(|tri| {
            let (tmin_x, tmin_y, tmax_x, tmax_y) = tri.bbox_xy();
            let col_lo = (((tmin_x - min_x) / cell_size).floor() as i64).max(0) as usize;
            let col_hi = (((tmax_x - min_x) / cell_size).ceil() as i64).min(ncols as i64) as usize;
            let row_lo = (((tmin_y - min_y) / cell_size).floor() as i64).max(0) as usize;
            let row_hi = (((tmax_y - min_y) / cell_size).ceil() as i64).min(nrows as i64) as usize;

            let mut local = Vec::new();
            for row in row_lo..row_hi {
                for col in col_lo..col_hi {
                    let cx = min_x + (col as f64 + 0.5) * cell_size;
                    let cy = min_y + (row as f64 + 0.5) * cell_size;
                    if let Some(z) = tri.interpolate_z(cx, cy) {
                        local.push((row * ncols + col, z));
                    }
                }
            }
            local
        })
        .collect();

    let mut data = vec![nodata; ncols * nrows];
    let mut valid_cells = 0usize;
    for (idx, z) in contributions {
        if data[idx] == nodata {
            data[idx] = z;
            valid_cells += 1;
        }
        // First-writer-wins: subsequent triangles writing the same cell
        // are ignored. This matches the typical TIN layout where triangles
        // share edges but don't overlap interiors.
    }

    Ok(DesignDem {
        ncols,
        nrows,
        cell_size,
        bounds: (min_x, min_y, max_x, max_y),
        data,
        nodata_value: nodata,
        valid_cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dxf::entities::{Entity, Face3D};
    use dxf::Point;

    fn write_test_dxf(path: &Path, faces: &[(Point, Point, Point, Point)]) {
        let mut drawing = Drawing::new();
        for (p1, p2, p3, p4) in faces {
            let e = Entity::new(EntityType::Face3D(Face3D::new(
                p1.clone(),
                p2.clone(),
                p3.clone(),
                p4.clone(),
            )));
            drawing.add_entity(e);
        }
        drawing.save_file(path).unwrap();
    }

    #[test]
    fn test_import_dxf_single_triangle() {
        let tmp = tempfile::NamedTempFile::with_suffix(".dxf").unwrap();
        let faces = [(
            Point::new(0.0, 0.0, 100.0),
            Point::new(10.0, 0.0, 100.0),
            Point::new(0.0, 10.0, 100.0),
            Point::new(0.0, 10.0, 100.0), // fourth == third → triangle
        )];
        write_test_dxf(tmp.path(), &faces);
        let surface = import_dxf_surface(tmp.path()).unwrap();
        assert_eq!(surface.source_face_count, 1);
        assert_eq!(surface.triangles.len(), 1);
        assert!((surface.bounds.2 - 100.0).abs() < 1e-6);
        assert!((surface.bounds.5 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_import_dxf_quad_yields_two_triangles() {
        let tmp = tempfile::NamedTempFile::with_suffix(".dxf").unwrap();
        let faces = [(
            Point::new(0.0, 0.0, 100.0),
            Point::new(10.0, 0.0, 100.0),
            Point::new(10.0, 10.0, 110.0),
            Point::new(0.0, 10.0, 110.0),
        )];
        write_test_dxf(tmp.path(), &faces);
        let surface = import_dxf_surface(tmp.path()).unwrap();
        assert_eq!(surface.source_face_count, 1);
        assert_eq!(surface.triangles.len(), 2);
    }

    #[test]
    fn test_rasterize_flat_tin_yields_constant_z() {
        let tmp = tempfile::NamedTempFile::with_suffix(".dxf").unwrap();
        // Two triangles covering a 10x10 square at z=50.
        let faces = [
            (
                Point::new(0.0, 0.0, 50.0),
                Point::new(10.0, 0.0, 50.0),
                Point::new(10.0, 10.0, 50.0),
                Point::new(0.0, 10.0, 50.0),
            ),
        ];
        write_test_dxf(tmp.path(), &faces);
        let surface = import_dxf_surface(tmp.path()).unwrap();
        let dem = rasterize_dxf_to_dem(&surface, 1.0, None).unwrap();
        assert_eq!(dem.ncols, 10);
        assert_eq!(dem.nrows, 10);
        for v in &dem.data {
            assert!((v - 50.0).abs() < 1e-3, "expected ~50, got {}", v);
        }
        assert_eq!(dem.valid_cells, 100);
    }

    #[test]
    fn test_rasterize_tilted_tin_interpolates_z() {
        let tmp = tempfile::NamedTempFile::with_suffix(".dxf").unwrap();
        // Single triangle: corners at z=0, z=10, z=20 — z should vary
        // linearly across the surface.
        let faces = [(
            Point::new(0.0, 0.0, 0.0),
            Point::new(10.0, 0.0, 10.0),
            Point::new(0.0, 10.0, 20.0),
            Point::new(0.0, 10.0, 20.0), // triangle
        )];
        write_test_dxf(tmp.path(), &faces);
        let surface = import_dxf_surface(tmp.path()).unwrap();
        let dem = rasterize_dxf_to_dem(&surface, 1.0, None).unwrap();
        // Cell (0, 0) centre (0.5, 0.5) → barycentric interpolation should
        // give a positive z that's a convex combination of (0, 10, 20).
        let v = dem.data[0];
        assert!(v >= 0.0 && v <= 20.0, "z={} out of range", v);
    }

    #[test]
    fn test_invalid_cell_size_errors() {
        let tmp = tempfile::NamedTempFile::with_suffix(".dxf").unwrap();
        let faces = [(
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        )];
        write_test_dxf(tmp.path(), &faces);
        let surface = import_dxf_surface(tmp.path()).unwrap();
        let result = rasterize_dxf_to_dem(&surface, 0.0, None);
        assert!(matches!(result, Err(DxfError::InvalidCellSize(0.0))));
    }

    #[test]
    fn test_no_faces_errors() {
        let tmp = tempfile::NamedTempFile::with_suffix(".dxf").unwrap();
        let drawing = Drawing::new();
        drawing.save_file(tmp.path()).unwrap();
        let result = import_dxf_surface(tmp.path());
        assert!(matches!(result, Err(DxfError::NoFaces)));
    }

    #[test]
    fn test_triangle_barycentric_outside_returns_none() {
        let t = Triangle {
            p1: (0.0, 0.0, 0.0),
            p2: (10.0, 0.0, 0.0),
            p3: (0.0, 10.0, 0.0),
        };
        assert!(t.interpolate_z(-1.0, -1.0).is_none());
        assert!(t.interpolate_z(5.0, 5.0).is_some()); // inside the triangle
    }
}
