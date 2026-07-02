/**
 * EoM Reconciliation Wizard — Sprint 2 Revenue Feature #1.
 *
 * Step-by-step wizard for mine surveyors to produce an audit-ready
 * End-of-Month Production Reconciliation Report.
 *
 * Workflow:
 *   1. Select previous survey DEM (GeoTIFF)
 *   2. Select current survey DEM (GeoTIFF)
 *   3. Configure: rock density, bench interval, pit perimeter
 *   4. Compute: volume delta, tonnage, bench breakdown
 *   5. Generate: branded PDF reconciliation report
 *
 * Revenue: $3,000-5,000/seat/year — highest probability revenue feature.
 */

import { useState } from "react";
import {
  X, ArrowRight, ArrowLeft, FileText, Loader2, CheckCircle2,
  Database, Calculator, Download,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  computeVolumes,
  generateReport,
  type VolumeResultRpc,
  type ReportSpec,
  type ReportTable,
  type ReportStat,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Step = 1 | 2 | 3 | 4 | 5;

const STEP_LABELS = ["Surveys", "Parameters", "Compute", "Report", "Done"];

export function EomReconciliationWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [step, setStep] = useState<Step>(1);
  const [prevPath, setPrevPath] = useState("");
  const [currPath, setCurrPath] = useState("");
  const [density, setDensity] = useState(2.7);
  const [benchInterval, setBenchInterval] = useState(5);
  const [clientName, setClientName] = useState("");
  const [siteName, setSiteName] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/eom_reconciliation.html");

  const [computing, setComputing] = useState(false);
  const [volumeResult, setVolumeResult] = useState<VolumeResultRpc | null>(null);
  const [generating, setGenerating] = useState(false);
  const [reportGenerated, setReportGenerated] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const canNext = step === 1 ? prevPath && currPath :
                  step === 2 ? true :
                  step === 3 ? volumeResult !== null :
                  step === 4 ? reportGenerated :
                  false;

  async function handleCompute() {
    setComputing(true);
    setError(null);
    setVolumeResult(null);
    try {
      const result = await computeVolumes(currPath, prevPath, benchInterval);
      if (result) {
        setVolumeResult(result);
        setStep(4);
      } else {
        setError("Browser mode — computation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setComputing(false);
    }
  }

  async function handleGenerateReport() {
    if (!volumeResult) return;
    setGenerating(true);
    setError(null);
    try {
      const tonnageFactor = density;
      const benches: ReportTable = {
        title: "Bench-by-Bench Volume Breakdown",
        headers: ["Bench (m)", "Fill (m³)", "Cut (m³)", "Net (m³)", "Fill (t)", "Cut (t)"],
        rows: volumeResult.benches.map((b) => [
          `${b.z_min.toFixed(1)}–${b.z_max.toFixed(1)}`,
          b.fill_volume > 0 ? b.fill_volume.toFixed(1) : "—",
          b.cut_volume > 0 ? b.cut_volume.toFixed(1) : "—",
          b.net_volume !== 0 ? b.net_volume.toFixed(1) : "—",
          (b.fill_volume * tonnageFactor).toFixed(0),
          (b.cut_volume * tonnageFactor).toFixed(0),
        ]),
      };

      const summary: ReportStat[] = [
        { label: "Fill Volume", value: volumeResult.fill_volume.toFixed(0), unit: "m³", color: colors.pass },
        { label: "Cut Volume", value: volumeResult.cut_volume.toFixed(0), unit: "m³", color: colors.fail },
        { label: "Net Volume", value: volumeResult.net_volume.toFixed(0), unit: "m³", color: colors.industrialOrange },
        { label: "Fill Tonnage", value: (volumeResult.fill_volume * tonnageFactor).toFixed(0), unit: "t", color: colors.pass },
        { label: "Cut Tonnage", value: (volumeResult.cut_volume * tonnageFactor).toFixed(0), unit: "t", color: colors.fail },
        { label: "Net Tonnage", value: (volumeResult.net_volume * tonnageFactor).toFixed(0), unit: "t", color: colors.industrialOrange },
        { label: "Fill Cells", value: volumeResult.fill_cells.toLocaleString(), unit: "", color: colors.steelLight },
        { label: "Cut Cells", value: volumeResult.cut_cells.toLocaleString(), unit: "", color: colors.steelLight },
      ];

      const spec: ReportSpec = {
        report_type: "eom_reconciliation",
        title: "End-of-Month Production Reconciliation",
        subtitle: siteName ? `${siteName} — ${new Date().toLocaleDateString()}` : new Date().toLocaleDateString(),
        client: clientName,
        metadata: {
          "Previous Survey": prevPath.split(/[\\/]/).pop() ?? prevPath,
          "Current Survey": currPath.split(/[\\/]/).pop() ?? currPath,
          "Rock Density": `${density} t/m³`,
          "Bench Interval": `${benchInterval} m`,
          "Cell Area": `${volumeResult.cell_area.toFixed(2)} m²`,
        },
        tables: [benches],
        summary,
        provenance_hash: `eom-${Date.now().toString(36)}`,
        output_path: reportPath,
      };

      const result = await generateReport(spec);
      if (result) {
        setReportGenerated(true);
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
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Calculator className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            EoM Production Reconciliation Wizard
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Step indicator */}
        <div className="flex border-b border-navy-border px-5 py-2">
          {STEP_LABELS.map((label, i) => (
            <div key={i} className="flex items-center gap-1.5 px-2">
              <div
                className="flex h-5 w-5 items-center justify-center rounded-full text-[9px] font-bold"
                style={{
                  background: step > i + 1 ? colors.pass : step === i + 1 ? colors.industrialOrange : colors.navyBorder,
                  color: step >= i + 1 ? colors.navyBase : colors.steelGray,
                }}
              >
                {step > i + 1 ? "✓" : i + 1}
              </div>
              <span
                className="text-[10px] font-medium"
                style={{ color: step >= i + 1 ? colors.white : colors.steelGray }}
              >
                {label}
              </span>
              {i < STEP_LABELS.length - 1 && <span className="text-steel-gray">→</span>}
            </div>
          ))}
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Step 1: Select surveys */}
          {step === 1 && (
            <div className="space-y-4">
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  <Database className="mr-1 inline h-3 w-3" />
                  Previous survey DEM (start of month)
                </label>
                {geotiffFiles.length === 0 ? (
                  <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-gray">
                    Drop a GeoTIFF DEM file on the map first.
                  </div>
                ) : (
                  <select
                    value={prevPath}
                    onChange={(e) => setPrevPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                  >
                    <option value="">— Select previous survey —</option>
                    {geotiffFiles.map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                )}
              </div>
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  <Database className="mr-1 inline h-3 w-3" />
                  Current survey DEM (end of month)
                </label>
                <select
                  value={currPath}
                  onChange={(e) => setCurrPath(e.target.value)}
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                >
                  <option value="">— Select current survey —</option>
                  {geotiffFiles.filter((f) => f.path !== prevPath).map((f) => (
                    <option key={f.id} value={f.path}>{f.name}</option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {/* Step 2: Parameters */}
          {step === 2 && (
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Rock density (t/m³)
                  </label>
                  <input
                    type="number" step="0.1" value={density}
                    onChange={(e) => setDensity(parseFloat(e.target.value) || 2.7)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">Iron ore: 2.7, Coal: 1.6, Copper: 2.5</p>
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Bench interval (m)
                  </label>
                  <input
                    type="number" step="0.5" value={benchInterval}
                    onChange={(e) => setBenchInterval(parseFloat(e.target.value) || 5)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">0 = no bench breakdown</p>
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Client / Mine name
                  </label>
                  <input
                    type="text" value={clientName}
                    onChange={(e) => setClientName(e.target.value)}
                    placeholder="e.g., BHP Iron Ore"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Site / Pit name
                  </label>
                  <input
                    type="text" value={siteName}
                    onChange={(e) => setSiteName(e.target.value)}
                    placeholder="e.g., Pit A — June 2026"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                </div>
                <div className="col-span-2">
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Report output path
                  </label>
                  <input
                    type="text" value={reportPath}
                    onChange={(e) => setReportPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                </div>
              </div>
            </div>
          )}

          {/* Step 3: Compute */}
          {step === 3 && (
            <div className="flex flex-col items-center justify-center py-10">
              <p className="mb-4 text-sm text-steel-light">
                Ready to compute volumes between:
              </p>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                <div className="font-mono text-steel-light">Previous: {prevPath.split(/[\\/]/).pop()}</div>
                <div className="font-mono text-steel-light">Current: {currPath.split(/[\\/]/).pop()}</div>
                <div className="mt-2 text-steel-gray">Density: {density} t/m³ · Bench: {benchInterval}m</div>
              </div>
              <button
                onClick={handleCompute}
                disabled={computing}
                className="flex items-center gap-2 rounded-md px-6 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.industrialOrange, color: colors.navyBase }}
              >
                {computing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Calculator className="h-4 w-4" />}
                {computing ? "Computing…" : "Compute Volumes"}
              </button>
            </div>
          )}

          {/* Step 4: Report */}
          {step === 4 && volumeResult && (
            <div className="space-y-4">
              <div className="grid grid-cols-4 gap-2">
                <ResultTile label="Fill (m³)" value={volumeResult.fill_volume.toFixed(0)} color={colors.pass} />
                <ResultTile label="Cut (m³)" value={volumeResult.cut_volume.toFixed(0)} color={colors.fail} />
                <ResultTile label="Net (m³)" value={volumeResult.net_volume.toFixed(0)} color={colors.industrialOrange} />
                <ResultTile label="Fill (t)" value={(volumeResult.fill_volume * density).toFixed(0)} color={colors.pass} />
                <ResultTile label="Cut (t)" value={(volumeResult.cut_volume * density).toFixed(0)} color={colors.fail} />
                <ResultTile label="Net (t)" value={(volumeResult.net_volume * density).toFixed(0)} color={colors.industrialOrange} />
                <ResultTile label="Fill cells" value={volumeResult.fill_cells.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Cut cells" value={volumeResult.cut_cells.toLocaleString()} color={colors.steelLight} />
              </div>

              {volumeResult.benches.length > 0 && (
                <div>
                  <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Bench Breakdown ({volumeResult.benches.length} benches)
                  </h4>
                  <div className="max-h-40 overflow-y-auto rounded-md border border-navy-border">
                    <table className="w-full text-left text-[10px]">
                      <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                        <tr>
                          <th className="px-2 py-1.5 font-mono">Bench</th>
                          <th className="px-2 py-1.5 text-right">Fill (m³)</th>
                          <th className="px-2 py-1.5 text-right">Cut (m³)</th>
                          <th className="px-2 py-1.5 text-right">Net (m³)</th>
                        </tr>
                      </thead>
                      <tbody>
                        {volumeResult.benches.map((b, i) => (
                          <tr key={i} className="border-t border-navy-border">
                            <td className="px-2 py-1.5 font-mono text-steel-light">{b.z_min.toFixed(1)}–{b.z_max.toFixed(1)}</td>
                            <td className="px-2 py-1.5 text-right font-mono" style={{ color: colors.pass }}>{b.fill_volume > 0 ? b.fill_volume.toFixed(1) : "—"}</td>
                            <td className="px-2 py-1.5 text-right font-mono" style={{ color: colors.fail }}>{b.cut_volume > 0 ? b.cut_volume.toFixed(1) : "—"}</td>
                            <td className="px-2 py-1.5 text-right font-mono text-white">{b.net_volume !== 0 ? b.net_volume.toFixed(1) : "—"}</td>
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
                style={{ background: colors.industrialOrange, color: colors.navyBase }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
                {generating ? "Generating report…" : "Generate Reconciliation Report"}
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Reconciliation Complete</h3>
              <p className="mt-1 text-sm text-steel-light">
                Report written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready output.
              </p>
              <div className="mt-4 grid grid-cols-3 gap-2 text-center">
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Fill</div>
                  <div className="font-mono text-sm font-bold" style={{ color: colors.pass }}>
                    {(volumeResult?.fill_volume ?? 0).toFixed(0)} m³
                  </div>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Cut</div>
                  <div className="font-mono text-sm font-bold" style={{ color: colors.fail }}>
                    {(volumeResult?.cut_volume ?? 0).toFixed(0)} m³
                  </div>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Net</div>
                  <div className="font-mono text-sm font-bold" style={{ color: colors.industrialOrange }}>
                    {(volumeResult?.net_volume ?? 0).toFixed(0)} m³
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <button
            onClick={() => setStep((s) => Math.max(1, s - 1) as Step)}
            disabled={step === 1 || step === 3 || step === 5}
            className="flex items-center gap-1 text-xs text-steel-light hover:text-white disabled:opacity-30"
          >
            <ArrowLeft className="h-3 w-3" /> Back
          </button>
          {step < 3 && (
            <button
              onClick={() => setStep((s) => (s + 1) as Step)}
              disabled={!canNext}
              className="flex items-center gap-1 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
              style={{ background: canNext ? colors.industrialOrange : colors.steelGray, color: colors.navyBase }}
            >
              Next <ArrowRight className="h-3 w-3" />
            </button>
          )}
          {step === 5 && (
            <button
              onClick={onClose}
              className="flex items-center gap-1 rounded-md px-4 py-1.5 text-xs font-medium"
              style={{ background: colors.pass, color: colors.navyBase }}
            >
              <Download className="h-3 w-3" /> Finish
            </button>
          )}
        </div>
      </div>
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
