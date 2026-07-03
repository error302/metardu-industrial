/**
 * Stockpile Inventory Audit Wizard — Sprint 4 Revenue Feature #4.
 *
 * Step-by-step wizard for mine surveyors to produce a monthly stockpile
 * inventory audit report.
 *
 * Workflow:
 *   1. Select current survey DEM (GeoTIFF of stockpile yard)
 *   2. Configure baseline (flat:0 for first audit, or previous survey DEM)
 *      + density + client/site metadata
 *   3. Compute fill volume + tonnage (single audit — for multi-stockpile
 *      audits, run this wizard once per stockpile polygon)
 *   4. Review results
 *   5. Generate branded PDF Stockpile Audit Report
 *
 * Revenue: $1,500-2,000/seat/year — every mine reports stockpile inventories
 * monthly (5-20 stockpiles × 12 months/year = 60-240 reports per site).
 */

import { useState } from "react";
import {
  X, ArrowRight, ArrowLeft, FileText, Loader2, CheckCircle2,
  Database, Boxes, Download,
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
import { pickFile, pickSaveFile } from "@/lib/file-picker";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Step = 1 | 2 | 3 | 4 | 5;

const STEP_LABELS = ["Survey", "Parameters", "Compute", "Report", "Done"];

export function StockpileAuditWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [step, setStep] = useState<Step>(1);
  const [currPath, setCurrPath] = useState("");
  // Baseline: "flat:0" (first audit) or path to previous survey
  const [baselineMode, setBaselineMode] = useState<"flat" | "previous">("flat");
  const [baselineDepth, setBaselineDepth] = useState(0);
  const [prevPath, setPrevPath] = useState("");
  const [density, setDensity] = useState(2.7);
  const [stockpileName, setStockpileName] = useState("");
  const [clientName, setClientName] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/stockpile_audit.html");

  const [computing, setComputing] = useState(false);
  const [volumeResult, setVolumeResult] = useState<VolumeResultRpc | null>(null);
  const [generating, setGenerating] = useState(false);
  const [reportGenerated, setReportGenerated] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const canNext =
    step === 1 ? !!currPath :
    step === 2 ? (baselineMode === "flat" ? true : !!prevPath) :
    step === 3 ? volumeResult !== null :
    step === 4 ? reportGenerated :
    false;

  function baselinePath(): string {
    return baselineMode === "flat" ? `flat:${baselineDepth}` : prevPath;
  }

  async function handleCompute() {
    setComputing(true);
    setError(null);
    setVolumeResult(null);
    try {
      // Note: computeVolumes(current, reference, benchInterval) — for stockpiles
      // we use current=survey, reference=baseline, bench=0 (no breakdown needed)
      const result = await computeVolumes(currPath, baselinePath(), 0);
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
      // For stockpiles, fill volume = stockpile volume above baseline
      // (cut should typically be 0 — stockpiles are additive)
      const tonnage = volumeResult.fill_volume * density;

      const summary: ReportStat[] = [
        { label: "Stockpile Volume", value: volumeResult.fill_volume.toFixed(1), unit: "m³", color: colors.industrialOrange },
        { label: "Tonnage", value: tonnage.toFixed(0), unit: "t", color: colors.pass },
        { label: "Density", value: density.toFixed(2), unit: "t/m³", color: colors.steelLight },
        { label: "Cell Area", value: volumeResult.cell_area.toFixed(2), unit: "m²", color: colors.steelLight },
        { label: "Fill Cells", value: volumeResult.fill_cells.toLocaleString(), unit: "", color: colors.steelLight },
        { label: "Cut Cells (removal)", value: volumeResult.cut_cells.toLocaleString(), unit: "", color: colors.fail },
        { label: "Net Volume", value: volumeResult.net_volume.toFixed(1), unit: "m³", color: colors.industrialOrange },
        { label: "Net Tonnage", value: (volumeResult.net_volume * density).toFixed(0), unit: "t", color: colors.industrialOrange },
      ];

      const breakdown: ReportTable = {
        title: "Stockpile Volume Summary",
        headers: ["Metric", "Value", "Unit"],
        rows: [
          ["Stockpile volume (fill)", volumeResult.fill_volume.toFixed(1), "m³"],
          ["Excavation from pile (cut)", volumeResult.cut_volume.toFixed(1), "m³"],
          ["Net volume change", volumeResult.net_volume.toFixed(1), "m³"],
          ["Material density", density.toFixed(2), "t/m³"],
          ["Stockpile tonnage", tonnage.toFixed(0), "t"],
          ["Net tonnage change", (volumeResult.net_volume * density).toFixed(0), "t"],
          ["Grid cell area", volumeResult.cell_area.toFixed(2), "m²"],
          ["Cells in fill region", volumeResult.fill_cells.toLocaleString(), ""],
        ],
      };

      const spec: ReportSpec = {
        report_type: "stockpile_audit",
        title: "Stockpile Inventory Audit Report",
        subtitle: stockpileName
          ? `${stockpileName} — ${new Date().toLocaleDateString()}`
          : new Date().toLocaleDateString(),
        client: clientName,
        metadata: {
          "Stockpile": stockpileName || "(unnamed)",
          "Current Survey": currPath.split(/[\\/]/).pop() ?? currPath,
          "Baseline": baselineMode === "flat"
            ? `Flat ${baselineDepth} m (first audit)`
            : prevPath.split(/[\\/]/).pop() ?? prevPath,
          "Material Density": `${density} t/m³`,
          "Cell Area": `${volumeResult.cell_area.toFixed(2)} m²`,
          "Audit Date": new Date().toISOString().slice(0, 10),
        },
        tables: [breakdown],
        summary,
        provenance_hash: `stockpile-${Date.now().toString(36)}`,
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
            <Boxes className="h-4 w-4" style={{ color: "#FFC107" }} />
            Stockpile Inventory Audit Wizard
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
                  background: step > i + 1 ? colors.pass : step === i + 1 ? "#FFC107" : colors.navyBorder,
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

          {/* Step 1: Select survey */}
          {step === 1 && (
            <div className="space-y-4">
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  <Database className="mr-1 inline h-3 w-3" />
                  Stockpile yard survey DEM (current month)
                </label>
                <div className="flex items-center gap-2">
                  <button
                    onClick={async () => { const p = await pickFile({ extensions: ["tif", "tiff"], filterName: "GeoTIFF DEM", title: "Select stockpile survey" }); if (p) setCurrPath(p); }}
                    className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
                  ><Database className="h-3.5 w-3.5" /> Browse</button>
                  {geotiffFiles.length > 0 && (
                    <select
                      value={currPath}
                      onChange={(e) => setCurrPath(e.target.value)}
                      className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                      style={{ borderColor: currPath ? "#FFC107" : undefined }}
                    >
                      <option value="">— Or pick loaded —</option>
                      {geotiffFiles.map((f) => (<option key={f.id} value={f.path}>{f.name}</option>))}
                    </select>
                  )}
                </div>
                <p className="mt-1 text-[10px] text-steel-gray">
                  Drone photogrammetry exports work well — clip to the stockpile polygon for best results.
                </p>
              </div>
            </div>
          )}

          {/* Step 2: Parameters */}
          {step === 2 && (
            <div className="space-y-4">
              <div>
                <label className="mb-2 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Baseline reference
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={() => setBaselineMode("flat")}
                    className="rounded-md border p-3 text-left text-xs transition-colors"
                    style={{
                      borderColor: baselineMode === "flat" ? "#FFC107" : colors.navyBorder,
                      background: baselineMode === "flat" ? "#FFC10710" : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">Flat pad level</div>
                    <div className="mt-1 text-[10px] text-steel-gray">
                      First audit — compare against the graded pad elevation
                    </div>
                  </button>
                  <button
                    onClick={() => setBaselineMode("previous")}
                    className="rounded-md border p-3 text-left text-xs transition-colors"
                    style={{
                      borderColor: baselineMode === "previous" ? "#FFC107" : colors.navyBorder,
                      background: baselineMode === "previous" ? "#FFC10710" : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">Previous survey</div>
                    <div className="mt-1 text-[10px] text-steel-gray">
                      Month-over-month delta (re-audit)
                    </div>
                  </button>
                </div>
              </div>

              {baselineMode === "flat" ? (
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Pad elevation (m) — flat baseline
                  </label>
                  <input
                    type="number" step="0.1" value={baselineDepth}
                    onChange={(e) => setBaselineDepth(parseFloat(e.target.value) || 0)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">
                    The graded pad elevation. Volume = (current − pad) × area for each cell.
                  </p>
                </div>
              ) : (
                <div>
                  <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Previous month survey DEM
                  </label>
                  <select
                    value={prevPath}
                    onChange={(e) => setPrevPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  >
                    <option value="">— Select previous survey —</option>
                    {geotiffFiles.filter((f) => f.path !== currPath).map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                </div>
              )}

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Material density (t/m³)
                  </label>
                  <input
                    type="number" step="0.05" value={density}
                    onChange={(e) => setDensity(parseFloat(e.target.value) || 2.7)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">
                    Iron ore: 2.7-3.0, Coal: 1.3-1.7, Copper: 2.5, Gold ore: 2.8
                  </p>
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Stockpile name / ID
                  </label>
                  <input
                    type="text" value={stockpileName}
                    onChange={(e) => setStockpileName(e.target.value)}
                    placeholder="e.g., ROM Pad — North Stockpile"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Client / Mine name
                  </label>
                  <input
                    type="text" value={clientName}
                    onChange={(e) => setClientName(e.target.value)}
                    placeholder="e.g., BHP Iron Ore — Mt Whaleback"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Report output path
                  </label>
                  <input
                    type="text" value={reportPath}
                    onChange={(e) => setReportPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                </div>
              </div>
            </div>
          )}

          {/* Step 3: Compute */}
          {step === 3 && (
            <div className="flex flex-col items-center justify-center py-10">
              <p className="mb-4 text-sm text-steel-light">
                Ready to compute stockpile volume:
              </p>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                <div className="font-mono text-steel-light">Survey:   {currPath.split(/[\\/]/).pop()}</div>
                <div className="font-mono text-steel-light">Baseline: {baselineMode === "flat" ? `Flat ${baselineDepth} m` : prevPath.split(/[\\/]/).pop()}</div>
                <div className="mt-2 text-steel-gray">Density: {density} t/m³</div>
              </div>
              <button
                onClick={handleCompute}
                disabled={computing}
                className="flex items-center gap-2 rounded-md px-6 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: "#FFC107", color: colors.navyBase }}
              >
                {computing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Boxes className="h-4 w-4" />}
                {computing ? "Computing…" : "Compute Stockpile Volume"}
              </button>
            </div>
          )}

          {/* Step 4: Report */}
          {step === 4 && volumeResult && (
            <div className="space-y-4">
              <div className="grid grid-cols-4 gap-2">
                <ResultTile label="Volume (m³)" value={volumeResult.fill_volume.toFixed(1)} color="#FFC107" />
                <ResultTile label="Tonnage (t)" value={(volumeResult.fill_volume * density).toFixed(0)} color={colors.pass} />
                <ResultTile label="Cut (m³)" value={volumeResult.cut_volume.toFixed(1)} color={colors.fail} />
                <ResultTile label="Net (m³)" value={volumeResult.net_volume.toFixed(1)} color={colors.industrialOrange} />
                <ResultTile label="Fill cells" value={volumeResult.fill_cells.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Cut cells" value={volumeResult.cut_cells.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Cell area (m²)" value={volumeResult.cell_area.toFixed(2)} color={colors.steelLight} />
                <ResultTile label="Net tonnage (t)" value={(volumeResult.net_volume * density).toFixed(0)} color={colors.industrialOrange} />
              </div>

              <button
                onClick={handleGenerateReport}
                disabled={generating}
                className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: "#FFC107", color: colors.navyBase }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
                {generating ? "Generating report…" : "Generate Stockpile Audit Report"}
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Stockpile Audit Complete</h3>
              <p className="mt-1 text-sm text-steel-light">
                Report written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready output.
              </p>
              <div className="mt-4 grid grid-cols-2 gap-3 text-center">
                <div className="rounded-md border border-navy-border bg-navy-base p-4">
                  <div className="text-[9px] uppercase text-steel-gray">Volume</div>
                  <div className="font-mono text-lg font-bold" style={{ color: "#FFC107" }}>
                    {(volumeResult?.fill_volume ?? 0).toFixed(1)} m³
                  </div>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-4">
                  <div className="text-[9px] uppercase text-steel-gray">Tonnage</div>
                  <div className="font-mono text-lg font-bold" style={{ color: colors.pass }}>
                    {((volumeResult?.fill_volume ?? 0) * density).toFixed(0)} t
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
              style={{ background: canNext ? "#FFC107" : colors.steelGray, color: colors.navyBase }}
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
