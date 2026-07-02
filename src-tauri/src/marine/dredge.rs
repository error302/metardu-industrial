// Dredge pay-volume audit — Revenue Feature #2.
//
// Categorizes each cell of a post-dredge survey grid into one of four
// contract-relevant buckets, using the design template as the reference.
//
// HYDROGRAPHIC CONVENTION: depths are POSITIVE DOWNWARD. A pre-dredge
// depth of 12m and a design depth of 15m means the contractor must dig
// 3m deeper. A post-dredge depth of 15.2m means they dug 0.2m below
// design (overdredge).
//
// Four buckets:
//   1. PAY VOLUME — material removed from pre-dredge seabed down to
//      design grade. Always paid.
//   2. ALLOWABLE OVERDREDGE — material removed between design and
//      (design + tolerance). Also paid. Tolerance typically 0.3-0.5m.
//   3. EXCESSIVE OVERDREDGE — material removed below (design + tolerance).
//      NOT paid. Often triggers back-charge if it destabilizes slopes.
//   4. SHOALING / UNDER-DREDGE — material remaining above design grade
//      that should have been removed. Requires re-dredging.
//
// All three grids must be aligned (same dimensions, same geographic
// extent). Caller is responsible for resampling if surveys were on
// different grids.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DredgeCategory {
    Pay,
    AllowableOverdredge,
    ExcessiveOverdredge,
    Shoaling,
    NoChange,
}

#[derive(Debug, Clone, Serialize)]
pub struct DredgeCell {
    pub category: DredgeCategory,
    pub row: usize,
    pub col: usize,
    pub post_depth: f64,
    pub design_depth: f64,
    /// Material removed at this cell (post - pre, positive = dredged). m.
    pub removed: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DredgeVolumeResult {
    /// Pay volume — material removed from pre down to design (m³)
    pub pay_volume: f64,
    /// Allowable overdredge volume (within tolerance, m³) — paid
    pub allowable_overdredge: f64,
    /// Excessive overdredge volume (below tolerance, m³) — unpaid
    pub excessive_overdredge: f64,
    /// Shoaling / under-dredge volume (material left above design, m³)
    pub shoaling: f64,
    /// Total paid volume = pay + allowable overdredge (m³)
    pub total_paid: f64,
    /// Cell counts per category
    pub pay_cells: usize,
    pub allowable_cells: usize,
    pub excessive_cells: usize,
    pub shoaling_cells: usize,
    pub no_change_cells: usize,
    /// Cell area used for integration (m²)
    pub cell_area: f64,
    /// Sparse cell list — only non-NoChange cells (for map overlay)
    pub cells: Vec<DredgeCell>,
    /// Tolerance used (m)
    pub tolerance_m: f64,
    /// Average dredge depth across cells where dredging occurred (m)
    pub avg_dredge_depth: f64,
    /// Maximum excessive overdredge depth below tolerance (m)
    pub max_excessive: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum DredgeError {
    #[error("grids have different lengths: post={0}, pre={1}, design={2}")]
    DimensionMismatch(usize, usize, usize),
    #[error("grid is empty")]
    Empty,
    #[error("cell dimensions must be positive: got {0}x{1}")]
    InvalidCellDims(f64, f64),
    #[error("tolerance must be non-negative: got {0}")]
    InvalidTolerance(f64),
}

/// Compute dredge pay-volume breakdown.
///
/// All three grids are flat row-major `Vec<f64>` with identical length.
/// Depths are POSITIVE DOWNWARD.
pub fn compute_dredge_volumes(
    post_dredge: &[f64],
    pre_dredge: &[f64],
    design: &[f64],
    cell_width_m: f64,
    cell_height_m: f64,
    tolerance_m: f64,
) -> Result<DredgeVolumeResult, DredgeError> {
    if post_dredge.is_empty() || pre_dredge.is_empty() || design.is_empty() {
        return Err(DredgeError::Empty);
    }
    let n = post_dredge.len();
    if pre_dredge.len() != n || design.len() != n {
        return Err(DredgeError::DimensionMismatch(
            n,
            pre_dredge.len(),
            design.len(),
        ));
    }
    if cell_width_m <= 0.0 || cell_height_m <= 0.0 {
        return Err(DredgeError::InvalidCellDims(cell_width_m, cell_height_m));
    }
    if tolerance_m < 0.0 {
        return Err(DredgeError::InvalidTolerance(tolerance_m));
    }

    let cell_area = cell_width_m * cell_height_m;
    let mut pay_vol = 0.0f64;
    let mut allow_vol = 0.0f64;
    let mut excess_vol = 0.0f64;
    let mut shoal_vol = 0.0f64;
    let mut pay_cells = 0usize;
    let mut allowable_cells = 0usize;
    let mut excessive_cells = 0usize;
    let mut shoaling_cells = 0usize;
    let mut no_change = 0usize;
    let mut cells: Vec<DredgeCell> = Vec::new();
    let mut sum_dredge_depth = 0.0f64;
    let mut dredge_depth_count = 0usize;
    let mut max_excessive = 0.0f64;

    // For row/col attribution in the map overlay. If grid isn't square,
    // frontend can reinterpret; we just need stable (row, col) per index.
    let approx_cols = ((n as f64).sqrt().round() as usize).max(1);

    for (i, ((&post, &pre), &des)) in post_dredge
        .iter()
        .zip(pre_dredge.iter())
        .zip(design.iter())
        .enumerate()
    {
        // Skip nodata cells
        if post.is_nan()
            || pre.is_nan()
            || des.is_nan()
            || post <= -9999.0
            || pre <= -9999.0
            || des <= -9999.0
        {
            no_change += 1;
            continue;
        }

        let removed = post - pre; // positive = dredged deeper
        let mut cell_category = DredgeCategory::NoChange;
        let mut cell_pay = 0.0f64;
        let mut cell_allow = 0.0f64;
        let mut cell_excess = 0.0f64;
        let mut cell_shoal = 0.0f64;

        if post >= des {
            // Case A: dredged to or beyond design
            // pay = max(0, design - pre) × area  (what we contracted to remove)
            let pay_depth = (des - pre).max(0.0);
            cell_pay = pay_depth * cell_area;

            // allowable = max(0, min(post, design + tol) - design) × area
            let allow_depth = (post.min(des + tolerance_m) - des).max(0.0);
            cell_allow = allow_depth * cell_area;

            // excessive = max(0, post - (design + tol)) × area
            let excess_depth = (post - (des + tolerance_m)).max(0.0);
            cell_excess = excess_depth * cell_area;

            if cell_excess > 0.0 {
                cell_category = DredgeCategory::ExcessiveOverdredge;
                excessive_cells += 1;
                if excess_depth > max_excessive {
                    max_excessive = excess_depth;
                }
            } else if cell_allow > 0.0 {
                cell_category = DredgeCategory::AllowableOverdredge;
                allowable_cells += 1;
            } else if cell_pay > 0.0 {
                cell_category = DredgeCategory::Pay;
                pay_cells += 1;
            } else {
                cell_category = DredgeCategory::NoChange;
            }

            if removed > 0.0 {
                sum_dredge_depth += removed;
                dredge_depth_count += 1;
            }
        } else {
            // Case B: post < design — didn't reach design depth
            // pay = max(0, post - pre) × area  (whatever was actually removed)
            let pay_depth = (post - pre).max(0.0);
            cell_pay = pay_depth * cell_area;

            // shoaling = (design - post) × area  (material left to remove)
            let shoal_depth = des - post;
            cell_shoal = shoal_depth * cell_area;

            if cell_shoal > 0.0 {
                cell_category = DredgeCategory::Shoaling;
                shoaling_cells += 1;
            } else if cell_pay > 0.0 {
                // Edge case: post == design exactly (boundary with Case A)
                cell_category = DredgeCategory::Pay;
                pay_cells += 1;
            } else {
                cell_category = DredgeCategory::NoChange;
            }

            if removed > 0.0 {
                sum_dredge_depth += removed;
                dredge_depth_count += 1;
            }
        }

        pay_vol += cell_pay;
        allow_vol += cell_allow;
        excess_vol += cell_excess;
        shoal_vol += cell_shoal;

        if cell_category != DredgeCategory::NoChange {
            let row = i / approx_cols;
            let col = i % approx_cols;
            cells.push(DredgeCell {
                category: cell_category,
                row,
                col,
                post_depth: post,
                design_depth: des,
                removed,
            });
        } else {
            no_change += 1;
        }
    }

    let total_paid = pay_vol + allow_vol;
    let avg_dredge_depth = if dredge_depth_count > 0 {
        sum_dredge_depth / dredge_depth_count as f64
    } else {
        0.0
    };

    Ok(DredgeVolumeResult {
        pay_volume: pay_vol,
        allowable_overdredge: allow_vol,
        excessive_overdredge: excess_vol,
        shoaling: shoal_vol,
        total_paid,
        pay_cells,
        allowable_cells,
        excessive_cells,
        shoaling_cells,
        no_change_cells: no_change,
        cell_area,
        cells,
        tolerance_m,
        avg_dredge_depth,
        max_excessive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(v: f64, n: usize) -> Vec<f64> {
        vec![v; n]
    }

    #[test]
    fn test_clean_dredge_to_design() {
        // pre=12, design=15, post=15 → dredged 3m, all pay
        let pre = flat(12.0, 100);
        let design = flat(15.0, 100);
        let post = flat(15.0, 100);
        let r = compute_dredge_volumes(&post, &pre, &design, 1.0, 1.0, 0.3).unwrap();
        assert!((r.pay_volume - 300.0).abs() < 0.01, "pay: {}", r.pay_volume);
        assert!((r.allowable_overdredge - 0.0).abs() < 0.01);
        assert!((r.excessive_overdredge - 0.0).abs() < 0.01);
        assert!((r.shoaling - 0.0).abs() < 0.01);
        assert_eq!(r.pay_cells, 100);
        assert_eq!(r.total_paid, 300.0);
        assert!((r.avg_dredge_depth - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_allowable_overdredge() {
        // pre=12, design=15, tol=0.3, post=15.2 → 0.2m within tolerance
        let pre = flat(12.0, 100);
        let design = flat(15.0, 100);
        let post = flat(15.2, 100);
        let r = compute_dredge_volumes(&post, &pre, &design, 1.0, 1.0, 0.3).unwrap();
        assert!((r.pay_volume - 300.0).abs() < 0.01, "pay: {}", r.pay_volume);
        assert!(
            (r.allowable_overdredge - 20.0).abs() < 0.01,
            "allow: {}",
            r.allowable_overdredge
        );
        assert!((r.excessive_overdredge - 0.0).abs() < 0.01);
        assert_eq!(r.allowable_cells, 100);
        assert_eq!(r.pay_cells, 0); // pay bucket is empty when overdredge occurs
    }

    #[test]
    fn test_excessive_overdredge() {
        // pre=12, design=15, tol=0.3, post=16 → 1m below design, 0.7m excessive
        let pre = flat(12.0, 100);
        let design = flat(15.0, 100);
        let post = flat(16.0, 100);
        let r = compute_dredge_volumes(&post, &pre, &design, 1.0, 1.0, 0.3).unwrap();
        assert!((r.pay_volume - 300.0).abs() < 0.01, "pay: {}", r.pay_volume);
        assert!(
            (r.allowable_overdredge - 30.0).abs() < 0.01,
            "allow: {}",
            r.allowable_overdredge
        );
        assert!(
            (r.excessive_overdredge - 70.0).abs() < 0.01,
            "excess: {}",
            r.excessive_overdredge
        );
        assert_eq!(r.excessive_cells, 100);
        assert!((r.max_excessive - 0.7).abs() < 0.001);
        assert!(
            (r.total_paid - 330.0).abs() < 0.01,
            "total_paid: {}",
            r.total_paid
        );
    }

    #[test]
    fn test_shoaling() {
        // pre=12, design=15, post=13 → only dredged 1m, 2m of material left
        let pre = flat(12.0, 100);
        let design = flat(15.0, 100);
        let post = flat(13.0, 100);
        let r = compute_dredge_volumes(&post, &pre, &design, 1.0, 1.0, 0.3).unwrap();
        assert!((r.shoaling - 200.0).abs() < 0.01, "shoal: {}", r.shoaling);
        assert_eq!(r.shoaling_cells, 100);
        // pay = (post - pre) = 1m × 100 = 100 m³ (only what was removed)
        assert!((r.pay_volume - 100.0).abs() < 0.01, "pay: {}", r.pay_volume);
    }

    #[test]
    fn test_no_dredging_done() {
        // pre=12, design=15, post=12 → nothing dredged, all shoaling
        let pre = flat(12.0, 100);
        let design = flat(15.0, 100);
        let post = flat(12.0, 100);
        let r = compute_dredge_volumes(&post, &pre, &design, 1.0, 1.0, 0.3).unwrap();
        assert!((r.pay_volume - 0.0).abs() < 0.01);
        assert!((r.shoaling - 300.0).abs() < 0.01, "shoal: {}", r.shoaling);
        assert_eq!(r.shoaling_cells, 100);
        assert_eq!(r.pay_cells, 0);
    }

    #[test]
    fn test_dimension_mismatch() {
        let a = flat(10.0, 50);
        let b = flat(10.0, 100);
        let r = compute_dredge_volumes(&a, &b, &a, 1.0, 1.0, 0.3);
        assert!(r.is_err());
    }

    #[test]
    fn test_empty_grids() {
        let r = compute_dredge_volumes(&[], &[], &[], 1.0, 1.0, 0.3);
        assert!(r.is_err());
    }

    #[test]
    fn test_negative_tolerance_errors() {
        let a = flat(10.0, 10);
        let r = compute_dredge_volumes(&a, &a, &a, 1.0, 1.0, -0.5);
        assert!(r.is_err());
    }
}
