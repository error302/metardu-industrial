/**
 * EOM Volumetric Auditor — commercial module v1.
 *
 * Single-dialog tool for end-of-month production reconciliation surveys.
 *
 * Pipeline (driven by run_eom_pipeline_cmd via Channel<EomProgressRpc>):
 *   1. Started            — pipeline spawned
 *   2. ReadingCurrentLas  — point count from header
 *   3. ClassifyingCurrent — CSF cloth simulation filters ground returns
 *   4. RasterizingCurrentDem — IDW interpolation → 2.5D grid
 *   5. ReadingPreviousLas — optional, for 4D diff
 *   6. ClassifyingPrevious
 *   7. RasterizingPreviousDem
 *   8. ComputingVolumes   — 2.5D matrix subtraction, bench breakdown
 *   9. HashingFiles       — SHA-256 of source LAS files (audit trail)
 *  10. Done               — EomOutputRpc returned
 *
 * Revenue: $2,500/seat/year — every mine runs this monthly for production
 * reporting and reconcile-against-design audits. License-gated: Trial mode
 * allows 3 reports; paid tiers unlock unlimited reports + watch folder.
 */

import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  X,
  Loader2,
  FolderOpen,
  Download,
  Copy,
  CheckCircle2,
  FileText,
  ShieldCheck,
  AlertTriangle,
  TrendingUp,
  TrendingDown,
  Calculator,
  Eye,
  Play,
  Square,
  Hash,
  Database,
  Grid3x3,
  Layers,
  KeyRound,
} from "lucide-react";
import { useEscapeKey } from "@/lib/use-escape-key";
import { pickFile, pickSaveFile, pickFolder } from "@/lib/file-picker";
import { useSurveyStore } from "@/stores/survey-store";
import { colors } from "@/lib/tokens";
import {
  runEomPipeline,
  generateEomReport,
  checkLicenseStatus,
  consumeReport,
  startEomWatchFolder,
  stopEomWatchFolder,
  isEomWatchFolderRunning,
  DEFAULT_CSF_PARAMS,
  DEFAULT_DEM_PARAMS,
  type EomInputRpc,
  type EomOutputRpc,
  type EomProgressRpc,
  type LicenseStatusRpc,
  type EomWatchEventRpc,
  type EomWatchFolderConfigRpc,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

/**
 * The 9 *active* stages of the EOM pipeline (Done is the terminal state —
 * we don't render a dot for it; instead, completion is signalled by all
 * dots turning green and the results panel appearing below).
 */
const STAGE_LABELS: { kind: EomProgressRpc["kind"]; label: string }[] = [
  { kind: "Started", label: "Start" },
  { kind: "ReadingCurrentLas", label: "Read LAS" },
  { kind: "ClassifyingCurrent", label: "Classify" },
  { kind: "RasterizingCurrentDem", label: "DEM" },
  { kind: "ReadingPreviousLas", label: "Prev LAS" },
  { kind: "ClassifyingPrevious", label: "Prev Classify" },
  { kind: "RasterizingPreviousDem", label: "Prev DEM" },
  { kind: "ComputingVolumes", label: "Volumes" },
  { kind: "HashingFiles", label: "SHA-256" },
];

export function EomAuditorDialog({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const lasFiles = useMemo(
    () => files.filter((f) => f.kind === "las" && f.status === "loaded"),
    [files],
  );

  // ─── Inputs ────────────────────────────────────────────────────────────
  const [currentLasPath, setCurrentLasPath] = useState("");
  const [previousLasPath, setPreviousLasPath] = useState("");
  const [referenceElevation, setReferenceElevation] = useState(0);
  const [cellSize, setCellSize] = useState(DEFAULT_DEM_PARAMS.cell_size);
  const [benchInterval, setBenchInterval] = useState(5);
  const [customer, setCustomer] = useState("");
  const [site, setSite] = useState("");
  const [surveyor, setSurveyor] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/eom_auditor_report.pdf");

  // ─── Runtime state ─────────────────────────────────────────────────────
  const [running, setRunning] = useState(false);
  const [activeStage, setActiveStage] = useState<number>(-1);
  const [stageMessage, setStageMessage] = useState<string>("");
  const [result, setResult] = useState<EomOutputRpc | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [reportGenerated, setReportGenerated] = useState(false);
  const [license, setLicense] = useState<LicenseStatusRpc | null>(null);

  // ─── Watch folder state ────────────────────────────────────────────────
  const [watchPath, setWatchPath] = useState("");
  const [watchRunning, setWatchRunning] = useState(false);
  const [watchEvents, setWatchEvents] = useState<EomWatchEventRpc[]>([]);

  useEscapeKey(onClose, open);
  if (!open) return null;

  // Refresh license status when the dialog opens.
  useEffect(() => {
    if (!open) return;
    checkLicenseStatus(null)
      .then(setLicense)
      .catch(() => setLicense({ state: "Invalid", reason: "license check failed" }));
  }, [open]);

  // Poll the watch-folder running flag once on open + when events arrive.
  useEffect(() => {
    if (!open) return;
    isEomWatchFolderRunning()
      .then(setWatchRunning)
      .catch(() => setWatchRunning(false));
  }, [open]);

  // Subscribe to eom://watch events emitted by the Rust watcher thread.
  useEffect(() => {
    if (!open) return;
    const unlisten = listen<EomWatchEventRpc>("eom://watch", (event) => {
      setWatchEvents((prev) => [event.payload, ...prev].slice(0, 50));
      if (event.payload.kind === "started" || event.payload.kind === "completed") {
        isEomWatchFolderRunning().then(setWatchRunning).catch(() => {});
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [open]);

  const canRun = !!currentLasPath && !running;

  async function handleRun() {
    setRunning(true);
    setError(null);
    setResult(null);
    setReportGenerated(false);
    setActiveStage(-1);
    setStageMessage("");

    const input: EomInputRpc = {
      current_las_path: currentLasPath,
      previous_las_path: previousLasPath || null,
      reference_flat_elevation: referenceElevation,
      csf_params: DEFAULT_CSF_PARAMS,
      dem_params: { ...DEFAULT_DEM_PARAMS, cell_size: cellSize },
      bench_interval: benchInterval,
      max_points: 5_000_000,
    };

    try {
      const out = await runEomPipeline(input, (p: EomProgressRpc) => {
        const idx = STAGE_LABELS.findIndex((s) => s.kind === p.kind);
        if (idx >= 0) {
          setActiveStage(idx);
          setStageMessage(formatStageMessage(p));
        }
        if (p.kind === "Done") {
          setResult(p.message);
          setActiveStage(STAGE_LABELS.length);
        }
      });
      // Browser-mode mock returns null; native returns the same payload via
      // the channel Done message. Use whichever we got.
      if (out) setResult(out);
      if (!out && !result) {
        // Browser mode: pipeline didn't emit Done — surface a soft notice.
        setError(
          "Browser mode — pipeline simulated. Run inside the Tauri shell to compute real volumes.",
        );
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

  async function handleCopy() {
    if (!result) return;
    const lines = [
      "MetaRDU Industrial — EOM Volumetric Auditor",
      `Customer: ${customer || "—"}  ·  Site: ${site || "—"}`,
      `Surveyor: ${surveyor || "—"}`,
      `Current LAS: ${currentLasPath.split(/[\\/]/).pop() ?? currentLasPath}`,
      previousLasPath ? `Previous LAS: ${previousLasPath.split(/[\\/]/).pop() ?? previousLasPath}` : "",
      `Reference elevation: ${referenceElevation} m`,
      `Cell size: ${cellSize} m  ·  Bench interval: ${benchInterval} m`,
      "",
      `Fill volume: ${result.volumes.fill_volume.toFixed(1)} m³`,
      `Cut  volume: ${result.volumes.cut_volume.toFixed(1)} m³`,
      `Net  volume: ${result.volumes.net_volume.toFixed(1)} m³`,
      "",
      `Points read: ${result.points_read.toLocaleString()}`,
      `Ground points: ${result.ground_points.toLocaleString()} (${(
        (result.ground_points / Math.max(1, result.points_read)) *
        100
      ).toFixed(1)}%)`,
      `DEM dims: ${result.dem_cols} × ${result.dem_rows} (cell: ${result.dem_cell_size}m)`,
      `Processing time: ${(result.processing_time_ms / 1000).toFixed(2)} s`,
      "",
      `Audit hash:  ${result.audit_hash}`,
    ];
    await navigator.clipboard.writeText(lines.filter(Boolean).join("\n"));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  async function handleGenerateReport() {
    if (!result) return;
    setGenerating(true);
    setError(null);
    try {
      await generateEomReport(
        result,
        customer || "(unspecified)",
        site || "(unspecified)",
        surveyor || "(unspecified)",
        reportPath,
        true,
      );
      const next = await consumeReport(null);
      setLicense(next);
      setReportGenerated(true);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setGenerating(false);
    }
  }

  async function handleStartWatch() {
    if (!watchPath) {
      setError("Pick a watch folder before starting.");
      return;
    }
    setError(null);
    try {
      const config: EomWatchFolderConfigRpc = {
        path: watchPath,
        poll_interval_secs: 30,
        csf_params: DEFAULT_CSF_PARAMS,
        dem_params: { ...DEFAULT_DEM_PARAMS, cell_size: cellSize },
        bench_interval: benchInterval,
        reference_flat_elevation: referenceElevation,
        customer: customer || "(unspecified)",
        site: site || "(unspecified)",
        surveyor: surveyor || "(unspecified)",
      };
      await startEomWatchFolder(config);
      setWatchRunning(true);
      setWatchEvents((prev) => [
        {
          kind: "started",
          file_path: watchPath,
          report_path: null,
          fill_volume: null,
          cut_volume: null,
          net_volume: null,
          error: null,
          processing_time_ms: null,
        },
        ...prev,
      ]);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleStopWatch() {
    try {
      await stopEomWatchFolder();
      setWatchRunning(false);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[92vh] w-full max-w-5xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* ─── Header ─── */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <ShieldCheck className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            EOM Volumetric Auditor
            <span
              className="ml-1 rounded-sm px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider"
              style={{
                background: `${colors.industrialOrange}20`,
                color: colors.industrialOrange,
                border: `1px solid ${colors.industrialOrange}40`,
              }}
            >
              Commercial
            </span>
          </h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
            aria-label="Close dialog"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* ─── License banner ─── */}
        <LicenseBanner status={license} />

        {/* ─── Body ─── */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div
              className="mb-4 rounded-md border p-3 text-xs"
              style={{
                borderColor: `${colors.fail}40`,
                background: `${colors.fail}10`,
                color: colors.fail,
              }}
            >
              {error}
            </div>
          )}

          <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
            {/* ═══ LEFT: Inputs ═══ */}
            <div className="space-y-4">
              {/* Current LAS/LAZ */}
              <PathField
                label="Current Survey (LAS/LAZ) *"
                value={currentLasPath}
                onPicked={setCurrentLasPath}
                onPick={() =>
                  pickFile({
                    extensions: ["las", "laz"],
                    filterName: "LAS/LAZ point cloud",
                    title: "Select current survey",
                  })
                }
                dropdownOptions={lasFiles.map((f) => ({
                  label: f.name,
                  value: f.path,
                }))}
                dropdownValue={currentLasPath}
                onDropdownChange={setCurrentLasPath}
              />

              <PathField
                label="Previous Survey (optional — for 4D diff)"
                value={previousLasPath}
                onPicked={setPreviousLasPath}
                onPick={() =>
                  pickFile({
                    extensions: ["las", "laz"],
                    filterName: "LAS/LAZ point cloud",
                    title: "Select previous survey",
                  })
                }
                dropdownOptions={lasFiles
                  .filter((f) => f.path !== currentLasPath)
                  .map((f) => ({ label: f.name, value: f.path }))}
                dropdownValue={previousLasPath}
                onDropdownChange={setPreviousLasPath}
              />

              {/* Reference elevation + cell size + bench interval */}
              <div className="grid grid-cols-3 gap-2">
                <NumberField
                  label="Reference elev (m)"
                  value={referenceElevation}
                  step={0.1}
                  onChange={setReferenceElevation}
                />
                <NumberField
                  label="Cell size (m)"
                  value={cellSize}
                  step={0.1}
                  onChange={setCellSize}
                />
                <NumberField
                  label="Bench (m)"
                  value={benchInterval}
                  step={0.5}
                  onChange={setBenchInterval}
                />
              </div>

              {/* Metadata */}
              <div className="grid grid-cols-3 gap-2">
                <TextField label="Customer" value={customer} onChange={setCustomer} placeholder="e.g., BHP Iron Ore" />
                <TextField label="Site / Pit" value={site} onChange={setSite} placeholder="e.g., Pit A — June" />
                <TextField label="Surveyor" value={surveyor} onChange={setSurveyor} placeholder="e.g., J. Smith" />
              </div>

              {/* PDF report path */}
              <div>
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  PDF report output
                </label>
                <div className="flex items-center gap-2">
                  <button
                    onClick={async () => {
                      const p = await pickSaveFile({
                        extensions: ["pdf"],
                        filterName: "PDF report",
                        title: "Save EOM audit report",
                      });
                      if (p) setReportPath(p);
                    }}
                    className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-1.5 text-xs text-white hover:bg-navy-elevated"
                  >
                    <Download className="h-3.5 w-3.5" /> Save As
                  </button>
                  <input
                    type="text"
                    value={reportPath}
                    onChange={(e) => setReportPath(e.target.value)}
                    className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
                  />
                </div>
              </div>

              {/* Run button */}
              <button
                onClick={handleRun}
                disabled={!canRun}
                className="flex w-full items-center justify-center gap-2 rounded-md px-4 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.industrialOrange, color: colors.navyBase }}
              >
                {running ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Calculator className="h-4 w-4" />
                )}
                {running ? "Auditing…" : "Run Volumetric Audit"}
              </button>

              {/* Watch folder section */}
              <WatchFolderSection
                watchPath={watchPath}
                onWatchPathChange={setWatchPath}
                onPickFolder={async () => {
                  const p = await pickFolder("Select EOM watch folder");
                  if (p) setWatchPath(p);
                }}
                watchRunning={watchRunning}
                onStart={handleStartWatch}
                onStop={handleStopWatch}
                events={watchEvents}
              />
            </div>

            {/* ═══ RIGHT: Progress + Results ═══ */}
            <div className="space-y-4">
              {/* Progress panel */}
              <div className="rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Pipeline Progress
                  </span>
                  {running && (
                    <span className="text-[10px] text-steel-light">{stageMessage}</span>
                  )}
                </div>
                <div className="flex items-center gap-1.5">
                  {STAGE_LABELS.map((s, idx) => {
                    const isDone = activeStage > idx || activeStage === STAGE_LABELS.length;
                    const isActive = activeStage === idx;
                    return (
                      <div key={s.kind} className="flex flex-1 flex-col items-center gap-1">
                        <div
                          className="h-2.5 w-full rounded-full transition-colors"
                          style={{
                            background: isDone
                              ? colors.pass
                              : isActive
                                ? colors.industrialOrange
                                : colors.navyElevated,
                            boxShadow: isActive
                              ? `0 0 0 2px ${colors.industrialOrange}40`
                              : "none",
                          }}
                        />
                        <span
                          className="text-[8px] uppercase tracking-wider"
                          style={{
                            color: isDone
                              ? colors.pass
                              : isActive
                                ? colors.industrialOrange
                                : colors.steelGray,
                          }}
                        >
                          {s.label}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* Empty state or results */}
              {!result ? (
                <div className="flex h-72 flex-col items-center justify-center rounded-md border border-dashed border-navy-border bg-navy-base p-8 text-center">
                  <Eye className="mb-2 h-8 w-8 text-steel-gray" />
                  <div className="text-sm font-medium text-steel-light">No audit results yet</div>
                  <div className="mt-1 max-w-xs text-[11px] leading-relaxed text-steel-gray">
                    Pick a current survey LAS/LAZ, set the reference elevation and bench interval,
                    then click <span className="font-semibold text-white">Run Volumetric Audit</span>.
                  </div>
                </div>
              ) : (
                <ResultPanel
                  result={result}
                  copied={copied}
                  generating={generating}
                  reportGenerated={reportGenerated}
                  onCopy={handleCopy}
                  onGenerateReport={handleGenerateReport}
                  licenseBlocked={
                    license?.state === "Exhausted" || license?.state === "Expired"
                  }
                />
              )}
            </div>
          </div>
        </div>

        {/* ─── Footer ─── */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <span>
            CSF cloth simulation · IDW DEM rasterization · 2.5D matrix subtraction ·
            SHA-256 audit trail
          </span>
          <button
            onClick={onClose}
            className="rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */
/* Sub-components                                                 */
/* ──────────────────────────────────────────────────────────── */

function LicenseBanner({ status }: { status: LicenseStatusRpc | null }) {
  if (!status) {
    return (
      <div
        className="border-b border-navy-border px-5 py-2 text-[11px] text-steel-gray"
        style={{ background: colors.navyBase }}
      >
        Checking license status…
      </div>
    );
  }
  const tone = licenseTone(status);
  return (
    <div
      className="flex items-center justify-between gap-3 border-b px-5 py-2 text-[11px]"
      style={{
        background: `${tone.color}10`,
        borderColor: `${tone.color}40`,
        color: tone.color,
      }}
    >
      <div className="flex items-center gap-2">
        <KeyRound className="h-3.5 w-3.5 flex-shrink-0" />
        <span className="font-semibold uppercase tracking-wider">{tone.label}</span>
        <span className="text-steel-light">{tone.detail}</span>
      </div>
      {tone.action && (
        <span className="text-[10px] text-steel-gray">{tone.action}</span>
      )}
    </div>
  );
}

function licenseTone(status: LicenseStatusRpc): {
  label: string;
  detail: string;
  color: string;
  action?: string;
} {
  switch (status.state) {
    case "Trial":
      return {
        label: "Trial mode",
        detail: `${status.trial_reports_remaining} report credit${
          status.trial_reports_remaining === 1 ? "" : "s"
        } remaining`,
        color: colors.investigate,
        action: "Activate a license for unlimited reports + watch folder",
      };
    case "Active":
      return {
        label: "Licensed",
        detail: `${status.customer} · ${status.tier}${
          status.reports_remaining != null
            ? ` · ${status.reports_remaining} reports left`
            : " · unlimited"
        }`,
        color: colors.pass,
      };
    case "Exhausted":
      return {
        label: "Report quota exhausted",
        detail: `${status.customer} — contact sales to renew`,
        color: colors.fail,
        action: "Generating new reports is blocked until the license is renewed",
      };
    case "Expired":
      return {
        label: "License expired",
        detail: `${status.customer} — expired ${status.expired_at}`,
        color: colors.fail,
        action: "Renew via the License Manager to resume audits",
      };
    case "Invalid":
      return {
        label: "Invalid license",
        detail: status.reason,
        color: colors.fail,
        action: "Trial mode is unavailable — install a valid license",
      };
  }
}

function VolumeTile({
  label,
  value,
  unit,
  color,
  icon,
  cells,
}: {
  label: string;
  value: number;
  unit: string;
  color: string;
  icon: React.ReactNode;
  cells: number;
}) {
  return (
    <div
      className="rounded-md border p-3"
      style={{ borderColor: `${color}40`, background: `${color}10` }}
    >
      <div
        className="flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider"
        style={{ color }}
      >
        {icon} {label}
      </div>
      <div className="mt-1.5 font-mono text-xl font-bold text-white">
        {value.toLocaleString(undefined, { maximumFractionDigits: 1 })}
        <span className="ml-1 text-[10px] font-normal text-steel-gray">{unit}</span>
      </div>
      <div className="mt-0.5 font-mono text-[10px] text-steel-light">
        {cells.toLocaleString()} cells
      </div>
    </div>
  );
}

function ResultPanel({
  result,
  copied,
  generating,
  reportGenerated,
  onCopy,
  onGenerateReport,
  licenseBlocked,
}: {
  result: EomOutputRpc;
  copied: boolean;
  generating: boolean;
  reportGenerated: boolean;
  onCopy: () => void;
  onGenerateReport: () => void;
  licenseBlocked: boolean;
}) {
  const v = result.volumes;
  const groundPct =
    (result.ground_points / Math.max(1, result.points_read)) * 100;

  return (
    <div className="space-y-3">
      {/* Volume summary tiles */}
      <div className="grid grid-cols-3 gap-2">
        <VolumeTile
          label="Fill"
          value={v.fill_volume}
          unit="m³"
          color={colors.pass}
          icon={<TrendingUp className="h-3 w-3" />}
          cells={v.fill_cells}
        />
        <VolumeTile
          label="Cut"
          value={v.cut_volume}
          unit="m³"
          color={colors.fail}
          icon={<TrendingDown className="h-3 w-3" />}
          cells={v.cut_cells}
        />
        <VolumeTile
          label="Net"
          value={v.net_volume}
          unit="m³"
          color={colors.industrialOrange}
          icon={<Calculator className="h-3 w-3" />}
          cells={v.fill_cells + v.cut_cells}
        />
      </div>

      {/* Survey metadata */}
      <div className="rounded-md border border-navy-border bg-navy-base p-3">
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
          Survey Metadata
        </div>
        <div className="grid grid-cols-2 gap-x-4 gap-y-1.5 text-[11px]">
          <MetaRow
            icon={<Database className="h-3 w-3" />}
            label="Point count"
            value={result.points_read.toLocaleString()}
          />
          <MetaRow
            icon={<Layers className="h-3 w-3" />}
            label="Ground %"
            value={`${groundPct.toFixed(1)}% (${result.ground_points.toLocaleString()})`}
          />
          <MetaRow
            icon={<Grid3x3 className="h-3 w-3" />}
            label="DEM dims"
            value={`${result.dem_cols} × ${result.dem_rows}`}
          />
          <MetaRow
            icon={<Grid3x3 className="h-3 w-3" />}
            label="Cell size"
            value={result.dem_cell_size.toFixed(2)}
          />
        </div>
      </div>

      {/* Audit trail (SHA-256 hashes) */}
      <div className="rounded-md border border-navy-border bg-navy-base p-3">
        <div className="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
          <Hash className="h-3 w-3" /> Audit Trail · SHA-256
        </div>
        <HashRow label="Audit Hash" hash={result.audit_hash} />
        <div className="mt-1.5 font-mono text-[10px] text-steel-gray">
          Processing time: {(result.processing_time_ms / 1000).toFixed(2)} s
        </div>
      </div>

      {/* Warnings panel */}
      {result.warnings.length > 0 && (
        <div
          className="rounded-md border p-3"
          style={{
            borderColor: `${colors.investigate}40`,
            background: `${colors.investigate}10`,
          }}
        >
          <div
            className="mb-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider"
            style={{ color: colors.investigate }}
          >
            <AlertTriangle className="h-3 w-3" /> Warnings ({result.warnings.length})
          </div>
          <ul className="space-y-1 text-[11px] text-steel-light">
            {result.warnings.map((w, i) => (
              <li key={i} className="leading-relaxed">
                • {w}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Action buttons */}
      <div className="flex items-center gap-2">
        <button
          onClick={onCopy}
          className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-3 py-1.5 text-xs text-white hover:bg-navy-elevated"
        >
          {copied ? (
            <CheckCircle2 className="h-3 w-3" style={{ color: colors.pass }} />
          ) : (
            <Copy className="h-3 w-3" />
          )}
          {copied ? "Copied!" : "Copy summary"}
        </button>
        <button
          onClick={onGenerateReport}
          disabled={generating || licenseBlocked}
          className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40"
          style={{ background: colors.industrialOrange, color: colors.navyBase }}
        >
          {generating ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : (
            <FileText className="h-3 w-3" />
          )}
          {reportGenerated ? "Report ✓" : "Generate PDF"}
        </button>
      </div>
      {reportGenerated && (
        <div className="text-[10px] text-steel-gray">
          Report written to: <span className="font-mono text-steel-light">/tmp/eom_auditor_report.pdf</span>
        </div>
      )}
    </div>
  );
}

function MetaRow({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="flex items-center gap-1 text-steel-gray">
        {icon}
        {label}
      </span>
      <span className="font-mono text-steel-light">{value}</span>
    </div>
  );
}

function HashRow({ label, hash }: { label: string; hash: string }) {
  return (
    <div className="mb-1 last:mb-0">
      <div className="text-[10px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="truncate font-mono text-[10px] text-steel-light" title={hash}>
        {hash}
      </div>
    </div>
  );
}

function WatchFolderSection({
  watchPath,
  onWatchPathChange,
  onPickFolder,
  watchRunning,
  onStart,
  onStop,
  events,
}: {
  watchPath: string;
  onWatchPathChange: (v: string) => void;
  onPickFolder: () => void;
  watchRunning: boolean;
  onStart: () => void;
  onStop: () => void;
  events: EomWatchEventRpc[];
}) {
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
          <FolderOpen className="h-3 w-3" /> Watch Folder (zero-touch ingest)
        </span>
        <span
          className="flex items-center gap-1 text-[10px]"
          style={{ color: watchRunning ? colors.pass : colors.steelGray }}
        >
          <span
            className="h-1.5 w-1.5 rounded-full"
            style={{ background: watchRunning ? colors.pass : colors.steelGray }}
          />
          {watchRunning ? "Running" : "Stopped"}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <button
          onClick={onPickFolder}
          className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-panel px-2.5 py-1.5 text-xs text-white hover:bg-navy-elevated"
        >
          <FolderOpen className="h-3.5 w-3.5" /> Browse
        </button>
        <input
          type="text"
          value={watchPath}
          onChange={(e) => onWatchPathChange(e.target.value)}
          placeholder="/path/to/incoming-las-folder"
          className="flex-1 rounded-md border border-navy-border bg-navy-panel px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
        />
        {watchRunning ? (
          <button
            onClick={onStop}
            className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium"
            style={{ background: colors.fail, color: colors.navyBase }}
          >
            <Square className="h-3 w-3" /> Stop
          </button>
        ) : (
          <button
            onClick={onStart}
            disabled={!watchPath}
            className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40"
            style={{ background: colors.pass, color: colors.navyBase }}
          >
            <Play className="h-3 w-3" /> Start
          </button>
        )}
      </div>

      {/* Recent activity log */}
      <div className="mt-3">
        <div className="mb-1 text-[10px] uppercase tracking-wider text-steel-gray">
          Recent activity
        </div>
        {events.length === 0 ? (
          <div className="rounded border border-dashed border-navy-border px-2 py-2 text-[11px] text-steel-gray">
            No activity yet. Start the watcher to auto-process new LAS files dropped into the folder.
          </div>
        ) : (
          <div className="max-h-32 space-y-1 overflow-y-auto">
            {events.map((ev, i) => (
              <WatchEventRow key={i} ev={ev} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function WatchEventRow({ ev }: { ev: EomWatchEventRpc }) {
  const tone =
    ev.kind === "completed"
      ? colors.pass
      : ev.kind === "failed"
        ? colors.fail
        : colors.info;
  const fileName = ev.file_path.split(/[\\/]/).pop() ?? ev.file_path;
  return (
    <div
      className="flex items-center justify-between gap-2 rounded border border-navy-border bg-navy-panel px-2 py-1 text-[10px]"
    >
      <span className="flex min-w-0 items-center gap-1.5">
        <span
          className="h-1.5 w-1.5 flex-shrink-0 rounded-full"
          style={{ background: tone }}
        />
        <span className="truncate font-mono text-steel-light" title={ev.file_path}>
          {fileName}
        </span>
      </span>
      <span className="flex-shrink-0 font-mono text-steel-gray">
        {ev.kind === "completed" &&
          `fill ${ev.fill_volume?.toFixed(0) ?? "—"} m³ · cut ${ev.cut_volume?.toFixed(0) ?? "—"} m³`}
        {ev.kind === "failed" && (ev.error ?? "failed")}
        {ev.kind === "started" && "processing…"}
      </span>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */
/* Form field primitives                                          */
/* ──────────────────────────────────────────────────────────── */

function PathField({
  label,
  value,
  onPicked,
  onPick,
  dropdownOptions,
  dropdownValue,
  onDropdownChange,
}: {
  label: string;
  value: string;
  onPicked: (v: string) => void;
  onPick: () => Promise<string | null>;
  dropdownOptions: { label: string; value: string }[];
  dropdownValue: string;
  onDropdownChange: (v: string) => void;
}) {
  async function handleBrowse() {
    const p = await onPick();
    if (p) onPicked(p);
  }
  return (
    <div>
      <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
        {label}
      </label>
      <div className="flex items-center gap-2">
        <button
          onClick={handleBrowse}
          className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-1.5 text-xs text-white hover:bg-navy-elevated"
        >
          <FolderOpen className="h-3.5 w-3.5" /> Browse
        </button>
        {dropdownOptions.length > 0 && (
          <select
            value={dropdownValue}
            onChange={(e) => onDropdownChange(e.target.value)}
            className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
          >
            <option value="">— Or pick loaded —</option>
            {dropdownOptions.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        )}
      </div>
      {value && (
        <div className="mt-0.5 truncate font-mono text-[10px] text-steel-light">
          ✓ {value.split(/[\\/]/).pop() ?? value}
        </div>
      )}
    </div>
  );
}

function NumberField({
  label,
  value,
  step,
  onChange,
}: {
  label: string;
  value: number;
  step: number;
  onChange: (v: number) => void;
}) {
  return (
    <div>
      <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
        {label}
      </label>
      <input
        type="number"
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
        className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none"
      />
    </div>
  );
}

function TextField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <div>
      <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
        {label}
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
      />
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */
/* Helpers                                                        */
/* ──────────────────────────────────────────────────────────── */

function formatStageMessage(p: EomProgressRpc): string {
  switch (p.kind) {
    case "Started":
      return "Initializing pipeline…";
    case "ReadingCurrentLas":
      return `Reading ${p.message.toLocaleString()} points…`;
    case "ClassifyingCurrent":
      return `Classified ${p.message.toLocaleString()} ground points…`;
    case "RasterizingCurrentDem":
      return `Building DEM ${p.message[0]} × ${p.message[1]}…`;
    case "ReadingPreviousLas":
      return `Reading previous survey (${p.message.toLocaleString()} pts)…`;
    case "ClassifyingPrevious":
      return `Previous survey: ${p.message.toLocaleString()} ground pts…`;
    case "RasterizingPreviousDem":
      return `Previous DEM ${p.message[0]} × ${p.message[1]}…`;
    case "ComputingVolumes":
      return "Subtracting DEMs · bench breakdown…";
    case "HashingFiles":
      return "Computing SHA-256 audit hashes…";
    case "Done":
      return "Done";
  }
}
