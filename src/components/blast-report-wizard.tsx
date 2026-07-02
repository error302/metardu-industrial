/**
 * Blast Fragmentation Report Wizard — Sprint 4 Revenue Feature #5.
 *
 * Step-by-step wizard for mine surveyors to produce a post-blast
 * performance report combining:
 *   - Fragment size distribution (p20/p50/p80/p90 + uniformity + quality)
 *   - Muck pile volume (from drone photogrammetry DEM)
 *   - Designed-vs-actual fragmentation comparison
 *
 * Workflow:
 *   1. Input fragment sizes (paste CSV, or upload from image-analysis pipeline)
 *   2. Select muck pile DEM GeoTIFF + configure baseline + density
 *   3. Compute fragmentation + muck pile volume
 *   4. Review p20/p50/p80/p90 distribution + quality rating
 *   5. Generate branded PDF Blast Performance Report
 *
 * Revenue: $2,000-3,000/seat/year — mine with 200 blasts/year = 200 reports.
 */

import { useState } from "react";
import {
  X, ArrowRight, ArrowLeft, FileText, Loader2, CheckCircle2,
  Database, Bomb, Download, Upload,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  analyzeFragmentation,
  computeVolumes,
  generateReport,
  type FragmentationResult,
  type FragmentationQuality,
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

const STEP_LABELS = ["Fragments", "Muck Pile", "Compute", "Report", "Done"];

const QUALITY_COLORS: Record<FragmentationQuality, string> = {
  excellent: "#10B981",
  acceptable: colors.industrialOrange,
  coarse: "#F59E0B",
  very_coarse: colors.fail,
};

const QUALITY_LABELS: Record<FragmentationQuality, string> = {
  excellent: "Excellent — well fragmented",
  acceptable: "Acceptable — within target range",
  coarse: "Coarse — oversize risk",
  very_coarse: "Very Coarse — secondary breakage needed",
};

export function BlastReportWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [step, setStep] = useState<Step>(1);
  // Fragment sizes (mm) — paste one-per-line or comma-separated
  const [fragmentsText, setFragmentsText] = useState(
    // Seed with realistic distribution (log-normal, median ~250mm)
    Array.from({ length: 200 }, () => {
      const u1 = Math.random();
      const u2 = Math.random();
      const z = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
      return Math.max(10, Math.round(250 * Math.exp(0.6 * z)));
    }).join("\n")
  );
  const [muckPath, setMuckPath] = useState("");
  const [useMuckVolume, setUseMuckVolume] = useState(true);
  const [baselineDepth, setBaselineDepth] = useState(0);
  const [density, setDensity] = useState(2.7);
  const [blastId, setBlastId] = useState("");
  const [location, setLocation] = useState("");
  const [clientName, setClientName] = useState("");
  const [designedP80, setDesignedP80] = useState(300);
  const [reportPath, setReportPath] = useState("/tmp/blast_report.html");

  const [computing, setComputing] = useState(false);
  const [fragResult, setFragResult] = useState<FragmentationResult | null>(null);
  const [volResult, setVolResult] = useState<VolumeResultRpc | null>(null);
  const [generating, setGenerating] = useState(false);
  const [reportGenerated, setReportGenerated] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const canNext =
    step === 1 ? parseFragments(fragmentsText).length >= 10 :
    step === 2 ? (!useMuckVolume || !!muckPath) :
    step === 3 ? fragResult !== null :
    step === 4 ? reportGenerated :
    false;

  function parseFragments(text: string): number[] {
    return text
      .split(/[\s,;]+/)
      .map((s) => parseFloat(s.trim()))
      .filter((n) => !isNaN(n) && n > 0);
  }

  async function handleCompute() {
    setComputing(true);
    setError(null);
    setFragResult(null);
    setVolResult(null);
    try {
      const sizes = parseFragments(fragmentsText);
      const frag = await analyzeFragmentation(sizes);
      if (!frag) {
        setError("Browser mode — fragmentation analysis requires the native Tauri shell");
        return;
      }
      setFragResult(frag);

      if (useMuckVolume && muckPath) {
        const vol = await computeVolumes(muckPath, `flat:${baselineDepth}`, 0);
        if (vol) setVolResult(vol);
      }
      setStep(4);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setComputing(false);
    }
  }

  async function handleGenerateReport() {
    if (!fragResult) return;
    setGenerating(true);
    setError(null);
    try {
      const muckVolume = volResult?.fill_volume ?? 0;
      const muckTonnage = muckVolume * density;
      const actualP80 = fragResult.p80;
      const designP80 = designedP80;
      const p80Ratio = designP80 > 0 ? actualP80 / designP80 : 1;

      const summary: ReportStat[] = [
        { label: "P20 (fine)", value: fragResult.p20.toFixed(0), unit: "mm", color: colors.pass },
        { label: "P50 (median)", value: fragResult.p50.toFixed(0), unit: "mm", color: colors.steelLight },
        { label: "P80 (coarse)", value: fragResult.p80.toFixed(0), unit: "mm", color: QUALITY_COLORS[fragResult.quality] },
        { label: "P90", value: fragResult.p90.toFixed(0), unit: "mm", color: colors.fail },
        { label: "Uniformity", value: fragResult.uniformity.toFixed(2), unit: "", color: colors.steelLight },
        { label: "Mean size", value: fragResult.mean_size.toFixed(0), unit: "mm", color: colors.steelLight },
        ...(volResult ? [
          { label: "Muck volume", value: muckVolume.toFixed(1), unit: "m³", color: colors.industrialOrange },
          { label: "Muck tonnage", value: muckTonnage.toFixed(0), unit: "t", color: colors.industrialOrange },
        ] : []),
      ];

      const fragTable: ReportTable = {
        title: "Fragment Size Distribution",
        headers: ["Percentile", "Size (mm)", "Interpretation"],
        rows: [
          ["P20 (fine fraction)", fragResult.p20.toFixed(0), fragResult.p20 < 100 ? "Fines — likely crusher-feed ready" : "Coarse fines"],
          ["P50 (median)", fragResult.p50.toFixed(0), fragResult.p50 < 300 ? "Within typical diggability range" : "Above typical diggability"],
          ["P80 (coarse fraction)", fragResult.p80.toFixed(0), fragResult.p80 > 500 ? "Oversize — secondary breakage likely" : "Acceptable oversize"],
          ["P90 (top decile)", fragResult.p90.toFixed(0), fragResult.p90 > 800 ? "Critical oversize — crusher feed risk" : "Acceptable"],
          ["Mean size", fragResult.mean_size.toFixed(0), "—"],
          ["Uniformity (P60/P10)", fragResult.uniformity.toFixed(2), fragResult.uniformity < 3 ? "Well-graded" : "Poorly-graded (gap-graded)"],
        ],
      };

      const performanceTable: ReportTable = {
        title: "Blast Performance vs. Design",
        headers: ["Metric", "Designed", "Actual", "Variance"],
        rows: [
          [
            "P80 (mm)",
            designP80.toFixed(0),
            actualP80.toFixed(0),
            `${((actualP80 - designP80) / designP80 * 100).toFixed(1)}%`,
          ],
          [
            "Quality rating",
            "—",
            QUALITY_LABELS[fragResult.quality].split(" — ")[0],
            p80Ratio <= 1.0 ? "On target" : p80Ratio <= 1.3 ? "Slight oversize" : "Significant oversize",
          ],
          ...(volResult ? [[
            "Muck pile volume (m³)",
            "—",
            muckVolume.toFixed(1),
            `${muckTonnage.toFixed(0)} t @ ${density} t/m³`,
          ]] as string[][] : []),
        ],
      };

      const spec: ReportSpec = {
        report_type: "blast_report",
        title: "Blast Fragmentation Performance Report",
        subtitle: blastId
          ? `${blastId} — ${location || "site"} — ${new Date().toLocaleDateString()}`
          : new Date().toLocaleDateString(),
        client: clientName,
        metadata: {
          "Blast ID": blastId || "(unspecified)",
          "Location": location || "(unspecified)",
          "Fragment Count": parseFragments(fragmentsText).length.toLocaleString(),
          ...(volResult ? { "Muck Pile DEM": muckPath.split(/[\\/]/).pop() ?? muckPath } : {}),
          "Material Density": `${density} t/m³`,
          "Designed P80": `${designedP80} mm`,
          "Audit Date": new Date().toISOString().slice(0, 10),
        },
        tables: [fragTable, performanceTable],
        summary,
        provenance_hash: `blast-${Date.now().toString(36)}`,
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
            <Bomb className="h-4 w-4" style={{ color: "#FF6B35" }} />
            Blast Fragmentation Report Wizard
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
                  background: step > i + 1 ? colors.pass : step === i + 1 ? "#FF6B35" : colors.navyBorder,
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

          {/* Step 1: Fragment input */}
          {step === 1 && (
            <div className="space-y-3">
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  <Upload className="mr-1 inline h-3 w-3" />
                  Fragment sizes (mm) — one per line or comma-separated
                </label>
                <textarea
                  value={fragmentsText}
                  onChange={(e) => setFragmentsText(e.target.value)}
                  rows={10}
                  placeholder="Paste fragment sizes in mm here…"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
                />
                <p className="mt-1 text-[10px] text-steel-gray">
                  {parseFragments(fragmentsText).length} fragments parsed (minimum 10 required).
                  Source options: WipFrag / SplitDesktop image-analysis export, manual sampling, or
                  AI vision pipeline (planned Sprint 6+).
                </p>
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[11px] text-steel-gray">
                <div className="mb-1 font-semibold text-steel-light">Tip — realistic distributions</div>
                Well-fragmented blast: median 150-250mm, P80 &lt; 300mm.
                Typical blast: median 250-400mm, P80 300-500mm.
                Poor blast: median &gt; 400mm, P80 &gt; 500mm (secondary breakage needed).
              </div>
            </div>
          )}

          {/* Step 2: Muck pile DEM */}
          {step === 2 && (
            <div className="space-y-4">
              <div>
                <label className="mb-2 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Muck pile volume source
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={() => setUseMuckVolume(true)}
                    className="rounded-md border p-3 text-left text-xs transition-colors"
                    style={{
                      borderColor: useMuckVolume ? "#FF6B35" : colors.navyBorder,
                      background: useMuckVolume ? "#FF6B3510" : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">Compute from DEM</div>
                    <div className="mt-1 text-[10px] text-steel-gray">
                      Drone photogrammetry of muck pile (full report)
                    </div>
                  </button>
                  <button
                    onClick={() => setUseMuckVolume(false)}
                    className="rounded-md border p-3 text-left text-xs transition-colors"
                    style={{
                      borderColor: !useMuckVolume ? "#FF6B35" : colors.navyBorder,
                      background: !useMuckVolume ? "#FF6B3510" : colors.navyBase,
                    }}
                  >
                    <div className="font-semibold text-white">Skip (fragmentation only)</div>
                    <div className="mt-1 text-[10px] text-steel-gray">
                      Faster — omit volume/tonnage sections
                    </div>
                  </button>
                </div>
              </div>

              {useMuckVolume && (
                <>
                  <div>
                    <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                      <Database className="mr-1 inline h-3 w-3" />
                      Muck pile DEM (post-blast drone survey)
                    </label>
                    {geotiffFiles.length === 0 ? (
                      <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-gray">
                        Drop a GeoTIFF DEM of the muck pile on the map first.
                      </div>
                    ) : (
                      <select
                        value={muckPath}
                        onChange={(e) => setMuckPath(e.target.value)}
                        className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                        style={{ borderColor: muckPath ? "#FF6B35" : undefined }}
                      >
                        <option value="">— Select muck pile DEM —</option>
                        {geotiffFiles.map((f) => (
                          <option key={f.id} value={f.path}>{f.name}</option>
                        ))}
                      </select>
                    )}
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                        Pre-blast grade (m) — flat baseline
                      </label>
                      <input
                        type="number" step="0.1" value={baselineDepth}
                        onChange={(e) => setBaselineDepth(parseFloat(e.target.value) || 0)}
                        className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                      />
                      <p className="mt-1 text-[10px] text-steel-gray">
                        The pit floor elevation before blasting. Volume = (muck − floor) × area.
                      </p>
                    </div>
                    <div>
                      <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                        Material density (t/m³)
                      </label>
                      <input
                        type="number" step="0.05" value={density}
                        onChange={(e) => setDensity(parseFloat(e.target.value) || 2.7)}
                        className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                      />
                    </div>
                  </div>
                </>
              )}

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Blast ID
                  </label>
                  <input
                    type="text" value={blastId}
                    onChange={(e) => setBlastId(e.target.value)}
                    placeholder="e.g., BL-2026-0142"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Location / Bench
                  </label>
                  <input
                    type="text" value={location}
                    onChange={(e) => setLocation(e.target.value)}
                    placeholder="e.g., Pit A — Bench 1050"
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
                    placeholder="e.g., Newcrest — Cadia Valley"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Designed P80 (mm)
                  </label>
                  <input
                    type="number" step="10" value={designedP80}
                    onChange={(e) => setDesignedP80(parseFloat(e.target.value) || 300)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none"
                  />
                  <p className="mt-1 text-[10px] text-steel-gray">Target P80 from blast design</p>
                </div>
                <div className="col-span-2">
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
                Ready to compute fragmentation {useMuckVolume ? "+ muck pile volume" : ""}:
              </p>
              <div className="mb-4 rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                <div className="font-mono text-steel-light">Fragments: {parseFragments(fragmentsText).length} samples</div>
                {useMuckVolume && (
                  <>
                    <div className="font-mono text-steel-light">Muck DEM:  {muckPath.split(/[\\/]/).pop()}</div>
                    <div className="font-mono text-steel-light">Baseline:  flat {baselineDepth} m</div>
                  </>
                )}
                <div className="mt-2 text-steel-gray">Density: {density} t/m³ · Designed P80: {designedP80}mm</div>
              </div>
              <button
                onClick={handleCompute}
                disabled={computing}
                className="flex items-center gap-2 rounded-md px-6 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: "#FF6B35", color: colors.navyBase }}
              >
                {computing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Bomb className="h-4 w-4" />}
                {computing ? "Computing…" : "Compute Fragmentation + Volume"}
              </button>
            </div>
          )}

          {/* Step 4: Report */}
          {step === 4 && fragResult && (
            <div className="space-y-4">
              <div className="grid grid-cols-4 gap-2">
                <ResultTile label="P20 (mm)" value={fragResult.p20.toFixed(0)} color={colors.pass} />
                <ResultTile label="P50 (mm)" value={fragResult.p50.toFixed(0)} color={colors.steelLight} />
                <ResultTile label="P80 (mm)" value={fragResult.p80.toFixed(0)} color={QUALITY_COLORS[fragResult.quality]} />
                <ResultTile label="P90 (mm)" value={fragResult.p90.toFixed(0)} color={colors.fail} />
                <ResultTile label="Mean (mm)" value={fragResult.mean_size.toFixed(0)} color={colors.steelLight} />
                <ResultTile label="Uniformity" value={fragResult.uniformity.toFixed(2)} color={colors.steelLight} />
                {volResult && (
                  <>
                    <ResultTile label="Muck Vol (m³)" value={volResult.fill_volume.toFixed(1)} color={colors.industrialOrange} />
                    <ResultTile label="Muck (t)" value={(volResult.fill_volume * density).toFixed(0)} color={colors.industrialOrange} />
                  </>
                )}
                {!volResult && (
                  <>
                    <ResultTile label="—" value="—" color={colors.steelGray} />
                    <ResultTile label="—" value="—" color={colors.steelGray} />
                  </>
                )}
              </div>

              {/* Quality banner */}
              <div
                className="rounded-md border p-3 text-xs"
                style={{
                  borderColor: `${QUALITY_COLORS[fragResult.quality]}60`,
                  background: `${QUALITY_COLORS[fragResult.quality]}10`,
                  color: QUALITY_COLORS[fragResult.quality],
                }}
              >
                <div className="font-semibold">{QUALITY_LABELS[fragResult.quality]}</div>
                <div className="mt-1 text-[10px]">
                  P80 of {fragResult.p80.toFixed(0)}mm vs. designed {designedP80}mm.
                  {fragResult.p80 > designedP80
                    ? ` Oversize by ${((fragResult.p80 - designedP80) / designedP80 * 100).toFixed(1)}% — review powder factor and burden.`
                    : ` On target — ${((designedP80 - fragResult.p80) / designedP80 * 100).toFixed(1)}% finer than design.`}
                </div>
              </div>

              <button
                onClick={handleGenerateReport}
                disabled={generating}
                className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: "#FF6B35", color: colors.navyBase }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
                {generating ? "Generating report…" : "Generate Blast Performance Report"}
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Blast Report Complete</h3>
              <p className="mt-1 text-sm text-steel-light">
                Report written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready output.
              </p>
              {fragResult && (
                <div className="mt-4 grid grid-cols-4 gap-2 text-center">
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">P20</div>
                    <div className="font-mono text-sm font-bold" style={{ color: colors.pass }}>
                      {fragResult.p20.toFixed(0)} mm
                    </div>
                  </div>
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">P50</div>
                    <div className="font-mono text-sm font-bold text-white">
                      {fragResult.p50.toFixed(0)} mm
                    </div>
                  </div>
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">P80</div>
                    <div className="font-mono text-sm font-bold" style={{ color: QUALITY_COLORS[fragResult.quality] }}>
                      {fragResult.p80.toFixed(0)} mm
                    </div>
                  </div>
                  <div className="rounded-md border border-navy-border bg-navy-base p-3">
                    <div className="text-[9px] uppercase text-steel-gray">P90</div>
                    <div className="font-mono text-sm font-bold" style={{ color: colors.fail }}>
                      {fragResult.p90.toFixed(0)} mm
                    </div>
                  </div>
                </div>
              )}
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
              style={{ background: canNext ? "#FF6B35" : colors.steelGray, color: colors.navyBase }}
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
