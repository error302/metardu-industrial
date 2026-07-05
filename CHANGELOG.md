# Changelog

All notable changes to MetaRDU Industrial will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — Security & Correctness Hardening

#### Security Fixes
- **License signing upgraded from RSA-PKCS#1v1.5 to RSA-PSS (PS256).**
  PSS is probabilistic and not vulnerable to Bleichenbacher padding-
  oracle attacks. Old RS256 licenses still verify (backward-compat
  path) so existing customers don't get locked out. 4 new security
  tests: PSS round-trip, legacy round-trip, cross-scheme tamper
  rejection, unknown-algorithm rejection.
- **Content Security Policy set in tauri.conf.json** (was `null`).
  `default-src 'self'` with controlled exceptions. No `unsafe-eval`.
- **Plugin loading now requires RSA-PSS signature verification.**
  Every `.so`/`.dll`/`.dylib` must be accompanied by a `.sig` sidecar
  containing a base64-encoded signature over the plugin's SHA-256
  hash. Unsigned plugins are refused. Prevents malicious file drop
  → native code execution on next launch.
- **Arbitrary shell execution removed from pipeline runner.**
  `PipelineAction::ShellCommand` used to run `sh -c <command>` with
  no allowlist. Now refuses with an error pointing to SECURITY.md.
- **License forge oracles removed from IPC.** `generate_license_cmd`
  (HMAC system) and `sign_eom_license_cmd` (RSA system) are no
  longer exposed via `invoke()` — they were signing oracles that
  would let any frontend code mint an Enterprise license. Functions
  kept as library fns for the standalone CLI tools.
- **HTML injection fixed in report engine + deliverable manifest.**
  `provenance_hash`, `src.description`, and `filename` were
  interpolated into HTML without escaping. All user-controlled
  fields now go through `esc()`/`esc_html()`.
- **Telemetry mutex poisoning recovery.** A single panic no longer
  permanently disables all telemetry for the session. Same fix for
  the EOM watcher's `seen` set.
- **SECURITY.md added** — threat model, vulnerability reporting
  policy, known gaps, pre-1.0 hardening checklist.
- **RELEASE.md added** — pre-flight checklist, build/sign/distribute
  steps, emergency hotfix procedure.

#### Correctness Fixes
- **NTRIP RTCM3 CRC-24Q verification implemented.** The parser
  previously trusted every byte from the caster — a single corrupted
  byte would silently produce wrong message types. Now verifies
  CRC-24Q (poly `0x1864CFB`) on every frame; corrupt frames are
  dropped and the parser resyncs on the next 0xD3 preamble.
- **Volume calculator NODATA handling fixed.** Both `compute_volumes`
  copies (core crate + src-tauri local) were silently inflating cut
  volume by ~10⁴ m³ per NODATA pixel. Now skip NODATA cells and
  expose a `nodata_cells` field for QC.
- **EOM audit `processing_time_ms` no longer hardcoded 0.** Stamped
  with real wall-clock time measured around the pipeline call.
- **`FileProbeResult` GeoTIFF kind tag fixed.** Rust serialized as
  `"geotiff"` but TS checked for `"geo-tiff"` — dropped GeoTIFFs got
  no bounds/epsg/dimensions. Fixed with `#[serde(rename = "geo-tiff")]`.
- **`max_points=0` now means "read all" in LAS reader.** Previously
  `header.num_point_records.min(0)` returned 0, silently reading
  zero points and breaking the EOM watch folder, slice editor, and
  CSF classifier. Regression test added.
- **NTRIP `bytes_received` double-counting fixed.** The streaming
  loop added consumed bytes per parsed frame AND raw bytes per socket
  read. Displayed stat was ~2x reality.
- **`NtripStatus` compile error fixed.** The `None` arm of
  `get_ntrip_status_cmd` was missing 3 fields added in commit eaaaecb.
  Hard compile error — CI would have failed.
- **React rules-of-hooks crashes fixed** in cube-disambiguation-dialog
  and triage-dialog (useMemo/useCallback called after early return).
- **SSS waterfall viewer stale closure fixed.** `computeHeight` was
  not in `handleCanvasClick`'s dep array — could compute heights
  against the wrong ping.

#### Performance Fixes
- **Volume calculator parallelized with rayon.** Single-threaded →
  `par_iter().fold().reduce()` across cores. 4-8x speedup on 8-core
  machines. Bench-assignment inner loop changed from O(n_benches)
  linear scan to O(1) index computation.
- **OpenLayers Map no longer rebuilt on domain change.** Toggling
  mining/marine/both used to destroy the entire map (OSM tiles,
  graticule, controls, view state) and rebuild it. Changed dep
  array to `[]`. Saves 200-500ms flicker + tile re-fetch.
- **DEM render effect no longer re-runs on every files change.**
  Adding a CSV while a 25M-cell DEM was rendered re-ran the entire
  Rust render + IPC + canvas rebuild (3-10s freeze). Now depends on
  a derived `loadedGeotiffPath` string — only re-runs when the
  actual GeoTIFF changes.

#### CI Enhancements
- **`core-test` job** runs `cargo test` on `metardu-core` (no system
  deps) on every push — 97 tests in <1 min.
- **`cargo-audit`** runs on every push to catch CVEs in dependencies.
- **`oxlint`** runs before tsc/vite in the frontend-check job.
- **Integration tests** (`crates/metardu-core/tests/integration.rs`)
  run as a separate CI step — guards NODATA + CRC fixes.
- **`rustfmt` violations fixed** across ~30 files so `cargo fmt
  --check` actually passes.

#### Documentation
- **License standardized to MIT** across LICENSE, README, ROADMAP,
  CONTRIBUTING, and all 5 package manifests (package.json + 4
  Cargo.tomls). Previously 4 docs gave 4 different answers.
- **ARCHITECTURE.md §12 repository structure** updated to match the
  actual flat layout (was describing a nonexistent `apps/desktop/`
  + `packages/` + `crates/metardu-{geodesy,formats,pipelines,provenance}`
  monorepo). React version corrected from 18 to 19.
- **README badges** now include a live CI status badge and a MIT
  license badge (was a static "TBD" badge).
- **README Node/Rust version requirements** corrected to match CI
  (Node 22+, Rust 1.87+).

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
