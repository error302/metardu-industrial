// Automation IPC commands — Phase 3.
//
// Exposes pipeline parsing, execution, watch folders, and scheduled jobs
// to the frontend.

use crate::automation::{
    global_scheduler_state, global_watch_state, parse_pipeline, serialize_pipeline, Pipeline,
    PipelineAction, PipelineRunResult, PipelineStatus, ScheduledJob, StepResult, WatchFolder,
};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};

/// Parse a YAML pipeline definition. Returns the parsed pipeline or error.
#[tauri::command]
pub fn parse_pipeline_cmd(yaml: String) -> Result<Pipeline, String> {
    parse_pipeline(&yaml).map_err(|e| e.to_string())
}

/// Serialize a pipeline to YAML (for saving to disk).
#[tauri::command]
pub fn serialize_pipeline_cmd(pipeline: Pipeline) -> Result<String, String> {
    serialize_pipeline(&pipeline).map_err(|e| e.to_string())
}

/// Run a pipeline. Streams progress via 'pipeline://progress' events.
///
/// Phase 3: executes noop and probe_file steps. Other actions are
/// stubbed — they emit a log line and succeed. Full execution of
/// odm_pipeline, classify_ground, etc. is wired in Phase 4.
#[tauri::command]
pub async fn run_pipeline_cmd(
    app: AppHandle,
    pipeline: Pipeline,
    input: HashMap<String, serde_json::Value>,
) -> Result<PipelineRunResult, String> {
    let start = std::time::Instant::now();
    let mut step_results = Vec::new();
    let mut step_outputs: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
    let mut overall_status = PipelineStatus::Complete;

    for step in &pipeline.steps {
        let step_start = std::time::Instant::now();
        let _ = app.emit(
            "pipeline://progress",
            serde_json::json!({
                "step_id": step.id,
                "action": step.action,
                "status": "running",
            }),
        );

        // Resolve params with template variables
        let resolved_params =
            crate::automation::resolve_params(&step.params, &input, &step_outputs)
                .map_err(|e| e.to_string())?;

        let (step_status, outputs, log_lines, error) =
            execute_step(step.action, &resolved_params).await;

        let result = StepResult {
            id: step.id.clone(),
            action: step.action,
            status: step_status.clone(),
            elapsed_seconds: step_start.elapsed().as_secs_f64(),
            outputs: outputs.clone(),
            error: error.clone(),
            log_lines: log_lines.clone(),
        };

        step_outputs.insert(step.id.clone(), outputs);

        let _ = app.emit(
            "pipeline://progress",
            serde_json::json!({
                "step_id": step.id,
                "action": step.action,
                "status": &step_status,
                "log_lines": &log_lines,
                "error": &error,
            }),
        );

        step_results.push(result);

        if step_status == PipelineStatus::Failed {
            overall_status = PipelineStatus::Failed;
            break;
        }
    }

    Ok(PipelineRunResult {
        pipeline_name: pipeline.name,
        status: overall_status,
        steps: step_results,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        error: None,
    })
}

/// Execute a single pipeline step. Returns (status, outputs, log_lines, error).
async fn execute_step(
    action: PipelineAction,
    params: &HashMap<String, serde_json::Value>,
) -> (
    PipelineStatus,
    HashMap<String, serde_json::Value>,
    Vec<String>,
    Option<String>,
) {
    let mut outputs = HashMap::new();
    let mut logs = Vec::new();

    match action {
        PipelineAction::Noop => {
            logs.push("No-op step executed".into());
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::ProbeFile => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                return (
                    PipelineStatus::Failed,
                    outputs,
                    logs,
                    Some("missing 'path' param".into()),
                );
            }
            logs.push(format!("Probing file: {path}"));
            // In Phase 3, we just log — the actual probe is done via
            // the existing probe_file IPC command from the frontend.
            outputs.insert("path".into(), serde_json::Value::String(path.into()));
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::OdmPipeline => {
            logs.push("ODM pipeline step — requires Docker + ODM image".into());
            logs.push(
                "In Phase 3, this is a stub. Use the ODM Pipeline dialog for real execution."
                    .into(),
            );
            outputs.insert(
                "las_path".into(),
                serde_json::Value::String("/tmp/odm_output.las".into()),
            );
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::ClassifyGround => {
            logs.push("CSF classification step".into());
            outputs.insert("ground_count".into(), serde_json::Value::Number(0.into()));
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::ComputeVolumes => {
            logs.push("Volume calculation step".into());
            outputs.insert("fill_volume".into(), serde_json::Value::Number(0.into()));
            outputs.insert("cut_volume".into(), serde_json::Value::Number(0.into()));
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::GenerateReport => {
            logs.push("Report generation step".into());
            let output_path = params
                .get("output_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp/report.pdf");
            outputs.insert(
                "report_path".into(),
                serde_json::Value::String(output_path.into()),
            );
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::GenerateCubeSurface => {
            logs.push("CUBE surface generation step".into());
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::CheckS44Compliance => {
            logs.push("S-44 compliance check step".into());
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::ExportS57 => {
            logs.push("S-57 export step".into());
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::ComputeEpochDiff => {
            logs.push("4D epoch diff step".into());
            (PipelineStatus::Complete, outputs, logs, None)
        }
        PipelineAction::ShellCommand => {
            logs.push("Shell command step — Phase 4 will execute via tauri-plugin-shell".into());
            (PipelineStatus::Complete, outputs, logs, None)
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Watch folder commands

#[tauri::command]
pub fn add_watch_folder(folder: WatchFolder) -> Result<(), String> {
    let mut state = global_watch_state().lock().map_err(|e| e.to_string())?;
    state.add_folder(folder);
    Ok(())
}

#[tauri::command]
pub fn remove_watch_folder(id: String) -> Result<(), String> {
    let mut state = global_watch_state().lock().map_err(|e| e.to_string())?;
    state.remove_folder(&id);
    Ok(())
}

#[tauri::command]
pub fn list_watch_folders() -> Result<Vec<crate::automation::WatchFolderStatus>, String> {
    let state = global_watch_state().lock().map_err(|e| e.to_string())?;
    Ok(state.get_status())
}

#[tauri::command]
pub fn scan_watch_folders() -> Result<Vec<(String, String, String)>, String> {
    let mut state = global_watch_state().lock().map_err(|e| e.to_string())?;
    Ok(state.scan())
}

// ──────────────────────────────────────────────────────────────────
// Scheduled job commands

#[tauri::command]
pub fn add_scheduled_job(job: ScheduledJob) -> Result<(), String> {
    let mut state = global_scheduler_state().lock().map_err(|e| e.to_string())?;
    state.add_job(job);
    Ok(())
}

#[tauri::command]
pub fn remove_scheduled_job(id: String) -> Result<(), String> {
    let mut state = global_scheduler_state().lock().map_err(|e| e.to_string())?;
    state.remove_job(&id);
    Ok(())
}

#[tauri::command]
pub fn list_scheduled_jobs() -> Result<Vec<crate::automation::ScheduledJobStatus>, String> {
    let state = global_scheduler_state().lock().map_err(|e| e.to_string())?;
    Ok(state.get_status())
}

#[tauri::command]
pub fn check_due_jobs() -> Result<Vec<String>, String> {
    let mut state = global_scheduler_state().lock().map_err(|e| e.to_string())?;
    Ok(state.check_due())
}
