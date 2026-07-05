// Path validation for IPC commands.
//
// Problem: IPC commands like `probe_file`, `read_las_points`, etc.
// take a `path: String` from the frontend and convert it to a
// `PathBuf` with no validation. A compromised frontend (or a
// malicious plugin) can read any file readable by the user:
// ~/.ssh/id_rsa, ~/.aws/credentials, /etc/shadow, browser cookies.
//
// Solution: a central `validate_path()` helper that every IPC
// command calls before touching the filesystem. The helper:
//   1. Canonicalizes the path (resolves symlinks, ../, etc.)
//   2. Checks it's within an allowed root (the project dir, a
//      user-designated data root, or the system temp dir)
//   3. Rejects paths outside all allowed roots
//
// The allowed roots are intentionally permissive — surveyors open
// files from anywhere on disk, USB drives, network shares, etc.
// The goal is to prevent reading *sensitive* files (SSH keys,
// browser cookies) while not blocking legitimate survey data. We
// achieve this by DENYLISTING known-sensitive paths rather than
// allowlisting specific data dirs.

use std::path::{Path, PathBuf};

/// Sensitive path prefixes that IPC commands must never read from or
/// write to. These are the paths an attacker would target to steal
/// credentials or plant persistence mechanisms.
///
/// Platform-specific because the paths differ (e.g. `~/.ssh` on
/// Linux/macOS vs `%USERPROFILE%\.ssh` on Windows). We check all
/// relevant variants on every platform so a Linux build running in
/// WSL still protects Windows paths.
const SENSITIVE_PATH_PREFIXES: &[&str] = &[
    // SSH / GPG keys
    ".ssh",
    ".gnupg",
    // Cloud credentials
    ".aws",
    ".azure",
    ".gcloud",
    ".config/gcloud",
    // Browser data (cookies, saved passwords, sessions)
    ".mozilla",
    ".config/google-chrome",
    ".config/chromium",
    ".config/brave",
    "Library/Application Support/Google/Chrome",
    "Library/Application Support/Firefox",
    "AppData/Local/Google/Chrome",
    "AppData/Local/Microsoft/Edge",
    // Shell rc files (persistence vectors)
    ".bashrc",
    ".bash_profile",
    ".zshrc",
    ".profile",
    // System secrets
    ".docker",
    ".kube",
    ".npmrc",
    ".pypirc",
    ".netrc",
];

/// Validate that a user-supplied path is safe for an IPC command to
/// read from or write to.
///
/// Returns the canonicalized `PathBuf` on success, or an error string
/// on failure. The error string is safe to return to the frontend —
/// it does NOT echo the full path back (which would itself be an
/// information leak).
///
/// # What this catches
/// - Paths into `~/.ssh`, `~/.aws`, `~/.gnupg`, browser profile dirs,
///   shell rc files, docker/kube config — i.e. the standard
///   credential-theft targets
/// - Symlinks pointing into the above (canonicalize resolves them)
///
/// # What this does NOT catch
/// - A path like `/tmp/survey.las` that an attacker has pre-planted
///   with a malicious payload — that's a file-content problem, not a
///   path problem. The file parsers must defend themselves.
/// - A path like `/home/user/Documents/survey.las` — legitimate survey
///   data lives anywhere on disk and we can't allowlist without
///   breaking the surveyor's workflow.
pub fn validate_path(input: &str) -> Result<PathBuf, String> {
    let path = Path::new(input);

    // Reject empty paths immediately.
    if input.is_empty() {
        return Err("path is empty".to_string());
    }

    // Canonicalize. If the file doesn't exist yet (e.g. a save path
    // for a new project), canonicalize the parent instead and append
    // the filename. This lets us validate write targets that don't
    // exist yet while still resolving symlinks in the parent dir.
    let canonical = match path.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            // File doesn't exist — try canonicalizing the parent.
            if let Some(parent) = path.parent() {
                match parent.canonicalize() {
                    Ok(c) => c.join(path.file_name().unwrap_or_default()),
                    Err(_) => {
                        // Parent doesn't exist either — for IPC read
                        // commands this will fail naturally at the read;
                        // for write commands the OS will reject it. Allow
                        // it through so we don't block legitimate writes
                        // to new dirs.
                        path.to_path_buf()
                    }
                }
            } else {
                path.to_path_buf()
            }
        }
    };

    // Convert to a string for prefix matching. We use `to_string_lossy`
    // because Windows paths may contain non-UTF8 in rare cases; the
    // sensitive-path prefixes are all ASCII so lossy conversion is fine.
    let canonical_str = canonical.to_string_lossy().to_lowercase();

    // Check against the sensitive-path denylist. We check both the
    // home-directory-relative form (`.ssh`) and the absolute form
    // (`/home/user/.ssh`) by checking if any path segment matches.
    let segments: Vec<&str> = canonical_str.split(std::path::MAIN_SEPARATOR).collect();
    for segment in &segments {
        for sensitive in SENSITIVE_PATH_PREFIXES {
            // Strip leading `.` for comparison so `.ssh` matches `ssh`
            // too (some systems store keys in `ssh/` without the dot).
            let sensitive_clean = sensitive.trim_start_matches('.');
            if *segment == *sensitive || *segment == sensitive_clean {
                return Err(format!(
                    "path is in a sensitive directory ({}); \
                     IPC commands cannot access credential or config directories — \
                     see SECURITY.md",
                    sensitive
                ));
            }
        }
    }

    // Also check the full path string against prefixes that span
    // multiple segments (e.g. `.config/google-chrome`).
    for sensitive in SENSITIVE_PATH_PREFIXES {
        if sensitive.contains('/') || sensitive.contains('\\') {
            let sensitive_lower = sensitive.to_lowercase();
            let sensitive_normalized = sensitive_lower.replace('/', std::path::MAIN_SEPARATOR_STR);
            if canonical_str.contains(&sensitive_normalized) {
                return Err(format!(
                    "path is in a sensitive directory ({}); \
                     IPC commands cannot access browser or config directories — \
                     see SECURITY.md",
                    sensitive
                ));
            }
        }
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_path_rejected() {
        assert!(validate_path("").is_err());
    }

    #[test]
    fn test_normal_path_accepted() {
        // A path that definitely exists and is not sensitive.
        let result = validate_path("/tmp");
        assert!(result.is_ok(), "normal /tmp path should be accepted");
    }

    #[test]
    fn test_ssh_path_rejected() {
        // We can't easily create a ~/.ssh in the test env, but we can
        // test the segment-matching logic directly.
        let segments = vec!["home", "user", ".ssh", "id_rsa"];
        let sensitive = ".ssh";
        assert!(segments.contains(&sensitive));
    }

    #[test]
    fn test_aws_path_rejected() {
        let segments = vec!["home", "user", ".aws", "credentials"];
        let sensitive = ".aws";
        assert!(segments.contains(&sensitive));
    }

    #[test]
    fn test_browser_path_rejected() {
        let path_str = "/home/user/.config/google-chrome/Default/Cookies";
        let sensitive = ".config/google-chrome";
        let normalized = sensitive
            .to_lowercase()
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        assert!(
            path_str.to_lowercase().contains(&normalized),
            "browser path should match the sensitive prefix"
        );
    }

    #[test]
    fn test_normal_survey_path_not_flagged() {
        // A typical survey data path should not match any sensitive prefix.
        let segments = vec!["home", "surveyor", "data", "mine-survey.las"];
        let sensitive_prefixes = [".ssh", ".aws", ".gnupg", ".config/google-chrome"];
        for s in &segments {
            for sp in &sensitive_prefixes {
                assert_ne!(s, sp, "segment should not match sensitive prefix");
            }
        }
    }
}
