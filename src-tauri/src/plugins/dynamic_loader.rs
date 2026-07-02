// Dynamic plugin loading — Phase 4.
//
// Loads .so/.dll/.dylib plugins at startup that implement the
// FileReaderPlugin, ProcessorPlugin, or ExporterPlugin traits.
//
// Phase 3 provided the static trait interface. Phase 4 adds the
// dynamic loading machinery via `libloading`.
//
// Plugin ABI: plugins are compiled as cdylib crates that export a
// `metardu_plugin_create` function returning a Box<dyn FileReaderPlugin>
// (or ProcessorPlugin / ExporterPlugin). The host app loads the library,
// calls the create function, and registers the plugin.
//
// Safety: plugins run in the same process as MetaRDU Industrial.
// A crashing plugin takes down the app. Phase 5+ should consider
// sandboxing via WASM or separate processes.

use crate::plugins::{FileReaderPlugin, PluginInfo, PluginRegistry};
use libloading::{Library, Symbol};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Type of the plugin creation function exported by each plugin library.
type CreateFileReaderFn = unsafe fn() -> *mut dyn FileReaderPlugin;

/// Load a dynamic plugin from a .so/.dll/.dylib file.
///
/// The plugin must export:
///   - `metardu_plugin_info() -> *const PluginInfo`
///   - `metardu_plugin_create() -> *mut dyn FileReaderPlugin`
///
/// The returned plugin is registered in the global PluginRegistry.
///
/// # Safety
/// Loading and calling into foreign code is inherently unsafe.
/// The plugin's ABI must match exactly (same Rust version, same
/// crate versions for serde/serde_json).
///
/// # Errors
/// Returns an error if:
///   - The file doesn't exist or isn't a valid library
///   - The required symbols aren't found
///   - The plugin's info function returns null
pub fn load_file_reader_plugin(path: &Path) -> Result<PluginInfo, PluginLoadError> {
    unsafe {
        let library = Library::new(path).map_err(|e| PluginLoadError::LoadFailed(e.to_string()))?;

        // Get plugin info
        let info_fn: Symbol<unsafe fn() -> *const PluginInfo> = library
            .get(b"metardu_plugin_info")
            .map_err(|e| PluginLoadError::SymbolMissing(e.to_string()))?;

        let info_ptr = info_fn();
        if info_ptr.is_null() {
            return Err(PluginLoadError::InfoNull);
        }
        let info = (*info_ptr).clone();

        // Get plugin create function
        let create_fn: Symbol<CreateFileReaderFn> = library
            .get(b"metardu_plugin_create")
            .map_err(|e| PluginLoadError::SymbolMissing(e.to_string()))?;

        let plugin_ptr = create_fn();
        if plugin_ptr.is_null() {
            return Err(PluginLoadError::CreateNull);
        }
        let plugin = Box::from_raw(plugin_ptr);

        // Register in global registry
        let registry = crate::plugins::global_registry();
        let mut reg = registry
            .lock()
            .map_err(|e| PluginLoadError::LockFailed(e.to_string()))?;
        reg.register_file_reader(plugin);

        // Keep the library loaded — leak the Library handle so it
        // stays mapped for the lifetime of the process. This is safe
        // because plugins are loaded once at startup and never unloaded.
        std::mem::forget(library);

        Ok(info)
    }
}

/// Scan a directory for plugin files (.so/.dll/.dylib) and load them all.
///
/// Returns a list of (path, result) tuples so the caller can report
/// which plugins loaded successfully and which failed.
pub fn load_plugins_from_dir(dir: &Path) -> Vec<(PathBuf, Result<PluginInfo, PluginLoadError>)> {
    let mut results = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_plugin_file(&path) {
            continue;
        }

        let result = load_file_reader_plugin(&path);
        results.push((path, result));
    }

    results
}

/// Check if a file has a plugin library extension.
fn is_plugin_file(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        path.extension() == Some(OsStr::new("so"))
    }
    #[cfg(target_os = "macos")]
    {
        path.extension() == Some(OsStr::new("dylib"))
    }
    #[cfg(target_os = "windows")]
    {
        path.extension() == Some(OsStr::new("dll"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Get the default plugin directory for this platform.
pub fn default_plugin_dir() -> PathBuf {
    let base = std::env::var("METARDU_PLUGIN_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        format!("{home}/.metardu/plugins")
    });
    PathBuf::from(base)
}

#[derive(Debug, thiserror::Error)]
pub enum PluginLoadError {
    #[error("failed to load library: {0}")]
    LoadFailed(String),
    #[error("required symbol not found: {0}")]
    SymbolMissing(String),
    #[error("plugin info function returned null")]
    InfoNull,
    #[error("plugin create function returned null")]
    CreateNull,
    #[error("registry lock failed: {0}")]
    LockFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_plugin_file() {
        #[cfg(target_os = "linux")]
        {
            assert!(is_plugin_file(Path::new("/tmp/test.so")));
            assert!(!is_plugin_file(Path::new("/tmp/test.txt")));
        }
    }

    #[test]
    fn test_default_plugin_dir() {
        let dir = default_plugin_dir();
        assert!(dir.to_string_lossy().contains("metardu"));
    }

    #[test]
    fn test_load_nonexistent() {
        let result = load_file_reader_plugin(Path::new("/nonexistent/plugin.so"));
        assert!(result.is_err());
    }
}
