import { withReportProfile } from "@/lib/report-profile";
/**
 * EoM Reconciliation — single-screen practical tool.
 *
 * Replaces the old 5-step wizard. Everything visible at once:
 * left column = inputs (Browse buttons, density, bench interval, metadata),
 * right column = results (fill/cut volumes, tonnage, bench breakdown,
 * copy + PDF buttons).
 */

import { useState } from "react";
import {
  Calculator, Loader2, FolderOpen, Copy, CheckCircle2, FileText,
  TrendingUp, TrendingDown,
  Download,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  computeVolumes, generateReport,
  type VolumeResultRpc, type ReportSpec, type ReportTable, type ReportStat,
} from "@/lib/tauri-ipc";
import { pickFile, pickSaveFile } from "@/lib/file-picker";
import { useSurveyStore } from "@/stores/survey-store";
import { useAppStore } from "@/stores/app-store";
import { formatDatumNote } from "@/lib/crs-quickpicks";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

interface Props { open: boolean; onClose: () => void; }

export function EomReconciliationWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");
  // Pull the user's default CRS so we can stamp the report with the correct
  // datum + epoch. This is a legal compliance field — many jurisdictions
  // (AU, US, EU) require the datum to be stated explicitly on every plan.
  const defaultEpsg = useAppStore((s) => s.settings.defaultEpsg);

  const [prevPath, setPrevPath] = useState("");
  const [currPath, setCurrPath] = useState("");
  const [density, setDensity] = useState(2.7);
  const [benchInterval, setBenchInterval] = useState(5);
  const [clientName, setClientName] = useState("");
  const [siteName, setSiteName] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/eom_reconciliation.html");

  const [computing, setComputing] = useState(false);
  const [result, setResult] = useState<VolumeResultRpc | null>(null);
  const [generating, setGenerating] = useState(false);
  const [reportGenerated, setReportGenerated] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

      const canCompute = prevPath && currPath;

  async function handleCompute() {
    setComputing(true); setError(null); setResult(null); setReportGenerated(false);
    try {
      const r = await computeVolumes(currPath, prevPath, benchInterval);
      if (r) setResult(r);
      else setError("Browser mode — requires the native Tauri shell");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally { setComputing(false); }
  }

  async function handleGenerateReport() {
    if (!result) return;
    setGenerating(true);
    try {
      const benches: ReportTable = {
        title: "Bench-by-Bench Volume Breakdown",
        headers: ["Bench (m)", "Fill (m³)", "Cut (m³)", "Net (m³)", "Fill (t)", "Cut (t)"],
        rows: result.benches.map((b) => [
          `${b.z_min.toFixed(1)}–${b.z_max.toFixed(1)}`,
          b.fill_volume > 0 ? b.fill_volume.toFixed(1) : "—",
          b.cut_volume > 0 ? b.cut_volume.toFixed(1) : "—",
          b.net_volume !== 0 ? b.net_volume.toFixed(1) : "—",
          (b.fill_volume * density).toFixed(0),
          (b.cut_volume * density).toFixed(0),
        ]),
      };
      const summary: ReportStat[] = [
        { label: "Fill Volume", value: result.fill_volume.toFixed(0), unit: "m³", color: colors.pass },
        { label: "Cut Volume", value: result.cut_volume.toFixed(0), unit: "m³", color: colors.fail },
        { label: "Net Volume", value: result.net_volume.toFixed(0), unit: "m³", color: colors.industrialOrange },
        { label: "Fill Tonnage", value: (result.fill_volume * density).toFixed(0), unit: "t", color: colors.pass },
        { label: "Cut Tonnage", value: (result.cut_volume * density).toFixed(0), unit: "t", color: colors.fail },
        { label: "Net Tonnage", value: (result.net_volume * density).toFixed(0), unit: "t", color: colors.industrialOrange },
      ];
      const profileFields = await withReportProfile();
      const spec: ReportSpec = {
        ...profileFields,
        report_type: "eom_reconciliation",
        title: "End-of-Month Production Reconciliation",
        subtitle: siteName || new Date().toLocaleDateString(),
        client: clientName,
        // Stamp the report with the user's default CRS datum + epoch.
        // For Australian GDA2020 plans this is the difference between
        // a legally-compliant plan and one that gets bounced back.
        datum_note: formatDatumNote(defaultEpsg),
        metadata: {
          "Previous Survey": prevPath.split(/[\\/]/).pop() ?? prevPath,
          "Current Survey": currPath.split(/[\\/]/).pop() ?? currPath,
          "Rock Density": `${density} t/m³`,
          "Bench Interval": `${benchInterval} m`,
          "Coordinate System": defaultEpsg,
        },
        tables: [benches],
        summary,
        provenance_hash: `eom-${Date.now().toString(36)}`,
        output_path: reportPath,
      };
      await generateReport(spec);
      setReportGenerated(true);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally { setGenerating(false); }
  }

  function handleCopy() {
    if (!result) return;
    const lines = [
      "MetaRDU Industrial — EoM Reconciliation",
      `Previous: ${prevPath.split(/[\\/]/).pop()}`,
      `Current: ${currPath.split(/[\\/]/).pop()}`,
      `Density: ${density} t/m³  ·  Bench: ${benchInterval}m`,
      "",
      `Fill: ${result.fill_volume.toFixed(1)} m³  (${(result.fill_volume * density).toFixed(0)} t)`,
      `Cut:  ${result.cut_volume.toFixed(1)} m³  (${(result.cut_volume * density).toFixed(0)} t)`,
      `Net:  ${result.net_volume.toFixed(1)} m³  (${(result.net_volume * density).toFixed(0)} t)`,
    ];
    navigator.clipboard.writeText(lines.join("\n"));
    setCopied(true); setTimeout(() => setCopied(false), 2000);
  }

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="EOM Reconciliation"
      icon={<Calculator className="h-4 w-4" />}
      iconColor={colors.industrialOrange}
      maxWidth="max-w-2xl"
      subtitle="Monthly production reconciliation"
      footerHint="Compare actual vs mine plan"
      actions={
        <>
          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
        </>
      }
    >
          {error && <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>{error}</div>}

          <div className="grid grid-cols-2 gap-5">
            {/* LEFT: Inputs */}
            <div className="space-y-3">
              {/* Previous survey */}
              <div>
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Previous Survey (start of month)</label>
                <div className="flex items-center gap-2">
                  <button onClick={async () => { const p = await pickFile({ extensions: ["tif", "tiff"], filterName: "GeoTIFF DEM", title: "Select previous survey" }); if (p) setPrevPath(p); }} className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-1.5 text-xs text-white hover:bg-navy-elevated"><FolderOpen className="h-3.5 w-3.5" /> Browse</button>
                  {geotiffFiles.length > 0 && <select value={prevPath} onChange={(e) => setPrevPath(e.target.value)} className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"><option value="">— Or pick loaded —</option>{geotiffFiles.map((f) => <option key={f.id} value={f.path}>{f.name}</option>)}</select>}
                </div>
                {prevPath && <div className="mt-0.5 truncate font-mono text-[10px] text-steel-light">✓ {prevPath.split(/[\\/]/).pop()}</div>}
              </div>

              {/* Current survey */}
              <div>
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Current Survey (end of month)</label>
                <div className="flex items-center gap-2">
                  <button onClick={async () => { const p = await pickFile({ extensions: ["tif", "tiff"], filterName: "GeoTIFF DEM", title: "Select current survey" }); if (p) setCurrPath(p); }} className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-1.5 text-xs text-white hover:bg-navy-elevated"><FolderOpen className="h-3.5 w-3.5" /> Browse</button>
                  {geotiffFiles.length > 0 && <select value={currPath} onChange={(e) => setCurrPath(e.target.value)} className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"><option value="">— Or pick loaded —</option>{geotiffFiles.filter((f) => f.path !== prevPath).map((f) => <option key={f.id} value={f.path}>{f.name}</option>)}</select>}
                </div>
                {currPath && <div className="mt-0.5 truncate font-mono text-[10px] text-steel-light">✓ {currPath.split(/[\\/]/).pop()}</div>}
              </div>

              {/* Density + bench */}
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Density (t/m³)</label>
                  <input type="number" step="0.1" value={density} onChange={(e) => setDensity(parseFloat(e.target.value) || 2.7)} className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none" />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Bench (m)</label>
                  <input type="number" step="0.5" value={benchInterval} onChange={(e) => setBenchInterval(parseFloat(e.target.value) || 5)} className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none" />
                </div>
              </div>

              {/* Metadata */}
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Client / Mine</label>
                  <input type="text" value={clientName} onChange={(e) => setClientName(e.target.value)} placeholder="e.g., BHP Iron Ore" className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none" />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Site / Pit</label>
                  <input type="text" value={siteName} onChange={(e) => setSiteName(e.target.value)} placeholder="e.g., Pit A — June" className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none" />
                </div>
              </div>

              {/* Report path */}
              <div>
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Report output</label>
                <div className="flex items-center gap-2">
                  <button onClick={async () => { const p = await pickSaveFile({ extensions: ["html"], filterName: "HTML Report", title: "Save report" }); if (p) setReportPath(p); }} className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-1.5 text-xs text-white hover:bg-navy-elevated"><Download className="h-3.5 w-3.5" /> Save As</button>
                  <input type="text" value={reportPath} onChange={(e) => setReportPath(e.target.value)} className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none" />
                </div>
              </div>

              <button onClick={handleCompute} disabled={!canCompute || computing} className="flex w-full items-center justify-center gap-2 rounded-md px-4 py-2.5 text-sm font-bold transition-colors disabled:opacity-40" style={{ background: colors.industrialOrange, color: colors.navyBase }}>
                {computing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Calculator className="h-4 w-4" />}
                {computing ? "Computing…" : "Compute Reconciliation"}
              </button>
            </div>

            {/* RIGHT: Results */}
            <div>
              {!result ? (
                <div className="flex h-full items-center justify-center rounded-md border border-navy-border bg-navy-base p-8 text-center text-xs text-steel-gray">
                  <div>Select previous + current surveys, then click Compute.</div>
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="grid grid-cols-3 gap-2">
                    <ResultTile label="Fill" value={result.fill_volume.toFixed(0)} unit="m³" tonnage={(result.fill_volume * density).toFixed(0)} color={colors.pass} icon={<TrendingUp className="h-3 w-3" />} />
                    <ResultTile label="Cut" value={result.cut_volume.toFixed(0)} unit="m³" tonnage={(result.cut_volume * density).toFixed(0)} color={colors.fail} icon={<TrendingDown className="h-3 w-3" />} />
                    <ResultTile label="Net" value={result.net_volume.toFixed(0)} unit="m³" tonnage={(result.net_volume * density).toFixed(0)} color={colors.industrialOrange} icon={<Calculator className="h-3 w-3" />} />
                  </div>

                  {/* Fill/cut ratio bar */}
                  {(() => {
                    const total = result.fill_volume + result.cut_volume;
                    const fillPct = total > 0 ? (result.fill_volume / total) * 100 : 0;
                    const cutPct = total > 0 ? (result.cut_volume / total) * 100 : 0;
                    return (
                      <div>
                        <div className="mb-1 text-[9px] uppercase tracking-wider text-steel-gray">Fill / Cut Ratio</div>
                        <div className="flex h-5 overflow-hidden rounded-md">
                          <div style={{ width: `${fillPct}%`, background: colors.pass }} className="flex items-center justify-center text-[8px] font-bold text-white">{fillPct > 15 ? `${fillPct.toFixed(0)}%` : ""}</div>
                          <div style={{ width: `${cutPct}%`, background: colors.fail }} className="flex items-center justify-center text-[8px] font-bold text-white">{cutPct > 15 ? `${cutPct.toFixed(0)}%` : ""}</div>
                        </div>
                      </div>
                    );
                  })()}

                  {/* Bench breakdown */}
                  {result.benches.length > 0 && (
                    <div className="max-h-28 overflow-y-auto rounded-md border border-navy-border">
                      <table className="w-full text-left text-[9px]">
                        <thead className="sticky top-0 bg-navy-panel text-steel-gray"><tr><th className="px-1.5 py-1">Bench</th><th className="px-1.5 py-1 text-right">Fill m³</th><th className="px-1.5 py-1 text-right">Cut m³</th><th className="px-1.5 py-1 text-right">Fill t</th><th className="px-1.5 py-1 text-right">Cut t</th></tr></thead>
                        <tbody>
                          {result.benches.map((b, i) => (
                            <tr key={i} className="border-t border-navy-border">
                              <td className="px-1.5 py-0.5 font-mono text-steel-light">{b.z_min.toFixed(1)}–{b.z_max.toFixed(1)}</td>
                              <td className="px-1.5 py-0.5 text-right font-mono" style={{ color: colors.pass }}>{b.fill_volume > 0 ? b.fill_volume.toFixed(0) : "—"}</td>
                              <td className="px-1.5 py-0.5 text-right font-mono" style={{ color: colors.fail }}>{b.cut_volume > 0 ? b.cut_volume.toFixed(0) : "—"}</td>
                              <td className="px-1.5 py-0.5 text-right font-mono text-steel-light">{b.fill_volume > 0 ? (b.fill_volume * density).toFixed(0) : "—"}</td>
                              <td className="px-1.5 py-0.5 text-right font-mono text-steel-light">{b.cut_volume > 0 ? (b.cut_volume * density).toFixed(0) : "—"}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}

                  {/* Actions */}
                  <div className="flex items-center gap-2">
                    <button onClick={handleCopy} className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-3 py-1.5 text-xs text-white hover:bg-navy-elevated">
                      {copied ? <CheckCircle2 className="h-3 w-3" style={{ color: colors.pass }} /> : <Copy className="h-3 w-3" />}{copied ? "Copied!" : "Copy"}
                    </button>
                    <button onClick={handleGenerateReport} disabled={generating} className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40" style={{ background: colors.industrialOrange, color: colors.navyBase }}>
                      {generating ? <Loader2 className="h-3 w-3 animate-spin" /> : <FileText className="h-3 w-3" />}{reportGenerated ? "Report ✓" : "Generate PDF"}
                    </button>
                  </div>
                  {reportGenerated && <div className="text-[10px] text-steel-gray">Report: <span className="font-mono">{reportPath}</span></div>}
                </div>
              )}
            </div>
          </div>
    </DialogShell>
  );
}

function ResultTile({ label, value, unit, tonnage, color, icon }: { label: string; value: string; unit: string; tonnage: string; color: string; icon: React.ReactNode }) {
  return (
    <div className="rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="flex items-center gap-1 text-[9px] uppercase tracking-wider" style={{ color }}>{icon} {label}</div>
      <div className="mt-1 font-mono text-base font-bold text-white">{value}<span className="ml-0.5 text-[10px] font-normal text-steel-gray">{unit}</span></div>
      <div className="font-mono text-[10px]" style={{ color }}>{tonnage} t</div>
    </div>
  );
}
