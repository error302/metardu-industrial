//! Reservoir bed and storage comparisons on aligned metre-based grids.
//!
//! Elevations are cell averages in metres, positive upward. Results cover
//! only mask cells with valid data in both epochs; they are not full-reservoir
//! estimates when coverage is incomplete.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridGeometry {
    pub columns: usize,
    pub rows: usize,
    pub origin_x_m: f64,
    pub origin_y_m: f64,
    pub cell_width_m: f64,
    pub cell_height_m: f64,
    pub horizontal_crs: String,
    pub vertical_datum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedGrid {
    pub geometry: GridGeometry,
    /// Missing values are represented by None, not by an assumed sentinel.
    pub elevations_m: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageAtLevel {
    pub water_level_m: f64,
    pub reference_storage_m3: f64,
    pub current_storage_m3: f64,
    pub storage_change_m3: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirComparison {
    pub mask_cells: usize,
    pub common_cells: usize,
    pub missing_cells: usize,
    pub common_area_m2: f64,
    pub coverage_fraction: f64,
    pub deposition_m3: f64,
    pub erosion_m3: f64,
    pub net_bed_rise_m3: f64,
    pub storage: Vec<StorageAtLevel>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReservoirError {
    #[error("invalid grid geometry or missing CRS/datum")]
    InvalidGeometry,
    #[error("survey grids do not have identical geometry, CRS and vertical datum")]
    GridMismatch,
    #[error("grid or reservoir mask length does not match its dimensions")]
    InvalidLength,
    #[error("reservoir mask contains no cells with data from both surveys")]
    NoCommonCoverage,
    #[error("water levels and supplied elevations must be finite")]
    NonFiniteInput,
    #[error("calculation produced a non-finite value")]
    NonFiniteResult,
}

/// Compare already aligned grids. No resampling, datum conversion or
/// extrapolation is performed. Callers must verify CRS units and datums.
pub fn compare_surveys(
    reference: &BedGrid,
    current: &BedGrid,
    reservoir_mask: &[bool],
    water_levels_m: &[f64],
) -> Result<ReservoirComparison, ReservoirError> {
    let g = &reference.geometry;
    let count = g.columns.checked_mul(g.rows).ok_or(ReservoirError::InvalidGeometry)?;
    if count == 0 || g.horizontal_crs.trim().is_empty() || g.vertical_datum.trim().is_empty()
        || !g.origin_x_m.is_finite() || !g.origin_y_m.is_finite()
        || !g.cell_width_m.is_finite() || !g.cell_height_m.is_finite()
        || g.cell_width_m <= 0.0 || g.cell_height_m <= 0.0 {
        return Err(ReservoirError::InvalidGeometry);
    }
    if reference.geometry != current.geometry { return Err(ReservoirError::GridMismatch); }
    if reference.elevations_m.len() != count || current.elevations_m.len() != count || reservoir_mask.len() != count {
        return Err(ReservoirError::InvalidLength);
    }
    if water_levels_m.iter().any(|v| !v.is_finite())
        || reference.elevations_m.iter().chain(&current.elevations_m).flatten().any(|v| !v.is_finite()) {
        return Err(ReservoirError::NonFiniteInput);
    }
    let area = g.cell_width_m * g.cell_height_m;
    if !area.is_finite() || area <= 0.0 { return Err(ReservoirError::InvalidGeometry); }
    let mut result = ReservoirComparison {
        mask_cells: 0, common_cells: 0, missing_cells: 0, common_area_m2: 0.0,
        coverage_fraction: 0.0, deposition_m3: 0.0, erosion_m3: 0.0, net_bed_rise_m3: 0.0,
        storage: water_levels_m.iter().map(|level| StorageAtLevel {
            water_level_m: *level, reference_storage_m3: 0.0, current_storage_m3: 0.0, storage_change_m3: 0.0,
        }).collect(),
    };
    for ((inside, old), new) in reservoir_mask.iter().zip(&reference.elevations_m).zip(&current.elevations_m) {
        if !inside { continue; }
        result.mask_cells += 1;
        let (old, new) = match (old, new) {
            (Some(old), Some(new)) => (*old, *new),
            _ => { result.missing_cells += 1; continue; }
        };
        result.common_cells += 1;
        let change = (new - old) * area;
        if change >= 0.0 { result.deposition_m3 += change; }
        else { result.erosion_m3 -= change; }
        for storage in &mut result.storage {
            storage.reference_storage_m3 += (storage.water_level_m - old).max(0.0) * area;
            storage.current_storage_m3 += (storage.water_level_m - new).max(0.0) * area;
        }
    }
    if result.common_cells == 0 { return Err(ReservoirError::NoCommonCoverage); }
    result.common_area_m2 = result.common_cells as f64 * area;
    result.coverage_fraction = result.common_cells as f64 / result.mask_cells as f64;
    result.net_bed_rise_m3 = result.deposition_m3 - result.erosion_m3;
    for storage in &mut result.storage {
        storage.storage_change_m3 = storage.current_storage_m3 - storage.reference_storage_m3;
        if !storage.reference_storage_m3.is_finite() || !storage.current_storage_m3.is_finite() || !storage.storage_change_m3.is_finite() {
            return Err(ReservoirError::NonFiniteResult);
        }
    }
    if !result.common_area_m2.is_finite() || !result.deposition_m3.is_finite() || !result.erosion_m3.is_finite() || !result.net_bed_rise_m3.is_finite() {
        return Err(ReservoirError::NonFiniteResult);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(values: Vec<Option<f64>>) -> BedGrid {
        BedGrid { geometry: GridGeometry { columns: 2, rows: 2, origin_x_m: 500_000.0, origin_y_m: 9_000_000.0, cell_width_m: 10.0, cell_height_m: 10.0, horizontal_crs: "EPSG:32737".into(), vertical_datum: "local datum".into() }, elevations_m: values }
    }

    #[test]
    fn compares_bed_and_storage_over_common_coverage() {
        let old = grid(vec![Some(0.0), Some(2.0), Some(4.0), Some(0.0)]);
        let new = grid(vec![Some(1.0), Some(1.0), None, Some(0.0)]);
        let result = compare_surveys(&old, &new, &[true, true, true, false], &[3.0]).unwrap();
        assert_eq!((result.mask_cells, result.common_cells, result.missing_cells), (3, 2, 1));
        assert_eq!(result.deposition_m3, 100.0);
        assert_eq!(result.erosion_m3, 100.0);
        assert_eq!(result.storage[0].reference_storage_m3, 400.0);
        assert_eq!(result.storage[0].current_storage_m3, 400.0);
        assert!((result.coverage_fraction - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_misalignment_and_missing_data() {
        let old = grid(vec![Some(0.0); 4]);
        let mut new = old.clone();
        new.geometry.origin_x_m += 1.0;
        assert!(matches!(compare_surveys(&old, &new, &[true; 4], &[1.0]), Err(ReservoirError::GridMismatch)));
        new = grid(vec![None; 4]);
        assert!(matches!(compare_surveys(&old, &new, &[true; 4], &[1.0]), Err(ReservoirError::NoCommonCoverage)));
        assert!(matches!(compare_surveys(&old, &old, &[true; 4], &[f64::NAN]), Err(ReservoirError::NonFiniteInput)));
    }

    #[test]
    fn storage_is_clipped_at_water_level() {
        let old = grid(vec![Some(0.0); 4]);
        let new = grid(vec![Some(3.0); 4]);
        let result = compare_surveys(&old, &new, &[true; 4], &[2.0, 5.0]).unwrap();
        assert_eq!(result.deposition_m3, 1200.0);
        assert_eq!(result.storage[0].reference_storage_m3, 800.0);
        assert_eq!(result.storage[0].current_storage_m3, 0.0);
        assert_eq!(result.storage[0].storage_change_m3, -800.0);
        assert_eq!(result.storage[1].storage_change_m3, -1200.0);
    }
}
