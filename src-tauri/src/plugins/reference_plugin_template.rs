// Reference Plugin — vendor-style FileReaderPlugin example.
//
// This file is shipped as source code (NOT compiled into the main app)
// so third-party vendors can study it, copy it, and build their own
// plugins for proprietary sensor formats.
//
// To build a plugin from this template:
//   1. Create a new Cargo project: `cargo new --lib my_vendor_plugin`
//   2. Copy this file to src/lib.rs
//   3. Modify the FileReaderPlugin implementation for your format
//   4. Add metardu-core as a dependency
//   5. Build with `cargo build --release`
//   6. Drop the resulting .so/.dll/.dylib into MetaRDU's plugins/ directory
//
// This example implements a fictional "Norbit WBM" format reader.
// Real vendors should replace the parsing logic with their actual
// format spec.
//
// Plugin loading is handled by `plugins/dynamic_loader.rs` in the main
// app via the `libloading` crate.

// IMPORTANT: This file is NOT part of the main app's build. It lives in
// examples/ so vendors can find it. The code below would be the entire
// contents of a vendor's plugin crate.

// === BEGIN VENDOR PLUGIN TEMPLATE ===

/*
Cargo.toml for the vendor plugin:

[package]
name = "metardu-plugin-norbit-wbm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Required for dynamic loading

[dependencies]
metardu-core = { path = "../crates/metardu-core" }
serde = { version = "1", features = ["derive"] }

# Plugin entry point macros (provided by metardu-core in Phase 8+)
# metardu-plugin-macros = "0.1"
*/

// src/lib.rs for the vendor plugin:

/*
use metardu_core::plugins::{FileReaderPlugin, PluginInfo, PluginCapability, FileProbeOutput, PluginError};
use std::path::Path;

/// Plugin entry point — called by MetaRDU's dynamic loader.
/// The name MUST be `metardu_plugin_create` and must return a
/// `Box<dyn FileReaderPlugin>`.
#[no_mangle]
pub extern "C" fn metardu_plugin_create() -> Box<dyn FileReaderPlugin> {
    Box::new(NorbitWbmPlugin)
}

pub struct NorbitWbmPlugin;

impl FileReaderPlugin for NorbitWbmPlugin {
    fn info(&self) -> &PluginInfo {
        static INFO: once_cell::sync::Lazy<PluginInfo> = once_cell::sync::Lazy::new(|| {
            PluginInfo {
                name: "Norbit WBM Reader".into(),
                version: "0.1.0".into(),
                vendor: "Norbit Subsea".into(),
                description: "Reads Norbit .wbm multibeam data files".into(),
                capabilities: vec![PluginCapability::FileReader],
            }
        });
        &INFO
    }

    fn can_read(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("wbm"))
            .unwrap_or(false)
    }

    fn probe(&self, path: &Path) -> Result<FileProbeOutput, PluginError> {
        // Open the file and read the magic header
        let mut file = std::fs::File::open(path).map_err(|e| PluginError::Io(e.to_string()))?;
        let mut magic = [0u8; 4];
        use std::io::Read;
        file.read_exact(&mut magic).map_err(|e| PluginError::Io(e.to_string()))?;

        // Verify Norbit magic (fictional — replace with actual format magic)
        if &magic != b"NBWM" {
            return Err(PluginError::InvalidFormat(format!(
                "not a Norbit WBM file: magic was {:?}",
                magic
            )));
        }

        // Read header fields (fictional layout)
        let mut header = [0u8; 64];
        file.read_exact(&mut header).map_err(|e| PluginError::Io(e.to_string()))?;

        let n_pings = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let n_beams = u16::from_le_bytes([header[4], header[5]]);
        let frequency_hz = u32::from_le_bytes([header[6], header[7], header[8], header[9]]);

        Ok(FileProbeOutput {
            format: "Norbit WBM".into(),
            n_records: n_pings as u64,
            bounds: None, // would populate from nav data
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("frequency_hz".into(), frequency_hz.to_string());
                m.insert("n_beams".into(), n_beams.to_string());
                m.insert("n_pings".into(), n_pings.to_string());
                m
            },
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wbm"]
    }
}
*/

// === END VENDOR PLUGIN TEMPLATE ===

// Below: documentation that ships with the app explaining the plugin SDK.
// This module is `#[allow(dead_code)]` so it compiles without warnings
// even though it's documentation-only.

/// Plugin SDK documentation — see `plugins/mod.rs` for the trait definitions.
///
/// ## Building a Plugin
///
/// 1. Create a new Cargo project with `crate-type = ["cdylib"]`
/// 2. Add `metardu-core` as a dependency
/// 3. Implement `FileReaderPlugin`, `ProcessorPlugin`, or `ExporterPlugin`
/// 4. Export a `metardu_plugin_create` entry point with `#[no_mangle]`
/// 5. Build with `cargo build --release`
/// 6. Drop the .so/.dll/.dylib into MetaRDU's plugins/ directory
///
/// ## Plugin Discovery
///
/// MetaRDU scans the `plugins/` directory at startup. Each `.so`/`.dll`/
/// `.dylib` file is loaded via `libloading::Library::new()`. The loader
/// calls `metardu_plugin_create()` to get a `Box<dyn FileReaderPlugin>`
/// (or `ProcessorPlugin` / `ExporterPlugin`), then registers it in the
/// `PluginRegistry`.
///
/// ## Safety
///
/// Plugins run in the same process as MetaRDU Industrial. A crashing
/// plugin takes down the app. Phase 8+ will add WASM-based sandboxing
/// for untrusted plugins.
///
/// ## Versioning
///
/// The plugin ABI is versioned via the `PluginInfo.version` field.
/// MetaRDU checks the plugin's required ABI version against its own
/// and refuses to load incompatible plugins.
#[allow(dead_code)]
pub mod sdk_docs {
    // This module exists solely for documentation purposes.
    // The actual plugin trait definitions live in `plugins/mod.rs`.
}
