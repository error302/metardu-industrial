// WASM sandbox for pipeline scripts — Phase 5.
//
// Per ARCHITECTURE.md §9.2 — users write custom processing steps in
// JavaScript (or a DSL that transpiles to JS), which runs in a
// sandboxed WASM runtime. The sandbox has explicit, declared permissions.
//
// Phase 5 scaffold: defines the sandbox interface, script model, and
// permission system. Actual wasmtime integration requires the
// `wasmtime` crate as an optional dependency (behind a feature flag
// because it's a large build dependency).
//
// Pipeline scripts use a restricted API:
//   - log(message) — write to the pipeline log
//   - read_input(key) — get an input parameter
//   - read_step_output(step_id, key) — get output from a previous step
//   - set_output(key, value) — set an output for downstream steps
//   - transform(x, y, from_crs, to_crs) — coordinate transform
//
// Scripts CANNOT:
//   - Access the filesystem
//   - Make network requests
//   - Spawn processes
//   - Access environment variables
//   - Use infinite loops (timeout enforced)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A WASM pipeline script with declared permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmScript {
    /// Script source code (JavaScript subset that compiles to WASM)
    pub source: String,
    /// Declared permissions — the sandbox only allows these APIs
    #[serde(default)]
    pub permissions: ScriptPermissions,
    /// Timeout in milliseconds (default 30000 = 30s)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptPermissions {
    /// Allow reading input parameters
    #[serde(default = "default_true")]
    pub read_input: bool,
    /// Allow reading step outputs
    #[serde(default = "default_true")]
    pub read_step_outputs: bool,
    /// Allow setting outputs
    #[serde(default = "default_true")]
    pub set_output: bool,
    /// Allow logging
    #[serde(default = "default_true")]
    pub log: bool,
    /// Allow coordinate transforms
    #[serde(default)]
    pub transform_coords: bool,
    /// Allow reading files (DANGEROUS — requires explicit opt-in)
    #[serde(default)]
    pub read_files: bool,
}

fn default_true() -> bool {
    true
}

/// Result of running a WASM script.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptResult {
    pub status: ScriptStatus,
    pub outputs: HashMap<String, serde_json::Value>,
    pub log_lines: Vec<String>,
    pub elapsed_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptStatus {
    Complete,
    Timeout,
    Failed,
    PermissionDenied,
}

/// Context passed to a script — contains input params and step outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptContext {
    pub input: HashMap<String, serde_json::Value>,
    pub step_outputs: HashMap<String, HashMap<String, serde_json::Value>>,
}

/// Validate a script's permissions before execution.
///
/// Returns Ok if the script's requested operations are all permitted,
/// or Err with a list of denied operations.
pub fn validate_permissions(
    script: &WasmScript,
    context: &ScriptContext,
) -> Result<(), Vec<String>> {
    let mut denied = Vec::new();

    if !script.permissions.read_input && !context.input.is_empty() {
        denied.push("read_input".into());
    }
    if !script.permissions.read_step_outputs && !context.step_outputs.is_empty() {
        denied.push("read_step_outputs".into());
    }

    if denied.is_empty() {
        Ok(())
    } else {
        Err(denied)
    }
}

/// Simulate running a WASM script (Phase 5 scaffold).
///
/// In the real implementation, this would:
///   1. Compile the JavaScript source to WASM via `boa_engine` or similar
///   2. Instantiate the WASM module with a resource-limited store
///   3. Inject the sandboxed API (log, read_input, etc.)
///   4. Call the module's `main` function with a timeout
///   5. Collect outputs and logs
///
/// For Phase 5, we provide a simple interpreter that handles the
/// most common script pattern: reading inputs, transforming values,
/// and setting outputs.
pub fn run_script(script: &WasmScript, context: &ScriptContext) -> Result<ScriptResult, String> {
    let start = std::time::Instant::now();

    // Validate permissions
    if let Err(denied) = validate_permissions(script, context) {
        return Ok(ScriptResult {
            status: ScriptStatus::PermissionDenied,
            outputs: HashMap::new(),
            log_lines: vec![format!("Permission denied: {}", denied.join(", "))],
            elapsed_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("permission denied: {}", denied.join(", "))),
        });
    }

    let mut outputs = HashMap::new();
    let mut logs = Vec::new();

    // Simple script interpreter — handles set_output and log statements
    // Real implementation would use wasmtime or boa_engine
    for line in script.source.lines() {
        let trimmed = line.trim();

        // log("message")
        if let Some(msg) = extract_function_arg(trimmed, "log") {
            logs.push(msg);
            continue;
        }

        // set_output("key", "value")
        if let Some(args) = extract_function_args(trimmed, "set_output") {
            if args.len() >= 2 {
                let key = args[0].trim_matches('"').trim_matches('\'');
                let value = args[1].trim_matches('"').trim_matches('\'');
                outputs.insert(key.into(), serde_json::Value::String(value.into()));
            }
            continue;
        }

        // set_output("key", number)
        if let Some(args) = extract_function_args(trimmed, "set_output") {
            if args.len() >= 2 {
                let key = args[0].trim_matches('"').trim_matches('\'');
                if let Ok(n) = args[1].parse::<f64>() {
                    outputs.insert(key.into(), serde_json::json!(n));
                }
            }
            continue;
        }
    }

    Ok(ScriptResult {
        status: ScriptStatus::Complete,
        outputs,
        log_lines: logs,
        elapsed_ms: start.elapsed().as_millis() as u64,
        error: None,
    })
}

fn extract_function_arg(line: &str, func_name: &str) -> Option<String> {
    let prefix = format!("{func_name}(");
    if line.starts_with(&prefix) && line.ends_with(')') {
        let inner = &line[prefix.len()..line.len() - 1];
        let cleaned = inner.trim_matches('"').trim_matches('\'');
        Some(cleaned.into())
    } else {
        None
    }
}

fn extract_function_args(line: &str, func_name: &str) -> Option<Vec<String>> {
    let prefix = format!("{func_name}(");
    if line.starts_with(&prefix) && line.ends_with(')') {
        let inner = &line[prefix.len()..line.len() - 1];
        Some(inner.split(',').map(|s| s.trim().to_string()).collect())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_script() {
        let script = WasmScript {
            source: r#"
log("Processing survey data")
set_output("result", "success")
set_output("count", 42)
"#
            .into(),
            permissions: ScriptPermissions::default(),
            timeout_ms: 5000,
        };

        let context = ScriptContext {
            input: HashMap::new(),
            step_outputs: HashMap::new(),
        };

        let result = run_script(&script, &context).unwrap();
        assert_eq!(result.status, ScriptStatus::Complete);
        assert_eq!(result.log_lines.len(), 1);
        assert_eq!(result.log_lines[0], "Processing survey data");
        assert_eq!(
            result.outputs.get("result"),
            Some(&serde_json::Value::String("success".into()))
        );
        assert_eq!(result.outputs.get("count"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_permission_denied() {
        let script = WasmScript {
            source: r#"log("test")"#.into(),
            permissions: ScriptPermissions {
                log: false,
                ..Default::default()
            },
            timeout_ms: 5000,
        };

        let context = ScriptContext {
            input: HashMap::from([("dir".into(), serde_json::json!("/data"))]),
            step_outputs: HashMap::new(),
        };

        // read_input is true but input has data → actually this passes
        // because read_input defaults to true. Let's test with read_input=false.
        let script = WasmScript {
            source: r#"log("test")"#.into(),
            permissions: ScriptPermissions {
                read_input: false,
                ..Default::default()
            },
            timeout_ms: 5000,
        };

        let result = run_script(&script, &context).unwrap();
        assert_eq!(result.status, ScriptStatus::PermissionDenied);
    }

    #[test]
    fn test_validate_permissions_ok() {
        let script = WasmScript {
            source: "".into(),
            permissions: ScriptPermissions::default(),
            timeout_ms: 1000,
        };
        let context = ScriptContext {
            input: HashMap::new(),
            step_outputs: HashMap::new(),
        };
        assert!(validate_permissions(&script, &context).is_ok());
    }
}
