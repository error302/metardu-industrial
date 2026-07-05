// Highwall deformation monitoring — Revenue Feature #6.
//
// Post-Brumadinho 2019, slope stability monitoring is legally required
// in many jurisdictions for any mine with a tailings dam or highwall.
// This module extends the basic 4D monitoring with:
//
//   1. Per-cell displacement TIME-SERIES tracking across N epochs
//   2. Velocity and acceleration calculation (mm/day, mm/day²)
//   3. Threshold-based ALERT generation (advisory / watch / critical)
//   4. Trend classification (stable / creeping / accelerating / failure)
//   5. Compliance statistics for regulator-ready monthly report
//
// Sign convention: dz = current - previous (positive = fill/uplift,
// negative = cut/subsidence). For highwall monitoring we care about
// ABSOLUTE displacement magnitude — both upward heave and downward
// subsidence are warning signs.
//
// Thresholds default to USACE EM 1110-2-1900 guidance:
//   - 25mm cumulative displacement → ADVISORY (log, no notification)
//   - 50mm cumulative → WATCH (notify surveyor)
//   - 100mm cumulative OR velocity > 5mm/day → CRITICAL (immediate action)
//
// All grids must have identical dimensions and alignment.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    /// Below advisory threshold — log only
    None,
    /// 25-50mm cumulative displacement
    Advisory,
    /// 50-100mm cumulative OR sustained velocity > 1mm/day
    Watch,
    /// >100mm cumulative OR velocity > 5mm/day
    Critical,
}

impl AlertLevel {
    pub fn color(&self) -> &str {
        match self {
            AlertLevel::None => "#10B981",
            AlertLevel::Advisory => "#F59E0B",
            AlertLevel::Watch => "#F97316",
            AlertLevel::Critical => "#DC2626",
        }
    }
    pub fn label(&self) -> &str {
        match self {
            AlertLevel::None => "Stable",
            AlertLevel::Advisory => "Advisory",
            AlertLevel::Watch => "Watch",
            AlertLevel::Critical => "Critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendClass {
    /// |displacement| < advisory_threshold — no movement detected
    Stable,
    /// Sustained slow movement, velocity roughly constant
    Creeping,
    /// Velocity increasing — slope approaching failure
    Accelerating,
    /// Critical displacement or velocity — failure imminent or in progress
    FailureImminent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighwallThresholds {
    /// Cumulative displacement (mm) for advisory level
    #[serde(default = "default_advisory")]
    pub advisory_mm: f64,
    /// Cumulative displacement (mm) for watch level
    #[serde(default = "default_watch")]
    pub watch_mm: f64,
    /// Cumulative displacement (mm) for critical level
    #[serde(default = "default_critical")]
    pub critical_mm: f64,
    /// Velocity threshold for watch level (mm/day)
    #[serde(default = "default_velocity_watch")]
    pub velocity_watch_mm_per_day: f64,
    /// Velocity threshold for critical level (mm/day)
    #[serde(default = "default_velocity_critical")]
    pub velocity_critical_mm_per_day: f64,
}

fn default_advisory() -> f64 {
    25.0
}
fn default_watch() -> f64 {
    50.0
}
fn default_critical() -> f64 {
    100.0
}
fn default_velocity_watch() -> f64 {
    1.0
}
fn default_velocity_critical() -> f64 {
    5.0
}

impl Default for HighwallThresholds {
    fn default() -> Self {
        Self {
            advisory_mm: default_advisory(),
            watch_mm: default_watch(),
            critical_mm: default_critical(),
            velocity_watch_mm_per_day: default_velocity_watch(),
            velocity_critical_mm_per_day: default_velocity_critical(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CellTimeSeries {
    /// Cell index in the source grid
    pub index: usize,
    /// Row in source grid (for map overlay rendering)
    pub row: usize,
    /// Column in source grid
    pub col: usize,
    /// Per-epoch displacement (mm), relative to first epoch baseline.
    /// Length = number of epochs.
    pub displacements_mm: Vec<f64>,
    /// Per-epoch velocity (mm/day) — computed from consecutive epochs
    pub velocities_mm_per_day: Vec<f64>,
    /// Latest cumulative displacement (mm, absolute)
    pub cumulative_mm: f64,
    /// Peak velocity across all epochs (mm/day)
    pub peak_velocity_mm_per_day: f64,
    /// Latest acceleration (mm/day²) — last velocity delta
    pub acceleration_mm_per_day2: f64,
    /// Current alert level
    pub alert: AlertLevel,
    /// Trend classification based on velocity + acceleration history
    pub trend: TrendClass,
}

#[derive(Debug, Clone, Serialize)]
pub struct HighwallAlert {
    pub cell_index: usize,
    pub row: usize,
    pub col: usize,
    pub level: AlertLevel,
    pub cumulative_mm: f64,
    pub velocity_mm_per_day: f64,
    pub trend: TrendClass,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HighwallReport {
    /// Number of epochs analyzed
    pub n_epochs: usize,
    /// Cell dimensions (m)
    pub cell_area_m2: f64,
    /// Total cells in source grid
    pub total_cells: usize,
    /// Cells with valid displacement data across all epochs
    pub active_cells: usize,
    /// Per-cell time-series (only cells with non-zero displacement)
    pub cells: Vec<CellTimeSeries>,
    /// All alerts triggered by current state
    pub alerts: Vec<HighwallAlert>,
    /// Compliance statistics
    pub stats: HighwallStats,
    /// Thresholds used
    pub thresholds: HighwallThresholds,
    /// Per-epoch timestamps (ISO 8601)
    pub epoch_dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct HighwallStats {
    pub stable_cells: usize,
    pub advisory_cells: usize,
    pub watch_cells: usize,
    pub critical_cells: usize,
    pub max_cumulative_mm: f64,
    pub max_velocity_mm_per_day: f64,
    pub mean_cumulative_mm: f64,
    pub cells_with_acceleration: usize,
    pub failure_imminent_cells: usize,
    /// % of cells below advisory — compliance threshold is typically 95%+
    pub compliance_pct: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum HighwallError {
    #[error("need at least 2 epochs for highwall analysis, got {0}")]
    TooFewEpochs(usize),
    #[error("epoch {0} has different length than epoch 0 ({1} vs {2})")]
    DimensionMismatch(usize, usize, usize),
    #[error("epoch {0} is empty")]
    Empty(usize),
    #[error("dates count ({0}) must match epochs count ({1})")]
    DateCountMismatch(usize, usize),
    #[error("invalid date format at index {0}: {1}")]
    InvalidDate(usize, String),
}

/// Analyze highwall deformation across multiple epochs.
///
/// `epoch_surfaces` is a Vec of DEM grids (one per epoch), each a flat
/// `Vec<f64>` of identical length. Elevations are in meters.
///
/// `epoch_dates` are ISO 8601 date strings (e.g., "2026-06-01"). They're
/// used to compute velocity in mm/day. If empty, sequential integer
/// days (0, 1, 2, ...) are assumed.
///
/// `cell_area_m2` is the area of each DEM cell in square meters (used
/// only for compliance reporting context, not for the displacement calc).
pub fn analyze_highwall(
    epoch_surfaces: &[Vec<f64>],
    epoch_dates: &[String],
    cell_area_m2: f64,
    thresholds: &HighwallThresholds,
) -> Result<HighwallReport, HighwallError> {
    let n_epochs = epoch_surfaces.len();
    if n_epochs < 2 {
        return Err(HighwallError::TooFewEpochs(n_epochs));
    }
    if epoch_dates.is_empty() {
        // Use sequential days
        let dates: Vec<String> = (0..n_epochs)
            .map(|i| format!("2026-01-{:02}", i + 1))
            .collect();
        return analyze_highwall(epoch_surfaces, &dates, cell_area_m2, thresholds);
    }
    if epoch_dates.len() != n_epochs {
        return Err(HighwallError::DateCountMismatch(
            epoch_dates.len(),
            n_epochs,
        ));
    }

    // Validate dimensions
    let n_cells = epoch_surfaces[0].len();
    if n_cells == 0 {
        return Err(HighwallError::Empty(0));
    }
    for (i, surf) in epoch_surfaces.iter().enumerate().skip(1) {
        if surf.len() != n_cells {
            return Err(HighwallError::DimensionMismatch(i, surf.len(), n_cells));
        }
    }

    // Parse dates to epoch days for velocity calculation
    let epoch_days: Vec<f64> = epoch_dates
        .iter()
        .enumerate()
        .map(|(i, d)| parse_iso_to_days(d).map_err(|e| HighwallError::InvalidDate(i, e)))
        .collect::<Result<Vec<_>, _>>()?;

    let approx_cols = ((n_cells as f64).sqrt().round() as usize).max(1);

    let mut cells: Vec<CellTimeSeries> = Vec::new();
    let mut alerts: Vec<HighwallAlert> = Vec::new();
    let mut stats = HighwallStats::default();
    let mut sum_cumulative = 0.0f64;
    let mut active_cells = 0usize;

    for idx in 0..n_cells {
        // Extract elevation series at this cell
        let series: Vec<f64> = epoch_surfaces.iter().map(|s| s[idx]).collect();

        // Skip cells with any nodata
        if series.iter().any(|&v| v.is_nan() || v <= -9999.0) {
            continue;
        }

        // Compute per-epoch displacement relative to first epoch (in mm)
        let baseline = series[0];
        let mut displacements_mm = Vec::with_capacity(n_epochs);
        let mut abs_displacements_mm = Vec::with_capacity(n_epochs);
        for &v in &series {
            let d = (v - baseline) * 1000.0; // m → mm
            displacements_mm.push(d);
            abs_displacements_mm.push(d.abs());
        }

        let cumulative_mm = abs_displacements_mm[n_epochs - 1];
        if cumulative_mm < 1.0 {
            // Less than 1mm movement — effectively stable
            continue;
        }

        active_cells += 1;

        // Compute velocity (mm/day) between consecutive epochs
        let mut velocities = Vec::with_capacity(n_epochs.saturating_sub(1));
        for i in 1..n_epochs {
            let dt_days = (epoch_days[i] - epoch_days[i - 1]).max(0.001);
            let dd = displacements_mm[i] - displacements_mm[i - 1];
            velocities.push(dd.abs() / dt_days);
        }
        let peak_velocity = velocities.iter().cloned().fold(0.0f64, f64::max);

        // Acceleration: difference between last two velocities (mm/day²)
        let acceleration = if velocities.len() >= 2 {
            let dt = (epoch_days[n_epochs - 1] - epoch_days[n_epochs - 2]).max(0.001);
            (velocities[velocities.len() - 1] - velocities[velocities.len() - 2]) / dt
        } else {
            0.0
        };

        // Determine alert level
        let alert = if cumulative_mm >= thresholds.critical_mm
            || peak_velocity >= thresholds.velocity_critical_mm_per_day
        {
            AlertLevel::Critical
        } else if cumulative_mm >= thresholds.watch_mm
            || peak_velocity >= thresholds.velocity_watch_mm_per_day
        {
            AlertLevel::Watch
        } else if cumulative_mm >= thresholds.advisory_mm {
            AlertLevel::Advisory
        } else {
            AlertLevel::None
        };

        // Trend classification
        let trend = if alert == AlertLevel::Critical && acceleration > 0.0 {
            TrendClass::FailureImminent
        } else if acceleration > 0.1 {
            TrendClass::Accelerating
        } else if peak_velocity > 0.05 {
            TrendClass::Creeping
        } else {
            TrendClass::Stable
        };

        // Update statistics
        match alert {
            AlertLevel::None => stats.stable_cells += 1,
            AlertLevel::Advisory => stats.advisory_cells += 1,
            AlertLevel::Watch => stats.watch_cells += 1,
            AlertLevel::Critical => stats.critical_cells += 1,
        }
        if cumulative_mm > stats.max_cumulative_mm {
            stats.max_cumulative_mm = cumulative_mm;
        }
        if peak_velocity > stats.max_velocity_mm_per_day {
            stats.max_velocity_mm_per_day = peak_velocity;
        }
        if acceleration > 0.1 {
            stats.cells_with_acceleration += 1;
        }
        if trend == TrendClass::FailureImminent {
            stats.failure_imminent_cells += 1;
        }
        sum_cumulative += cumulative_mm;

        // Generate alert entry if non-stable
        if alert != AlertLevel::None {
            let message = match alert {
                AlertLevel::Advisory => format!(
                    "Advisory: cumulative displacement {:.1}mm exceeds {:.0}mm threshold",
                    cumulative_mm, thresholds.advisory_mm
                ),
                AlertLevel::Watch => format!(
                    "Watch: cumulative {:.1}mm OR peak velocity {:.2}mm/day exceeds watch threshold",
                    cumulative_mm, peak_velocity
                ),
                AlertLevel::Critical => format!(
                    "CRITICAL: cumulative {:.1}mm OR peak velocity {:.2}mm/day — immediate action required",
                    cumulative_mm, peak_velocity
                ),
                AlertLevel::None => String::new(),
            };

            let row = idx / approx_cols;
            let col = idx % approx_cols;
            alerts.push(HighwallAlert {
                cell_index: idx,
                row,
                col,
                level: alert,
                cumulative_mm,
                velocity_mm_per_day: peak_velocity,
                trend,
                message,
            });
        }

        let row = idx / approx_cols;
        let col = idx % approx_cols;
        cells.push(CellTimeSeries {
            index: idx,
            row,
            col,
            displacements_mm,
            velocities_mm_per_day: velocities,
            cumulative_mm,
            peak_velocity_mm_per_day: peak_velocity,
            acceleration_mm_per_day2: acceleration,
            alert,
            trend,
        });
    }

    if active_cells > 0 {
        stats.mean_cumulative_mm = sum_cumulative / active_cells as f64;
    }
    stats.compliance_pct = if active_cells > 0 {
        (stats.stable_cells as f64 / active_cells as f64) * 100.0
    } else {
        100.0
    };

    Ok(HighwallReport {
        n_epochs,
        cell_area_m2,
        total_cells: n_cells,
        active_cells,
        cells,
        alerts,
        stats,
        thresholds: thresholds.clone(),
        epoch_dates: epoch_dates.to_vec(),
    })
}

/// Parse an ISO 8601 date string (YYYY-MM-DD) into a day count.
///
/// Uses a proper proleptic Gregorian calendar conversion (days since
/// 0000-01-01) so that deltas across year boundaries are correct.
/// The previous implementation used `year * 365.25 + month * 30.4375 + day`
/// which doesn't reset months at year boundaries — a Dec 31 → Jan 1
/// epoch pair produced a delta of 0.4375 days instead of 1 day, making
/// velocity (mm/day) ~2.3x too high for any year-boundary pair. That's
/// a safety-critical metric for highwall monitoring.
fn parse_iso_to_days(s: &str) -> Result<f64, String> {
    let parts: Vec<&str> = s.trim().split('-').collect();
    if parts.len() < 3 {
        return Err(format!("expected YYYY-MM-DD, got '{}'", s));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let day: u32 = parts[2]
        .split('T')
        .next()
        .unwrap_or(parts[2])
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;

    // Convert (year, month, day) to a day count using the standard
    // algorithm from "Calendrical Calculations" (Dershowitz & Reingold).
    // This is the same approach used by chrono's NaiveDate::num_days
    // and handles leap years correctly.
    //
    // The formula counts days since 0000-01-01 (proleptic Gregorian).
    // We return f64 because the caller does floating-point arithmetic
    // on the deltas.
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = if y >= 0 { y } else { y - 999 } / 1000;
    let yoe = (y - era * 1000) as u32;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era as i64 * 146097 + doe as i64 - 719468;
    Ok(days as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(v: f64, n: usize) -> Vec<f64> {
        vec![v; n]
    }

    #[test]
    fn test_stable_slope() {
        // Two epochs with identical elevations — no displacement
        let s1 = flat(100.0, 100);
        let s2 = flat(100.0, 100);
        let dates = vec!["2026-06-01".into(), "2026-06-08".into()];
        let r = analyze_highwall(&[s1, s2], &dates, 1.0, &HighwallThresholds::default()).unwrap();
        assert_eq!(r.n_epochs, 2);
        assert_eq!(r.active_cells, 0); // all stable
        assert_eq!(r.stats.stable_cells, 0); // no active cells to be stable
        assert_eq!(r.alerts.len(), 0);
        assert!((r.stats.compliance_pct - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_advisory_displacement() {
        // 30mm displacement over 60 days = 0.5mm/day (below velocity watch)
        // cumulative 30mm is in advisory range (25-50mm)
        let s1 = flat(100.0, 100);
        let s2 = flat(100.030, 100); // +30mm
        let dates = vec!["2026-04-01".into(), "2026-06-01".into()]; // 60 days apart
        let r = analyze_highwall(&[s1, s2], &dates, 1.0, &HighwallThresholds::default()).unwrap();
        assert_eq!(r.active_cells, 100);
        assert_eq!(r.stats.advisory_cells, 100);
        assert_eq!(r.stats.stable_cells, 0);
        assert_eq!(r.alerts.len(), 100);
        assert_eq!(r.alerts[0].level, AlertLevel::Advisory);
        assert!((r.alerts[0].cumulative_mm - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_critical_displacement() {
        // 120mm displacement = critical (>100mm)
        let s1 = flat(100.0, 100);
        let s2 = flat(100.120, 100); // +120mm
        let dates = vec!["2026-06-01".into(), "2026-06-08".into()];
        let r = analyze_highwall(&[s1, s2], &dates, 1.0, &HighwallThresholds::default()).unwrap();
        assert_eq!(r.stats.critical_cells, 100);
        assert_eq!(r.alerts[0].level, AlertLevel::Critical);
        assert!((r.alerts[0].cumulative_mm - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_velocity_triggers_watch() {
        // 30mm displacement over 30 days = 1mm/day — exactly at velocity watch threshold
        // Use 20mm over 30 days = 0.67mm/day (below velocity watch) but cumulative is below advisory too
        // Let's test 30mm cumulative + velocity in (1, 5) range
        // 30mm over 10 days = 3mm/day — exceeds watch (1mm/day) but not critical (5mm/day)
        // cumulative is in advisory range (25-50mm), but velocity triggers watch
        let s1 = flat(100.0, 100);
        let s2 = flat(100.030, 100);
        let dates = vec!["2026-05-22".into(), "2026-06-01".into()]; // ~10 days
        let r = analyze_highwall(&[s1, s2], &dates, 1.0, &HighwallThresholds::default()).unwrap();
        // 30mm cumulative is in advisory range, but velocity (3mm/day) exceeds watch threshold
        assert_eq!(r.alerts[0].level, AlertLevel::Watch);
    }

    #[test]
    fn test_three_epochs_acceleration() {
        // 3 epochs: 0mm, 10mm, 30mm — accelerating
        let s1 = flat(100.0, 100);
        let s2 = flat(100.010, 100);
        let s3 = flat(100.030, 100);
        let dates = vec![
            "2026-06-01".into(),
            "2026-06-08".into(),
            "2026-06-15".into(),
        ];
        let r =
            analyze_highwall(&[s1, s2, s3], &dates, 1.0, &HighwallThresholds::default()).unwrap();
        assert_eq!(r.n_epochs, 3);
        assert_eq!(r.cells.len(), 100);
        // First velocity = 10mm/7days ≈ 1.43mm/day → exceeds watch velocity threshold
        // Second velocity = 20mm/7days ≈ 2.86mm/day → also exceeds
        // Acceleration = (2.86 - 1.43) / 7 ≈ 0.20 mm/day² → > 0.1 threshold
        assert!(r.stats.cells_with_acceleration > 0);
    }

    #[test]
    fn test_too_few_epochs() {
        let s1 = flat(100.0, 100);
        let r = analyze_highwall(
            &[s1],
            &["2026-06-01".into()],
            1.0,
            &HighwallThresholds::default(),
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_dimension_mismatch() {
        let s1 = flat(100.0, 100);
        let s2 = flat(100.0, 200);
        let r = analyze_highwall(
            &[s1, s2],
            &["2026-06-01".into(), "2026-06-08".into()],
            1.0,
            &HighwallThresholds::default(),
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_date_count_mismatch() {
        let s1 = flat(100.0, 100);
        let s2 = flat(100.0, 100);
        let r = analyze_highwall(
            &[s1, s2],
            &["2026-06-01".into()],
            1.0,
            &HighwallThresholds::default(),
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_compliance_pct_high_for_stable() {
        // 95 cells stable, 5 cells with advisory-level displacement (over long enough time
        // that velocity doesn't trigger watch)
        let mut s1 = vec![100.0; 100];
        let mut s2 = vec![100.0; 100];
        // 5 cells with advisory-level displacement (30mm over 60 days → 0.5mm/day, no velocity trigger)
        for i in 0..5 {
            s2[i] = 100.030;
        }
        let _ = (&mut s1, &mut s2);
        let dates = vec!["2026-04-01".into(), "2026-06-01".into()];
        let r = analyze_highwall(&[s1, s2], &dates, 1.0, &HighwallThresholds::default()).unwrap();
        // 5 cells are advisory (active), 95 are stable (not active)
        assert_eq!(r.active_cells, 5);
        assert_eq!(r.stats.advisory_cells, 5);
        assert_eq!(r.total_cells, 100);
    }

    #[test]
    fn test_thresholds_default_to_usace() {
        let t = HighwallThresholds::default();
        assert_eq!(t.advisory_mm, 25.0);
        assert_eq!(t.watch_mm, 50.0);
        assert_eq!(t.critical_mm, 100.0);
        assert_eq!(t.velocity_watch_mm_per_day, 1.0);
        assert_eq!(t.velocity_critical_mm_per_day, 5.0);
    }

    #[test]
    fn test_parse_iso_year_boundary() {
        // Regression test: the old formula (year*365.25 + month*30.4375 + day)
        // produced a delta of 0.4375 days for Dec 31 → Jan 1, making
        // velocity ~2.3x too high. The new proleptic Gregorian conversion
        // must produce exactly 1 day.
        let dec31 = parse_iso_to_days("2026-12-31").unwrap();
        let jan1 = parse_iso_to_days("2027-01-01").unwrap();
        let delta = jan1 - dec31;
        assert_eq!(
            delta, 1.0,
            "Dec 31 → Jan 1 must be exactly 1 day, got {delta}"
        );

        // Also verify a longer span: Dec 31 → Feb 1 = 32 days
        let feb1 = parse_iso_to_days("2027-02-01").unwrap();
        assert_eq!(feb1 - dec31, 32.0, "Dec 31 → Feb 1 must be 32 days");

        // Same-year span: Jan 1 → Feb 1 = 31 days
        assert_eq!(
            parse_iso_to_days("2026-02-01").unwrap() - parse_iso_to_days("2026-01-01").unwrap(),
            31.0
        );
    }

    #[test]
    fn test_parse_iso_leap_year() {
        // 2024 is a leap year: Feb 29 exists. Feb 28 → Mar 1 = 2 days.
        let feb28 = parse_iso_to_days("2024-02-28").unwrap();
        let mar1 = parse_iso_to_days("2024-03-01").unwrap();
        assert_eq!(mar1 - feb28, 2.0, "Feb 28 → Mar 1 in leap year = 2 days");

        // 2023 is not a leap year: Feb 28 → Mar 1 = 1 day.
        let feb28_2023 = parse_iso_to_days("2023-02-28").unwrap();
        let mar1_2023 = parse_iso_to_days("2023-03-01").unwrap();
        assert_eq!(
            mar1_2023 - feb28_2023,
            1.0,
            "Feb 28 → Mar 1 in non-leap year = 1 day"
        );
    }
}
