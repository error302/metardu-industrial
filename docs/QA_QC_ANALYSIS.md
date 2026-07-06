# QA/QC Analysis — Calculation Checks & Error Propagation

**Audience**: Engineering lead, surveyor, regulator.
**Question**: Do MetaRDU's calculations have built-in checks? How do we prevent errors from propagating forward through the workflow?
**Date**: 2026-07-07

---

## Executive Summary

MetaRDU's calculation engine has **partial** QA/QC coverage. Several critical modules have unit tests (volume, change detection, NMEA, tide, S-44) but lack **runtime cross-checks** — that is, the production code computes a value once and returns it without verifying against an independent method. Error propagation is **not modeled** — when a volume calculation depends on a DEM that depends on a CSF classification that depends on raw LAS points, the uncertainty of the final volume is not reported to the user.

This document:
1. Audits the existing checks (where they exist, where they're missing)
2. Defines a **Calculation Verification Protocol** — every critical calc should have ≥2 independent implementations and they must agree
3. Defines an **Error Propagation Strategy** — track uncertainty through every transformation and report it on every output
4. Lists concrete code changes (Sprint 12 scope) to close the gaps

---

## Part 1: Audit of Existing Calculation Checks

### What's Already Checked (Good)

| Module | Check | Implementation | Status |
|---|---|---|---|
| `volume.rs::compute_volumes` | Unit tests verify fill+cut+net on synthetic grids | `#[test]` block | ✅ Tested |
| `change_detection.rs::detect_stockpile_change` | 4 unit tests: pure fill, pure cut, hotspots, invalid input | `#[test]` block | ✅ Tested |
| `survey_tools.rs::compute_setout` | Bearing/distance from origin to (100,100) = 45° / 141.42 m | `#[test]` | ✅ Tested |
| `survey_tools.rs::shoelace_area` | 10×10 square = 100 m² | `#[test]` | ✅ Tested |
| `survey_tools.rs::mine_grid_round_trip` | grid → CRS → grid returns original coords | `#[test]` | ✅ Tested |
| `realtime/nmea.rs::parse_sentence` | 8 tests: GGA, RMC, GLL, GLL S/W hemisphere, invalid checksum, unknown type, merge | `#[test]` | ✅ Tested |
| `realtime/tide.rs::parse_noaa_response` | 9 tests: interpolation, apply_to_soundings, ISO/Unix timestamp, leap year | `#[test]` | ✅ Tested |
| `marine/s44.rs::check_s44_compliance` | Per-order density + uncertainty targets | `#[test]` | ✅ Tested |
| `formats/kongsberg_all.rs` | Header magic byte check (0x49) | Runtime | ✅ Checked at runtime |
| `formats/las.rs::read_header` | LAS signature "LASF" check | Runtime | ✅ Checked at runtime |
| NTRIP RTCM3 | CRC-24Q verification (Sprint 9 security fix) | Runtime | ✅ Checked at runtime |
| License | RSA-PSS signature verification | Runtime | ✅ Checked at runtime |

### What's NOT Checked (Bad)

| Module | Missing Check | Risk |
|---|---|---|
| `volume.rs::compute_volumes` | No runtime cross-check. The fill + cut volumes are computed once and returned. There's no second method (e.g., TIN-based volume) computed independently and compared. | A bug in the grid-based method produces wrong volumes silently. |
| `change_detection.rs::detect_stockpile_change` | No cross-check. Net change is `fill - cut`, but it's not independently recomputed as `sum(delta_grid * cell_area)` and compared. | The headline net-change number could be inconsistent with the per-cell deltas. |
| `survey_tools.rs::compute_setout` | Bearing computed via `atan2(delta_e, delta_n)` — no quadrant check. | If `delta_e` or `delta_n` is NaN, bearing becomes NaN silently. |
| `survey_tools.rs::mine_grid_to_crs` | Round-trip test exists in unit tests but NOT at runtime. | A user could enter a grid definition that's mathematically invalid (e.g., scale_factor = 0) and the transform would return NaN silently. |
| `survey_tools.rs::analyze_tunnel_profile` | Shoelace area is computed once. No second method (e.g., triangulation) to verify. | A bug in shoelace (e.g., wrong point ordering) produces wrong areas. |
| `dem_render.rs` (hillshade) | Parallelized with rayon but no checksum or sanity check on the output range. | A bug could produce hillshade values outside [0, 255] silently. |
| `formats/las.rs::read_points` | Point count from header vs actual points read — not checked. | A truncated LAS file returns fewer points than advertised without warning. |
| `formats/geotiff.rs::read_dem_grid` | Pixel dimensions vs header dimensions — not checked. | A corrupt GeoTIFF returns wrong-sized grid silently. |
| `marine/tidal_datums.rs::convert_depths` | Offset applied to all depths, no range sanity check. | A typo'd offset (e.g., 100 m instead of 1.0 m) shifts all depths by 100 m silently. |
| `realtime/nmea.rs` (runtime) | Parsed position is not range-checked. | A corrupt NMEA sentence with lat=999° would propagate to the map. |
| `realtime/tide.rs` (runtime) | Tide level not range-checked against station's historical range. | A parsing bug returning 100 m tide would shift all soundings by 100 m. |
| **Error propagation** | **No module reports propagated uncertainty.** Volume result returns `f64`, not `f64 ± f64`. | The surveyor has no idea whether the volume is ±0.5 m³ or ±500 m³. |

---

## Part 2: Calculation Verification Protocol

### Principle: Every Critical Calc Gets a Cross-Check

For every calculation whose result drives a decision (volume for payment, S-44 for compliance, setout for construction), the engine should compute the result **two independent ways** and warn if they disagree beyond a tolerance.

### Protocol

```rust
/// Verification wrapper — runs a primary and secondary calculation,
/// compares results, and returns the primary with a verification flag.
pub struct VerifiedCalculation<T> {
    pub value: T,
    pub cross_check_value: T,
    pub agreement: bool,
    pub tolerance: f64,
    pub warnings: Vec<String>,
}

pub fn verify_calculation<T: PartialEq + Into<f64> + Copy>(
    primary: impl FnOnce() -> T,
    secondary: impl FnOnce() -> T,
    tolerance_pct: f64,
    description: &str,
) -> VerifiedCalculation<T> {
    let primary_value = primary();
    let secondary_value = secondary();
    let p: f64 = primary_value.into();
    let s: f64 = secondary_value.into();
    let diff = (p - s).abs();
    let tol = (p.abs().max(s.abs()) * tolerance_pct / 100.0).max(1e-9);
    let agreement = diff <= tol;
    let warnings = if !agreement {
        vec![format!(
            "Cross-check failed for {}: primary={:?}, secondary={:?}, diff={}, tolerance={}",
            description, primary_value, secondary_value, diff, tol
        )]
    } else {
        vec![]
    };
    VerifiedCalculation {
        value: primary_value,
        cross_check_value: secondary_value,
        agreement,
        tolerance: tol,
        warnings,
    }
}
```

### Concrete Cross-Checks to Add (Sprint 12)

| Calculation | Primary Method | Secondary Method | Tolerance |
|---|---|---|---|
| Stockpile volume (grid) | Sum of cell × Δz | TIN-based volume (triangulate the DEM, integrate against reference) | 0.5% |
| Stockpile change (cut/fill) | Sum of cut cells + sum of fill cells | Sum of all Δz × cell_area | 0.1 m³ |
| Tunnel profile area | Shoelace | Sum of triangles from centroid | 0.01 m² |
| Setout bearing | `atan2(ΔE, ΔN)` | Quadrant-based arctan | 0.001° |
| Setout distance | `sqrt(ΔE² + ΔN²)` | Haversine (treating as lat/lon) for sanity | 0.01% |
| Mine grid transform | Forward formula | Inverse formula then compare | 1 mm |
| Tide correction | Linear interpolation | Cubic spline interpolation, compare at sample points | 0.01 m |
| NMEA position | GGA-derived lat/lon | RMC-derived lat/lon, if both present | 0.001° |
| LAS point count | Header's point count | Actual points read | Exact match |
| GeoTIFF dimensions | Header width × length | Actual pixel data size | Exact match |

### Implementation Plan

Add `src-tauri/src/qc/` module:
```
qc/
├── mod.rs                  // VerifiedCalculation<T> struct + verify_calculation()
├── volume_checks.rs        // TIN-based volume as cross-check for grid-based
├── geometry_checks.rs      // Independent area, bearing, distance methods
├── coordinate_checks.rs    // Round-trip, range, and residual checks
└── propagation.rs          // Error propagation (see Part 3)
```

---

## Part 3: Error Propagation Strategy

### Principle: Track Uncertainty Through Every Transformation

Every measurement has uncertainty. Every transformation of a measurement propagates and (usually) increases uncertainty. MetaRDU currently treats every number as exact. The fix is to replace `f64` with `UncertainValue { value: f64, uncertainty: f64, confidence: f64 }` in critical paths.

### UncertainValue Type

```rust
/// A scalar with associated uncertainty (1-sigma) and confidence level.
#[derive(Debug, Clone, Serialize)]
pub struct UncertainValue {
    pub value: f64,
    /// 1-sigma standard deviation (same units as value)
    pub sigma: f64,
    /// Confidence level (0-1). 0.95 = 95% confidence interval = value ± 1.96*sigma
    pub confidence: f64,
}

impl UncertainValue {
    pub fn certain(value: f64) -> Self {
        Self { value, sigma: 0.0, confidence: 1.0 }
    }

    pub fn from_sigma(value: f64, sigma: f64) -> Self {
        Self { value, sigma, confidence: 0.68 } // 1-sigma = 68%
    }

    /// 95% confidence interval
    pub fn ci_95(&self) -> (f64, f64) {
        let moe = 1.96 * self.sigma;
        (self.value - moe, self.value + moe)
    }

    /// Propagate uncertainty through addition: (a ± σa) + (b ± σb) = (a+b) ± sqrt(σa² + σb²)
    pub fn add(&self, other: &UncertainValue) -> UncertainValue {
        UncertainValue {
            value: self.value + other.value,
            sigma: (self.sigma.powi(2) + other.sigma.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// Propagate through subtraction
    pub fn sub(&self, other: &UncertainValue) -> UncertainValue {
        UncertainValue {
            value: self.value - other.value,
            sigma: (self.sigma.powi(2) + other.sigma.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// Propagate through multiplication: (a ± σa) * (b ± σb)
    /// σ_result = |result| * sqrt((σa/a)² + (σb/b)²)
    pub fn mul(&self, other: &UncertainValue) -> UncertainValue {
        let result = self.value * other.value;
        let rel_a = if self.value.abs() > 1e-12 { self.sigma / self.value } else { 0.0 };
        let rel_b = if other.value.abs() > 1e-12 { other.sigma / other.value } else { 0.0 };
        UncertainValue {
            value: result,
            sigma: result.abs() * (rel_a.powi(2) + rel_b.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }

    /// Propagate through division
    pub fn div(&self, other: &UncertainValue) -> UncertainValue {
        let result = self.value / other.value;
        let rel_a = if self.value.abs() > 1e-12 { self.sigma / self.value } else { 0.0 };
        let rel_b = if other.value.abs() > 1e-12 { other.sigma / other.value } else { 0.0 };
        UncertainValue {
            value: result,
            sigma: result.abs() * (rel_a.powi(2) + rel_b.powi(2)).sqrt(),
            confidence: self.confidence.min(other.confidence),
        }
    }
}
```

### Where Uncertainty Enters the Pipeline

| Source | Typical Uncertainty (1-sigma) |
|---|---|
| RTK GNSS position (horizontal) | ±10 mm |
| RTK GNSS elevation | ±20 mm |
| Static GNSS (post-processed) | ±5 mm horizontal, ±10 mm vertical |
| Total station distance (EDM) | ±2 mm + 2 ppm |
| Total station angle | ±2 arc-seconds |
| Drone photogrammetry point (relative) | ±20 mm in X/Y, ±40 mm in Z |
| Drone photogrammetry point (absolute, GCP-adjusted) | ±30 mm in X/Y/Z |
| MBES sounding (depth) | ±0.05 m (S-44 Special Order) |
| MBES sounding (depth) | ±0.5 m (S-44 Order 2) |
| Tide gauge reading | ±0.02 m |
| SVP cast (sound speed) | ±0.05 m/s |
| Density factor (ore) | ±2% (lab measurement) |
| DEM cell (interpolated) | ±0.1 m (depends on point density) |

### Propagation Through MetaRDU's Critical Paths

#### Path 1: Drone survey → Stockpile volume

```
Raw GNSS observations (±10mm H, ±20mm V)
  → ODM SfM adjustment (±30mm absolute, ±20mm relative)
    → CSF ground classification (introduces ~5cm bias if classification wrong)
      → DEM rasterization (±0.1m vertical, depends on grid size + point density)
        → Volume = Σ(cell_area × Δz)
          → σ_volume = sqrt(N) × cell_area × σ_z
          → Example: 1000 cells, 1m² each, σ_z = 0.1m
          → σ_volume = sqrt(1000) × 1 × 0.1 = 3.16 m³
          → 95% CI = ±6.2 m³
```

**Current behavior**: Volume returned as exact number. Surveyor has no idea if ±0.5 m³ or ±50 m³.

**Fix**: Thread `UncertainValue` through `compute_volumes()`. Report `VolumeResult { fill_volume: UncertainValue, cut_volume: UncertainValue, net_change: UncertainValue, ... }`. UI shows `12,345 ± 6 m³ (95%)` instead of `12,345 m³`.

#### Path 2: Total station → Setout bearing

```
EDM distance (±2mm + 2ppm) + angle (±2")
  → Bearing = atan2(ΔE, ΔN)
    → σ_bearing = sqrt((σ_ΔE / ΔN)² + (ΔE × σ_ΔN / ΔN²)²)  [radians]
  → Distance = sqrt(ΔE² + ΔN²)
    → σ_distance = sqrt((ΔE × σ_ΔE / distance)² + (ΔN × σ_ΔN / distance)²)
```

**Current behavior**: Bearing and distance returned as exact f64.

**Fix**: `SetoutResult { bearing: UncertainValue, distance: UncertainValue, ... }`. UI shows `Bearing: 45.023° ± 0.001°` and `Distance: 141.421 ± 0.003 m`.

#### Path 3: MBES sounding → S-44 compliance

```
Sound speed at transducer (±0.05 m/s)
  → Ray tracing through SVP (±0.1% of depth)
    → Heave (±0.05 m), roll/pitch lever-arm (±0.05 m)
      → Tide correction (±0.02 m)
        → Final corrected depth uncertainty = sqrt(σ_ray² + σ_heave² + σ_lever² + σ_tide²)
```

**Current behavior**: `compute_tpu()` exists and computes TPU (Total Propagated Uncertainty) per S-44 spec. This is actually done correctly! But the TPU is reported per-sounding, not aggregated into a volume uncertainty.

**Fix**: When volumes are computed from MBES soundings, propagate per-sounding TPU into volume uncertainty.

### Implementation Plan

1. Add `qc/propagation.rs` with `UncertainValue` struct + arithmetic (add/sub/mul/div/pow/sqrt) — ~400 lines
2. Refactor `volume.rs::compute_volumes` to accept `UncertainValue` inputs and return `UncertainValue` outputs — ~200 lines changed
3. Refactor `survey_tools.rs::compute_setout` similarly — ~100 lines
4. Refactor `change_detection.rs::detect_stockpile_change` — ~150 lines
5. Update frontend dialogs to display uncertainty — ~200 lines across 4 dialogs
6. Update `report_engine.rs` to include uncertainty in PDF reports — ~100 lines

---

## Part 4: Blunder Detection

Beyond gradual uncertainty, surveys are vulnerable to **blunders** — gross errors (a misread angle, a wrong unit, a swapped coordinate). MetaRDU has no blunder detection. The standard methods are:

### 1. Redundant Observations + LSA Residuals
- Observe each angle/distance at least twice
- Least-squares adjustment computes residuals
- A blunder produces a residual > 3-sigma → flag and remove

**Status in MetaRDU**: Not implemented (LSA engine is in the gap analysis as Tier 1 #5)

### 2. Closure Checks
- Traverse: sum of angles = (n-2) × 180°; sum of latitudes = 0; sum of departures = 0
- Level loop: closure = sum of rises - sum of falls; should be < tolerance
- Triangles: sum of angles = 180° (spherical excess for geodetic)

**Status in MetaRDU**: Not implemented. **Add `qc/closure.rs`** — ~300 lines.

### 3. Range Checks (Sanity)
- Latitude ∈ [-90, 90], longitude ∈ [-180, 180]
- Elevation within ±1000 m of regional MSL (configurable)
- Distance < 100 km (typical total station range is 3 km)
- Bearing ∈ [0, 360)
- Volume magnitude < 10× expected (catch unit errors — m³ vs ft³)

**Status in MetaRDU**: Not implemented as a systematic layer. **Add `qc/range_checks.rs`** — ~200 lines.

### 4. Statistical Tests
- Chi-square test on a-posteriori variance factor (LSA)
- Baarda's method (data snooping) for blunder detection
- F-test for redundancy

**Status in MetaRDU**: Not implemented. Depends on LSA engine.

### 5. Cross-Sensor Consistency
- If both GGA and RMC are present, compare lat/lon — they should agree to within receiver noise
- If both GNSS and total station observations of the same point exist, compare
- If drone DEM and ground survey overlap, compare at sample points

**Status in MetaRDU**: NMEA parser supports both GGA and RMC but doesn't compare them at runtime. **Add cross-sensor check in `realtime/rover.rs`** — ~50 lines.

---

## Part 5: Concrete Sprint 12 Changes

Ordered by ROI:

| # | Change | Effort | Lines | Impact |
|---|---|---|---|---|
| 1 | `qc/propagation.rs` — `UncertainValue` struct + arithmetic | Medium | ~400 | Foundation for all uncertainty reporting |
| 2 | `qc/verify.rs` — `verify_calculation()` wrapper + cross-check framework | Small | ~150 | Catches silent calculation bugs |
| 3 | Cross-check for volume (grid vs TIN) | Medium | ~600 | Catches volume bugs (highest financial impact) |
| 4 | Cross-check for setout (atan2 vs quadrant) | Small | ~50 | Catches setout bugs |
| 5 | Range checks (lat/lon/elev/distance/bearing) | Small | ~200 | Catches gross input errors |
| 6 | Closure checks (traverse angles, lat/dep) | Medium | ~300 | Required for total station work |
| 7 | NMEA GGA vs RMC consistency check | Small | ~50 | Catches receiver glitches |
| 8 | LAS point count vs header check | Trivial | ~20 | Catches truncated files |
| 9 | GeoTIFF dimension vs header check | Trivial | ~20 | Catches corrupt files |
| 10 | Refactor `VolumeResult` to use `UncertainValue` | Medium | ~200 | User-facing uncertainty display |
| 11 | Refactor `SetoutResult` to use `UncertainValue` | Small | ~100 | User-facing uncertainty display |
| 12 | Frontend: display uncertainty in volume/setout/change dialogs | Medium | ~200 | User sees ± on every result |
| 13 | PDF report: include uncertainty + cross-check status | Small | ~100 | Audit trail |

**Total**: ~2,400 lines of new Rust + ~200 lines of TypeScript. ~3-4 days of focused work.

---

## Part 6: Long-Term Vision — Provenance Graph

The ultimate QA/QC system is a **provenance graph**: every output value remembers every input that contributed to it, with the transformation chain and uncertainty at each step. When a user clicks "12,345 m³" in a report, they see:

```
Stockpile Volume: 12,345 ± 6 m³ (95%)
├── DEM (current): grid_2026-07-07.tif
│   ├── Source: drone_survey_2026-07-07.las (10.2M points)
│   │   ├── GNSS base: NTRIP station BASE-001, ±10mm
│   │   ├── Drone: DJI M300 RTK, ±30mm absolute
│   │   └── SfM: ODM v3.5.2, ±20mm relative
│   ├── CSF classification: threshold=0.5, cloth_resolution=2.0
│   │   └── Bias estimate: ±0.05m (from validation against GCPs)
│   └── Rasterization: 1m grid, IDW interpolation, ±0.1m vertical
├── DEM (reference): grid_2026-06-01.tif
│   └── (same lineage)
├── Cell area: 1.0 m² (exact)
└── Cross-check: TIN-based volume = 12,347 m³ ✓ (within 0.02%)
```

This is the gold standard for survey-grade software. It's a large effort (~5,000 lines, 4-6 weeks) but it transforms the product from "calculates volumes" to "defensibly certifies volumes". For the dredge pay-volume and EOM reconciliation revenue features, this is the difference between a tool and a trusted tool.

**Recommendation**: Implement in Sprint 14-15 after the LSA engine and COGO modules land.

---

## Bottom Line

MetaRDU's calculation engine is **correct in the happy path** (unit tests verify the math) but **silent in the unhappy path** (no runtime cross-checks, no uncertainty reporting, no blunder detection). A surveyor using MetaRDU today gets a number; they don't get a number with a confidence interval or a verification flag.

The Sprint 12 changes above (2,400 lines, ~3-4 days) close the most critical gaps:
- Cross-checks catch silent calculation bugs
- Uncertainty propagation lets the surveyor defend the number
- Range + closure checks catch gross input errors
- Provenance is the long-term differentiator

After Sprint 12, every volume report should say **"12,345 ± 6 m³ (95% confidence, cross-checked against TIN method)"** instead of just **"12,345 m³"**. That's the difference between a surveyor who can defend their work in court and one who can't.
