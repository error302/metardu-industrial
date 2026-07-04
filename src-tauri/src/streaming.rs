// Real-time streaming ingest — Phase 4.
//
// Listens for real-time multibeam data via UDP and processes it
// on-the-fly. The surveyor sees a live bathymetric surface build
// on the map during acquisition.
//
// Protocol: Kongsberg KMBinary or simple JSON over UDP.
// The listener runs in a tokio task, buffers pings, and emits
// 'stream://ping' events for the frontend to render.
//
// Phase 4 scaffold: defines the listener, buffer, and event protocol.
// Actual datagram parsing (SISO/KEU/KMBinary) is a Phase 4+ task.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use tauri::Emitter;
use tokio::net::UdpSocket;

/// Configuration for the streaming ingest listener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// UDP port to listen on (default 4000)
    #[serde(default = "default_port")]
    pub port: u16,
    /// Maximum buffer size before flushing to the frontend
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    /// Flush interval in milliseconds
    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,
    /// Expected data format
    #[serde(default)]
    pub format: StreamFormat,
}

fn default_port() -> u16 {
    4000
}
fn default_buffer_size() -> usize {
    1000
}
fn default_flush_interval() -> u64 {
    500
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            buffer_size: default_buffer_size(),
            flush_interval_ms: default_flush_interval(),
            format: StreamFormat::Json,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamFormat {
    /// Simple JSON ping: {"x":..., "y":..., "depth":..., "uncertainty":...}
    Json,
    /// Kongsberg KMBinary (Phase 4+)
    KMBinary,
    /// Raw bytes with custom parser
    Raw,
}

impl Default for StreamFormat {
    fn default() -> Self {
        StreamFormat::Json
    }
}

/// A single streaming ping received from the sonar.
#[derive(Debug, Clone, Serialize)]
pub struct StreamPing {
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    pub uncertainty: f64,
    pub timestamp: u64, // ms since listener start
}

/// Streaming listener state.
pub struct StreamState {
    pub config: StreamConfig,
    pub is_running: bool,
    pub pings_received: u64,
    pub pings_buffered: usize,
    pub bytes_received: u64,
    pub start_time: Option<Instant>,
    pub last_error: Option<String>,
}

impl StreamState {
    pub fn new() -> Self {
        Self {
            config: StreamConfig::default(),
            is_running: false,
            pings_received: 0,
            pings_buffered: 0,
            bytes_received: 0,
            start_time: None,
            last_error: None,
        }
    }
}

impl Default for StreamState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global stream state.
pub fn global_stream_state() -> &'static Mutex<StreamState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<StreamState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(StreamState::new()))
}

/// Start the UDP streaming listener. Runs in a background tokio task.
///
/// Pings are buffered and flushed every `flush_interval_ms` via
/// `stream://pings` Tauri events.
pub async fn start_stream_listener(
    app: tauri::AppHandle,
    config: StreamConfig,
) -> Result<(), String> {
    {
        let mut state = global_stream_state().lock().map_err(|e| e.to_string())?;
        if state.is_running {
            return Err("stream listener already running".into());
        }
        state.is_running = true;
        state.config = config.clone();
        state.start_time = Some(Instant::now());
        state.pings_received = 0;
        state.bytes_received = 0;
    }

    let port = config.port;
    let sock = UdpSocket::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|e| format!("failed to bind UDP port {port}: {e}"))?;

    let buffer_size = config.buffer_size;
    let flush_interval = std::time::Duration::from_millis(config.flush_interval_ms);

    // Spawn the listener task
    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        let mut ping_buffer: Vec<StreamPing> = Vec::with_capacity(buffer_size);
        let mut last_flush = Instant::now();
        let start = Instant::now();

        loop {
            // Check if we should stop
            {
                let state = global_stream_state().lock();
                if let Ok(state) = state {
                    if !state.is_running {
                        break;
                    }
                }
            }

            // Try to receive with a short timeout so we can flush periodically
            let recv_result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                sock.recv_from(&mut buf),
            )
            .await;

            if let Ok(Ok((len, _addr))) = recv_result {
                // Update byte count
                {
                    let state = global_stream_state().lock();
                    if let Ok(mut state) = state {
                        state.bytes_received += len as u64;
                    }
                }

                // Parse the datagram
                let data = &buf[..len];
                if let Some(ping) = parse_json_ping(data, start) {
                    ping_buffer.push(ping);
                    {
                        let state = global_stream_state().lock();
                        if let Ok(mut state) = state {
                            state.pings_received += 1;
                            state.pings_buffered = ping_buffer.len();
                        }
                    }
                }
            }

            // Flush if buffer is full or interval has elapsed
            if ping_buffer.len() >= buffer_size
                || (last_flush.elapsed() >= flush_interval && !ping_buffer.is_empty())
            {
                let _ = app.emit("stream://pings", &ping_buffer);
                ping_buffer.clear();
                last_flush = Instant::now();

                let state = global_stream_state().lock();
                if let Ok(mut state) = state {
                    state.pings_buffered = 0;
                }
            }
        }
    });

    Ok(())
}

/// Stop the streaming listener.
pub fn stop_stream_listener() -> Result<(), String> {
    let mut state = global_stream_state().lock().map_err(|e| e.to_string())?;
    state.is_running = false;
    Ok(())
}

/// Get current stream status.
pub fn get_stream_status() -> Result<StreamStatus, String> {
    let state = global_stream_state().lock().map_err(|e| e.to_string())?;
    let elapsed = state
        .start_time
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);
    let rate = if elapsed > 0.0 {
        state.pings_received as f64 / elapsed
    } else {
        0.0
    };
    Ok(StreamStatus {
        is_running: state.is_running,
        pings_received: state.pings_received,
        pings_buffered: state.pings_buffered,
        bytes_received: state.bytes_received,
        elapsed_seconds: elapsed,
        pings_per_second: rate,
        last_error: state.last_error.clone(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamStatus {
    pub is_running: bool,
    pub pings_received: u64,
    pub pings_buffered: usize,
    pub bytes_received: u64,
    pub elapsed_seconds: f64,
    pub pings_per_second: f64,
    pub last_error: Option<String>,
}

/// Parse a JSON ping datagram.
fn parse_json_ping(data: &[u8], start: Instant) -> Option<StreamPing> {
    let json: serde_json::Value = serde_json::from_slice(data).ok()?;
    Some(StreamPing {
        x: json.get("x")?.as_f64()?,
        y: json.get("y")?.as_f64()?,
        depth: json.get("depth")?.as_f64()?,
        uncertainty: json.get("uncertainty")?.as_f64().unwrap_or(0.1),
        timestamp: start.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let data = br#"{"x": 130.8, "y": -12.3, "depth": 25.0, "uncertainty": 0.1}"#;
        let start = Instant::now();
        let ping = parse_json_ping(data, start);
        assert!(ping.is_some());
        let ping = ping.unwrap();
        assert!((ping.x - 130.8).abs() < 0.01);
        assert!((ping.depth - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_invalid_json() {
        let data = b"not json";
        let start = Instant::now();
        let ping = parse_json_ping(data, start);
        assert!(ping.is_none());
    }

    #[test]
    fn test_stream_state_lifecycle() {
        let mut state = StreamState::new();
        assert!(!state.is_running);
        state.is_running = true;
        state.pings_received = 100;
        assert!(state.is_running);
        assert_eq!(state.pings_received, 100);
    }
}
