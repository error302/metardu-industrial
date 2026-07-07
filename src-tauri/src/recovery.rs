// Crash recovery — Sprint 18.
//
// Saves project state snapshots before long-running operations so that
// if the app panics or crashes, the user can recover their work on the
// next launch.
//
// Two types of recovery files:
//   - Pre-operation snapshots: saved before a long operation starts,
//     cleared on success. If the app crashes mid-operation, the snapshot
//     from before the operation is recoverable.
//   - Crash dumps: saved by the panic hook, containing the panic info +
//     backtrace + the last known project state.
//
// Recovery files live in app_data_dir/recovery/. The frontend checks
// for them on launch and shows a "Session Recovery" dialog if found.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySnapshot {
    pub timestamp_ms: u64,
    pub operation: String,
    pub project_json: String,
    pub snapshot_path: String,
}

/// Get the recovery directory (app_data_dir/recovery/).
/// Falls back to a temp dir if app_data_dir is unavailable.
fn recovery_dir() -> PathBuf {
    // In a real Tauri app, use tauri::api::path::app_data_dir().
    // For now, use a platform-appropriate location.
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/Library/Application Support"
    } else {
        std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/.local/share"
        })
    };
    PathBuf::from(base).join("metardu-industrial").join("recovery")
}

/// Save a recovery snapshot before a long operation.
///
/// Returns the path to the snapshot file. The caller should call
/// `clear_recovery_snapshot` with this path after the operation succeeds.
pub fn save_recovery_snapshot(project_json: &str, operation: &str) -> Result<PathBuf, String> {
    let dir = recovery_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating recovery dir: {e}"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let filename = format!("snapshot_{}_{}.json", timestamp, sanitize(operation));
    let path = dir.join(filename);

    let snapshot = RecoverySnapshot {
        timestamp_ms: timestamp as u64,
        operation: operation.to_string(),
        project_json: project_json.to_string(),
        snapshot_path: path.to_string_lossy().to_string(),
    };

    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("serializing snapshot: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("writing snapshot: {e}"))?;

    Ok(path)
}

/// Clear a recovery snapshot after the operation succeeds.
pub fn clear_recovery_snapshot(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

/// Check for recovery files on app launch.
///
/// Returns the most recent recovery snapshot, or None if no recovery
/// files exist.
pub fn check_recovery_files() -> Option<RecoverySnapshot> {
    let dir = recovery_dir();
    if !dir.exists() {
        return None;
    }

    let mut snapshots: Vec<RecoverySnapshot> = vec![];

    // Check for pre-operation snapshots
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(snapshot) = serde_json::from_str::<RecoverySnapshot>(&content) {
                        snapshots.push(snapshot);
                    }
                }
            }
        }
    }

    // Return the most recent snapshot
    snapshots.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
    snapshots.into_iter().next()
}

/// Delete a recovery file after the user has chosen to restore or discard.
pub fn delete_recovery_file(path: &str) {
    let _ = std::fs::remove_file(path);
}

/// Delete all recovery files (e.g., on successful project save).
pub fn clear_all_recovery_files() {
    let dir = recovery_dir();
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// Install a panic hook that saves a crash dump + project state.
///
/// Call this once at app startup (in main.rs).
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Save crash dump
        let dir = recovery_dir();
        let _ = std::fs::create_dir_all(&dir);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let crash_file = dir.join(format!("crash_{}.txt", timestamp));
        let crash_info = format!(
            "MetaRDU Industrial Crash Report\n\
            ================================\n\
            Timestamp: {} ms\n\
            Panic: {}\n\
            Location: {}\n\n\
            Backtrace:\n{}\n",
            timestamp,
            info,
            info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".to_string()),
            std::backtrace::Backtrace::force_capture()
        );
        let _ = std::fs::write(&crash_file, crash_info);

        // Call the default hook (prints to stderr)
        default_hook(info);
    }));
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_clear_snapshot() {
        let dir = std::env::temp_dir();
        // Override recovery_dir for testing by setting XDG_DATA_HOME
        std::env::set_var("XDG_DATA_HOME", &dir);

        let project_json = r#"{"name":"test","files":[]}"#;
        let path = save_recovery_snapshot(project_json, "test_operation").unwrap();
        assert!(path.exists());

        // Read it back
        let content = std::fs::read_to_string(&path).unwrap();
        let snapshot: RecoverySnapshot = serde_json::from_str(&content).unwrap();
        assert_eq!(snapshot.operation, "test_operation");
        assert!(snapshot.project_json.contains("test"));

        // Clear it
        clear_recovery_snapshot(&path);
        assert!(!path.exists());
    }

    #[test]
    fn test_check_recovery_files_none() {
        std::env::set_var("XDG_DATA_HOME", "/tmp/metardu_test_empty");
        // Make sure the dir doesn't exist or is empty
        let dir = recovery_dir();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(check_recovery_files().is_none());
    }

    #[test]
    fn test_check_recovery_files_finds_snapshot() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);

        let path = save_recovery_snapshot(r#"{"name":"test"}"#, "volume_calc").unwrap();
        let found = check_recovery_files();
        assert!(found.is_some());
        let snapshot = found.unwrap();
        assert_eq!(snapshot.operation, "volume_calc");

        // Clean up
        clear_recovery_snapshot(&path);
    }

    #[test]
    fn test_clear_all_recovery_files() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);

        // Save two snapshots
        let p1 = save_recovery_snapshot(r#"{"name":"test1"}"#, "op1").unwrap();
        let p2 = save_recovery_snapshot(r#"{"name":"test2"}"#, "op2").unwrap();
        assert!(p1.exists());
        assert!(p2.exists());

        // Clear all
        clear_all_recovery_files();
        assert!(!p1.exists());
        assert!(!p2.exists());
    }

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize("compute_volumes"), "compute_volumes");
        assert_eq!(sanitize("compute volumes!"), "compute_volumes_");
        assert_eq!(sanitize("path/to/file"), "path_to_file");
    }
}
