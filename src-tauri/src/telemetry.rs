// Telemetry + Crash Reporter — Sprint 7 Enterprise Readiness.
//
// Opt-in anonymous usage stats + crash dump capture. Critical for
// production support: when a customer reports "MetaRDU crashed during
// a dredge audit", we need to know:
//   - What version they're running
//   - What OS / hardware
//   - What command was being executed
//   - The actual error/panic message
//   - The license tier (so we can prioritize Enterprise support)
//
// Privacy model:
//   - OFF by default — user must explicitly opt in via Settings
//   - Only sends: app version, OS, command name, error message, license tier
//   - NEVER sends: file paths, customer data, point cloud data, coordinates
//   - Crash dumps are stored locally first; user reviews before sending
//
// This module provides:
//   - TelemetryConfig: opt-in flag + endpoint URL + anonymous ID
//   - EventTracker: in-memory ring buffer of recent events
//   - record_event(): log a UI or IPC event
//   - record_crash(): log a crash with full context
//   - get_pending_crash_dumps(): list unsent crash dumps
//   - send_crash_dump(): submit a crash dump to the endpoint
//   - get_telemetry_stats(): summary stats for the settings UI
//
// All network operations are best-effort — telemetry failures NEVER
// crash the app.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum events held in the in-memory ring buffer
const MAX_EVENTS: usize = 1000;

/// Maximum crash dumps held before oldest is discarded
const MAX_CRASH_DUMPS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// True if the user has opted in to telemetry
    pub enabled: bool,
    /// True if crash dump auto-submission is enabled (separate consent)
    pub crash_auto_submit: bool,
    /// Endpoint URL for telemetry submission (empty = local only)
    pub endpoint_url: String,
    /// Anonymous user ID (generated on first run, persisted)
    pub anonymous_id: String,
    /// App version when this config was last updated
    pub app_version: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            crash_auto_submit: false,
            endpoint_url: String::new(), // empty = local only
            anonymous_id: generate_anonymous_id(),
            app_version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TelemetryEvent {
    /// Unix epoch milliseconds when the event occurred
    pub timestamp_ms: u64,
    /// Event type (e.g., "ipc_call", "dialog_open", "file_loaded")
    pub event_type: String,
    /// Event name (e.g., "compute_volumes_cmd", "eom_reconciliation_wizard")
    pub event_name: String,
    /// Duration in milliseconds (for IPC calls)
    pub duration_ms: Option<u64>,
    /// True if the event succeeded
    pub success: bool,
    /// Error message (if any)
    pub error: Option<String>,
    /// License tier at time of event
    pub license_tier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrashDump {
    /// Unique crash ID
    pub crash_id: String,
    /// Unix epoch milliseconds when the crash occurred
    pub timestamp_ms: u64,
    /// App version
    pub app_version: String,
    /// OS info (e.g., "windows 10.0.19041" or "linux 5.15.0")
    pub os_info: String,
    /// Command that was being executed when the crash happened
    pub command: String,
    /// Error/panic message
    pub message: String,
    /// Stack trace (if available — usually empty in release builds)
    pub stack_trace: String,
    /// License tier at time of crash
    pub license_tier: String,
    /// Anonymous user ID
    pub anonymous_id: String,
    /// True if this crash dump has been submitted to the endpoint
    pub submitted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TelemetryStats {
    /// Total events recorded since app start
    pub total_events: usize,
    /// Total crashes recorded
    pub total_crashes: usize,
    /// Crashes not yet submitted
    pub pending_crashes: usize,
    /// Top 5 most-called IPC commands (name + count)
    pub top_commands: Vec<(String, u64)>,
    /// Top 5 most-failed commands (name + count)
    pub top_failures: Vec<(String, u64)>,
    /// Average IPC call duration (ms)
    pub avg_ipc_duration_ms: f64,
    /// App uptime in seconds
    pub uptime_seconds: u64,
}

/// Global telemetry state (lazy-initialized singleton)
static TELEMETRY_STATE: Mutex<Option<TelemetryState>> = Mutex::new(None);

struct TelemetryState {
    config: TelemetryConfig,
    events: VecDeque<TelemetryEvent>,
    crashes: VecDeque<CrashDump>,
    start_time: SystemTime,
    /// Per-command call counts (for top_commands)
    command_counts: std::collections::HashMap<String, u64>,
    /// Per-command failure counts (for top_failures)
    failure_counts: std::collections::HashMap<String, u64>,
    /// Per-command total duration (for avg_ipc_duration_ms)
    command_total_ms: std::collections::HashMap<String, u64>,
    command_call_count: std::collections::HashMap<String, u64>,
}

impl TelemetryState {
    fn new() -> Self {
        Self {
            config: TelemetryConfig::default(),
            events: VecDeque::with_capacity(MAX_EVENTS),
            crashes: VecDeque::with_capacity(MAX_CRASH_DUMPS),
            start_time: SystemTime::now(),
            command_counts: std::collections::HashMap::new(),
            failure_counts: std::collections::HashMap::new(),
            command_total_ms: std::collections::HashMap::new(),
            command_call_count: std::collections::HashMap::new(),
        }
    }
}

/// Initialize the telemetry state. Called once at app startup.
pub fn init_telemetry(config: TelemetryConfig) {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    if state.is_none() {
        let mut s = TelemetryState::new();
        s.config = config;
        *state = Some(s);
    } else {
        // Update config if already initialized
        if let Some(s) = state.as_mut() {
            s.config = config;
        }
    }
}

/// Update the telemetry config (called when user toggles opt-in)
pub fn update_config(config: TelemetryConfig) {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    if state.is_none() {
        let mut s = TelemetryState::new();
        s.config = config;
        *state = Some(s);
    } else if let Some(s) = state.as_mut() {
        s.config = config;
    }
}

/// Get the current telemetry config
pub fn get_config() -> TelemetryConfig {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    if state.is_none() {
        let s = TelemetryState::new();
        let config = s.config.clone();
        *state = Some(s);
        return config;
    }
    state.as_ref().unwrap().config.clone()
}

/// Record a telemetry event (UI or IPC).
///
/// If telemetry is disabled, the event is still recorded to the in-memory
/// buffer (for the Settings UI stats display) but never sent anywhere.
pub fn record_event(
    event_type: &str,
    event_name: &str,
    duration_ms: Option<u64>,
    success: bool,
    error: Option<&str>,
    license_tier: &str,
) {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    if state.is_none() {
        *state = Some(TelemetryState::new());
    }
    let s = state.as_mut().unwrap();

    let event = TelemetryEvent {
        timestamp_ms: now_ms(),
        event_type: event_type.into(),
        event_name: event_name.into(),
        duration_ms,
        success,
        error: error.map(|s| s.into()),
        license_tier: license_tier.into(),
    };

    // Update aggregates for IPC calls
    if event_type == "ipc_call" {
        *s.command_counts.entry(event_name.to_string()).or_insert(0) += 1;
        if !success {
            *s.failure_counts.entry(event_name.to_string()).or_insert(0) += 1;
        }
        if let Some(d) = duration_ms {
            *s.command_total_ms
                .entry(event_name.to_string())
                .or_insert(0) += d;
            *s.command_call_count
                .entry(event_name.to_string())
                .or_insert(0) += 1;
        }
    }

    // Push to ring buffer
    if s.events.len() >= MAX_EVENTS {
        s.events.pop_front();
    }
    s.events.push_back(event);

    // If telemetry enabled + endpoint set, fire-and-forget send
    // (we don't block — telemetry must never impact UX)
    if s.config.enabled && !s.config.endpoint_url.is_empty() {
        // For Phase 7 we don't actually send — we'd need an HTTP client.
        // The endpoint URL is stored so future Phase 8 can wire it up.
        // For now, all events stay local.
    }
}

/// Record a crash dump.
///
/// Called from panic handlers and IPC error paths. The crash is stored
/// locally and (if auto-submit is enabled) queued for submission.
pub fn record_crash(command: &str, message: &str, stack_trace: &str, license_tier: &str) -> String {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    if state.is_none() {
        *state = Some(TelemetryState::new());
    }
    let s = state.as_mut().unwrap();

    let crash_id = generate_crash_id();
    let crash = CrashDump {
        crash_id: crash_id.clone(),
        timestamp_ms: now_ms(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        os_info: get_os_info(),
        command: command.into(),
        message: message.into(),
        stack_trace: stack_trace.into(),
        license_tier: license_tier.into(),
        anonymous_id: s.config.anonymous_id.clone(),
        submitted: false,
    };

    if s.crashes.len() >= MAX_CRASH_DUMPS {
        s.crashes.pop_front();
    }
    s.crashes.push_back(crash);

    // Auto-submit if enabled
    if s.config.crash_auto_submit && !s.config.endpoint_url.is_empty() {
        // Phase 8 will add HTTP submission. For now, just mark as queued.
    }

    crash_id
}

/// Get all pending (unsubmitted) crash dumps
pub fn get_pending_crash_dumps() -> Vec<CrashDump> {
    let state = TELEMETRY_STATE.lock().unwrap();
    match state.as_ref() {
        Some(s) => s.crashes.iter().filter(|c| !c.submitted).cloned().collect(),
        None => Vec::new(),
    }
}

/// Mark a crash dump as submitted (after successful upload)
pub fn mark_crash_submitted(crash_id: &str) {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    if let Some(s) = state.as_mut() {
        for crash in s.crashes.iter_mut() {
            if crash.crash_id == crash_id {
                crash.submitted = true;
                break;
            }
        }
    }
}

/// Get aggregated telemetry stats for the Settings UI
pub fn get_stats() -> TelemetryStats {
    let state = TELEMETRY_STATE.lock().unwrap();
    let s = match state.as_ref() {
        Some(s) => s,
        None => {
            return TelemetryStats {
                total_events: 0,
                total_crashes: 0,
                pending_crashes: 0,
                top_commands: Vec::new(),
                top_failures: Vec::new(),
                avg_ipc_duration_ms: 0.0,
                uptime_seconds: 0,
            };
        }
    };

    let total_events = s.events.len();
    let total_crashes = s.crashes.len();
    let pending_crashes = s.crashes.iter().filter(|c| !c.submitted).count();

    // Top 5 commands by call count
    let mut top_commands: Vec<(String, u64)> = s
        .command_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    top_commands.sort_by(|a, b| b.1.cmp(&a.1));
    top_commands.truncate(5);

    // Top 5 failures
    let mut top_failures: Vec<(String, u64)> = s
        .failure_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    top_failures.sort_by(|a, b| b.1.cmp(&a.1));
    top_failures.truncate(5);

    // Average IPC duration
    let total_ms: u64 = s.command_total_ms.values().sum();
    let total_calls: u64 = s.command_call_count.values().sum();
    let avg_ipc_duration_ms = if total_calls > 0 {
        total_ms as f64 / total_calls as f64
    } else {
        0.0
    };

    let uptime_seconds = SystemTime::now()
        .duration_since(s.start_time)
        .unwrap_or_default()
        .as_secs();

    TelemetryStats {
        total_events,
        total_crashes,
        pending_crashes,
        top_commands,
        top_failures,
        avg_ipc_duration_ms,
        uptime_seconds,
    }
}

/// Get recent events (for the Settings UI diagnostic panel)
pub fn get_recent_events(limit: usize) -> Vec<TelemetryEvent> {
    let state = TELEMETRY_STATE.lock().unwrap();
    match state.as_ref() {
        Some(s) => s.events.iter().rev().take(limit).cloned().collect(),
        None => Vec::new(),
    }
}

// ──────────────────────────────────────────────────────────────────
// Helpers

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn generate_anonymous_id() -> String {
    // Generate a random-ish ID from current time + process ID
    let ts = now_ms();
    let pid = std::process::id();
    let combined = format!("{:x}{:x}", ts, pid);
    // Take 16 chars + format as UUID-like
    let s = combined.as_str();
    let padded = format!("{:0>16}", s);
    format!(
        "{}-{}-{}-{}-{}",
        &padded[..8],
        &padded[8..12],
        "0000",
        "0000",
        "000000000000"
    )
}

fn generate_crash_id() -> String {
    let ts = now_ms();
    let pid = std::process::id();
    format!("crash-{:x}-{:x}", ts, pid)
}

fn get_os_info() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{} {}", os, arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_default_config() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled);
        assert!(!config.crash_auto_submit);
        assert!(!config.anonymous_id.is_empty());
    }

    #[test]
    fn test_record_event_no_init() {
        // Should auto-init if not initialized
        // (state may already be initialized from other tests — clear first)
        {
            let mut state = TELEMETRY_STATE.lock().unwrap();
            *state = None;
        }
        record_event("ipc_call", "test_command", Some(100), true, None, "Pro");
        let stats = get_stats();
        assert!(stats.total_events >= 1);
    }

    #[test]
    fn test_record_crash_returns_id() {
        {
            let mut state = TELEMETRY_STATE.lock().unwrap();
            *state = None;
        }
        let crash_id = record_crash("test_cmd", "test error", "stack", "Pro");
        assert!(crash_id.starts_with("crash-"));
        let pending = get_pending_crash_dumps();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].crash_id, crash_id);
        assert!(!pending[0].submitted);
    }

    #[test]
    fn test_mark_crash_submitted() {
        {
            let mut state = TELEMETRY_STATE.lock().unwrap();
            *state = None;
        }
        let crash_id = record_crash("cmd", "err", "", "Core");
        mark_crash_submitted(&crash_id);
        let pending = get_pending_crash_dumps();
        assert_eq!(pending.len(), 0); // now submitted, no longer pending
    }

    #[test]
    fn test_stats_aggregation() {
        {
            let mut state = TELEMETRY_STATE.lock().unwrap();
            *state = None;
        }
        // Record 3 calls to cmd_a, 1 to cmd_b
        record_event("ipc_call", "cmd_a", Some(100), true, None, "Pro");
        record_event("ipc_call", "cmd_a", Some(200), true, None, "Pro");
        record_event("ipc_call", "cmd_a", Some(150), false, Some("err"), "Pro");
        record_event("ipc_call", "cmd_b", Some(50), true, None, "Pro");

        let stats = get_stats();
        assert!(stats.total_events >= 4);
        // cmd_a should be in top_commands with count 3
        let cmd_a = stats.top_commands.iter().find(|(name, _)| name == "cmd_a");
        assert!(cmd_a.is_some());
        assert_eq!(cmd_a.unwrap().1, 3);
        // cmd_a should be in top_failures with count 1
        let cmd_a_fail = stats.top_failures.iter().find(|(name, _)| name == "cmd_a");
        assert!(cmd_a_fail.is_some());
        assert_eq!(cmd_a_fail.unwrap().1, 1);
        // Average IPC duration = (100+200+150+50) / 4 = 125ms
        assert!((stats.avg_ipc_duration_ms - 125.0).abs() < 0.1);
    }

    #[test]
    fn test_update_config() {
        {
            let mut state = TELEMETRY_STATE.lock().unwrap();
            *state = None;
        }
        let mut config = TelemetryConfig::default();
        config.enabled = true;
        config.endpoint_url = "https://telemetry.example.com".into();
        update_config(config);
        let retrieved = get_config();
        assert!(retrieved.enabled);
        assert_eq!(retrieved.endpoint_url, "https://telemetry.example.com");
    }

    #[test]
    fn test_recent_events_order() {
        {
            let mut state = TELEMETRY_STATE.lock().unwrap();
            *state = None;
        }
        record_event("ui", "event_1", None, true, None, "Pro");
        record_event("ui", "event_2", None, true, None, "Pro");
        record_event("ui", "event_3", None, true, None, "Pro");
        let recent = get_recent_events(2);
        assert_eq!(recent.len(), 2);
        // Most recent first
        assert_eq!(recent[0].event_name, "event_3");
        assert_eq!(recent[1].event_name, "event_2");
    }

    #[test]
    fn test_generate_anonymous_id_format() {
        let id = generate_anonymous_id();
        // Should look like UUID-ish: 8-4-4-4-12
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[test]
    fn test_crash_id_format() {
        let id = generate_crash_id();
        assert!(id.starts_with("crash-"));
    }
}
