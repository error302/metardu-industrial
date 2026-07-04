# Contributing to MetaRDU Industrial

Thanks for considering a contribution to MetaRDU Industrial. This document covers the basics; for the full architecture and design rationale, read [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) first.

## Project Status

We are in **Phase 0 — Foundation**. The codebase has:

- ✅ Tauri 2.0 shell + React 19 + Vite frontend
- ✅ OpenLayers 10 map canvas with proj4js CRS switching
- ✅ Pure-Rust LAS 1.2/1.3/1.4 header parser
- ✅ Pure-Rust GeoTIFF reader (uncompressed + LZW, GeoKey directory, EPSG extraction)
- ✅ Module loading + settings persistence via IPC
- ✅ CI matrix across linux/windows/macos (arm64 + x64)

Phase 1 (Mining MVP) and Phase 2 (Marine MVP) work has not started. See [the roadmap](./docs/ARCHITECTURE.md#10-development-roadmap) for what's planned.

## Prerequisites

- **Node.js** 22+
- **Rust** 1.77+ ([rustup](https://rustup.rs))
- **Tauri 2.0 system deps** — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development Setup

```bash
git clone https://github.com/error302/metardu-industrial.git
cd metardu-industrial
npm install
npm run dev          # frontend only — http://localhost:1420
```

For the full native app:

```bash
cargo install tauri-cli --version "^2.0"
cargo tauri dev      # compiles Rust core + launches native window
```

## Code Style

### Rust

- Format with `cargo fmt` (enforced in CI).
- Lint with `cargo clippy --all-targets -- -D warnings` (enforced in CI).
- Use `thiserror` for error enums.
- Public functions and types must have doc comments (`///`).
- Async commands must not hold `MutexGuard` across `.await` — acquire, copy needed data, drop, then await.

### TypeScript / React

- Format with the project's Prettier config (if present) or follow existing style.
- All UI components use the design tokens in `src/lib/tokens.ts` — do not hardcode colors.
- Use the `@/` path alias for imports from `src/`.
- Components go in `src/components/`, screens in `src/screens/`, stores in `src/stores/`, libraries in `src/lib/`.
- Type-check passes are required: `npm run build` runs `tsc -b && vite build`.

### Design System

The MetaRDU Industrial logo is the source of truth for visual identity. Tokens extracted from it live in `src/lib/tokens.ts` and `src/index.css`. Key rules:

- **Navy base** (`#0A192F`) is the background. Always.
- **Industrial orange** (`#FFA500`) is the primary accent for CTAs and active state.
- **Mining yellow** (`#FFC107`) and **marine turquoise** (`#20B2AA`) are domain accents — only use them for domain-specific UI.
- Coordinates and numeric readouts use **JetBrains Mono** (monospaced). Non-negotiable.
- 8px base grid for all spacing.

## Branching Strategy

- `main` — stable, deployable. CI must pass.
- `develop` — integration branch for the next release.
- Feature branches: `feat/<short-name>`, `fix/<short-name>`, `docs/<short-name>`.
- Open a PR into `main` (or `develop` once it exists). Squash-merge on approval.

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(<scope>): <description>

[optional body]

[optional footer]
```

Common scopes: `icons`, `ci`, `core`, `ingest`, `crs`, `rust`, `deps`, `ui`.

Examples from this repo's history:

- `feat(ingest+crs): pure-Rust LAS header parser, proj4js integration`
- `fix(rust): clippy lints — unused imports, dead code, large enum variant`
- `ci(release): restrict trigger to tags only`

## Testing

Phase 0 has no formal test suite yet. When adding tests:

- Rust unit tests live inline (`#[cfg(test)] mod tests { ... }`).
- Frontend tests will use Vitest + React Testing Library (TBD).
- Manual smoke-test the full boot sequence (splash → modules → onboarding → workspace) before opening a PR.

## Opening Issues

Use the issue templates in `.github/ISSUE_TEMPLATE/`. If none fits, open a blank issue with:

1. What you expected
2. What happened
3. Steps to reproduce
4. OS + Tauri version + commit SHA

## License

TBD — likely open core (engine MIT/Apache-2.0, UI + pro plugins commercial). Don't add LICENSE files without coordinating with @error302.
