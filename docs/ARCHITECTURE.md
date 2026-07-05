# MetaRDU Industrial — Architecture & Engineering Plan

> **Document status**: Living document, v1.0
> **Repository**: `github.com/error302/metardu-industrial`
> **Scope**: Specialised desktop application for mining and marine survey workflows, automation, and QA/QC.
> **Last revised**: 2026-07-02

---

## 0. Executive Summary

MetaRDU Industrial is a cross-platform, offline-capable desktop application built to automate the repetitive 60–70% of mining and marine survey work that currently burns project time in data wrangling, format translation, and manual QA/QC. It is a **standalone product** — not a sibling, plugin, or companion to any existing cadastral or topographic web app — with its own binary, brand identity, update channel, and user data directory.

The application is built on **Tauri 2.0** (Rust core, web frontend) for a ~10 MB binary that runs on rugged field laptops, **OpenLayers 10** as the primary 2D map canvas (no vendor lock-in, no API keys, full OGC and custom-CRS support), and a **dual-domain shared core** architecture where geodesy, point cloud processing, and coordinate registration are reused across mining and marine modules. The visual identity is derived directly from the MetaRDU Industrial logo: navy base, industrial orange accent, with domain-specific yellow (mining) and turquoise (marine) accents.

This document defines the complete engineering plan: technology stack, system architecture, module-level workflows, UI design system, loading and onboarding flows, map stack, advanced engineering features, development roadmap, and risk mitigations.

---

## 1. Strategic Positioning

### 1.1 What MetaRDU Industrial Is

MetaRDU Industrial is a **workflow-automation layer** that sits between raw sensor data (UAV imagery, terrestrial laser scans, multibeam echosounders, side-scan sonar, GNSS streams) and downstream planning or charting systems (Surpac, Datamine, Deswik, CARIS S-57 Composer, ENC editors). Its primary value proposition is **auditable automation pipelines** — every output sounding, pixel, or vector feature traces back to its source through a provenance graph, satisfying the QA/QC requirements that mine site surveyors and hydrographic offices demand.

### 1.2 What It Is Not

MetaRDU Industrial is **not** another QGIS plugin, a CARIS clone, or a generic GIS. It is not a cadastral or topographic engineering tool — those domains are handled by separate products and will not be merged into this codebase. It is not a SaaS: the application runs fully offline, with optional networked PostgreSQL/PostGIS for team scenarios, and never requires an internet connection for core operation.

### 1.3 Target Users

- **Mine surveyors** at open-pit and underground operations, responsible for monthly volume reconciliations, blast design, stockpile surveys, and subsidence monitoring.
- **Hydrographic surveyors** at ports, offshore energy sites, and survey contractors, responsible for IHO S-44 compliant bathymetry, feature detection, and ENC production support.
- **Survey contractors** who serve both domains and need a single toolset that respects both workflow conventions without compromise.

### 1.4 Differentiators

The commercial market is dominated by CARIS (marine, expensive, Windows-only), Surpac/Datamine/Deswik (mining planning, not survey automation), and QGIS (general-purpose, no domain automation). MetaRDU Industrial's wedge is:

1. **Dual-domain** in a single tool — contractors stop paying for two software stacks.
2. **Automation-first** — watch folders, YAML pipelines, scheduled jobs.
3. **Auditable provenance** — every output traces to inputs via a DAG.
4. **Cross-platform** — Windows, macOS, Linux from one binary.
5. **Open data formats** — Parquet, GeoTIFF, GPKG, S-57, LAS/LAZ. No lock-in.
6. **Open core** — processing engine open-source, UI and pro plugins commercial.

---

## 2. Technology Stack

### 2.1 Core Framework — Tauri 2.0

Tauri 2.0 is the application shell. The Rust core handles all heavy processing, geodesy, and file I/O; the web frontend (React + TypeScript) handles UI and user interaction. Communication happens through Tauri's IPC layer (JSON-RPC over a custom protocol).

**Rationale**: Mining field laptops are ruggedized Panasonics or Dell Latitudes with 8–16 GB RAM and integrated graphics. Electron's 150+ MB memory footprint per window is a liability on these machines. Tauri's ~10 MB binary and native webview keep memory low and startup fast. Marine surveyors increasingly run macOS on survey vessels for Unix tooling, and Tauri's cross-platform story is clean.

**Why not .NET MAUI**: Excellent for Windows-only deployments, but marine surveyors on macOS would be excluded. Cross-platform is non-negotiable for a dual-domain product.

**Why not Qt**: Rust-native ecosystem is more aligned with modern geospatial libraries (GDAL, PDAL, PROJ all have Rust bindings), and the web frontend gives us access to OpenLayers, Deck.gl, and CesiumJS without reimplementation.

### 2.2 Processing Engine (Rust)

| Library | Purpose | Version |
|---|---|---|
| `gdal-rs` | Raster/vector I/O, format translation, warp | GDAL 3.8+ |
| `proj` (Rust bindings) | Coordinate transforms, CRS management | PROJ 9.4+ |
| `pdal-sys` | Point cloud pipelines, classification, ground extraction | PDAL 2.6+ |
| `rayon` | Data-parallel processing for point clouds, grids | latest |
| `tokio` | Async runtime for streaming ingest, network I/O | 1.x |
| `polars` | DataFrame for survey metadata, fix lists, position data | 0.40+ |
| `ndarray` + `ndarray-linalg` | Least-squares adjustments, geoid modeling | latest |
| `rusqlite` + SpatiaLite | Embedded local cache, spatial index | latest |
| `sqlx` (PostgreSQL/PostGIS) | Networked multi-user mode | 0.7+ |
| `arrow` + `parquet` | Canonical interchange format, columnar storage | 50+ |
| `serde` + `serde_json` | Serialization for configs, pipelines, metadata | latest |
| `tracing` | Structured logging, pipeline provenance | latest |

### 2.3 Frontend (UI Layer)

| Library | Purpose | Version |
|---|---|---|
| React 19 + TypeScript | UI framework | 19.2+ |
| Vite | Build tooling, HMR | 5+ |
| Zustand | State management | 4+ |
| OpenLayers 10 | **Primary 2D map canvas** | 10+ |
| Deck.gl 9 | WebGL-accelerated overlay for heavy datasets (used inside OL) | 9+ |
| CesiumJS | 3D viewport (separate tab, not default) | 1.120+ |
| Nivo | QC charts, profiles, statistical plots | latest |
| Monaco Editor | Pipeline YAML/JS scripting UI | latest |
| TanStack Query | Async data fetching from Rust core | 5+ |
| Tailwind CSS 4 | Utility-first styling, design token enforcement | 4+ |
| Radix UI | Accessible primitives (modals, dropdowns, menus) | latest |
| Lucide React | Icon set (industrial-feel, consistent stroke) | latest |

### 2.4 Data Pipeline & Interchange

- **Apache Arrow / Parquet** is the canonical interchange format for tabular survey data (soundings, fix lists, classified points). Columnar, compressed, schema-rich, and increasingly the industry standard (USGS, NOAA, ESA).
- **GeoTIFF** (COG — Cloud-Optimized GeoTIFF where possible) for rasters: DEMs, bathymetric surfaces, backscatter mosaics.
- **GeoPackage (GPKG)** for vector data interchange — OGC standard, single-file, SpatiaLite-compatible.
- **LAS/LAZ 1.4** for point clouds.
- **S-57 / S-101** for marine chart outputs.
- **DGN / DXF / LandXML** for mining plan interchange with Surpac/Datamine/Bentley.
- **DuckDB** (embedded) for in-process analytical queries on survey datasets — fast aggregation over millions of soundings without loading into memory.

### 2.5 Build & Distribution

- **Monorepo**: Cargo workspace for Rust crates, npm workspaces for frontend packages.
- **CI/CD**: GitHub Actions matrix — Windows (x64), macOS (Universal), Linux (x64 + arm64 AppImage).
- **Code signing**: Windows Authenticode, macOS Developer ID + notarization, Linux GPG signatures.
- **Updates**: Tauri's built-in updater with Ed25519-signed manifests, delta patches for bandwidth-conscious field deployments (satcom from vessels is expensive).
- **Installers**: `.msi` (Win), `.dmg` + `.pkg` (mac), `.deb` + `.rpm` + `.AppImage` (Linux).

---

## 3. System Architecture

### 3.1 High-Level Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Window (React UI)                   │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ Mining Mode  │  │ Marine Mode  │  │ Pipelines/Admin  │   │
│  │ Workspace    │  │ Workspace    │  │ Workspace        │   │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘   │
└─────────┼──────────────────┼───────────────────┼────────────┘
          │                  │                   │
          │     Tauri IPC (JSON-RPC, typed)      │
          │                  │                   │
┌─────────▼──────────────────▼───────────────────▼────────────┐
│                  Rust Processing Core                        │
│                                                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  Geodesy Layer  │  │ Point Cloud Eng │  │ Coord Reg.   │ │
│  │  (PROJ, ITRF,   │  │ (PDAL, ICP,     │  │ (LSQ Adjust, │ │
│  │   geoid, tides) │  │  classification)│  │  deformation)│ │
│  └────────┬────────┘  └────────┬────────┘  └──────┬───────┘ │
│           │                    │                   │         │
│  ┌────────▼────────┐  ┌────────▼────────┐  ┌──────▼───────┐ │
│  │  Mining Module  │  │  Marine Module  │  │  Reporting   │ │
│  │  (volumes,      │  │  (CUBE, TPU,    │  │  Engine      │ │
│  │   benches,      │  │   S-44, S-57,   │  │  (PDF, KML,  │ │
│  │   blast design) │  │   backscatter)  │  │   DXF, GTiff)│ │
│  └────────┬────────┘  └────────┬────────┘  └──────┬───────┘ │
│           │                    │                   │         │
│  ┌────────▼────────────────────▼───────────────────▼───────┐ │
│  │           Pipeline Orchestrator (DAG, provenance)        │ │
│  └────────────────────────┬────────────────────────────────┘ │
└───────────────────────────┼──────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────┐
│       Storage Layer                                          │
│                                                              │
│  SpatiaLite (default, local)  ←→  PostgreSQL + PostGIS (LAN) │
│  Parquet / GeoTIFF / LAS (file-based survey datasets)        │
│  DuckDB (analytical cache)                                   │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Architectural Principles

**Principle 1 — Shared core, domain shells.** The geodesy, point cloud, and coordinate registration layers are shared across mining and marine modules. Approximately 60% of the underlying math (coordinate transforms, least-squares, surface modeling, point cloud operations) is identical between domains. Only the domain-specific top-level modules differ. This eliminates duplicated maintenance and ensures bug fixes in shared math propagate to both domains.

**Principle 2 — Offline-first.** Every feature works without internet. Networked PostGIS is an *optional* enhancement for multi-user teams, never a hard requirement. License validation uses signed tokens that survive months offline.

**Principle 3 — Provenance is non-optional.** Every output (sounding, pixel, vector feature, report) traces back to its inputs through an immutable DAG stored alongside the data. This is the foundation of survey-grade QA/QC and the regulatory compliance story for both IHO S-44 and mining JORC/NI 43-101 contexts.

**Principle 4 — Pipelines are first-class.** Automation isn't a bolt-on feature; it's a core abstraction. Every operation the UI can do can also be expressed as a YAML pipeline step and run headlessly. This is what makes the watch-folder and scheduled job features possible without duplicating logic.

**Principle 5 — No silent data loss.** Coordinate transforms that fail, point cloud classifications with low confidence, soundings that fail S-44 thresholds — all are flagged, not dropped. The user always sees what was rejected and why.

### 3.3 Process Model

The Tauri main process hosts the Rust core. Heavy processing jobs (CUBE surface generation, point cloud classification, volume calculations) run as **detached worker tasks** via `tokio::spawn` so the UI never blocks. Long-running pipelines report progress through Tauri events; the UI subscribes and updates a pipeline status panel.

For extremely heavy jobs (100M+ soundings, 500M+ point clouds), an optional **out-of-process worker** mode spawns a dedicated binary that communicates over a local Unix socket or named pipe. This isolates crashes — a PDAL segfault takes down the worker, not the main app.

---

## 4. Mining Survey Module

### 4.1 Core Workflows

#### 4.1.1 UAV/Drone Photogrammetry Ingestion

Auto-detect projects from DJI (MMC/FlightHub), Phoenix LiDAR, and SenseFly exports. The ingestion pipeline reads the camera EXIF, flight log, and ground control point file, then runs structure-from-motion. For SfM, MetaRDU Industrial integrates **OpenDroneMap** (via the `opendronemap/odm` Docker image or native bindings) as the default engine, with optional **MicMac** integration for higher-accuracy mining-grade outputs. Output is a classified point cloud (LAS 1.4) and orthomosaic (COG).

#### 4.1.2 Terrestrial Laser Scanner (TLS) Registration

Multi-station registration using a two-pass approach: coarse ICP on overlap regions, then fine registration using surveyed target coordinates with a least-squares bundle adjustment. Supports target types commonly used in mining (checkerboard, spherical, prism). The registration report includes station residuals, point cloud overlap percentages, and a chi-square test on the adjustment.

#### 4.1.3 Point Cloud Classification

A layered classifier: CSF (Cloth Simulation Filter) for ground extraction as the base, then a custom Random Forest model trained on mining-specific classes (highwall, bench, ramp, stockpile, vegetation, infrastructure, water). Users can retrain the model on their site-specific data via the ML Plugin. Confidence scores are preserved per point — classifications below threshold are flagged, not silently dropped.

#### 4.1.4 Volume Calculations

Two modes: **stockpile** (triangulated surface to a reference plane, with base-plane auto-detection or user-specified) and **pit** (current survey differenced against previous survey or design surface). Volume results break down by bench level, with tonnage estimates using user-supplied density (per lithology if available). Outputs include a PDF report with cross-sections, a CSV breakdown, and a GeoPackage of the calculation surfaces.

#### 4.1.5 Blast Design Automation

From rock factor (per lithology), powder factor, and bench geometry, MetaRDU Industrial auto-generates drill patterns: burden, spacing, sub-drill, stemming, and timing. Patterns are editable in the 3D viewport and exportable to drill rig GPS systems via standard CSV or vendor-specific formats (Sandvik, Epiroc). The blast design report includes powder consumption, expected fragmentation (Kuz-Ram model), and a 3D visualization of the pattern.

#### 4.1.6 4D Pit Progression

Multi-temporal surface differencing: register N surveys in a common frame, generate a per-cell elevation change raster, integrate to monthly volume deltas, and reconcile against the mine plan (Surpac block model imported via DXF or CSV). Discrepancies greater than a user-defined threshold (default 5%) are flagged for investigation. The 4D viewer scrubs through time with a slider, showing the pit evolve month-by-month.

#### 4.1.7 Subsidence Monitoring

For underground operations and tailings dams: deformation vectors between epochs, statistical significance testing (F-test on residuals), and automated alerts when displacement exceeds tolerance. Long-term monitoring uses an InfluxDB backend (optional) for time-series storage of control point movements.

### 4.2 Mining Automations

- **Watch folder → ingest → classify → volume calc → email PDF report.** The classic mine surveyor workflow, fully automated. Drop the drone SD card contents into a watched folder on the surveyor's laptop; the pipeline runs to completion overnight.
- **Survey-to-plan reconciliation.** Auto-compare the monthly survey against the Surpac/Datamine block model. Flag bench over-dig, under-dig, and stockpile discrepancies >5%.
- **Coordinate system auto-detection.** Sniff WKT/PRJ/EPSG from inputs, propose a transformation pipeline to the mine grid, and log the transformation chain for audit.
- **Blast plan templates.** Per-bench, per-rock-type templates with parameter sweeps — generate five variants of a pattern in one click.

---

## 5. Marine Survey Module

### 5.1 Core Workflows

#### 5.1.1 MbES Ingestion

Unified reader for Kongsberg `.all` and `.kmwcd`, Reson Teledyne `.s7k`, R2Sonic `.bsf`, and Norbit `.wbm`. The reader parses datagrams, extracts bathymetry, backscatter, snippet, and attitude, and stages data in Parquet for downstream processing. Multi-ping streaming allows partial processing during acquisition for real-time QC.

#### 5.1.2 Sound Velocity Correction

Apply SVP casts with ray-tracing through the water column. SVPs are stored with timestamps and applied based on temporal proximity to each ping. The user is warned when the SVP is stale (configurable threshold, default 6 hours). Multiple SVPs across a survey are interpolated spatially and temporally.

#### 5.1.3 Tide & Water Level Correction

Integrate tide gauge streams (TCOON, NOAA CO-OPS, port authority feeds), apply VDATUM transformations, and zone tides per IHO standards. For surveys without local tide gauges, predicted tides from XTide or NOAA are supported with explicit uncertainty propagation into TPU.

#### 5.1.4 Position/Attitude Filtering

Kalman filter on GNSS positions with IMU integration for attitude (heave, pitch, roll, yaw). The filter accepts position corrections (RTK, PPK, SBAS) and outputs a smoothed navigation solution at ping rate. Smoothing artifacts are flagged — a smooth track is not always a correct track.

#### 5.1.5 CUBE Surface Generation

NOAA's CUBE (Combined Uncertainty and Bathymetry Estimator) algorithm is the surface generation engine. CUBE is public domain (developed at UNB/CCOM), making it the right choice for an open-core product. CUBE+ (CARIS's enhanced variant) is licensed and will not be reimplemented. The output is a gridded bathymetric surface with per-cell uncertainty and a hypothesis stack for QC.

#### 5.1.6 IHO S-44 Compliance

Per the 6th edition (2022): Order 1a, 1b, 2, and Special Order feature detection and uncertainty budgets. MetaRDU Industrial computes the Total Propagated Uncertainty (TPU) for every sounding — combining sensor, attitude, SVP, tide, and coordinate transformation uncertainties — and flags soundings that fail their target order. The S-44 compliance panel gives a global survey status (Pass / Investigate / Fail) and drill-down to failing soundings.

#### 5.1.7 Side Scan Sonar Mosaicking

Georeferenced mosaics from SSS waterfall data, with slant-range correction, beam-pattern correction, and tile-based mosaicking. Target digitization uses IHO INT 1 symbology — wrecks, obstructions, rocks, seabed changes — not generic pins. The digitized targets export as S-57 objects or GeoPackage features.

#### 5.1.8 Backscatter Processing

Angular response analysis for seafloor classification: compute the backscatter angular response curve per tile, classify using supervised (random forest) or unsupervised (clustering) methods, and produce a habitat map. Classifications are exported alongside the bathymetric surface.

#### 5.1.9 Chart Production

S-57 object creation for ENC production support. The S-57 writer produces compliant files for ingestion by CARIS S-57 Composer or direct submission to a hydrographic office. S-101 (next-generation ENC) support is on the roadmap, contingent on the S-101ENC production tooling maturing.

### 5.2 Marine Automations

- **Daily QC pipeline.** Overnight run: ingest → clean → CUBE → TPU → S-44 flags → morning report PDF. The surveyor arrives at 06:00 to a complete QC summary of the previous day's acquisition.
- **Wreck/obstruction auto-detection.** YOLOv8 model trained on backscatter mosaics and MbES snippet data. Detections are staged for surveyor review — never auto-classified as final.
- **Cross-line error analysis.** Statistical comparison at line intersections, surface bias removal via a median filter on cross-line differences. Bias >5% of depth triggers a calibration alert.
- **Vessel coordinate frame calibration (patch test).** Automated patch test on calibration lines: solve for roll, pitch, yaw, and navigation latency offsets using a least-squares fit. Results include confidence intervals and a recommendation to accept or rerun.

---

## 6. UI Design System

### 6.1 Design Philosophy

The UI must **telegraph which domain the user is in**. A generic GIS UI fails both mining and marine surveyors. The application boots in workspace selection mode; the user picks Mining or Marine (or both in split view). Each mode shifts the color palette, default panels, keyboard shortcuts, and map symbology. The design language is derived directly from the MetaRDU Industrial logo to ensure brand consistency.

### 6.2 Color Tokens (Extracted from Logo)

```
Brand Core
─────────────────────────────────────────────
--color-navy-base:        #0A192F   /* primary background */
--color-industrial-orange: #FFA500  /* primary accent, CTAs */
--color-white:            #FFFFFF   /* primary text */
--color-steel-gray:       #6B7280   /* secondary text, dividers */

Mining Mode Accents
─────────────────────────────────────────────
--color-mining-yellow:    #FFC107   /* mining-mode accents */
--color-mining-burnt:     #FFB347   /* mining-mode secondary */
--color-mining-terrain:   #8B4513   /* earth tones, terrain */

Marine Mode Accents
─────────────────────────────────────────────
--color-marine-deep:      #1E3A8A   /* marine-mode primary */
--color-marine-turquoise: #20B2AA   /* marine-mode accent, water */
--color-marine-cyan:      #06B6D4   /* depth indicators */

Geospatial Grid
─────────────────────────────────────────────
--color-survey-grid:      #1E2A3F   /* subtle grid lines */
--color-crosshair:        #FFA500   /* coordinate cursor */
--color-coord-readout:    #E5E7EB   /* monospaced coordinate text */

Semantic States
─────────────────────────────────────────────
--color-pass:             #10B981   /* S-44 pass, validation OK */
--color-investigate:      #F59E0B   /* S-44 investigate */
--color-fail:             #EF4444   /* S-44 fail, error */
--color-info:             #3B82F6   /* informational */
```

### 6.3 Logo DNA → UI Application

The logo is the source of truth for visual language. Each visual element in the logo maps to a concrete UI component:

| Logo Element | UI Translation |
|---|---|
| **Theodolite / total station** as central focal point | The app's workspace hub — survey project selector occupies the center of the home screen |
| **Split lens** (mining terrain / marine water) | The dual-mode toggle; split-screen mining/marine comparison view for contractors serving both domains |
| **Coordinate grid overlay** in the lens | The survey grid drawn on every map canvas (toggleable, snapped to active CRS) |
| **White "M" frame** in background | The window chrome accent — M-shaped top-left brand mark in every window title bar |
| **Drone with signal arcs** | Status indicator icon (data acquisition active, GNSS fix quality, sonar ping rate) |
| **Sonar grid pattern** underwater | Loading pattern for marine data ingestion; subtle background pattern in marine mode |
| **Excavator/truck silhouettes** | Workflow icons for mining operations (blast design, volume calc, stockpile) |
| **Horizontal dividers** around "MINING & MARINE SURVEYS" | Section divider style throughout the UI — thin orange lines with center labels |

### 6.4 Typography

- **Headings & UI body**: Inter (Latin), with `Inter Display` for large headings. Free, open-source, exceptional legibility at small sizes.
- **Coordinates & monospaced readouts**: JetBrains Mono. Coordinates MUST be monospaced — non-negotiable for surveyors reading MGA/UTM coordinates where digit alignment matters.
- **Numeric readouts (depths, volumes, residuals)**: IBM Plex Mono as an alternative where the engineering feel is preferred.
- **CJK fallback** (for international mine sites): Noto Sans SC, Noto Sans JP, Noto Sans KR.

### 6.5 Layout System

- **Grid**: 8px base unit. All spacing, padding, and component sizes are multiples of 8 (8, 16, 24, 32, 48, 64).
- **Density modes**: "Compact" (12px row height, for surveyors with 4K monitors running multiple panels) and "Comfortable" (16px row height, for field laptops). Toggle in Settings.
- **Dark mode default**: Vessels run dark at night; mining control rooms are dim. Light mode is available but secondary.
- **Keyboard-first**: Every panel action has a shortcut. Surveyors despise mouse-heavy UIs. Shortcuts are discoverable via a `?` overlay (GitHub-style).
- **Multi-window support**: Pop out any panel into its own Tauri window. Critical for multi-monitor control rooms and survey vessels with chartplotters on secondary displays.

### 6.6 Mining Mode Layout

Three-pane layout optimized for the mine surveyor's primary task: visualize the pit, cut a cross-section, read the volume report.

```
┌─────────────────────────────────────────────────────────────┐
│ Menu │ Mining Workspace - BHP Iron Ore Pit A │ Survey #43   │
├──────┼──────────────────────────────────────────────────────┤
│      │                                                      │
│ Tree │         3D Pit Visualization (CesiumJS)              │
│ Nav  │         with bench coloring and volume               │
│      │         isolines overlaid                            │
│  -   ├──────────────────────────────────────────────────────┤
│ Surv │   Cross-Section Profile    │  Volume Calc Report    │
│  ey  │   (along-design-line)      │  (bench-by-bench)      │
│      │                            │                         │
└──────┴────────────────────────────────────────────────────┘
```

**Key UX patterns**:
- **Bench ribbons** — horizontal color-coded bands in the 3D view showing active mining level.
- **Volume isolines** — contour overlays on pit surfaces.
- **Drag-to-section** — draw a line on the 3D view, cross-section generates instantly below.
- **Stockpile heatmaps** — color by tonnage density.
- **Time slider** — scrub through 4D progression, see pit evolve month-by-month.

### 6.7 Marine Mode Layout

Four-pane layout for surveyors who need data density: data tree on the left, bathymetric surface in the primary pane, cross-track profile and TPU/S-44 status on the right.

```
┌─────────────────────────────────────────────────────────────────┐
│ Marine Workspace - Port of Darwin Survey  │ Vessel: RV Solander │
├────────────┬────────────────────────────────────────────────────┤
│            │                                                     │
│  Data Tree │     Bathymetric Surface (OpenLayers + CUBE)         │
│  - Lines   │     with S-44 uncertainty overlay                   │
│  - Casts   │     and feature flags                               │
│  - Tide    │                                                     │
│            │                                                     │
│            ├─────────────────────────┬──────────────────────────┤
│            │  Cross-Track Profile    │  TPU / S-44 Status Panel │
│            │  (depth vs distance)    │  (per-sounding TPU)      │
└────────────┴─────────────────────────┴──────────────────────────┘
```

**Key UX patterns**:
- **Depth color ramp** — IHO standard color palette 5 (or user-selectable 1–5).
- **Uncertainty halos** — every sounding shows a translucent halo sized by TPU.
- **Feature pins** — wrecks, obstructions, rocks with IHO INT 1 symbology (paper-chart style, not generic pins).
- **Along-track profile** — instantaneous depth profile as you hover the surface.
- **SVP timeline** — when was the last cast? How stale is the SVP? Color-coded staleness.
- **S-44 compliance badge** — global survey status (Pass / Investigate / Fail) in the top bar.

---

## 7. Loading & Onboarding Flows

For a serious desktop survey app, loading isn't a single screen — it's a **choreographed sequence** that establishes trust with a professional user base. Each stage communicates specific progress and surfaces failures explicitly.

### 7.1 Cold-Start Splash

A transparent Tauri window with the navy background, animated logo, and a thin orange progress bar. Duration: ~1.5 seconds on a typical field laptop.

The theodolite lens rotates 90° as modules load. The split-lens fills from mining (top half) and marine (bottom half) simultaneously — a visual metaphor for dual-domain initialization that reinforces brand identity. The version string and build date display below the logo to confirm the user is running the expected release.

### 7.2 Module Loading Screen

After the splash, a fuller screen shows module-by-module initialization. This is critical for surveyors — if PDAL or GDAL fails to load, they need to see *which* module failed and why, not a generic "application failed to start" message.

Each module is listed with its version (PROJ 9.4, GDAL 3.8, PDAL 2.6) and load time. Failures show the error message inline with an "Open logs" button. A "Hide details" toggle collapses the list for users who just want the app to start.

### 7.3 Project Loading Screen

When the user opens a 500 MB point cloud or a 2 GB MbES file, the loading screen shows step-by-step progress: reading the header, indexing the spatial tree, classifying points (where applicable), and building the LOD pyramid. An animated grid fills with points as the dataset loads — a visual confirmation that data is flowing. The ETA updates based on actual processing rate, not a fixed estimate.

A "Background" button lets the user dismiss the loading screen and continue working in other parts of the app; the load completes in the background and posts a notification when ready.

### 7.4 Pipeline Execution Screen

When running an automation pipeline (watch folder, scheduled job, or manual run), the execution screen shows the pipeline DAG as a vertical step list with status indicators (pending, running, complete, failed, skipped). Elapsed time, ETA, and a live log tail provide full visibility. The user can pause, skip a step, or cancel without losing completed intermediate outputs.

### 7.5 First-Run Onboarding

When the user launches MetaRDU Industrial for the first time, the onboarding screen asks two questions:

1. **Which surveys will you be running?** — Mining, Marine, or both. This configures default panels, shortcuts, and the active color mode. Switchable later in Settings.
2. **Default coordinate system** — a searchable EPSG picker with proj4js definitions. Common mining and marine CRSs are surfaced as quick-picks (UTM zones, MGA, local mine grid examples).

The onboarding is skippable — power users can dismiss it and configure later. But for first-time users, it sets up sensible defaults without requiring a trip to Settings.

---

## 8. Map Stack

### 8.1 Why OpenLayers as Primary

Both OpenLayers and MapLibre GL are 100% free and open source — neither requires subscriptions, API keys, or accounts. (Mapbox GL is the one that requires a token; MapLibre is the open fork.) The decision is purely technical, and for **survey-grade work, OpenLayers wins**.

| Requirement | OpenLayers | MapLibre GL | Winner |
|---|---|---|---|
| OGC services (WMS/WMTS/WFS/WCS) | First-class, mature since 2006 | Limited, requires tile proxy | **OL** |
| Custom CRS / on-the-fly reprojection | Native, via proj4js | Vector tiles assume Web Mercator | **OL** |
| Mine grid support (local engineering CRS) | Register custom proj4js def — done | Hard; VT spec is Web Mercator-locked | **OL** |
| Survey graticules, scale bars, coordinate readouts | Built-in, configurable | Basic | **OL** |
| Symbolization (IHO S-57 / mining symbology) | Full Style API, per-feature | Style spec, less flexible | **OL** |
| Format support (GeoJSON, KML, GML, WKT, MVT, GPKG, Shapefile) | 20+ formats native | MVT + GeoJSON only | **OL** |
| Field laptop GPU reliability | Canvas 2D fallback works anywhere | WebGL mandatory | **OL** |
| Performance on 10M+ vector features | Slower (Canvas) | Faster (WebGL) | MapLibre |
| 3D terrain | Limited | Better | MapLibre |
| Modern aesthetic | Functional, less polished | Beautiful, modern | MapLibre |

For mining and marine survey workflows, the first seven rows matter more than the last three. Surveyors need WMS overlays from state agencies (Geoscience Australia, USGS, NOAA, state portals), custom CRS switching (mine grids are local engineering CRSs, not EPSG entries), and bulletproof rendering on rugged laptops with integrated graphics. OpenLayers is the industry standard for exactly this.

### 8.2 Hybrid Map Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Map Stack                              │
├──────────────────────────────────────────────────────────┤
│                                                           │
│  PRIMARY 2D CANVAS  ── OpenLayers 10+                    │
│  ─────────────────────────────────────                    │
│  • WMS/WMTS/WFS/WCS ingestion                            │
│  • Custom CRS via proj4js (mine grids, marine datums)    │
│  • Survey symbology (IHO INT 1, mining conventions)      │
│  • Graticules, scale bars, coordinate readout            │
│  • GeoJSON/KML/GML/Shapefile/GPKG                        │
│  • All day-to-day survey work                            │
│                                                           │
│  HEAVY-DUTY WEBGL LAYER  ── Deck.gl (optional)           │
│  ─────────────────────────────────────                    │
│  • Only spawned when dataset > 1M features               │
│  • Point clouds (10M+ points)                            │
│  • Dense MbES soundings                                  │
│  • Embedded as OpenLayers layer (deck.gl-OL adapter)     │
│                                                           │
│  3D VIEWPORT  ── CesiumJS (separate tab/window)          │
│  ─────────────────────────────────────                    │
│  • Pit visualization, 4D progression                     │
│  • Marine 3D bathymetry                                  │
│  • Sub-surface geology                                   │
│  • NOT used for 2D survey work                           │
│                                                           │
└──────────────────────────────────────────────────────────┘
```

OpenLayers is the **default canvas** users see 90% of the time. Deck.gl layers are added *inside* OpenLayers when WebGL acceleration is needed (via the deck.gl OpenLayers integration). CesiumJS is a separate 3D viewport, not a replacement for the 2D map.

This means:
- No vendor lock-in, no API keys, no subscriptions anywhere in the stack.
- Native support for the OGC services surveyors actually use.
- WebGL only where it's justified (massive datasets).
- Canvas 2D fallback for rugged field laptops with weak GPUs.

### 8.3 Specific OpenLayers Features Leveraged

1. **`ol/proj/proj4` integration** — register mine grids (`+proj=tmerc +lat_0=... +k_0=... +x_0=...`) and marine datums as custom CRS. One-click switching, with the active CRS displayed in the status bar.
2. **`ol/source/TileWMS`** — pull WMS basemaps from Geoscience Australia, USGS, NOAA, state portals. No proxy needed.
3. **`ol/format/WKT` and `ol/format/GML`** — direct import/export of survey control points and cadastral data.
4. **`ol/layer/VectorTile`** — for large background layers (admin boundaries, coastline) served as MVT.
5. **`ol/control/MousePosition`** — bottom-bar coordinate readout, projected into whatever CRS is active. Monospaced, copy-on-click — surveyors will appreciate this.
6. **`ol/style/Style` with `circle/regularShape`** — implement IHO S-57 symbology natively (wrecks, rocks, obstructions) and mining markers (drill holes, survey stakes).
7. **`ol/source/Cluster`** — for dense control point networks.
8. **`ol/interaction/Draw` and `ol/interaction/Modify`** — for digitizing features (wrecks, stockpile polygons, bench lines) with snap-to-grid in the active CRS.

---

## 9. Engineering "Push to the Limits" Features

These are the features that differentiate MetaRDU Industrial from commercial alternatives and justify its existence as a new product rather than another QGIS plugin.

### 9.1 Real-Time Streaming Ingest

Process MbES pings as they arrive over UDP, not just post-mission. The marine surveyor sees a live bathymetric surface build on the OpenLayers canvas during acquisition. This requires a high-performance streaming pipeline in Rust (tokio + crossbeam channels) and a throttled renderer that decimates for display without losing data for the CUBE surface. Target: sustain 50 pings/second on a survey laptop without dropping frames.

### 9.2 WASM-Based Pipeline Scripting

Users write custom processing steps in JavaScript (or a domain-specific language that transpiles to JS), which runs in a sandboxed WASM runtime (wasmtime) embedded in the Rust core. The sandbox has explicit, declared permissions — a script can read from the input dataset and write to the output, but cannot touch the filesystem or network without declaration. This enables a plugin ecosystem without the security risk of native plugins.

### 9.3 Distributed Processing

For heavy jobs (CUBE on a 50M sounding dataset, classification on a 500M point cloud), MetaRDU Industrial can spawn workers across a local cluster. The orchestrator uses a work-stealing scheduler with chunk-based parallelism. This is opt-in: the default is single-machine, and the cluster mode requires explicit configuration. Target use case: survey vessels with a small render farm on board, or mining contractors with a workstation rack.

### 9.4 Time-Series Database for Monitoring

Long-term subsidence monitoring (mining) and morphodynamic monitoring (marine, for sand-wave tracking) benefit from time-series storage. MetaRDU Industrial integrates InfluxDB (optional, self-hosted) for control point displacement histories and seabed change time series. The UI surfaces trends, anomalies, and alert thresholds.

### 9.5 ML-Assisted Classification

Pre-trained models for seafloor habitat classification (from backscatter) and blast fragmentation analysis (from drone imagery of muck piles). Users can retrain on their site-specific data via the ML Plugin, which exposes a transfer-learning pipeline. Models run via the ONNX runtime, embedded in the Rust core.

### 9.6 Provenance Graph

Every output — every sounding, pixel, vector feature, report — traces back to its inputs through an immutable DAG stored alongside the data. The provenance graph is queryable: "show me every sounding that contributed to this CUBE cell" or "show me every report that used data from this survey line." This is the foundation of survey-grade QA/QC and the regulatory compliance story.

### 9.7 Versioned Surveys

Git-like versioning of survey datasets: mine survey v43.1, with diff against v43.0. The diff is a meaningful survey diff (added/removed points, changed classifications, volume delta), not a binary diff. This supports the audit trail required by JORC and NI 43-101 reporting standards.

### 9.8 Plugin SDK

A Rust SDK for third-party sensor support. A sonar vendor can ship a MetaRDU Industrial plugin that registers their proprietary format reader, and it loads at startup without recompiling the main app. The SDK is documented, semver-stable, and the API surface is minimal: register a reader, register a processor, register an exporter.

### 9.9 Hardware Integration

Direct drivers for total stations (Leica, Trimble, Topcon) over serial or TCP, GNSS rovers (RTK corrections via NTRIP), and sonars over NMEA 0183/2000. The hardware abstraction layer means new devices are added without touching the core.

### 9.10 Augmented Reality Field Mode

An iPad companion app (separate binary, syncs with the desktop app) for stakeout with AR overlays. The surveyor holds up the iPad, sees the design points overlaid on the real world through the camera, and verifies placement. This is a Phase 4+ feature and may be descoped based on demand.

---

## 10. Development Roadmap

### Phase 0 — Foundation (Months 1–2)

- Repository bootstrap: Cargo workspace + npm workspaces + Tauri 2.0 shell.
- CI/CD pipeline: GitHub Actions matrix (Windows, macOS, Linux), code signing, notarization.
- Core geodesy layer: PROJ integration, CRS management UI, proj4js integration in frontend.
- OpenLayers 10 integration with custom CRS support, graticule, scale bar, coordinate readout.
- CesiumJS integration for the 3D viewport (separate tab).
- Basic file ingest: LAS/LAZ, GeoTIFF, GeoPackage, Kongsberg `.all`, Reson `.s7k`.
- Splash screen, module loading screen, first-run onboarding.
- Design system implementation: Tailwind tokens, typography, density modes.

**Exit criteria**: A user can launch the app, see the branded splash, pick a workspace, open a LAS file, and see it on an OpenLayers canvas in their local CRS.

### Phase 1 — Mining MVP (Months 3–5)

- UAV photogrammetry ingestion (DJI, SenseFly).
- Point cloud classification (CSF + custom RF).
- Volume calculation engine (stockpile + pit modes).
- Pit visualization with bench extraction in CesiumJS.
- Cross-section tool (drag-to-section).
- PDF/QGIS report export.
- First automation pipeline: drone → volume report.

**Exit criteria**: A mine surveyor can drop a drone survey into the app, classify it, compute volumes by bench, and generate a PDF report end-to-end.

### Phase 2 — Marine MVP (Months 6–8)

- MbES full ingest pipeline (.all, .s7k, .bsf).
- SVP correction with ray-tracing.
- Tide and water level correction.
- Position/attitude Kalman filtering.
- CUBE surface generation.
- TPU calculation.
- S-44 compliance checks (Order 1a/1b/2/Special).
- S-57 export.
- First marine automation pipeline: daily QC.

**Exit criteria**: A hydrographic surveyor can ingest a day's MbES acquisition, generate a CUBE surface with TPU, check S-44 compliance, and export an S-57 file.

### Phase 3 — Automation Layer (Months 9–11)

- Watch-folder automation with file-pattern matching.
- Pipeline YAML DSL with full operator coverage.
- Scheduled jobs (cron-style) for daily QC.
- Email and notification system (SMTP + webhook).
- Pipeline DAG visualization and provenance graph storage.
- WASM-based scripting sandbox for custom steps.

**Exit criteria**: A contractor can configure a watch folder, drop files in, and receive an emailed PDF report the next morning without manual intervention.

### Phase 4 — Advanced (Months 12–15)

- 4D monitoring (mining): multi-temporal differencing, mine plan reconciliation.
- ML classification models (seafloor habitat, blast fragmentation) with transfer learning.
- Plugin SDK (Rust) for third-party sensor support.
- Distributed processing (local cluster mode).
- AR companion app (iPad) for stakeout — descoped if demand is low.
- S-101 support (next-gen ENC), contingent on tooling maturity.

**Exit criteria**: MetaRDU Industrial has feature parity with the core workflows of CARIS and Surpac's survey modules, with automation as the differentiator.

### Phase 5 — Polish & Certify (Months 16–18)

- IHO S-44 certification preparation (if pursuing formal certification).
- CASA/aviation compliance review (if UAV features are promoted).
- Performance hardening: handle 100M+ point datasets, 50M+ sounding surveys without degradation.
- Documentation: user manual, API reference, plugin SDK guide, training videos.
- Localization: at minimum English, Spanish, French, Mandarin (mining and marine are international industries).

**Exit criteria**: MetaRDU Industrial is a production-ready, certified, documented, and localized product suitable for commercial release.

---

## 11. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| CUBE algorithm licensing (NOAA) | Low | High | NOAA CUBE is public domain; CUBE+ in CARIS is licensed — stick with open CUBE. Verify with NOAA CCOM before commercial release. |
| MbES proprietary format changes | Medium | Medium | Reverse-engineer via open-source readers (.all/.s7k are documented). Maintain format readers as plugins so vendor-specific code is isolated. |
| Surveyor adoption (entrenched CARIS/Surpac) | High | High | Open data formats, plugin SDK, lower pricing, automation ROI calculator. Pilot programs with two contractors in each domain. |
| Cross-platform GPU perf for large point clouds | Medium | Medium | WebGPU via Deck.gl where supported; Canvas 2D fallback; LOD streaming for >50M points. |
| Field deployment (no internet) | Low | High | Fully offline-first; license check via signed token valid for 90+ days. Updates distributed as signed delta patches, installable from USB. |
| Rust ecosystem maturity for geospatial | Medium | Medium | `gdal-rs` and `pdal-sys` are actively maintained but less mature than Python equivalents. Maintain a Python fallback for experimental features (via PyO3) where the Rust path is risky. |
| ML model drift in classification | Medium | Medium | Confidence scores preserved per point; low-confidence classifications flagged for manual review. Retraining pipeline exposed in the ML Plugin. |
| Tauri 2.0 breaking changes | Low | Medium | Pin Tauri version in Cargo.lock; subscribe to Tauri release notes; maintain a `tauri-1.x` fallback branch for emergencies. |
| Regulatory certification (IHO, JORC) | Medium | High | Engage with IHO and mining regulators early. Phase 5 dedicates time to certification preparation. If certification is required for market entry, the timeline extends by 3–6 months. |
| Single-maintainer bus factor | High | High | Open-core model: processing engine open-source encourages community contributions. Document everything. Pair-program critical modules. |

---

## 12. Repository Structure

```
metardu-industrial/
├── .github/
│   └── workflows/
│       ├── ci.yml                # Frontend check + core test + rust check + build matrix
│       └── release.yml           # Tagged-release artifact builder
├── src/                          # React 19 frontend (TypeScript + Vite)
│   ├── components/               # 33 dialogs + map canvas + point cloud layer
│   ├── screens/                  # Splash, module loading, onboarding, workspace shell
│   ├── stores/                   # Zustand stores (app-store, survey-store)
│   ├── lib/                      # Tauri IPC wrapper, CRS registry, hooks, tokens
│   └── App.tsx                   # Root: splash → modules → onboarding → workspace
├── src-tauri/                    # Tauri 2.0 shell (Rust)
│   ├── src/
│   │   ├── commands/             # IPC command modules (108 commands across 10 files)
│   │   ├── marine/               # Dredge, density gates, cross section, tidal spline, SVP
│   │   ├── mining/               # CSF, volume, machine control, highwall, 4D monitoring, drone ingest
│   │   ├── formats/              # LAS, GeoTIFF, Kongsberg .all, Reson .s7k, SSS XTF
│   │   ├── geodesy/              # Pure-Rust UTM transform + optional PROJ/GDAL bindings
│   │   ├── pipelines/            # ODM (OpenDroneMap) Docker shell-out
│   │   ├── plugins/              # Dynamic plugin loader (signature-gated) + registry
│   │   ├── automation/           # Pipeline orchestrator, watch folders, scheduler
│   │   ├── distributed/          # Distributed CUBE coordinator (TCP)
│   │   ├── streaming.rs          # Live sonar stream listener (UDP)
│   │   ├── telemetry.rs          # Opt-in telemetry + crash dump capture
│   │   ├── license.rs            # HMAC license system (Pro/Enterprise)
│   │   ├── updater.rs            # Auto-updater (STUB — see RELEASE.md)
│   │   └── lib.rs                # Tauri entry point + invoke_handler registration
│   ├── keys/
│   │   └── license_pub.pem       # Bundled RSA-2048 public key (for license + plugin verification)
│   └── Cargo.toml
├── crates/
│   └── metardu-core/             # Shared pure-Rust processing core (no system deps)
│       ├── src/
│       │   ├── marine/           # CUBE, S-44, S-57, SVP, TPU
│       │   ├── mining/           # LAS, CSF, DEM, DXF, EOM pipeline, volume, license, report
│       │   ├── ntrip/            # NTRIP/RTCM3 client with CRC-24Q verification
│       │   └── triage/           # Field data triage (EXIF, RINEX, NMEA, LAS headers)
│       └── tests/
│           └── integration.rs    # Cross-module integration tests
├── metardu-eom-cli/              # Standalone CLI: EOM pipeline + license signing
├── metardu-verify/               # Standalone CLI: PDF chain-of-custody verifier (free, open-source)
├── docs/
│   ├── ARCHITECTURE.md           # This file
│   ├── ROADMAP.md                # Sprint-by-sprint status
│   ├── manual/
│   │   ├── USER_MANUAL.md
│   │   ├── IPC_REFERENCE.md
│   │   └── PIPELINE_REFERENCE.md
│   ├── SECURITY.md               # Threat model, vulnerability reporting, known gaps
│   └── RELEASE.md                # Pre-flight checklist, build/sign/distribute steps
├── SECURITY.md                   # Security policy (root-level for GitHub detection)
├── RELEASE.md                    # Release checklist (root-level for discoverability)
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE                       # MIT
├── package.json                  # Frontend deps + scripts
└── README.md
```

> **Note:** The repository is a flat single-workspace layout, not the
> multi-workspace `apps/` + `packages/` + `crates/` monorepo originally
> envisioned in the Phase 0 architecture. The flat layout was chosen
> for simplicity during solo development — the `metardu-core` crate
> is the only shared Rust library, and the frontend is a single Vite
> app rather than a multi-package npm workspace. The `apps/desktop/`
> structure may be adopted if a second app (e.g. a field-data uploader)
> is added later.

---

## 13. Appendices

### Appendix A — Coordinate Reference System Handling

MetaRDU Industrial treats CRS as a first-class concern. Every dataset has an explicit CRS, stored as WKT2 in its metadata. On load, the dataset is *not* reprojected by default — it stays in its source CRS, and the OpenLayers canvas reprojects on the fly via proj4js. When the user explicitly reprojections (e.g., to merge datasets in a common frame), the transformation chain is logged in the provenance graph.

Mine grids are registered as custom CRSs via proj4js definitions. A mine grid library ships with common examples (e.g., Western Australian mine grids) and users can add their own via a CRS editor. Marine datums (CD, MSL, MLLW, LAT) are handled as vertical CRSs separate from the horizontal CRS, with VDATUM integration for transformations.

### Appendix B — Supported File Formats

**Read**:
- Raster: GeoTIFF (including COG), ERDAS IMG, JPEG2000, ASCII Grid
- Vector: GeoPackage, Shapefile, GeoJSON, KML, GML (WFS), DXF, DGN (limited)
- Point cloud: LAS 1.4, LAZ 1.4, PCD, PLY
- Marine: Kongsberg .all/.kmwcd, Reson .s7k, R2Sonic .bsf, Norbit .wbm, GSF
- Tabular: CSV, Parquet, Arrow, XLSX (metadata only)
- Survey: LandXML, Leica .dbx, Trimble .dc

**Write**:
- Raster: GeoTIFF (COG preferred), ASCII Grid
- Vector: GeoPackage, Shapefile, GeoJSON, KML, DXF, GML
- Point cloud: LAS 1.4, LAZ 1.4
- Marine: S-57, S-101 (Phase 4+), GSF
- Reports: PDF, HTML, CSV
- Tabular: Parquet, Arrow, CSV

### Appendix C — IHO S-44 Compliance (6th Edition, 2022)

MetaRDU Industrial computes TPU per sounding by combining:
- Sensor uncertainties (MbES beam angle, range, attitude sensor noise)
- Attitude uncertainties (roll, pitch, yaw, heave, latency)
- SVP uncertainties (cast accuracy, temporal/spatial representativeness)
- Tide and water level uncertainties (gauge accuracy, zoning errors)
- Coordinate transformation uncertainties (datum shift residuals)

The combined TPU is compared against the S-44 order threshold for the survey's target order. Soundings failing the threshold are flagged in the UI and excluded from the CUBE surface by default (configurable). The S-44 compliance report summarizes pass/investigate/fail counts per survey line, with drill-down to individual soundings.

### Appendix D — Provenance Graph Schema

Each processing step records:
- `step_id`: UUID
- `step_type`: ingest | classify | cube | volume | report | ...
- `inputs`: list of dataset UUIDs
- `outputs`: list of dataset UUIDs
- `parameters`: JSON object of step parameters
- `timestamp`: ISO 8601
- `operator`: user id or "automation"
- `software_version`: MetaRDU Industrial version + plugin versions
- `environment`: hostname, OS, hardware fingerprint

The provenance graph is stored in the local SQLite database (or PostGIS in networked mode) and is queryable via the UI. Every output dataset has a "trace lineage" action that walks the graph backward to its source inputs, with full parameter history.

### Appendix E — Performance Targets

| Operation | Dataset Size | Target Time | Hardware |
|---|---|---|---|
| LAS file load + index | 500 MB (142M pts) | <15s | Field laptop (i7, 16GB) |
| Point cloud classification (CSF + RF) | 142M pts | <3 min | Field laptop |
| Volume calculation (stockpile) | 10M pts surface | <5s | Field laptop |
| CUBE surface generation | 50M soundings | <8 min | Workstation (i9, 64GB) |
| S-44 TPU computation | 50M soundings | <2 min | Workstation |
| S-57 export | 100k features | <10s | Any |
| OpenLayers render (1M vector features) | — | 30 FPS pan/zoom | Field laptop |
| Deck.gl render (10M points) | — | 30 FPS pan/zoom | Workstation with GPU |

### Appendix F — Glossary

- **CUBE**: Combined Uncertainty and Bathymetry Estimator (NOAA public-domain algorithm)
- **CRS**: Coordinate Reference System
- **COG**: Cloud-Optimized GeoTIFF
- **MbES**: Multibeam Echosounder
- **MGA**: Map Grid of Australia (UTM-based)
- **RTK / PPK**: Real-Time / Post-Processed Kinematic GNSS positioning
- **S-44**: IHO Standards for Hydrographic Surveys (6th edition, 2022)
- **S-57**: IHO transfer standard for digital hydrographic data (current ENC format)
- **S-101**: Next-generation ENC standard (in development)
- **SSS**: Side Scan Sonar
- **SVP**: Sound Velocity Profile
- **TLS**: Terrestrial Laser Scanner
- **TPU**: Total Propagated Uncertainty
- **WKT2**: Well-Known Text version 2 (OGC standard for CRS definition)
