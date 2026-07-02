// Automation module — pipeline DSL, watch folders, scheduled jobs.
//
// Per ARCHITECTURE.md §3.2 Principle 4 — "Pipelines are first-class."
// This module provides the automation layer that makes MetaRDU Industrial
// a workflow automation tool, not just a viewer.

pub mod pipeline;
pub mod scheduler;
pub mod watcher;

pub use pipeline::{
    parse_pipeline, resolve_params, resolve_template, serialize_pipeline, Pipeline, PipelineAction,
    PipelineError, PipelineRunResult, PipelineStatus, PipelineStep, StepResult,
};
pub use scheduler::{global_scheduler_state, ScheduledJob, ScheduledJobStatus, SchedulerState};
pub use watcher::{global_watch_state, WatchFolder, WatchFolderStatus, WatchState};
