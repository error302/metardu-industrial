# Geomatics Engineer Gap Analysis — Is MetaRDU Enough?

**Audience**: A licensed geomatics engineer doing **cadastral**, **topographic**, and **engineering surveys**.
**Question**: Can MetaRDU Industrial serve as their primary tool, or do they need a companion?
**Date**: 2026-07-07

---

## Verdict

**No — MetaRDU alone is NOT enough for a geomatics engineer whose practice includes cadastral work.** It is strong for the mining + marine surveying scope it was built for, and it covers ~70% of topographic and ~60% of engineering survey needs. But cadastral surveying has hard legal requirements (boundary determination, coordinate-of-record, plan registration) that MetaRDU does not address at all.

**Recommended setup**:
- **Primary**: MetaRDU Industrial for topo, engineering, mining, marine
- **Companion**: A dedicated cadastral package (Trimble Business Center, MicroSurvey FieldGenius, or open-source QCAD/QGIS with the cadastre plugin) for boundary work
- **Total station / GNSS post-processing**: MetaRDU's NTRIP + RTK rover (Sprint 11) covers real-time, but **post-processing of static GNSS baselines** is missing — pair with RTKLIB or Trimble Business Center for sub-mm control networks

The gap analysis below lists every missing capability, ranked by how often a geomatics engineer would hit the gap.

---

## What MetaRDU Already Has (Strong Coverage)

These workflows are well-supported today:

| Workflow | MetaRDU Module | Strength |
|---|---|---|
| Drone photogrammetry (topo) | ODM pipeline + LAS ingest + CSF ground classification | Good — produces DEMs and ortho-ready point clouds |
| Volume calculation (cut/fill) | `volume.rs` + `change_detection.rs` | Excellent — bench breakdown, hotspot detection, median-of-cell rasterization |
| Stockpile audit | `StockpileAuditWizard` + `StockpileChangeDialog` | Excellent — tonnage with density factor, monthly reconciliation |
| Setting out / markout | `SetoutToolDialog` (Sprint 10) | Good — bearing, distance, slope from reference peg |
| Mine grid transform | `MineGridDialog` (Sprint 10) | Good — bidirectional with rotation + scale |
| Tunnel profile / overbreak | `TunnelProfileDialog` (Sprint 10) | Good — area, overbreak/underbreak vs design |
| RTK rover visualization | `RoverStreamDialog` (Sprint 11) | Good — NMEA over TCP, 5 Hz polling, trail |
| NTRIP corrections | `NtripDialog` (Sprint 9) | Good — RTCM v3 parsing, base64 auth |
| Tide correction (marine topo of foreshore) | `TideGaugeDialog` (Sprint 11) | Good — NOAA CO-OPS, real-time |
| Profile / cross-section | `CrossSectionProfilerWizard` + profile tool | Good — DEM sampling, perpendicular sections |
| LAS / LAZ / GeoTIFF / DXF | `formats/` modules | Good — pure-Rust parsers, no external deps |
| MBES bathymetry | `MbesSurveyDialog` + `KongsbergAll` parser | Good — for marine topo of submerged areas |
| Report generation (PDF) | `report_engine.rs` | Good — branded, chain-of-custody |
| Project file format | `.metardu` JSON | Good — save/load, auto-save, versioning |

---

## What's Missing — Ranked by Frequency of Need

### Tier 1: Gaps a geomatics engineer hits weekly

#### 1. Cadastral Boundary Determination
**Gap**: No module for boundary restoration, subdivision design, or coordinate-of-record computation. MetaRDU has no concept of a "parcel", a "deed", or a "coordinate adjustment to a legal monument".

**What's needed**:
- Boundary Comer Search tool — fit observed angles/distances to a record description, compute residuals
- Subdivision designer — split a parcel along a defined line, compute new lot areas to ±0.01 m²
- Coordinate-of-Record (COR) computation — least-squares adjustment of repeated observations to a monument, with chi-square test for blunder detection
- Plan of Survey generator — title block, north arrow, scale bar, bearing/distance table, area callout, monument symbols, signature block. Output as DXF or PDF.

**Why it matters**: In most jurisdictions, only a licensed cadastral surveyor can sign a plan of survey, and the plan must meet specific format requirements (e.g., Australia's CSDM format, US PLSS, UK HM Land Registry). MetaRDU can't produce these.

**Recommendation**: **Do NOT try to build this in MetaRDU.** Cadastral law is jurisdiction-specific and legally risky. Instead, add a "Cadastral Export" plugin that ships observed coordinates + monument descriptions to a dedicated cadastral package (Trimble, MicroSurvey, open-source option).

#### 2. GNSS Post-Processing (Static Baselines)
**Gap**: MetaRDU's NTRIP + RTK rover (Sprint 11) handles real-time kinematic, but **static GNSS post-processing** is missing. For control networks (e.g., establishing 10 control points for a topo survey with sub-mm accuracy), the surveyor records 4+ hours of static observations at each point and post-processes the baselines.

**What's needed**:
- RINEX file import (already partially exists — Triage can detect RINEX, but no parser)
- Baseline processor — double-difference ambiguity resolution, ionosphere-free combination, troposphere modeling
- Network adjustment — least-squares adjustment of all baselines together, with redundancy and chi-square test
- Coordinate output in ITRF / WGS84 / local datum, with epoch transformation for plate motion

**Why it matters**: Real-time RTK gives ±1-2 cm. Static post-processing gives ±2-5 mm. For control networks, legal monuments, and deformation monitoring, you need the latter. RTKLIB (open-source) does this; MetaRDU should integrate rather than reinvent.

**Recommendation**: Add a "GNSS Post-Processing" plugin that shells out to RTKLIB (`rtklib-shutdown` / `pos2kml`) and ingests the results. Don't reinvent baseline processing — it's 10,000+ lines of careful math.

#### 3. Total Station Data Import (Raw Files)
**Gap**: MetaRDU's `SetoutToolDialog` computes setout from design coordinates, but it doesn't import **raw total station observations** (angles, distances, weather corrections) from the instrument. A geomatics engineer needs to:
- Import raw `.job` / `.raw` / `.tps` files from Trimble, Leica, Topcon, Sokkia
- Apply weather corrections (temperature, pressure, humidity → EDM scale correction)
- Reduce slope distances to horizontal
- Compute adjusted coordinates via least-squares traverse adjustment

**What's needed**:
- Raw file parsers for at least Trimble `.job`, Leica `.gsi`, Topcon `.raw` (3 most common)
- EDM correction module — velocity correction from temperature/pressure/humidity per NOAA formulas
- Traverse adjustment — Bowditch / Transit / Crandall's method, with closure check
- Side-shot / radial shot computation

**Recommendation**: Build this in MetaRDU. It's a natural extension of `survey_tools.rs` and fits the existing "mining surveyor" scope perfectly. ~2,000 lines of Rust.

#### 4. Coordinate Geometry (COGO)
**Gap**: No COGO engine. A geomatics engineer constantly needs:
- Intersection of two lines, line and circle, two circles
- Offset point from a line/curve
- Curve fitting (radius from 3 points, spiral curves)
- Area computation by coordinates (already have shoelace, but no DMD method)
- Bearing/distance between two known points (inverse)
- Pre-determined area subdivision

**What's needed**: A COGO module with at least 15 standard algorithms. The math is well-published (e.g., Davis, Foote, Anderson *Surveying Theory and Practice*).

**Recommendation**: Build this in MetaRDU as `cogo.rs`. Pure math, ~800 lines. High value, low risk.

#### 5. Least-Squares Adjustment
**Gap**: MetaRDU has no LSA engine. Every traverse, network, or control survey needs LSA to:
- Distribute misclosures proportionally to observation precision
- Compute adjusted coordinates with rigorous error propagation
- Detect blunders via chi-square test on a-posteriori variance factor
- Report standard deviations of adjusted coordinates

**What's needed**:
- Observation equation formulation (angles, distances, GNSS baselines)
- Normal equation solver (Cholesky decomposition — already available via `nalgebra`)
- A-posteriori variance factor + chi-square test
- Covariance matrix output for downstream error propagation
- Blunder detection via Baarda's method (data snooping)

**Recommendation**: Build this in MetaRDU as `adjustment/lsa.rs`. The math is well-understood, ~1,500 lines. Pair with the COGO module.

### Tier 2: Gaps a geomatics engineer hits monthly

#### 6. Coordinate Transformation / Datum Conversion
**Gap**: MetaRDU relies on `proj4js` (in the frontend) and PROJ availability check (in Rust). But:
- No time-dependent transformations (ITF14 → ITRF20 with epoch)
- No grid shifts (Canada NTv2, Australia AGD66→GDA94, US NADCON)
- No deformation model (post-earthquake coordinate updates — NZ Geonet, Japan GEONET)

**Recommendation**: Wrap `proj` crate with grid file loading. Already on the roadmap (Sprint 13 Standards & Compliance).

#### 7. Field Coding / Feature Code Library
**Gap**: No feature code library. Topographic surveys use a coding system (e.g., Trimble feature codes, Leica feature code lists) to attribute points in the field — "TREE" for vegetation, "EDGE" for road edge, "MH" for manhole. MetaRDU has no concept of this.

**What's needed**:
- Feature code library (JSON, editable)
- Auto-symbolization on the map based on code
- Auto-line-connection by code + sequence number
- Layer grouping by code prefix

**Recommendation**: Build as a frontend module. ~600 lines TypeScript.

#### 8. Contour Generation
**Gap**: MetaRDU produces DEMs (GeoTIFF rasters) but doesn't generate **contour lines** from them. Topo surveys always need contours (1m, 2m, 5m intervals).

**What's needed**: Marching squares algorithm on the DEM, with smoothing and labeling. Output as GeoJSON for overlay.

**Recommendation**: Build as `dem_render.rs::generate_contours()`. ~400 lines.

#### 9. Volume by End-Area (Cross-Section Method)
**Gap**: MetaRDU's volume calculator uses grid-based (cell-by-cell) method. For road/railway/canal engineering, the **end-area method** (average end area × distance between sections) is the standard. The `CrossSectionProfilerWizard` extracts sections but doesn't compute end-area volumes.

**What's needed**: End-area volume module that takes a series of cross-sections + chainage + design template and computes cut/fill per section + total.

**Recommendation**: Build as `mining/volume.rs::compute_end_area_volumes()`. ~300 lines.

#### 10. Profile Sheets (Longitudinal Profile)
**Gap**: Engineers need **profile sheets** — a standardized plot showing existing ground profile vs. design grade, with curve data, stationing, elevations, and a grid. MetaRDU's profile tool draws the line but doesn't produce the sheet.

**What's needed**: PDF generator for profile sheets following a standard template (e.g., AASHTO, state DOT).

**Recommendation**: Build as a `report_engine` template. ~500 lines.

### Tier 3: Gaps a geomatics engineer hits a few times a year

#### 11. Deformation Monitoring (High-Precision)
**Gap**: MetaRDU has `monitoring_4d.rs` for pit progression, but it's tuned for meter-scale changes. **Deformation monitoring** (dam, bridge, tunnel, slope stability) requires sub-mm precision with statistical significance testing.

**What's needed**:
- Repeat-observation adjustment (multi-epoch LSA)
- Statistical test for displacement (chi-square on coordinate differences)
- Temperature correction for invar tapes / steel tapes
- Tilt-meter and strain-gauge integration

**Recommendation**: Partner with a geotechnical firm — this is a niche market with high regulatory burden.

#### 12. Hydrographic Survey Standards (Other Than S-44)
**Gap**: MetaRDU has S-44 compliance but is missing:
- USACE hydrographic survey standards (EM 1110-2-1003) — common for US Army Corps work
- IHO S-102 (next-gen bathymetric surface) — on the roadmap for Sprint 13
- LDS (Laser Detection and Ranging) processing — bathymetric lidar

#### 13. Point-Cloud Classification (Beyond CSF)
**Gap**: MetaRDU has CSF for ground classification. But for engineering surveys, you also need:
- Building / structure classification
- Vegetation classification (low / medium / high)
- Power line / wire classification
- Pole / tree trunk classification

**Recommendation**: This was previously in the AI/ML theme — but the user removed it. Build deterministic classifiers based on geometric features (planarity, verticality, scatter) instead. ~1,500 lines.

#### 14. Quality Level Reporting (ASPRS)
**Gap**: LiDAR data needs ASPRS QL (Quality Level) reporting — QL0 (best) through QL3. MetaRDU's LAS reader doesn't compute or report QL.

**What's needed**: Compute relative accuracy, absolute accuracy, point density, and classification confidence per ASPRS specifications.

#### 15. Geodetic Database Integration
**Gap**: No integration with geodetic databases (NGS OPUS, AusPOS, Trimble VRS). A geomatics engineer needs to submit RINEX files to these services for precise point positioning.

**What's needed**: Upload RINEX → submit to NGS OPUS / AusPOS → parse returned coordinates + uncertainties. Could be a plugin.

---

## Summary Scorecard

| Domain | MetaRDU Coverage | Companion Needed? |
|---|---|---|
| Cadastral surveying | 0% | **Yes — non-negotiable** |
| GNSS static post-processing | 10% (RINEX detection only) | Yes — RTKLIB or TBC |
| Total station field work | 25% (setout only) | Yes — until raw import is added |
| Coordinate geometry (COGO) | 5% | Yes — until COGO module is added |
| Least-squares adjustment | 0% | Yes — until LSA module is added |
| Topographic surveying (drone) | 80% | No |
| Topographic surveying (total station) | 30% | Yes — until field coding + raw import |
| Engineering surveying (volumes) | 90% | No |
| Engineering surveying (cross-sections) | 60% | Yes — end-area volumes missing |
| Contour generation | 0% | Yes — use QGIS or add module |
| Profile sheets | 20% | Yes — until PDF template added |
| Marine surveying (bathymetry) | 85% | No |
| Mining surveying | 95% | No |
| Deformation monitoring | 30% | Yes — for sub-mm precision work |

---

## Recommendation Path

**Immediate (Sprint 12-13)**:
1. Build COGO module — highest ROI, pure math
2. Build Least-Squares Adjustment engine — pairs with COGO
3. Add raw total station file import (Trimble/Leica/Topcon)
4. Add contour generation
5. Add end-area volume method

**Medium-term (Sprint 14-15)**:
6. GNSS post-processing plugin (wrap RTKLIB)
7. Feature code library
8. Profile sheet PDF generator
9. Coordinate transformation grids (NTv2, NADCON)
10. ASPRS QL reporting

**Long-term / Out of scope**:
11. Cadastral boundary determination (legal risk — leave to dedicated packages)
12. Deformation monitoring at sub-mm (partner with geotechnical firm)
13. Plan of Survey generator (jurisdiction-specific format compliance)

---

## Bottom Line

For a geomatics engineer whose practice is **70% topo + engineering + mining/marine** and **30% cadastral**, MetaRDU can be the primary tool after Sprint 12-13 adds COGO, LSA, total station import, and contours. The cadastral 30% should stay in a dedicated package.

For a geomatics engineer whose practice is **70% cadastral**, MetaRDU is the wrong primary tool — but it can serve as the secondary tool for the topo/engineering portions of their work.
