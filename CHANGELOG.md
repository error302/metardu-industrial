# Changelog

All notable changes to MetaRDU Industrial will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — Sprint 9: Commercial Module + Field Tools

#### EOM Volumetric Auditor (Commercial Module)
- LAS 1.2/1.3/1.4 reader with transparent LAZ decompression (pure Rust, `laz` crate)
- CSF (Cloth Simulation Filter) ground classification — pure Rust port
- IDW DEM rasterization with rayon parallelism
- Cut/fill volume calculation with per-bench breakdown
- EOM pipeline orchestrator: LAS → CSF → DEM → volumes → signed PDF
- SHA-256 audit hash + chain-of-custody appendix embedded in PDF metadata
- RSA-2048 node-locked license verification (PKCS#1v1.5 + SHA-256)
  - Three tiers: perpetual, per-report, site-based
  - Per-report metering (only signed exports decrement counter)
  - Machine fingerprint (MAC + CPU + disk serial hash)
  - PEM key import/export, tamper-evident signing
- Local report counter sidecar (JSON, platform-aware paths)
- Standalone PDF verifier binary (metardu-verify — free, open-source)

#### DXF Design Surface Import
- DXF TIN surface import via `dxf` crate (3DFACE entities)
- Barycentric interpolation rasterization to regular DEM grid
- Volume calc against design surface (actual vs design = overbreak/underbreak)
- Wired into Volume Calc dialog as 3rd reference option

#### NTRIP/RTCM3 Client
- TCP NTRIP caster connection with HTTP-style auth (base64)
- RTCM v3.x message stream parsing (preamble detection, length extraction)
- Background streaming thread with status reporting
- Config dialog: caster host/port, mountpoint, credentials, live status

#### Mission Data Triage
- EXIF parsing for drone images (kamadak-exif — GPS position + timestamp)
- LAS/LAZ header analysis (bounds, point count, file health)
- RINEX header parsing (approximate position from APPROX POSITION XYZ)
- NMEA log parsing (GGA sentence extraction, trajectory bounds)
- Parallel file analysis via rayon
- CRS mismatch detection, empty file detection, temporal span

#### Watch Folder Zero-Touch Ingest
- Background polling loop detects new .las/.laz files
- Auto-runs EOM pipeline + generates signed PDF next to input
- Tauri events for UI notifications

#### UI Improvements
- Responsive workspace shell (sidebar drawer, icon-only rail, auto-collapse)
- Density setting (compact 12px / comfortable 14px) wired to CSS
- Reduced motion toggle wired to CSS
- Escape-to-close on all 33 dialogs (useEscapeKey hook)
- All 47 sidebar items have onClick handlers (zero dead stubs)
- EOM Auditor dialog with live progress bar, license banner, watch folder section
- Triage dialog with file inventory table, summary tiles, gap warnings
- NTRIP dialog with connection status, message counter, uptime

#### Machine Control File Compiler
- DXF → Leica .svd / Trimble .tp3 / Topcon .top (573-line Rust implementation)
- UI with vendor selector, file picker, compile + result display

### Changed
- Updated ROADMAP.md with Sprint 9 status
- Updated README.md from "Phase 0 in progress" to current feature list
- Gitignore: private key protection, target directories excluded

### Added — Phase 0 Foundation

#### Brand & UX
- Branded splash screen with animated theodolite-lens loading sequence
- Module loading screen showing PROJ/GDAL/PDAL/SpatiaLite init per module
- First-run onboarding: Mining/Marine/Both selector + EPSG picker (10 quick-picks)
- Workspace shell: title bar, left sidebar (project tree), OpenLayers canvas,
  right panel (S-44/survey status), status bar with CRS + UTC clock
- Brand logo SVG component (theodolite, split mining/marine lens, M frame)
- Custom favicon SVG derived from the logo
- File drop overlay covering map canvas — classifies by extension
  (LAS/LAZ, GeoTIFF, .all, .s7k, .bsf, CSV, GPKG, KML)
- CRS auto-switch banner — when dropped GeoTIFF's EPSG differs from
  active map CRS, banner offers Switch / Dismiss
- Settings dialog — change default domain, EPSG, density, reduced motion
  with Tauri IPC persistence (localStorage fallback in browser)

#### Rust Core
- Tauri 2.0 shell with 8-module registry (geodesy, raster, pointcloud,
  spatialite, coord-reg, marine, mining, reporting) — simulated init
  timings for Phase 0
- IPC commands: ping, app_version, init_module, list_modules,
  get_settings, save_settings, probe_file
- Pure-Rust LAS 1.2/1.3/1.4 header parser (~300 lines, no external deps)
  - Reads all 30+ header fields (point count, bounds, scale/offset, PDRF, version)
  - VLR scanning: WKT (LAS 1.4, record 2112), GeoTIFF GeoKeyDirectory
    (LAS 1.2/1.3, record 34735), LasZip VLR for LAZ detection
- Pure-Rust GeoTIFF reader (~425 lines, no external deps)
  - TIFF 6.0 header + IFD walker (little + big endian)
  - Reads dimensions, bits per sample, compression, photometric, strip/tile counts
  - GeoTIFF tags: ModelPixelScale, ModelTiepoint, GeoKeyDirectory, GeoAsciiParams
  - EPSG extraction from GeoKeyDirectory (GeographicTypeGeoKey 2048
    or ProjectedCSTypeGeoKey 3072 with TIFFTagLocation==0)
  - Derives geographic bounds from pixel scale + tiepoint
  - Supports uncompressed (1) and LZW (5); JPEG/DEFLATE not yet supported

#### Frontend
- OpenLayers 10 map canvas with proj4js integration
  - registerEpsg() fetches proj4 definitions from epsg.io, caches in
    localStorage for offline reuse
  - MousePosition control displays in active CRS (monospaced, orange)
  - Graticule, ScaleLine, Zoom, FullScreen controls
  - Auto-fit extent to all features when files change
- Tauri IPC bridge with browser-mode fallbacks for all commands
- Survey store: tracks dropped files with bounds, point count, EPSG,
  dimensions, vendor, status (pending/probing/loaded/error)
- Workspace right panel shows Staged Files list with kind badges,
  file sizes, click-to-focus, remove button

#### Infrastructure
- Tauri 2.0 config (tauri.conf.json) — 1440×900 window, navy background
- App icons generated from logo PNG: 32, 128, 128@2x, icns (macOS),
  ico (Windows multi-res 16-256), 1024 master, Square* Windows Store
- GitHub Actions CI matrix:
  - Frontend typecheck + build (Node 22)
  - Rust fmt + clippy (-D warnings)
  - 4-platform build: linux-x64, windows-x64, macos-arm64, macos-x64
- GitHub Actions release workflow (tag-triggered, draft GitHub Release)
- Issue templates: bug report, feature request, config with contact links
- Pull request template with type/domain/CI checklist
- CONTRIBUTING.md with prereqs, dev setup, code style, branching strategy
- .editorconfig, .gitattributes, .gitignore
- ARCHITECTURE.md (13 sections, 6 appendices, ~7,500 words)

### Tooling
- Rust 1.77+ required
- Node 22+ required
- TypeScript 5+ with `@/` path alias
- Tailwind CSS 4 with @theme tokens extracted from logo
- Vite 5+ with HMR on port 1420

## [0.1.0] — Phase 0 foundation (in development)

Initial scaffold. Not yet released.
