// Pipeline DSL — YAML-defined processing pipelines for survey automation.
//
// Per ARCHITECTURE.md §3.2 Principle 4 — "Pipelines are first-class."
// Every operation the UI can do can also be expressed as a YAML pipeline
// step and run headlessly. This is what makes the watch-folder and
// scheduled job features possible without duplicating logic.
//
// Pipeline YAML schema:
//   name: "Drone → Volume Report"
//   description: "Ingest drone photos, classify, compute volumes, email PDF"
//   steps:
//     - id: ingest
//       action: odm_pipeline
//       params:
//         images_dir: "{{input.dir}}"
//         feature_quality: high
//       outputs:
//         las_path: "{{steps.ingest.las_path}}"
//     - id: classify
//       action: classify_ground
//       params:
//         path: "{{steps.ingest.las_path}}"
//         cloth_resolution: 0.5
//       outputs:
//         ground_count: "{{steps.classify.ground_count}}"
//     - id: volume
//       action: compute_volumes
//       params:
//         current_path: "{{steps.ingest.las_path}}"
//         reference_path: "flat:100.0"
//         bench_interval: 5.0
//       outputs:
//         fill_volume: "{{steps.volume.fill_volume}}"
//     - id: report
//       action: generate_report
//       params:
//         template: stockpile
//         output_path: "{{input.dir}}/report.pdf"
//
// Template variables:
//   {{input.*}} — pipeline input parameters
//   {{steps.<id>.*}} — outputs from previous steps

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub steps: Vec<PipelineStep>,
    /// Optional: trigger this pipeline when files appear in these directories
    #[serde(default)]
    pub watch_folders: Vec<String>,
    /// Optional: schedule as cron expression (e.g., "0 6 * * *" = daily at 6am)
    #[serde(default)]
    pub schedule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub id: String,
    pub action: PipelineAction,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub outputs: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineAction {
    /// Run ODM (OpenDroneMap) on a directory of drone images
    OdmPipeline,
    /// Classify ground points via CSF
    ClassifyGround,
    /// Compute fill/cut volumes between two DEMs
    ComputeVolumes,
    /// Generate a PDF report
    GenerateReport,
    /// Probe a file (read header metadata)
    ProbeFile,
    /// Run CUBE surface generation
    GenerateCubeSurface,
    /// Check S-44 compliance
    CheckS44Compliance,
    /// Export S-57 features
    ExportS57,
    /// Run 4D monitoring epoch diff
    ComputeEpochDiff,
    /// Custom shell command (via tauri-plugin-shell)
    ShellCommand,
    /// No-op (for testing)
    Noop,
}

impl PipelineAction {
    pub fn label(&self) -> &'static str {
        match self {
            PipelineAction::OdmPipeline => "ODM Pipeline",
            PipelineAction::ClassifyGround => "Classify Ground (CSF)",
            PipelineAction::ComputeVolumes => "Compute Volumes",
            PipelineAction::GenerateReport => "Generate Report",
            PipelineAction::ProbeFile => "Probe File",
            PipelineAction::GenerateCubeSurface => "CUBE Surface",
            PipelineAction::CheckS44Compliance => "S-44 Compliance",
            PipelineAction::ExportS57 => "S-57 Export",
            PipelineAction::ComputeEpochDiff => "4D Epoch Diff",
            PipelineAction::ShellCommand => "Shell Command",
            PipelineAction::Noop => "No-op",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineRunResult {
    pub pipeline_name: String,
    pub status: PipelineStatus,
    pub steps: Vec<StepResult>,
    pub elapsed_seconds: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Running,
    Complete,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    pub id: String,
    pub action: PipelineAction,
    pub status: PipelineStatus,
    pub elapsed_seconds: f64,
    pub outputs: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub log_lines: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("step '{0}' not found in pipeline")]
    StepNotFound(String),
    #[error("step '{0}' failed: {1}")]
    StepFailed(String, String),
    #[error("template variable not resolved: {0}")]
    UnresolvedTemplate(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse a YAML pipeline definition.
pub fn parse_pipeline(yaml: &str) -> Result<Pipeline, PipelineError> {
    serde_yaml::from_str(yaml).map_err(PipelineError::Yaml)
}

/// Serialize a pipeline to YAML.
pub fn serialize_pipeline(pipeline: &Pipeline) -> Result<String, PipelineError> {
    serde_yaml::to_string(pipeline).map_err(PipelineError::Yaml)
}

/// Resolve template variables like {{input.dir}} or {{steps.ingest.las_path}}
/// in a string value, using the provided context.
pub fn resolve_template(
    template: &str,
    input: &HashMap<String, serde_json::Value>,
    step_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> Result<String, PipelineError> {
    let mut result = template.to_string();
    let mut offset = 0;

    while let Some(start) = result[offset..].find("{{") {
        let abs_start = offset + start;
        if let Some(end) = result[abs_start..].find("}}") {
            let abs_end = abs_start + end + 2;
            let var = &result[abs_start + 2..abs_end - 2];
            let trimmed = var.trim();

            let resolved = resolve_variable(trimmed, input, step_outputs)?;
            result = format!("{}{}{}", &result[..abs_start], resolved, &result[abs_end..]);
            offset = abs_start + resolved.len();
        } else {
            break;
        }
    }

    Ok(result)
}

fn resolve_variable(
    var: &str,
    input: &HashMap<String, serde_json::Value>,
    step_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> Result<String, PipelineError> {
    let parts: Vec<&str> = var.split('.').collect();
    if parts.len() < 2 {
        return Err(PipelineError::UnresolvedTemplate(var.into()));
    }

    match parts[0] {
        "input" => {
            let key = parts[1..].join(".");
            input
                .get(&key)
                .map(|v| value_to_string(v))
                .ok_or_else(|| PipelineError::UnresolvedTemplate(var.into()))
        }
        "steps" => {
            if parts.len() < 3 {
                return Err(PipelineError::UnresolvedTemplate(var.into()));
            }
            let step_id = parts[1];
            let output_key = parts[2..].join(".");
            step_outputs
                .get(step_id)
                .and_then(|outputs| outputs.get(&output_key))
                .map(|v| value_to_string(v))
                .ok_or_else(|| PipelineError::UnresolvedTemplate(var.into()))
        }
        _ => Err(PipelineError::UnresolvedTemplate(var.into())),
    }
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => v.to_string(),
    }
}

/// Resolve all string values in a params HashMap, replacing templates.
pub fn resolve_params(
    params: &HashMap<String, serde_json::Value>,
    input: &HashMap<String, serde_json::Value>,
    step_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> Result<HashMap<String, serde_json::Value>, PipelineError> {
    let mut resolved = HashMap::new();
    for (k, v) in params {
        let resolved_value = match v {
            serde_json::Value::String(s) => {
                let resolved_str = resolve_template(s, input, step_outputs)?;
                serde_json::Value::String(resolved_str)
            }
            other => other.clone(),
        };
        resolved.insert(k.clone(), resolved_value);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pipeline() {
        let yaml = r#"
name: "Test Pipeline"
description: "A test"
steps:
  - id: noop
    action: noop
    params: {}
"#;
        let pipeline = parse_pipeline(yaml).unwrap();
        assert_eq!(pipeline.name, "Test Pipeline");
        assert_eq!(pipeline.steps.len(), 1);
        assert_eq!(pipeline.steps[0].action, PipelineAction::Noop);
    }

    #[test]
    fn test_template_resolution() {
        let mut input = HashMap::new();
        input.insert(
            "dir".into(),
            serde_json::Value::String("/data/survey".into()),
        );

        let mut step_outputs: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
        let mut ingest_outputs = HashMap::new();
        ingest_outputs.insert(
            "las_path".into(),
            serde_json::Value::String("/output/result.las".into()),
        );
        step_outputs.insert("ingest".into(), ingest_outputs);

        let result = resolve_template("{{input.dir}}/photos", &input, &step_outputs).unwrap();
        assert_eq!(result, "/data/survey/photos");

        let result = resolve_template("{{steps.ingest.las_path}}", &input, &step_outputs).unwrap();
        assert_eq!(result, "/output/result.las");
    }

    #[test]
    fn test_unresolved_template_errors() {
        let input = HashMap::new();
        let step_outputs = HashMap::new();
        let result = resolve_template("{{input.missing}}", &input, &step_outputs);
        assert!(matches!(result, Err(PipelineError::UnresolvedTemplate(_))));
    }

    #[test]
    fn test_serialize_roundtrip() {
        let yaml = r#"
name: "Roundtrip"
description: "Test"
steps:
  - id: step1
    action: noop
    params: {}
"#;
        let pipeline = parse_pipeline(yaml).unwrap();
        let serialized = serialize_pipeline(&pipeline).unwrap();
        let reparsed = parse_pipeline(&serialized).unwrap();
        assert_eq!(pipeline.name, reparsed.name);
        assert_eq!(pipeline.steps.len(), reparsed.steps.len());
    }
}
