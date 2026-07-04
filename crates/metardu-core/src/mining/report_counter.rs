// Local JSON sidecar tracking report consumption per license.
//
// When the application generates a PDF report, this counter is incremented
// so that a license cannot be reused to generate more reports than its
// quota allows. The sidecar lives at:
//
//     {app_data_dir}/metardu/report_counter.json
//
// where `app_data_dir` is provided by the application shell (typically
// `dirs::data_dir()` on desktop platforms).
//
// Layout:
//
// ```json
// {
//   "counts": {
//     "trial": 3,
//     "license-001": 12
//   },
//   "updated_at": 1700000000
// }
// ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Hard-coded default trial quota — used by the application shell when no
/// `LicenseClaims::reports_remaining` is set. The actual enforcement is
/// done by the caller (which compares `ReportCounter::consumed_for` to
/// `TRIAL_REPORT_QUOTA`).
pub const TRIAL_REPORT_QUOTA: u32 = 5;

/// Special license-id key used when no license file is present (trial mode).
pub const TRIAL_KEY: &str = "trial";

/// On-disk JSON sidecar tracking per-license report consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCounter {
    /// Path of the sidecar file (not serialised).
    #[serde(skip)]
    pub path: PathBuf,
    /// Map of `license_id` → number of reports generated.
    pub counts: BTreeMap<String, u32>,
    /// Unix timestamp of the last update.
    #[serde(default)]
    pub updated_at: u64,
}

impl ReportCounter {
    /// Compute the sidecar path for a given app-data directory.
    pub fn path_for(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join("metardu").join("report_counter.json")
    }

    /// Load the counter from `{app_data_dir}/metardu/report_counter.json`.
    /// If the file does not exist, returns an empty counter (no error).
    pub fn load(app_data_dir: &Path) -> Result<Self, ReportCounterError> {
        let path = Self::path_for(app_data_dir);
        Self::load_from(&path)
    }

    /// Load the counter from an explicit path. Missing file → empty counter.
    pub fn load_from(path: &Path) -> Result<Self, ReportCounterError> {
        if !path.exists() {
            return Ok(Self {
                path: path.to_path_buf(),
                counts: BTreeMap::new(),
                updated_at: 0,
            });
        }
        let bytes = std::fs::read(path)?;
        let (counts, _stored_updated_at) = if bytes.is_empty() {
            (BTreeMap::new(), 0u64)
        } else {
            // Tolerate either the full `{ counts, updated_at }` envelope or
            // a bare `{ "trial": 3, ... }` map (older format).
            #[derive(Deserialize)]
            struct Envelope {
                counts: BTreeMap<String, u32>,
                #[serde(default)]
                updated_at: u64,
            }
            match serde_json::from_slice::<Envelope>(&bytes) {
                Ok(env) => (env.counts, env.updated_at),
                Err(_) => (
                    serde_json::from_slice::<BTreeMap<String, u32>>(&bytes).unwrap_or_default(),
                    0,
                ),
            }
        };
        Ok(Self {
            path: path.to_path_buf(),
            counts,
            updated_at: current_unix_seconds(),
        })
    }

    /// Save the counter to its sidecar path, creating parent directories
    /// as needed.
    pub fn save(&self) -> Result<(), ReportCounterError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    /// Increment the consumed-report count for `license_id` and return the
    /// new total.
    pub fn increment(&mut self, license_id: &str) -> u32 {
        let entry = self.counts.entry(license_id.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        self.updated_at = current_unix_seconds();
        *entry
    }

    /// Number of reports already consumed for `license_id` (0 if never
    /// recorded).
    pub fn consumed_for(&self, license_id: &str) -> u32 {
        self.counts.get(license_id).copied().unwrap_or(0)
    }

    /// Reports remaining = max(0, `license_reports` - `consumed_for`).
    ///
    /// If `license_reports` is `None` (unlimited license) returns `u32::MAX`.
    pub fn remaining(&self, license_id: &str, license_reports: u32) -> u32 {
        license_reports.saturating_sub(self.consumed_for(license_id))
    }

    /// Reset the counter for `license_id` (used by the application shell
    /// when a new license file replaces an exhausted one).
    pub fn reset(&mut self, license_id: &str) {
        self.counts.remove(license_id);
        self.updated_at = current_unix_seconds();
    }
}

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, thiserror::Error)]
pub enum ReportCounterError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_file_returns_empty_counter() {
        let tmp = tempfile::tempdir().unwrap();
        let counter = ReportCounter::load(tmp.path()).unwrap();
        assert!(counter.counts.is_empty());
        assert_eq!(counter.consumed_for("anything"), 0);
    }

    #[test]
    fn test_increment_save_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut counter = ReportCounter::load(tmp.path()).unwrap();
        assert_eq!(counter.increment("license-001"), 1);
        assert_eq!(counter.increment("license-001"), 2);
        assert_eq!(counter.increment("license-002"), 1);
        counter.save().unwrap();

        let reloaded = ReportCounter::load(tmp.path()).unwrap();
        assert_eq!(reloaded.consumed_for("license-001"), 2);
        assert_eq!(reloaded.consumed_for("license-002"), 1);
    }

    #[test]
    fn test_remaining_computes_quota_minus_consumed() {
        let tmp = tempfile::tempdir().unwrap();
        let mut counter = ReportCounter::load(tmp.path()).unwrap();
        counter.increment("L1");
        counter.increment("L1");
        assert_eq!(counter.remaining("L1", 10), 8);
        assert_eq!(counter.remaining("L1", 1), 0);
    }

    #[test]
    fn test_trial_key_uses_constant() {
        let tmp = tempfile::tempdir().unwrap();
        let mut counter = ReportCounter::load(tmp.path()).unwrap();
        counter.increment(TRIAL_KEY);
        assert_eq!(counter.consumed_for(TRIAL_KEY), 1);
        assert_eq!(counter.remaining(TRIAL_KEY, TRIAL_REPORT_QUOTA), 4);
    }

    #[test]
    fn test_reset_clears_license_count() {
        let tmp = tempfile::tempdir().unwrap();
        let mut counter = ReportCounter::load(tmp.path()).unwrap();
        counter.increment("L1");
        counter.increment("L1");
        counter.reset("L1");
        assert_eq!(counter.consumed_for("L1"), 0);
    }

    #[test]
    fn test_path_for_appends_metardu_subdir() {
        let p = ReportCounter::path_for(Path::new("/var/lib/app"));
        assert!(p.ends_with("metardu/report_counter.json"));
    }
}
