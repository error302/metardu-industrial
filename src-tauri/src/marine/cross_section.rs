// Cross-section profiler — Revenue Feature #8.
//
// Generates a series of cross-sections perpendicular to a user-drawn
// centerline, sampled from a DEM surface. Used by port engineers to
// verify dredged channels meet design specifications.
//
// Workflow:
//   1. User draws centerline on map (Polyline in projected coordinates)
//   2. User specifies cross-section spacing (e.g., 50m) and half-width
//      (e.g., 25m to each side of centerline)
//   3. Module walks the centerline at `spacing` intervals, extracts a
//      perpendicular cross-section of `half_width` on each side
//   4. If a design surface is provided, the surveyed-vs-design profile
//      is computed and under-dredge areas are highlighted
//   5. Output is a Vec<CrossSection> suitable for rendering in the
//      frontend and for embedding in a branded PDF report
//
// All inputs are in METERS (projected coordinates). The caller is
// responsible for converting geographic coordinates to a projected CRS.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    fn distance_to(&self, other: &Point2D) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrossSectionRequest {
    /// Centerline vertices in projected coordinates (meters). Length >= 2.
    pub centerline: Vec<Point2D>,
    /// Spacing between cross-sections along the centerline (meters)
    pub spacing_m: f64,
    /// Half-width of each cross-section (meters). Total width = 2 × half_width.
    pub half_width_m: f64,
    /// Sampling resolution along each cross-section (meters). Default 1m.
    pub sample_resolution_m: f64,
    /// Path to surveyed DEM GeoTIFF
    #[serde(rename = "surveyPath")]
    pub survey_path: String,
    /// Optional path to design DEM GeoTIFF. If provided, the cross-section
    /// report includes surveyed-vs-design comparison.
    #[serde(rename = "designPath")]
    pub design_path: Option<String>,
    /// Channel design depth (meters) — only used if design_path is None
    /// and only for the under-dredge highlight calculation.
    #[serde(rename = "designDepth")]
    pub design_depth: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossSectionPoint {
    /// Distance from centerline (meters). Negative = left, positive = right.
    pub offset_m: f64,
    /// Distance along centerline from start (meters)
    pub chainage_m: f64,
    /// Surveyed elevation/depth at this point (meters). NaN if outside DEM.
    pub survey_z: f64,
    /// Design elevation/depth at this point (meters). NaN if no design.
    pub design_z: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossSection {
    /// Sequential index (0-based)
    pub index: usize,
    /// Chainage along centerline (meters from start)
    pub chainage_m: f64,
    /// Center point of this cross-section (projected coords)
    pub center: Point2DSer,
    /// Sampled points along the cross-section (left to right)
    pub points: Vec<CrossSectionPoint>,
    /// Under-dredge area (m²) — where survey > design (material left)
    pub under_dredge_area: f64,
    /// Over-dredge area (m²) — where survey < design - tolerance
    pub over_dredge_area: f64,
    /// Maximum under-dredge depth at this section (meters)
    pub max_under_dredge: f64,
    /// True if any point has survey > design (under-dredge detected)
    pub has_under_dredge: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Point2DSer {
    pub x: f64,
    pub y: f64,
}

impl From<Point2D> for Point2DSer {
    fn from(p: Point2D) -> Self {
        Self { x: p.x, y: p.y }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossSectionReport {
    /// Total centerline length (meters)
    pub total_length_m: f64,
    /// Number of cross-sections generated
    pub n_sections: usize,
    /// Cross-section spacing used (meters)
    pub spacing_m: f64,
    /// Half-width used (meters)
    pub half_width_m: f64,
    /// All cross-sections
    pub sections: Vec<CrossSection>,
    /// Summary statistics across all sections
    pub summary: CrossSectionSummary,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CrossSectionSummary {
    /// Total under-dredge area across all sections (m²)
    pub total_under_dredge_area: f64,
    /// Total over-dredge area across all sections (m²)
    pub total_over_dredge_area: f64,
    /// Maximum under-dredge depth across all sections (meters)
    pub max_under_dredge_depth: f64,
    /// Number of sections with any under-dredge
    pub sections_with_under_dredge: usize,
    /// Number of sections fully compliant (no under-dredge)
    pub compliant_sections: usize,
    /// Compliance percentage (compliant / total × 100)
    pub compliance_pct: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum CrossSectionError {
    #[error("centerline must have at least 2 points, got {0}")]
    TooFewPoints(usize),
    #[error("spacing must be positive: got {0}")]
    InvalidSpacing(f64),
    #[error("half-width must be positive: got {0}")]
    InvalidHalfWidth(f64),
    #[error("sample resolution must be positive: got {0}")]
    InvalidResolution(f64),
    #[error("failed to read survey DEM: {0}")]
    SurveyRead(String),
    #[error("failed to read design DEM: {0}")]
    DesignRead(String),
}

/// Compute cross-sections along a centerline.
///
/// The caller is responsible for converting geographic coordinates to
/// a projected CRS before passing them in (use the `transform_coords`
/// IPC command).
///
/// DEM sampling uses bilinear interpolation. NaN is returned for points
/// outside the DEM extent.
pub fn compute_cross_sections(
    request: &CrossSectionRequest,
) -> Result<CrossSectionReport, CrossSectionError> {
    if request.centerline.len() < 2 {
        return Err(CrossSectionError::TooFewPoints(request.centerline.len()));
    }
    if request.spacing_m <= 0.0 {
        return Err(CrossSectionError::InvalidSpacing(request.spacing_m));
    }
    if request.half_width_m <= 0.0 {
        return Err(CrossSectionError::InvalidHalfWidth(request.half_width_m));
    }
    let resolution = if request.sample_resolution_m > 0.0 {
        request.sample_resolution_m
    } else {
        1.0
    };

    // Load DEMs
    let survey_dem = DemSampler::open(&request.survey_path)
        .map_err(CrossSectionError::SurveyRead)?;
    let design_dem = request.design_path.as_ref()
        .map(|p| DemSampler::open(p).map_err(CrossSectionError::DesignRead))
        .transpose()?;
    let flat_design_depth = request.design_depth;

    // Walk centerline and generate cross-sections
    let total_length = total_centerline_length(&request.centerline);
    let mut sections = Vec::new();
    let mut chainage = 0.0;
    let mut section_idx = 0usize;

    while chainage <= total_length {
        let center = point_at_chainage(&request.centerline, chainage);
        let tangent = tangent_at_chainage(&request.centerline, chainage);
        // Perpendicular direction (rotate tangent 90°)
        let perp = Point2D { x: -tangent.y, y: tangent.x };

        let mut points = Vec::new();
        let mut under_dredge_area = 0.0f64;
        let mut over_dredge_area = 0.0f64;
        let mut max_under_dredge = 0.0f64;
        let mut has_under_dredge = false;

        let mut offset = -request.half_width_m;
        while offset <= request.half_width_m + 1e-9 {
            let sample_x = center.x + perp.x * offset;
            let sample_y = center.y + perp.y * offset;
            let survey_z = survey_dem.sample_bilinear(sample_x, sample_y);
            let design_z = if let Some(ref dd) = design_dem {
                dd.sample_bilinear(sample_x, sample_y)
            } else if let Some(d) = flat_design_depth {
                d
            } else {
                f64::NAN
            };

            // Under-dredge calculation: survey is shallower than design
            // (i.e., material left above design grade). For depths positive
            // downward: survey < design means under-dredge.
            if !survey_z.is_nan() && !design_z.is_nan() {
                let diff = design_z - survey_z; // positive = under-dredge (need to dig more)
                if diff > 0.0 {
                    under_dredge_area += diff * resolution;
                    if diff > max_under_dredge {
                        max_under_dredge = diff;
                    }
                    has_under_dredge = true;
                } else if diff < -0.3 {
                    // Over-dredge: more than 0.3m below design (typical tolerance)
                    over_dredge_area += (-diff - 0.3) * resolution;
                }
            }

            points.push(CrossSectionPoint {
                offset_m: offset,
                chainage_m: chainage,
                survey_z,
                design_z,
            });

            offset += resolution;
        }

        sections.push(CrossSection {
            index: section_idx,
            chainage_m: chainage,
            center: center.into(),
            points,
            under_dredge_area,
            over_dredge_area,
            max_under_dredge,
            has_under_dredge,
        });

        section_idx += 1;
        chainage += request.spacing_m;
    }

    // Summary
    let mut summary = CrossSectionSummary::default();
    for s in &sections {
        summary.total_under_dredge_area += s.under_dredge_area;
        summary.total_over_dredge_area += s.over_dredge_area;
        if s.max_under_dredge > summary.max_under_dredge_depth {
            summary.max_under_dredge_depth = s.max_under_dredge;
        }
        if s.has_under_dredge {
            summary.sections_with_under_dredge += 1;
        } else {
            summary.compliant_sections += 1;
        }
    }
    if !sections.is_empty() {
        summary.compliance_pct = (summary.compliant_sections as f64 / sections.len() as f64) * 100.0;
    }

    Ok(CrossSectionReport {
        total_length_m: total_length,
        n_sections: sections.len(),
        spacing_m: request.spacing_m,
        half_width_m: request.half_width_m,
        sections,
        summary,
    })
}

// ──────────────────────────────────────────────────────────────────
// Geometry helpers

fn total_centerline_length(line: &[Point2D]) -> f64 {
    let mut total = 0.0;
    for i in 1..line.len() {
        total += line[i - 1].distance_to(&line[i]);
    }
    total
}

/// Get the point at a given chainage along the polyline.
fn point_at_chainage(line: &[Point2D], mut chainage: f64) -> Point2D {
    if chainage <= 0.0 {
        return line[0];
    }
    for i in 1..line.len() {
        let seg_len = line[i - 1].distance_to(&line[i]);
        if chainage <= seg_len {
            let t = chainage / seg_len.max(1e-9);
            return Point2D {
                x: line[i - 1].x + t * (line[i].x - line[i - 1].x),
                y: line[i - 1].y + t * (line[i].y - line[i - 1].y),
            };
        }
        chainage -= seg_len;
    }
    *line.last().unwrap()
}

/// Get the unit tangent vector at a given chainage.
fn tangent_at_chainage(line: &[Point2D], mut chainage: f64) -> Point2D {
    if chainage <= 0.0 {
        let dx = line[1].x - line[0].x;
        let dy = line[1].y - line[0].y;
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        return Point2D { x: dx / len, y: dy / len };
    }
    for i in 1..line.len() {
        let seg_len = line[i - 1].distance_to(&line[i]);
        if chainage <= seg_len {
            let dx = line[i].x - line[i - 1].x;
            let dy = line[i].y - line[i - 1].y;
            let len = (dx * dx + dy * dy).sqrt().max(1e-9);
            return Point2D { x: dx / len, y: dy / len };
        }
        chainage -= seg_len;
    }
    let n = line.len();
    let dx = line[n - 1].x - line[n - 2].x;
    let dy = line[n - 1].y - line[n - 2].y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    Point2D { x: dx / len, y: dy / len }
}

// ──────────────────────────────────────────────────────────────────
// DEM sampler

/// Lightweight GeoTIFF sampler. Reads the entire grid into memory and
/// provides bilinear interpolation. Reuses the existing pure-Rust
/// GeoTIFF parser from `formats::geotiff`.
struct DemSampler {
    grid: Vec<f64>,
    width: usize,
    height: usize,
    /// World coordinates of the top-left pixel center
    origin_x: f64,
    origin_y: f64,
    /// Pixel size in world units (meters for projected DEMs)
    pixel_w: f64,
    pixel_h: f64,
}

impl DemSampler {
    fn open(path: &str) -> Result<Self, String> {
        use crate::commands::mining::{derive_cell_meters, read_dem_grid};
        use crate::formats::read_geotiff_header;
        use std::path::Path;
        let p = Path::new(path);
        let header = read_geotiff_header(p).map_err(|e| e.to_string())?;
        let grid = read_dem_grid(p, &header).map_err(|e| e.to_string())?;
        let (pix_w, pix_h) = derive_cell_meters(&header);

        // Tie-point: assume standard GeoTIFF ModelTiepointTag (0,0,0) → (origin_x, origin_y)
        // For Phase 1 we approximate: top-left pixel center is at (tiepoint_x + pix_w/2, tiepoint_y - pix_h/2)
        let (origin_x, origin_y) = if let Some(tp) = header.model_tiepoint.as_ref() {
            (tp[3] + pix_w / 2.0, tp[4] - pix_h / 2.0)
        } else {
            // No tiepoint — fall back to 0,0 origin
            (0.0, 0.0)
        };

        Ok(Self {
            grid,
            width: header.width as usize,
            height: header.length as usize,
            origin_x,
            origin_y,
            pixel_w: pix_w,
            pixel_h: pix_h,
        })
    }

    /// Bilinear interpolation. Returns NaN if outside the DEM extent.
    fn sample_bilinear(&self, x: f64, y: f64) -> f64 {
        // Convert world coords to pixel coords (0-indexed, fractional)
        let px = (x - self.origin_x) / self.pixel_w;
        let py = (self.origin_y - y) / self.pixel_h; // Y flipped (raster rows go down)

        if px < 0.0 || py < 0.0 || px >= self.width as f64 - 1.0 || py >= self.height as f64 - 1.0 {
            return f64::NAN;
        }

        let x0 = px.floor() as usize;
        let y0 = py.floor() as usize;
        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let fx = px - x0 as f64;
        let fy = py - y0 as f64;

        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        // If any neighbor is NaN, return NaN (don't interpolate across nodata)
        if v00.is_nan() || v10.is_nan() || v01.is_nan() || v11.is_nan() {
            return f64::NAN;
        }

        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;
        v0 * (1.0 - fy) + v1 * fy
    }

    fn get(&self, x: usize, y: usize) -> f64 {
        if x >= self.width || y >= self.height {
            return f64::NAN;
        }
        self.grid[y * self.width + x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2D {
        Point2D { x, y }
    }

    #[test]
    fn test_total_centerline_length() {
        let line = vec![p(0.0, 0.0), p(3.0, 0.0), p(3.0, 4.0)];
        assert!((total_centerline_length(&line) - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_point_at_chainage_simple() {
        let line = vec![p(0.0, 0.0), p(10.0, 0.0)];
        let pt = point_at_chainage(&line, 3.0);
        assert!((pt.x - 3.0).abs() < 0.001);
        assert!((pt.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_point_at_chainage_multi_segment() {
        let line = vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0)];
        // 7m along = 2m into the second segment (vertical)
        let pt = point_at_chainage(&line, 7.0);
        assert!((pt.x - 5.0).abs() < 0.001);
        assert!((pt.y - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_tangent_at_start() {
        let line = vec![p(0.0, 0.0), p(10.0, 0.0)];
        let t = tangent_at_chainage(&line, 0.0);
        assert!((t.x - 1.0).abs() < 0.001);
        assert!((t.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_tangent_perpendicular() {
        let line = vec![p(0.0, 0.0), p(10.0, 0.0)];
        let t = tangent_at_chainage(&line, 5.0);
        let perp = Point2D { x: -t.y, y: t.x };
        // Perpendicular should point in +Y direction
        assert!((perp.x - 0.0).abs() < 0.001);
        assert!((perp.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_too_few_points() {
        let req = CrossSectionRequest {
            centerline: vec![p(0.0, 0.0)],
            spacing_m: 10.0,
            half_width_m: 5.0,
            sample_resolution_m: 1.0,
            survey_path: "/nonexistent.tif".into(),
            design_path: None,
            design_depth: None,
        };
        let r = compute_cross_sections(&req);
        assert!(r.is_err());
    }

    #[test]
    fn test_invalid_spacing() {
        let req = CrossSectionRequest {
            centerline: vec![p(0.0, 0.0), p(10.0, 0.0)],
            spacing_m: 0.0,
            half_width_m: 5.0,
            sample_resolution_m: 1.0,
            survey_path: "/nonexistent.tif".into(),
            design_path: None,
            design_depth: None,
        };
        let r = compute_cross_sections(&req);
        assert!(r.is_err());
    }
}
