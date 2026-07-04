// Error context helpers — make every IPC failure point precisely at
// where and why it happened.
//
// Problem we're solving: the original IPC commands did
//   `.map_err(|e| e.to_string())?` everywhere, which means the frontend
// sees error messages like "I/O error: Permission denied (os error 13)"
// with no hint about WHICH file, WHICH operation, or WHICH module the
// error came from. Debugging meant grep-and-pray.
//
// Solution: every IPC command now wraps its error with a `ctx!()` macro
// that adds:
//   1. Module path (e.g., `commands::marine::compute_dredge_audit_cmd`)
//   2. Operation label (e.g., `reading post-dredge GeoTIFF header`)
//   3. Input identifier (e.g., the file path that failed)
//
// Example output:
//   "commands::marine::compute_dredge_audit_cmd: reading post-dredge
//    GeoTIFF header for '/tmp/survey.tif': I/O error: No such file or
//    directory (os error 2)"
//
// This means a surveyor reporting a bug can paste the error message
// and we know EXACTLY which line of code produced it.
//
// Usage:
//   use crate::error_context::ctx;
//
//   read_geotiff_header(&path).map_err(|e| ctx!("reading post-dredge DEM header", path, e))?;
//
// Or for paths through Result-returning functions:
//   let header = ctx!(read_geotiff_header, &path, "post-dredge GeoTIFF header")?;

use std::fmt::Display;

/// Wrap an error with full debug context: module path, operation,
/// input identifier, and the original error.
///
/// Returns a `String` suitable for the `Err` variant of any IPC command.
///
/// # Examples
/// ```ignore
/// use crate::error_context::ctx;
///
/// let header = read_geotiff_header(&path)
///     .map_err(|e| ctx!("reading post-dredge DEM header", path, e))?;
/// ```
#[macro_export]
macro_rules! ctx {
    ($op:expr, $input:expr, $err:expr) => {{
        // module_path!() expands to the full path of the calling function,
        // e.g., "metardu_industrial::commands::marine::compute_dredge_audit_cmd"
        $crate::error_context::format_error(module_path!(), $op, &$input, &$err)
    }};
}

/// Format an error with full context. Called by the `ctx!` macro —
/// not intended for direct use.
pub fn format_error<I: Display, E: Display>(
    module: &str,
    operation: &str,
    input: &I,
    err: &E,
) -> String {
    format!(
        "{module}: {operation} for '{input}': {err}",
        module = module,
        operation = operation,
        input = input,
        err = err,
    )
}

/// Format an error with module + operation but no input identifier.
/// Use when there's no natural "thing" to identify (e.g., a computation
/// that takes many inputs).
pub fn format_error_no_input<E: Display>(module: &str, operation: &str, err: &E) -> String {
    format!("{module}: {operation}: {err}")
}

/// Wrap an error with module + operation context, no input identifier.
///
/// Use this variant when the operation has no natural identifier
/// (e.g., a multi-input computation).
#[macro_export]
macro_rules! ctx_no_input {
    ($op:expr, $err:expr) => {{
        $crate::error_context::format_error_no_input(module_path!(), $op, &$err)
    }};
}

/// Convenience: wrap a Result-returning call with context.
/// Falls back to .map_err with ctx!.
///
/// Example:
/// ```ignore
/// let header = ctx_call!(read_geotiff_header(&path), "post-dredge DEM header", path);
/// ```
#[macro_export]
macro_rules! ctx_call {
    ($expr:expr, $op:expr, $input:expr) => {
        $expr.map_err(|e| $crate::ctx!($op, $input, e))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_error_includes_all_context() {
        let msg = format_error(
            "metardu_industrial::commands::marine::compute_dredge_audit_cmd",
            "reading post-dredge DEM header",
            &"/tmp/survey.tif",
            &std::io::Error::new(std::io::ErrorKind::NotFound, "No such file or directory"),
        );
        assert!(msg.contains("compute_dredge_audit_cmd"));
        assert!(msg.contains("reading post-dredge DEM header"));
        assert!(msg.contains("/tmp/survey.tif"));
        assert!(msg.contains("No such file or directory"));
    }

    #[test]
    fn test_format_error_no_input() {
        let msg = format_error_no_input(
            "metardu_industrial::commands::mining::compute_volumes_cmd",
            "computing volumes",
            &"grid is empty",
        );
        assert!(msg.contains("compute_volumes_cmd"));
        assert!(msg.contains("computing volumes"));
        assert!(msg.contains("grid is empty"));
    }

    #[test]
    fn test_ctx_macro_format() {
        let err: std::io::Error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let msg = ctx!("reading DEM", "/tmp/foo.tif", err);
        // module_path!() in tests expands to something with "tests" in it
        assert!(msg.contains("reading DEM"));
        assert!(msg.contains("/tmp/foo.tif"));
        assert!(msg.contains("denied"));
    }
}
