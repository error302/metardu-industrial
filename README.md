# MetaRDU Industrial

> Specialised desktop application for **mining and marine survey workflows, automation, and QA/QC**.
> Built with Tauri 2.0 (Rust core) + React 19 + OpenLayers 10. 100% open-source stack, no subscriptions, no API keys.

[![Architecture](https://img.shields.io/badge/architecture-v1.0-orange)](./docs/ARCHITECTURE.md)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue)](https://tauri.app)
[![OpenLayers](https://img.shields.io/badge/OpenLayers-10-green)](https://openlayers.org)
[![License](https://img.shields.io/badge/license-TBD-lightgrey)](#license)

---

## What is this?

MetaRDU Industrial is a cross-platform desktop app that automates the repetitive 60–70% of mining and marine survey work — data wrangling, format translation, manual QA/QC. It sits between raw sensor data (UAV imagery, TLS scans, multibeam echosounders, side-scan sonar) and downstream planning/charting systems (Surpac, Datamine, CARIS S-57 Composer).

The full engineering plan lives in [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md). Read that first.

## Status

**Phase 0 — Foundation (in progress)**

This repository currently contains:
- ✅ Tauri 2.0 shell (`src-tauri/`)
- ✅ React 19 + TypeScript + Vite frontend (`src/`)
- ✅ OpenLayers 10 map canvas with custom CRS scaffolding, graticule, monospaced coordinate readout, scale bar
- ✅ Branded splash screen with animated theodolite-lens loading sequence
- ✅ Module loading screen (PROJ / GDAL / PDAL / SpatiaLite init display)
- ✅ First-run onboarding (Mining / Marine / Both selector + EPSG picker)
- ✅ Workspace shell (sidebar, map canvas, right panel, status bar)
- ✅ Design system: tokens extracted from the logo (navy base, industrial orange, mining yellow, marine turquoise)
- ✅ Brand logo as React SVG component (`src/components/brand-logo.tsx`)

Coming next:
- ⏳ Rust core: PROJ integration, GDAL bindings, real module loading
- ⏳ File ingest (LAS/LAZ, GeoTIFF, Kongsberg `.all`, Reson `.s7k`)
- ⏳ Phase 1: Mining MVP — drone → point cloud → volume report
- ⏳ Phase 2: Marine MVP — MbES → CUBE → S-44 → S-57

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

- **Node.js** 20+ and npm
- **Rust** 1.77+ (install via [rustup](https://rustup.rs))
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

TBD — contributing guidelines will be added once Phase 0 is complete. For now, open an issue or PR against the `main` branch.

## License

TBD — likely **open core** (processing engine open-source under MIT/Apache-2.0, UI and pro plugins commercial). Final decision before Phase 1.

## Credits

Built by [@error302](https://github.com/error302). Architecture informed by 20+ years of geomatics engineering across open-pit mining and hydrographic survey operations.
