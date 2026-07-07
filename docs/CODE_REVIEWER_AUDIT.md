# Code Reviewer — Code Quality Audit

**Agent**: Code Reviewer (activated from `skills/agency-agents/engineering/engineering-code-reviewer.md`)
**Date**: 2026-07-07
**Scope**: ~30,000 lines Rust + ~25,000 lines TypeScript across 17 sprints

---

## Executive Summary

MetaRDU's codebase is **functional and well-tested** (250+ unit tests) but has **260 `unwrap()` calls** in production Rust code — any of which can panic and crash the app. TypeScript is clean (only 2 `any` types, 0 `@ts-ignore`). The main risks are: panics from `unwrap()`, 3 `unsafe` blocks, and 5 functions over 100 lines that need decomposition.

**Code Quality Score**: 6.8 / 10 — good foundation, needs panic-hardening before v1.0.

---

## Critical Findings

### 🔴 Critical — Fix before v1.0

#### 1. 260 `unwrap()` calls in production Rust code
**Finding**: 260 `.unwrap()` calls outside test code. Each one is a potential panic that crashes the app. The most dangerous are in IPC command handlers — a single malformed input could kill the process.

**Distribution**:
- `src-tauri/src/formats/` — 89 calls (file parsers: LAS, GeoTIFF, .all, .s7k, Shapefile)
- `src-tauri/src/commands/` — 45 calls (IPC handlers)
- `src-tauri/src/marine/` — 28 calls (CUBE, S-44, dredge)
- `src-tauri/src/mining/` — 22 calls (volume, CSF, highwall)
- `src-tauri/src/realtime/` — 18 calls (NMEA, rover, tide)
- Other modules — 58 calls

**Top 5 most dangerous** (in IPC handlers, on user input):
1. `commands/mining.rs` — `read_dem_grid()` unwraps on GeoTIFF parse
2. `commands/marine.rs` — `read_all_survey()` unwraps on .all parse
3. `commands/eom.rs` — `run_eom_pipeline_cmd()` unwraps on LAS read
4. `commands/gis_features.rs` — `read_shapefile_cmd()` unwraps on .shp parse
5. `realtime/rover.rs` — `TcpStream::connect()` unwraps on connection failure

**Fix**: Replace every `unwrap()` with `?` + proper error propagation via `MetarduError` (Sprint 14). For IPC handlers, this means returning `Result<T, MetarduError>` instead of `Result<T, String>`.

**Effort**: ~8 hours (mechanical replacement, 260 calls × ~2 min each).

#### 2. 3 `unsafe` blocks
**Finding**: 3 `unsafe` blocks in the codebase. These bypass Rust's safety guarantees.

**Locations**:
- `src-tauri/src/formats/las.rs` — unsafe transmute for byte array → struct
- `src-tauri/src/formats/geotiff.rs` — unsafe pointer arithmetic for strip reading
- `src-tauri/src/wasm_sandbox.rs` — unsafe for wasmtime FFI

**Fix**: The LAS + GeoTIFF unsafe blocks can be replaced with safe `try_into()` + `from_le_bytes()` patterns (already used elsewhere in the same files). The WASM unsafe is legitimate FFI — document it with a `// SAFETY:` comment.

**Effort**: ~2 hours for LAS + GeoTIFF. WASM is fine as-is.

#### 3. 4 `panic!()` calls
**Finding**: 4 explicit `panic!()` calls in production code.

**Fix**: Replace with `Result<T, MetarduError>` propagation. Panics in a desktop app kill the process — there's no "crash to home page" recovery.

**Effort**: ~1 hour.

### 🟠 Major — Fix in Sprint 19-20

#### 4. 5 functions over 100 lines
**Finding**: 5 functions exceed 100 lines, making them hard to test and maintain.

| Function | File | Lines | Issue |
|---|---|---|---|
| `read_header` | `formats/geotiff.rs` | 267 | IFD parsing should be decomposed into per-tag functions |
| `read_xtf_pings` | `formats/sss_xtf.rs` | 229 | Packet walking should be extracted |
| `read_header` | `formats/reson_s7k.rs` | 174 | Similar to GeoTIFF — decompose per record type |
| `write_shapefile` | `formats/shapefile.rs` | 149 | Separate .shp / .shx / .dbf writers |
| `read_header` | `formats/las.rs` | 120 | Header field extraction should be per-field |

**Fix**: Extract helper functions. Each function should be ≤50 lines (fit on one screen).

**Effort**: ~6 hours across all 5 functions.

#### 5. 173 `clone()` calls — potential performance issue
**Finding**: 173 `.clone()` calls in Rust. While some are necessary (passing ownership to async tasks), many are avoidable with `&` references.

**Top offenders**:
- `commands/mining.rs` — 15 clones (path strings cloned for error messages)
- `marine/cube.rs` — 12 clones (soundings cloned during hypothesis tracking)
- `mining/volume.rs` — 8 clones (bench results cloned)

**Fix**: Audit the top 5 files. Replace `String` clones with `&str` where the original outlives the borrow. Replace `Vec` clones with `&[T]` slices.

**Effort**: ~4 hours for the top 5 files.

#### 6. 12 `console.log/warn/error` in TypeScript
**Finding**: 12 console statements in production TypeScript. These leak to the browser console in dev mode and are invisible in production Tauri builds.

**Locations**: `file-drop-overlay.tsx` (3), `map-canvas.tsx` (3), `live-stream-panel.tsx` (2), others (4).

**Fix**: Replace with the telemetry system (`record_telemetry_event_cmd`) for production visibility, or remove if they were debug-only.

**Effort**: ~1 hour.

### 🟡 Minor — Fix opportunistically

#### 7. Inconsistent error handling in IPC commands
**Finding**: 3 error patterns in use:
- `map_err(|e| format!("...{e}"))` — 45 occurrences
- `ctx!(...)` — 57 occurrences
- `MetarduError` (Sprint 14) — 11 occurrences

**Fix**: Standardize on `MetarduError` (Sprint 14 pattern). The `ctx!()` macro is fine for context, but the return type should be `Result<T, MetarduError>` not `Result<T, String>`.

**Effort**: ~12 hours (155 commands to migrate). Do incrementally.

#### 8. No clippy configuration
**Finding**: The project runs `cargo clippy -D warnings` in CI but has no `clippy.toml` for custom lints.

**Fix**: Add `clippy.toml` with:
- `cognitive-complexity-threshold = 30` (flag complex functions)
- `too-many-arguments-threshold = 8` (flag wide APIs)
- `enum-variant-size-threshold = 256` (flag large enums)

**Effort**: 30 minutes.

---

## TypeScript Quality — GOOD

| Metric | Count | Status |
|---|---|---|
| `any` types | 2 | ✅ Excellent |
| `@ts-ignore` | 0 | ✅ Perfect |
| `@ts-expect-error` | 0 | ✅ Perfect |
| `console.log` | 12 | 🟡 Minor cleanup |
| `TODO/FIXME/HACK` | 0 | ✅ Clean |

TypeScript code quality is excellent. The strict tsconfig + `noUnusedLocals` + `noUnusedParameters` catches issues at compile time.

---

## Rust Quality — NEEDS WORK

| Metric | Count | Status |
|---|---|---|
| `unwrap()` in production | 260 | 🔴 Critical |
| `expect()` | 6 | 🟡 Acceptable if message is good |
| `panic!()` | 4 | 🔴 Critical |
| `unsafe` blocks | 3 | 🟡 2 fixable, 1 legitimate |
| `clone()` | 173 | 🟡 Top 5 files need audit |
| Functions >100 lines | 5 | 🟡 Decompose |
| Unit tests | 250+ | ✅ Good coverage |
| `TODO/FIXME/HACK` | 0 | ✅ Clean |

---

## Recommendation: Sprint 18-19 Hardening Plan

### Sprint 18 (panic hardening)
1. Replace all 260 `unwrap()` with `?` + `MetarduError` — 8 hours
2. Replace 4 `panic!()` with `Result` — 1 hour
3. Fix 2 `unsafe` blocks in LAS + GeoTIFF — 2 hours
4. Add `clippy.toml` with complexity thresholds — 30 min

### Sprint 19 (refactoring)
5. Decompose 5 functions >100 lines — 6 hours
6. Audit top 5 files for unnecessary `clone()` — 4 hours
7. Replace 12 `console.log` with telemetry — 1 hour
8. Begin migrating IPC commands from `String` to `MetarduError` errors — ongoing

**Total**: ~22 hours of focused hardening work to reach production-grade Rust quality.

---

## Bottom Line

The codebase is **well-structured and well-tested** but **panic-prone** in production. The 260 `unwrap()` calls are the #1 risk — a single malformed file could crash the app during a field survey. Sprint 18 should focus exclusively on replacing `unwrap()` with `?` + `MetarduError` propagation. This is the difference between an app that crashes on bad data and one that shows a helpful error message.

TypeScript quality is already excellent — the strict tsconfig + zero `@ts-ignore` + zero `any` (except 2) is rare and commendable.
