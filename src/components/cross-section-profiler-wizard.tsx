import { withReportProfile } from "@/lib/report-profile";
/**
 * Cross-Section Profiler Wizard — Sprint 5 Revenue Feature #8.
 *
 * Step-by-step wizard for port engineers to verify dredged channels
 * meet design specifications via cross-sections.
 *
 * Workflow:
 *   1. Draw centerline on map (or import as lon/lat → reproject)
 *   2. Configure cross-section spacing + half-width + design
 *   3. Compute cross-sections (surveyed vs. design)
 *   4. Review compliance stats + section list
 *   5. Generate branded PDF cross-section report
 *
 * Revenue: $2,000-3,000/seat. Complements the dredge volume engine.
 */

import { useState } from "react";
import {
  FileText, Loader2, CheckCircle2,
  Database, Ruler, AlertTriangle,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  computeCrossSections,
  generateReport,
  type CrossSectionReport,
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

export function CrossSectionProfilerWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [step, setStep] = useState<Step>(1);
  const [surveyPath, setSurveyPath] = useState("");
  const [designMode, setDesignMode] = useState<"flat" | "tiff">("flat");
  const [designDepth, setDesignDepth] = useState(15);
  const [designTiffPath, setDesignTiffPath] = useState("");
  const [spacingM, setSpacingM] = useState(50);
  const [halfWidthM, setHalfWidthM] = useState(25);
  const [sampleResM, setSampleResM] = useState(1);
  // Centerline as a list of [x, y] projected coordinates (one per line)
  const [centerlineText, setCenterlineText] = useState("");
  const [projectName, setProjectName] = useState("");
  const [clientName, setClientName] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/cross_section_report.html");

  const [computing, setComputing] = useState(false);
  const [result, setResult] = useState<CrossSectionReport | null>(null);
  const [generating, setGenerating] = useState(false);
    const [error, setError] = useState<string | null>(null);

      
  function parseCenterline(text: string): { x: number; y: number }[] {
    return text
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"))
      .map((line) => {
        const parts = line.split(/[,\s]+/).map((p) => parseFloat(p));
        if (parts.length >= 2 && !isNaN(parts[0]) && !isNaN(parts[1])) {
          return { x: parts[0], y: parts[1] };
        }
        return null;
      })
      .filter((p): p is { x: number; y: number } => p !== null);
  }

  function designPath(): string | undefined {
    return designMode === "flat" ? undefined : designTiffPath || undefined;
  }

  async function handleCompute() {
    setComputing(true);
    setError(null);
    setResult(null);
    try {
      const centerline = parseCenterline(centerlineText);
      if (centerline.length < 2) {
        setError("Centerline needs at least 2 points");
        return;
      }
      const r = await computeCrossSections({
        centerline,
        spacing_m: spacingM,
        half_width_m: halfWidthM,
        sample_resolution_m: sampleResM,
        surveyPath,
        designPath: designPath(),
        designDepth: designMode === "flat" ? designDepth : undefined,
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
        { label: "Centerline Length", value: result.total_length_m.toFixed(1), unit: "m", color: colors.steelLight },
        { label: "Cross-Sections", value: result.n_sections.toLocaleString(), unit: "", color: colors.steelLight },
        { label: "Spacing", value: result.spacing_m.toFixed(1), unit: "m", color: colors.steelLight },
        { label: "Half-Width", value: result.half_width_m.toFixed(1), unit: "m", color: colors.steelLight },
        { label: "Under-Dredge Area", value: result.summary.total_under_dredge_area.toFixed(1), unit: "m²", color: colors.fail },
        { label: "Over-Dredge Area", value: result.summary.total_over_dredge_area.toFixed(1), unit: "m²", color: colors.industrialOrange },
        { label: "Max Under-Dredge", value: result.summary.max_under_dredge_depth.toFixed(2), unit: "m", color: colors.fail },
        { label: "Compliant Sections", value: `${result.summary.compliant_sections}/${result.n_sections}`, unit: "", color: colors.pass },
        { label: "Compliance", value: result.summary.compliance_pct.toFixed(1), unit: "%", color: result.summary.compliance_pct > 95 ? colors.pass : colors.fail },
      ];

      const sectionTable: ReportTable = {
        title: "Cross-Section Compliance Breakdown",
        headers: ["Chainage (m)", "Under-Dredge (m²)", "Over-Dredge (m²)", "Max Under-Dredge (m)", "Status"],
        rows: result.sections.map((s) => [
          s.chainage_m.toFixed(1),
          s.under_dredge_area > 0 ? s.under_dredge_area.toFixed(1) : "—",
          s.over_dredge_area > 0 ? s.over_dredge_area.toFixed(1) : "—",
          s.max_under_dredge > 0 ? s.max_under_dredge.toFixed(2) : "—",
          s.has_under_dredge ? "UNDER-DREDGE" : "Compliant",
        ]),
      };

      const profileFields = await withReportProfile();
      const spec: ReportSpec = {
        ...profileFields,
        report_type: "cross_section",
        title: "Cross-Section Profile Report",
        subtitle: projectName ? `${projectName} — ${new Date().toLocaleDateString()}` : new Date().toLocaleDateString(),
        client: clientName,
        metadata: {
          "Survey": surveyPath.split(/[\\/]/).pop() ?? surveyPath,
          "Design": designMode === "flat" ? `Flat ${designDepth} m` : designTiffPath.split(/[\\/]/).pop() ?? designTiffPath,
          "Centerline Length": `${result.total_length_m.toFixed(1)} m`,
          "Section Spacing": `${result.spacing_m} m`,
          "Half-Width": `${result.half_width_m} m`,
          "Total Sections": `${result.n_sections}`,
        },
        tables: [sectionTable],
        summary,
        provenance_hash: `xsec-${Date.now().toString(36)}`,
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
      title="Cross-Section Profiler"
      icon={<Ruler className="h-4 w-4" />}
      iconColor={colors.marineTurquoise}
      maxWidth="max-w-3xl"
      subtitle="Channel design compliance"
      footerHint="Bilinear DEM sampling"
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

          {/* Step 1: Select survey */}
          {step === 1 && (
            <div className="space-y-4">
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  <Database className="mr-1 inline h-3 w-3" />
                  Surveyed DEM (post-dredge bathymetric surface)
                </label>
                {geotiffFiles.length === 0 ? (
                  <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-gray">
                    Drop a GeoTIFF DEM of the surveyed channel on the map first.
                  </div>
                ) : (
                  <select
                    value={surveyPath}
                    onChange={(e) => setSurveyPath(e.target.value)}
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                    style={{ borderColor: surveyPath ? colors.marine : undefined }}
                  >
                    <option value="">— Select survey DEM —</option>
                    {geotiffFiles.map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                )}
                <p className="mt-1 text-[10px] text-steel-gray">
                  Hydrographic convention: depths positive downward.
                </p>
              </div>
            </div>
          )}

          {/* Step 2: Centerline + spacing */}
          {step === 2 && (
            <div className="space-y-4">
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Centerline vertices (projected coordinates, one per line: "x, y" or "x y")
                </label>
                <textarea
                  value={centerlineText}
                  onChange={(e) => setCenterlineText(e.target.value)}
                  rows={6}
                  placeholder={"# Format: x, y (projected, e.g., UTM easting/northing)\n# Example for EPSG:28355 (MGA Zone 55):\n337000.0, 6253000.0\n337500.0, 6253050.0\n338000.0, 6253100.0"}
                  className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
                />
                <p className="mt-1 text-[10px] text-steel-gray">
                  {parseCenterline(centerlineText).length} vertices parsed (minimum 2 required).
                  Use the profile tool on the map to draw a line, then transfer the coordinates here.
                  Future Sprint 6+ will auto-populate from a map-drawn polygon.
                </p>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                    Cross-section spacing (m)
                  </label>
                  <input
                    type="number" step="5" value={spacingM}
                    onChange={(e) => setSpacingM(parseFloat(e.target.value) || 50)}
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                    Half-width (m)
                  </label>
                  <input
                    type="number" step="5" value={halfWidthM}
                    onChange={(e) => setHalfWidthM(parseFloat(e.target.value) || 25)}
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                    Sample resolution (m)
                  </label>
                  <input
                    type="number" step="0.5" value={sampleResM}
                    onChange={(e) => setSampleResM(parseFloat(e.target.value) || 1)}
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none"
                  />
                </div>
              </div>

              <div>
                <label className="mb-2 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                  Design template
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={() => setDesignMode("flat")}
                    className="rounded-md border p-2.5 text-left text-xs transition-colors"
                    style={{
                      borderColor: designMode === "flat" ? colors.marine : colors.navyBorder,
                      background: designMode === "flat" ? "#0EA5E910" : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">Flat depth</div>
                  </button>
                  <button
                    onClick={() => setDesignMode("tiff")}
                    className="rounded-md border p-2.5 text-left text-xs transition-colors"
                    style={{
                      borderColor: designMode === "tiff" ? colors.marine : colors.navyBorder,
                      background: designMode === "tiff" ? "#0EA5E910" : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">GeoTIFF template</div>
                  </button>
                </div>
                {designMode === "flat" ? (
                  <input
                    type="number" step="0.1" value={designDepth}
                    onChange={(e) => setDesignDepth(parseFloat(e.target.value) || 15)}
                    className="input-enterprise mt-2 w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                ) : (
                  <select
                    value={designTiffPath}
                    onChange={(e) => setDesignTiffPath(e.target.value)}
                    className="input-enterprise mt-2 w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  >
                    <option value="">— Select design GeoTIFF —</option>
                    {geotiffFiles.filter((f) => f.path !== surveyPath).map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                )}
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                    Project / Channel name
                  </label>
                  <input
                    type="text" value={projectName} onChange={(e) => setProjectName(e.target.value)}
                    placeholder="e.g., Entrance Channel"
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                    Client / Port authority
                  </label>
                  <input
                    type="text" value={clientName} onChange={(e) => setClientName(e.target.value)}
                    placeholder="e.g., Port of Rotterdam"
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                    Report path
                  </label>
                  <input
                    type="text" value={reportPath} onChange={(e) => setReportPath(e.target.value)}
                    className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                </div>
              </div>
            </div>
          )}

          {/* Step 3: Compute */}
          {step === 3 && (
            <div className="flex flex-col items-center justify-center py-10">
              <p className="mb-4 text-sm text-steel-light">Ready to compute cross-sections:</p>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                <div className="font-mono text-steel-light">Survey:    {surveyPath.split(/[\\/]/).pop()}</div>
                <div className="font-mono text-steel-light">Vertices:  {parseCenterline(centerlineText).length}</div>
                <div className="font-mono text-steel-light">Design:    {designMode === "flat" ? `${designDepth}m (flat)` : designTiffPath.split(/[\\/]/).pop()}</div>
                <div className="mt-2 text-steel-gray">Spacing: {spacingM}m · Half-width: ±{halfWidthM}m · Sample: {sampleResM}m</div>
              </div>
              <button
                onClick={handleCompute}
                disabled={computing}
                className="flex items-center gap-2 rounded-md px-6 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.marine, color: colors.navyBase }}
              >
                {computing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Ruler className="h-4 w-4" />}
                {computing ? "Computing…" : "Compute Cross-Sections"}
              </button>
            </div>
          )}

          {/* Step 4: Report */}
          {step === 4 && result && (
            <div className="space-y-4">
              <div className="grid grid-cols-3 gap-2">
                <ResultTile label="Length (m)" value={result.total_length_m.toFixed(1)} color={colors.steelLight} />
                <ResultTile label="Sections" value={result.n_sections.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Compliance" value={`${result.summary.compliance_pct.toFixed(1)}%`} color={result.summary.compliance_pct > 95 ? colors.pass : colors.fail} />
                <ResultTile label="Under-Dredge (m²)" value={result.summary.total_under_dredge_area.toFixed(1)} color={colors.fail} />
                <ResultTile label="Over-Dredge (m²)" value={result.summary.total_over_dredge_area.toFixed(1)} color={colors.industrialOrange} />
                <ResultTile label="Max Under-Dredge (m)" value={result.summary.max_under_dredge_depth.toFixed(2)} color={colors.fail} />
              </div>

              {result.summary.sections_with_under_dredge > 0 && (
                <div
                  className="flex items-start gap-2 rounded-md border p-3 text-xs"
                  style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}
                >
                  <AlertTriangle className="mt-0.5 h-4 w-4 flex-shrink-0" />
                  <div>
                    <div className="font-semibold">Under-dredge detected at {result.summary.sections_with_under_dredge} sections</div>
                    <div className="mt-0.5 text-[10px]">
                      Maximum under-dredge: {result.summary.max_under_dredge_depth.toFixed(2)}m.
                      Total under-dredge area: {result.summary.total_under_dredge_area.toFixed(1)}m².
                      Re-dredge may be required to achieve design depth.
                    </div>
                  </div>
                </div>
              )}

              <div>
                <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Sections ({result.sections.length})
                </h4>
                <div className="max-h-32 overflow-y-auto rounded-md border border-navy-border">
                  <table className="table-enterprise w-full text-left text-[10px]">
                    <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                      <tr>
                        <th className="px-2 py-1.5">Chainage (m)</th>
                        <th className="px-2 py-1.5 text-right">Under (m²)</th>
                        <th className="px-2 py-1.5 text-right">Over (m²)</th>
                        <th className="px-2 py-1.5 text-right">Max Under (m)</th>
                        <th className="px-2 py-1.5">Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      {result.sections.slice(0, 20).map((s) => (
                        <tr key={s.index} className="border-t border-navy-border">
                          <td className="px-2 py-1 font-mono text-steel-light">{s.chainage_m.toFixed(1)}</td>
                          <td className="px-2 py-1 text-right font-mono" style={{ color: colors.fail }}>
                            {s.under_dredge_area > 0 ? s.under_dredge_area.toFixed(1) : "—"}
                          </td>
                          <td className="px-2 py-1 text-right font-mono text-steel-light">
                            {s.over_dredge_area > 0 ? s.over_dredge_area.toFixed(1) : "—"}
                          </td>
                          <td className="px-2 py-1 text-right font-mono" style={{ color: colors.fail }}>
                            {s.max_under_dredge > 0 ? s.max_under_dredge.toFixed(2) : "—"}
                          </td>
                          <td className="px-2 py-1" style={{ color: s.has_under_dredge ? colors.fail : colors.pass }}>
                            ● {s.has_under_dredge ? "Under" : "OK"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>

              <button
                onClick={handleGenerateReport}
                disabled={generating}
                className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.marine, color: colors.navyBase }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
                {generating ? "Generating report…" : "Generate Cross-Section Report"}
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Cross-Section Report Complete</h3>
              <p className="mt-1 text-sm text-steel-light">
                Report written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready output.
              </p>
              {result && (
                <div className="mt-4 grid grid-cols-3 gap-3 text-center">
                  <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-4">
                    <div className="text-[9px] uppercase text-steel-gray">Sections</div>
                    <div className="font-mono text-lg font-bold text-white">{result.n_sections}</div>
                  </div>
                  <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-4">
                    <div className="text-[9px] uppercase text-steel-gray">Under-Dredge</div>
                    <div className="font-mono text-lg font-bold" style={{ color: colors.fail }}>
                      {result.summary.total_under_dredge_area.toFixed(0)} m²
                    </div>
                  </div>
                  <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-4">
                    <div className="text-[9px] uppercase text-steel-gray">Compliance</div>
                    <div className="font-mono text-lg font-bold" style={{ color: result.summary.compliance_pct > 95 ? colors.pass : colors.fail }}>
                      {result.summary.compliance_pct.toFixed(1)}%
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
    </DialogShell>
  );
}

function ResultTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="card-enterprise rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-sm font-bold text-white">{value}</div>
    </div>
  );
}
