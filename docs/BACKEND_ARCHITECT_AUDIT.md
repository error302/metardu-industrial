# Backend Architect — IPC + Rust Architecture Audit

**Agent**: Backend Architect (activated from `skills/agency-agents/engineering/engineering-backend-architect.md`)
**Method**: Architecture review against scalability, reliability, and observability criteria
**Date**: 2026-07-07
**Scope**: MetaRDU Industrial — Tauri IPC layer + Rust backend (src-tauri/)

---

## Methodology

I reviewed the IPC layer (155 commands across 13 modules) and the Rust backend (~25,000 lines) against the Backend Architect's core criteria:

1. **API Contract Governance** — versioning, error standardization, idempotency
2. **Data/Schema Engineering** — type safety, serialization, validation
3. **System Reliability** — error handling, timeouts, circuit breakers
4. **Performance** — async usage, blocking, memory
5. **Observability** — logging, metrics, tracing
6. **Security** — input validation, path traversal, injection

---

## Architecture Overview

```
Frontend (React/TypeScript)
    ↕ Tauri IPC (155 commands, JSON + ArrayBuffer)
Backend (Rust)
├── commands/     13 modules — IPC handlers (thin wrappers)
├── formats/      LAS, GeoTIFF, Kongsberg .all, Reson .s7k, SSS XTF
├── mining/       CSF, volume, change detection, highwall, survey tools
├── marine/       CUBE, S-44, S-57, dredge, cross-section, tide, backscatter
├── realtime/     NMEA parser, rover TCP client, NOAA tide fetcher
├── qc/           UncertainValue, verify_calculation, range checks
├── cogo.rs       Coordinate geometry
├── contours.rs   Marching squares
└── ...           (20+ other modules)
```

**Verdict**: The architecture is a **well-organized modular monolith** — appropriate for a desktop app with a single user. The command modules are thin wrappers over domain modules, which is correct. The main gaps are in **API contract governance** (no versioning, inconsistent error responses) and **observability** (no structured logging, no metrics).

---

## Top 10 Findings (Ranked by Severity)

### 🔴 Critical — Must fix before production

#### 1. No IPC API versioning
**Finding**: All 155 IPC commands are unversioned. If a command's signature changes (e.g., `compute_volumes_cmd` adds a new parameter), old frontends break silently. There's no `v1::` / `v2::` namespace, no deprecation window.

**Risk**: A user on app v0.1.0 with a cached frontend chunk calling a v0.2.0 backend gets a cryptic "parameter missing" error with no recovery path.

**Recommendation**: Add a `ipc_version: u32` field to the Tauri state. Frontend checks it on startup and warns if mismatched. Long-term, namespace commands as `v1::compute_volumes_cmd`.

#### 2. Inconsistent error responses
**Finding**: 45 commands use `map_err(|e| format!("...{e}"))`, 57 use `ctx!(...)`, and the rest return raw strings. Errors are always `Result<T, String>` — no error codes, no structured fields, no way for the frontend to distinguish "file not found" from "parse error" from "permission denied".

**Risk**: The frontend can't show appropriate recovery actions. A "file not found" should offer a Browse button; a "permission denied" should suggest checking file locks; a "parse error" should show the line number.

**Recommendation**: Define a `MetarduError` enum with variants:
```rust
#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum MetarduError {
    FileNotFound { path: String },
    ParseError { format: String, line: Option<u32>, detail: String },
    PermissionDenied { path: String },
    InvalidInput { field: String, value: String, reason: String },
    CalculationError { step: String, detail: String },
    IoError { detail: String },
}
```
Frontend pattern-matches on `kind` to show the right UI.

#### 3. No timeout on long-running commands
**Finding**: 24 commands use `spawn_blocking` for CPU-bound work. None have a timeout. A 10GB LAS file with a bug in the parser could hang the thread pool indefinitely, blocking all other IPC commands.

**Risk**: One bad file freezes the entire app. The user has to kill the process.

**Recommendation**: Wrap each `spawn_blocking` in `tokio::time::timeout(Duration::from_secs(300), ...)` (5 minutes). Return a `MetarduError::Timeout` variant. Make the timeout configurable per command type.

### 🟠 Major — Should fix before scaling

#### 4. No structured logging
**Finding**: The backend uses `eprintln!` in 3 places and `log::warn!` in 2. There's no structured logging, no log levels, no correlation IDs. When a user reports "the volume calc was wrong", there's no way to trace what happened.

**Recommendation**: Add `tracing` crate with structured spans. Every IPC command gets a span with `command_name`, `user_id` (future), `correlation_id`. Emit to a rotating log file in `app_data_dir/logs/`. The frontend can attach the correlation ID to error reports.

#### 5. No metrics / telemetry on backend performance
**Finding**: The `benchmarks.rs` module exists but only runs on-demand. There's no continuous measurement of command latency, error rates, or memory usage.

**Recommendation**: Add a `metrics` module that records per-command: invocation count, p50/p95/p99 latency, error count, last error. Expose via `get_backend_metrics_cmd` IPC. Show in the existing Telemetry dialog.

#### 6. `spawn_blocking` thread pool not bounded
**Finding**: 24 commands spawn blocking tasks. Tauri's default tokio runtime has a 512-thread limit for blocking tasks. On a 16-core machine, 16 concurrent file parses could exhaust the pool.

**Risk**: Under heavy load (watch folder processing multiple files), the app becomes unresponsive.

**Recommendation**: Use a dedicated `Runtime` with a bounded blocking pool (`max_blocking_threads(8)`). Queue file-processing tasks instead of spawning them all at once.

### 🟡 Moderate — Improve for maintainability

#### 7. No contract tests for IPC
**Finding**: The 155 IPC commands have unit tests on the underlying Rust functions, but no integration tests that verify the IPC serialization round-trips correctly. A `serde` attribute typo (e.g., `#[serde(rename = "snake_case")]` missing) would break the frontend silently.

**Recommendation**: Add `tests/ipc_contract.rs` that invokes each command via `tauri::test::mock_app()` and verifies the JSON shape matches what the frontend expects. Run in CI.

#### 8. Path validation is inconsistent
**Finding**: `path_validation.rs` exists (Sprint 9 security fix) but only 18 of 155 commands that take a path use it. The other 137 commands accept arbitrary paths — including `~/.ssh`, `~/.aws`, and browser directories that were explicitly denylisted.

**Risk**: A malicious LAS file path could read sensitive files. (Low risk for a desktop app, but still a gap.)

**Recommendation**: Audit all 155 commands. Any that take a `path: String` parameter must call `validate_path()`. Add a clippy lint or macro to enforce this.

#### 9. No idempotency for side-effecting commands
**Finding**: Commands like `save_project_cmd`, `install_plugin_cmd`, `generate_report_cmd` have side effects (write files, download plugins). If the frontend retries (e.g., user double-clicks), the command runs twice — potentially overwriting a file or charging a license twice.

**Recommendation**: Add an optional `idempotency_key: Option<String>` parameter to side-effecting commands. The backend deduplicates within a 60-second window. Return the cached result for duplicate keys.

#### 10. No graceful shutdown
**Finding**: When the user closes the app, in-flight IPC commands are killed mid-execution. A CUBE surface generation at 90% completion is lost. The rover TCP stream is dropped without a clean disconnect.

**Recommendation**: Register a `tauri::RunEvent::ExitRequested` handler that:
1. Stops accepting new IPC commands
2. Waits up to 10 seconds for in-flight commands to complete
3. Saves the current project state
4. Cleanly disconnects the rover TCP stream
5. Then exits

---

## Architecture Strengths (What's Good)

| Area | Assessment |
|---|---|
| **Module separation** | ✅ Excellent — `commands/` are thin wrappers, domain logic in `mining/`, `marine/`, etc. |
| **Pure-Rust parsers** | ✅ Excellent — no external C dependencies, no FFI safety issues |
| **Async/sync split** | ✅ Good — CPU-bound work in `spawn_blocking`, I/O in async |
| **Type safety** | ✅ Good — `serde` with `rename_all` for snake_case consistency |
| **Error context** | ✅ Good — `ctx!()` macro adds path + operation context |
| **Path validation** | ✅ Good (where used) — denylists sensitive directories |
| **License security** | ✅ Excellent — RSA-PSS signing, per-feature gating |
| **Test coverage** | ✅ Good — ~115 unit tests, focused on calculation correctness |

---

## IPC Command Audit — By Module

| Module | Commands | Async | Blocking | Error Handling | Path Validation |
|---|---|---|---|---|---|
| `marine` | 11 | 7 | 4 | `ctx!()` | ✅ 7/11 |
| `mining` | 9 | 3 | 2 | `ctx!()` | ✅ 5/9 |
| `qc` | 14 | 0 | 0 | `format!()` | N/A |
| `cogo` | 11 | 0 | 0 | `Option<T>` | N/A |
| `contours` | 2 | 0 | 0 | `format!()` | N/A |
| `realtime` | 8 | 2 | 1 | `format!()` | ✅ 2/8 |
| `eom` | 8 | 4 | 2 | `ctx!()` | ✅ 6/8 |
| `sprint6-8` | 18 | 2 | 0 | Mixed | Partial |
| `pipelines` | 3 | 1 | 1 | `ctx!()` | ✅ 3/3 |
| `monitoring` | 3 | 0 | 0 | `format!()` | N/A |
| `ml` | 2 | 0 | 0 | `format!()` | N/A |
| `streaming` | 2 | 2 | 0 | `format!()` | N/A |
| `automation` | 2 | 1 | 0 | `format!()` | N/A |
| Other | ~30 | Mixed | Mixed | Mixed | Mixed |

**Summary**: 155 commands total. 50 async, 24 blocking, 87 sync. Path validation only on ~40% of path-taking commands.

---

## Recommendation: Sprint 13-14 Backend Priorities

### Sprint 13 (current — backend prerequisites for UX fixes)
1. **`MetarduError` enum** — structured error responses (#2). 1 day.
2. **Progress callbacks** — `tauri::ipc::Channel` for streaming progress from long-running commands (#3 from UX audit). 2 days.
3. **Path validation audit** — wire `validate_path()` into all 137 unvalidated commands (#8). 1 day.
4. **Timeouts on blocking commands** — 5-minute default, configurable (#3). 0.5 days.

### Sprint 14 (reliability + observability)
5. **`tracing` structured logging** — rotating log files, correlation IDs (#4). 2 days.
6. **Backend metrics** — per-command latency, error rates, expose via IPC (#5). 2 days.
7. **Contract tests** — `tests/ipc_contract.rs` for all 155 commands (#7). 3 days.
8. **Graceful shutdown** — exit handler + in-flight wait (#10). 1 day.
9. **IPC versioning** — `ipc_version` field + frontend mismatch warning (#1). 0.5 days.
10. **Idempotency keys** — for side-effecting commands (#9). 2 days.

**Total**: ~15 days of backend work to reach production-grade reliability.

---

## Bottom Line

MetaRDU's backend is **well-architected for a desktop app** — modular, type-safe, with good separation of concerns. The gaps are in **operational maturity**: no structured errors, no progress callbacks, no logging, no timeouts. These are the difference between a tool that works in testing and one that survives a year of field use.

The UX Researcher audit (companion document) identified that the top 3 user-facing issues (file paths, progress bars, result persistence) all require backend work to fix properly. Sprint 13 should address both the UX surface and the backend prerequisites in parallel.
