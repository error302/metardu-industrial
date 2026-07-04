// Scheduled job manager — interval-based job execution for recurring tasks.
//
// Per ARCHITECTURE.md §4.2 — scheduled jobs for daily QC, overnight
// processing, etc.
//
// Phase 3 uses simple interval scheduling (every N seconds/minutes/hours).
// Phase 4+ can add full cron expression parsing.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub pipeline_name: String,
    pub interval_secs: u64,
    #[serde(default = "default_true")]
    pub active: bool,
    /// Pipeline input parameters (passed as the `input` context)
    #[serde(default)]
    pub params: std::collections::HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduledJobStatus {
    pub id: String,
    pub name: String,
    pub pipeline_name: String,
    pub active: bool,
    pub interval_secs: u64,
    pub runs_completed: usize,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
}

/// Global scheduler state.
pub struct SchedulerState {
    pub jobs: Vec<ScheduledJob>,
    pub run_counts: std::collections::HashMap<String, usize>,
    pub last_runs: std::collections::HashMap<String, Instant>,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            run_counts: std::collections::HashMap::new(),
            last_runs: std::collections::HashMap::new(),
        }
    }

    pub fn add_job(&mut self, job: ScheduledJob) {
        self.run_counts.insert(job.id.clone(), 0);
        self.jobs.push(job);
    }

    pub fn remove_job(&mut self, id: &str) {
        self.jobs.retain(|j| j.id != id);
        self.run_counts.remove(id);
        self.last_runs.remove(id);
    }

    /// Check which jobs are due to run. Returns job IDs that should
    /// be triggered now.
    pub fn check_due(&mut self) -> Vec<String> {
        let now = Instant::now();
        let mut due = Vec::new();

        for job in &self.jobs {
            if !job.active {
                continue;
            }

            let should_run = match self.last_runs.get(&job.id) {
                Some(last) => now.duration_since(*last).as_secs() >= job.interval_secs,
                None => true, // Never run before — run now
            };

            if should_run {
                due.push(job.id.clone());
                self.last_runs.insert(job.id.clone(), now);
                *self.run_counts.entry(job.id.clone()).or_default() += 1;
            }
        }

        due
    }

    pub fn get_status(&self) -> Vec<ScheduledJobStatus> {
        let now = Instant::now();
        self.jobs
            .iter()
            .map(|job| {
                let runs = self.run_counts.get(&job.id).copied().unwrap_or(0);
                let next = match self.last_runs.get(&job.id) {
                    Some(last) => {
                        let elapsed = now.duration_since(*last).as_secs();
                        let remaining = job.interval_secs.saturating_sub(elapsed);
                        format!("in {}s", remaining)
                    }
                    None => "now".into(),
                };
                ScheduledJobStatus {
                    id: job.id.clone(),
                    name: job.name.clone(),
                    pipeline_name: job.pipeline_name.clone(),
                    active: job.active,
                    interval_secs: job.interval_secs,
                    runs_completed: runs,
                    last_run: self.last_runs.get(&job.id).map(|_| "completed".into()),
                    next_run: Some(next),
                }
            })
            .collect()
    }
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn global_scheduler_state() -> &'static Mutex<SchedulerState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<SchedulerState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SchedulerState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_job() {
        let mut state = SchedulerState::new();
        state.add_job(ScheduledJob {
            id: "job1".into(),
            name: "Daily QC".into(),
            pipeline_name: "qc".into(),
            interval_secs: 86400,
            active: true,
            params: Default::default(),
        });
        assert_eq!(state.jobs.len(), 1);
        state.remove_job("job1");
        assert_eq!(state.jobs.len(), 0);
    }

    #[test]
    fn test_first_run_is_immediate() {
        let mut state = SchedulerState::new();
        state.add_job(ScheduledJob {
            id: "job1".into(),
            name: "Test".into(),
            pipeline_name: "test".into(),
            interval_secs: 3600,
            active: true,
            params: Default::default(),
        });
        let due = state.check_due();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0], "job1");
    }

    #[test]
    fn test_inactive_jobs_not_due() {
        let mut state = SchedulerState::new();
        state.add_job(ScheduledJob {
            id: "job1".into(),
            name: "Test".into(),
            pipeline_name: "test".into(),
            interval_secs: 60,
            active: false,
            params: Default::default(),
        });
        let due = state.check_due();
        assert!(due.is_empty());
    }
}
