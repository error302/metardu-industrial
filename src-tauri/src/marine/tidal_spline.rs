// Tidal Spline Interpolator — Marine survey depth correction.
//
// The #2 bottleneck for marine surveyors: they collect raw sonar depths
// that need tide correction. Currently this means manual Excel work or
// custom Python scripts to match sonar timestamps with tide gauge logs.
//
// This tool takes:
//   1. A sonar depth CSV (timestamp, raw_depth)
//   2. A tide gauge CSV (timestamp, tide_level)
//
// And outputs a corrected depth CSV (timestamp, raw_depth, tide_level,
// corrected_depth) where corrected_depth = raw_depth + tide_level.
//
// The tide correction uses cubic spline interpolation to compute the
// tide level at each sonar ping's exact timestamp — no more manual
// spreadsheet lookup + linear interpolation.
//
// No AI. Pure deterministic spline interpolation.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalCorrectionRequest {
    /// Path to the sonar depth CSV file.
    /// Format: timestamp_unix_secs, raw_depth_m
    pub sonar_csv_path: String,
    /// Path to the tide gauge CSV file.
    /// Format: timestamp_unix_secs, tide_level_m
    pub tide_csv_path: String,
    /// Output path for the corrected CSV.
    /// Format: timestamp_unix_secs, raw_depth_m, tide_level_m, corrected_depth_m
    pub output_csv_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TidalCorrectionResult {
    /// Number of sonar pings corrected
    pub pings_corrected: usize,
    /// Number of tide gauge readings used
    pub tide_readings: usize,
    /// Min tide level (m)
    pub min_tide_m: f64,
    /// Max tide level (m)
    pub max_tide_m: f64,
    /// Mean tide level (m)
    pub mean_tide_m: f64,
    /// Min corrected depth (m)
    pub min_corrected_depth_m: f64,
    /// Max corrected depth (m)
    pub max_corrected_depth_m: f64,
    /// Output file path
    pub output_path: String,
    /// Warnings (e.g., sonar timestamps outside tide range)
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TidalError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV parse error in {file}: {line}")]
    CsvParse { file: String, line: String },
    #[error("sonar CSV has no valid rows")]
    EmptySonar,
    #[error("tide CSV has no valid rows")]
    EmptyTide,
    #[error("tide CSV needs at least 4 points for cubic spline")]
    TooFewTidePoints,
}

/// Run the tidal correction.
///
/// Reads both CSVs, builds a cubic spline from the tide data, and
/// applies it to each sonar ping timestamp. Outputs a corrected CSV.
pub fn run_tidal_correction(
    request: &TidalCorrectionRequest,
) -> Result<TidalCorrectionResult, String> {
    // Parse sonar CSV
    let sonar_data = parse_csv_pairs(&request.sonar_csv_path)
        .map_err(|e| ctx!("parsing sonar CSV", request.sonar_csv_path, e))?;
    if sonar_data.is_empty() {
        return Err("sonar CSV has no valid rows".into());
    }

    // Parse tide CSV
    let tide_data = parse_csv_pairs(&request.tide_csv_path)
        .map_err(|e| ctx!("parsing tide CSV", request.tide_csv_path, e))?;
    if tide_data.is_empty() {
        return Err("tide CSV has no valid rows".into());
    }
    if tide_data.len() < 4 {
        return Err("tide CSV needs at least 4 points for cubic spline".into());
    }

    // Sort tide data by timestamp (required for spline)
    let mut tide_sorted = tide_data.clone();
    tide_sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Build cubic spline
    let spline = CubicSpline::new(&tide_sorted)?;

    // Apply correction to each sonar ping
    let mut warnings = Vec::new();
    let mut output_lines = String::new();
    output_lines.push_str("timestamp_unix_secs,raw_depth_m,tide_level_m,corrected_depth_m\n");

    let mut min_tide = f64::INFINITY;
    let mut max_tide = f64::NEG_INFINITY;
    let mut sum_tide = 0.0f64;
    let mut min_corrected = f64::INFINITY;
    let mut max_corrected = f64::NEG_INFINITY;
    let mut count = 0usize;

    let tide_min_ts = tide_sorted.first().unwrap().0;
    let tide_max_ts = tide_sorted.last().unwrap().0;

    for (ts, raw_depth) in &sonar_data {
        // Clamp timestamps outside tide range (with warning)
        let clamped_ts = if *ts < tide_min_ts {
            if warnings.is_empty() {
                warnings.push(format!(
                    "some sonar pings are before tide range (tide starts at {}, sonar starts at {})",
                    tide_min_ts, sonar_data.first().unwrap().0
                ));
            }
            tide_min_ts
        } else if *ts > tide_max_ts {
            if warnings.len() < 2 {
                warnings.push(format!(
                    "some sonar pings are after tide range (tide ends at {}, sonar ends at {})",
                    tide_max_ts, sonar_data.last().unwrap().0
                ));
            }
            tide_max_ts
        } else {
            *ts
        };

        let tide_level = spline.interpolate(clamped_ts);
        let corrected = raw_depth + tide_level;

        output_lines.push_str(&format!(
            "{:.3},{:.4},{:.4},{:.4}\n",
            ts, raw_depth, tide_level, corrected
        ));

        min_tide = min_tide.min(tide_level);
        max_tide = max_tide.max(tide_level);
        sum_tide += tide_level;
        min_corrected = min_corrected.min(corrected);
        max_corrected = max_corrected.max(corrected);
        count += 1;
    }

    // Write output
    std::fs::write(&request.output_csv_path, output_lines)
        .map_err(|e| ctx!("writing corrected CSV", request.output_csv_path, e))?;

    Ok(TidalCorrectionResult {
        pings_corrected: count,
        tide_readings: tide_data.len(),
        min_tide_m: min_tide,
        max_tide_m: max_tide,
        mean_tide_m: if count > 0 { sum_tide / count as f64 } else { 0.0 },
        min_corrected_depth_m: min_corrected,
        max_corrected_depth_m: max_corrected,
        output_path: request.output_csv_path.clone(),
        warnings,
    })
}

/// Parse a CSV file with two numeric columns (timestamp, value).
/// Skips header rows and blank lines.
fn parse_csv_pairs(path: &str) -> Result<Vec<(f64, f64)>, TidalError> {
    let content = std::fs::read_to_string(Path::new(path))?;
    let mut data = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Skip header rows (non-numeric first field)
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 2 {
            continue;
        }

        let ts = match parts[0].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                // Skip header row (first line only)
                if i == 0 {
                    continue;
                }
                return Err(TidalError::CsvParse {
                    file: path.to_string(),
                    line: format!("line {}: cannot parse timestamp '{}'", i + 1, parts[0].trim()),
                });
            }
        };

        let val = match parts[1].trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                if i == 0 {
                    continue;
                }
                return Err(TidalError::CsvParse {
                    file: path.to_string(),
                    line: format!("line {}: cannot parse value '{}'", i + 1, parts[1].trim()),
                });
            }
        };

        data.push((ts, val));
    }

    Ok(data)
}

/// Natural cubic spline interpolation.
///
/// Given N data points (x_i, y_i) sorted by x, computes a piecewise
/// cubic polynomial that passes through all points with continuous
/// first and second derivatives. Used for smooth tide curve
/// interpolation between gauge readings.
struct CubicSpline {
    /// X values (timestamps)
    xs: Vec<f64>,
    /// Y values (tide levels)
    ys: Vec<f64>,
    /// Second derivatives at each point (computed by tridiagonal solve)
    y2: Vec<f64>,
}

impl CubicSpline {
    fn new(points: &[(f64, f64)]) -> Result<Self, String> {
        let n = points.len();
        if n < 4 {
            return Err("need at least 4 points for cubic spline".into());
        }

        let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = points.iter().map(|p| p.1).collect();

        // Natural spline: second derivative = 0 at endpoints
        let mut y2 = vec![0.0f64; n];
        let mut u = vec![0.0f64; n];

        for i in 1..n - 1 {
            let sig = (xs[i] - xs[i - 1]) / (xs[i + 1] - xs[i - 1]);
            let p = sig * y2[i - 1] + 2.0;
            y2[i] = (sig - 1.0) / p;
            u[i] = (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i])
                - (ys[i] - ys[i - 1]) / (xs[i] - xs[i - 1]);
            u[i] = (6.0 * u[i] / (xs[i + 1] - xs[i - 1]) - sig * u[i - 1]) / p;
        }

        // Back-substitution
        for i in (0..n - 1).rev() {
            y2[i] = y2[i] * y2[i + 1] + u[i];
        }

        Ok(Self { xs, ys, y2 })
    }

    /// Interpolate y at the given x value.
    /// Clamps to the spline range if x is outside.
    fn interpolate(&self, x: f64) -> f64 {
        let n = self.xs.len();

        // Clamp to range
        if x <= self.xs[0] {
            return self.ys[0];
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1];
        }

        // Binary search for the interval
        let mut lo = 0usize;
        let mut hi = n - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if self.xs[mid] > x {
                hi = mid;
            } else {
                lo = mid;
            }
        }

        let h = self.xs[hi] - self.xs[lo];
        if h == 0.0 {
            return self.ys[lo];
        }

        let a = (self.xs[hi] - x) / h;
        let b = (x - self.xs[lo]) / h;

        a * self.ys[lo] + b * self.ys[hi]
            + ((a * a * a - a) * self.y2[lo] + (b * b * b - b) * self.y2[hi]) * (h * h) / 6.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_csv(path: &Path, content: &str) {
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn test_parse_csv_pairs_basic() {
        let tmp = std::env::temp_dir().join("metardu_test_sonar.csv");
        write_csv(&tmp, "timestamp,depth\n1000.0,10.5\n1001.0,10.6\n1002.0,10.7\n");
        let data = parse_csv_pairs(tmp.to_str().unwrap()).unwrap();
        assert_eq!(data.len(), 3);
        assert!((data[0].0 - 1000.0).abs() < 0.001);
        assert!((data[0].1 - 10.5).abs() < 0.001);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_csv_pairs_skips_comments() {
        let tmp = std::env::temp_dir().join("metardu_test_tide.csv");
        write_csv(&tmp, "# comment\n1000.0,1.5\n\n1001.0,1.6\n");
        let data = parse_csv_pairs(tmp.to_str().unwrap()).unwrap();
        assert_eq!(data.len(), 2);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_cubic_spline_interpolation() {
        // Sine wave: y = sin(x) at x = 0, π/2, π, 3π/2, 2π
        let points = vec![
            (0.0, 0.0),
            (std::f64::consts::FRAC_PI_2, 1.0),
            (std::f64::consts::PI, 0.0),
            (3.0 * std::f64::consts::FRAC_PI_2, -1.0),
            (2.0 * std::f64::consts::PI, 0.0),
        ];
        let spline = CubicSpline::new(&points).unwrap();

        // At x = π/4, sin = √2/2 ≈ 0.707
        let val = spline.interpolate(std::f64::consts::FRAC_PI_4);
        assert!(
            (val - 0.707).abs() < 0.15,
            "expected ≈0.707, got {}",
            val
        );

        // At x = π, should be exactly 0 (data point)
        let val2 = spline.interpolate(std::f64::consts::PI);
        assert!((val2 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cubic_spline_clamps_outside_range() {
        let points = vec![
            (0.0, 0.0),
            (1.0, 1.0),
            (2.0, 0.0),
            (3.0, 1.0),
            (4.0, 0.0),
        ];
        let spline = CubicSpline::new(&points).unwrap();

        // Before range — should return first y
        assert!((spline.interpolate(-1.0) - 0.0).abs() < 0.001);
        // After range — should return last y
        assert!((spline.interpolate(5.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cubic_spline_too_few_points() {
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)];
        let result = CubicSpline::new(&points);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_tidal_correction_full() {
        let sonar_path = std::env::temp_dir().join("metardu_test_sonar_full.csv");
        let tide_path = std::env::temp_dir().join("metardu_test_tide_full.csv");
        let output_path = std::env::temp_dir().join("metardu_test_corrected.csv");

        // Sonar: 10 pings from t=1000 to t=1009, depth ≈ 15m
        let mut sonar_content = String::from("timestamp,depth\n");
        for i in 0..10 {
            sonar_content.push_str(&format!("{}\n", 1000.0 + i as f64, 15.0 + (i as f64 * 0.01)));
        }
        // Fix: format string is wrong, let me just write it properly
        let mut sonar_content = String::from("timestamp,depth\n");
        for i in 0..10 {
            sonar_content.push_str(&format!("{:.1},{:.2}\n", 1000.0 + i as f64, 15.0 + i as f64 * 0.01));
        }
        write_csv(&sonar_path, &sonar_content);

        // Tide: 8 readings from t=1000 to t=1007, tide ranges 1.0-2.0m
        let mut tide_content = String::from("timestamp,tide\n");
        for i in 0..8 {
            tide_content.push_str(&format!(
                "{:.1},{:.2}\n",
                1000.0 + i as f64,
                1.0 + (i as f64 * 0.14)
            ));
        }
        write_csv(&tide_path, &tide_content);

        let request = TidalCorrectionRequest {
            sonar_csv_path: sonar_path.to_string_lossy().to_string(),
            tide_csv_path: tide_path.to_string_lossy().to_string(),
            output_csv_path: output_path.to_string_lossy().to_string(),
        };

        let result = run_tidal_correction(&request).unwrap();
        assert_eq!(result.pings_corrected, 10);
        assert_eq!(result.tide_readings, 8);
        assert!(result.min_tide_m >= 1.0);
        assert!(result.max_tide_m <= 2.0);

        // Verify output file exists
        assert!(output_path.exists());
        let output = std::fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("timestamp_unix_secs,raw_depth_m"));
        // Should have 10 data lines + 1 header
        assert_eq!(output.lines().count(), 11);

        let _ = std::fs::remove_file(&sonar_path);
        let _ = std::fs::remove_file(&tide_path);
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn test_run_tidal_correction_empty_sonar() {
        let sonar_path = std::env::temp_dir().join("metardu_test_empty_sonar.csv");
        write_csv(&sonar_path, "timestamp,depth\n");
        let tide_path = std::env::temp_dir().join("metardu_test_tide_ok.csv");
        write_csv(&tide_path, "1.0,1.0\n2.0,1.1\n3.0,1.2\n4.0,1.3\n");

        let request = TidalCorrectionRequest {
            sonar_csv_path: sonar_path.to_string_lossy().to_string(),
            tide_csv_path: tide_path.to_string_lossy().to_string(),
            output_csv_path: "/tmp/output.csv".into(),
        };
        let result = run_tidal_correction(&request);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&sonar_path);
        let _ = std::fs::remove_file(&tide_path);
    }

    #[test]
    fn test_run_tidal_correction_too_few_tide() {
        let sonar_path = std::env::temp_dir().join("metardu_test_sonar_ok.csv");
        write_csv(&sonar_path, "1000.0,15.0\n1001.0,15.1\n");
        let tide_path = std::env::temp_dir().join("metardu_test_short_tide.csv");
        write_csv(&tide_path, "1000.0,1.0\n1001.0,1.1\n1002.0,1.2\n");

        let request = TidalCorrectionRequest {
            sonar_csv_path: sonar_path.to_string_lossy().to_string(),
            tide_csv_path: tide_path.to_string_lossy().to_string(),
            output_csv_path: "/tmp/output.csv".into(),
        };
        let result = run_tidal_correction(&request);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&sonar_path);
        let _ = std::fs::remove_file(&tide_path);
    }
}
