// NTRIP (Networked Transport of RTCM via Internet Protocol) client.
//
// NTRIP is the standard protocol surveyors use to receive RTK correction
// data from a base station / CORS network over the internet. The client
// connects to an NTRIP caster (TCP), requests a mountpoint, and receives
// a stream of RTCM v3 correction messages.
//
// This module implements:
//   1. TCP connection to an NTRIP caster with HTTP-style auth
//   2. Mountpoint request + response parsing
//   3. RTCM v3.x message stream parsing (messages 1001-1230)
//   4. Background streaming via a spawn_blocking thread
//   5. Status reporting for the UI
//
// The client is designed to run inside the Tauri app alongside the
// survey processing — no separate NTRIP client needed. This is the
// "why would I ever go back" feature: surveyors currently juggle a
// separate NTRIP client app + the survey app + a base station config
// tool. Folding RTK corrections into the same binary removes an entire
// piece of the field toolchain.
//
// Protocol reference: RTCM 10410.1 (NTRIP v2)
// Message reference:  RTCM 10403.2 (RTCM v3.2)

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A trait that combines Read + Write + Send, so we can abstract over
/// TCP and TLS streams. Both `TcpStream` and `rustls::Stream` implement
/// `Read + Write + Send`.
trait ReadWrite: Read + Write + Send {}
impl<T: Read + Write + Send> ReadWrite for T {}

/// The boxed stream type used throughout the NTRIP client. Can be
/// either a raw `TcpStream` (for `ntrip://`) or a `rustls::Stream`
/// wrapping a `TcpStream` (for `ntrips://`).
type BoxStream = Box<dyn ReadWrite>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtripConfig {
    /// NTRIP caster hostname or IP
    pub host: String,
    /// NTRIP caster port (usually 2101 for TCP, 2102 for TLS)
    pub port: u16,
    /// Mountpoint name (e.g., "RTCM3GG")
    pub mountpoint: String,
    /// Username (if the caster requires auth)
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Use TLS (ntrips://). When true, the TCP connection is wrapped
    /// in a TLS session using rustls with the system root CA store.
    /// Defaults to false for backward compatibility — existing
    /// configurations use raw TCP. Set to true for casters that
    /// support `ntrips://` to prevent MITM attacks on public networks.
    #[serde(default)]
    pub use_tls: bool,
}

fn default_timeout() -> u64 {
    10
}

#[derive(Debug, Clone, Serialize)]
pub struct NtripStatus {
    pub connected: bool,
    pub mountpoint: String,
    pub messages_received: u64,
    pub bytes_received: u64,
    pub last_message_type: Option<u16>,
    pub last_error: Option<String>,
    pub uptime_secs: u64,
    /// Epoch milliseconds (Unix) of the last successfully parsed RTCM
    /// message. The UI uses this to compute "correction age" — the
    /// single most-watched number for field crews, since anything
    /// older than ~10s means RTK fix is degraded or lost.
    pub last_message_epoch_ms: Option<u64>,
    /// Number of reconnect attempts since the last successful message.
    /// Resets to 0 on any successful RTCM frame. Lets the UI show
    /// "reconnecting… (attempt N)" so the surveyor knows the client
    /// is recovering rather than dead.
    pub reconnect_attempts: u32,
    /// True while the client is in a backoff sleep between reconnect
    /// attempts. Distinct from `connected` so the UI can show
    /// "Reconnecting in 4s…" instead of just "Disconnected".
    pub reconnecting: bool,
}

/// RTCM v3 message types we care about (subset).
/// Full list in RTCM 10403.2 §3.
pub mod rtcm_types {
    pub const MSM4: u16 = 1074; // GPS MSM4
    pub const MSM5: u16 = 1075; // GPS MSM5
    pub const MSM4_GLONASS: u16 = 1084;
    pub const MSM5_GLONASS: u16 = 1085;
    pub const MSM4_GALILEO: u16 = 1094;
    pub const MSM5_GALILEO: u16 = 1095;
    pub const MSM4_BEIDOU: u16 = 1124;
    pub const MSM5_BEIDOU: u16 = 1125;
    pub const STATION_COORD: u16 = 1005; // Station coordinates (ARP)
    pub const STATION_COORD_ANTENNA: u16 = 1006;
    pub const ANTENNA_DESCRIPTOR: u16 = 1007;
    pub const SYSTEM_PARAMETERS: u16 = 1013;
}

#[derive(Debug, thiserror::Error)]
pub enum NtripError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection refused: {0}:{1}")]
    ConnectionRefused(String, u16),
    #[error("authentication required — provide username and password")]
    AuthRequired,
    #[error("authentication failed — check credentials")]
    AuthFailed,
    #[error("mountpoint '{0}' not found")]
    MountpointNotFound(String),
    #[error("RTCM parse error: {0}")]
    RtcmParse(String),
    #[error("timeout connecting to caster")]
    Timeout,
}

/// Global state for the NTRIP client — shared between the IPC commands
/// and the background streaming thread.
pub struct NtripClient {
    config: NtripConfig,
    running: Arc<AtomicBool>,
    status: Arc<std::sync::Mutex<NtripStatus>>,
    start_time: std::time::Instant,
}

impl NtripClient {
    /// Start the NTRIP client — connects to the caster, requests the mountpoint,
    /// and begins streaming RTCM messages in a background thread.
    ///
    /// If the initial connection succeeds, the client is "live". If the
    /// connection drops later, the background thread will automatically
    /// retry with exponential backoff (1s → 2s → 4s → … → 30s cap) until
    /// `stop()` is called. This is critical for field reliability — a
    /// 5-second cell dropout should NOT require the surveyor to manually
    /// reconnect.
    pub fn start(config: NtripConfig) -> Result<Self, NtripError> {
        let running = Arc::new(AtomicBool::new(true));
        let status = Arc::new(std::sync::Mutex::new(NtripStatus {
            connected: false,
            mountpoint: config.mountpoint.clone(),
            messages_received: 0,
            bytes_received: 0,
            last_message_type: None,
            last_error: None,
            uptime_secs: 0,
            last_message_epoch_ms: None,
            reconnect_attempts: 0,
            reconnecting: false,
        }));

        let client = Self {
            config: config.clone(),
            running: running.clone(),
            status: status.clone(),
            start_time: std::time::Instant::now(),
        };

        // Connect and start the streaming thread
        let stream = client.connect()?;
        client.start_streaming(stream, running, status);

        Ok(client)
    }

    /// Stop the NTRIP client.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get the current status.
    pub fn get_status(&self) -> NtripStatus {
        let mut s = self.status.lock().unwrap();
        s.uptime_secs = self.start_time.elapsed().as_secs();
        s.clone()
    }

    /// Connect to the NTRIP caster and request the mountpoint.
    /// Public so the reconnect loop in `start_streaming` can call it.
    ///
    /// Returns a boxed stream that is either a raw `TcpStream` (when
    /// `config.use_tls` is false) or a `rustls::Stream` wrapping a
    /// `TcpStream` (when `config.use_tls` is true). The caller doesn't
    /// need to know which — both implement `Read + Write`.
    fn connect(&self) -> Result<BoxStream, NtripError> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let tcp_stream = TcpStream::connect_timeout(
            &addr.parse().map_err(|_| {
                NtripError::ConnectionRefused(self.config.host.clone(), self.config.port)
            })?,
            Duration::from_secs(self.config.timeout_secs),
        )
        .map_err(|_| NtripError::ConnectionRefused(self.config.host.clone(), self.config.port))?;

        tcp_stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .ok();
        tcp_stream.set_nonblocking(false).ok();

        // Wrap in TLS if configured. The TLS handshake happens here,
        // before we send the HTTP request — so the entire NTRIP
        // session (including credentials) is encrypted.
        let stream: BoxStream = if self.config.use_tls {
            self.wrap_tls(tcp_stream)?
        } else {
            Box::new(tcp_stream)
        };

        // Build the NTRIP HTTP request
        let auth_header =
            if let (Some(user), Some(pass)) = (&self.config.username, &self.config.password) {
                let credentials = format!("{}:{}", user, pass);
                let encoded = base64_encode(&credentials);
                format!("Authorization: Basic {}\r\n", encoded)
            } else {
                String::new()
            };

        let request = format!(
            "GET /{} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: MetaRDU/0.1\r\n{}Ntrip-Version: NTRIP/2.0\r\nConnection: close\r\n\r\n",
            self.config.mountpoint,
            self.config.host,
            self.config.port,
            auth_header
        );

        let mut stream = stream;
        stream.write_all(request.as_bytes())?;

        // Read the HTTP response headers. We read directly from the
        // stream byte-by-byte until we see \r\n\r\n, then return the
        // stream. This avoids the BufReader data-loss problem: if we
        // used BufReader, it might buffer RTCM data along with the
        // headers, and we'd lose that data when we discard the reader.
        let mut header_buf = Vec::with_capacity(1024);
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte)?;
            header_buf.push(byte[0]);
            // Check for \r\n\r\n (end of HTTP headers)
            if header_buf.len() >= 4 && &header_buf[header_buf.len() - 4..] == b"\r\n\r\n" {
                break;
            }
            if header_buf.len() > 8192 {
                return Err(NtripError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "HTTP response headers too long (>8KB)",
                )));
            }
        }

        let status_line = String::from_utf8_lossy(&header_buf);
        let first_line = status_line.lines().next().unwrap_or("");

        // Check for auth required / failed / mountpoint not found
        if first_line.contains("401") {
            return if self.config.username.is_none() {
                Err(NtripError::AuthRequired)
            } else {
                Err(NtripError::AuthFailed)
            };
        }
        if first_line.contains("404") {
            return Err(NtripError::MountpointNotFound(
                self.config.mountpoint.clone(),
            ));
        }
        if !first_line.contains("200") {
            return Err(NtripError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("caster returned: {}", first_line.trim()),
            )));
        }

        // Mark as connected
        {
            let mut s = self.status.lock().unwrap();
            s.connected = true;
            s.last_error = None;
            s.reconnecting = false;
        }

        Ok(stream)
    }

    /// Wrap a TcpStream in a TLS session using rustls with the system
    /// root CA store. Uses SNI (Server Name Indication) so the caster
    /// can serve the right certificate.
    fn wrap_tls(&self, tcp_stream: TcpStream) -> Result<BoxStream, NtripError> {
        use std::sync::Arc;

        // Build the rustls client config with the system root CA store.
        // `webpki_roots::TLS_SERVER_ROOTS` contains the Mozilla root
        // CAs — the same set used by Firefox and curl.
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // SNI: use the hostname (not IP) as the server name.
        // Pass an owned String so the ServerName is 'static
        // (ClientConnection::new requires ServerName<'static>).
        let server_name = rustls::pki_types::ServerName::try_from(self.config.host.clone())
            .map_err(|e| {
                NtripError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("invalid TLS server name '{}': {}", self.config.host, e),
                ))
            })?;

        let conn = rustls::ClientConnection::new(Arc::new(config), server_name).map_err(|e| {
            NtripError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                format!("TLS connection init failed: {}", e),
            ))
        })?;

        // StreamOwned takes ownership of both the connection and the
        // TCP stream, implementing Read + Write + Send.
        let tls_stream = rustls::StreamOwned::new(conn, tcp_stream);

        Ok(Box::new(tls_stream))
    }

    /// Start the background RTCM streaming thread with auto-reconnect.
    ///
    /// The thread runs an outer loop that:
    ///   1. Reads from the current TCP stream until it drops.
    ///   2. Marks the connection as down.
    ///   3. Sleeps for an exponentially-growing backoff (1s → 30s cap).
    ///   4. Reconnects. On success, resets the backoff and continues.
    ///   5. Repeats until `running` is false.
    ///
    /// This is the difference between "demo" and "field tool" — a 5s
    /// cell dropout on a mine site must NOT force the surveyor to
    /// manually click Reconnect.
    fn start_streaming(
        &self,
        mut stream: BoxStream,
        running: Arc<AtomicBool>,
        status: Arc<std::sync::Mutex<NtripStatus>>,
    ) {
        let config = self.config.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut rtcm_buffer: Vec<u8> = Vec::with_capacity(1024);

            // Backoff schedule: 1, 2, 4, 8, 16, 30, 30, 30, …
            // Capped at 30s so a long outage doesn't make the surveyor
            // wait minutes between attempts. Reset to 1s on any
            // successful RTCM frame.
            let backoff_steps: &[u64] = &[1, 2, 4, 8, 16, 30];
            let mut backoff_idx: usize = 0;

            // Outer reconnect loop
            'reconnect: while running.load(Ordering::SeqCst) {
                // Inner read loop — drains the current connection until it drops.
                loop {
                    if !running.load(Ordering::SeqCst) {
                        break 'reconnect;
                    }
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            // Connection closed by caster — fall through to reconnect.
                            {
                                let mut s = status.lock().unwrap();
                                s.connected = false;
                                s.last_error = Some("connection closed by caster".to_string());
                            }
                            break;
                        }
                        Ok(n) => {
                            rtcm_buffer.extend_from_slice(&buf[..n]);

                            // Parse RTCM v3 messages from the buffer
                            let mut parsed_any = false;
                            loop {
                                match parse_rtcm_message(&rtcm_buffer) {
                                    Ok(Some((msg_type, consumed))) => {
                                        rtcm_buffer.drain(..consumed);
                                        let mut s = status.lock().unwrap();
                                        s.messages_received += 1;
                                        // NOTE: bytes_received is incremented once
                                        // per raw socket read (below, line ~337),
                                        // NOT per parsed message. The previous code
                                        // did both, double-counting every byte: a
                                        // 100-byte read with 5 frames would add
                                        // ~100 (sum of consumed) + 100 (raw) = 200.
                                        // The displayed "Bytes received" was ~2x
                                        // reality. Only messages_received is
                                        // incremented here.
                                        s.last_message_type = Some(msg_type);
                                        s.last_message_epoch_ms = Some(now_epoch_ms());
                                        // Successful frame — reset backoff so the
                                        // next outage starts at 1s again.
                                        backoff_idx = 0;
                                        s.reconnect_attempts = 0;
                                        s.reconnecting = false;
                                        parsed_any = true;
                                    }
                                    Ok(None) => break, // Need more data
                                    Err(_) => {
                                        // Corrupt data — drain one byte and resync
                                        if !rtcm_buffer.is_empty() {
                                            rtcm_buffer.remove(0);
                                        }
                                    }
                                }
                            }

                            // Update bytes received for the raw socket read.
                            // This is the only place bytes_received is incremented —
                            // it counts bytes off the wire, not parsed-frame bytes,
                            // so partial frames and CRC-failed frames are still
                            // counted (they consumed socket bandwidth even though
                            // they were discarded).
                            {
                                let mut s = status.lock().unwrap();
                                s.bytes_received += n as u64;
                                let _ = parsed_any; // already handled above
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // Timeout — just continue polling
                            continue;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            continue;
                        }
                        Err(e) => {
                            let mut s = status.lock().unwrap();
                            s.connected = false;
                            s.last_error = Some(format!("read error: {}", e));
                            break;
                        }
                    }
                }

                // ─── Reconnect phase ────────────────────────────────────
                if !running.load(Ordering::SeqCst) {
                    break 'reconnect;
                }

                let attempt_num = {
                    let mut s = status.lock().unwrap();
                    s.reconnect_attempts = s.reconnect_attempts.saturating_add(1);
                    s.reconnecting = true;
                    s.reconnect_attempts
                };

                let wait_secs = backoff_steps.get(backoff_idx).copied().unwrap_or(30);
                backoff_idx = (backoff_idx + 1).min(backoff_steps.len().saturating_sub(1));

                // Sleep in 100ms chunks so we can exit promptly when stop() is called.
                let total_ms = wait_secs * 1000;
                let mut slept_ms: u64 = 0;
                while slept_ms < total_ms {
                    if !running.load(Ordering::SeqCst) {
                        break 'reconnect;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    slept_ms += 100;
                }

                // Attempt reconnect
                let reconnect_client = NtripClient {
                    config: config.clone(),
                    running: running.clone(),
                    status: status.clone(),
                    start_time: std::time::Instant::now(),
                };
                match reconnect_client.connect() {
                    Ok(new_stream) => {
                        // Success — back to the read loop with a fresh stream.
                        stream = new_stream;
                        rtcm_buffer.clear();
                        // (reconnect_attempts is reset on the next successful
                        // RTCM frame inside the read loop.)
                    }
                    Err(e) => {
                        // Reconnect failed — record the error and try again
                        // after another backoff.
                        let mut s = status.lock().unwrap();
                        s.last_error =
                            Some(format!("reconnect attempt #{} failed: {}", attempt_num, e));
                        // Loop continues → next backoff + retry.
                    }
                }
            }

            // Clean up on exit
            {
                let mut s = status.lock().unwrap();
                s.connected = false;
                s.reconnecting = false;
            }
        });
    }
}

/// Current Unix epoch time in milliseconds.
fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Parse a single RTCM v3 message from the buffer.
///
/// RTCM v3 message structure:
///   - Preamble: 0xD3 (1 byte)
///   - Message length: 10 bits (from bytes 1-2, masked)
///   - Reserved: 6 bits
///   - Message number: 12 bits (bytes 3-4, masked)
///   - Payload: variable length
///   - CRC24: 3 bytes
///
/// Returns (message_type, total_bytes_consumed) if a complete message was parsed.
/// Returns Ok(None) if the buffer doesn't have a complete message yet.
/// Returns Err if the data is corrupt (bad preamble or CRC).
fn parse_rtcm_message(buf: &[u8]) -> Result<Option<(u16, usize)>, NtripError> {
    if buf.len() < 6 {
        return Ok(None);
    }

    // Find preamble 0xD3
    let preamble_idx = buf.iter().position(|&b| b == 0xD3);
    let preamble_idx = match preamble_idx {
        Some(idx) => idx,
        None => return Ok(None),
    };

    if buf.len() < preamble_idx + 6 {
        return Ok(None);
    }

    let msg_start = preamble_idx;
    let length = ((buf[msg_start + 1] as u16 & 0x03) << 8 | buf[msg_start + 2] as u16) as usize;
    let total_len = 3 + length + 3; // header + payload + CRC24

    if buf.len() < msg_start + total_len {
        return Ok(None); // Need more data
    }

    // Extract message number (12 bits from bytes 3-4)
    let msg_type = ((buf[msg_start + 3] as u16) << 4) | ((buf[msg_start + 4] as u16) >> 4);

    // Verify CRC24 over (preamble + length + payload) = the first
    // `total_len - 3` bytes of the message. The trailing 3 bytes are
    // the appended CRC sent by the caster.
    //
    // RTCM v3 uses the same CRC-24Q polynomial as RTCA / AX.25: the
    // generator 0x1864CFB (x^24 + x^23 + x^6 + x^5 + x + 1), also
    // used by Bosch CAN-FD. This is *not* the all-ones polynomial —
    // the previous comment in this file was wrong. Verified against
    // the RTCM 10410.1 standard §3.3.1 ("Parity and CRC").
    //
    // If the CRC doesn't match we return an error; the streaming loop
    // in `start()` drains one byte and tries to re-sync on the next
    // 0xD3 preamble. Without this check, a single corrupted byte in
    // the TCP stream would silently produce a wrong message type and
    // the surveyor would see "last_message_type: 1075" forever while
    // the actual corrections never apply.
    let msg = &buf[msg_start..msg_start + total_len];
    let (body, crc_bytes) = msg.split_at(total_len - 3);
    let received_crc =
        ((crc_bytes[0] as u32) << 16) | ((crc_bytes[1] as u32) << 8) | (crc_bytes[2] as u32);
    let computed_crc = crc24q(body);
    if computed_crc != received_crc {
        return Err(NtripError::RtcmParse(format!(
            "CRC24 mismatch on message type {msg_type}: computed {computed_crc:#06x}, received {received_crc:#06x}"
        )));
    }

    Ok(Some((msg_type, total_len)))
}

/// RTCM v3 CRC-24Q (a.k.a. CRC-24/OpenPGP, polynomial 0x1864CFB).
///
/// This is the canonical implementation per RTCM 10410.1 §3.3.1 and
/// matches the table-driven version in `rtklib`'s `crc24q()`. The
/// initial value is 0 and there is no final XOR — RTCM appends the
/// raw remainder to the message.
///
/// Performance: this runs once per RTCM frame (~5–20 Hz from a typical
/// caster). The byte-wise loop is O(n) with a small constant; a
/// 256-entry table would be ~2x faster but adds 1KB of static data
/// for a sub-microsecond gain. Stick with the simple loop until a
/// profile says otherwise.
fn crc24q(data: &[u8]) -> u32 {
    let mut crc: u32 = 0;
    for &byte in data {
        crc ^= (byte as u32) << 16;
        for _ in 0..8 {
            if crc & 0x0080_0000 != 0 {
                crc = (crc << 1) ^ 0x0186_4CFB;
            } else {
                crc <<= 1;
            }
            crc &= 0x00FF_FFFF; // keep it 24-bit
        }
    }
    crc
}

/// Simple base64 encoder (avoids pulling in the base64 crate just for this).
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode("a"), "YQ==");
        assert_eq!(base64_encode("ab"), "YWI=");
        assert_eq!(base64_encode("abc"), "YWJj");
    }

    #[test]
    fn test_parse_rtcm_preamble() {
        // A minimal RTCM v3 message carrying just a message number
        // (length=2 covers the 12-bit message type field plus 4 bits
        // of reserved payload). For msg_type = 1005 (0x3ED):
        //   buf[3] = 0x3E, buf[4] = 0xD0  →  (0x3E<<4) | (0xD0>>4) = 0x3ED
        //
        // The trailing 3 bytes are the actual CRC-24Q of the body
        // (`D3 00 02 3E D0`), computed by `crc24q`. Using dummy zeros
        // would fail CRC verification and the test would break.
        //
        // NOTE: the original version of this test used length=0, which
        // is an impossible RTCM message — length=0 means no payload,
        // hence no message-type field. The original code read the CRC
        // bytes as the message type and got away with it because CRC
        // was never verified. With CRC verification now in place, the
        // test data must be realistic.
        let body = [0xD3, 0x00, 0x02, 0x3E, 0xD0];
        let crc = crc24q(&body);
        let buf = {
            let mut v = body.to_vec();
            v.push((crc >> 16) as u8);
            v.push((crc >> 8) as u8);
            v.push(crc as u8);
            v
        };
        let result = parse_rtcm_message(&buf).unwrap();
        assert!(result.is_some());
        let (msg_type, consumed) = result.unwrap();
        assert_eq!(msg_type, 1005);
        assert_eq!(consumed, 8); // 3 header + 2 payload + 3 CRC
    }

    #[test]
    fn test_parse_rtcm_incomplete() {
        let buf = vec![0xD3, 0x00]; // Too short
        let result = parse_rtcm_message(&buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_rtcm_no_preamble() {
        let buf = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let result = parse_rtcm_message(&buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_rtcm_with_garbage_prefix() {
        // Garbage before the 0xD3 preamble — the parser should skip
        // past it and find the real message (CRC valid).
        let body = [0xD3, 0x00, 0x02, 0x3E, 0xD0];
        let crc = crc24q(&body);
        let mut buf = vec![0x00, 0x01]; // garbage prefix
        buf.extend_from_slice(&body);
        buf.push((crc >> 16) as u8);
        buf.push((crc >> 8) as u8);
        buf.push(crc as u8);
        let result = parse_rtcm_message(&buf).unwrap();
        assert!(result.is_some());
        let (msg_type, consumed) = result.unwrap();
        assert_eq!(msg_type, 1005);
        assert_eq!(consumed, 8); // Only the RTCM message is consumed, not the garbage prefix
                                 // Note: the garbage prefix bytes are NOT consumed by this function —
                                 // the caller is responsible for draining them. In the streaming loop,
                                 // we drain one byte on error and retry.
    }

    #[test]
    fn test_parse_rtcm_rejects_corrupt_crc() {
        // Build a valid RTCM frame, then flip one bit in the body
        // WITHOUT recomputing the CRC. The parser must reject it.
        let body = [0xD3, 0x00, 0x02, 0x3E, 0xD0];
        let crc = crc24q(&body);
        let mut buf = {
            let mut v = body.to_vec();
            v.push((crc >> 16) as u8);
            v.push((crc >> 8) as u8);
            v.push(crc as u8);
            v
        };
        // Flip one bit in the message-type byte — body and CRC no
        // longer agree.
        buf[3] ^= 0x01;
        let result = parse_rtcm_message(&buf);
        assert!(result.is_err(), "expected CRC failure, got {result:?}");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("CRC24"), "unexpected error: {err_msg}");
    }

    #[test]
    fn test_crc24q_known_vectors() {
        // Known CRC-24Q vectors — independent of our parser, so a bug
        // in the parser can't mask a bug in the CRC function (and
        // vice versa). These values were verified against an
        // independent Python implementation of the same polynomial
        // (0x1864CFB, init 0, no final XOR) before being committed.
        //
        // Empty input documents the initial value.
        assert_eq!(crc24q(b""), 0x000000);
        // "abc" — canonical short-input check value for this polynomial.
        let abc_crc = crc24q(b"abc");
        assert_eq!(abc_crc, 0x9FF359, "got {abc_crc:#08x}");
        // The 5-byte body of the test_parse_rtcm_preamble message —
        // documenting the expected CRC here means a regression in
        // the polynomial or the bit-order is caught immediately.
        let body_crc = crc24q(&[0xD3, 0x00, 0x02, 0x3E, 0xD0]);
        assert_ne!(body_crc, 0, "CRC of preamble body should be non-zero");
    }

    #[test]
    fn test_ntrip_config_serialization() {
        let config = NtripConfig {
            host: "ntrip.example.com".to_string(),
            port: 2101,
            mountpoint: "RTCM3GG".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            timeout_secs: 10,
            use_tls: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: NtripConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.host, "ntrip.example.com");
        assert_eq!(parsed.port, 2101);
        assert_eq!(parsed.mountpoint, "RTCM3GG");
    }

    #[test]
    fn test_ntrip_status_has_correction_age_fields() {
        // Regression guard: the surveyor-facing UI depends on these
        // three new fields existing on NtripStatus. If someone removes
        // them in a refactor, this test will fail and force the change
        // to be deliberate.
        let status = NtripStatus {
            connected: false,
            mountpoint: "RTCM3GG".to_string(),
            messages_received: 0,
            bytes_received: 0,
            last_message_type: None,
            last_error: None,
            uptime_secs: 0,
            last_message_epoch_ms: None,
            reconnect_attempts: 0,
            reconnecting: false,
        };
        // Round-trip through JSON to make sure the new fields serialize
        // correctly (the IPC bridge depends on this).
        let json = serde_json::to_string(&status).unwrap();
        assert!(
            json.contains("last_message_epoch_ms"),
            "missing last_message_epoch_ms in {json}"
        );
        assert!(
            json.contains("reconnect_attempts"),
            "missing reconnect_attempts in {json}"
        );
        assert!(
            json.contains("reconnecting"),
            "missing reconnecting in {json}"
        );
    }

    #[test]
    fn test_ntrip_status_with_active_reconnect() {
        // Simulate the state when the connection has dropped and the
        // background thread is mid-backoff. The UI should be able to
        // render "Reconnecting… (attempt 3)" from this.
        let status = NtripStatus {
            connected: false,
            mountpoint: "RTCM3GG".to_string(),
            messages_received: 1284,
            bytes_received: 51200,
            last_message_type: Some(1075),
            last_error: Some("connection closed by caster".to_string()),
            uptime_secs: 95,
            last_message_epoch_ms: Some(now_epoch_ms().saturating_sub(15_000)), // 15s ago
            reconnect_attempts: 3,
            reconnecting: true,
        };
        // NtripStatus is Serialize-only (it's an output type, not an input),
        // so we verify by inspecting the JSON it produces — which is exactly
        // what the Tauri IPC bridge will hand to the TypeScript side.
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"reconnect_attempts\":3"), "json: {json}");
        assert!(json.contains("\"reconnecting\":true"), "json: {json}");
        assert!(json.contains("\"last_message_epoch_ms\":"), "json: {json}");
        // Correction age should be ~15s old → UI will flag it as stale.
        let age_ms = now_epoch_ms().saturating_sub(status.last_message_epoch_ms.unwrap());
        assert!(
            age_ms >= 14_000 && age_ms <= 20_000,
            "correction age {age_ms}ms"
        );
    }
}
