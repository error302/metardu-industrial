// Plugin Marketplace — Sprint 8 Production Distribution.
//
// Discover, download, verify, and install third-party plugins from a
// plugin registry. Extends the platform so vendors can ship format
// readers + processors + exporters without recompiling the main app.
//
// Architecture:
//   - Registry is a JSON file (hosted at a URL or local path) listing
//     available plugins with metadata + download URLs + signatures
//   - Plugins are .so/.dll/.dylib files signed with the vendor's key
//   - The marketplace verifies the signature before installing
//   - Installed plugins live in the app's plugins/ directory
//   - On startup, the dynamic loader scans plugins/ and registers them
//
// Security model:
//   - Each plugin entry in the registry includes a SHA-256 hash + a
//     signature from the registry operator (not the vendor)
//   - The app has the registry operator's public key embedded
//   - Downloads are verified against the hash before installation
//   - Users must explicitly approve each install (no auto-install)
//
// Phase 8: registry JSON format + install/verify logic
// Phase 9: real HTTP downloads + actual plugin loading

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    /// Registry format version
    pub version: u32,
    /// Registry name (e.g., "MetaRDU Official Plugin Registry")
    pub name: String,
    /// ISO 8601 timestamp when the registry was last updated
    pub updated: String,
    /// List of available plugins
    pub plugins: Vec<RegistryPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPlugin {
    /// Unique plugin ID (e.g., "norbit-wbm-reader")
    pub id: String,
    /// Display name (e.g., "Norbit WBM Reader")
    pub name: String,
    /// Version (semver, e.g., "0.1.0")
    pub version: String,
    /// Vendor / author
    pub vendor: String,
    /// Description
    pub description: String,
    /// Plugin type: "file_reader" | "processor" | "exporter"
    pub plugin_type: String,
    /// Supported file extensions (for file readers)
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Download URL for the platform-specific binary
    pub download_url: String,
    /// SHA-256 hash of the binary (hex-encoded)
    pub sha256: String,
    /// File size in bytes
    pub file_size: u64,
    /// Minimum MetaRDU version required
    pub min_app_version: String,
    /// License (e.g., "MIT", "Proprietary")
    pub license: String,
    /// Homepage URL
    #[serde(default)]
    pub homepage: String,
    /// True if this is an official MetaRDU plugin
    #[serde(default)]
    pub official: bool,
    /// Download count (for popularity sorting)
    #[serde(default)]
    pub downloads: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub vendor: String,
    pub installed_path: String,
    pub installed_date: String,
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum MarketplaceError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("plugin not found in registry: {0}")]
    NotFound(String),
    #[error("plugin already installed: {0}")]
    AlreadyInstalled(String),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("download failed: {0}")]
    Download(String),
    #[error("registry not configured")]
    NotConfigured,
}

/// Fetch the plugin registry from a URL or local path.
///
/// Phase 9 will use real HTTP. For now, accepts a local file path
/// for testing.
pub fn fetch_registry(source: &str) -> Result<PluginRegistry, MarketplaceError> {
    if source.is_empty() {
        return Err(MarketplaceError::NotConfigured);
    }

    // Try as a local file first (for testing)
    let path = Path::new(source);
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        let registry: PluginRegistry = serde_json::from_str(&content)?;
        return Ok(registry);
    }

    // Phase 9: real HTTP fetch
    // For now, return an empty registry if the source is a URL
    Ok(PluginRegistry {
        version: 1,
        name: "Local Registry".into(),
        updated: String::new(),
        plugins: Vec::new(),
    })
}

/// Get the plugins directory (creates it if it doesn't exist).
pub fn get_plugins_dir(app_data_dir: &Path) -> PathBuf {
    let dir = app_data_dir.join("plugins");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// List installed plugins by scanning the plugins directory.
///
/// Looks for .json metadata sidecar files (one per installed plugin).
/// Each sidecar contains the InstalledPlugin metadata. If a .so/.dll/.dylib
/// binary exists without a sidecar, derives metadata from the filename.
pub fn list_installed_plugins(app_data_dir: &Path) -> Result<Vec<InstalledPlugin>, MarketplaceError> {
    let plugins_dir = get_plugins_dir(app_data_dir);
    let mut installed = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    if !plugins_dir.exists() {
        return Ok(installed);
    }

    // First pass: scan for .json sidecar files (preferred path)
    for entry in std::fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.to_lowercase() == "json" {
                if let Ok(metadata_content) = std::fs::read_to_string(&path) {
                    if let Ok(meta) = serde_json::from_str::<InstalledPlugin>(&metadata_content) {
                        if seen_ids.insert(meta.id.clone()) {
                            installed.push(meta);
                        }
                    }
                }
            }
        }
    }

    // Second pass: scan for .so/.dll/.dylib files without sidecars
    for entry in std::fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if ext_lower == "so" || ext_lower == "dll" || ext_lower == "dylib" {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                if seen_ids.insert(filename.to_string()) {
                    installed.push(InstalledPlugin {
                        id: filename.into(),
                        name: filename.into(),
                        version: "unknown".into(),
                        vendor: "unknown".into(),
                        installed_path: path.to_string_lossy().to_string(),
                        installed_date: String::new(),
                    });
                }
            }
        }
    }

    Ok(installed)
}

/// Install a plugin from the registry.
///
/// Phase 9 will download + verify. For now, creates a placeholder.
pub fn install_plugin(
    registry: &PluginRegistry,
    plugin_id: &str,
    app_data_dir: &Path,
) -> Result<InstalledPlugin, MarketplaceError> {
    // Find the plugin in the registry
    let plugin = registry.plugins.iter()
        .find(|p| p.id == plugin_id)
        .ok_or_else(|| MarketplaceError::NotFound(plugin_id.into()))?;

    let plugins_dir = get_plugins_dir(app_data_dir);

    // Check if already installed
    let installed = list_installed_plugins(app_data_dir)?;
    if installed.iter().any(|p| p.id == plugin.id) {
        return Err(MarketplaceError::AlreadyInstalled(plugin.id.clone()));
    }

    // Phase 9: download from plugin.download_url + verify SHA-256
    // For now, create a metadata sidecar file
    let installed_plugin = InstalledPlugin {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        vendor: plugin.vendor.clone(),
        installed_path: plugins_dir.join(format!("{}.json", plugin.id)).to_string_lossy().to_string(),
        installed_date: now_iso(),
    };

    // Write the metadata sidecar
    let metadata_path = plugins_dir.join(format!("{}.json", plugin.id));
    let metadata_json = serde_json::to_string_pretty(&installed_plugin)?;
    std::fs::write(&metadata_path, metadata_json)?;

    Ok(installed_plugin)
}

/// Uninstall a plugin by ID.
pub fn uninstall_plugin(plugin_id: &str, app_data_dir: &Path) -> Result<(), MarketplaceError> {
    let plugins_dir = get_plugins_dir(app_data_dir);

    // Remove the .so/.dll/.dylib + .json sidecar
    for ext in &["so", "dll", "dylib", "json"] {
        let path = plugins_dir.join(format!("{}.{}", plugin_id, ext));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
    }

    Ok(())
}

/// Check if a plugin is installed.
#[allow(dead_code)]
pub fn is_plugin_installed(plugin_id: &str, app_data_dir: &Path) -> bool {
    if let Ok(installed) = list_installed_plugins(app_data_dir) {
        return installed.iter().any(|p| p.id == plugin_id);
    }
    false
}

/// Search the registry for plugins matching a query.
pub fn search_registry<'a>(registry: &'a PluginRegistry, query: &str) -> Vec<&'a RegistryPlugin> {
    if query.is_empty() {
        return registry.plugins.iter().collect();
    }
    let q = query.to_lowercase();
    registry.plugins.iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&q)
                || p.description.to_lowercase().contains(&q)
                || p.vendor.to_lowercase().contains(&q)
                || p.id.to_lowercase().contains(&q)
        })
        .collect()
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = ((day_of_year / 30) as u8).min(11) + 1;
    let day = ((day_of_year % 30) as u8) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_registry() -> PluginRegistry {
        PluginRegistry {
            version: 1,
            name: "Test Registry".into(),
            updated: "2026-07-03".into(),
            plugins: vec![
                RegistryPlugin {
                    id: "norbit-wbm-reader".into(),
                    name: "Norbit WBM Reader".into(),
                    version: "0.1.0".into(),
                    vendor: "Norbit Subsea".into(),
                    description: "Reads Norbit .wbm multibeam data files".into(),
                    plugin_type: "file_reader".into(),
                    extensions: vec!["wbm".into()],
                    download_url: "https://example.com/norbit-wbm-0.1.0.so".into(),
                    sha256: "abc123".into(),
                    file_size: 1024000,
                    min_app_version: "0.1.0".into(),
                    license: "Proprietary".into(),
                    homepage: "https://norbit.example".into(),
                    official: false,
                    downloads: 150,
                },
                RegistryPlugin {
                    id: "kongsberg-all-extension".into(),
                    name: "Kongsberg .all Extension".into(),
                    version: "0.2.0".into(),
                    vendor: "Kongsberg Maritime".into(),
                    description: "Extended .all datagram support".into(),
                    plugin_type: "file_reader".into(),
                    extensions: vec!["all".into()],
                    download_url: "https://example.com/km-all-0.2.0.so".into(),
                    sha256: "def456".into(),
                    file_size: 2048000,
                    min_app_version: "0.1.0".into(),
                    license: "MIT".into(),
                    homepage: "https://kongsberg.example".into(),
                    official: true,
                    downloads: 500,
                },
            ],
        }
    }

    #[test]
    fn test_fetch_registry_from_file() {
        let tmp = std::env::temp_dir().join("metardu_test_registry.json");
        let registry = make_test_registry();
        std::fs::write(&tmp, serde_json::to_string_pretty(&registry).unwrap()).unwrap();

        let fetched = fetch_registry(tmp.to_str().unwrap()).unwrap();
        assert_eq!(fetched.name, "Test Registry");
        assert_eq!(fetched.plugins.len(), 2);
        assert_eq!(fetched.plugins[0].id, "norbit-wbm-reader");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_fetch_registry_not_configured() {
        let result = fetch_registry("");
        assert!(matches!(result, Err(MarketplaceError::NotConfigured)));
    }

    #[test]
    fn test_search_registry_by_name() {
        let registry = make_test_registry();
        let results = search_registry(&registry, "norbit");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "norbit-wbm-reader");
    }

    #[test]
    fn test_search_registry_by_vendor() {
        let registry = make_test_registry();
        let results = search_registry(&registry, "kongsberg");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "kongsberg-all-extension");
    }

    #[test]
    fn test_search_registry_empty_query_returns_all() {
        let registry = make_test_registry();
        let results = search_registry(&registry, "");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_install_and_uninstall_plugin() {
        let tmp_dir = std::env::temp_dir().join("metardu_plugins_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let registry = make_test_registry();
        let installed = install_plugin(&registry, "norbit-wbm-reader", &tmp_dir).unwrap();
        assert_eq!(installed.id, "norbit-wbm-reader");
        assert_eq!(installed.name, "Norbit WBM Reader");

        // Verify it shows up in list_installed_plugins
        let list = list_installed_plugins(&tmp_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "norbit-wbm-reader");

        // Uninstall
        uninstall_plugin("norbit-wbm-reader", &tmp_dir).unwrap();
        let list = list_installed_plugins(&tmp_dir).unwrap();
        assert_eq!(list.len(), 0);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_install_already_installed() {
        let tmp_dir = std::env::temp_dir().join("metardu_plugins_test2");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let registry = make_test_registry();
        install_plugin(&registry, "norbit-wbm-reader", &tmp_dir).unwrap();
        let result = install_plugin(&registry, "norbit-wbm-reader", &tmp_dir);
        assert!(matches!(result, Err(MarketplaceError::AlreadyInstalled(_))));

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_install_not_found() {
        let tmp_dir = std::env::temp_dir().join("metardu_plugins_test3");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let registry = make_test_registry();
        let result = install_plugin(&registry, "nonexistent-plugin", &tmp_dir);
        assert!(matches!(result, Err(MarketplaceError::NotFound(_))));

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_is_plugin_installed() {
        let tmp_dir = std::env::temp_dir().join("metardu_plugins_test4");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let registry = make_test_registry();
        assert!(!is_plugin_installed("norbit-wbm-reader", &tmp_dir));
        install_plugin(&registry, "norbit-wbm-reader", &tmp_dir).unwrap();
        assert!(is_plugin_installed("norbit-wbm-reader", &tmp_dir));

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
