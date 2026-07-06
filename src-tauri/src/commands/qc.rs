// QC IPC commands — Sprint 12.
//
// Exposes the UncertainValue arithmetic and range-check utilities to the
// frontend so dialogs can format uncertainty in results and validate
// user inputs before sending them to the calculation engine.

use crate::qc::propagation::UncertainValue;
use crate::qc::range_checks::{check_bearing, check_distance, check_elevation, check_lat_lon, check_uncertainty, check_volume, RangeCheckResult};

/// Format an UncertainValue with units. Used by the frontend to display
/// "12,345 ± 6 m³ (95%)" in volume reports and dialog results.
#[tauri::command]
pub fn format_uncertain_value_cmd(value: UncertainValue, unit: String) -> String {
    value.format_with_units(&unit)
}

/// Compute the 95% confidence interval for an UncertainValue.
#[tauri::command]
pub fn uncertain_value_ci_95_cmd(value: UncertainValue) -> (f64, f64) {
    value.ci_95()
}

/// Add two uncertain values (propagating uncertainty).
#[tauri::command]
pub fn uncertain_value_add_cmd(a: UncertainValue, b: UncertainValue) -> UncertainValue {
    a.add(&b)
}

/// Subtract two uncertain values.
#[tauri::command]
pub fn uncertain_value_sub_cmd(a: UncertainValue, b: UncertainValue) -> UncertainValue {
    a.sub(&b)
}

/// Multiply two uncertain values.
#[tauri::command]
pub fn uncertain_value_mul_cmd(a: UncertainValue, b: UncertainValue) -> UncertainValue {
    a.mul(&b)
}

/// Divide two uncertain values.
#[tauri::command]
pub fn uncertain_value_div_cmd(a: UncertainValue, b: UncertainValue) -> UncertainValue {
    a.div(&b)
}

/// Sum a list of uncertain values (e.g., for aggregating per-cell volumes).
#[tauri::command]
pub fn uncertain_value_sum_cmd(values: Vec<UncertainValue>) -> UncertainValue {
    UncertainValue::sum(&values)
}

/// Mean of a list of uncertain values (reduces uncertainty by sqrt(N)).
#[tauri::command]
pub fn uncertain_value_mean_cmd(values: Vec<UncertainValue>) -> UncertainValue {
    UncertainValue::mean(&values)
}

// ── Range checks ──

#[tauri::command]
pub fn check_lat_lon_cmd(lat: f64, lon: f64) -> (RangeCheckResult, RangeCheckResult) {
    check_lat_lon(lat, lon)
}

#[tauri::command]
pub fn check_elevation_cmd(elev_m: f64, regional_msl: f64, max_deviation_m: f64) -> RangeCheckResult {
    check_elevation(elev_m, regional_msl, max_deviation_m)
}

#[tauri::command]
pub fn check_distance_cmd(distance_m: f64, min_m: f64, max_m: f64) -> RangeCheckResult {
    check_distance(distance_m, min_m, max_m)
}

#[tauri::command]
pub fn check_bearing_cmd(bearing_deg: f64) -> RangeCheckResult {
    check_bearing(bearing_deg)
}

#[tauri::command]
pub fn check_volume_cmd(volume_m3: f64, max_m3: f64) -> RangeCheckResult {
    check_volume(volume_m3, max_m3)
}

#[tauri::command]
pub fn check_uncertainty_cmd(sigma: f64, max_sigma: f64) -> RangeCheckResult {
    check_uncertainty(sigma, max_sigma)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cmd() {
        let v = UncertainValue::from_sigma(100.0, 2.0);
        let s = format_uncertain_value_cmd(v, "m³".to_string());
        assert!(s.contains("100"));
        assert!(s.contains("m³"));
    }

    #[test]
    fn test_add_cmd() {
        let a = UncertainValue::from_sigma(10.0, 1.0);
        let b = UncertainValue::from_sigma(20.0, 2.0);
        let c = uncertain_value_add_cmd(a, b);
        assert!((c.value - 30.0).abs() < 1e-9);
    }

    #[test]
    fn test_check_lat_lon_cmd() {
        let (lat, _) = check_lat_lon_cmd(45.0, 0.0);
        assert!(lat.passed);
        let (lat, _) = check_lat_lon_cmd(95.0, 0.0);
        assert!(!lat.passed);
    }
}
