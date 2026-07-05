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
//
// ⚠️ SECURITY: As of this commit, plugins MUST be accompanied by a
// `.sig` sidecar file containing the SHA-256 hash of the plugin
// binary, signed with the bundled RSA-PSS public key. If the sidecar
// is missing or the signature doesn't verify, the plugin is refused.
// This prevents a malicious file dropped into the plugins folder
// from executing native code on next launch. See SECURITY.md.

use crate::plugins::{FileReaderPlugin, PluginInfo};
use libloading::{Library, Symbol};
use sha2::{Digest, Sha256};
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
/// **Security:** The plugin binary must be accompanied by a `.sig`
/// sidecar file containing a base64-encoded RSA-PSS signature over
/// the SHA-256 hash of the plugin binary. The signature is verified
/// against the bundled public key before the library is loaded. If
/// the sidecar is missing or the signature doesn't verify, the load
/// is refused with `PluginLoadError::SignatureMissing` or
/// `PluginLoadError::SignatureInvalid`.
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
///   - The `.sig` sidecar is missing or invalid
///   - The required symbols aren't found
///   - The plugin's info function returns null
pub fn load_file_reader_plugin(path: &Path) -> Result<PluginInfo, PluginLoadError> {
    // Security gate: verify the plugin's signature BEFORE loading
    // the library. A missing or invalid signature refuses the load.
    verify_plugin_signature(path)?;

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

/// Verify the plugin's RSA-PSS signature sidecar.
///
/// The sidecar file is `<plugin_path>.sig` and contains a base64-encoded
/// RSA-PSS signature over the SHA-256 hash of the plugin binary. The
/// signature is verified against the bundled public key
/// (`src/keys/license_pub.pem` — same key used for license verification).
///
/// This prevents a malicious `.so`/`.dll`/`.dylib` dropped into the
/// plugins folder from executing on next launch — only plugins signed
/// by the issuing authority will load.
fn verify_plugin_signature(plugin_path: &Path) -> Result<(), PluginLoadError> {
    let sig_path = plugin_path.with_extension(
        plugin_path
            .extension()
            .map(|e| {
                let mut s = e.to_str().unwrap_or("").to_string();
                s.push_str(".sig");
                s
            })
            .unwrap_or_else(|| "sig".to_string()),
    );

    // Read the signature sidecar
    let sig_b64 = std::fs::read_to_string(&sig_path).map_err(|_| {
        PluginLoadError::SignatureMissing(format!(
            "plugin signature sidecar not found: {}. Plugins must be signed by the issuing authority — see SECURITY.md.",
            sig_path.display()
        ))
    })?;

    // Decode the base64 signature
    use base64::Engine;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(sig_b64.trim().as_bytes())
        .map_err(|e| PluginLoadError::SignatureInvalid(format!("base64 decode failed: {e}")))?;

    // Hash the plugin binary
    let plugin_bytes = std::fs::read(plugin_path)
        .map_err(|e| PluginLoadError::LoadFailed(format!("failed to read plugin binary: {e}")))?;
    let mut hasher = Sha256::new();
    hasher.update(&plugin_bytes);
    let plugin_hash = hasher.finalize();

    // Verify the signature against the bundled public key.
    // We reuse the license verification key — same RSA-PSS scheme,
    // same bundled PEM. If the license key is rotated, plugins must
    // be re-signed too.
    let pub_key_pem = include_str!("../keys/license_pub.pem");
    let pub_key =
        metardu_core::mining::license::import_public_key_pem(pub_key_pem).map_err(|e| {
            PluginLoadError::SignatureInvalid(format!("bundled pubkey parse error: {e}"))
        })?;

    // The signature is over the plugin hash (32 bytes), not the full
    // binary — this matches how license claims are signed (over the
    // canonical JSON bytes). Smaller payload = faster verify.
    use rsa::signature::Verifier;
    let verifying_key = rsa::pss::VerifyingKey::<Sha256>::new(pub_key);
    let sig = rsa::pss::Signature::try_from(signature.as_slice())
        .map_err(|_| PluginLoadError::SignatureInvalid("signature has wrong length".into()))?;
    verifying_key.verify(&plugin_hash, &sig).map_err(|_| {
        PluginLoadError::SignatureInvalid(
            "RSA-PSS signature does not match the plugin binary. \
                 The plugin may have been tampered with or was not signed \
                 by the issuing authority."
                .into(),
        )
    })
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
    #[error("plugin signature missing: {0}")]
    SignatureMissing(String),
    #[error("plugin signature invalid: {0}")]
    SignatureInvalid(String),
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
