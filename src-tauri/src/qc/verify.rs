// Calculation verification framework — runs a primary and a secondary
// independent calculation, compares results, and flags disagreement.
//
// Use this to wrap every critical calculation in MetaRDU so that a bug
// in the primary method is caught by disagreement with the secondary.
//
// Example:
//   let result = verify_calculation(
//     || compute_volume_grid(dem, reference, cell_size),
//     || compute_volume_tin(dem, reference),
//     0.5,  // 0.5% tolerance
//     "stockpile volume",
//   );
//   if !result.agreement {
//     log::warn!("Cross-check failed: {}", result.warnings[0]);
//   }

use serde::{Deserialize, Serialize};

/// The result of a verified calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedCalculation {
    /// The primary (authoritative) value
    pub value: f64,
    /// The secondary (cross-check) value
    pub cross_check_value: f64,
    /// True if |primary - secondary| <= tolerance
    pub agreement: bool,
    /// The absolute tolerance used
    pub tolerance: f64,
    /// The relative difference as a percentage (|primary - secondary| / max(|primary|, |secondary|) × 100)
    pub relative_diff_pct: f64,
    /// Warnings (empty if agreement is true)
    pub warnings: Vec<String>,
    /// Description of the calculation (for logging)
    pub description: String,
}

/// Run two independent calculations and verify they agree.
///
/// `tolerance_pct` is the maximum allowed relative difference as a
/// percentage. For volume calcs, 0.5% is typical (grid vs TIN methods
/// differ by ~0.1-0.3% on well-conditioned data).
///
/// The primary calculation's return value is the authoritative one —
/// the secondary is only used for verification.
pub fn verify_calculation<T: Into<f64> + Copy>(
    primary: impl FnOnce() -> T,
    secondary: impl FnOnce() -> T,
    tolerance_pct: f64,
    description: &str,
) -> VerifiedCalculation {
    let primary_value: f64 = primary().into();
    let secondary_value: f64 = secondary().into();
    let diff = (primary_value - secondary_value).abs();
    let max_abs = primary_value.abs().max(secondary_value.abs());
    let relative_diff_pct = if max_abs > 1e-12 {
        (diff / max_abs) * 100.0
    } else if diff < 1e-12 {
        0.0
    } else {
        f64::INFINITY
    };
    let tolerance = (max_abs * tolerance_pct / 100.0).max(1e-9);
    let agreement = diff <= tolerance;
    let warnings = if !agreement {
        vec![format!(
            "Cross-check failed for '{}': primary={:.6}, secondary={:.6}, diff={:.6}, tolerance={:.6} ({:.4}%)",
            description, primary_value, secondary_value, diff, tolerance, relative_diff_pct
        )]
    } else {
        vec![]
    };
    VerifiedCalculation {
        value: primary_value,
        cross_check_value: secondary_value,
        agreement,
        tolerance,
        relative_diff_pct,
        warnings,
        description: description.to_string(),
    }
}

/// Convenience: verify that two values agree within a relative tolerance.
/// Returns true if they agree, false otherwise. Use this for inline
/// checks where you don't need the full VerifiedCalculation report.
pub fn values_agree(a: f64, b: f64, tolerance_pct: f64) -> bool {
    let diff = (a - b).abs();
    let max_abs = a.abs().max(b.abs());
    if max_abs < 1e-12 {
        return diff < 1e-12;
    }
    let rel = (diff / max_abs) * 100.0;
    rel <= tolerance_pct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agreement_within_tolerance() {
        let result = verify_calculation(|| 100.0, || 100.4, 0.5, "test");
        assert!(result.agreement);
        assert!(result.warnings.is_empty());
        assert!((result.relative_diff_pct - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_disagreement_beyond_tolerance() {
        let result = verify_calculation(|| 100.0, || 101.0, 0.5, "test");
        assert!(!result.agreement);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Cross-check failed"));
        assert!((result.relative_diff_pct - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_both_zero_agrees() {
        let result = verify_calculation(|| 0.0, || 0.0, 0.5, "test");
        assert!(result.agreement);
    }

    #[test]
    fn test_one_zero_one_nonzero_disagrees() {
        let result = verify_calculation(|| 0.0, || 1.0, 0.5, "test");
        assert!(!result.agreement);
    }

    #[test]
    fn test_negative_values() {
        let result = verify_calculation(|| -100.0, || -100.4, 0.5, "test");
        assert!(result.agreement);
    }

    #[test]
    fn test_values_agree_helper() {
        assert!(values_agree(100.0, 100.4, 0.5));
        assert!(!values_agree(100.0, 101.0, 0.5));
        assert!(values_agree(0.0, 0.0, 0.5));
    }

    #[test]
    fn test_description_preserved() {
        let result = verify_calculation(|| 1.0, || 1.0, 0.1, "stockpile volume");
        assert_eq!(result.description, "stockpile volume");
    }
}
