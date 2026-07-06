// Range checks — sanity checks for gross input errors.
//
// These catch the "obvious" mistakes: latitude of 950°, a distance of
// 10,000 km from a total station, a bearing of -45°. Without these,
// the calculation engine happily propagates garbage through to the
// final report.
//
// Each check returns a RangeCheckResult with pass/fail + a warning
// message. The caller decides whether to reject the input or just warn.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeCheckResult {
    pub passed: bool,
    pub message: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

impl RangeCheckResult {
    fn ok(value: f64, min: f64, max: f64) -> Self {
        Self { passed: true, message: String::new(), value, min, max }
    }
    fn fail(value: f64, min: f64, max: f64, label: &str) -> Self {
        Self {
            passed: false,
            message: format!(
                "{} out of range: {:.6} is outside [{:.6}, {:.6}]",
                label, value, min, max
            ),
            value,
            min,
            max,
        }
    }
}

/// Check that a latitude is in [-90, 90] degrees.
pub fn check_lat_lon(lat: f64, lon: f64) -> (RangeCheckResult, RangeCheckResult) {
    let lat_check = if lat.is_finite() && (-90.0..=90.0).contains(&lat) {
        RangeCheckResult::ok(lat, -90.0, 90.0)
    } else {
        RangeCheckResult::fail(lat, -90.0, 90.0, "Latitude")
    };
    let lon_check = if lon.is_finite() && (-180.0..=180.0).contains(&lon) {
        RangeCheckResult::ok(lon, -180.0, 180.0)
    } else {
        RangeCheckResult::fail(lon, -180.0, 180.0, "Longitude")
    };
    (lat_check, lon_check)
}

/// Check that an elevation is plausible.
///
/// `regional_msl` is the approximate mean sea level elevation for the
/// survey area (meters). `max_deviation_m` is how far above/below MSL
/// is considered plausible. Default 2000 m covers most surface surveys;
/// underground mines use a larger value.
pub fn check_elevation(elev_m: f64, regional_msl: f64, max_deviation_m: f64) -> RangeCheckResult {
    let min = regional_msl - max_deviation_m;
    let max = regional_msl + max_deviation_m;
    if elev_m.is_finite() && (min..=max).contains(&elev_m) {
        RangeCheckResult::ok(elev_m, min, max)
    } else {
        RangeCheckResult::fail(elev_m, min, max, "Elevation")
    }
}

/// Check that a distance is plausible for the instrument.
///
/// Total stations typically measure 1-3000 m. GNSS baselines are
/// 0.1-50 km. Distances outside these ranges are likely unit errors
/// (m vs km) or swapped coordinates.
pub fn check_distance(distance_m: f64, min_m: f64, max_m: f64) -> RangeCheckResult {
    if distance_m.is_finite() && distance_m >= 0.0 && (min_m..=max_m).contains(&distance_m) {
        RangeCheckResult::ok(distance_m, min_m, max_m)
    } else if !distance_m.is_finite() {
        RangeCheckResult::fail(distance_m, min_m, max_m, "Distance (non-finite)")
    } else if distance_m < 0.0 {
        RangeCheckResult::fail(distance_m, min_m, max_m, "Distance (negative)")
    } else {
        RangeCheckResult::fail(distance_m, min_m, max_m, "Distance")
    }
}

/// Check that a bearing is in [0, 360) degrees.
pub fn check_bearing(bearing_deg: f64) -> RangeCheckResult {
    if bearing_deg.is_finite() && (0.0..360.0).contains(&bearing_deg) {
        RangeCheckResult::ok(bearing_deg, 0.0, 360.0)
    } else {
        RangeCheckResult::fail(bearing_deg, 0.0, 360.0, "Bearing")
    }
}

/// Convenience: check that a volume is plausible.
///
/// A "negative volume" is suspicious (fill < 0 or cut < 0). Volumes
/// larger than `max_m3` suggest a unit error (m³ vs ft³) or a wrong
/// reference surface. Default max is 1 billion m³ (1 km³) which covers
/// the largest open-pit mines.
pub fn check_volume(volume_m3: f64, max_m3: f64) -> RangeCheckResult {
    if !volume_m3.is_finite() {
        return RangeCheckResult::fail(volume_m3, 0.0, max_m3, "Volume (non-finite)");
    }
    if volume_m3.abs() > max_m3 {
        return RangeCheckResult::fail(volume_m3, -max_m3, max_m3, "Volume (excessive)");
    }
    RangeCheckResult::ok(volume_m3, -max_m3, max_m3)
}

/// Check that a horizontal/vertical uncertainty is plausible.
///
/// Survey uncertainties should be positive and small. A sigma > 100 m
/// suggests a unit error or a computation bug.
pub fn check_uncertainty(sigma: f64, max_sigma: f64) -> RangeCheckResult {
    if !sigma.is_finite() {
        return RangeCheckResult::fail(sigma, 0.0, max_sigma, "Uncertainty (non-finite)");
    }
    if sigma < 0.0 {
        return RangeCheckResult::fail(sigma, 0.0, max_sigma, "Uncertainty (negative)");
    }
    if sigma > max_sigma {
        return RangeCheckResult::fail(sigma, 0.0, max_sigma, "Uncertainty (excessive)");
    }
    RangeCheckResult::ok(sigma, 0.0, max_sigma)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lat_lon_valid() {
        let (lat, lon) = check_lat_lon(45.0, -75.0);
        assert!(lat.passed);
        assert!(lon.passed);
    }

    #[test]
    fn test_lat_out_of_range() {
        let (lat, _) = check_lat_lon(95.0, 0.0);
        assert!(!lat.passed);
        assert!(lat.message.contains("Latitude"));
    }

    #[test]
    fn test_lon_out_of_range() {
        let (_, lon) = check_lat_lon(0.0, 200.0);
        assert!(!lon.passed);
        assert!(lon.message.contains("Longitude"));
    }

    #[test]
    fn test_lat_nan() {
        let (lat, _) = check_lat_lon(f64::NAN, 0.0);
        assert!(!lat.passed);
    }

    #[test]
    fn test_elevation_within_regional_msl() {
        let r = check_elevation(1050.0, 1000.0, 2000.0);
        assert!(r.passed);
    }

    #[test]
    fn test_elevation_underground() {
        // A mine working at -1500 m below MSL is plausible with a 2000 m tolerance
        let r = check_elevation(-1500.0, 0.0, 2000.0);
        assert!(r.passed);
    }

    #[test]
    fn test_elevation_excessive() {
        // 10,000 m elevation is implausible everywhere on Earth
        let r = check_elevation(10_000.0, 0.0, 2000.0);
        assert!(!r.passed);
    }

    #[test]
    fn test_distance_total_station() {
        // 500 m is a normal total station shot
        let r = check_distance(500.0, 1.0, 3000.0);
        assert!(r.passed);
    }

    #[test]
    fn test_distance_negative() {
        let r = check_distance(-50.0, 1.0, 3000.0);
        assert!(!r.passed);
        assert!(r.message.contains("negative"));
    }

    #[test]
    fn test_distance_excessive() {
        // 100 km is way beyond a total station — likely a unit error
        let r = check_distance(100_000.0, 1.0, 3000.0);
        assert!(!r.passed);
    }

    #[test]
    fn test_bearing_valid() {
        assert!(check_bearing(0.0).passed);
        assert!(check_bearing(180.0).passed);
        assert!(check_bearing(359.999).passed);
    }

    #[test]
    fn test_bearing_out_of_range() {
        assert!(!check_bearing(-1.0).passed);
        assert!(!check_bearing(360.0).passed);
        assert!(!check_bearing(720.0).passed);
    }

    #[test]
    fn test_volume_plausible() {
        let r = check_volume(12_345.0, 1e9);
        assert!(r.passed);
    }

    #[test]
    fn test_volume_excessive() {
        // 5 km³ is implausible for a single stockpile
        let r = check_volume(5e9, 1e9);
        assert!(!r.passed);
    }

    #[test]
    fn test_uncertainty_valid() {
        let r = check_uncertainty(0.05, 100.0);
        assert!(r.passed);
    }

    #[test]
    fn test_uncertainty_negative() {
        let r = check_uncertainty(-1.0, 100.0);
        assert!(!r.passed);
        assert!(r.message.contains("negative"));
    }

    #[test]
    fn test_uncertainty_excessive() {
        let r = check_uncertainty(500.0, 100.0);
        assert!(!r.passed);
        assert!(r.message.contains("excessive"));
    }
}
