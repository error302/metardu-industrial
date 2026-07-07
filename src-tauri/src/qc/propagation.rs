// UncertainValue — a scalar with associated uncertainty and confidence.
//
// Every measurement in surveying has uncertainty. Every transformation
// of a measurement propagates and (usually) increases uncertainty.
// Treating numbers as exact `f64` values loses this information and
// produces false confidence in results.
//
// UncertainValue carries the value, the 1-sigma standard deviation, and
// the confidence level (0-1) through every arithmetic operation. The
// arithmetic follows the standard rules for propagation of uncertainty
// (see Taylor, "An Introduction to Error Analysis", 2nd ed.):
//
//   Addition/Subtraction: σ_result = sqrt(σ_a² + σ_b²)
//   Multiplication/Division: σ_result = |result| × sqrt((σ_a/a)² + (σ_b/b)²)
//   Power: σ_result = |n × a^(n-1)| × σ_a
//   sqrt: σ_result = σ_a / (2 × sqrt(a))
//
// These are first-order approximations (linearized via Taylor series).
// For most surveying applications they're sufficient. For highly non-
// linear transformations, use Monte Carlo simulation (not implemented
// here — would need a separate module).

use serde::{Deserialize, Serialize};

/// A scalar value with associated 1-sigma uncertainty and confidence level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UncertainValue {
    /// The central value (best estimate)
    pub value: f64,
    /// 1-sigma standard deviation (same units as value)
    pub sigma: f64,
    /// Confidence level (0-1). 0.68 = 1-sigma, 0.95 = 95% CI, 0.997 = 3-sigma
    pub confidence: f64,
}

impl UncertainValue {
    /// Create a value with zero uncertainty (treated as exact).
    pub fn certain(value: f64) -> Self {
        Self { value, sigma: 0.0, confidence: 1.0 }
    }

    /// Create a value with a 1-sigma uncertainty (68% confidence).
    pub fn from_sigma(value: f64, sigma: f64) -> Self {
        Self { value, sigma, confidence: 0.6827 }
    }

    /// Create a value with a 95% confidence interval (1.96 sigma).
    pub fn from_95_ci(value: f64, ci_95: f64) -> Self {
        Self { value, sigma: ci_95 / 1.96, confidence: 0.95 }
    }

    /// 95% confidence interval (lower, upper)
    pub fn ci_95(&self) -> (f64, f64) {
        let moe = 1.96 * self.sigma;
        (self.value - moe, self.value + moe)
    }

    /// 99.7% confidence interval (3-sigma)
    pub fn ci_997(&self) -> (f64, f64) {
        let moe = 3.0 * self.sigma;
        (self.value - moe, self.value + moe)
    }

    /// Margin of error at the current confidence level
    pub fn margin(&self) -> f64 {
        // Convert 1-sigma to the z-score matching self.confidence
        let z = confidence_to_z(self.confidence);
        z * self.sigma
    }

    /// Format as a string: "12,345 ± 6 m³ (95%)"
    pub fn format_with_units(&self, unit: &str) -> String {
        if self.sigma == 0.0 {
            format!("{:.3} {}", self.value, unit)
        } else {
            let pct = (self.confidence * 100.0).round() as u32;
            format!("{:.3} ± {:.3} {} ({}%)", self.value, self.margin(), unit, pct)
        }
    }

    // ── Arithmetic with uncertainty propagation ──

    /// (a ± σa) + (b ± σb) = (a+b) ± sqrt(σa² + σb²)
    pub fn add(&self, other: &UncertainValue) -> UncertainValue {
        UncertainValue {
            value: self.value + other.value,
            sigma: (self.sigma.powi(2) + other.sigma.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// (a ± σa) - (b ± σb) = (a-b) ± sqrt(σa² + σb²)
    pub fn sub(&self, other: &UncertainValue) -> UncertainValue {
        UncertainValue {
            value: self.value - other.value,
            sigma: (self.sigma.powi(2) + other.sigma.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// (a ± σa) × (b ± σb) — relative uncertainties add in quadrature
    pub fn mul(&self, other: &UncertainValue) -> UncertainValue {
        let result = self.value * other.value;
        let rel_a = relative_uncertainty(self.value, self.sigma);
        let rel_b = relative_uncertainty(other.value, other.sigma);
        UncertainValue {
            value: result,
            sigma: result.abs() * (rel_a.powi(2) + rel_b.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// (a ± σa) / (b ± σb) — relative uncertainties add in quadrature
    pub fn div(&self, other: &UncertainValue) -> UncertainValue {
        if other.value.abs() < 1e-15 {
            return UncertainValue {
                value: f64::NAN,
                sigma: f64::NAN,
                confidence: 0.0,
            };
        }
        let result = self.value / other.value;
        let rel_a = relative_uncertainty(self.value, self.sigma);
        let rel_b = relative_uncertainty(other.value, other.sigma);
        UncertainValue {
            value: result,
            sigma: result.abs() * (rel_a.powi(2) + rel_b.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// (a ± σa)^n — σ_result = |n × a^(n-1)| × σ_a
    pub fn powi(&self, n: i32) -> UncertainValue {
        let result = self.value.powi(n);
        let deriv = (n as f64) * self.value.powi(n - 1);
        UncertainValue {
            value: result,
            sigma: deriv.abs() * self.sigma,
            confidence: self.confidence,
        }
    }

    /// sqrt(a ± σa) — σ_result = σ_a / (2 × sqrt(a))
    pub fn sqrt(&self) -> UncertainValue {
        if self.value < 0.0 {
            return UncertainValue {
                value: f64::NAN,
                sigma: f64::NAN,
                confidence: 0.0,
            };
        }
        let result = self.value.sqrt();
        let sigma = if result > 0.0 {
            self.sigma / (2.0 * result)
        } else {
            0.0
        };
        UncertainValue {
            value: result,
            sigma,
            confidence: self.confidence,
        }
    }

    /// Scale by a constant (exact) — value × k, sigma × k
    pub fn scale(&self, k: f64) -> UncertainValue {
        UncertainValue {
            value: self.value * k,
            sigma: self.sigma * k.abs(),
            confidence: self.confidence,
        }
    }

    /// Add a constant (exact) — only value changes, sigma unchanged
    pub fn add_constant(&self, c: f64) -> UncertainValue {
        UncertainValue {
            value: self.value + c,
            sigma: self.sigma,
            confidence: self.confidence,
        }
    }

    /// Sum of independent values — for aggregating N independent measurements
    /// σ_sum = sqrt(Σ σ_i²)
    pub fn sum(values: &[UncertainValue]) -> UncertainValue {
        if values.is_empty() {
            return UncertainValue::certain(0.0);
        }
        let total: f64 = values.iter().map(|v| v.value).sum();
        let variance: f64 = values.iter().map(|v| v.sigma.powi(2)).sum();
        let min_conf = values.iter().map(|v| v.confidence).fold(1.0, f64::min);
        UncertainValue {
            value: total,
            sigma: variance.sqrt(),
            confidence: min_conf,
        }
    }

    /// Mean of N independent measurements — σ_mean = σ_sum / sqrt(N)
    pub fn mean(values: &[UncertainValue]) -> UncertainValue {
        if values.is_empty() {
            return UncertainValue::certain(0.0);
        }
        let n = values.len() as f64;
        let sum = Self::sum(values);
        UncertainValue {
            value: sum.value / n,
            sigma: sum.sigma / n.sqrt(),
            confidence: sum.confidence,
        }
    }
}

impl From<f64> for UncertainValue {
    fn from(v: f64) -> Self {
        Self::certain(v)
    }
}

/// Relative uncertainty (σ/|value|), with safe handling of zero values.
fn relative_uncertainty(value: f64, sigma: f64) -> f64 {
    if value.abs() > 1e-12 {
        sigma / value.abs()
    } else {
        0.0
    }
}

/// Convert a confidence level (0-1) to a z-score (standard deviations).
///
/// Uses the inverse normal CDF approximation (Beasley-Springer-Moro).
/// For the common cases we hard-code the values; otherwise we interpolate.
fn confidence_to_z(confidence: f64) -> f64 {
    match confidence {
        c if (c - 0.6827).abs() < 0.001 => 1.0,
        c if (c - 0.90).abs() < 0.001 => 1.6449,
        c if (c - 0.95).abs() < 0.001 => 1.96,
        c if (c - 0.99).abs() < 0.001 => 2.5758,
        c if (c - 0.9973).abs() < 0.001 => 3.0,
        c if c <= 0.0 => 0.0,
        c if c >= 1.0 => 5.0, // cap at 5-sigma
        _ => {
            // Linear interpolation between known points — not exact but
            // good enough for confidence display purposes
            if confidence < 0.6827 {
                confidence / 0.6827
            } else if confidence < 0.95 {
                1.0 + (confidence - 0.6827) / (0.95 - 0.6827) * (1.96 - 1.0)
            } else if confidence < 0.9973 {
                1.96 + (confidence - 0.95) / (0.9973 - 0.95) * (3.0 - 1.96)
            } else {
                3.0 + (confidence - 0.9973) / (1.0 - 0.9973) * 2.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certain_value() {
        let v = UncertainValue::certain(42.0);
        assert_eq!(v.value, 42.0);
        assert_eq!(v.sigma, 0.0);
        assert_eq!(v.confidence, 1.0);
        let (lo, hi) = v.ci_95();
        assert_eq!(lo, 42.0);
        assert_eq!(hi, 42.0);
    }

    #[test]
    fn test_from_sigma() {
        let v = UncertainValue::from_sigma(100.0, 2.0);
        assert_eq!(v.value, 100.0);
        assert_eq!(v.sigma, 2.0);
        let (lo, hi) = v.ci_95();
        assert!((lo - 96.08).abs() < 0.01);
        assert!((hi - 103.92).abs() < 0.01);
    }

    #[test]
    fn test_addition_propagation() {
        let a = UncertainValue::from_sigma(10.0, 1.0);
        let b = UncertainValue::from_sigma(20.0, 2.0);
        let c = a.add(&b);
        assert!((c.value - 30.0).abs() < 1e-9);
        // σ = sqrt(1² + 2²) = sqrt(5) ≈ 2.236
        assert!((c.sigma - 2.236).abs() < 0.001);
    }

    #[test]
    fn test_subtraction_propagation() {
        let a = UncertainValue::from_sigma(30.0, 1.0);
        let b = UncertainValue::from_sigma(10.0, 2.0);
        let c = a.sub(&b);
        assert!((c.value - 20.0).abs() < 1e-9);
        assert!((c.sigma - 2.236).abs() < 0.001);
    }

    #[test]
    fn test_multiplication_propagation() {
        let a = UncertainValue::from_sigma(10.0, 0.1); // 1% relative
        let b = UncertainValue::from_sigma(20.0, 0.4); // 2% relative
        let c = a.mul(&b);
        assert!((c.value - 200.0).abs() < 1e-9);
        // σ = 200 × sqrt(0.01² + 0.02²) = 200 × sqrt(0.0005) ≈ 4.472
        assert!((c.sigma - 4.472).abs() < 0.01);
    }

    #[test]
    fn test_division_propagation() {
        let a = UncertainValue::from_sigma(100.0, 1.0); // 1%
        let b = UncertainValue::from_sigma(10.0, 0.2); // 2%
        let c = a.div(&b);
        assert!((c.value - 10.0).abs() < 1e-9);
        // σ = 10 × sqrt(0.01² + 0.02²) ≈ 0.2236
        assert!((c.sigma - 0.2236).abs() < 0.001);
    }

    #[test]
    fn test_division_by_zero() {
        let a = UncertainValue::from_sigma(100.0, 1.0);
        let b = UncertainValue::certain(0.0);
        let c = a.div(&b);
        assert!(c.value.is_nan());
        assert!(c.sigma.is_nan());
        assert_eq!(c.confidence, 0.0);
    }

    #[test]
    fn test_sqrt_propagation() {
        let a = UncertainValue::from_sigma(100.0, 1.0);
        let s = a.sqrt();
        assert!((s.value - 10.0).abs() < 1e-9);
        // σ = 1 / (2 × 10) = 0.05
        assert!((s.sigma - 0.05).abs() < 1e-9);
    }

    #[test]
    fn test_sqrt_negative() {
        let a = UncertainValue::from_sigma(-4.0, 1.0);
        let s = a.sqrt();
        assert!(s.value.is_nan());
    }

    #[test]
    fn test_powi_propagation() {
        let a = UncertainValue::from_sigma(10.0, 0.5);
        let p = a.powi(2);
        assert!((p.value - 100.0).abs() < 1e-9);
        // σ = |2 × 10^1| × 0.5 = 10.0
        assert!((p.sigma - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_scale() {
        let a = UncertainValue::from_sigma(10.0, 1.0);
        let s = a.scale(3.0);
        assert_eq!(s.value, 30.0);
        assert_eq!(s.sigma, 3.0);
    }

    #[test]
    fn test_add_constant() {
        let a = UncertainValue::from_sigma(10.0, 1.0);
        let s = a.add_constant(5.0);
        assert_eq!(s.value, 15.0);
        assert_eq!(s.sigma, 1.0); // unchanged
    }

    #[test]
    fn test_sum_of_independent() {
        let values = vec![
            UncertainValue::from_sigma(10.0, 1.0),
            UncertainValue::from_sigma(20.0, 2.0),
            UncertainValue::from_sigma(30.0, 3.0),
        ];
        let s = UncertainValue::sum(&values);
        assert!((s.value - 60.0).abs() < 1e-9);
        // σ = sqrt(1 + 4 + 9) = sqrt(14) ≈ 3.742
        assert!((s.sigma - 3.742).abs() < 0.001);
    }

    #[test]
    fn test_mean_reduces_uncertainty() {
        let values = vec![
            UncertainValue::from_sigma(100.0, 2.0),
            UncertainValue::from_sigma(100.0, 2.0),
            UncertainValue::from_sigma(100.0, 2.0),
            UncertainValue::from_sigma(100.0, 2.0),
        ];
        let m = UncertainValue::mean(&values);
        assert!((m.value - 100.0).abs() < 1e-9);
        // σ_mean = σ_sum / sqrt(N) = 4.0 / 2.0 = 2.0
        // σ_sum = sqrt(4 × 4) = 4.0
        assert!((m.sigma - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_format_with_units() {
        let v = UncertainValue::from_sigma(12345.678, 3.0);
        let s = v.format_with_units("m³");
        assert!(s.contains("12345"));
        assert!(s.contains("m³"));
        assert!(s.contains("68%"));
    }

    #[test]
    fn test_confidence_to_z() {
        assert!((confidence_to_z(0.6827) - 1.0).abs() < 1e-6);
        assert!((confidence_to_z(0.95) - 1.96).abs() < 1e-4);
        assert!((confidence_to_z(0.9973) - 3.0).abs() < 1e-6);
        assert!((confidence_to_z(0.0)).abs() < 1e-9);
    }

    #[test]
    fn test_volume_propagation_example() {
        // Example from QA_QC_ANALYSIS.md:
        // 1000 cells, 1m² each, σ_z = 0.1m
        // σ_volume = sqrt(N) × cell_area × σ_z = sqrt(1000) × 1 × 0.1 ≈ 3.16 m³
        let n_cells = 1000usize;
        let cell_area = UncertainValue::certain(1.0); // 1 m², exact
        let sigma_z = 0.1; // 1-sigma vertical uncertainty per cell
        let per_cell_volume_sigma = cell_area.scale(sigma_z); // 1 × 0.1 = 0.1 m³ per cell
        // Sum of N independent cells: σ_sum = sqrt(N) × σ_per_cell
        let sum_sigma = per_cell_volume_sigma.sigma * (n_cells as f64).sqrt();
        assert!((sum_sigma - 3.1623).abs() < 0.001);
    }
}
