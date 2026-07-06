# Mining + Marine Surveyor Gap Analysis

**Audience**: A mining + marine surveyor using MetaRDU Industrial as their primary tool.
**Question**: For the workflows MetaRDU specializes in — open-pit + underground mining, dredging, hydrographic surveying — what's still missing?
**Date**: 2026-07-07 (revised — cadastral scope removed, belongs to the separate MetaRDU web app)

---

## Verdict

For a **mining + marine surveyor**, MetaRDU Industrial is **~85-90% coverage** today and reaches **~95%** after the Sprint 12-13 additions listed below. The remaining 5% is niche (deformation monitoring at sub-mm precision, S-100 next-gen standards) that can be deferred or partnered.

The app's scope is deliberately bounded:
- **Mining**: open-pit volume reconciliation, stockpile audit, blast fragmentation, highwall monitoring, underground tunnel profile, mine-grid transforms, setting out
- **Marine**: MBES bathymetry, CUBE surface generation, S-44 compliance, S-57 export, dredge pay-volume, cross-section profiler, backscatter mosaic, real-time QC, tide correction
- **Cross-cutting**: drone photogrammetry (ODM), LAS/LAZ/GeoTIFF/.all/.s7k ingest, RTK rover + NTRIP, project file format, branded PDF reports, license management

Cadastral surveying, BIM coordination, and mobile PWA field capture are explicitly **out of scope** — they belong to the separate MetaRDU web app or dedicated third-party tools.

---

## What's Already Strong (Sprint 0-11 Coverage)

| Workflow | Module | Coverage |
|---|---|---|
| Drone photogrammetry → LAS | ODM pipeline + LAS/LAZ ingest | ✅ Excellent |
| Ground classification (CSF) | `csf.rs` | ✅ Excellent |
| Volume calculation (grid method) | `volume.rs` | ✅ Good — needs cross-check (Sprint 12) |
| Stockpile audit + change detection | `StockpileAuditWizard` + `change_detection.rs` | ✅ Excellent (Sprint 10) |
| EOM reconciliation | `eom.rs` + signed PDF | ✅ Excellent |
| Blast fragmentation report | `ml/mod.rs::analyze_fragmentation` | ✅ Good |
| Highwall deformation monitoring | `highwall.rs` (USACE thresholds) | ✅ Good |
| 4D pit progression | `monitoring_4d.rs` | ✅ Good |
| Tunnel profile + overbreak | `TunnelProfileDialog` (Sprint 10) | ✅ Good |
| Setting out + markout | `SetoutToolDialog` (Sprint 10) | ✅ Good |
| Mine grid transform | `MineGridDialog` (Sprint 10) | ✅ Good |
| Safety inspection report | `SafetyReportDialog` (Sprint 10) | ✅ Good |
| Machine control file compiler | `MachineControlTool` (Sprint 9) | ✅ Good |
| MBES bathymetry (.all parser) | `kongsberg_all.rs` | ✅ Excellent (water column added Sprint 10) |
| CUBE surface generation | `cube.rs` + disambiguation UI | ✅ Good |
| S-44 compliance + certificate | `s44.rs` + `S44CertificateDialog` | ✅ Excellent |
| S-57 export | `s57.rs` | ✅ Good |
| Dredge pay-volume audit | `dredge.rs` (4-bucket categorization) | ✅ Excellent |
| Cross-section profiler | `cross_section.rs` | ✅ Good |
| Backscatter mosaic | `BackscatterMosaicDialog` (Sprint 10) | ✅ Good |
| Real-time QC dashboard | `QcDashboardDialog` (Sprint 10) | ✅ Good |
| SVP editor + ray tracing | `svp.rs` + `SvpEditorDialog` | ✅ Good |
| Vessel lever-arm config | `VesselConfigDialog` | ✅ Good |
| Tidal datum conversion | `TidalDatumDialog` (Sprint 10) | ✅ Good |
| Tide gauge (NOAA CO-OPS) | `TideGaugeDialog` (Sprint 11) | ✅ Good |
| RTK rover visualization | `RoverStreamDialog` (Sprint 11) | ✅ Good |
| NTRIP client | `NtripDialog` (Sprint 9) | ✅ Good |
| Project templates | `project-templates.ts` (Sprint 11) | ✅ Good |
| Undo/redo stack | `undo-store.ts` (Sprint 11) | ✅ Good |
| License + telemetry | Sprint 7-9 | ✅ Good |
| Plugin marketplace | Sprint 8 | ✅ Good |

---

## What's Missing — Ranked by Mining/Marine Frequency

### Tier 1: Gaps a mining/marine surveyor hits weekly

#### 1. Coordinate Geometry (COGO) Engine
**Gap**: No COGO module. A mining surveyor constantly needs:
- **Inverse** — bearing + distance between two known coordinates (the `SetoutToolDialog` does this, but only for setout, not as a general tool)
- **Intersection** — point at the intersection of two bearings, two circles, or bearing+distance from two known points
- **Offset** — point offset perpendicular from a line by a fixed distance
- **Curve fitting** — radius from 3 points (common for bench crest curves)
- **Area subdivision** — split a stockpile polygon along a defined line, compute each part's area
- **DMD area** — Double Meridian Distance method (alternative to shoelace, used as a cross-check)

**Why it matters for mining**: Setting out a blast pattern requires computing hole positions at regular offsets along a design line. Computing the area of an irregular stockpile toe requires COGO. Resolving a disputed boundary inside the pit (ore/waste contact) needs intersection.

**Why it matters for marine**: Dredge design templates are COGO shapes (trapezoidal channels with side slopes). Computing the design template at each chainage is COGO.

**Recommendation**: **Build in MetaRDU** as `cogo.rs`. Pure math, well-published (Davis, Foote, Anderson), ~800 lines. Pairs with the existing `survey_tools.rs`.

#### 2. Contour Generation from DEM
**Gap**: MetaRDU produces DEMs (GeoTIFF rasters) but doesn't generate **contour lines**. Mining surveyors need:
- Bench crest contours (every 5-10 m elevation interval)
- Stockpile shape contours (for volumetric progress visualization)
- Pit shell contours (for reconciliation against design)
- Dredge channel contours (for shoal detection)

**What's needed**: Marching squares algorithm on the DEM grid, with smoothing and elevation labeling. Output as GeoJSON for OpenLayers overlay, plus DXF export for Civil3D handoff.

**Recommendation**: **Build in MetaRDU** as `dem_render.rs::generate_contours()`. ~400 lines. Pure Rust, no external deps.

#### 3. End-Area Volume Method
**Gap**: MetaRDU's `volume.rs` uses the grid method (cell-by-cell Δz × cell_area). For linear infrastructure — **haul roads, ramp design, dredge channels, tailings dam construction** — the **end-area method** is the standard:
```
Volume = (A1 + A2) / 2 × L
```
where A1 and A2 are cross-sectional areas at two chainages and L is the distance between them.

**Why it matters for mining**: Haul road earthwork quantities are always computed end-area. The `CrossSectionProfilerWizard` extracts sections but doesn't compute end-area volumes between them.

**Why it matters for marine**: Dredge pay-volume along a channel is computed end-area — each cross-section's cut area × distance to next section.

**Recommendation**: **Build in MetaRDU** as `volume.rs::compute_end_area_volumes()`. ~300 lines. Uses the existing cross-section extraction.

#### 4. Total Station Raw File Import
**Gap**: MetaRDU's `SetoutToolDialog` computes setout from design coordinates, but it doesn't import **raw total station observations** from the instrument. An underground mining surveyor shoots traverses with a total station and needs to:
- Import raw `.job` (Trimble), `.gsi` (Leica), `.raw` (Topcon) files
- Apply weather corrections (temperature, pressure, humidity → EDM scale correction)
- Reduce slope distances to horizontal
- Compute adjusted coordinates via traverse adjustment (Bowditch / Crandall's)
- Check angular closure (sum of angles = (n-2) × 180°) and linear closure

**Why it matters for mining**: Underground traverses are the backbone of tunnel surveying. Without raw import, the surveyor has to reduce observations in a separate tool (Excel, Civil3D) and import coordinates — losing the observation-level audit trail.

**Recommendation**: **Build in MetaRDU** as `formats/total_station.rs` + `traverse.rs`. ~2,000 lines. Trimble `.job` first (most common in mining), then Leica `.gsi`.

#### 5. Least-Squares Adjustment (LSA)
**Gap**: No LSA engine. Every traverse, control network, or deformation monitoring network needs LSA to:
- Distribute misclosures proportionally to observation precision
- Compute adjusted coordinates with rigorous error propagation
- Detect blunders via chi-square test on a-posteriori variance factor
- Report standard deviations of adjusted coordinates (feeds into `UncertainValue`)

**Why it matters for mining**: Control networks for pit pegs need LSA. Deformation monitoring of highwalls needs LSA + statistical significance testing. The existing `monitoring_4d.rs` computes displacement but doesn't adjust the observations.

**Why it matters for marine**: Vessel offsets (IMU → transducer → GNSS lever arms) are measured once and then treated as exact. LSA would let the surveyor over-determine the offsets and report their uncertainty.

**Recommendation**: **Build in MetaRDU** as `adjustment/lsa.rs`. ~1,500 lines. Uses `nalgebra` (already a dependency) for the linear algebra. Pair with the COGO module.

### Tier 2: Gaps a mining/marine surveyor hits monthly

#### 6. GNSS Static Post-Processing
**Gap**: MetaRDU's NTRIP + RTK rover (Sprint 11) handles real-time kinematic, but **static GNSS post-processing** is missing. For control networks (e.g., establishing 10 control points around a pit with sub-mm accuracy), the surveyor records 4+ hours of static observations and post-processes the baselines.

**What's needed**:
- RINEX file parser (Triage detects RINEX, but no parser)
- Baseline processor — double-difference ambiguity resolution
- Network adjustment — least-squares adjustment of all baselines

**Recommendation**: **Integrate, don't build**. Wrap RTKLIB (open-source, well-tested) via a plugin. Building a baseline processor from scratch is 10,000+ lines of careful math and validation.

#### 7. Coordinate Transformation Grids
**Gap**: MetaRDU relies on `proj4js` (frontend) and PROJ (Rust). But:
- No grid shifts (Australia AGD66→GDA94 via NTv2, US NAD27→NAD83 via NADCON)
- No time-dependent transformations (ITRF2014 → ITRF2020 with epoch, for plate motion)
- No deformation model (post-earthquake coordinate updates)

**Why it matters for mining**: Australian mines often have legacy data in AGD66 / AMG coordinates. Modern surveys are in GDA2020. Without NTv2 grid shift, the surveyor can't reconcile historical pit plans with current surveys.

**Recommendation**: Wrap the `proj` crate with grid file loading. Already on the roadmap (Sprint 13 Standards & Compliance).

#### 8. Feature Code Library
**Gap**: No feature code library. Surveys use a coding system to attribute points in the field — "TOE" for bench toe, "CREST" for bench crest, "MH" for manhole, "EDGE" for road edge. MetaRDU has no concept of this.

**What's needed**:
- Feature code library (JSON, editable)
- Auto-symbolization on the map based on code
- Auto-line-connection by code + sequence number
- Layer grouping by code prefix

**Recommendation**: Build as a frontend module. ~600 lines TypeScript.

#### 9. Profile Sheets (Longitudinal Profile PDF)
**Gap**: Engineers need **profile sheets** — a standardized plot showing existing ground profile vs. design grade, with curve data, stationing, elevations, and a grid. MetaRDU's profile tool draws the line but doesn't produce the sheet.

**Why it matters for mining**: Haul road profiles. Ramp profiles. Decline profiles (underground).

**Why it matters for marine**: Channel centerline profiles (existing vs design grade).

**Recommendation**: Build as a `report_engine` template. ~500 lines.

#### 10. Point-Cloud Classification (Beyond CSF)
**Gap**: MetaRDU has CSF for ground classification. But for mining, you also need:
- **Highwall extraction** — classify the near-vertical face points separately from the bench floor
- **Bench toe/crest detection** — automatic identification of the breakline
- **Vegetation classification** — for greenfield pit expansions
- **Power line / wire classification** — for safety clearance around conveyors

**Recommendation**: Build deterministic classifiers based on geometric features (planarity, verticality, scatter, local density). ~1,500 lines. NOT ML — the user removed the AI/ML theme; these are geometric computations.

### Tier 3: Gaps a mining/marine surveyor hits a few times a year

#### 11. Deformation Monitoring (Sub-mm Precision)
**Gap**: `monitoring_4d.rs` is tuned for meter-scale pit progression. **Deformation monitoring** (dam, tailings impoundment, highwall stability) requires sub-mm precision with statistical significance testing.

**What's needed**:
- Repeat-observation adjustment (multi-epoch LSA)
- Statistical test for displacement (chi-square on coordinate differences)
- Tilt-meter and strain-gauge integration
- Temperature correction for invar tapes

**Recommendation**: Partner with a geotechnical firm — this is a niche market with high regulatory burden. The existing `highwall.rs` with USACE thresholds covers the most common mining case.

#### 12. S-100 / S-102 Export
**Gap**: S-57 export works (Sprint 5). S-100 (the next-gen IHO framework) and S-102 (Bathymetric Surface Product) are not yet supported. The IHO ecosystem isn't mature enough for production use until ~2027.

**Recommendation**: Deferred per the existing roadmap (Sprint 13 Standards & Compliance theme).

#### 13. Bathymetric Lidar (Laser Detection and Ranging)
**Gap**: MetaRDU processes MBES (multibeam sonar) but not bathymetric lidar (e.g., RIEGL VQ-880-G, Leica Chiroptera). Bathymetric lidar is increasingly used for shallow-water surveys (<10 m depth) where MBES can't operate safely.

**Recommendation**: Add a LAS reader extension for bathymetric lidar waveforms. ~1,000 lines. Niche — defer until a customer requests it.

#### 14. ASPRS Quality Level Reporting
**Gap**: LiDAR data needs ASPRS QL (Quality Level) reporting — QL0 (best) through QL3. MetaRDU's LAS reader doesn't compute or report QL.

**Recommendation**: Compute relative accuracy, absolute accuracy, point density, and classification confidence per ASPRS specifications. ~300 lines.

#### 15. Hydrographic Survey Standards (Beyond S-44)
**Gap**: MetaRDU has S-44 compliance but is missing:
- USACE EM 1110-2-1003 (US Army Corps hydrographic survey standards)
- IHO S-102 (next-gen bathymetric surface)
- LDS (bathymetric lidar) processing

**Recommendation**: Add USACE EM 1110-1-1004 QA checks. ~500 lines. Already on the roadmap.

---

## Summary Scorecard — Mining + Marine Only

| Workflow | MetaRDU Coverage | Action |
|---|---|---|
| **Mining — open-pit volume reconciliation** | 95% | Add TIN cross-check (Sprint 12) |
| **Mining — stockpile audit** | 95% | Add uncertainty display (Sprint 12) |
| **Mining — blast fragmentation** | 90% | Good as-is |
| **Mining — highwall monitoring** | 90% | Add LSA for sub-mm (Sprint 13) |
| **Mining — underground traverse** | 30% | **Add total station import + LSA (Sprint 13)** |
| **Mining — tunnel profile** | 90% | Good as-is |
| **Mining — setting out** | 85% | **Add COGO (Sprint 12)** |
| **Mining — machine control** | 95% | Good as-is |
| **Marine — MBES bathymetry** | 95% | Good as-is |
| **Marine — CUBE surface** | 90% | Add GPU acceleration (Sprint 14) |
| **Marine — S-44 compliance** | 95% | Good as-is |
| **Marine — dredge pay-volume** | 95% | Add end-area method (Sprint 12) |
| **Marine — cross-section profiler** | 70% | **Add end-area volumes (Sprint 12)** |
| **Marine — backscatter mosaic** | 90% | Good as-is |
| **Marine — real-time QC** | 90% | Good as-is |
| **Marine — tide correction** | 90% | Good as-is |
| **Cross-cutting — drone photogrammetry** | 85% | Good as-is |
| **Cross-cutting — RTK rover** | 90% | Good as-is |
| **Cross-cutting — contour generation** | 0% | **Build (Sprint 12)** |
| **Cross-cutting — COGO** | 5% | **Build (Sprint 12)** |
| **Cross-cutting — LSA** | 0% | Build (Sprint 13) |
| **Cross-cutting — total station import** | 0% | Build (Sprint 13) |
| **Cross-cutting — feature codes** | 0% | Build (Sprint 13) |
| **Cross-cutting — profile sheets** | 20% | Build (Sprint 13) |

---

## Recommendation Path

### Sprint 12 (current — QA/QC + COGO + contours + end-area)
1. **QA/QC foundation** — `UncertainValue`, `verify_calculation`, range checks ✅ Done
2. **Wire UncertainValue into VolumeResult** — every volume shows "± m³ (95%)" 
3. **TIN-based volume cross-check** — grid vs TIN agreement flag on every report
4. **COGO module** — inverse, intersection, offset, curve, area subdivision, DMD
5. **Contour generation** — marching squares, GeoJSON + DXF output
6. **End-area volume method** — for haul roads, ramps, dredge channels

### Sprint 13 (total station + LSA + feature codes)
7. **Total station raw import** — Trimble `.job`, Leica `.gsi`, Topcon `.raw`
8. **Traverse adjustment** — Bowditch / Crandall's, closure checks
9. **Least-squares adjustment engine** — observation equations, normal equations, chi-square, Baarda
10. **Feature code library** — JSON + auto-symbolization + auto-line-connection
11. **Profile sheet PDF generator** — haul road / channel centerline profiles
12. **Coordinate transformation grids** — NTv2, NADCON via `proj` crate

### Sprint 14 (advanced)
13. **GNSS static post-processing** — RTKLIB plugin wrapper
14. **Geometric point-cloud classifiers** — highwall, toe/crest, vegetation, wires
15. **USACE EM 1110-1-1004 QA checks**
16. **ASPRS QL reporting**

### Out of scope (belongs to other tools)
- Cadastral boundary determination → MetaRDU web app
- BIM coordination → Revit / Navisworks
- Mobile PWA field capture → separate product
- Plan of Survey generator → jurisdiction-specific packages

---

## Bottom Line

For a mining + marine surveyor, MetaRDU Industrial is **the right primary tool** and is ~85-90% coverage today. Sprint 12 (QA/QC + COGO + contours + end-area) pushes it to ~92%. Sprint 13 (total station + LSA + feature codes) pushes it to ~95%. The remaining 5% is niche (deformation sub-mm, S-100, bathymetric lidar) that can be deferred or partnered.

The cadastral gap that was flagged in the original analysis belongs to the separate MetaRDU web app — MetaRDU Industrial should stay focused on what it does best: **mining and marine surveying, done defensibly**.
