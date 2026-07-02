/**
 * S-44 Compliance Certificate — Sprint 2 Revenue Feature #3.
 *
 * Generates a branded PDF S-44 compliance certificate after running
 * the compliance check. This is a regulatory deliverable that every
 * hydrographic survey must include.
 *
 * Revenue: $2,000-3,000/seat — regulatory mandate = guaranteed market.
 */

import { useState } from "react";
import { X, Shield, FileText, Loader2, CheckCircle2, Download } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  checkS44Compliance,
  generateReport,
  type S44Order,
  type S44ComplianceResult,
  type ReportSpec,
  type ReportStat,
  type ReportTable,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

const ORDER_LABELS: Record<S44Order, string> = {
  special: "Special Order (harbors, berthing)",
  order_1a: "Order 1a (harbor approaches)",
  order_1b: "Order 1b (coastal routes)",
  order_2: "Order 2 (open ocean)",
};

export function S44CertificateDialog({ open, onClose }: Props) {
  const [targetOrder, setTargetOrder] = useState<S44Order>("order_1a");
  const [csvInput, setCsvInput] = useState(
    Array.from({ length: 100 }, (_, i) => ({
      depth: 10 + (i % 5) * 2,
      v: 0.25 + (i % 3) * 0.05,
      h: 1.5 + (i % 4) * 0.5,
    }))
      .map((s) => `${s.depth},${s.v},${s.h}`)
      .join("\n"),
  );
  const [vessel, setVessel] = useState("");
  const [sonar, setSonar] = useState("");
  const [surveyArea, setSurveyArea] = useState("");
  const [reportPath, setReportPath] = useState("/tmp/s44_certificate.html");

  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<S44ComplianceResult | null>(null);
  const [generating, setGenerating] = useState(false);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  async function handleCheck() {
    setChecking(true);
    setError(null);
    setResult(null);
    setDone(false);
    try {
      const lines = csvInput.trim().split("\n");
      const soundings = lines
        .map((line) => {
          const parts = line.trim().split(",").map((s) => parseFloat(s.trim()));
          if (parts.length < 3 || parts.some(isNaN)) return null;
          return { depth: parts[0], vertical_tpu_95: parts[1], horizontal_tpu_95: parts[2] };
        })
        .filter((s): s is NonNullable<typeof s> => s !== null);

      if (soundings.length === 0) {
        setError("No valid soundings. Use CSV: depth,v_tpu_95,h_tpu_95 per line.");
        setChecking(false);
        return;
      }

      const r = await checkS44Compliance(soundings, targetOrder);
      if (r) setResult(r);
      else setError("Browser mode — S-44 check requires the native Tauri shell");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setChecking(false);
    }
  }

  async function handleGenerateCert() {
    if (!result) return;
    setGenerating(true);
    setError(null);
    try {
      const orderLabel = ORDER_LABELS[targetOrder];
      const statusColor = result.status === "pass" ? colors.pass :
                           result.status === "investigate" ? colors.investigate : colors.fail;

      const summary: ReportStat[] = [
        { label: "Status", value: result.status.toUpperCase(), unit: "", color: statusColor },
        { label: "Pass Rate", value: `${(result.pass_rate * 100).toFixed(1)}%`, unit: "", color: statusColor },
        { label: "Passing", value: result.passing_soundings.toLocaleString(), unit: "pts", color: colors.pass },
        { label: "Failing", value: result.failing_soundings.toLocaleString(), unit: "pts", color: colors.fail },
        { label: "Min Depth", value: result.min_depth.toFixed(1), unit: "m", color: colors.steelLight },
        { label: "Max Depth", value: result.max_depth.toFixed(1), unit: "m", color: colors.steelLight },
        { label: "Mean Depth", value: result.mean_depth.toFixed(1), unit: "m", color: colors.steelLight },
        { label: "Total", value: result.total_soundings.toLocaleString(), unit: "pts", color: colors.steelLight },
      ];

      const worstTable: ReportTable | undefined = result.worst_failures.length > 0 ? {
        title: "Worst Failures (Top 20)",
        headers: ["#", "Depth (m)", "V-TPU (m)", "V-Threshold (m)", "H-TPU (m)", "H-Threshold (m)", "Violation"],
        rows: result.worst_failures.map((f, i) => [
          `${i + 1}`,
          f.depth.toFixed(1),
          f.vertical_tpu_95.toFixed(3),
          f.vertical_threshold.toFixed(3),
          f.horizontal_tpu_95.toFixed(2),
          f.horizontal_threshold.toFixed(2),
          f.violation,
        ]),
      } : undefined;

      const spec: ReportSpec = {
        report_type: "s44_compliance",
        title: "IHO S-44 Compliance Certificate",
        subtitle: `${orderLabel} — ${surveyArea || "Survey Area"}`,
        client: vessel,
        metadata: {
          "Survey Order": orderLabel,
          "Vessel": vessel || "—",
          "Sonar": sonar || "—",
          "Survey Area": surveyArea || "—",
          "Total Soundings": result.total_soundings.toLocaleString(),
          "Date": new Date().toLocaleDateString(),
        },
        tables: worstTable ? [worstTable] : [],
        summary,
        provenance_hash: `s44-${Date.now().toString(36)}`,
        output_path: reportPath,
      };

      const r = await generateReport(spec);
      if (r) setDone(true);
      else setError("Browser mode — report generation requires the native Tauri shell");
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
        className="flex max-h-[88vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Shield className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            S-44 Compliance Certificate
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {!done && (
            <>
              {/* Survey metadata */}
              <div className="mb-4 grid grid-cols-2 gap-3">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Target order</label>
                  <select
                    value={targetOrder}
                    onChange={(e) => setTargetOrder(e.target.value as S44Order)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                  >
                    {(Object.entries(ORDER_LABELS) as [S44Order, string][]).map(([k, v]) => (
                      <option key={k} value={k}>{v}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Vessel</label>
                  <input type="text" value={vessel} onChange={(e) => setVessel(e.target.value)} placeholder="RV Solander" className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none" />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Sonar</label>
                  <input type="text" value={sonar} onChange={(e) => setSonar(e.target.value)} placeholder="EM 710" className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none" />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Survey area</label>
                  <input type="text" value={surveyArea} onChange={(e) => setSurveyArea(e.target.value)} placeholder="Port of Darwin" className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none" />
                </div>
              </div>

              {/* CSV input */}
              <div className="mb-4">
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Soundings (CSV: depth, vertical_tpu_95, horizontal_tpu_95)
                </label>
                <textarea
                  value={csvInput}
                  onChange={(e) => setCsvInput(e.target.value)}
                  rows={5}
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none"
                />
              </div>

              {/* Report path */}
              <div className="mb-4">
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Certificate output path</label>
                <input type="text" value={reportPath} onChange={(e) => setReportPath(e.target.value)} className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none" />
              </div>

              {/* Results */}
              {result && (
                <div className="mb-4 space-y-3">
                  <div
                    className="flex items-center gap-3 rounded-md border p-4"
                    style={{
                      borderColor: result.status === "pass" ? `${colors.pass}40` :
                                   result.status === "investigate" ? `${colors.investigate}40` : `${colors.fail}40`,
                      background: result.status === "pass" ? `${colors.pass}10` :
                                  result.status === "investigate" ? `${colors.investigate}10` : `${colors.fail}10`,
                    }}
                  >
                    <div
                      className="text-sm font-bold uppercase"
                      style={{ color: result.status === "pass" ? colors.pass :
                                       result.status === "investigate" ? colors.investigate : colors.fail }}
                    >
                      {result.status}
                    </div>
                    <div className="text-xs text-steel-light">
                      {result.passing_soundings} / {result.total_soundings} pass ({(result.pass_rate * 100).toFixed(1)}%)
                    </div>
                  </div>
                </div>
              )}
            </>
          )}

          {/* Done state */}
          {done && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Certificate Generated</h3>
              <p className="mt-1 text-sm text-steel-light">
                Written to: <span className="font-mono">{reportPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                Open in browser → Ctrl+P → Save as PDF for print-ready certificate.
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">IHO S-44 6th edition (2022)</div>
          {!done ? (
            <div className="flex gap-2">
              {!result && (
                <button
                  onClick={handleCheck}
                  disabled={checking}
                  className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
                  style={{ background: checking ? colors.steelGray : colors.marineTurquoise, color: colors.navyBase }}
                >
                  {checking ? <Loader2 className="h-3 w-3 animate-spin" /> : <Shield className="h-3 w-3" />}
                  {checking ? "Checking…" : "Check compliance"}
                </button>
              )}
              {result && !done && (
                <button
                  onClick={handleGenerateCert}
                  disabled={generating}
                  className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
                  style={{ background: generating ? colors.steelGray : colors.industrialOrange, color: colors.navyBase }}
                >
                  {generating ? <Loader2 className="h-3 w-3 animate-spin" /> : <FileText className="h-3 w-3" />}
                  {generating ? "Generating…" : "Generate certificate"}
                </button>
              )}
            </div>
          ) : (
            <button
              onClick={onClose}
              className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium"
              style={{ background: colors.pass, color: colors.navyBase }}
            >
              <Download className="h-3 w-3" /> Done
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
