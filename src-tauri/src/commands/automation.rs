// Automation IPC commands — Phase 3.
//
// Exposes pipeline parsing, execution, watch folders, and scheduled jobs
// to the frontend. The pipeline executor calls real module functions
// directly — no IPC round-trip needed since it runs inside the Rust core.

use crate::automation::{
    global_scheduler_state, global_watch_state, parse_pipeline, resolve_params, serialize_pipeline,
    Pipeline, PipelineAction, PipelineRunResult, PipelineStatus, ScheduledJob, StepResult,
    WatchFolder,
};
use crate::commands::mining::read_dem_grid;
use crate::formats::{read_geotiff_header, read_las_header, read_las_points};
use crate::marine::{
    check_s44_compliance, generate_cube_surface, write_s57, CubeParams, S44CheckInput, S44Order,
    S57Feature, Sounding,
};
use crate::mining::{classify_ground as csf_classify, compute_volumes, CsfParams};
use std::collections::HashMap;
use std::path::PathBuf;
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
/// Each step calls the real module function directly — no IPC round-trip.
/// Step outputs are stored in a context HashMap and made available to
/// subsequent steps via template variables ({{steps.<id>.*}}).
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
            resolve_params(&step.params, &input, &step_outputs).map_err(|e| e.to_string())?;

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

/// Helper: extract a string param, returning an error if missing.
fn param_str(params: &HashMap<String, serde_json::Value>, key: &str) -> Result<String, String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required param '{key}'"))
}

/// Helper: extract a f64 param with a default.
fn param_f64(params: &HashMap<String, serde_json::Value>, key: &str, default: f64) -> f64 {
    params.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
}

/// Helper: extract a u64 param with a default.
fn param_u64(params: &HashMap<String, serde_json::Value>, key: &str, default: u64) -> u64 {
    params.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

/// Execute a single pipeline step by calling the real module function.
/// Returns (status, outputs, log_lines, error).
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
            let path = match param_str(params, "path") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            logs.push(format!("Probing file: {path}"));
            let path_buf = PathBuf::from(&path);
            let ext = path_buf
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            match ext.as_str() {
                "las" | "laz" => match read_las_header(&path_buf) {
                    Ok(header) => {
                        logs.push(format!(
                            "LAS {} — {} points, bounds [{:.4}, {:.4}] – [{:.4}, {:.4}]",
                            header.version_major,
                            header.point_count,
                            header.min_x,
                            header.min_y,
                            header.max_x,
                            header.max_y
                        ));
                        outputs.insert("point_count".into(), serde_json::json!(header.point_count));
                        outputs.insert("min_x".into(), serde_json::json!(header.min_x));
                        outputs.insert("min_y".into(), serde_json::json!(header.min_y));
                        outputs.insert("max_x".into(), serde_json::json!(header.max_x));
                        outputs.insert("max_y".into(), serde_json::json!(header.max_y));
                        outputs.insert(
                            "las_version".into(),
                            serde_json::json!(format!(
                                "{}.{}",
                                header.version_major, header.version_minor
                            )),
                        );
                        (PipelineStatus::Complete, outputs, logs, None)
                    }
                    Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
                },
                "tif" | "tiff" => match read_geotiff_header(&path_buf) {
                    Ok(header) => {
                        logs.push(format!(
                            "GeoTIFF {}×{}, {}bps, EPSG: {:?}",
                            header.width, header.length, header.bits_per_sample, header.epsg
                        ));
                        if let Some(bounds) = header.bounds {
                            outputs.insert("min_x".into(), serde_json::json!(bounds[0]));
                            outputs.insert("min_y".into(), serde_json::json!(bounds[1]));
                            outputs.insert("max_x".into(), serde_json::json!(bounds[2]));
                            outputs.insert("max_y".into(), serde_json::json!(bounds[3]));
                        }
                        outputs.insert("width".into(), serde_json::json!(header.width));
                        outputs.insert("height".into(), serde_json::json!(header.length));
                        outputs.insert("epsg".into(), serde_json::json!(header.epsg));
                        (PipelineStatus::Complete, outputs, logs, None)
                    }
                    Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
                },
                _ => {
                    logs.push(format!("Unsupported extension: .{ext}"));
                    (
                        PipelineStatus::Failed,
                        outputs,
                        logs,
                        Some(format!("unsupported extension: .{ext}")),
                    )
                }
            }
        }

        PipelineAction::ClassifyGround => {
            let path = match param_str(params, "path") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            let cloth_resolution = param_f64(params, "cloth_resolution", 0.5);
            let classification_threshold = param_f64(params, "classification_threshold", 0.5);
            let max_iterations = param_u64(params, "max_iterations", 500) as u32;
            let max_points = param_u64(params, "max_points", 0);

            logs.push(format!("Loading LAS points from {path}..."));
            let path_buf = PathBuf::from(&path);
            let points = match read_las_points(&path_buf, max_points) {
                Ok(p) => p,
                Err(e) => {
                    return (PipelineStatus::Failed, outputs, logs, Some(e.to_string()));
                }
            };
            logs.push(format!("Loaded {} points", points.len()));

            let csf_params = CsfParams {
                cloth_resolution,
                classification_threshold,
                max_iterations,
                ..Default::default()
            };

            logs.push("Running CSF classification...".into());
            match csf_classify(&points, &csf_params) {
                Ok(result) => {
                    logs.push(format!(
                        "Classification complete: {} ground / {} non-ground ({} iterations)",
                        result.ground_count, result.non_ground_count, result.iterations_run
                    ));
                    outputs.insert(
                        "ground_count".into(),
                        serde_json::json!(result.ground_count),
                    );
                    outputs.insert(
                        "non_ground_count".into(),
                        serde_json::json!(result.non_ground_count),
                    );
                    outputs.insert("total_points".into(), serde_json::json!(result.point_count));
                    outputs.insert(
                        "iterations".into(),
                        serde_json::json!(result.iterations_run),
                    );
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::ComputeVolumes => {
            let current_path = match param_str(params, "current_path") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            let reference_path = match param_str(params, "reference_path") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            let bench_interval = param_f64(params, "bench_interval", 5.0);

            logs.push(format!("Loading current DEM: {current_path}"));
            let curr_path_buf = PathBuf::from(&current_path);
            let curr_header = match read_geotiff_header(&curr_path_buf) {
                Ok(h) => h,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            };
            let curr_grid = match read_dem_grid(&curr_path_buf, &curr_header) {
                Ok(g) => g,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            logs.push(format!(
                "Current DEM: {}×{} cells",
                curr_header.width, curr_header.length
            ));

            // Handle flat:Z reference or file path
            let ref_grid: Vec<f64> = if reference_path.starts_with("flat:") {
                let z: f64 = reference_path
                    .strip_prefix("flat:")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                logs.push(format!("Using flat reference plane at z={z}"));
                vec![z; curr_grid.len()]
            } else {
                logs.push(format!("Loading reference DEM: {reference_path}"));
                let ref_path_buf = PathBuf::from(&reference_path);
                let ref_header = match read_geotiff_header(&ref_path_buf) {
                    Ok(h) => h,
                    Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
                };
                match read_dem_grid(&ref_path_buf, &ref_header) {
                    Ok(g) => g,
                    Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
                }
            };

            // Derive cell dimensions from pixel scale
            let (cell_w, cell_h) = curr_header
                .model_pixel_scale
                .map(|s| (s[0].abs(), s[1].abs()))
                .unwrap_or((1.0, 1.0));

            logs.push("Computing volumes...".into());
            match compute_volumes(&curr_grid, &ref_grid, cell_w, cell_h, bench_interval) {
                Ok(result) => {
                    logs.push(format!(
                        "Fill: {:.1} m³, Cut: {:.1} m³, Net: {:.1} m³ ({} benches)",
                        result.fill_volume,
                        result.cut_volume,
                        result.net_volume,
                        result.benches.len()
                    ));
                    outputs.insert("fill_volume".into(), serde_json::json!(result.fill_volume));
                    outputs.insert("cut_volume".into(), serde_json::json!(result.cut_volume));
                    outputs.insert("net_volume".into(), serde_json::json!(result.net_volume));
                    outputs.insert("fill_cells".into(), serde_json::json!(result.fill_cells));
                    outputs.insert("cut_cells".into(), serde_json::json!(result.cut_cells));
                    outputs.insert(
                        "bench_count".into(),
                        serde_json::json!(result.benches.len()),
                    );
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::GenerateReport => {
            let output_path = param_str(params, "output_path")
                .unwrap_or_else(|_| "/tmp/metardu_report.html".into());
            logs.push(format!("Generating report → {output_path}"));

            // Generate a simple HTML report summarizing all step outputs
            // passed via template variables. In Phase 4+ this becomes a
            // proper PDF with charts.
            let mut html = String::new();
            html.push_str("<!DOCTYPE html><html><head><meta charset='utf-8'>");
            html.push_str(
                "<style>body{font-family:monospace;background:#0A192F;color:#fff;padding:20px} ",
            );
            html.push_str("h1{color:#FFA500} .stat{margin:4px 0} .label{color:#6B7280} ");
            html.push_str(".value{color:#fff;font-weight:bold}</style></head><body>");
            html.push_str("<h1>MetaRDU Industrial — Survey Report</h1>");
            html.push_str(&format!("<p>Generated: {}</p>", {
                use std::time::{SystemTime, UNIX_EPOCH};
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                format!("epoch {secs}")
            }));

            // Write report content from any params that look like data
            for (key, value) in params {
                if key.starts_with("report_")
                    || matches!(
                        key.as_str(),
                        "fill_volume"
                            | "cut_volume"
                            | "net_volume"
                            | "ground_count"
                            | "point_count"
                    )
                {
                    html.push_str(&format!(
                        "<div class='stat'><span class='label'>{key}:</span> <span class='value'>{value}</span></div>"
                    ));
                }
            }
            html.push_str("</body></html>");

            match std::fs::write(&output_path, html) {
                Ok(_) => {
                    logs.push(format!("Report written to {output_path}"));
                    outputs.insert("report_path".into(), serde_json::Value::String(output_path));
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::GenerateCubeSurface => {
            // Soundings are passed as a JSON array in params
            let soundings_json = match params.get("soundings") {
                Some(v) => v,
                None => {
                    return (
                        PipelineStatus::Failed,
                        outputs,
                        logs,
                        Some("missing 'soundings' param".into()),
                    );
                }
            };
            let soundings: Vec<Sounding> = match serde_json::from_value(soundings_json.clone()) {
                Ok(s) => s,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            };
            let resolution = param_f64(params, "resolution", 1.0);

            logs.push(format!("Running CUBE on {} soundings...", soundings.len()));
            let cube_params = CubeParams {
                resolution,
                ..Default::default()
            };
            match generate_cube_surface(&soundings, &cube_params) {
                Ok(surface) => {
                    logs.push(format!(
                        "CUBE complete: {} valid cells, {} ambiguous ({}×{} grid)",
                        surface.valid_cells,
                        surface.ambiguous_cells,
                        surface.dims.0,
                        surface.dims.1
                    ));
                    outputs.insert("valid_cells".into(), serde_json::json!(surface.valid_cells));
                    outputs.insert(
                        "ambiguous_cells".into(),
                        serde_json::json!(surface.ambiguous_cells),
                    );
                    outputs.insert(
                        "total_soundings".into(),
                        serde_json::json!(surface.total_soundings),
                    );
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::CheckS44Compliance => {
            let soundings_json = match params.get("soundings") {
                Some(v) => v,
                None => {
                    return (
                        PipelineStatus::Failed,
                        outputs,
                        logs,
                        Some("missing 'soundings' param".into()),
                    )
                }
            };
            let soundings: Vec<S44CheckInput> = match serde_json::from_value(soundings_json.clone())
            {
                Ok(s) => s,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            };
            let order_str = param_str(params, "order").unwrap_or_else(|_| "order_1a".into());
            let order = match order_str.as_str() {
                "exclusive" => S44Order::Exclusive,
                "special" => S44Order::Special,
                "order_1a" => S44Order::Order1a,
                "order_1b" => S44Order::Order1b,
                "order_2" => S44Order::Order2,
                _ => S44Order::Order1a,
            };

            logs.push(format!(
                "Checking S-44 {} compliance on {} soundings...",
                order_str,
                soundings.len()
            ));
            match check_s44_compliance(&soundings, order) {
                Ok(result) => {
                    logs.push(format!(
                        "S-44 {}: {}/{} pass ({:.1}%)",
                        order_str,
                        result.passing_soundings,
                        result.total_soundings,
                        result.pass_rate * 100.0
                    ));
                    outputs.insert("pass_rate".into(), serde_json::json!(result.pass_rate));
                    outputs.insert(
                        "passing".into(),
                        serde_json::json!(result.passing_soundings),
                    );
                    outputs.insert(
                        "failing".into(),
                        serde_json::json!(result.failing_soundings),
                    );
                    outputs.insert("status".into(), serde_json::json!(result.status));
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::ExportS57 => {
            let features_json = match params.get("features") {
                Some(v) => v,
                None => {
                    return (
                        PipelineStatus::Failed,
                        outputs,
                        logs,
                        Some("missing 'features' param".into()),
                    )
                }
            };
            let features: Vec<S57Feature> = match serde_json::from_value(features_json.clone()) {
                Ok(f) => f,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            };
            let path = param_str(params, "path").unwrap_or_else(|_| "/tmp/export.000".into());

            logs.push(format!(
                "Exporting {} S-57 features → {path}",
                features.len()
            ));
            match write_s57(&PathBuf::from(&path), &features) {
                Ok(_) => {
                    logs.push(format!("S-57 export complete → {path}"));
                    outputs.insert("export_path".into(), serde_json::Value::String(path));
                    outputs.insert("feature_count".into(), serde_json::json!(features.len()));
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::ComputeEpochDiff => {
            let prev_path = match param_str(params, "previous_path") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            let curr_path = match param_str(params, "current_path") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };

            logs.push(format!("Loading previous DEM: {prev_path}"));
            let prev_path_buf = PathBuf::from(&prev_path);
            let prev_header = match read_geotiff_header(&prev_path_buf) {
                Ok(h) => h,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            };
            let prev_grid = match read_dem_grid(&prev_path_buf, &prev_header) {
                Ok(g) => g,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };

            logs.push(format!("Loading current DEM: {curr_path}"));
            let curr_path_buf = PathBuf::from(&curr_path);
            let curr_header = match read_geotiff_header(&curr_path_buf) {
                Ok(h) => h,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            };
            let curr_grid = match read_dem_grid(&curr_path_buf, &curr_header) {
                Ok(g) => g,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };

            let density = param_f64(params, "density", 2.7);
            let params_4d = crate::mining::Monitoring4DParams {
                cell_area: 1.0,
                density,
                ..Default::default()
            };

            logs.push("Computing epoch difference...".into());
            match crate::mining::compute_epoch_diff(&prev_grid, &curr_grid, &params_4d) {
                Ok(diff) => {
                    logs.push(format!(
                        "Fill: {:.1} m³, Cut: {:.1} m³, {} hotspots",
                        diff.summary.total_fill_volume,
                        diff.summary.total_cut_volume,
                        diff.hotspots.len()
                    ));
                    outputs.insert(
                        "fill_volume".into(),
                        serde_json::json!(diff.summary.total_fill_volume),
                    );
                    outputs.insert(
                        "cut_volume".into(),
                        serde_json::json!(diff.summary.total_cut_volume),
                    );
                    outputs.insert(
                        "net_volume".into(),
                        serde_json::json!(diff.summary.net_volume),
                    );
                    outputs.insert(
                        "fill_tonnage".into(),
                        serde_json::json!(diff.summary.total_fill_tonnage),
                    );
                    outputs.insert(
                        "cut_tonnage".into(),
                        serde_json::json!(diff.summary.total_cut_tonnage),
                    );
                    outputs.insert("hotspots".into(), serde_json::json!(diff.hotspots.len()));
                    outputs.insert("max_fill".into(), serde_json::json!(diff.summary.max_fill));
                    outputs.insert("max_cut".into(), serde_json::json!(diff.summary.max_cut));
                    (PipelineStatus::Complete, outputs, logs, None)
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::OdmPipeline => {
            // ODM requires Docker — can't run headlessly in all environments.
            // The pipeline step validates the config and delegates to the
            // ODM runner. If Docker isn't available, it fails gracefully.
            let images_dir = match param_str(params, "images_dir") {
                Ok(p) => p,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            logs.push(format!("ODM pipeline step: images_dir={images_dir}"));
            logs.push("ODM requires Docker + opendronemap/odm image. Run via ODM Pipeline dialog for interactive progress.".into());
            // For pipeline mode, we check Docker availability
            match crate::pipelines::check_odm("opendronemap/odm:latest").await {
                Ok(true) => {
                    logs.push("Docker + ODM image available. Starting ODM run...".into());
                    let config = crate::pipelines::OdmConfig {
                        images_dir,
                        ..Default::default()
                    };
                    match crate::pipelines::run_odm(&config, |line| {
                        logs.push(line.to_string());
                    })
                    .await
                    {
                        Ok(las_path) => {
                            let las_str = las_path.display().to_string();
                            logs.push(format!("ODM complete → {las_str}"));
                            outputs.insert("las_path".into(), serde_json::Value::String(las_str));
                            (PipelineStatus::Complete, outputs, logs, None)
                        }
                        Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
                    }
                }
                Ok(false) => {
                    logs.push(
                        "ODM image not pulled. Run: docker pull opendronemap/odm:latest".into(),
                    );
                    (
                        PipelineStatus::Failed,
                        outputs,
                        logs,
                        Some("ODM image not pulled".into()),
                    )
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
        }

        PipelineAction::ShellCommand => {
            let cmd = match param_str(params, "command") {
                Ok(c) => c,
                Err(e) => return (PipelineStatus::Failed, outputs, logs, Some(e)),
            };
            logs.push(format!("Executing: {cmd}"));
            // Use tokio::process for async shell command
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .await;

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stdout.is_empty() {
                        logs.push(stdout.trim().to_string());
                    }
                    if !stderr.is_empty() {
                        logs.push(format!("[stderr] {}", stderr.trim()));
                    }
                    if output.status.success() {
                        outputs.insert(
                            "exit_code".into(),
                            serde_json::json!(output.status.code().unwrap_or(0)),
                        );
                        outputs.insert(
                            "stdout".into(),
                            serde_json::Value::String(stdout.trim().to_string()),
                        );
                        (PipelineStatus::Complete, outputs, logs, None)
                    } else {
                        (
                            PipelineStatus::Failed,
                            outputs,
                            logs,
                            Some(format!("exit code: {}", output.status.code().unwrap_or(-1))),
                        )
                    }
                }
                Err(e) => (PipelineStatus::Failed, outputs, logs, Some(e.to_string())),
            }
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
