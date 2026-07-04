# Changelog

All notable changes to MetaRDU Industrial will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
