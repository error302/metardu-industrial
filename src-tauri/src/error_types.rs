// MetarduError — structured error type for all IPC commands (Sprint 14).
//
// Before this, every IPC command returned `Result<T, String>` — a flat
// string with no structure. The frontend couldn't distinguish "file not
// found" from "parse error" from "permission denied", so it couldn't
// show appropriate recovery actions.
//
// MetarduError is a tagged enum serialized as a JSON object with a
// `kind` field so the frontend can pattern-match:
//
// ```ts
// try {
//   await invoke("compute_volumes_cmd", ...);
// } catch (err) {
//   const e = err as MetarduError;
//   switch (e.kind) {
//     case "file_not_found":
//       showBrowseButton(e.path);
//       break;
//     case "parse_error":
//       showFormatHelp(e.format, e.line);
//       break;
//     case "permission_denied":
//       showFileLockHelp(e.path);
//       break;
//   }
// }
// ```
//
// Backwards compat: `MetarduError::to_string()` produces the same
// human-readable string as before, so existing `map_err(|e| format!(...))`
// callers can migrate incrementally.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MetarduError {
    /// File or directory not found.
    FileNotFound {
        path: String,
    },
    /// Parse error in a file format (LAS, GeoTIFF, .all, etc.)
    ParseError {
        format: String,
        line: Option<u32>,
        detail: String,
    },
    /// Permission denied (file locked, read-only, etc.)
    PermissionDenied {
        path: String,
    },
    /// Invalid user input (out of range, wrong type, etc.)
    InvalidInput {
        field: String,
        value: String,
        reason: String,
    },
    /// Calculation failed (numerical error, empty data, etc.)
    CalculationError {
        step: String,
        detail: String,
    },
    /// I/O error (disk full, network failure, etc.)
    IoError {
        detail: String,
    },
    /// Operation timed out (Sprint 14 timeout wrapper)
    Timeout {
        operation: String,
        timeout_secs: u64,
    },
    /// Feature not available in browser mode
    BrowserMode {
        feature: String,
    },
    /// License required for this feature
    LicenseRequired {
        feature: String,
    },
    /// Catch-all for unexpected errors
    Internal {
        detail: String,
    },
}

impl fmt::Display for MetarduError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetarduError::FileNotFound { path } => {
                write!(f, "File not found: {}", path)
            }
            MetarduError::ParseError { format, line, detail } => {
                match line {
                    Some(ln) => write!(f, "{} parse error at line {}: {}", format, ln, detail),
                    None => write!(f, "{} parse error: {}", format, detail),
                }
            }
            MetarduError::PermissionDenied { path } => {
                write!(f, "Permission denied: {} (file may be locked or read-only)", path)
            }
            MetarduError::InvalidInput { field, value, reason } => {
                write!(f, "Invalid input for '{}': '{}' — {}", field, value, reason)
            }
            MetarduError::CalculationError { step, detail } => {
                write!(f, "Calculation failed at step '{}': {}", step, detail)
            }
            MetarduError::IoError { detail } => {
                write!(f, "I/O error: {}", detail)
            }
            MetarduError::Timeout { operation, timeout_secs } => {
                write!(f, "Operation '{}' timed out after {} seconds", operation, timeout_secs)
            }
            MetarduError::BrowserMode { feature } => {
                write!(f, "'{}' is not available in browser mode — requires the native Tauri shell", feature)
            }
            MetarduError::LicenseRequired { feature } => {
                write!(f, "License required for '{}' — activate a license to use this feature", feature)
            }
            MetarduError::Internal { detail } => {
                write!(f, "Internal error: {}", detail)
            }
        }
    }
}

impl std::error::Error for MetarduError {}

// ──────────────────────────────────────────────────────────────────
// Convenience constructors
// ──────────────────────────────────────────────────────────────────

impl MetarduError {
    pub fn file_not_found(path: impl Into<String>) -> Self {
        MetarduError::FileNotFound { path: path.into() }
    }

    pub fn parse_error(format: impl Into<String>, detail: impl Into<String>) -> Self {
        MetarduError::ParseError {
            format: format.into(),
            line: None,
            detail: detail.into(),
        }
    }

    pub fn parse_error_at_line(format: impl Into<String>, line: u32, detail: impl Into<String>) -> Self {
        MetarduError::ParseError {
            format: format.into(),
            line: Some(line),
            detail: detail.into(),
        }
    }

    pub fn permission_denied(path: impl Into<String>) -> Self {
        MetarduError::PermissionDenied { path: path.into() }
    }

    pub fn invalid_input(field: impl Into<String>, value: impl Into<String>, reason: impl Into<String>) -> Self {
        MetarduError::InvalidInput {
            field: field.into(),
            value: value.into(),
            reason: reason.into(),
        }
    }

    pub fn calculation_error(step: impl Into<String>, detail: impl Into<String>) -> Self {
        MetarduError::CalculationError {
            step: step.into(),
            detail: detail.into(),
        }
    }

    pub fn io_error(detail: impl Into<String>) -> Self {
        MetarduError::IoError { detail: detail.into() }
    }

    pub fn timeout(operation: impl Into<String>, timeout_secs: u64) -> Self {
        MetarduError::Timeout {
            operation: operation.into(),
            timeout_secs,
        }
    }

    pub fn browser_mode(feature: impl Into<String>) -> Self {
        MetarduError::BrowserMode { feature: feature.into() }
    }

    pub fn license_required(feature: impl Into<String>) -> Self {
        MetarduError::LicenseRequired { feature: feature.into() }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        MetarduError::Internal { detail: detail.into() }
    }
}

// ──────────────────────────────────────────────────────────────────
// Conversions from common error types
// ──────────────────────────────────────────────────────────────────

impl From<std::io::Error> for MetarduError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => MetarduError::IoError { detail: e.to_string() },
            std::io::ErrorKind::PermissionDenied => MetarduError::IoError { detail: e.to_string() },
            _ => MetarduError::IoError { detail: e.to_string() },
        }
    }
}

impl From<serde_json::Error> for MetarduError {
    fn from(e: serde_json::Error) -> Self {
        MetarduError::ParseError {
            format: "JSON".to_string(),
            line: Some(e.line() as u32),
            detail: e.to_string(),
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Timeout wrapper for spawn_blocking commands
// ──────────────────────────────────────────────────────────────────

/// Wrap a `spawn_blocking` future with a timeout.
///
/// Usage:
/// ```rust
/// let result = with_timeout(
///     "compute_volumes",
///     300,
///     tokio::task::spawn_blocking(move || { ... }),
/// ).await?;
/// ```
///
/// Returns `MetarduError::Timeout` if the operation doesn't complete
/// within `timeout_secs` seconds.
pub async fn with_timeout<T, E>(
    operation: &str,
    timeout_secs: u64,
    fut: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, MetarduError>
where
    E: Into<MetarduError>,
{
    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        fut,
    ).await {
        Ok(result) => result.map_err(Into::into),
        Err(_) => Err(MetarduError::timeout(operation, timeout_secs)),
    }
}

/// Default timeout for file-parsing operations (5 minutes).
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Default timeout for computation-heavy operations (10 minutes).
pub const LONG_TIMEOUT_SECS: u64 = 600;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_file_not_found() {
        let e = MetarduError::file_not_found("/path/to/file.las");
        assert_eq!(e.to_string(), "File not found: /path/to/file.las");
    }

    #[test]
    fn test_display_parse_error_with_line() {
        let e = MetarduError::parse_error_at_line("LAS", 42, "truncated header");
        assert_eq!(e.to_string(), "LAS parse error at line 42: truncated header");
    }

    #[test]
    fn test_display_parse_error_without_line() {
        let e = MetarduError::parse_error("GeoTIFF", "invalid tag");
        assert_eq!(e.to_string(), "GeoTIFF parse error: invalid tag");
    }

    #[test]
    fn test_display_timeout() {
        let e = MetarduError::timeout("compute_volumes", 300);
        assert_eq!(e.to_string(), "Operation 'compute_volumes' timed out after 300 seconds");
    }

    #[test]
    fn test_display_browser_mode() {
        let e = MetarduError::browser_mode("LAS parsing");
        assert!(e.to_string().contains("browser mode"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let e = MetarduError::file_not_found("/test/path.las");
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"kind\":\"file_not_found\""));
        assert!(json.contains("/test/path.las"));
        let decoded: MetarduError = serde_json::from_str(&json).unwrap();
        match decoded {
            MetarduError::FileNotFound { path } => assert_eq!(path, "/test/path.las"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_serde_all_variants() {
        let errors = vec![
            MetarduError::file_not_found("/path"),
            MetarduError::parse_error("LAS", "bad header"),
            MetarduError::parse_error_at_line("GeoTIFF", 10, "bad tag"),
            MetarduError::permission_denied("/locked"),
            MetarduError::invalid_input("cell_size", "-1", "must be positive"),
            MetarduError::calculation_error("volume", "empty grid"),
            MetarduError::io_error("disk full"),
            MetarduError::timeout("csf", 300),
            MetarduError::browser_mode("NTRIP"),
            MetarduError::license_required("eom_auditor"),
            MetarduError::internal("unexpected"),
        ];
        for e in &errors {
            let json = serde_json::to_string(e).unwrap();
            let _: MetarduError = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let e: MetarduError = io_err.into();
        match e {
            MetarduError::IoError { detail } => assert!(detail.contains("missing")),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result: Result<i32, MetarduError> = with_timeout("test", 5, async { Ok(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_expires() {
        let result: Result<i32, MetarduError> = with_timeout(
            "test_slow",
            0,
            async {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                Ok(42)
            },
        ).await;
        assert!(matches!(result, Err(MetarduError::Timeout { .. })));
    }
}
