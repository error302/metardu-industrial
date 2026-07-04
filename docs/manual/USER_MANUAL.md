# MetaRDU Industrial — User Manual

**Version**: 0.1.0-beta.1  
**Date**: 2026-07-03  
**Audience**: Mine surveyors, hydrographic surveyors, survey contractors

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Boot Sequence](#2-boot-sequence)
3. [Workspace Overview](#3-workspace-overview)
4. [File Ingest](#4-file-ingest)
5. [Mining Tools](#5-mining-tools)
6. [Marine Tools](#6-marine-tools)
7. [Cross-Cutting Tools](#7-cross-cutting-tools)
8. [Automation](#8-automation)
9. [Settings](#9-settings)
10. [Keyboard Shortcuts](#10-keyboard-shortcuts)

---

## 1. Getting Started

### Prerequisites

- **Node.js** 22+ and npm
- **Rust** 1.87+ ([rustup.rs](https://rustup.rs))
- **Tauri 2.0 system deps** — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- **Docker** (optional — only for ODM photogrammetry pipeline)

### Installation

#### From source

```bash
git clone https://github.com/error302/metardu-industrial.git
cd metardu-industrial
npm install
cargo tauri dev    # development
cargo tauri build  # production installer
```

#### From release

Download the installer for your platform from [GitHub Releases](https://github.com/error302/metardu-industrial/releases):
- Windows: `.msi`
- macOS: `.dmg`
- Linux: `.deb` or `.AppImage`

### First launch

On first launch, MetaRDU Industrial shows an onboarding screen:
1. **Select your domain**: Mining, Marine, or Both
2. **Pick your default coordinate system**: EPSG:4326 (WGS 84), MGA zones, UTM zones, NAD83, RGF93

These choices configure the default panels, shortcuts, and color mode. Switchable later in Settings.

---

## 2. Boot Sequence

MetaRDU Industrial boots in four stages:

1. **Splash screen** (~2.5s): animated logo with theodolite-lens rotation
2. **Module loading** (~4s): initializes PROJ, GDAL, PDAL, SpatiaLite, coordinate registry, marine/mining readers, reporting engine
3. **Onboarding** (first run only): domain + EPSG selection
4. **Workspace**: main application window

If a module fails to load, it appears in red on the module loading screen. Optional modules (marine, mining) can fail without blocking boot — only core modules (geodesy, raster, pointcloud, spatialite, coord-reg, reporting) are required.

---

## 3. Workspace Overview

The workspace has five regions:

```
┌─────────────────────────────────────────────────────────┐
│ Title Bar (brand mark, domain badge, window controls)    │
├──────┬──────────────────────────────────┬───────────────┤
│      │                                   │               │
│ Side │   OpenLayers Map Canvas           │  Right Panel  │
│ bar  │   (with Deck.gl point cloud       │  (survey      │
│      │    overlay + CUBE raster)         │   status,     │
│      │                                   │   staged      │
│      │                                   │   files,      │
│      │                                   │   profile)    │
│      │                                   │               │
├──────┴──────────────────────────────────┴───────────────┤
│ Status Bar (CRS, domain, UTC clock, version)             │
└─────────────────────────────────────────────────────────┘
```

- **Left sidebar**: project tree, domain-specific tools, automation, settings
- **Map canvas**: OpenLayers 10 with OSM basemap, graticule, coordinate readout, file bounds rendering, CUBE raster overlay, Deck.gl point cloud
- **Right panel**: survey status, staged files list, elevation profile (when profile tool active)
- **Floating actions** (top-right): toggle panels, profile tool, volume calc, settings
- **Status bar**: active CRS, domain mode, UTC clock, app version

### Color modes

- **Mining mode**: industrial dark with amber/yellow accents
- **Marine mode**: deep navy with turquoise/cyan accents  
- **Both mode**: orange accents (mixed)

---

## 4. File Ingest

### Drag and drop

Drag survey files onto the map canvas. Supported formats:

| Extension | Format | Parser |
|---|---|---|
| `.las` | LAS 1.2/1.3/1.4 point cloud | Pure-Rust header parser |
| `.laz` | LAZ compressed | Not yet supported (Phase 1) |
| `.tif` / `.tiff` | GeoTIFF raster (DEM, orthomosaic) | Pure-Rust TIFF + GeoTIFF reader |
| `.all` | Kongsberg multibeam | Datagram walker |
| `.s7k` | Reson Teledyne multibeam | Record walker |
| `.bsf` | R2Sonic multibeam | Stub (Phase 2) |
| `.mrk` | DJI MMC drone manifest | Text parser |
| `.json` | DJI FlightHub export | JSON parser |
| `.csv` | Generic drone manifest / tabular | CSV parser |
| `.gpkg` | GeoPackage vector | Not yet supported |
| `.kml` | KML | Not yet supported |

### What happens when you drop a file

1. File is added to the **Staged Files** list in the right panel
2. `probe_file` IPC command reads the file header
3. Bounds are rendered as a vector rectangle on the map (with point count label for LAS)
4. Map auto-fits to the file extent
5. For GeoTIFFs with EPSG info, a **CRS switch banner** appears if the file's EPSG differs from the active map CRS

### CRS auto-switch

When a GeoTIFF's EPSG differs from the active map CRS, a blue banner appears:
- **Switch**: updates the map CRS to match the file
- **Dismiss**: keep the current CRS

---

## 5. Mining Tools

### 5.1 ODM Pipeline (Terminal icon)

Runs OpenDroneMap via Docker to convert drone photos into a classified point cloud.

**Prerequisites**: Docker installed, `docker pull opendronemap/odm:latest`

**Steps**:
1. Open from sidebar → Mining → ODM Pipeline
2. The dialog checks Docker + image availability automatically
3. Enter the images directory path (folder of JPEG/TIFF files)
4. Configure: max concurrency, feature quality (ultra/high/medium/low/lowest), output format (LAS/LAZ/PLY/CSV), skip 3D model
5. Click **Run pipeline**
6. Progress streams in real-time with log tail
7. On completion, the resulting LAS is auto-added to the staged files list

### 5.2 Classify Ground — CSF (Layers3 icon)

Runs Cloth Simulation Filter ground extraction on a loaded LAS point cloud.

**Steps**:
1. Drop a LAS file on the map
2. Open from sidebar → Mining → Classify (CSF)
3. Select the LAS file from the dropdown
4. Configure parameters:
   - **Cloth resolution** (m): grid spacing for cloth particles (default 0.5)
   - **Classification threshold** (m): max distance from cloth for ground (default 0.5)
   - **Max iterations**: cap on simulation steps (default 500)
   - **Terrain rigidness**: 1=gentle, 2=sloped, 3=cliff
   - **Time step**: simulation dt (default 0.65)
   - **Max points**: limit for very large clouds (0 = all)
5. Click **Classify**
6. Result shows ground/non-ground counts + ground ratio bar
7. The point cloud on the map recolors: green = ground, orange = non-ground

### 5.3 Volume Calculator (Calculator icon)

Computes fill/cut volumes by differencing two GeoTIFF DEM surfaces.

**Steps**:
1. Drop 2 GeoTIFF DEMs on the map
2. Open from sidebar → Mining → Volume Calculator
3. Select **Current survey** DEM
4. Choose **Reference surface**:
   - **Flat plane**: enter elevation in meters (for stockpile-to-base volumes)
   - **Previous survey DEM**: select from loaded GeoTIFFs
5. Set **Bench interval** (m) for bench-by-bench breakdown (0 = skip)
6. Click **Compute**
7. Result shows:
   - Fill / Cut / Net volume tiles (m³)
   - Cell area + fill/cut cell counts
   - Bench breakdown table with per-band fill/cut/net

### 5.4 4D Monitoring (History icon)

Compares two GeoTIFF DEMs from different survey epochs to detect pit progression.

**Steps**:
1. Drop 2 GeoTIFF DEMs (previous + current survey)
2. Open from sidebar → Mining → 4D Monitoring
3. Select **Previous survey** and **Current survey**
4. Configure:
   - **Rock density** (t/m³): 2.7 for iron ore, 1.6 for coal
   - **Hotspot threshold** (m): elevation change flagged as hotspot
5. Click **Compute diff**
6. Result shows:
   - Fill / Cut / Net volume + tonnage tiles
   - Hotspot count with warning badge
   - Cell statistics: fill/cut/stable/no-data counts
   - Max fill/cut, mean Δz, RMS Δz

### 5.5 ML Classification (Brain icon)

Two ML tools in a tabbed dialog:

**Seafloor Habitat tab** (also in Marine section):
- Enter backscatter features (mean intensity, std, angular slope, texture homogeneity, depth)
- Click **Classify** → returns habitat class (Rock/Sand/Mud/etc.) + confidence + probability bars

**Blast Fragmentation tab**:
- Enter fragment sizes (mm, one per line)
- Click **Analyze** → returns P20/P50/P80/P90 percentiles + quality assessment (Excellent/Acceptable/Coarse/VeryCoarse)

---

## 6. Marine Tools

### 6.1 CUBE Surface (Waves icon)

Generates a gridded bathymetric surface from sounding data using the CUBE algorithm.

**Steps**:
1. Open from sidebar → Marine → CUBE Surface
2. Choose sounding source:
   - **Synthetic data**: 10,000-point test dataset (for trying the feature without real data)
   - **CSV input**: enter soundings as `x,y,depth,uncertainty` per line
3. Configure CUBE parameters:
   - **Grid resolution** (m): cell size for output grid
   - **Capture distance** (m): max distance for hypothesis merging
   - **Init uncertainty** (m): starting sigma per hypothesis
   - **Max hypotheses**: per cell before pruning
   - **Min soundings**: per cell for inclusion
4. Click **Generate surface**
5. The CUBE depth grid renders as a **blue raster overlay** on the map with a depth legend

### 6.2 S-44 Compliance (Shield icon)

Checks IHO S-44 (6th edition, 2022) compliance for a batch of soundings.

**Steps**:
1. Open from sidebar → Marine → S-44 Compliance
2. Select **Target survey order**: Special / 1a / 1b / 2
3. Enter soundings as CSV: `depth,vertical_tpu_95,horizontal_tpu_95` per line
   - Pre-populated with 100 synthetic soundings for testing
4. Click **Check compliance**
5. Result shows:
   - Status banner: Pass (green) / Investigate (amber) / Fail (red)
   - Pass/fail counts + pass rate
   - Min/max depth
   - Worst failures table (top 20) with per-sounding TPU vs threshold

### 6.3 S-57 Export (Anchor icon)

Digitizes marine features and exports them to an S-57 .000 file.

**Steps**:
1. Open from sidebar → Marine → S-57 Export
2. Review/edit the feature table:
   - **Object class**: WRECKS, OBSTRN, UWTROC, DEPARE, SOUNDG, COALNE, LNDARE
   - **Longitude / Latitude**: WGS84 decimal degrees
   - **Sounding**: VALSOU attribute (meters)
   - **Extra attributes**: `LABEL=VALUE;LABEL=VALUE` format
3. Add/remove rows as needed
4. Set the **export file path** (.000 extension)
5. Click **Export S-57**
6. The .000 file is written to disk — ingestible by CARIS S-57 Composer

### 6.4 ML Classification (Brain icon)

Same dialog as mining ML — the Seafloor Habitat tab is shared between domains.

---

## 7. Cross-Cutting Tools

### 7.1 Elevation Profile (TrendingUp icon)

Draws a line on the map and shows the elevation profile along it.

**Steps**:
1. Click the TrendingUp floating action button (top-right)
2. Click two points on the map — the profile line is drawn
3. The right panel shows an SVG elevation profile:
   - If a GeoTIFF DEM is loaded: **Real DEM** badge (bilinear-sampled elevations)
   - If no DEM: **Synthesized** badge (multi-octave sine noise for demo)
4. Profile shows: distance, min/max/Δ elevation, grid lines
5. Click again to redraw, or click **Clear**

### 7.2 Point Cloud Rendering

When a LAS file is active (clicked in the Staged Files list), points render on the map via Deck.gl:
- **Before CSF**: all points in steel gray
- **After CSF**: ground = green, non-ground = orange
- **LOD decimation**: points are spatially decimated based on map zoom level
  - Zoom ≥16: all points (no decimation)
  - Zoom 14: 1m cells
  - Zoom 12: 5m cells
  - Zoom 10: 25m cells
  - Zoom <10: 100m cells
- Point count badge at bottom-center shows total + ground/non-ground counts

### 7.3 CRS Switch Banner

When a dropped GeoTIFF's EPSG differs from the active map CRS, a blue banner appears at the top of the canvas:
- **Switch**: updates the map CRS
- **Dismiss**: keep current CRS

### 7.4 Settings (gear icon)

Configure application defaults:

- **Default domain**: Mining / Marine / Both
- **Default coordinate system**: EPSG dropdown (10 common CRSs)
- **UI density**: Comfortable (16px rows) / Compact (12px rows)
- **Reduced motion**: disable splash animations

Settings persist to `app_config_dir/settings.json` via Tauri IPC.

---

## 8. Automation

### 8.1 Pipelines

Define processing pipelines as YAML. Each pipeline has steps that chain together via template variables.

**Open**: sidebar → Automation → Pipelines

**YAML schema**:

```yaml
name: "Pipeline name"
description: "Optional description"
steps:
  - id: step_id
    action: <action_name>
    params:
      key: value
    outputs:
      output_name: "{{steps.step_id.output_name}}"
```

**Template variables**:
- `{{input.*}}` — pipeline input parameters (passed at run time)
- `{{steps.<id>.*}}` — outputs from previous steps

**Example: Drone → Volume Report**

```yaml
name: "Drone → Volume Report"
description: "Ingest drone photos, classify, compute volumes, generate report"
steps:
  - id: ingest
    action: odm_pipeline
    params:
      images_dir: "{{input.dir}}"
      feature_quality: high
  - id: classify
    action: classify_ground
    params:
      path: "{{steps.ingest.las_path}}"
      cloth_resolution: 0.5
  - id: volume
    action: compute_volumes
    params:
      current_path: "{{steps.ingest.las_path}}"
      reference_path: "flat:100.0"
      bench_interval: 5.0
  - id: report
    action: generate_report
    params:
      output_path: "{{input.dir}}/report.html"
```

**Running a pipeline**:
1. Edit the YAML in the editor
2. Click **Run pipeline**
3. Watch per-step results + log output stream in real-time
4. Each step shows: status (✓/✗), action type, elapsed time

### 8.2 Watch Folders

Automatically trigger pipelines when new files appear in a directory.

**Open**: sidebar → Automation → Watch Folders

**Setup**:
1. Click **Add**
2. Enter:
   - **Path**: directory to watch
   - **Pipeline name**: which pipeline to trigger
   - **Extensions**: comma-separated (e.g., `las,tif,all`)
3. The watcher polls every 5 seconds
4. New files are detected after a 2-second cool-down (to avoid partial writes)
5. Stats show: files detected, pipelines triggered, last file

### 8.3 Scheduled Jobs

Run pipelines on a recurring interval.

**Open**: sidebar → Automation → Scheduled Jobs

**Setup**:
1. Click **Add**
2. Enter:
   - **Name**: job name (e.g., "Daily QC")
   - **Pipeline name**: which pipeline to run
   - **Interval** (seconds): e.g., 86400 = daily
3. First run is immediate; subsequent runs respect the interval
4. Stats show: runs completed, last run, next run countdown

---

## 9. Settings

Access via sidebar → Settings or the gear floating action button.

| Setting | Options | Default |
|---|---|---|
| Default domain | Mining / Marine / Both | Both |
| Default coordinate system | 10 EPSG presets | EPSG:4326 |
| UI density | Comfortable / Compact | Comfortable |
| Reduced motion | On / Off | Off |

Settings are persisted to disk and survive restarts.

---

## 10. Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `?` | Show shortcut overlay (planned) |
| `Ctrl+O` | Open file (planned) |
| `Ctrl+S` | Save project (planned) |
| `Esc` | Close active dialog |
| `Click` on staged file | Activate point cloud rendering |

---

## Appendix A: Supported File Formats

| Format | Extension | Read | Write | Notes |
|---|---|---|---|---|
| LAS 1.2/1.3/1.4 | `.las` | ✅ | — | Header + points |
| LAZ | `.laz` | ❌ | — | Phase 1 |
| GeoTIFF | `.tif`, `.tiff` | ✅ | — | Uncompressed + LZW |
| Kongsberg .all | `.all` | ✅ | — | Datagram walk |
| Reson .s7k | `.s7k` | ✅ | — | Record walk |
| R2Sonic .bsf | `.bsf` | Stub | — | Phase 2 |
| DJI MMC | `.mrk` | ✅ | — | Text parser |
| DJI FlightHub | `.json` | ✅ | — | JSON parser |
| CSV | `.csv` | ✅ | — | Generic |
| S-57 | `.000` | — | ✅ | ISO 8211 writer |
| GeoPackage | `.gpkg` | ❌ | — | Phase 4 |
| KML | `.kml` | ❌ | — | Phase 4 |

---

## Appendix B: Technical Stack

| Layer | Technology |
|---|---|
| Shell | Tauri 2.0 (Rust) |
| Frontend | React 19 + TypeScript + Vite |
| Map (2D) | OpenLayers 10 |
| Map (point cloud) | Deck.gl 9 |
| CRS | proj4js + PROJ 9 (behind feature flag) |
| Styling | Tailwind CSS 4 |
| State | Zustand |
| Storage | SpatiaLite / PostGIS |
| Build | Cargo + npm workspaces |
| CI | GitHub Actions (Win/macOS/Linux) |
