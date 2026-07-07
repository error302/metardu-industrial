import { withReportProfile } from "@/lib/report-profile";
/**
 * Highwall Monitoring Wizard — Sprint 5 Revenue Feature #6.
 *
 * Step-by-step wizard for mine surveyors to produce a regulator-ready
 * Highwall Deformation Compliance Report.
 *
 * Post-Brumadinho 2019, slope stability monitoring is legally required
 * in many jurisdictions. This wizard:
 *   1. Collects N sequential TLS/drone DEM scans (epochs)
 *   2. Asks for survey dates + threshold settings
 *   3. Runs the highwall analysis (per-cell time-series + alerts)
 *   4. Reviews alert summary and trend classifications
 *   5. Generates a branded PDF compliance report
 *
 * Revenue: $5,000-10,000/site/year — safety-critical = non-negotiable.
 */

import { useState } from "react";
import {
  FileText, Loader2, CheckCircle2,
  AlertTriangle, ShieldAlert, Plus, Trash2,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  analyzeHighwall,
  generateReport,
  type HighwallReport,
  type AlertLevel,
  type TrendClass,
  type ReportSpec,
  type ReportTable,
  type ReportStat,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Step = 1 | 2 | 3 | 4 | 5;

const ALERT_COLORS: Record<AlertLevel, string> = {
  none: colors.pass,
  advisory: colors.warn,
  watch: colors.accent,
  critical: colors.failDim,
};

const ALERT_LABELS: Record<AlertLevel, string> = {
  none: "Stable",
  advisory: "Advisory",
  watch: "Watch",
  critical: "Critical",
};

const TREND_LABELS: Record<TrendClass, string> = {
  stable: "Stable",
  creeping: "Creeping",
  accelerating: "Accelerating",
  failure_imminent: "Failure Imminent",
};

export function HighwallMonitoringWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  // Epoch DEM paths + dates
  const [epochs, setEpochs] = useState<{ path: string; date: string }[]>([
    { path: "", date: "2026-04-01" },
    { path: "", date: "2026-05-01" },
  ]);

  // Thresholds (USACE defaults)
  const [advisoryMm, setAdvisoryMm] = useState(25);
  const [watchMm, setWatchMm] = useState(50);
  const [criticalMm, setCriticalMm] = useState(100);
  const [velocityWatch, setVelocityWatch] = useState(1.0);
  const [velocityCritical, setVelocityCritical] = useState(5.0);

  // Project metadata
  const [siteName, setSiteName] = useState("");
  const [clientName, setClientName] = useState("");
  const [regulatorName, setRegulatorName] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/highwall_compliance.html");

  const [analyzing, setAnalyzing] = useState(false);
  const [result, setResult] = useState<HighwallReport | null>(null);
  const [generating, setGenerating] = useState(false);
    const [error, setError] = useState<string | null>(null);
  const [step, setStep] = useState<Step>(1);

      const validEpochs = epochs.filter((e) => e.path && e.date);
  
  function addEpoch() {
    setEpochs([...epochs, { path: "", date: "" }]);
  }
  function removeEpoch(idx: number) {
    setEpochs(epochs.filter((_, i) => i !== idx));
  }
  function updateEpoch(idx: number, field: "path" | "date", value: string) {
    setEpochs(epochs.map((e, i) => (i === idx ? { ...e, [field]: value } : e)));
  }

  async function handleAnalyze() {
    setAnalyzing(true);
    setError(null);
    setResult(null);
    try {
      const r = await analyzeHighwall({
        paths: validEpochs.map((e) => e.path),
        epochDates: validEpochs.map((e) => e.date),
        cellAreaM2: 1.0,
        thresholds: {
          advisory_mm: advisoryMm,
          watch_mm: watchMm,
          critical_mm: criticalMm,
          velocity_watch_mm_per_day: velocityWatch,
          velocity_critical_mm_per_day: velocityCritical,
        },
      });
      if (r) {
        setResult(r);
        setStep(4);
      } else {
        setError("Browser mode — analysis requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setAnalyzing(false);
    }
  }

  async function handleGenerateReport() {
    if (!result) return;
    setGenerating(true);
    setError(null);
    try {
      const summary: ReportStat[] = [
        { label: "Total Cells", value: result.total_cells.toLocaleString(), unit: "", color: colors.steelLight },
        { label: "Active Cells", value: result.active_cells.toLocaleString(), unit: "", color: colors.steelLight },
        { label: "Stable", value: result.stats.stable_cells.toLocaleString(), unit: "", color: ALERT_COLORS.none },
        { label: "Advisory", value: result.stats.advisory_cells.toLocaleString(), unit: "", color: ALERT_COLORS.advisory },
        { label: "Watch", value: result.stats.watch_cells.toLocaleString(), unit: "", color: ALERT_COLORS.watch },
        { label: "Critical", value: result.stats.critical_cells.toLocaleString(), unit: "", color: ALERT_COLORS.critical },
        { label: "Max Displacement", value: result.stats.max_cumulative_mm.toFixed(1), unit: "mm", color: result.stats.max_cumulative_mm > criticalMm ? ALERT_COLORS.critical : ALERT_COLORS.watch },
        { label: "Max Velocity", value: result.stats.max_velocity_mm_per_day.toFixed(2), unit: "mm/day", color: result.stats.max_velocity_mm_per_day > velocityCritical ? ALERT_COLORS.critical : ALERT_COLORS.watch },
        { label: "Mean Cumulative", value: result.stats.mean_cumulative_mm.toFixed(1), unit: "mm", color: colors.steelLight },
        { label: "Accelerating", value: result.stats.cells_with_acceleration.toLocaleString(), unit: "", color: ALERT_COLORS.watch },
        { label: "Failure Imminent", value: result.stats.failure_imminent_cells.toLocaleString(), unit: "", color: ALERT_COLORS.critical },
        { label: "Compliance", value: result.stats.compliance_pct.toFixed(1), unit: "%", color: result.stats.compliance_pct > 95 ? ALERT_COLORS.none : ALERT_COLORS.advisory },
      ];

      const alertTable: ReportTable = {
        title: "Alert Summary",
        headers: ["Alert Level", "Cell Count", "% of Active", "Action Required"],
        rows: [
          ["Stable (no alert)", result.stats.stable_cells.toLocaleString(),
            pct(result.stats.stable_cells, result.active_cells), "None — routine monitoring"],
          ["Advisory (>25mm)", result.stats.advisory_cells.toLocaleString(),
            pct(result.stats.advisory_cells, result.active_cells), "Log in register, no notification"],
          ["Watch (>50mm OR >1mm/day)", result.stats.watch_cells.toLocaleString(),
            pct(result.stats.watch_cells, result.active_cells), "Notify surveyor + increase frequency"],
          ["Critical (>100mm OR >5mm/day)", result.stats.critical_cells.toLocaleString(),
            pct(result.stats.critical_cells, result.active_cells), "IMMEDIATE — notify engineer, halt operations"],
        ],
      };

      const topAlerts: ReportTable = {
        title: "Top Critical Alerts (by cumulative displacement)",
        headers: ["Cell (row,col)", "Cumulative (mm)", "Velocity (mm/day)", "Trend", "Message"],
        rows: result.alerts
          .filter((a) => a.level === "critical")
          .sort((a, b) => b.cumulative_mm - a.cumulative_mm)
          .slice(0, 20)
          .map((a) => [
            `(${a.row}, ${a.col})`,
            a.cumulative_mm.toFixed(1),
            a.velocity_mm_per_day.toFixed(2),
            TREND_LABELS[a.trend],
            a.message,
          ]),
      };

      const profileFields = await withReportProfile();
      const spec: ReportSpec = {
        ...profileFields,
        report_type: "highwall_report",
        title: "Highwall Deformation Compliance Report",
        subtitle: siteName
          ? `${siteName} — ${new Date().toLocaleDateString()}`
          : new Date().toLocaleDateString(),
        client: clientName,
        metadata: {
          Site: siteName || "(unspecified)",
          Regulator: regulatorName || "(unspecified)",
          Epochs: `${result.n_epochs} surveys`,
          "Date Range": `${result.epoch_dates[0] ?? "—"} to ${result.epoch_dates[result.epoch_dates.length - 1] ?? "—"}`,
          "Cell Area": `${result.cell_area_m2.toFixed(2)} m²`,
          Thresholds: `advisory ${advisoryMm}mm / watch ${watchMm}mm / critical ${criticalMm}mm`,
        },
        tables: [alertTable, topAlerts],
        summary,
        provenance_hash: `highwall-${Date.now().toString(36)}`,
        output_path: reportPath,
      };

      const r = await generateReport(spec);
      if (r) {
                setStep(5);
      } else {
        setError("Browser mode — report generation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setGenerating(false);
    }
  }

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Highwall Deformation Monitoring"
      icon={<ShieldAlert className="h-4 w-4" />}
      iconColor={colors.fail}
      maxWidth="max-w-3xl"
      subtitle="USACE compliant"
      footerHint="Per-cell displacement + alerts"
      actions={
        <>
          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
        </>
      }
    >
          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Step 1: Select epoch DEMs */}
          {step === 1 && (
            <div className="space-y-3">
              <p className="text-xs text-steel-gray">
                Select 2+ sequential TLS or drone DEM scans. Each epoch needs a survey date
                (YYYY-MM-DD) for velocity calculation. Minimum 2 epochs required.
              </p>
              {epochs.map((epoch, i) => (
                <div key={i} className="flex items-center gap-2 rounded-md border border-navy-border bg-navy-base p-2">
                  <span className="rounded px-2 py-1 text-[10px] font-bold" style={{ background: `${colors.failDim}20`, color: colors.failDim }}>
                    EPOCH {i + 1}
                  </span>
                  <select
                    value={epoch.path}
                    onChange={(e) => updateEpoch(i, "path", e.target.value)}
                    className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
                  >
                    <option value="">— Select DEM —</option>
                    {geotiffFiles.map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                  <input
                    type="date"
                    value={epoch.date}
                    onChange={(e) => updateEpoch(i, "date", e.target.value)}
                    className="rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
                  />
                  {epochs.length > 2 && (
                    <button onClick={() => removeEpoch(i)} className="rounded p-1 text-steel-gray hover:text-fail">
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  )}
                </div>
              ))}
              <button
                onClick={addEpoch}
                className="flex items-center gap-1.5 rounded-md border border-dashed border-navy-border px-3 py-2 text-xs text-steel-light hover:border-steel-light hover:text-white"
              >
                <Plus className="h-3.5 w-3.5" /> Add Epoch
              </button>
              {geotiffFiles.length === 0 && (
                <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-gray">
                  Drop GeoTIFF DEM files (one per epoch) on the map first.
                </div>
              )}
            </div>
          )}

          {/* Step 2: Thresholds + metadata */}
          {step === 2 && (
            <div className="space-y-4">
              <div className="rounded-md border p-3" style={{ borderColor: `${colors.failDim}40`, background: `${colors.failDim}10` }}>
                <div className="mb-2 flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider" style={{ color: colors.failDim }}>
                  <AlertTriangle className="h-3 w-3" /> Thresholds (USACE EM 1110-2-1900 defaults)
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <ThresholdField label="Advisory (mm)" value={advisoryMm} onChange={setAdvisoryMm} color={colors.warn} />
                  <ThresholdField label="Watch (mm)" value={watchMm} onChange={setWatchMm} color={colors.accent} />
                  <ThresholdField label="Critical (mm)" value={criticalMm} onChange={setCriticalMm} color={colors.failDim} />
                  <ThresholdField label="Velocity Watch (mm/day)" value={velocityWatch} onChange={setVelocityWatch} color={colors.accent} step={0.1} />
                  <ThresholdField label="Velocity Critical (mm/day)" value={velocityCritical} onChange={setVelocityCritical} color={colors.failDim} step={0.1} />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Site / Pit name
                  </label>
                  <input
                    type="text" value={siteName} onChange={(e) => setSiteName(e.target.value)}
                    placeholder="e.g., Pit B — North Highwall"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Client / Mine name
                  </label>
                  <input
                    type="text" value={clientName} onChange={(e) => setClientName(e.target.value)}
                    placeholder="e.g., Vale — Brucutu Mine"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Regulator (compliance recipient)
                  </label>
                  <input
                    type="text" value={regulatorName} onChange={(e) => setRegulatorName(e.target.value)}
                    placeholder="e.g., ANM (Brazil) / DMP (Australia)"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Report output path
                  </label>
                  <input
                    type="text" value={reportPath} onChange={(e) => setReportPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                </div>
              </div>
            </div>
          )}

          {/* Step 3: Analyze */}
          {step === 3 && (
            <div className="flex flex-col items-center justify-center py-10">
              <p className="mb-4 text-sm text-steel-light">
                Ready to analyze {validEpochs.length} epochs:
              </p>
              <div className="mb-4 w-full max-w-md rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                {validEpochs.map((e, i) => (
                  <div key={i} className="font-mono text-steel-light">
                    {i + 1}. {e.date} — {e.path.split(/[\\/]/).pop()}
                  </div>
                ))}
                <div className="mt-2 text-steel-gray">
                  Thresholds: advisory {advisoryMm}mm / watch {watchMm}mm / critical {criticalMm}mm
                </div>
              </div>
              <button
                onClick={handleAnalyze}
                disabled={analyzing}
                className="flex items-center gap-2 rounded-md px-6 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.failDim, color: "white" }}
              >
                {analyzing ? <Loader2 className="h-4 w-4 animate-spin" /> : <ShieldAlert className="h-4 w-4" />}
                {analyzing ? "Analyzing…" : "Run Highwall Analysis"}
              </button>
            </div>
          )}

          {/* Step 4: Report */}
          {step === 4 && result && (
            <div className="space-y-4">
              <div className="grid grid-cols-4 gap-2">
                <ResultTile label="Total Cells" value={result.total_cells.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Active Cells" value={result.active_cells.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Stable" value={result.stats.stable_cells.toLocaleString()} color={ALERT_COLORS.none} />
                <ResultTile label="Advisory" value={result.stats.advisory_cells.toLocaleString()} color={ALERT_COLORS.advisory} />
                <ResultTile label="Watch" value={result.stats.watch_cells.toLocaleString()} color={ALERT_COLORS.watch} />
                <ResultTile label="Critical" value={result.stats.critical_cells.toLocaleString()} color={ALERT_COLORS.critical} />
                <ResultTile label="Max Disp (mm)" value={result.stats.max_cumulative_mm.toFixed(1)} color={result.stats.max_cumulative_mm > criticalMm ? ALERT_COLORS.critical : ALERT_COLORS.watch} />
                <ResultTile label="Max Vel (mm/d)" value={result.stats.max_velocity_mm_per_day.toFixed(2)} color={result.stats.max_velocity_mm_per_day > velocityCritical ? ALERT_COLORS.critical : ALERT_COLORS.watch} />
                <ResultTile label="Mean Disp (mm)" value={result.stats.mean_cumulative_mm.toFixed(1)} color={colors.steelLight} />
                <ResultTile label="Accelerating" value={result.stats.cells_with_acceleration.toLocaleString()} color={ALERT_COLORS.watch} />
                <ResultTile label="Failure Imminent" value={result.stats.failure_imminent_cells.toLocaleString()} color={ALERT_COLORS.critical} />
                <ResultTile label="Compliance" value={`${result.stats.compliance_pct.toFixed(1)}%`} color={result.stats.compliance_pct > 95 ? ALERT_COLORS.none : ALERT_COLORS.advisory} />
              </div>

              {result.stats.critical_cells > 0 && (
                <div
                  className="flex items-start gap-2 rounded-md border p-3 text-xs"
                  style={{ borderColor: `${ALERT_COLORS.critical}60`, background: `${ALERT_COLORS.critical}10`, color: ALERT_COLORS.critical }}
                >
                  <AlertTriangle className="mt-0.5 h-4 w-4 flex-shrink-0" />
                  <div>
                    <div className="font-semibold">CRITICAL ALERTS ACTIVE</div>
                    <div className="mt-0.5 text-[10px]">
                      {result.stats.critical_cells.toLocaleString()} cells exceed critical threshold
                      ({criticalMm}mm cumulative OR {velocityCritical}mm/day velocity).
                      {result.stats.failure_imminent_cells > 0 && ` ${result.stats.failure_imminent_cells} cells show accelerating failure trend — halt operations immediately.`}
                    </div>
                  </div>
                </div>
              )}

              {/* Top alerts */}
              {result.alerts.length > 0 && (
                <div>
                  <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    All Alerts ({result.alerts.length})
                  </h4>
                  <div className="max-h-32 overflow-y-auto rounded-md border border-navy-border">
                    <table className="w-full text-left text-[10px]">
                      <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                        <tr>
                          <th className="px-2 py-1.5">Cell</th>
                          <th className="px-2 py-1.5 text-right">Cum. (mm)</th>
                          <th className="px-2 py-1.5 text-right">Vel. (mm/d)</th>
                          <th className="px-2 py-1.5">Level</th>
                          <th className="px-2 py-1.5">Trend</th>
                        </tr>
                      </thead>
                      <tbody>
                        {result.alerts.slice(0, 30).map((a, i) => (
                          <tr key={i} className="border-t border-navy-border">
                            <td className="px-2 py-1 font-mono text-steel-light">({a.row}, {a.col})</td>
                            <td className="px-2 py-1 text-right font-mono text-white">{a.cumulative_mm.toFixed(1)}</td>
                            <td className="px-2 py-1 text-right font-mono text-steel-light">{a.velocity_mm_per_day.toFixed(2)}</td>
                            <td className="px-2 py-1" style={{ color: ALERT_COLORS[a.level] }}>● {ALERT_LABELS[a.level]}</td>
                            <td className="px-2 py-1 text-steel-light">{TREND_LABELS[a.trend]}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}

              <button
                onClick={handleGenerateReport}
                disabled={generating}
                className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.failDim, color: "white" }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
                {generating ? "Generating report…" : "Generate Compliance Report"}
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Compliance Report Complete</h3>
              <p className="mt-1 text-sm text-steel-light">
                Report written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready output.
                Submit to {regulatorName || "regulator"} per local compliance schedule.
              </p>
              {result && (
                <div className="mt-4 grid grid-cols-4 gap-2 text-center">
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">Advisory</div>
                    <div className="font-mono text-sm font-bold" style={{ color: ALERT_COLORS.advisory }}>
                      {result.stats.advisory_cells}
                    </div>
                  </div>
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">Watch</div>
                    <div className="font-mono text-sm font-bold" style={{ color: ALERT_COLORS.watch }}>
                      {result.stats.watch_cells}
                    </div>
                  </div>
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">Critical</div>
                    <div className="font-mono text-sm font-bold" style={{ color: ALERT_COLORS.critical }}>
                      {result.stats.critical_cells}
                    </div>
                  </div>
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">Compliance</div>
                    <div className="font-mono text-sm font-bold" style={{ color: result.stats.compliance_pct > 95 ? ALERT_COLORS.none : ALERT_COLORS.advisory }}>
                      {result.stats.compliance_pct.toFixed(1)}%
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
    </DialogShell>
  );
}

function pct(part: number, total: number): string {
  if (total === 0) return "0.0%";
  return `${((part / total) * 100).toFixed(1)}%`;
}

function ThresholdField({
  label, value, onChange, color, step = 1,
}: {
  label: string; value: number; onChange: (v: number) => void; color: string; step?: number;
}) {
  return (
    <div>
      <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider" style={{ color }}>
        {label}
      </label>
      <input
        type="number" step={step} value={value}
        onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
        className="w-full rounded-md border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none"
        style={{ borderColor: `${color}60` }}
      />
    </div>
  );
}

function ResultTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-sm font-bold text-white">{value}</div>
    </div>
  );
}
