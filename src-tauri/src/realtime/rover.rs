// RTK rover TCP client — streams NMEA sentences from a serial-to-TCP
// bridge (typical Windows field setup: com2tcp) or a direct TCP port
// from a base station. Decodes each sentence and updates a shared
// position + 60-second trail.
//
// The frontend polls `get_rover_position` and `get_rover_trail` via
// IPC at 5 Hz — fast enough for smooth map movement, slow enough to
// not thrash the IPC bridge.

use crate::realtime::nmea::{merge_into_position, parse_sentence};
use crate::realtime::RoverPosition;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The shared state of the rover stream — latest position + trail +
/// connection status. Cloned cheaply (Arc inside).
#[derive(Clone)]
pub struct RoverState {
    inner: Arc<Mutex<RoverStateInner>>,
    running: Arc<AtomicBool>,
}

struct RoverStateInner {
    /// Latest merged position
    position: RoverPosition,
    /// Position trail — last N positions, oldest first
    trail: Vec<RoverPosition>,
    /// Max trail length (default 300 = 60 seconds at 5 Hz)
    trail_max: usize,
    /// Connection status
    connected: bool,
    /// Last error message (None if no error)
    last_error: Option<String>,
    /// Total sentences parsed
    sentences_parsed: u64,
    /// Sentences with invalid checksum or malformed payload
    sentences_rejected: u64,
}

impl Default for RoverStateInner {
    fn default() -> Self {
        Self {
            position: RoverPosition::default(),
            trail: Vec::new(),
            trail_max: 300,
            connected: false,
            last_error: None,
            sentences_parsed: 0,
            sentences_rejected: 0,
        }
    }
}

impl RoverState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RoverStateInner::default())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start streaming from a TCP source. Spawns a background thread
    /// that reads NMEA sentences and updates the shared state.
    ///
    /// `host:port` is the TCP endpoint. Returns an error string if the
    /// connection can't be opened. If a stream is already running,
    /// returns Ok(()) without starting a new one.
    pub fn start(&self, host: String, port: u16) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect_timeout(
            &addr.parse().map_err(|e: std::net::AddrParseError| e.to_string())?,
            Duration::from_secs(5),
        )
        .map_err(|e| format!("connecting to {}: {}", addr, e))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| e.to_string())?;

        self.running.store(true, Ordering::SeqCst);
        {
            let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
            inner.connected = true;
            inner.last_error = None;
        }

        let inner = Arc::clone(&self.inner);
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                match line {
                    Ok(line) => {
                        if let Some(sentence) = parse_sentence(&line) {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64();
                            let mut pos_guard = match inner.lock() {
                                Ok(g) => g,
                                Err(_) => break,
                            };
                            let mut pos = pos_guard.position.clone();
                            pos.timestamp = now;
                            merge_into_position(&sentence, &mut pos);
                            pos_guard.position = pos.clone();
                            pos_guard.sentences_parsed += 1;

                            // Push to trail (drop oldest if over max)
                            pos_guard.trail.push(pos);
                            if pos_guard.trail.len() > pos_guard.trail_max {
                                pos_guard.trail.remove(0);
                            }
                        } else {
                            if let Ok(mut g) = inner.lock() {
                                g.sentences_rejected += 1;
                            }
                        }
                    }
                    Err(e) => {
                        if let Ok(mut g) = inner.lock() {
                            g.connected = false;
                            g.last_error = Some(format!("read error: {}", e));
                        }
                        break;
                    }
                }
            }

            // Stream ended
            if let Ok(mut g) = inner.lock() {
                g.connected = false;
            }
            running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// Stop the stream. Idempotent.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut g) = self.inner.lock() {
            g.connected = false;
        }
    }

    /// Get the latest position (cloned). Returns a default
    /// `RoverPosition` if no fix has been received yet.
    pub fn position(&self) -> RoverPosition {
        self.inner
            .lock()
            .map(|g| g.position.clone())
            .unwrap_or_default()
    }

    /// Get the position trail (oldest first).
    pub fn trail(&self) -> Vec<RoverPosition> {
        self.inner
            .lock()
            .map(|g| g.trail.clone())
            .unwrap_or_default()
    }

    /// Get connection status and counters.
    pub fn status(&self) -> RoverStatus {
        self.inner
            .lock()
            .map(|g| RoverStatus {
                connected: g.connected,
                last_error: g.last_error.clone(),
                sentences_parsed: g.sentences_parsed,
                sentences_rejected: g.sentences_rejected,
                is_running: self.running.load(Ordering::SeqCst),
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RoverStatus {
    pub connected: bool,
    pub is_running: bool,
    pub last_error: Option<String>,
    pub sentences_parsed: u64,
    pub sentences_rejected: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_lifecycle() {
        let state = RoverState::new();
        assert!(!state.status().is_running);
        // Start with a non-routable address — should fail fast
        let _ = state.start("127.0.0.1".to_string(), 1);
        // Either failed to connect (likely) or started briefly
        state.stop();
        assert!(!state.status().is_running);
    }

    #[test]
    fn test_default_trail_empty() {
        let state = RoverState::new();
        assert!(state.trail().is_empty());
        assert!(state.position().latitude.is_none());
    }
}
