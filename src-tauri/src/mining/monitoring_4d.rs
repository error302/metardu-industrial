// 4D monitoring — multi-temporal surface differencing for pit progression.
//
// Per ARCHITECTURE.md §4.1.6 — compares N surveys in a common frame,
// generates per-cell elevation change rasters, integrates to monthly
// volume deltas, and reconciles against the mine plan.
//
// This module operates on GeoTIFF DEM grids read into memory by the
// calling IPC command. It computes:
//   - Per-cell elevation difference (dz = current - previous)
//   - Volume delta per cell (dz × cell_area)
//   - Zone classification: Fill / Cut / Stable / No-data
//   - Tonnage estimate (using user-supplied density)
//   - Hotspot detection (cells with |dz| > threshold)
//   - Cumulative progression over N epochs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitoring4DParams {
    /// Cell area in square meters (from DEM pixel scale)
    #[serde(default = "default_cell_area")]
    pub cell_area: f64,
    /// Rock density in tonnes per cubic meter (typical: 2.7 for iron ore, 1.6 for coal)
    #[serde(default = "default_density")]
    pub density: f64,
    /// Elevation change threshold for hotspot detection (meters)
    #[serde(default = "default_hotspot_threshold")]
    pub hotspot_threshold: f64,
    /// Minimum elevation change to count as active (meters)
    #[serde(default = "default_active_threshold")]
    pub active_threshold: f64,
}

fn default_cell_area() -> f64 {
    1.0
}
fn default_density() -> f64 {
    2.7
}
fn default_hotspot_threshold() -> f64 {
    1.0
}
fn default_active_threshold() -> f64 {
    0.1
}

impl Default for Monitoring4DParams {
    fn default() -> Self {
        Self {
            cell_area: default_cell_area(),
            density: default_density(),
            hotspot_threshold: default_hotspot_threshold(),
            active_threshold: default_active_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EpochDiff {
    /// Per-cell elevation difference (current - previous). NaN where either
    /// surface has no data.
    pub dz: Vec<f64>,
    /// Per-cell volume delta (cubic meters). Positive = fill, negative = cut.
    pub volume_delta: Vec<f64>,
    /// Per-cell tonnage delta (tonnes). Uses density parameter.
    pub tonnage_delta: Vec<f64>,
    /// Per-cell zone classification
    pub zones: Vec<ChangeZone>,
    /// Summary statistics
    pub summary: DiffSummary,
    /// Hotspot indices (cells where |dz| > hotspot_threshold)
    pub hotspots: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeZone {
    /// dz > active_threshold (material added)
    Fill,
    /// dz < -active_threshold (material removed)
    Cut,
    /// |dz| <= active_threshold (no significant change)
    Stable,
    /// Either surface has no data at this cell
    NoData,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DiffSummary {
    pub total_fill_volume: f64,
    pub total_cut_volume: f64,
    pub net_volume: f64,
    pub total_fill_tonnage: f64,
    pub total_cut_tonnage: f64,
    pub net_tonnage: f64,
    pub fill_cells: usize,
    pub cut_cells: usize,
    pub stable_cells: usize,
    pub nodata_cells: usize,
    pub active_cells: usize,
    pub max_fill: f64,
    pub max_cut: f64,
    pub mean_dz: f64,
    pub rms_dz: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressionReport {
    pub epochs: Vec<EpochSummary>,
    pub cumulative_fill: f64,
    pub cumulative_cut: f64,
    pub cumulative_net: f64,
    pub cumulative_tonnage: f64,
    pub max_single_epoch_change: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct EpochSummary {
    pub epoch: usize,
    pub fill_volume: f64,
    pub cut_volume: f64,
    pub net_volume: f64,
    pub fill_tonnage: f64,
    pub cut_tonnage: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("surfaces have different dimensions: {0} vs {1}")]
    DimensionMismatch(usize, usize),
    #[error("at least 2 surfaces required for differencing")]
    TooFewSurfaces,
    #[error("empty surface")]
    Empty,
}

/// Compute the difference between two DEM surfaces.
///
/// Both surfaces must have the same dimensions. NaN values in either
/// surface produce NaN in the difference.
pub fn compute_epoch_diff(
    previous: &[f64],
    current: &[f64],
    params: &Monitoring4DParams,
) -> Result<EpochDiff, MonitoringError> {
    if previous.is_empty() || current.is_empty() {
        return Err(MonitoringError::Empty);
    }
    if previous.len() != current.len() {
        return Err(MonitoringError::DimensionMismatch(
            previous.len(),
            current.len(),
        ));
    }

    let n = previous.len();
    let mut dz = Vec::with_capacity(n);
    let mut volume_delta = Vec::with_capacity(n);
    let mut tonnage_delta = Vec::with_capacity(n);
    let mut zones = Vec::with_capacity(n);
    let mut hotspots = Vec::new();

    let mut summary = DiffSummary::default();
    let mut dz_sum = 0.0f64;
    let mut dz_sq_sum = 0.0f64;
    let mut valid_count = 0usize;

    for i in 0..n {
        let prev = previous[i];
        let curr = current[i];

        if prev.is_nan() || curr.is_nan() {
            dz.push(f64::NAN);
            volume_delta.push(f64::NAN);
            tonnage_delta.push(f64::NAN);
            zones.push(ChangeZone::NoData);
            summary.nodata_cells += 1;
            continue;
        }

        let d = curr - prev;
        dz.push(d);
        let vol = d * params.cell_area;
        volume_delta.push(vol);
        let tonnage = vol * params.density;
        tonnage_delta.push(tonnage);

        let zone = if d > params.active_threshold {
            summary.fill_cells += 1;
            summary.total_fill_volume += vol;
            summary.total_fill_tonnage += tonnage;
            if d > summary.max_fill {
                summary.max_fill = d;
            }
            ChangeZone::Fill
        } else if d < -params.active_threshold {
            summary.cut_cells += 1;
            summary.total_cut_volume += -vol;
            summary.total_cut_tonnage += -tonnage;
            if d < summary.max_cut {
                summary.max_cut = d;
            }
            ChangeZone::Cut
        } else {
            summary.stable_cells += 1;
            ChangeZone::Stable
        };

        zones.push(zone);

        if d.abs() > params.hotspot_threshold {
            hotspots.push(i);
        }

        dz_sum += d;
        dz_sq_sum += d * d;
        valid_count += 1;
    }

    summary.net_volume = summary.total_fill_volume - summary.total_cut_volume;
    summary.net_tonnage = summary.total_fill_tonnage - summary.total_cut_tonnage;
    summary.active_cells = summary.fill_cells + summary.cut_cells;

    if valid_count > 0 {
        summary.mean_dz = dz_sum / valid_count as f64;
        summary.rms_dz = (dz_sq_sum / valid_count as f64).sqrt();
    }

    Ok(EpochDiff {
        dz,
        volume_delta,
        tonnage_delta,
        zones,
        summary,
        hotspots,
    })
}

/// Compute a cumulative progression report across N epochs.
///
/// `surfaces` is a Vec of DEM grids, ordered chronologically. The
/// function computes the difference between each consecutive pair and
/// accumulates totals.
pub fn compute_progression(
    surfaces: &[Vec<f64>],
    params: &Monitoring4DParams,
) -> Result<ProgressionReport, MonitoringError> {
    if surfaces.len() < 2 {
        return Err(MonitoringError::TooFewSurfaces);
    }

    let mut epochs = Vec::with_capacity(surfaces.len() - 1);
    let mut cumulative_fill = 0.0;
    let mut cumulative_cut = 0.0;
    let mut cumulative_net = 0.0;
    let mut cumulative_tonnage = 0.0;
    let mut max_single_epoch_change = 0.0f64;

    for i in 1..surfaces.len() {
        let diff = compute_epoch_diff(&surfaces[i - 1], &surfaces[i], params)?;
        let epoch_summary = EpochSummary {
            epoch: i,
            fill_volume: diff.summary.total_fill_volume,
            cut_volume: diff.summary.total_cut_volume,
            net_volume: diff.summary.net_volume,
            fill_tonnage: diff.summary.total_fill_tonnage,
            cut_tonnage: diff.summary.total_cut_tonnage,
        };

        cumulative_fill += epoch_summary.fill_volume;
        cumulative_cut += epoch_summary.cut_volume;
        cumulative_net += epoch_summary.net_volume;
        cumulative_tonnage += epoch_summary.fill_tonnage - epoch_summary.cut_tonnage;

        let change = epoch_summary.fill_volume + epoch_summary.cut_volume;
        if change > max_single_epoch_change {
            max_single_epoch_change = change;
        }

        epochs.push(epoch_summary);
    }

    Ok(ProgressionReport {
        epochs,
        cumulative_fill,
        cumulative_cut,
        cumulative_net,
        cumulative_tonnage,
        max_single_epoch_change,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_flat_grid(n: usize, z: f64) -> Vec<f64> {
        vec![z; n]
    }

    #[test]
    fn test_fill_progression() {
        // Previous at z=100, current at z=110 → 10m fill everywhere
        let prev = make_flat_grid(100, 100.0);
        let curr = make_flat_grid(100, 110.0);
        let params = Monitoring4DParams {
            cell_area: 10.0,
            density: 2.7,
            ..Default::default()
        };
        let result = compute_epoch_diff(&prev, &curr, &params).unwrap();
        assert_eq!(result.summary.fill_cells, 100);
        assert_eq!(result.summary.cut_cells, 0);
        // 100 cells × 10m × 10m² = 10000 m³ fill
        assert!((result.summary.total_fill_volume - 10000.0).abs() < 0.1);
        // 10000 m³ × 2.7 t/m³ = 27000 tonnes
        assert!((result.summary.total_fill_tonnage - 27000.0).abs() < 0.1);
    }

    #[test]
    fn test_cut_progression() {
        let prev = make_flat_grid(100, 110.0);
        let curr = make_flat_grid(100, 100.0);
        let result = compute_epoch_diff(&prev, &curr, &Monitoring4DParams::default()).unwrap();
        assert_eq!(result.summary.cut_cells, 100);
        assert_eq!(result.summary.fill_cells, 0);
    }

    #[test]
    fn test_stable_zone() {
        let prev = make_flat_grid(100, 100.0);
        let curr = make_flat_grid(100, 100.05); // 0.05m change < 0.1m threshold
        let result = compute_epoch_diff(&prev, &curr, &Monitoring4DParams::default()).unwrap();
        assert_eq!(result.summary.stable_cells, 100);
    }

    #[test]
    fn test_nodata_cells() {
        let mut prev = make_flat_grid(100, 100.0);
        let mut curr = make_flat_grid(100, 110.0);
        prev[0] = f64::NAN;
        curr[1] = f64::NAN;
        let result = compute_epoch_diff(&prev, &curr, &Monitoring4DParams::default()).unwrap();
        assert_eq!(result.summary.nodata_cells, 2);
        assert_eq!(result.summary.fill_cells, 98);
    }

    #[test]
    fn test_hotspot_detection() {
        let prev = make_flat_grid(100, 100.0);
        let mut curr = make_flat_grid(100, 100.0);
        curr[5] = 105.0; // 5m change > 1m hotspot threshold
        curr[10] = 94.0; // -6m change
        let result = compute_epoch_diff(&prev, &curr, &Monitoring4DParams::default()).unwrap();
        assert!(result.hotspots.contains(&5));
        assert!(result.hotspots.contains(&10));
        assert!(result.hotspots.len() >= 2);
    }

    #[test]
    fn test_progression_report() {
        let surfaces = vec![
            make_flat_grid(100, 100.0),
            make_flat_grid(100, 110.0),
            make_flat_grid(100, 105.0),
        ];
        let params = Monitoring4DParams {
            cell_area: 1.0,
            ..Default::default()
        };
        let report = compute_progression(&surfaces, &params).unwrap();
        assert_eq!(report.epochs.len(), 2);
        // Epoch 1: +10m fill = 1000 m³
        assert!((report.epochs[0].fill_volume - 1000.0).abs() < 0.1);
        // Epoch 2: -5m cut = 500 m³
        assert!((report.epochs[1].cut_volume - 500.0).abs() < 0.1);
        // Cumulative net = 1000 - 500 = 500 m³
        assert!((report.cumulative_net - 500.0).abs() < 0.1);
    }

    #[test]
    fn test_dimension_mismatch() {
        let prev = vec![1.0, 2.0, 3.0];
        let curr = vec![1.0, 2.0];
        let result = compute_epoch_diff(&prev, &curr, &Monitoring4DParams::default());
        assert!(matches!(
            result,
            Err(MonitoringError::DimensionMismatch(3, 2))
        ));
    }
}
