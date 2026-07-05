# MetaRDU Industrial

> Specialised desktop application for **mining and marine survey workflows, automation, and QA/QC**.
> Built with Tauri 2.0 (Rust core) + React 19 + OpenLayers 10. 100% open-source stack, no subscriptions, no API keys.

[![CI](https://github.com/error302/metardu-industrial/actions/workflows/ci.yml/badge.svg)](https://github.com/error302/metardu-industrial/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Architecture](https://img.shields.io/badge/architecture-v1.0-orange)](./docs/ARCHITECTURE.md)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue)](https://tauri.app)
[![OpenLayers](https://img.shields.io/badge/OpenLayers-10-green)](https://openlayers.org)

---

## What is this?

MetaRDU Industrial is a cross-platform desktop app that automates the repetitive 60–70% of mining and marine survey work — data wrangling, format translation, manual QA/QC. It sits between raw sensor data (UAV imagery, TLS scans, multibeam echosounders, side-scan sonar) and downstream planning/charting systems (Surpac, Datamine, CARIS S-57 Composer).

The full engineering plan lives in [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md). Read that first.

## Status

**Sprint 9 — Commercial Module + Field Tools (complete)**

This repository contains a production-ready surveying application with:

### Mining Module (EOM Volumetric Auditor)
- ✅ LAS 1.2/1.3/1.4 reader with transparent LAZ decompression (pure Rust)
- ✅ CSF (Cloth Simulation Filter) ground classification
- ✅ IDW DEM rasterization with rayon parallelism
- ✅ Cut/fill volume calculation with per-bench breakdown
- ✅ EOM pipeline: LAS → CSF → DEM → volumes → signed PDF (87ms for 2500 points)
- ✅ SHA-256 audit hash + chain-of-custody appendix embedded in PDF
- ✅ DXF TIN design surface import (barycentric interpolation)
- ✅ Watch folder zero-touch ingest (drop file → signed PDF appears)
- ✅ Machine control file compiler (Leica .svd / Trimble .tp3 / Topcon .top)

### License System
- ✅ RSA-2048 node-locked license verification (offline, no phone-home)
- ✅ Three tiers: perpetual, per-report, site-based
- ✅ Per-report metering (only signed exports decrement counter)
- ✅ Standalone PDF verifier (metardu-verify — free, open-source)

### Marine Module
- ✅ CUBE surface generation (Combined Uncertainty and Bathymetry Estimation)
- ✅ IHO S-44 compliance checking (Special Order / 1a / 1b / 2)
- ✅ S-57 chart export
- ✅ TPU (Total Propagated Uncertainty) calculation
- ✅ SVP editor with interactive graph
- ✅ Vessel lever-arm configuration
- ✅ Dredge pay-volume audit
- ✅ Cross-section profiler for channel design
- ✅ Density gates + tidal spline correction
- ✅ SSS waterfall viewer (XTF format)
- ✅ 3D slice editor with reject brush

### Field Tools
- ✅ NTRIP/RTCM3 client (TCP + RTCM v3 parsing — eliminates separate NTRIP app)
- ✅ Mission Data Triage (EXIF + RINEX + NMEA + gap analysis)
- ✅ Command palette (Ctrl+K)

### Infrastructure
- ✅ 119 Tauri IPC commands
- ✅ 33 dialogs (all with Escape-to-close)
- ✅ 80 Rust unit tests
- ✅ Responsive UI (sidebar drawer, density settings, reduced motion)
- ✅ Daylight high-contrast theme for outdoor field use
- ✅ 2 standalone binaries (metardu-eom-cli, metardu-verify)

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Shell | Tauri 2.0 | ~10 MB binary, runs on rugged field laptops, cross-platform |
| Core | Rust | Native perf, GDAL/PDAL/PROJ bindings, no GC pauses |
| Frontend | React 19 + TypeScript + Vite | Ecosystem maturity, type safety |
| Map (2D) | **OpenLayers 10** | OGC services, custom CRS (mine grids!), Canvas 2D fallback, 100% free |
| Map (heavy) | Deck.gl 9 | WebGL acceleration for >1M features, embedded as OL layer |
| 3D viewport | CesiumJS | Pit visualization, 4D progression, marine bathymetry |
| State | Zustand | Lightweight, no boilerplate |
| Styling | Tailwind CSS 4 | Design token enforcement, utility-first |
| Storage | SpatiaLite (local) / PostGIS (networked) | Survey-grade spatial index |
| Interchange | Parquet, GeoTIFF/COG, GPKG, S-57, LAS/LAZ | Open formats only |

**No Mapbox, no subscription services, no API keys anywhere in the stack.**

## Getting Started

### Prerequisites

- **Node.js** 22+ and npm
- **Rust** 1.87+ (install via [rustup](https://rustup.rs))
- **Tauri 2.0 system deps** for your OS — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

#### Linux (Debian/Ubuntu)

```bash
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

#### macOS

```bash
xcode-select --install
```

#### Windows

Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/).

### Install & Run (frontend only)

If you just want to preview the UI without setting up Rust:

```bash
cd metardu-industrial
npm install
npm run dev
```

Open `http://localhost:1420` in your browser. You'll see:
1. **Splash screen** (~2.5s, animated logo + progress)
2. **Module loading screen** (simulated module init)
3. **Onboarding** (pick Mining/Marine/Both + EPSG)
4. **Workspace shell** with OpenLayers canvas

### Install & Run (full Tauri app)

```bash
cd metardu-industrial
npm install
cargo install tauri-cli --version "^2.0"
cargo tauri dev
```

This compiles the Rust core and launches the native window.

### Production build

```bash
cargo tauri build
```

Outputs platform-specific installers in `src-tauri/target/release/bundle/`:
- Windows: `.msi`
- macOS: `.dmg`
- Linux: `.deb`, `.AppImage`

## Project Structure

```
metardu-industrial/
├── docs/
│   └── ARCHITECTURE.md          # Full engineering plan (READ THIS FIRST)
├── src/                         # React frontend
│   ├── components/
│   │   ├── brand-logo.tsx       # SVG logo as React component
│   │   └── map-canvas.tsx       # OpenLayers 10 integration
│   ├── screens/
│   │   ├── splash-screen.tsx
│   │   ├── module-loading-screen.tsx
│   │   ├── onboarding-screen.tsx
│   │   └── workspace-shell.tsx
│   ├── stores/
│   │   └── app-store.ts         # Zustand global state
│   ├── lib/
│   │   └── tokens.ts            # Design tokens (colors from logo)
│   ├── App.tsx                  # Boot sequence orchestrator
│   ├── main.tsx
│   └── index.css                # Tailwind 4 + design tokens
├── src-tauri/                   # Rust core
│   ├── src/
│   │   ├── lib.rs               # Tauri builder + IPC commands
│   │   └── main.rs
│   ├── capabilities/
│   │   └── default.json
│   ├── Cargo.toml
│   ├── build.rs
│   └── tauri.conf.json
├── index.html
├── package.json
├── tsconfig.app.json
├── tsconfig.json
├── tsconfig.node.json
└── vite.config.ts
```

## Design System

Tokens extracted directly from the MetaRDU Industrial logo (see `src/lib/tokens.ts` and `src/index.css`):

| Token | Hex | Use |
|---|---|---|
| `--color-navy-base` | `#0A192F` | Primary background |
| `--color-industrial-orange` | `#FFA500` | Primary accent, CTAs |
| `--color-white` | `#FFFFFF` | Primary text |
| `--color-steel-gray` | `#6B7280` | Secondary text |
| `--color-mining-yellow` | `#FFC107` | Mining mode accent |
| `--color-marine-turquoise` | `#20B2AA` | Marine mode accent |
| `--color-pass` | `#10B981` | S-44 pass, validation OK |
| `--color-fail` | `#EF4444` | S-44 fail, error |

Typography: **Inter** for UI, **JetBrains Mono** for coordinate readouts (monospaced — non-negotiable for surveyors).

## Roadmap

| Phase | Months | Scope |
|---|---|---|
| 0 — Foundation | 1–2 | Tauri shell, OpenLayers, design system, basic ingest |
| 1 — Mining MVP | 3–5 | UAV, classification, volumes, PDF report |
| 2 — Marine MVP | 6–8 | MbES, CUBE, TPU, S-44, S-57 |
| 3 — Automation | 9–11 | Watch folders, YAML pipelines, scheduled jobs |
| 4 — Advanced | 12–15 | 4D monitoring, ML, plugin SDK, distributed |
| 5 — Certify | 16–18 | IHO S-44 prep, perf hardening, docs |

See [`docs/ARCHITECTURE.md` §10](./docs/ARCHITECTURE.md#10-development-roadmap) for the full breakdown.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development setup, how to add
new IPC commands, and how to run the test suite.

## License

**MIT** — see [LICENSE](./LICENSE). The processing engine, frontend, and
standalone binaries (`metardu-eom-cli`, `metardu-verify`) are all MIT.
The EOM Volumetric Auditor's signed-report feature is gated by a license
check (open-core model) but the source code is fully open.

## Credits

Built by [@error302](https://github.com/error302). Architecture informed by 20+ years of geomatics engineering across open-pit mining and hydrographic survey operations.
