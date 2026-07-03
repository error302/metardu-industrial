// Plugin SDK — trait-based interface for third-party sensor support.
//
// Per ARCHITECTURE.md §9.8 — a Rust SDK for third-party sensor plugins.
// A sonar vendor can ship a plugin that registers their proprietary
// format reader, and it loads at startup without recompiling the main app.
//
// Phase 3: defines the plugin trait, registry, and lifecycle.
// Phase 4: dynamic loading via `libloading` (see dynamic_loader.rs).
//
// Plugin types:
//   - FileReaderPlugin: read a proprietary file format (e.g., Norbit .wbm)
//   - ProcessorPlugin: custom processing step (e.g., vendor-specific filter)
//   - ExporterPlugin: custom export format (e.g., vendor-specific chart)
//
// Each plugin declares its capabilities via the PluginInfo struct and
// implements the relevant trait(s). The PluginRegistry holds all
// registered plugins and provides lookup by capability.

#[allow(dead_code)]
pub mod dynamic_loader;
#[allow(dead_code)]
pub mod reference_plugin_template;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Metadata about a plugin — name, version, vendor, capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub vendor: String,
    pub description: String,
    pub capabilities: Vec<PluginCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    FileReader,
    Processor,
    Exporter,
}

/// File reader plugin — reads a proprietary sensor file format.
pub trait FileReaderPlugin: Send + Sync {
    fn info(&self) -> &PluginInfo;

    /// Check if this plugin can read the given file (by extension or magic).
    fn can_read(&self, path: &std::path::Path) -> bool;

    /// Read the file header and return probe metadata.
    fn probe(&self, path: &std::path::Path) -> Result<FileProbeOutput, PluginError>;

    /// Supported file extensions (e.g., ["wbm", "wbd"])
    fn extensions(&self) -> &[&str];
}

/// Processor plugin — custom processing step in a pipeline.
pub trait ProcessorPlugin: Send + Sync {
    fn info(&self) -> &PluginInfo;

    /// Process a set of input data and return output data.
    fn process(&self, input: &ProcessorInput) -> Result<ProcessorOutput, PluginError>;
}

/// Exporter plugin — writes to a custom output format.
pub trait ExporterPlugin: Send + Sync {
    fn info(&self) -> &PluginInfo;

    /// Export data to the given path.
    fn export(&self, data: &ExportData, path: &std::path::Path) -> Result<(), PluginError>;

    /// Supported file extensions for export
    fn extensions(&self) -> &[&str];
}

/// Output from a file reader plugin's probe method.
#[derive(Debug, Clone, Serialize)]
pub struct FileProbeOutput {
    pub format: String,
    pub bounds: Option<[f64; 4]>,
    pub point_count: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Input to a processor plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorInput {
    pub data_type: String,
    pub data: Vec<f64>,
    pub metadata: HashMap<String, String>,
}

/// Output from a processor plugin.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessorOutput {
    pub data_type: String,
    pub data: Vec<f64>,
    pub metadata: HashMap<String, String>,
}

/// Data for an exporter plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub format: String,
    pub features: Vec<serde_json::Value>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin error: {0}")]
    Generic(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Central registry for all registered plugins.
pub struct PluginRegistry {
    file_readers: Vec<Box<dyn FileReaderPlugin>>,
    processors: Vec<Box<dyn ProcessorPlugin>>,
    exporters: Vec<Box<dyn ExporterPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            file_readers: Vec::new(),
            processors: Vec::new(),
            exporters: Vec::new(),
        }
    }

    pub fn register_file_reader(&mut self, plugin: Box<dyn FileReaderPlugin>) {
        self.file_readers.push(plugin);
    }

    pub fn register_processor(&mut self, plugin: Box<dyn ProcessorPlugin>) {
        self.processors.push(plugin);
    }

    pub fn register_exporter(&mut self, plugin: Box<dyn ExporterPlugin>) {
        self.exporters.push(plugin);
    }

    /// Find a file reader that can handle the given path.
    pub fn find_file_reader(&self, path: &std::path::Path) -> Option<&dyn FileReaderPlugin> {
        self.file_readers
            .iter()
            .find(|p| p.can_read(path))
            .map(|p| p.as_ref())
    }

    /// List all registered plugins (for the About dialog).
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut infos = Vec::new();
        for p in &self.file_readers {
            infos.push(p.info().clone());
        }
        for p in &self.processors {
            infos.push(p.info().clone());
        }
        for p in &self.exporters {
            infos.push(p.info().clone());
        }
        infos
    }

    /// Get all file reader extensions (for drag-and-drop validation).
    pub fn supported_extensions(&self) -> Vec<String> {
        let mut exts = Vec::new();
        for p in &self.file_readers {
            for e in p.extensions() {
                exts.push(e.to_string());
            }
        }
        exts
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global plugin registry — initialized at startup, accessible from IPC commands.
pub fn global_registry() -> &'static Mutex<PluginRegistry> {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<Mutex<PluginRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(PluginRegistry::new()))
}

/// IPC command: list all registered plugins.
#[tauri::command]
pub fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    let registry = global_registry().lock().map_err(|e| e.to_string())?;
    Ok(registry.list_plugins())
}

/// IPC command: get all supported file extensions from plugins.
#[tauri::command]
pub fn get_supported_extensions() -> Result<Vec<String>, String> {
    let registry = global_registry().lock().map_err(|e| e.to_string())?;
    Ok(registry.supported_extensions())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct TestReaderPlugin {
        info: PluginInfo,
    }

    impl FileReaderPlugin for TestReaderPlugin {
        fn info(&self) -> &PluginInfo {
            &self.info
        }
        fn can_read(&self, path: &Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e == "test")
                .unwrap_or(false)
        }
        fn probe(&self, _path: &Path) -> Result<FileProbeOutput, PluginError> {
            Ok(FileProbeOutput {
                format: "test".into(),
                bounds: None,
                point_count: Some(0),
                metadata: HashMap::new(),
            })
        }
        fn extensions(&self) -> &[&str] {
            &["test"]
        }
    }

    #[test]
    fn test_register_and_find() {
        let mut registry = PluginRegistry::new();
        registry.register_file_reader(Box::new(TestReaderPlugin {
            info: PluginInfo {
                name: "Test Reader".into(),
                version: "0.1".into(),
                vendor: "Test".into(),
                description: "Test plugin".into(),
                capabilities: vec![PluginCapability::FileReader],
            },
        }));

        let path = Path::new("file.test");
        assert!(registry.find_file_reader(path).is_some());

        let path = Path::new("file.las");
        assert!(registry.find_file_reader(path).is_none());
    }

    #[test]
    fn test_list_plugins() {
        let mut registry = PluginRegistry::new();
        registry.register_file_reader(Box::new(TestReaderPlugin {
            info: PluginInfo {
                name: "Test Reader".into(),
                version: "0.1".into(),
                vendor: "Test".into(),
                description: "Test plugin".into(),
                capabilities: vec![PluginCapability::FileReader],
            },
        }));
        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "Test Reader");
    }

    #[test]
    fn test_supported_extensions() {
        let mut registry = PluginRegistry::new();
        registry.register_file_reader(Box::new(TestReaderPlugin {
            info: PluginInfo {
                name: "Test".into(),
                version: "0.1".into(),
                vendor: "Test".into(),
                description: "Test".into(),
                capabilities: vec![PluginCapability::FileReader],
            },
        }));
        let exts = registry.supported_extensions();
        assert!(exts.contains(&"test".to_string()));
    }
}
