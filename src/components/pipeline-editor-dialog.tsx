import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * Pipeline Editor Dialog — Phase 3 Automation.
 *
 * Define, edit, and run YAML processing pipelines. The classic
 * "drop files, get PDF report" workflow.
 *
 * Includes a YAML editor, step list with live status, run button,
 * progress streaming via 'pipeline://progress' events, and log output.
 *
 * Also manages watch folders and scheduled jobs.
 */

import { useEffect, useState } from "react";
import { X, Play, Loader2, GitBranch, Clock, FolderOpen, Plus, Trash2 } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { colors } from "@/lib/tokens";
import {
  runPipelineCmd,
  parsePipelineCmd,
  addWatchFolder,
  removeWatchFolder,
  listWatchFolders,
  addScheduledJob,
  removeScheduledJob,
  listScheduledJobs,
  type PipelineRunResult,
  type WatchFolderStatus,
  type ScheduledJobStatus,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

const SAMPLE_YAML = `name: "Drone → Volume Report"
description: "Ingest drone photos, classify, compute volumes, generate report"
steps:
  - id: ingest
    action: odm_pipeline
    params:
      images_dir: "{{input.dir}}"
      feature_quality: high
    outputs:
      las_path: "{{steps.ingest.las_path}}"
  - id: classify
    action: classify_ground
    params:
      path: "{{steps.ingest.las_path}}"
      cloth_resolution: 0.5
    outputs:
      ground_count: "{{steps.classify.ground_count}}"
  - id: volume
    action: compute_volumes
    params:
      current_path: "{{steps.ingest.las_path}}"
      reference_path: "flat:100.0"
      bench_interval: 5.0
    outputs:
      fill_volume: "{{steps.volume.fill_volume}}"
  - id: report
    action: generate_report
    params:
      template: stockpile
      output_path: "{{input.dir}}/report.pdf"
`;

type Tab = "pipeline" | "watch" | "schedule";

export function PipelineEditorDialog({ open, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("pipeline");
  const [yaml, setYaml] = useState(SAMPLE_YAML);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<PipelineRunResult | null>(null);
  const [logLines, setLogLines] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Watch folders + scheduled jobs state
  const [watchFolders, setWatchFolders] = useState<WatchFolderStatus[]>([]);
  const [jobs, setJobs] = useState<ScheduledJobStatus[]>([]);
  const [newWatchPath, setNewWatchPath] = useState("");
  const [newWatchPipeline, setNewWatchPipeline] = useState("Drone → Volume Report");
  const [newWatchExts, setNewWatchExts] = useState("las,tif,all");
  const [newJobName, setNewJobName] = useState("");
  const [newJobPipeline, setNewJobPipeline] = useState("Drone → Volume Report");
  const [newJobInterval, setNewJobInterval] = useState(86400);

  // Subscribe to pipeline progress events
  useEscapeKey(onClose, open);
  useEffect(() => {
    if (!open) return;
    const unlisten = listen<{ step_id: string; action: string; status: string; log_lines?: string[]; error?: string }>(
      "pipeline://progress",
      (event) => {
        const p = event.payload;
        const lines = p.log_lines;
        if (lines && lines.length > 0) {
          setLogLines((prev) => [...prev, ...lines].slice(-200));
        }
        if (p.status === "complete" || p.status === "failed") {
          setRunning(false);
        }
      },
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [open]);

  // Load watch folders + jobs on open
  useEffect(() => {
    if (!open) return;
    listWatchFolders().then(setWatchFolders);
    listScheduledJobs().then(setJobs);
  }, [open]);

  if (!open) return null;

  async function handleRun() {
    setRunning(true);
    setError(null);
    setResult(null);
    setLogLines([]);
    try {
      // Parse YAML to Pipeline object via Rust
      const pipeline = await parsePipelineCmd(yaml);
      if (!pipeline) {
        setError("Browser mode — pipeline execution requires the native Tauri shell");
        setRunning(false);
        return;
      }
      const r = await runPipelineCmd(pipeline, { dir: "/data/survey" });
      if (r) {
        setResult(r);
        if (r.status === "failed") {
          setError(r.error || "Pipeline failed");
        }
      } else {
        setError("Browser mode — pipeline execution requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

  async function handleAddWatch() {
    if (!newWatchPath) return;
    await addWatchFolder({
      id: `wf_${Date.now()}`,
      path: newWatchPath,
      pipeline_name: newWatchPipeline,
      extensions: newWatchExts.split(",").map((s) => s.trim()),
      active: true,
      poll_interval_secs: 5,
    });
    setNewWatchPath("");
    listWatchFolders().then(setWatchFolders);
  }

  async function handleRemoveWatch(id: string) {
    await removeWatchFolder(id);
    listWatchFolders().then(setWatchFolders);
  }

  async function handleAddJob() {
    if (!newJobName) return;
    await addScheduledJob({
      id: `job_${Date.now()}`,
      name: newJobName,
      pipeline_name: newJobPipeline,
      interval_secs: newJobInterval,
      active: true,
      params: {},
    });
    setNewJobName("");
    listScheduledJobs().then(setJobs);
  }

  async function handleRemoveJob(id: string) {
    await removeScheduledJob(id);
    listScheduledJobs().then(setJobs);
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[85vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <GitBranch className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Automation
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-navy-border px-5 pt-3">
          <TabBtn active={tab === "pipeline"} onClick={() => setTab("pipeline")} icon={<GitBranch className="h-3 w-3" />} label="Pipelines" />
          <TabBtn active={tab === "watch"} onClick={() => setTab("watch")} icon={<FolderOpen className="h-3 w-3" />} label="Watch Folders" />
          <TabBtn active={tab === "schedule"} onClick={() => setTab("schedule")} icon={<Clock className="h-3 w-3" />} label="Scheduled Jobs" />
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {tab === "pipeline" && (
            <div>
              <textarea
                value={yaml}
                onChange={(e) => setYaml(e.target.value)}
                rows={14}
                className="mb-4 w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none"
              />
              {error && (
                <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
                  {error}
                </div>
              )}
              {result && (
                <div className="mb-4 space-y-2">
                  <div className="flex items-center gap-2 rounded-md border p-3" style={{ borderColor: `${result.status === "complete" ? colors.pass : colors.fail}40`, background: `${result.status === "complete" ? colors.pass : colors.fail}10` }}>
                    <span className="text-sm font-semibold uppercase" style={{ color: result.status === "complete" ? colors.pass : colors.fail }}>
                      {result.status}
                    </span>
                    <span className="text-xs text-steel-gray">· {result.elapsed_seconds.toFixed(2)}s · {result.steps.length} steps</span>
                  </div>
                  {result.steps.map((step, i) => (
                    <div key={i} className="flex items-center gap-2 rounded-md border border-navy-border bg-navy-base p-2 text-[10px]">
                      <span className={`h-2 w-2 rounded-full ${step.status === "complete" ? "bg-pass" : "bg-fail"}`} style={{ background: step.status === "complete" ? colors.pass : colors.fail }} />
                      <span className="font-mono text-white">{step.id}</span>
                      <span className="text-steel-gray">{step.action}</span>
                      <span className="ml-auto font-mono text-steel-gray">{step.elapsed_seconds.toFixed(2)}s</span>
                    </div>
                  ))}
                </div>
              )}
              {logLines.length > 0 && (
                <div className="mb-4 max-h-32 overflow-y-auto rounded-md border border-navy-border bg-black/50 p-2 font-mono text-[10px] text-steel-light">
                  {logLines.slice(-30).map((line, i) => <div key={i}>{line}</div>)}
                </div>
              )}
            </div>
          )}

          {tab === "watch" && (
            <div>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Add Watch Folder</div>
                <div className="grid grid-cols-2 gap-2">
                  <input type="text" value={newWatchPath} onChange={(e) => setNewWatchPath(e.target.value)} placeholder="/path/to/watch" className="rounded border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none" />
                  <input type="text" value={newWatchPipeline} onChange={(e) => setNewWatchPipeline(e.target.value)} placeholder="Pipeline name" className="rounded border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:border-industrial-orange focus:outline-none" />
                  <input type="text" value={newWatchExts} onChange={(e) => setNewWatchExts(e.target.value)} placeholder="las,tif,all" className="rounded border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none" />
                  <button onClick={handleAddWatch} className="flex items-center justify-center gap-1 rounded border border-navy-border bg-navy-elevated px-2 py-1.5 text-xs text-steel-light hover:bg-navy-base">
                    <Plus className="h-3 w-3" /> Add
                  </button>
                </div>
              </div>
              <div className="space-y-2">
                {watchFolders.length === 0 ? (
                  <div className="text-center text-xs text-steel-gray py-8">No watch folders configured.</div>
                ) : (
                  watchFolders.map((wf) => (
                    <div key={wf.id} className="flex items-center gap-3 rounded-md border border-navy-border bg-navy-base p-3">
                      <FolderOpen className="h-4 w-4 text-steel-gray" />
                      <div className="flex-1 min-w-0">
                        <div className="font-mono text-xs text-white truncate">{wf.path}</div>
                        <div className="text-[10px] text-steel-gray">→ {wf.pipeline_name} · {wf.files_detected} files · {wf.pipelines_triggered} runs</div>
                      </div>
                      <span className={`h-2 w-2 rounded-full ${wf.active ? "" : "opacity-30"}`} style={{ background: wf.active ? colors.pass : colors.steelGray }} />
                      <button onClick={() => handleRemoveWatch(wf.id)} className="text-steel-gray hover:text-fail">
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {tab === "schedule" && (
            <div>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Add Scheduled Job</div>
                <div className="grid grid-cols-2 gap-2">
                  <input type="text" value={newJobName} onChange={(e) => setNewJobName(e.target.value)} placeholder="Job name (e.g. Daily QC)" className="rounded border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:border-industrial-orange focus:outline-none" />
                  <input type="text" value={newJobPipeline} onChange={(e) => setNewJobPipeline(e.target.value)} placeholder="Pipeline name" className="rounded border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:border-industrial-orange focus:outline-none" />
                  <input type="number" value={newJobInterval} onChange={(e) => setNewJobInterval(parseInt(e.target.value) || 86400)} placeholder="Interval (secs)" className="rounded border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none" />
                  <button onClick={handleAddJob} className="flex items-center justify-center gap-1 rounded border border-navy-border bg-navy-elevated px-2 py-1.5 text-xs text-steel-light hover:bg-navy-base">
                    <Plus className="h-3 w-3" /> Add
                  </button>
                </div>
              </div>
              <div className="space-y-2">
                {jobs.length === 0 ? (
                  <div className="text-center text-xs text-steel-gray py-8">No scheduled jobs configured.</div>
                ) : (
                  jobs.map((job) => (
                    <div key={job.id} className="flex items-center gap-3 rounded-md border border-navy-border bg-navy-base p-3">
                      <Clock className="h-4 w-4 text-steel-gray" />
                      <div className="flex-1 min-w-0">
                        <div className="text-xs text-white">{job.name}</div>
                        <div className="text-[10px] text-steel-gray">→ {job.pipeline_name} · every {job.interval_secs}s · {job.runs_completed} runs · next: {job.next_run}</div>
                      </div>
                      <span className={`h-2 w-2 rounded-full ${job.active ? "" : "opacity-30"}`} style={{ background: job.active ? colors.pass : colors.steelGray }} />
                      <button onClick={() => handleRemoveJob(job.id)} className="text-steel-gray hover:text-fail">
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">YAML pipeline DSL · template variables: {'{{input.*}}, {{steps.<id>.*}}'}</div>
          {tab === "pipeline" && (
            <button
              onClick={handleRun}
              disabled={running}
              className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
              style={{ background: running ? colors.steelGray : colors.industrialOrange, color: colors.navyBase }}
            >
              {running ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
              {running ? "Running…" : "Run pipeline"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function TabBtn({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 border-b-2 px-3 py-2 text-xs font-medium transition-colors ${active ? "text-white" : "text-steel-gray hover:text-steel-light"}`}
      style={active ? { borderColor: colors.industrialOrange } : { borderColor: "transparent" }}
    >
      {icon}
      {label}
    </button>
  );
}
