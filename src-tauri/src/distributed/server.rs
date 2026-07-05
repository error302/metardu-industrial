// Coordinator TCP server — accepts worker connections, dispatches
// work chunks, and collects results.
//
// Protocol (line-delimited JSON over TCP):
//   Worker → Coordinator: {"type":"connect","worker_id":"..."}
//   Coordinator → Worker: {"type":"dispatch","chunk":{...}}
//   Worker → Coordinator: {"type":"result","result":{...}}
//   Coordinator → Worker: {"type":"idle"}
//   Coordinator → Worker: {"type":"shutdown"}

use crate::distributed::{global_coordinator, WorkChunk, WorkResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkerMessage {
    Connect { worker_id: String },
    Result { result: WorkResult },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CoordinatorMessage {
    Dispatch { chunk: WorkChunk },
    Idle,
    Shutdown,
}

pub struct ServerState {
    pub is_running: bool,
    pub port: u16,
    pub workers_connected: usize,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            is_running: false,
            port: 9753,
            workers_connected: 0,
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn global_server_state() -> &'static Mutex<ServerState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<ServerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ServerState::new()))
}

pub async fn start_coordinator_server(app: AppHandle, port: u16) -> Result<(), String> {
    // Security: bind to 127.0.0.1 only. The previous code bound to
    // 0.0.0.0 (all interfaces) with no auth — anyone on the LAN could
    // connect as a "worker", receive work chunks (which may contain
    // proprietary mine/marine data), and inject false results. Loopback
    // binding keeps the coordinator private to the local machine. If
    // a real distributed deployment is needed, add TLS + a pre-shared
    // worker token — never expose this on 0.0.0.0 unauthenticated.
    //
    // Also fix correctness: the previous code set is_running=true
    // BEFORE attempting the bind. If bind failed (port in use), the
    // state was stuck "running" with no actual listener — the user
    // was locked out until app restart. Now we bind first and only
    // flip is_running after a successful bind.
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| format!("failed to bind TCP port {port} on 127.0.0.1: {e}"))?;

    {
        let mut state = global_server_state().lock().map_err(|e| e.to_string())?;
        if state.is_running {
            return Err("coordinator server already running".into());
        }
        state.is_running = true;
        state.port = port;
    }

    tokio::spawn(async move {
        loop {
            {
                let state = global_server_state().lock();
                if let Ok(state) = state {
                    if !state.is_running {
                        break;
                    }
                }
            }

            let accept_result =
                tokio::time::timeout(std::time::Duration::from_millis(200), listener.accept())
                    .await;

            if let Ok(Ok((stream, addr))) = accept_result {
                {
                    let state = global_server_state().lock();
                    if let Ok(mut state) = state {
                        state.workers_connected += 1;
                    }
                }
                let app = app.clone();
                tokio::spawn(handle_worker_connection(stream, addr, app));
            }
        }
    });

    Ok(())
}

pub fn stop_coordinator_server() -> Result<(), String> {
    let mut state = global_server_state().lock().map_err(|e| e.to_string())?;
    state.is_running = false;
    Ok(())
}

async fn handle_worker_connection(stream: TcpStream, addr: std::net::SocketAddr, app: AppHandle) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line_buf = String::new();
    let worker_id = format!("worker_{addr}");

    loop {
        line_buf.clear();
        match reader.read_line(&mut line_buf).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<WorkerMessage>(line_buf.trim()) {
                    match msg {
                        WorkerMessage::Connect { worker_id: wid } => {
                            let _ = app.emit(
                                "distributed://progress",
                                serde_json::json!({
                                    "status": "worker_connected",
                                    "worker_id": wid,
                                    "address": addr.to_string(),
                                }),
                            );
                        }
                        WorkerMessage::Result { result } => {
                            {
                                let coord = global_coordinator().lock();
                                if let Ok(mut coord) = coord {
                                    coord.complete(result.clone());
                                }
                            }
                            let _ = app.emit(
                                "distributed://progress",
                                serde_json::json!({
                                    "status": "chunk_complete",
                                    "chunk_id": result.chunk_id,
                                    "worker_id": result.worker_id,
                                    "elapsed": result.elapsed_seconds,
                                }),
                            );
                        }
                    }
                }

                let next_chunk = {
                    let coord = global_coordinator().lock();
                    match coord {
                        Ok(mut coord) => coord.dispatch(&worker_id),
                        Err(_) => None,
                    }
                };

                let response = match next_chunk {
                    Some(chunk) => {
                        let _ = app.emit(
                            "distributed://progress",
                            serde_json::json!({
                                "status": "dispatched",
                                "chunk_id": chunk.id,
                                "worker_id": &worker_id,
                            }),
                        );
                        serde_json::to_string(&CoordinatorMessage::Dispatch { chunk })
                            .unwrap_or_else(|_| r#"{"type":"idle"}"#.into())
                    }
                    None => r#"{"type":"idle"}"#.to_string(),
                };

                let response_line = format!("{response}\n");
                let _ = writer.write_all(response_line.as_bytes()).await;
            }
            Err(_) => break,
        }
    }

    {
        let state = global_server_state().lock();
        if let Ok(mut state) = state {
            state.workers_connected = state.workers_connected.saturating_sub(1);
        }
    }
    let _ = app.emit(
        "distributed://progress",
        serde_json::json!({
            "status": "worker_disconnected",
            "worker_id": worker_id,
        }),
    );
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub is_running: bool,
    pub port: u16,
    pub workers_connected: usize,
    pub pending_chunks: usize,
    pub in_progress_chunks: usize,
    pub completed_chunks: usize,
    pub progress: f64,
}

pub fn get_server_status() -> Result<ServerStatus, String> {
    let state = global_server_state().lock().map_err(|e| e.to_string())?;
    let coord = global_coordinator().lock().map_err(|e| e.to_string())?;
    Ok(ServerStatus {
        is_running: state.is_running,
        port: state.port,
        workers_connected: state.workers_connected,
        pending_chunks: coord.pending.len(),
        in_progress_chunks: coord.in_progress.len(),
        completed_chunks: coord.completed.len(),
        progress: coord.progress(),
    })
}
