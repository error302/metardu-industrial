import { withReportProfile } from "@/lib/report-profile";
import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * Dredge Audit Wizard — Sprint 4 Revenue Feature #2.
 *
 * Step-by-step wizard for marine surveyors and port engineers to produce
 * a contract-ready Dredge Pay-Volume Audit Report.
 *
 * The four-bucket model is the international standard for dredge contracts:
 *   - PAY VOLUME — material removed from pre-dredge seabed down to design
 *     grade (always paid)
 *   - ALLOWABLE OVERDREDGE — material removed within tolerance band below
 *     design (paid — typically 0.3-0.5m)
 *   - EXCESSIVE OVERDREDGE — material removed below design + tolerance
 *     (NOT paid — often triggers back-charge)
 *   - SHOALING / UNDER-DREDGE — material left above design (re-dredge required)
 *
 * Workflow:
 *   1. Select pre-dredge survey GeoTIFF
 *   2. Select post-dredge survey GeoTIFF
 *   3. Configure design depth (flat:Z or GeoTIFF template) + tolerance
 *   4. Compute 4-bucket volumes
 *   5. Generate branded PDF Dredge Audit Report
 *
 * Revenue: $5,000-10,000/project — every dredging contract needs this.
 */

import { useState } from "react";
import {
  X, ArrowRight, ArrowLeft, FileText, Loader2, CheckCircle2,
  Database, Waves, Download, AlertTriangle,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  computeDredgeAudit,
  generateReport,
  type DredgeVolumeResult,
  type DredgeCategory,
  type ReportSpec,
  type ReportTable,
  type ReportStat,
} from "@/lib/tauri-ipc";
import { pickFile } from "@/lib/file-picker";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Step = 1 | 2 | 3 | 4 | 5;

const STEP_LABELS = ["Surveys", "Design", "Compute", "Report", "Done"];

const CATEGORY_COLORS: Record<DredgeCategory, string> = {
  pay: colors.pass,
  allowable_overdredge: colors.industrialOrange,
  excessive_overdredge: colors.fail,
  shoaling: "#F59E0B", // amber for shoaling
  no_change: colors.steelGray,
};

const CATEGORY_LABELS: Record<DredgeCategory, string> = {
  pay: "Pay Volume",
  allowable_overdredge: "Allowable Overdredge",
  excessive_overdredge: "Excessive Overdredge",
  shoaling: "Shoaling / Under-Dredge",
  no_change: "No Change",
};

export function DredgeAuditWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [step, setStep] = useState<Step>(1);
  const [prePath, setPrePath] = useState("");
  const [postPath, setPostPath] = useState("");
  const [designMode, setDesignMode] = useState<"flat" | "tiff">("flat");
  const [designDepth, setDesignDepth] = useState(15);
  const [designTiffPath, setDesignTiffPath] = useState("");
  const [tolerance, setTolerance] = useState(0.3);
  const [clientName, setClientName] = useState("");
  const [projectName, setProjectName] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/dredge_audit.html");

  const [computing, setComputing] = useState(false);
  const [result, setResult] = useState<DredgeVolumeResult | null>(null);
  const [generating, setGenerating] = useState(false);
  const [reportGenerated, setReportGenerated] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  const canNext =
    step === 1 ? prePath && postPath :
    step === 2 ? (designMode === "flat" ? designDepth > 0 : !!designTiffPath) :
    step === 3 ? result !== null :
    step === 4 ? reportGenerated :
    false;

  function designPath(): string {
    return designMode === "flat" ? `flat:${designDepth}` : designTiffPath;
  }

  async function handleCompute() {
    setComputing(true);
    setError(null);
    setResult(null);
    try {
      const r = await computeDredgeAudit({
        postPath,
        prePath,
        designPath: designPath(),
        toleranceM: tolerance,
      });
      if (r) {
        setResult(r);
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
    if (!result) return;
    setGenerating(true);
    setError(null);
    try {
      const summary: ReportStat[] = [
        { label: "Pay Volume", value: result.pay_volume.toFixed(1), unit: "m³", color: colors.pass },
        { label: "Allowable OD", value: result.allowable_overdredge.toFixed(1), unit: "m³", color: colors.industrialOrange },
        { label: "Excessive OD", value: result.excessive_overdredge.toFixed(1), unit: "m³", color: colors.fail },
        { label: "Shoaling", value: result.shoaling.toFixed(1), unit: "m³", color: "#F59E0B" },
        { label: "Total Paid", value: result.total_paid.toFixed(1), unit: "m³", color: colors.pass },
        { label: "Avg Dredge Depth", value: result.avg_dredge_depth.toFixed(2), unit: "m", color: colors.steelLight },
        { label: "Max Excessive OD", value: result.max_excessive.toFixed(2), unit: "m", color: colors.fail },
        { label: "Cell Area", value: result.cell_area.toFixed(2), unit: "m²", color: colors.steelLight },
      ];

      const breakdown: ReportTable = {
        title: "Volume Breakdown by Category",
        headers: ["Category", "Volume (m³)", "Cells", "% of Total", "Status"],
        rows: [
          ["Pay Volume", result.pay_volume.toFixed(1), result.pay_cells.toLocaleString(),
            pctOfTotal(result.pay_volume, result), "Paid"],
          ["Allowable Overdredge", result.allowable_overdredge.toFixed(1), result.allowable_cells.toLocaleString(),
            pctOfTotal(result.allowable_overdredge, result), "Paid"],
          ["Excessive Overdredge", result.excessive_overdredge.toFixed(1), result.excessive_cells.toLocaleString(),
            pctOfTotal(result.excessive_overdredge, result), "NOT Paid"],
          ["Shoaling / Under-Dredge", result.shoaling.toFixed(1), result.shoaling_cells.toLocaleString(),
            pctOfTotal(result.shoaling, result), "Re-dredge required"],
        ],
      };

      const profileFields = await withReportProfile();
      const spec: ReportSpec = {
        ...profileFields,
        report_type: "dredge_audit",
        title: "Dredge Pay-Volume Audit Report",
        subtitle: projectName ? `${projectName} — ${new Date().toLocaleDateString()}` : new Date().toLocaleDateString(),
        client: clientName,
        metadata: {
          "Pre-Dredge Survey": prePath.split(/[\\/]/).pop() ?? prePath,
          "Post-Dredge Survey": postPath.split(/[\\/]/).pop() ?? postPath,
          "Design Template": designMode === "flat" ? `Flat ${designDepth} m` : designTiffPath.split(/[\\/]/).pop() ?? designTiffPath,
          "Tolerance": `${tolerance} m`,
          "Cell Area": `${result.cell_area.toFixed(2)} m²`,
        },
        tables: [breakdown],
        summary,
        provenance_hash: `dredge-${Date.now().toString(36)}`,
        output_path: reportPath,
      };

      const r = await generateReport(spec);
      if (r) {
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
            <Waves className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            Dredge Pay-Volume Audit Wizard
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
                  background: step > i + 1 ? colors.pass : step === i + 1 ? colors.marineTurquoise : colors.navyBorder,
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
                  Pre-dredge survey (baseline seabed BEFORE dredging)
                </label>
                <div className="flex items-center gap-2">
                  <button
                    onClick={async () => { const p = await pickFile({ extensions: ["tif", "tiff"], filterName: "GeoTIFF DEM", title: "Select pre-dredge survey" }); if (p) setPrePath(p); }}
                    className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
                  ><Database className="h-3.5 w-3.5" /> Browse</button>
                  {geotiffFiles.length > 0 && (
                    <select
                      value={prePath}
                      onChange={(e) => setPrePath(e.target.value)}
                      className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                      style={{ borderColor: prePath ? colors.marineTurquoise : undefined }}
                    >
                      <option value="">— Or pick loaded —</option>
                      {geotiffFiles.map((f) => (<option key={f.id} value={f.path}>{f.name}</option>))}
                    </select>
                  )}
                </div>
                <p className="mt-1 text-[10px] text-steel-gray">
                  Hydrographic convention: depths positive downward (e.g., 12.5m = 12.5m below chart datum)
                </p>
              </div>
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  <Database className="mr-1 inline h-3 w-3" />
                  Post-dredge survey (seabed AFTER dredging)
                </label>
                <div className="flex items-center gap-2">
                  <button
                    onClick={async () => { const p = await pickFile({ extensions: ["tif", "tiff"], filterName: "GeoTIFF DEM", title: "Select post-dredge survey" }); if (p) setPostPath(p); }}
                    className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
                  ><Database className="h-3.5 w-3.5" /> Browse</button>
                  {geotiffFiles.length > 0 && (
                    <select
                      value={postPath}
                      onChange={(e) => setPostPath(e.target.value)}
                      className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                      style={{ borderColor: postPath ? colors.marineTurquoise : undefined }}
                    >
                      <option value="">— Or pick loaded —</option>
                      {geotiffFiles.filter((f) => f.path !== prePath).map((f) => (<option key={f.id} value={f.path}>{f.name}</option>))}
                    </select>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Step 2: Design + Tolerance */}
          {step === 2 && (
            <div className="space-y-4">
              <div>
                <label className="mb-2 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Design template source
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={() => setDesignMode("flat")}
                    className="rounded-md border p-3 text-left text-xs transition-colors"
                    style={{
                      borderColor: designMode === "flat" ? colors.marineTurquoise : colors.navyBorder,
                      background: designMode === "flat" ? `${colors.marineTurquoise}10` : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">Flat depth</div>
                    <div className="mt-1 text-[10px] text-steel-gray">
                      Constant design depth (e.g., 15.0m for a berth)
                    </div>
                  </button>
                  <button
                    onClick={() => setDesignMode("tiff")}
                    className="rounded-md border p-3 text-left text-xs transition-colors"
                    style={{
                      borderColor: designMode === "tiff" ? colors.marineTurquoise : colors.navyBorder,
                      background: designMode === "tiff" ? `${colors.marineTurquoise}10` : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">GeoTIFF template</div>
                    <div className="mt-1 text-[10px] text-steel-gray">
                      Variable-depth design surface (DXF/DGN converted to GeoTIFF)
                    </div>
                  </button>
                </div>
              </div>

              {designMode === "flat" ? (
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Design depth (m, positive downward)
                  </label>
                  <input
                    type="number" step="0.1" value={designDepth}
                    onChange={(e) => setDesignDepth(parseFloat(e.target.value) || 0)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">
                    e.g., 15.0 for a berth, 12.0 for an approach channel
                  </p>
                </div>
              ) : (
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Design template GeoTIFF
                  </label>
                  <select
                    value={designTiffPath}
                    onChange={(e) => setDesignTiffPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  >
                    <option value="">— Select design template —</option>
                    {geotiffFiles.filter((f) => f.path !== prePath && f.path !== postPath).map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                </div>
              )}

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Allowable overdredge tolerance (m)
                  </label>
                  <input
                    type="number" step="0.05" min="0" value={tolerance}
                    onChange={(e) => setTolerance(parseFloat(e.target.value) || 0)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">
                    Standard contracts: 0.3m (precision), 0.5m (standard)
                  </p>
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Client / Contractor
                  </label>
                  <input
                    type="text" value={clientName}
                    onChange={(e) => setClientName(e.target.value)}
                    placeholder="e.g., Port of Rotterdam / Van Oord"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Project / Channel name
                  </label>
                  <input
                    type="text" value={projectName}
                    onChange={(e) => setProjectName(e.target.value)}
                    placeholder="e.g., Entrance Channel — June 2026"
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
                Ready to compute 4-bucket dredge volumes:
              </p>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                <div className="font-mono text-steel-light">Pre-dredge:  {prePath.split(/[\\/]/).pop()}</div>
                <div className="font-mono text-steel-light">Post-dredge: {postPath.split(/[\\/]/).pop()}</div>
                <div className="font-mono text-steel-light">Design:      {designMode === "flat" ? `${designDepth}m (flat)` : designTiffPath.split(/[\\/]/).pop()}</div>
                <div className="mt-2 text-steel-gray">Tolerance: {tolerance}m</div>
              </div>
              <button
                onClick={handleCompute}
                disabled={computing}
                className="flex items-center gap-2 rounded-md px-6 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.marineTurquoise, color: colors.navyBase }}
              >
                {computing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Waves className="h-4 w-4" />}
                {computing ? "Computing…" : "Compute Dredge Volumes"}
              </button>
            </div>
          )}

          {/* Step 4: Report */}
          {step === 4 && result && (
            <div className="space-y-4">
              <div className="grid grid-cols-4 gap-2">
                <ResultTile label="Pay (m³)" value={result.pay_volume.toFixed(1)} color={colors.pass} />
                <ResultTile label="Allowable OD (m³)" value={result.allowable_overdredge.toFixed(1)} color={colors.industrialOrange} />
                <ResultTile label="Excessive OD (m³)" value={result.excessive_overdredge.toFixed(1)} color={colors.fail} />
                <ResultTile label="Shoaling (m³)" value={result.shoaling.toFixed(1)} color="#F59E0B" />
                <ResultTile label="Total Paid (m³)" value={result.total_paid.toFixed(1)} color={colors.pass} />
                <ResultTile label="Avg Dredge (m)" value={result.avg_dredge_depth.toFixed(2)} color={colors.steelLight} />
                <ResultTile label="Max Excessive (m)" value={result.max_excessive.toFixed(2)} color={colors.fail} />
                <ResultTile label="Cell Area (m²)" value={result.cell_area.toFixed(2)} color={colors.steelLight} />
              </div>

              {/* Category breakdown */}
              <div>
                <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Category Breakdown
                </h4>
                <div className="rounded-md border border-navy-border">
                  <table className="w-full text-left text-[10px]">
                    <thead className="bg-navy-panel text-steel-gray">
                      <tr>
                        <th className="px-2 py-1.5">Category</th>
                        <th className="px-2 py-1.5 text-right">Volume (m³)</th>
                        <th className="px-2 py-1.5 text-right">Cells</th>
                        <th className="px-2 py-1.5 text-right">% Paid</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(["pay", "allowable_overdredge", "excessive_overdredge", "shoaling"] as DredgeCategory[]).map((cat) => {
                        const vol = catVol(result, cat);
                        const cells = catCells(result, cat);
                        return (
                          <tr key={cat} className="border-t border-navy-border">
                            <td className="px-2 py-1.5" style={{ color: CATEGORY_COLORS[cat] }}>
                              ● {CATEGORY_LABELS[cat]}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono text-white">{vol.toFixed(1)}</td>
                            <td className="px-2 py-1.5 text-right font-mono text-steel-light">{cells.toLocaleString()}</td>
                            <td className="px-2 py-1.5 text-right font-mono text-steel-light">
                              {result.total_paid > 0 ? ((vol / result.total_paid) * 100).toFixed(1) : "0.0"}%
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>

              {result.excessive_overdredge > 0 && (
                <div
                  className="flex items-start gap-2 rounded-md border p-3 text-xs"
                  style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}
                >
                  <AlertTriangle className="mt-0.5 h-3.5 w-3.5 flex-shrink-0" />
                  <div>
                    <div className="font-semibold">Excessive overdredge detected</div>
                    <div className="mt-0.5 text-[10px]">
                      {result.excessive_cells.toLocaleString()} cells dredged below tolerance
                      (max {(result.max_excessive * 100).toFixed(0)}cm beyond). Unpaid volume: {result.excessive_overdredge.toFixed(1)} m³.
                      Review slope stability if excessive OD exceeds 5% of total paid.
                    </div>
                  </div>
                </div>
              )}

              <button
                onClick={handleGenerateReport}
                disabled={generating}
                className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.marineTurquoise, color: colors.navyBase }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
                {generating ? "Generating report…" : "Generate Dredge Audit Report"}
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Dredge Audit Complete</h3>
              <p className="mt-1 text-sm text-steel-light">
                Report written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready output.
              </p>
              <div className="mt-4 grid grid-cols-4 gap-2 text-center">
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Pay</div>
                  <div className="font-mono text-sm font-bold" style={{ color: colors.pass }}>
                    {(result?.pay_volume ?? 0).toFixed(0)} m³
                  </div>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Allow OD</div>
                  <div className="font-mono text-sm font-bold" style={{ color: colors.industrialOrange }}>
                    {(result?.allowable_overdredge ?? 0).toFixed(0)} m³
                  </div>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Excess OD</div>
                  <div className="font-mono text-sm font-bold" style={{ color: colors.fail }}>
                    {(result?.excessive_overdredge ?? 0).toFixed(0)} m³
                  </div>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <div className="text-[9px] uppercase text-steel-gray">Shoaling</div>
                  <div className="font-mono text-sm font-bold" style={{ color: "#F59E0B" }}>
                    {(result?.shoaling ?? 0).toFixed(0)} m³
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
              style={{ background: canNext ? colors.marineTurquoise : colors.steelGray, color: colors.navyBase }}
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

function pctOfTotal(vol: number, result: DredgeVolumeResult): string {
  const total = result.pay_volume + result.allowable_overdredge + result.excessive_overdredge + result.shoaling;
  if (total === 0) return "0.0%";
  return `${((vol / total) * 100).toFixed(1)}%`;
}

function catVol(result: DredgeVolumeResult, cat: DredgeCategory): number {
  switch (cat) {
    case "pay": return result.pay_volume;
    case "allowable_overdredge": return result.allowable_overdredge;
    case "excessive_overdredge": return result.excessive_overdredge;
    case "shoaling": return result.shoaling;
    default: return 0;
  }
}

function catCells(result: DredgeVolumeResult, cat: DredgeCategory): number {
  switch (cat) {
    case "pay": return result.pay_cells;
    case "allowable_overdredge": return result.allowable_cells;
    case "excessive_overdredge": return result.excessive_cells;
    case "shoaling": return result.shoaling_cells;
    default: return 0;
  }
}

function ResultTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-sm font-bold text-white">{value}</div>
    </div>
  );
}
