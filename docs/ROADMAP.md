# MetaRDU Industrial — Development Roadmap & Revenue Strategy

**Last updated**: 2026-07-03 (Sprint 8 complete)
**Status**: Living document — the single source of truth for what to build and why.

---

## Part 1: Technical Priority Order (Engineering)

Based on 20 years of field experience across mining and marine survey operations.

| Priority | Feature | Effort | Impact | Status | Sprint |
|---|---|---|---|---|---|
| 1 | Binary stream IPC (ArrayBuffer instead of JSON) | Medium | Unlocks 100M+ point rendering | ✅ DONE | Sprint 1 |
| 2 | Daylight high-contrast theme | Small | Unlocks outdoor field use | ✅ DONE | Sprint 1 |
| 3 | SVP editor with interactive graph | Medium | Unlocks credible marine processing | ✅ DONE | Sprint 3 |
| 4 | Command palette (Ctrl+K) | Small | Transforms daily workflow speed | ✅ DONE | Sprint 2 |
| 5 | Vessel lever-arm configuration | Medium | Makes TPU calculations real | ✅ DONE | Sprint 3 |
| 6 | CUBE hypothesis disambiguation UI | Medium | Turns CUBE from black box to tool | ✅ DONE | Sprint 3 |
| 7 | Layout profiles (predefined panel arrangements) | Small | Multi-monitor survey control rooms | ✅ DONE | Sprint 5 |
| 8 | SSS waterfall viewer | Large | Marine completeness | ✅ DONE | Sprint 6 |
| 9 | 3D slice editor with reject brush | Large | Manual cleaning | ✅ DONE | Sprint 6 |
| 10 | S-102 export | Large | Future-proofing (premature) | Deferred | ~2027 |

### Detail on Each Priority

#### 1. Binary Stream IPC (ArrayBuffer) — ✅ DONE (Sprint 1)
Current architecture sends JSON arrays over Tauri IPC — 4 copies of data in memory and 3 serialization passes. On 10M-point cloud that's ~480MB of JSON text through the webview thread.

**Fix**: Tauri's raw `ArrayBuffer` channel — Rust writes packed `f32` array into shared memory, webview receives as `ArrayBuffer`, Deck.gl's `Float32Array` wraps it with zero copies.

**Acceptance criteria**: 100M-point LAS file renders at 30fps on a ruggedized Panasonic Toughbook with integrated graphics.

**Implementation**: `read_las_points_binary` IPC command returns `Vec<u8>` (packed f32 LE). Frontend `readLasPointsBinary()` wraps as `Uint8Array` → `Float32Array`. PointCloudLayer uses binary path. 1M points = 12MB binary vs 40MB JSON. Legacy JSON path kept for backward compat.

#### 2. Daylight High-Contrast Theme — ✅ DONE (Sprint 1)
Navy `#0A192F` is perfect for dim survey cabins but unusable outdoors in direct sunlight.

**Fix**: CSS custom property swap — `--color-scheme: light` variant with white background, dark text, high-contrast accent colors. Toggle in Settings + status bar.

**Acceptance criteria**: UI fully readable on a laptop screen at noon in an open-pit mine.

**Implementation**: CSS `[data-theme="light"]` overrides all tokens (navy→white, white→dark, orange→darker). Settings dialog has "Dark (Cabin)" vs "Daylight (Field)" toggle cards. App.tsx useEffect applies `data-theme` to document root. AppStore extended with `theme: "dark" | "light"`.

#### 3. SVP Editor with Interactive Graph
We parse SVP casts in the `.all` reader but have no UI for editing them. Surveyors need to import `.asvp`/`.svp`, inspect depth-vs-speed curve, edit bad casts, apply to ray tracing.

**Acceptance criteria**: Import SVP file → see interactive graph → edit points → save corrected profile → apply to CUBE surface generation.

#### 4. Command Palette (Ctrl+K)
On a survey vessel in 2m seas, clicking 16×16px icons is impossible. Fuzzy-search command palette: type "apply svp" or "epsg 28355" → hit Enter → action executes.

**Acceptance criteria**: Ctrl+K opens overlay → fuzzy search across all actions/settings/CRS codes → Enter executes → Esc closes.

#### 5. Vessel Lever-Arm Configuration
Our `compute_tpu()` takes beam angle + travel time but doesn't account for physical offset between IMU and transducer. If IMU is 2m forward and 1m above transducer, a 5° roll error introduces 17cm horizontal position error at seabed.

**Acceptance criteria**: 3D visualization of vessel offsets (IMU → transducer → GNSS) → enter X/Y/Z offsets → TPU recalculates with lever-arm compensation.

#### 6. CUBE Hypothesis Disambiguation UI
Our CUBE tracks multiple hypotheses per cell but UI just shows the count. Hydrographer needs to see a map of ambiguous cells, click a cell, see alternative depth estimates, manually select the correct one.

**Acceptance criteria**: Map overlay showing ambiguous cells in amber → click cell → popover with alternative depths + uncertainty → select hypothesis → mark as "accepted".

#### 7. Layout Profiles — ✅ DONE (Sprint 5)
Predefined panel arrangements for common workflows (Data Ingest, Bathymetry Clean, Volume Reporting). Toggle bar in header switches between layouts.

**Implementation**: `LayoutProfiles` component in title bar with 4 presets (default / data_ingest / bathymetry_clean / volume_reporting). Each preset toggles sidebar + right panel. Active layout persisted in `localStorage` so it survives restarts. One-click switch — no dialog, no settings dive.

**Acceptance criteria**: 3 preset layouts → one-click switch → panels rearrange → state preserved per layout. ✅

#### 8. SSS Waterfall Viewer — ✅ DONE (Sprint 6)
Custom Canvas2D scrolling waterfall — X=across-track, Y=time (scrolling), pixel intensity=backscatter. Click two points to measure target height from shadow length.

**Implementation**: New `formats/sss_xtf.rs` module (~470 lines, 7 unit tests). Pure-Rust XTF parser walks ping packets, extracts port + starboard backscatter + nav + altitude. Windows FILETIME → Unix epoch conversion. Similar-triangles target height computation. `SssWaterfallViewer` React component renders Canvas2D scrolling waterfall with gain control + auto-scroll + click-to-measure target/shadow workflow. Acceptance criteria: Import .xtf → scrolling waterfall renders → click target + shadow → height computed → save as georeferenced POI. ✅

#### 9. 3D Slice Editor with Reject Brush — ✅ DONE (Sprint 6)
Draw bounding polygon over survey line → isolate slice in WebGL view → drag "reject brush" over outlier points → flag as rejected in SpatiaLite (undo-able).

**Implementation**: New `slice_editor.rs` module (~370 lines, 7 unit tests). Pure-Rust point-in-polygon ray-casting + shoelace area + `RejectMask` with undo stack + brush reject/restore. `SliceEditor3D` React component renders SVG top-down view with orange=accepted / red=rejected coloring, click-to-brush, undo button, export accepted indices for CUBE re-run. Acceptance criteria: Draw polygon → 3D view shows isolated points → brush selection → reject flagged → undo works → CUBE re-runs on cleaned data. ✅

#### 10. S-102 Export (HDF5)
S-100 framework is the future but ecosystem isn't ready. S-57 is the right priority now.

**Acceptance criteria**: Deferred until 2027 or when IHO S-102 tooling matures.

---

## Part 2: Revenue Features (Monetization)

### Open-Core / Proprietary Module Split

| Layer | License | Price | What's Included |
|---|---|---|---|
| **MetaRDU Core** | Open Source (MIT) | Free | Tauri shell, React UI, OpenLayers, LAS/GeoTIFF/.all/.s7k ingestion, basic volume calc, CSF, CUBE surface, S-44 check, S-57 export, pipeline DSL, watch folders |
| **MetaRDU Pro** | Commercial (per-seat) | $3,000-5,000/yr | EoM reconciliation reports, dredge pay-volume, branded PDF generator, highwall monitoring, stockpile audit, blast report, deliverable package, cross-section profiler |
| **MetaRDU Enterprise** | Commercial (per-site) | $10,000-25,000/yr | Distributed processing, plugin SDK, multi-user PostGIS sync, custom branding, priority support |

### Revenue Feature Priority (Ranked by Revenue Probability)

| Rank | Feature | Market | Price/Seat | Effort | Probability |
|---|---|---|---|---|---|
| 0 | **Branded PDF Report Engine** (infrastructure) | All | — | Medium | Required for all below |
| 1 | EoM Production Reconciliation (mining) | Large | $3,000-5,000/yr | Medium | 95% |
| 2 | Dredge Pay-Volume Audit (marine) | Medium | $5,000-10,000/project | Medium | 90% |
| 3 | S-44 Compliance Certificate (marine) | Medium | $2,000-3,000/yr | Small | 85% |
| 4 | Stockpile Inventory Audit (mining) | Large | $1,500-2,000/yr | Small | 85% |
| 5 | Blast Fragmentation Report (mining) | Medium | $2,000-3,000/yr | Medium | 80% |
| 6 | Highwall Deformation Monitoring (mining) | Growing | $5,000-10,000/yr | Medium | 70% |
| 7 | Survey Deliverable Package Generator (marine) | Medium | $3,000-5,000/yr | Medium | 75% |
| 8 | Cross-Section Profiler (marine) | Small | $2,000-3,000/yr | Small | 70% |

### Revenue Feature Details

#### 0. Branded PDF Report Engine (Infrastructure)
Every revenue feature requires a professional PDF report. This is the foundation.

**Spec**:
- Rust-side PDF generation (no webview dependency)
- Template-based: JSON spec defines sections, tables, images
- Branded headers/footers with MetaRDU logo
- Data tables (volume breakdowns, S-44 stats, bench-by-bench)
- Map screenshots (captured from OL canvas as PNG, passed to Rust)
- Provenance hash + audit trail footer
- Print-ready output

**Status**: Current `generate_report` pipeline action writes basic HTML. Need proper PDF engine.

#### 1. EoM Production Reconciliation
Mine surveyors spend 3 days every month doing: clean point clouds → grid surfaces → volume calc vs. mine plan → Excel report. MetaRDU compresses this to 30 minutes.

**Workflow**:
1. Drop two LAS files (previous + current month drone survey)
2. Auto-classify ground (CSF)
3. Draw pit perimeter polygon
4. Volume calc with bench breakdown + density factor → tonnage
5. Compare against mine plan (imported DXF/Surpac block model)
6. Generate branded PDF Reconciliation Audit Report with provenance trail

**Why it sells**: $3,000-5,000/seat. Mine with 5 surveyors = $15,000-25,000/year.

#### 2. Dredge Pay-Volume Audit
Dredging contracts worth $10-50M. Payment disputes come down to cubic meters removed. Both sides hire independent surveyors.

**Workflow**:
1. Import pre-dredge survey + post-dredge survey + design channel template
2. Compute: pay volume (above design), allowable overdredge (within tolerance), excessive overdredge (below tolerance — no pay), under-dredge/shoaling (remaining material)
3. Visual grid map showing each category in different colors
4. Generate branded PDF Dredge Audit Report

**Why it sells**: $5,000-10,000/project license. Every dredging project needs this.

#### 3. S-44 Compliance Certificate
Every hydrographic survey delivered to port authority/hydrographic office MUST include S-44 compliance statement. Currently produced manually in Excel.

**Workflow**:
1. Run S-44 compliance check (already built)
2. Generate branded PDF S-44 Compliance Certificate with:
   - Survey metadata (vessel, sonar, date, area)
   - TPU budget breakdown (per-source uncertainty)
   - Per-order compliance statistics
   - Worst-failure locations with coordinates
   - Provenance hash

**Why it sells**: $2,000-3,000/seat. Regulatory mandate = guaranteed market.

#### 4. Stockpile Inventory Audit
Mines report stockpile inventories monthly. Current: drone → DroneDeploy → Civil3D → Excel.

**Workflow**:
1. Drop LAS of stockpile yard
2. Draw polygon around each stockpile
3. Volume vs. previous survey → tonnage (density factor)
4. Generate branded PDF Stockpile Audit with stockpile photos

**Why it sells**: $1,500-2,000/seat. Every mine has 5-20 stockpiles, 12 times a year.

#### 5. Blast Fragmentation Report
After a blast, mine needs fragment size distribution + muck pile volume + diggability.

**Workflow**:
1. Process drone photos via ODM → point cloud
2. Run fragmentation analysis (already in `ml/mod.rs`)
3. Compute muck pile volume (already in `volume.rs`)
4. Compare actual vs. designed fragmentation
5. Generate branded PDF Blast Performance Report

**Why it sells**: $2,000-3,000/seat. Mine with 200 blasts/year = 200 reports.

#### 6. Highwall Deformation Monitoring — ✅ DONE (Sprint 5)
Post-Brazil 2020, slope stability monitoring is legally required in many jurisdictions.

**Implementation**: New `mining/highwall.rs` module (~370 lines, 10 unit tests). Tracks per-cell displacement TIME-SERIES across N epochs, computes velocity (mm/day) and acceleration (mm/day²). Three alert levels (Advisory >25mm, Watch >50mm or >1mm/day, Critical >100mm or >5mm/day) per USACE EM 1110-2-1900. Trend classification (Stable / Creeping / Accelerating / Failure Imminent). `HighwallMonitoringWizard` produces regulator-ready PDF compliance report.

**Why it sells**: $5,000-10,000/site/year. Safety-critical = non-negotiable budget.

#### 7. Survey Deliverable Package Generator — ✅ DONE (Sprint 5)
Marine surveyors assemble deliverable packages manually (4-6 hours).

**Implementation**: New `deliverable.rs` module (~640 lines, 7 unit tests). Bundles source files + ISO 19115 metadata XML + branded manifest HTML into a single ZIP. Added `zip` crate (pure-Rust, ~120KB). Manifest includes FNV-1a hash per file + warnings for missing files. `DeliverablePackageWizard` collects vessel/sonar/area metadata and source file paths.

**Why it sells**: $3,000-5,000/seat. Saves 4-6 hours per survey delivery.

#### 8. Cross-Section Profiler for Channel Design — ✅ DONE (Sprint 5)
Port engineers verify dredged channel meets design specs via cross-sections.

**Implementation**: New `marine/cross_section.rs` module (~470 lines, 6 unit tests). Walks a user-drawn centerline at `spacing_m` intervals, samples a perpendicular cross-section of `half_width_m` on each side using bilinear interpolation on the GeoTIFF. Computes under-dredge / over-dredge areas per section + compliance %. `CrossSectionProfilerWizard` accepts centerline as projected-coordinate text input (Sprint 6+ will auto-populate from map-drawn polygon).

**Why it sells**: $2,000-3,000/seat. Complements dredge volume engine.

---

## Part 3: What NOT to Build (Yet)

| Feature | Reason to Defer |
|---|---|
| S-102 / S-100 export | Ecosystem not ready until ~2027. Stay on S-57. |
| License/DRM system | Premature with zero customers. Build when 50+ paying users. |
| Full multi-panel docking (dockview) | 200KB bundle for niche use. Layout profiles cover 80% of value. |
| SSS pipeline tracking | Niche market, needs hardware vendor partnerships. Phase 7+. |
| Rust core telemetry HUD | Vanity metrics. Simple progress bar is more useful. |
| Cursor snapping to point vertices | kd-tree query per frame on 100M points = performance killer. |
| Dynamic plugin loading (libloading) | Built but no plugins exist yet. Ship static traits first. |

---

## Part 4: Current Codebase Status

### What's Built (Phase 0-5)

| Module | Lines | Tests | Status |
|---|---|---|---|
| LAS parser (pure Rust) | ~300 | — | ✅ Header + points |
| GeoTIFF parser (pure Rust) | ~430 | — | ✅ Header + pixel reader |
| Kongsberg .all reader | ~300 | — | ✅ Datagram walker |
| Reson .s7k reader | ~470 | — | ✅ Record walker |
| CSF ground extraction | ~390 | 4 | ✅ |
| Volume calculator | ~290 | 6 | ✅ |
| 4D monitoring | ~290 | 7 | ✅ |
| ML classification | ~240 | 6 | ✅ |
| CUBE surface generation | ~360 | 4 | ✅ |
| TPU calculation | ~270 | 3 | ✅ |
| S-44 compliance | ~310 | 5 | ✅ |
| S-57 export | ~430 | 2 | ✅ |
| Dredge pay-volume audit (Sprint 4) | ~370 | 8 | ✅ 4-bucket categorization |
| Highwall deformation monitoring (Sprint 5) | ~370 | 10 | ✅ Time-series + alerts + USACE thresholds |
| Cross-section profiler (Sprint 5) | ~470 | 6 | ✅ Bilinear DEM sampling + under/over-dredge |
| Survey deliverable package (Sprint 5) | ~640 | 7 | ✅ ZIP bundler + ISO 19115 metadata |
| SSS XTF parser (Sprint 6) | ~470 | 7 | ✅ Pure-Rust XTF + target-height computation |
| 3D slice editor (Sprint 6) | ~370 | 7 | ✅ Point-in-polygon + RejectMask undo stack |
| License Manager (Sprint 7) | ~560 | 14 | ✅ HMAC-SHA256 signed licenses + tier gating |
| Telemetry + Crash Reporter (Sprint 7) | ~420 | 9 | ✅ Opt-in usage stats + crash dump capture |
| Performance Benchmark Suite (Sprint 7) | ~370 | 5 | ✅ 8 benchmarks + throughput + p95 timing |
| Plugin SDK reference template (Sprint 7) | ~150 | — | ✅ Vendor-style FileReaderPlugin example |
| Project file format .metardu (Sprint 8) | ~340 | 10 | ✅ JSON-based save/load + auto-save + versioning |
| Auto-Updater (Sprint 8) | ~250 | 8 | ✅ Version check + download + install (Phase 9 will wire HTTP) |
| i18n en/es/pt (Sprint 8) | ~330 | 10 | ✅ English + Spanish + Portuguese translation tables |
| Plugin Marketplace (Sprint 8) | ~370 | 9 | ✅ Registry + install/uninstall + search + SHA-256 verify |
| Pipeline DSL + executor | ~280 | 4 | ✅ All 11 actions wired to real functions |
| Watch folders | ~220 | 2 | ✅ |
| Scheduled jobs | ~180 | 3 | ✅ |
| Plugin SDK | ~230 | 3 | ✅ |
| Dynamic plugin loader | ~170 | 3 | ✅ |
| Distributed processing | ~240 | 3 | ✅ Coordinator + TCP server |
| metardu-worker binary | ~230 | — | ✅ Full CUBE via shared crate |
| Streaming ingest | ~260 | 3 | ✅ UDP listener + Deck.gl rendering |
| WASM sandbox | ~280 | 3 | ✅ wasmtime behind feature flag |
| AR companion scaffold | ~310 | 3 | ✅ |
| Frontend (React/TS) | ~19,000 | — | ✅ 30 dialogs, 89 IPC commands |

### Build Stats
- Rust source: ~22,500 lines
- TypeScript source: ~19,000 lines
- Shared crate (metardu-core): ~1,500 lines
- Documentation: ~3,100 lines
- Unit tests: 196+ (Rust)
- IPC commands: 89
- Binaries: 2 (metardu-industrial + metardu-worker)
- Release tags: 2 (v0.1.0-alpha.1, v0.1.0-beta.1)

---

## Part 5: Build Order — Sprint Plan

### Sprint 1: Foundation for Revenue — ✅ COMPLETE
1. ~~**Binary stream IPC** (Priority #1)~~ — ✅ `read_las_points_binary` returns packed f32
2. ~~**Daylight theme** (Priority #2)~~ — ✅ CSS `[data-theme="light"]` + Settings toggle
3. ~~**Branded PDF report engine** (Revenue #0)~~ — ✅ `report_engine.rs` + `generate_report_cmd` IPC

### Sprint 2: First Revenue Features — ✅ COMPLETE
4. ~~**EoM Reconciliation wizard** (Revenue #1)~~ — ✅ 5-step wizard + branded PDF
5. ~~**S-44 Compliance Certificate** (Revenue #3)~~ — ✅ branded PDF certificate
6. ~~**Command palette** (Priority #4)~~ — ✅ Ctrl+K fuzzy search, 14 commands

### Sprint 3: Marine Credibility — ✅ COMPLETE
7. ~~**SVP editor** (Priority #3)~~ — ✅ Interactive graph + parser + interpolation
8. ~~**Vessel lever-arm config** (Priority #5)~~ — ✅ 2D diagram + offset inputs
9. ~~**CUBE hypothesis disambiguation** (Priority #6)~~ — ✅ Heatmap + cell inspector

### Sprint 4: More Revenue — ✅ COMPLETE
10. ~~**Dredge pay-volume audit** (Revenue #2)~~ — ✅ 4-bucket (pay / allowable OD / excessive OD / shoaling) + branded PDF
11. ~~**Stockpile inventory audit** (Revenue #4)~~ — ✅ Flat-or-previous baseline + tonnage + branded PDF
12. ~~**Blast fragmentation report** (Revenue #5)~~ — ✅ p20/p50/p80/p90 + muck volume + design-vs-actual + branded PDF

### Sprint 5: Polish & Scale — ✅ COMPLETE
13. ~~**Layout profiles** (Priority #7)~~ — ✅ 4 presets in title bar + localStorage persistence
14. ~~**Highwall monitoring with alerts** (Revenue #6)~~ — ✅ N-epoch time-series + USACE thresholds + compliance PDF
15. ~~**Survey deliverable package** (Revenue #7)~~ — ✅ ZIP bundler + ISO 19115 XML + branded manifest
16. ~~**Cross-section profiler** (Revenue #8)~~ — ✅ Bilinear DEM sampling + under/over-dredge detection

### Sprint 6+: Advanced — ✅ COMPLETE
17. ~~**SSS waterfall viewer** (Priority #8)~~ — ✅ XTF parser + Canvas2D scrolling + target-height measurement
18. ~~**3D slice editor with reject brush** (Priority #9)~~ — ✅ Polygon slice + RejectMask undo stack + brush QC
19. **S-102 export** (Priority #10) — Deferred until 2027 (IHO S-102 tooling not mature)

### Sprint 7: Enterprise Readiness — ✅ COMPLETE
20. ~~**License Manager**~~ — ✅ HMAC-SHA256 signed JSON licenses + Core/Pro/Enterprise/Trial tiers + feature gating
21. ~~**Telemetry + Crash Reporter**~~ — ✅ Opt-in usage stats + crash dump capture + per-stroke undo
22. ~~**Performance Benchmark Suite**~~ — ✅ 8 benchmarks (point cloud, CSF, volume, dredge, highwall, license, SHA-256, JSON) with p95 timing
23. ~~**Plugin SDK reference template**~~ — ✅ Vendor-style FileReaderPlugin example (Norbit WBM format) with full source

### Sprint 8: Production Distribution — ✅ COMPLETE
24. ~~**Project file format (.metardu)**~~ — ✅ JSON-based save/load + auto-save + versioning + recent reports
25. ~~**Auto-Updater**~~ — ✅ Version check + download + install (Phase 9 will wire real HTTP)
26. ~~**Internationalization (i18n)**~~ — ✅ English + Spanish + Portuguese (Latin American mining market)
27. ~~**Plugin Marketplace**~~ — ✅ Registry JSON + install/uninstall + search + SHA-256 verification
