// Auto-Updater — Production Implementation
//
// Uses tauri-plugin-updater to check for, download, and install
// signed updates. Updates are signature-verified against the public
// key configured in tauri.conf.json → plugins.updater.pubkey.
//
// Setup (see RELEASE.md for full instructions):
//   1. Generate a signing keypair:
//        npx @tauri-apps/cli signer generate -w ~/.tauri/metardu.key
//   2. Put the public key in tauri.conf.json → plugins.updater.pubkey
//   3. Put the private key in CI secrets as TAURI_SIGNING_PRIVATE_KEY
//   4. Configure endpoints in tauri.conf.json → plugins.updater.endpoints
//      (e.g. ["https://github.com/error302/metardu-industrial/releases/latest/download/latest.json"])
//   5. On each release, the CI workflow signs the bundle and publishes
//      the latest.json manifest to the endpoint.
//
// Security: the updater plugin verifies the Ed25519 signature of the
// downloaded bundle against the pubkey before installing. No unsigned
// updates can be installed. The private key never leaves CI.
//
// Update flow:
//   1. Frontend calls check_for_updates_cmd on startup + manual trigger
//   2. Backend calls tauri_plugin_updater::Updater::check()
//   3. If a newer version exists, returns UpdateInfo with available=true
//   4. User clicks "Download & Install" → download_and_install_update_cmd
//   5. Plugin downloads the bundle, verifies signature, installs
//   6. Frontend shows "Restart to update" prompt
//
// If the updater is not configured (empty pubkey/endpoints), all
// commands return UpdateError::NotConfigured so the frontend can show
// "auto-update not available" instead of crashing.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

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
    /// File size in bytes (0 if unknown)
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
    /// No check has been performed yet
    Idle,
    /// Currently checking for updates
    Checking,
    /// An update is available for download
    Available,
    /// Currently downloading the update
    Downloading,
    /// Currently installing the update
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
    #[error("updater not configured (endpoint or pubkey missing in tauri.conf.json)")]
    NotConfigured,
    #[error("updater plugin error: {0}")]
    Plugin(String),
}

/// Check for updates using the tauri-plugin-updater.
///
/// Returns `UpdateInfo` with `available=true` if a newer version exists,
/// `available=false` if up to date. Returns `UpdateError::NotConfigured`
/// if the updater plugin has no pubkey or endpoints configured.
pub async fn check_for_updates(app: &AppHandle) -> Result<UpdateInfo, UpdateError> {
    let updater = app
        .updater()
        .map_err(|e| UpdateError::Plugin(format!("failed to get updater: {e}")))?;

    // Check if the updater is configured. The plugin returns an error
    // if pubkey or endpoints are missing — we translate that to
    // NotConfigured so the frontend can show a helpful message.
    let update = match updater.check().await {
        Ok(opt) => opt,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("pubkey") || msg.contains("endpoint") || msg.contains("url") {
                return Err(UpdateError::NotConfigured);
            }
            return Err(UpdateError::Network(msg));
        }
    };

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    match update {
        Some(update) => {
            // An update is available
            let latest_version = update.version.clone();
            let release_date = update
                .date
                .map(|d| d.to_string())
                .unwrap_or_default();
            let release_notes = update.body.clone().unwrap_or_default();

            // The download URL and signature are internal to the plugin;
            // we expose them as empty strings since the frontend doesn't
            // need them (the plugin handles download + verify internally).
            Ok(UpdateInfo {
                available: true,
                latest_version,
                current_version,
                release_date,
                release_notes,
                download_url: String::new(),
                file_size: 0,
                signature: String::new(),
            })
        }
        None => {
            // Up to date
            Ok(UpdateInfo {
                available: false,
                latest_version: current_version.clone(),
                current_version,
                release_date: String::new(),
                release_notes: String::new(),
                download_url: String::new(),
                file_size: 0,
                signature: String::new(),
            })
        }
    }
}

/// Download and install the update. The plugin handles signature
/// verification internally — if the signature doesn't match the
/// configured pubkey, the install is aborted.
///
/// This is a blocking call that downloads the full bundle. The
/// frontend should show a progress spinner while this runs.
pub async fn download_and_install_update(app: &AppHandle) -> Result<(), UpdateError> {
    let updater = app
        .updater()
        .map_err(|e| UpdateError::Plugin(format!("failed to get updater: {e}")))?;

    let update = updater
        .check()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?
        .ok_or(UpdateError::NoUpdate)?;

    // Download and install. The API takes two closures:
    //   1. FnMut(usize, Option<u64>) — progress: (downloaded_bytes, total_bytes)
    //   2. FnOnce() — called when download completes, before install
    // The plugin verifies the Ed25519 signature before installing.
    update
        .download_and_install(
            |_downloaded, _total| {
                // Progress: downloaded bytes, total bytes (None = unknown)
                // We could emit a Tauri event here for a progress bar.
            },
            || {
                // Download complete — install is about to start
            },
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("signature") {
                UpdateError::InvalidSignature
            } else {
                UpdateError::Download(msg)
            }
        })?;

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
    let lat = parse(latest);
    for i in 0..cur.len().max(lat.len()) {
        let c = cur.get(i).copied().unwrap_or(0);
        let l = lat.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.0", "0.1.1"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("0.2.0", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(is_newer_version("v0.1.0", "v0.2.0"));
    }

    #[test]
    fn test_update_status_default() {
        let status = UpdateStatus::default();
        assert_eq!(status.state, UpdateState::Idle);
        assert_eq!(status.download_progress, 0.0);
        assert!(status.info.is_none());
        assert!(status.error.is_none());
    }
}
