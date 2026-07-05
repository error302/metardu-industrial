// Auto-Updater — Sprint 8 Production Distribution.
//
// ⚠️  NOT PRODUCTION-READY — see RELEASE.md §"Auto-updater" for the
// full migration plan. As of this commit, this module is a stub:
//   - `check_for_updates` always returns "up to date" (no real HTTP)
//   - `download_update` returns a temp path without downloading
//   - `install_update` is a no-op
//   - `tauri.conf.json` has no `plugins.updater` config
//   - No signing key is configured
//
// This means **there is currently no way to deliver security patches
// to installed clients**. Any customer who installs this build will
// be stuck on it forever unless they manually re-download.
//
// To make this production-ready:
//   1. Add `tauri-plugin-updater` to Cargo.toml
//   2. Configure `plugins.updater` in tauri.conf.json with:
//      - pubkey (generate with `tauri signer generate`)
//      - endpoints (your update manifest URL)
//   3. Generate a signing keypair and store the private key in CI
//      secrets (NEVER in the repo)
//   4. Wire `check_for_updates_cmd` to `tauri_plugin_updater::Updater`
//   5. Publish a `latest.json` manifest to your endpoint on each
//      release with the signed bundle URL + signature
//   6. Test the full update flow on Windows + macOS + Linux
//
// Until that's done, do NOT ship this to customers in a way that
// prevents manual re-install. The auto-updater UI exists to show
// "no update available" today; calling it an "updater" is generous.
//
// Uses Tauri's built-in updater plugin under the hood (configured in
// tauri.conf.json). This module provides the IPC surface for the
// frontend to trigger checks + display status.
//
// Update flow (when fully wired):
//   1. Frontend calls check_for_updates_cmd on startup + manual trigger
//   2. Backend fetches the update manifest from the endpoint
//   3. If a newer version exists, returns UpdateInfo
//   4. User clicks "Download" → download_update_cmd downloads to temp
//   5. User clicks "Install" → install_update_cmd signals Tauri to
//      apply on next restart
//   6. Frontend shows "Restart to update" prompt
//
// Security: updates are signature-verified by Tauri's updater plugin
// using a public key embedded in the binary. No unsigned updates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// True if an update is available
    pub available: bool,
    /// Latest version available (e.g., "0.2.0")
    pub latest_version: String,
    /// Current installed version
    pub current_version: String,
    /// Release date (ISO 8601)
    pub release_date: String,
    /// Human-readable release notes (markdown)
    pub release_notes: String,
    /// Download URL for the platform-specific binary
    pub download_url: String,
    /// File size in bytes
    pub file_size: u64,
    /// Signature for verification (Tauri updater format)
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    /// Current state of the updater
    pub state: UpdateState,
    /// Last check time (ISO 8601, empty if never checked)
    pub last_check: String,
    /// Update info if available
    pub info: Option<UpdateInfo>,
    /// Download progress (0.0 to 1.0)
    pub download_progress: f64,
    /// Error message if any
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateState {
    /// Haven't checked yet
    Idle,
    /// Currently checking for updates
    Checking,
    /// Update available, waiting for user to download
    Available,
    /// No update available (on latest version)
    UpToDate,
    /// Currently downloading
    Downloading,
    /// Download complete, waiting for user to install
    Downloaded,
    /// Installing (applying the update)
    Installing,
    /// Update installed, restart required
    RestartRequired,
    /// Error occurred
    Error,
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            last_check: String::new(),
            info: None,
            download_progress: 0.0,
            error: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("network error: {0}")]
    Network(String),
    #[error("manifest parse error: {0}")]
    Parse(String),
    #[error("signature verification failed")]
    InvalidSignature,
    #[error("download failed: {0}")]
    Download(String),
    #[error("no update available")]
    NoUpdate,
    #[error("updater not configured (endpoint missing)")]
    NotConfigured,
}

/// Default update endpoint — in production this would be a real URL.
/// For now it's a placeholder so the frontend can detect "not configured".
const DEFAULT_UPDATE_ENDPOINT: &str = "";

/// Check for updates by fetching the manifest from the endpoint.
///
/// In a real implementation this would use an HTTP client (reqwest or
/// Tauri's built-in HTTP). For Sprint 8 we provide the structure and
/// simulate the network call so the frontend can be built and tested.
///
/// Phase 9+ will wire this to Tauri's actual updater plugin.
pub fn check_for_updates(endpoint: &str) -> Result<UpdateInfo, UpdateError> {
    let endpoint = if endpoint.is_empty() {
        DEFAULT_UPDATE_ENDPOINT
    } else {
        endpoint
    };
    if endpoint.is_empty() {
        return Err(UpdateError::NotConfigured);
    }

    // Simulated network call — Phase 9 will use real HTTP
    // For now, return a mock "up to date" response
    Ok(UpdateInfo {
        available: false,
        latest_version: env!("CARGO_PKG_VERSION").into(),
        current_version: env!("CARGO_PKG_VERSION").into(),
        release_date: String::new(),
        release_notes: String::new(),
        download_url: String::new(),
        file_size: 0,
        signature: String::new(),
    })
}

/// Simulate downloading an update. Phase 9 will use Tauri's updater.
pub fn download_update(info: &UpdateInfo) -> Result<String, UpdateError> {
    if !info.available {
        return Err(UpdateError::NoUpdate);
    }
    if info.download_url.is_empty() {
        return Err(UpdateError::Download("no download URL".into()));
    }
    // Phase 9: real download + signature verification
    // For now, return a temp path placeholder
    let temp_path =
        std::env::temp_dir().join(format!("metardu-update-{}.bin", info.latest_version));
    Ok(temp_path.to_string_lossy().to_string())
}

/// Simulate installing the update. Phase 9 will signal Tauri to apply.
pub fn install_update(_info: &UpdateInfo) -> Result<(), UpdateError> {
    // Phase 9: tauri_plugin_updater::Updater::download_and_install()
    Ok(())
}

/// Compare two semantic version strings.
/// Returns true if `latest` is newer than `current`.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.split('-').next()?.parse().ok())
            .collect()
    };
    let cur = parse(current);
    let new = parse(latest);
    for i in 0..cur.len().max(new.len()) {
        let c = cur.get(i).copied().unwrap_or(0);
        let n = new.get(i).copied().unwrap_or(0);
        if n > c {
            return true;
        }
        if n < c {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version_basic() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "1.1.0"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("2.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_with_v_prefix() {
        assert!(is_newer_version("v1.0.0", "v1.0.1"));
        assert!(is_newer_version("1.0.0", "v1.1.0"));
    }

    #[test]
    fn test_is_newer_version_with_suffix() {
        assert!(is_newer_version("1.0.0", "1.0.1-beta"));
        assert!(!is_newer_version("1.0.1", "1.0.0-beta"));
    }

    #[test]
    fn test_check_for_updates_not_configured() {
        let result = check_for_updates("");
        assert!(matches!(result, Err(UpdateError::NotConfigured)));
    }

    #[test]
    fn test_check_for_updates_simulated() {
        // With a non-empty endpoint, returns a simulated "up to date" response
        let info = check_for_updates("https://updates.example.com").unwrap();
        assert!(!info.available);
        assert_eq!(info.current_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_download_update_no_update() {
        let info = UpdateInfo {
            available: false,
            latest_version: "1.0.0".into(),
            current_version: "1.0.0".into(),
            release_date: String::new(),
            release_notes: String::new(),
            download_url: String::new(),
            file_size: 0,
            signature: String::new(),
        };
        let result = download_update(&info);
        assert!(matches!(result, Err(UpdateError::NoUpdate)));
    }

    #[test]
    fn test_download_update_no_url() {
        let info = UpdateInfo {
            available: true,
            latest_version: "2.0.0".into(),
            current_version: "1.0.0".into(),
            release_date: String::new(),
            release_notes: String::new(),
            download_url: String::new(),
            file_size: 0,
            signature: String::new(),
        };
        let result = download_update(&info);
        assert!(matches!(result, Err(UpdateError::Download(_))));
    }

    #[test]
    fn test_update_status_default() {
        let status = UpdateStatus::default();
        assert_eq!(status.state, UpdateState::Idle);
        assert!(status.last_check.is_empty());
        assert!(status.info.is_none());
        assert_eq!(status.download_progress, 0.0);
    }
}
