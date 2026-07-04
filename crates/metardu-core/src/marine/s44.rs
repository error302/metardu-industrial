// IHO S-44 (6th edition, 2022) compliance checker — pure Rust.
//
// Reference: IHO Standards for Hydrographic Surveys, 6th edition (2022)
// https://iho.int/uploads/user/pubs/standards/s-44/S-44_Edition_6.1.0_.pdf
//
// S-44 defines 4 survey orders with progressively tighter requirements:
//   - Special Order: harbors, berthing areas, critical channels (shallow, very tight)
//   - Order 1a: harbors, harbor approaches, coastal routes (shallow-to-medium)
//   - Order 1b: same areas as 1a but with less strict seafloor coverage
//   - Order 2: areas not covered above (open ocean, deep water)
//
// Each order has:
//   - Maximum horizontal uncertainty (95% confidence)
//   - Maximum vertical uncertainty (95% confidence)
//   - Minimum feature detection requirements (for Special Order and Order 1a)
//   - Full bottom search requirements (for Special Order and Order 1a)
//
// The vertical uncertainty formula combines a depth-proportional term
// and a constant term: σ_95 = sqrt(a² + (b × d)²)
//   where a = constant, b = depth-proportional coefficient, d = depth

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum S44Order {
    Special,
    Order1a,
    Order1b,
    Order2,
}

impl S44Order {
    /// Vertical uncertainty threshold (95% confidence) in meters.
    /// Formula: sqrt(a² + (b × d)²)
    #[allow(dead_code)]
    pub fn vertical_threshold(self, depth: f64) -> f64 {
        let (a, b) = match self {
            S44Order::Special => (0.25, 0.0075),
            S44Order::Order1a => (0.25, 0.0075),
            S44Order::Order1b => (0.25, 0.0075),
            S44Order::Order2 => (0.50, 0.013),
        };
        (a * a + (b * depth).powi(2)).sqrt()
    }

    /// Horizontal uncertainty threshold (95% confidence) in meters.
    pub fn horizontal_threshold(self) -> f64 {
        match self {
            S44Order::Special => 2.0,
            S44Order::Order1a => 5.0,
            S44Order::Order1b => 5.0,
            S44Order::Order2 => 20.0,
        }
    }

    /// Minimum feature detection size (cubic objects) in meters.
    /// None means no feature detection requirement.
    #[allow(dead_code)]
    pub fn feature_detection_size(self) -> Option<f64> {
        match self {
            S44Order::Special => Some(1.0),
            S44Order::Order1a => Some(2.0),
            S44Order::Order1b => None,
            S44Order::Order2 => None,
        }
    }

    /// Whether full bottom search is required.
    #[allow(dead_code)]
    pub fn requires_full_search(self) -> bool {
        matches!(self, S44Order::Special | S44Order::Order1a)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct S44ComplianceResult {
    pub target_order: S44Order,
    pub total_soundings: usize,
    pub passing_soundings: usize,
    pub failing_soundings: usize,
    pub pass_rate: f64, // 0.0–1.0
    pub status: S44Status,
    /// Per-sounding pass/fail (parallel to input)
    pub is_compliant: Vec<bool>,
    /// Per-sounding vertical TPU vs threshold
    pub vertical_margins: Vec<f64>,
    /// Per-sounding horizontal TPU vs threshold
    pub horizontal_margins: Vec<f64>,
    /// Depth statistics
    pub min_depth: f64,
    pub max_depth: f64,
    pub mean_depth: f64,
    /// Worst failing soundings (highest margin violation)
    pub worst_failures: Vec<S44Failure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct S44Failure {
    pub index: usize,
    pub depth: f64,
    pub vertical_tpu_95: f64,
    pub vertical_threshold: f64,
    pub horizontal_tpu_95: f64,
    pub horizontal_threshold: f64,
    pub violation: S44ViolationType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum S44ViolationType {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum S44Status {
    Pass,
    Investigate,
    Fail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S44CheckInput {
    pub depth: f64,
    pub vertical_tpu_95: f64,
    pub horizontal_tpu_95: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum S44Error {
    #[error("empty sounding set")]
    Empty,
    #[error("invalid depth: {0} (must be positive)")]
    InvalidDepth(f64),
}

/// Check S-44 compliance for a batch of soundings against a target order.
pub fn check_compliance(
    soundings: &[S44CheckInput],
    target_order: S44Order,
) -> Result<S44ComplianceResult, S44Error> {
    if soundings.is_empty() {
        return Err(S44Error::Empty);
    }

    let mut is_compliant = Vec::with_capacity(soundings.len());
    let mut vertical_margins = Vec::with_capacity(soundings.len());
    let mut horizontal_margins = Vec::with_capacity(soundings.len());
    let mut failures = Vec::new();

    let mut passing = 0usize;
    let mut min_depth = f64::INFINITY;
    let mut max_depth = f64::NEG_INFINITY;
    let mut depth_sum = 0.0f64;

    for (i, s) in soundings.iter().enumerate() {
        if s.depth <= 0.0 {
            return Err(S44Error::InvalidDepth(s.depth));
        }

        min_depth = min_depth.min(s.depth);
        max_depth = max_depth.max(s.depth);
        depth_sum += s.depth;

        let v_thresh = target_order.vertical_threshold(s.depth);
        let h_thresh = target_order.horizontal_threshold();

        let v_ok = s.vertical_tpu_95 <= v_thresh;
        let h_ok = s.horizontal_tpu_95 <= h_thresh;
        let ok = v_ok && h_ok;

        is_compliant.push(ok);
        vertical_margins.push(v_thresh - s.vertical_tpu_95);
        horizontal_margins.push(h_thresh - s.horizontal_tpu_95);

        if ok {
            passing += 1;
        } else {
            let violation = match (v_ok, h_ok) {
                (false, true) => S44ViolationType::Vertical,
                (true, false) => S44ViolationType::Horizontal,
                (false, false) => S44ViolationType::Both,
                _ => S44ViolationType::Vertical, // unreachable
            };
            failures.push(S44Failure {
                index: i,
                depth: s.depth,
                vertical_tpu_95: s.vertical_tpu_95,
                vertical_threshold: v_thresh,
                horizontal_tpu_95: s.horizontal_tpu_95,
                horizontal_threshold: h_thresh,
                violation,
            });
        }
    }

    let total = soundings.len();
    let failing = total - passing;
    let pass_rate = passing as f64 / total as f64;

    // Sort failures by worst margin violation (descending).
    // f64 doesn't implement Ord so sort_by_key isn't usable directly;
    // sort_by with partial_cmp is the idiomatic approach.
    let mut failures_with_margin: Vec<(f64, S44Failure)> = failures
        .into_iter()
        .map(|f| {
            let margin = (f.vertical_tpu_95 - f.vertical_threshold)
                .max(f.horizontal_tpu_95 - f.horizontal_threshold);
            (margin, f)
        })
        .collect();
    #[allow(unknown_lints, clippy::manual_sort_by)]
    failures_with_margin.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let worst_failures: Vec<S44Failure> = failures_with_margin
        .into_iter()
        .take(20)
        .map(|(_, f)| f)
        .collect();

    let status = if pass_rate >= 0.95 {
        S44Status::Pass
    } else if pass_rate >= 0.80 {
        S44Status::Investigate
    } else {
        S44Status::Fail
    };

    Ok(S44ComplianceResult {
        target_order,
        total_soundings: total,
        passing_soundings: passing,
        failing_soundings: failing,
        pass_rate,
        status,
        is_compliant,
        vertical_margins,
        horizontal_margins,
        min_depth,
        max_depth,
        mean_depth: depth_sum / total as f64,
        worst_failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_soundings(depth: f64, v_tpu: f64, h_tpu: f64, n: usize) -> Vec<S44CheckInput> {
        vec![
            S44CheckInput {
                depth,
                vertical_tpu_95: v_tpu,
                horizontal_tpu_95: h_tpu,
            };
            n
        ];
        std::iter::repeat_n(
            S44CheckInput {
                depth,
                vertical_tpu_95: v_tpu,
                horizontal_tpu_95: h_tpu,
            },
            n,
        )
        .collect()
    }

    #[test]
    fn test_special_order_pass() {
        // 10m depth, Special Order threshold = sqrt(0.25² + (0.0075×10)²) ≈ 0.27m
        // Horizontal threshold = 2m
        let soundings = make_soundings(10.0, 0.20, 1.5, 100);
        let result = check_compliance(&soundings, S44Order::Special).unwrap();
        assert_eq!(result.status, S44Status::Pass);
        assert_eq!(result.passing_soundings, 100);
    }

    #[test]
    fn test_special_order_fail_vertical() {
        // 10m depth, TPU exceeds vertical threshold
        let soundings = make_soundings(10.0, 0.50, 1.5, 100); // 0.5m > 0.27m threshold
        let result = check_compliance(&soundings, S44Order::Special).unwrap();
        assert_eq!(result.status, S44Status::Fail);
        assert_eq!(result.failing_soundings, 100);
    }

    #[test]
    fn test_order2_more_lenient() {
        // Same TPU that fails Special Order passes Order 2
        let soundings = make_soundings(10.0, 0.50, 1.5, 100);
        let special = check_compliance(&soundings, S44Order::Special).unwrap();
        let order2 = check_compliance(&soundings, S44Order::Order2).unwrap();
        assert_eq!(special.status, S44Status::Fail);
        assert_eq!(order2.status, S44Status::Pass);
    }

    #[test]
    fn test_threshold_formula() {
        // Special Order at 10m: sqrt(0.25² + 0.075²) = sqrt(0.0625 + 0.005625) ≈ 0.2602
        let threshold = S44Order::Special.vertical_threshold(10.0);
        assert!((threshold - 0.2602).abs() < 0.001, "threshold {threshold}");
    }

    #[test]
    fn test_empty_errors() {
        let result = check_compliance(&[], S44Order::Special);
        assert!(matches!(result, Err(S44Error::Empty)));
    }
}
