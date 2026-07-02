// Watch folder manager — polls directories for new survey files and
// triggers pipelines when files appear.
//
// Per ARCHITECTURE.md §4.2 — "Watch folder → ingest → classify →
// volume calc → email PDF report. The classic mine surveyor workflow,
// fully automated."
//
// Phase 3 uses polling (every 5 seconds) rather than inotify/FSEvents
// to avoid platform-specific dependencies. Phase 4+ can switch to the
// `notify` crate for instant notifications.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchFolder {
    pub id: String,
    pub path: String,
    pub pipeline_name: String,
    /// File extensions to watch (e.g., ["las", "tif", "all"])
    pub extensions: Vec<String>,
    /// Whether this watcher is active
    #[serde(default = "default_true")]
    pub active: bool,
    /// Seconds between polling checks
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

fn default_true() -> bool {
    true
}
fn default_poll_interval() -> u64 {
    5
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchFolderStatus {
    pub id: String,
    pub path: String,
    pub pipeline_name: String,
    pub active: bool,
    pub files_detected: usize,
    pub pipelines_triggered: usize,
    pub last_check: Option<String>,
    pub last_file: Option<String>,
    pub pending_files: Vec<String>,
}

/// Global watch folder state — tracks seen files to avoid reprocessing.
pub struct WatchState {
    pub folders: Vec<WatchFolder>,
    pub seen_files: HashMap<String, SystemTime>,
    pub stats: HashMap<String, WatchStats>,
}

#[derive(Debug, Clone, Default)]
pub struct WatchStats {
    pub files_detected: usize,
    pub pipelines_triggered: usize,
    pub last_file: Option<String>,
}

impl WatchState {
    pub fn new() -> Self {
        Self {
            folders: Vec::new(),
            seen_files: HashMap::new(),
            stats: HashMap::new(),
        }
    }

    pub fn add_folder(&mut self, folder: WatchFolder) {
        self.stats.insert(folder.id.clone(), WatchStats::default());
        self.folders.push(folder);
    }

    pub fn remove_folder(&mut self, id: &str) {
        self.folders.retain(|f| f.id != id);
        self.stats.remove(id);
    }

    /// Scan all active watch folders for new files. Returns a list of
    /// (folder_id, pipeline_name, file_path) tuples for files that
    /// haven't been seen before.
    pub fn scan(&mut self) -> Vec<(String, String, String)> {
        let mut triggers = Vec::new();
        let now = SystemTime::now();

        for folder in &self.folders {
            if !folder.active {
                continue;
            }

            let path = PathBuf::from(&folder.path);
            if !path.is_dir() {
                continue;
            }

            let entries = match std::fs::read_dir(&path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_file() {
                    continue;
                }

                // Check extension
                let ext = entry_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                if !folder.extensions.iter().any(|e| *e == ext) {
                    continue;
                }

                let file_key = entry_path.display().to_string();

                // Check if we've seen this file before
                if self.seen_files.contains_key(&file_key) {
                    continue;
                }

                // Check file is not still being written (modification time
                // should be at least 2 seconds ago)
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if now
                            .duration_since(modified)
                            .unwrap_or(Duration::from_secs(0))
                            .as_secs()
                            < 2
                        {
                            continue; // File is too fresh — might still be copying
                        }
                    }
                }

                // New file detected!
                self.seen_files.insert(file_key.clone(), SystemTime::now());

                if let Some(stats) = self.stats.get_mut(&folder.id) {
                    stats.files_detected += 1;
                    stats.last_file = Some(entry_path.display().to_string());
                }

                triggers.push((
                    folder.id.clone(),
                    folder.pipeline_name.clone(),
                    entry_path.display().to_string(),
                ));
            }
        }

        // Update stats for triggered pipelines
        for (folder_id, _, _) in &triggers {
            if let Some(stats) = self.stats.get_mut(folder_id) {
                stats.pipelines_triggered += 1;
            }
        }

        triggers
    }

    pub fn get_status(&self) -> Vec<WatchFolderStatus> {
        self.folders
            .iter()
            .map(|f| {
                let stats = self.stats.get(&f.id);
                WatchFolderStatus {
                    id: f.id.clone(),
                    path: f.path.clone(),
                    pipeline_name: f.pipeline_name.clone(),
                    active: f.active,
                    files_detected: stats.map(|s| s.files_detected).unwrap_or(0),
                    pipelines_triggered: stats.map(|s| s.pipelines_triggered).unwrap_or(0),
                    last_check: None, // Set by the polling loop
                    last_file: stats.and_then(|s| s.last_file.clone()),
                    pending_files: Vec::new(),
                }
            })
            .collect()
    }
}

impl Default for WatchState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global watch state — accessible from IPC commands.
pub fn global_watch_state() -> &'static Mutex<WatchState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<WatchState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(WatchState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_folder() {
        let mut state = WatchState::new();
        state.add_folder(WatchFolder {
            id: "wf1".into(),
            path: "/tmp/test".into(),
            pipeline_name: "test".into(),
            extensions: vec!["las".into()],
            active: true,
            poll_interval_secs: 5,
        });
        assert_eq!(state.folders.len(), 1);
        state.remove_folder("wf1");
        assert_eq!(state.folders.len(), 0);
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let mut state = WatchState::new();
        state.add_folder(WatchFolder {
            id: "wf1".into(),
            path: "/nonexistent/path".into(),
            pipeline_name: "test".into(),
            extensions: vec!["las".into()],
            active: true,
            poll_interval_secs: 5,
        });
        let triggers = state.scan();
        assert!(triggers.is_empty());
    }
}
