import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * S-44 Compliance Dialog — Phase 2 Marine MVP.
 *
 * Check IHO S-44 (6th edition, 2022) compliance for a batch of soundings.
 * The user inputs synthetic sounding data (depth + TPU) or selects from
 * loaded marine files, picks a target order (Special / 1a / 1b / 2), and
 * gets pass/fail/investigate status with per-sounding breakdown.
 */

import { useState } from "react";
import { X, Shield, Loader2, AlertTriangle, CheckCircle2, XCircle } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  checkS44Compliance,
  type S44CheckInput,
  type S44ComplianceResult,
  type S44Order,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

const ORDER_LABELS: Record<S44Order, string> = {
  exclusive: "Exclusive Order (critical under-keel clearance, ≥6.1.0)",
  special: "Special Order (harbors, berthing)",
  order_1a: "Order 1a (harbor approaches)",
  order_1b: "Order 1b (coastal routes)",
  order_2: "Order 2 (open ocean, deep water)",
};

export function S44ComplianceDialog({ open, onClose }: Props) {
  const [targetOrder, setTargetOrder] = useState<S44Order>("order_1a");
  const [soundingsInput, setSoundingsInput] = useState<string>(
    // Default: 100 soundings at 10m depth, 0.25m TPU, 1.5m horizontal TPU
    Array.from({ length: 100 }, (_, i) => ({
      depth: 10 + (i % 5) * 2,
      vertical_tpu_95: 0.25 + (i % 3) * 0.05,
      horizontal_tpu_95: 1.5 + (i % 4) * 0.5,
    }))
      .map((s) => `${s.depth},${s.vertical_tpu_95},${s.horizontal_tpu_95}`)
      .join("\n"),
  );
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<S44ComplianceResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  async function handleCheck() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const lines = soundingsInput.trim().split("\n");
      const soundings: S44CheckInput[] = lines
        .map((line) => {
          const parts = line.trim().split(",").map((s) => parseFloat(s.trim()));
          if (parts.length < 3 || parts.some(isNaN)) return null;
          return {
            depth: parts[0],
            vertical_tpu_95: parts[1],
            horizontal_tpu_95: parts[2],
          };
        })
        .filter((s): s is S44CheckInput => s !== null);

      if (soundings.length === 0) {
        setError("No valid soundings. Use CSV: depth,v_tpu_95,h_tpu_95 per line.");
        setLoading(false);
        return;
      }

      const r = await checkS44Compliance(soundings, targetOrder);
      if (r) {
        setResult(r);
      } else {
        setError("Browser mode — S-44 check requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const statusColor = result?.status === "pass" ? colors.pass :
                       result?.status === "investigate" ? colors.investigate :
                       result?.status === "fail" ? colors.fail : colors.steelGray;
  const statusIcon = result?.status === "pass" ? <CheckCircle2 className="h-5 w-5" /> :
                     result?.status === "investigate" ? <AlertTriangle className="h-5 w-5" /> :
                     result?.status === "fail" ? <XCircle className="h-5 w-5" /> : null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[85vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Shield className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            S-44 Compliance Check
          </h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* Target order */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Target survey order (IHO S-44 6th ed.)
            </label>
            <select
              value={targetOrder}
              onChange={(e) => setTargetOrder(e.target.value as S44Order)}
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
            >
              {(Object.entries(ORDER_LABELS) as [S44Order, string][]).map(([k, v]) => (
                <option key={k} value={k}>
                  {v}
                </option>
              ))}
            </select>
          </section>

          {/* Soundings input */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Soundings (CSV: depth, vertical_tpu_95, horizontal_tpu_95)
            </label>
            <textarea
              value={soundingsInput}
              onChange={(e) => setSoundingsInput(e.target.value)}
              rows={6}
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none"
              placeholder="10.0,0.25,1.5&#10;12.0,0.27,1.6"
            />
            <p className="mt-1 text-[10px] text-steel-gray">
              One sounding per line. Units: meters. TPU at 95% confidence.
            </p>
          </section>

          {/* Error */}
          {error && (
            <div
              className="mb-4 rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}
            >
              {error}
            </div>
          )}

          {/* Result */}
          {result && (
            <div className="space-y-4">
              {/* Status banner */}
              <div
                className="flex items-center gap-3 rounded-md border p-4"
                style={{ borderColor: `${statusColor}40`, background: `${statusColor}10` }}
              >
                <div style={{ color: statusColor }}>{statusIcon}</div>
                <div className="flex-1">
                  <div className="text-sm font-semibold uppercase" style={{ color: statusColor }}>
                    {result.status}
                  </div>
                  <div className="text-xs text-steel-light">
                    {result.passing_soundings} of {result.total_soundings} soundings pass ({(result.pass_rate * 100).toFixed(1)}%)
                  </div>
                </div>
              </div>

              {/* Summary tiles */}
              <div className="grid grid-cols-4 gap-2">
                <SummaryTile label="Pass" value={result.passing_soundings} color={colors.pass} />
                <SummaryTile label="Fail" value={result.failing_soundings} color={colors.fail} />
                <SummaryTile label="Min depth" value={`${result.min_depth.toFixed(1)}m`} color={colors.steelLight} />
                <SummaryTile label="Max depth" value={`${result.max_depth.toFixed(1)}m`} color={colors.steelLight} />
              </div>

              {/* Worst failures */}
              {result.worst_failures.length > 0 && (
                <div>
                  <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Worst Failures (top {result.worst_failures.length})
                  </h4>
                  <div className="max-h-40 overflow-y-auto rounded-md border border-navy-border">
                    <table className="w-full text-left text-[10px]">
                      <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                        <tr>
                          <th className="px-2 py-1.5">#</th>
                          <th className="px-2 py-1.5 text-right">Depth</th>
                          <th className="px-2 py-1.5 text-right">V TPU</th>
                          <th className="px-2 py-1.5 text-right">V Thresh</th>
                          <th className="px-2 py-1.5 text-right">H TPU</th>
                          <th className="px-2 py-1.5 text-right">H Thresh</th>
                          <th className="px-2 py-1.5">Type</th>
                        </tr>
                      </thead>
                      <tbody>
                        {result.worst_failures.map((f, i) => (
                          <tr key={i} className="border-t border-navy-border">
                            <td className="px-2 py-1.5 font-mono">{f.index}</td>
                            <td className="px-2 py-1.5 text-right font-mono">{f.depth.toFixed(1)}m</td>
                            <td className="px-2 py-1.5 text-right font-mono" style={{ color: colors.fail }}>
                              {f.vertical_tpu_95.toFixed(3)}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono text-steel-gray">
                              {f.vertical_threshold.toFixed(3)}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono" style={{ color: colors.fail }}>
                              {f.horizontal_tpu_95.toFixed(2)}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono text-steel-gray">
                              {f.horizontal_threshold.toFixed(2)}
                            </td>
                            <td className="px-2 py-1.5 uppercase text-steel-gray">{f.violation}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            IHO S-44 6th edition (2022) — vertical: √(a² + (b×d)²)
          </div>
          <button
            onClick={handleCheck}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{
              background: loading ? colors.steelGray : colors.marineTurquoise,
              color: colors.navyBase,
            }}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Shield className="h-3 w-3" />}
            {loading ? "Checking…" : "Check compliance"}
          </button>
        </div>
      </div>
    </div>
  );
}

function SummaryTile({
  label,
  value,
  color,
}: {
  label: string;
  value: string | number;
  color: string;
}) {
  return (
    <div
      className="rounded-md border p-2.5"
      style={{ borderColor: `${color}40`, background: `${color}10` }}
    >
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>
        {label}
      </div>
      <div className="mt-0.5 font-mono text-sm font-semibold text-white">{value}</div>
    </div>
  );
}
