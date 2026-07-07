# SRE — Crash Recovery Design

**Agent**: SRE (activated from `skills/agency-agents/engineering/engineering-sre.md`)
**Date**: 2026-07-07
**Scope**: Reliability of MetaRDU Industrial desktop app in field conditions

---

## Executive Summary

MetaRDU Industrial has **no crash recovery**. If the app panics during a 30-minute CUBE surface generation or a 10-minute EOM audit, all work is lost. There's no auto-save before long operations, no panic recovery hook, and no crash report submission. For a field surveyor on a vessel or in a mine, a crash during a survey means re-running the entire operation.

This document designs a crash recovery system with:
1. **Auto-save** — save project state before every long-running operation
2. **Panic recovery** — catch Rust panics, save state, show recovery dialog
3. **Crash reporting** — submit crash dumps to the telemetry backend
4. **Session restore** — on next launch, offer to restore the previous session

---

## Current State

| Capability | Status |
|---|---|
| Auto-save before long ops | 🔴 None |
| Panic hook (catch Rust panics) | 🔴 None |
| Crash dump capture | 🟡 Telemetry module exists but only captures JS errors |
| Crash report submission | 🟡 `record_crash_cmd` exists but not wired to panic hook |
| Session restore on launch | 🔴 None |
| Auto-save project file | 🟡 `.metardu` save exists but manual only |
| Tauri `ExitRequested` handler | 🔴 None |

---

## Design: Crash Recovery System

### 1. Auto-Save Before Long Operations

**Principle**: Before any operation that takes >10 seconds, save a snapshot of the project state to a temp file. If the app crashes, the snapshot is recoverable.

**Operations that need auto-save**:
- `compute_volumes_cmd` (30-60 seconds)
- `compute_volumes_verified_cmd` (30-60 seconds)
- `compute_stockpile_change_cmd` (30-120 seconds)
- `classify_ground` (CSF — 30-120 seconds)
- `run_eom_pipeline_cmd` (10-30 seconds)
- `run_odm_pipeline` (5-30 minutes)
- `generate_cube_surface_cmd` (30-90 seconds)
- `generate_map_layout_cmd` (5-15 seconds)

**Implementation** (Rust):
```rust
/// Save a crash-recovery snapshot before a long operation.
/// Stored in app_data_dir/recovery/snapshot_<timestamp>.metardu
pub fn save_recovery_snapshot(project: &MetarduProject, operation: &str) -> PathBuf {
    let recovery_dir = app_data_dir().join("recovery");
    std::fs::create_dir_all(&recovery_dir).ok();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let path = recovery_dir.join(format!("snapshot_{}_{}.metardu", timestamp, operation));
    let json = serde_json::to_string(project).unwrap_or_default();
    std::fs::write(&path, json).ok();
    path
}

/// Clean up the snapshot after the operation succeeds.
pub fn clear_recovery_snapshot(path: &Path) {
    std::fs::remove_file(path).ok();
}
```

**IPC command**:
```rust
#[tauri::command]
pub fn save_recovery_snapshot_cmd(project: MetarduProject, operation: String) -> Result<String, String> {
    let path = save_recovery_snapshot(&project, &operation);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn clear_recovery_snapshot_cmd(path: String) {
    clear_recovery_snapshot(&PathBuf::from(path));
}
```

### 2. Panic Recovery Hook

**Principle**: Install a Rust panic hook that saves the project state + writes a crash dump before the process dies.

**Implementation** (Rust, in `main.rs`):
```rust
use std::panic;
use std::sync::{Arc, Mutex};

// Global project state for panic recovery
static RECOVERY_PROJECT: OnceLock<Mutex<Option<MetarduProject>>> = OnceLock::new();

fn install_panic_hook() {
    let recovery_project = RECOVERY_PROJECT.get_or_init(|| Mutex::new(None));
    panic::set_hook(Box::new(move |info| {
        // Save the project state
        if let Ok(guard) = recovery_project.lock() {
            if let Some(project) = guard.as_ref() {
                let recovery_dir = app_data_dir().join("recovery");
                std::fs::create_dir_all(&recovery_dir).ok();
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let path = recovery_dir.join(format!("crash_{}.metardu", timestamp));
                let json = serde_json::to_string(project).unwrap_or_default();
                std::fs::write(&path, json).ok();
            }
        }

        // Write crash dump
        let crash_dir = app_data_dir().join("crashes");
        std::fs::create_dir_all(&crash_dir).ok();
        let crash_file = crash_dir.join(format!("crash_{}.txt", timestamp));
        std::fs::write(&crash_file, format!("Panic: {}\n\nBacktrace:\n{}", info, std::backtrace::Backtrace::force_capture())).ok();

        // Call the original hook
        // (The default hook prints to stderr)
    }));
}
```

**Update global state** — before every long operation, update the recovery project:
```rust
// In each long-running IPC command:
pub async fn compute_volumes_cmd(request: ComputeVolumesRequest) -> Result<VolumeResult, String> {
    // Save recovery snapshot
    save_recovery_snapshot(&current_project(), "compute_volumes");

    // Update global state for panic hook
    if let Some(lock) = RECOVERY_PROJECT.get() {
        if let Ok(mut guard) = lock.lock() {
            *guard = Some(current_project());
        }
    }

    // ... do the work ...

    // Clear recovery snapshot on success
    clear_recovery_snapshot(&snapshot_path);
    Ok(result)
}
```

### 3. Session Restore on Launch

**Principle**: On app launch, check for crash recovery files. If found, show a "Session Recovery" dialog offering to restore.

**Implementation** (Frontend, in `App.tsx` or `workspace-shell.tsx`):
```tsx
function useSessionRecovery() {
  const [recoveryFile, setRecoveryFile] = useState<string | null>(null);

  useEffect(() => {
    // Check for crash recovery files
    invoke<string | null>("check_recovery_files_cmd").then(path => {
      if (path) setRecoveryFile(path);
    });
  }, []);

  async function restore() {
    if (!recoveryFile) return;
    const project = await invoke<MetarduProject>("load_project_cmd", { path: recoveryFile });
    // Load the project...
    // Clear the recovery file
    await invoke("delete_recovery_file_cmd", { path: recoveryFile });
    setRecoveryFile(null);
  }

  return { recoveryFile, restore, dismiss: () => setRecoveryFile(null) };
}
```

**Recovery dialog**:
- Title: "Session Recovery"
- Message: "MetaRDU was unexpectedly closed during: [operation name]. A recovery snapshot from [timestamp] is available."
- Buttons: "Restore Session" / "Discard"

### 4. Tauri ExitRequested Handler

**Principle**: When the user closes the app, wait for in-flight operations to complete (up to 10 seconds) before exiting.

**Implementation** (Rust, in `lib.rs`):
```rust
tauri::Builder::default()
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            // Check if any long operations are running
            if is_operation_running() {
                // Show "waiting for operations to complete" dialog
                // Wait up to 10 seconds
                // Then force-close
            }
        }
    })
```

---

## Reliability Runbook

### Scenario 1: App crashes during volume calculation
1. Panic hook fires → saves `crash_<timestamp>.metardu` to `app_data_dir/recovery/`
2. Panic hook writes crash dump to `app_data_dir/crashes/crash_<timestamp>.txt`
3. Process exits
4. User relaunches the app
5. Session recovery detects `crash_<timestamp>.metardu`
6. Recovery dialog: "Restore session from [timestamp]?"
7. User clicks "Restore" → project loads → user re-runs the volume calc

### Scenario 2: App crashes during ODM pipeline (5-30 minutes)
1. Same as Scenario 1, but the ODM Docker container may still be running
2. Recovery dialog shows: "ODM pipeline was in progress. The Docker container may still be running. Check `docker ps` and stop if needed."
3. User can re-run the ODM pipeline without re-uploading photos (ODM caches intermediate results)

### Scenario 3: Power loss during survey
1. No panic hook fires (power is off)
2. But the auto-save snapshot from before the operation exists
3. On next launch, recovery dialog offers the pre-operation snapshot
4. User loses the in-progress operation but not the project state

### Scenario 4: Disk full during auto-save
1. `save_recovery_snapshot()` fails silently (returns path but file not written)
2. If the app then crashes, there's no recovery file
3. User loses the session — but this is the same as today (no recovery at all)
4. **Mitigation**: Check disk space before long operations; warn the user

---

## Implementation Plan

### Sprint 18 (core recovery)
1. `recovery.rs` module — save/clear/load recovery snapshots — 2 hours
2. Panic hook installation in `main.rs` — 2 hours
3. `check_recovery_files_cmd` + `delete_recovery_file_cmd` IPC — 1 hour
4. Session recovery dialog (frontend) — 2 hours
5. Wire auto-save into 3 most-critical commands (volume, CSF, EOM) — 1 hour

### Sprint 19 (full coverage)
6. Wire auto-save into remaining 5 long-running commands — 2 hours
7. Tauri `CloseRequested` handler — 2 hours
8. Crash dump capture (backtrace + system info) — 1 hour
9. Crash report submission via telemetry — 1 hour
10. Disk space check before long operations — 30 min

**Total**: ~15 hours for a complete crash recovery system.

---

## SLOs (Service Level Objectives)

For a desktop app, SLOs are about data loss, not uptime:

| SLO | Target | How |
|---|---|---|
| No data loss on crash | 99% of crashes recoverable | Auto-save before long ops |
| Recovery time | <30 seconds from relaunch | Fast file load + dialog |
| Crash report submission | 95% of crashes reported | Telemetry + crash dump |
| Max work lost | <5 minutes of operation | Auto-save timestamp tracks start |

---

## Bottom Line

MetaRDU has **zero crash recovery today** — a panic during a survey operation loses everything. The 15-hour implementation plan above provides:
1. Auto-save before long operations (prevents data loss)
2. Panic hook (catches crashes, saves state)
3. Session restore (recovers on next launch)
4. Crash reporting (informs future fixes)

This is the difference between a tool that's usable in testing and one that survives a year of field use. For mining + marine surveyors who may be 2 hours from an office, losing a 30-minute calculation is unacceptable.
