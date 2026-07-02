// Pipeline IPC commands — Phase 1 ODM integration.
//
// Exposes the ODM subprocess manager to the frontend. Progress is streamed
// via Tauri events rather than polled — the frontend subscribes to
// 'odm://progress' events.

use crate::pipelines::{check_odm, run_odm, OdmConfig};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

/// Shared state — holds the latest ODM status for get_odm_status queries
/// and the optional in-flight run handle for cancellation (Phase 2).
#[derive(Default)]
pub struct OdmState {
    pub last_status: Option<OdmRunStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OdmRunStatus {
    pub phase: String,
    pub last_log_line: String,
    pub output_las_path: Option<String>,
    pub error: Option<String>,
    pub running: bool,
}

#[derive(Debug, Serialize)]
pub struct OdmCheckResult {
    pub docker_available: bool,
    pub image_pulled: bool,
    pub image_name: String,
}

/// Check if Docker is installed and the ODM image is available.
#[tauri::command]
pub async fn check_odm_availability(image: Option<String>) -> Result<OdmCheckResult, String> {
    let image = image.unwrap_or_else(|| "opendronemap/odm:latest".into());
    let image_pulled = check_odm(&image).await.map_err(|e| e.to_string())?;
    Ok(OdmCheckResult {
        docker_available: true, // if check_odm returned Ok, docker is there
        image_pulled,
        image_name: image,
    })
}

/// Run the ODM pipeline. Streams progress via 'odm://progress' events.
///
/// The frontend should subscribe to 'odm://progress' before calling this,
/// then await the result. On success, the result contains the output LAS path.
#[tauri::command]
pub async fn run_odm_pipeline(
    app: AppHandle,
    state: State<'_, Mutex<OdmState>>,
    config: OdmConfig,
) -> Result<String, String> {
    // Mark as running
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.last_status = Some(OdmRunStatus {
            phase: "starting".into(),
            last_log_line: String::new(),
            output_las_path: None,
            error: None,
            running: true,
        });
    }

    let app_for_log = app.clone();
    let result = run_odm(&config, move |line: &str| {
        // Emit a progress event for every log line
        let phase = crate::pipelines::estimate_progress(line)
            .map(|p| format!("progress: {:.0}%", p * 100.0))
            .unwrap_or_else(|| "running".to_string());
        let _ = app_for_log.emit(
            "odm://progress",
            OdmRunStatus {
                phase: phase.clone(),
                last_log_line: line.to_string(),
                output_las_path: None,
                error: None,
                running: true,
            },
        );
    })
    .await;

    match result {
        Ok(las_path) => {
            let path_str = las_path.display().to_string();
            let _ = app.emit(
                "odm://progress",
                OdmRunStatus {
                    phase: "complete".into(),
                    last_log_line: format!("ODM completed — LAS at {path_str}"),
                    output_las_path: Some(path_str.clone()),
                    error: None,
                    running: false,
                },
            );
            // Update shared state
            if let Ok(mut s) = state.lock() {
                s.last_status = Some(OdmRunStatus {
                    phase: "complete".into(),
                    last_log_line: String::new(),
                    output_las_path: Some(path_str.clone()),
                    error: None,
                    running: false,
                });
            }
            Ok(path_str)
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = app.emit(
                "odm://progress",
                OdmRunStatus {
                    phase: "error".into(),
                    last_log_line: String::new(),
                    output_las_path: None,
                    error: Some(msg.clone()),
                    running: false,
                },
            );
            if let Ok(mut s) = state.lock() {
                s.last_status = Some(OdmRunStatus {
                    phase: "error".into(),
                    last_log_line: String::new(),
                    output_las_path: None,
                    error: Some(msg.clone()),
                    running: false,
                });
            }
            Err(msg)
        }
    }
}

/// Get the latest ODM status (for refreshing after a window reload).
#[tauri::command]
pub fn get_odm_status(state: State<'_, Mutex<OdmState>>) -> Result<Option<OdmRunStatus>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.last_status.clone())
}
