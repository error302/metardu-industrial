// Recovery IPC commands — Sprint 18.
//
// Exposes the crash recovery system to the frontend so the app can:
//   1. Save a snapshot before long operations
//   2. Clear the snapshot on success
//   3. Check for recovery files on launch
//   4. Delete recovery files after restore/discard

use crate::recovery::{check_recovery_files, clear_all_recovery_files, clear_recovery_snapshot, delete_recovery_file, save_recovery_snapshot, RecoverySnapshot};

#[tauri::command]
pub fn save_recovery_snapshot_cmd(project_json: String, operation: String) -> Result<String, String> {
    let path = save_recovery_snapshot(&project_json, &operation)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn clear_recovery_snapshot_cmd(path: String) {
    clear_recovery_snapshot(&std::path::PathBuf::from(path));
}

#[tauri::command]
pub fn check_recovery_files_cmd() -> Option<RecoverySnapshot> {
    check_recovery_files()
}

#[tauri::command]
pub fn delete_recovery_file_cmd(path: String) {
    delete_recovery_file(&path);
}

#[tauri::command]
pub fn clear_all_recovery_files_cmd() {
    clear_all_recovery_files();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_check_recovery() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);
        clear_all_recovery_files_cmd();

        let path = save_recovery_snapshot_cmd(r#"{"name":"test"}"#.to_string(), "test_op".to_string()).unwrap();
        assert!(check_recovery_files_cmd().is_some());

        delete_recovery_file_cmd(path);
        // May still find other recovery files, but this one is gone
    }
}
