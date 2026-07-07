/**
 * Tunnel Profile Analyzer — Sprint 10 Mining Field Tool #3.
 *
 * Computes cross-sectional area of an underground excavation, max width /
 * height, and overbreak/underbreak vs a design profile. Per-chainage
 * reporting supports drive advance reconciliation.
 *
 * Workflow:
 *   1. Enter chainage (m along drive)
 *   2. Enter as-built profile points (width, height) — width positive =
 *      right wall, negative = left wall; height positive = above floor
 *   3. (Optional) enter design profile for overbreak/underbreak
 *   4. Click Analyze → SVG preview + area / overbreak / underbreak stats
 */

import { useState, useMemo } from "react";
import { SquareDashed } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

interface TunnelProfileResult {
  area: number;
  design_area: number | null;
  overbreak: number | null;
  underbreak: number | null;
  max_width: number;
  max_height: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

export function TunnelProfileDialog({ open, onClose }: Props) {
  const [chainage, setChainage] = useState("125.0");
  const [asBuilt, setAsBuilt] = useState<string>([
    "-3.0,0.0",
    "-3.2,1.5",
    "-2.8,3.0",
    "-1.5,4.2",
    "0.0,4.5",
    "1.5,4.2",
    "2.8,3.0",
    "3.2,1.5",
    "3.0,0.0",
  ].join("\n"));
  const [design, setDesign] = useState<string>([
    "-2.8,0.0",
    "-2.8,2.8",
    "-1.4,4.0",
    "0.0,4.2",
    "1.4,4.0",
    "2.8,2.8",
    "2.8,0.0",
  ].join("\n"));
  const [result, setResult] = useState<TunnelProfileResult | null>(null);
  const [loading, setLoading] = useState(false);
  void loading;
  const [error, setError] = useState<string | null>(null);


  function parsePoints(text: string): [number, number][] {
    return text
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const parts = l.split(/[,\s]+/).map(Number);
        if (parts.length !== 2 || parts.some((n) => Number.isNaN(n))) {
          throw new Error(`Invalid point line: "${l}" — use "width,height"`);
        }
        return [parts[0], parts[1]] as [number, number];
      });
  }

  async function handleAnalyze() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      if (!isNative()) {
        setError("Browser mode — tunnel analysis requires the native Tauri shell");
        return;
      }
      const points = parsePoints(asBuilt);
      if (points.length < 3) throw new Error("As-built profile needs at least 3 points");
      const designPts = design.trim() ? parsePoints(design) : null;
      const profile = {
        chainage: parseFloat(chainage) || 0,
        points,
        design_profile: designPts && designPts.length >= 3 ? designPts : null,
      };
      const r = await invoke<TunnelProfileResult>("analyze_tunnel_profile_cmd", { profile });
      setResult(r);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  // SVG preview
  const W = 480, H = 360, pad = 40;
  const allPts = useMemo(() => {
    const asBuiltPts = asBuilt.split("\n").filter(Boolean).map((l) => {
      const [w, h] = l.split(/[,\s]+/).map(Number);
      return [w || 0, h || 0] as [number, number];
    });
    const designPts = design.split("\n").filter(Boolean).map((l) => {
      const [w, h] = l.split(/[,\s]+/).map(Number);
      return [w || 0, h || 0] as [number, number];
    });
    return [...asBuiltPts, ...designPts];
  }, [asBuilt, design]);

  const maxW = Math.max(5, ...allPts.map((p) => Math.abs(p[0]))) * 1.2;
  const maxH = Math.max(5, ...allPts.map((p) => p[1])) * 1.2;

  function toSvg(w: number, h: number): [number, number] {
    const x = pad + ((w + maxW) / (2 * maxW)) * (W - 2 * pad);
    const y = H - pad - (h / maxH) * (H - 2 * pad);
    return [x, y];
  }

  function buildPath(pts: [number, number][] | null, close = true): string {
    if (!pts || pts.length === 0) return "";
    const d = pts.map(([w, h], i) => {
      const [x, y] = toSvg(w, h);
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
    return close ? `${d} Z` : d;
  }

  const asBuiltPts = useMemo(() => asBuilt.split("\n").filter(Boolean).map((l) => {
    const [w, h] = l.split(/[,\s]+/).map(Number);
    return [w || 0, h || 0] as [number, number];
  }), [asBuilt]);
  const designPts = useMemo(() => design.split("\n").filter(Boolean).map((l) => {
    const [w, h] = l.split(/[,\s]+/).map(Number);
    return [w || 0, h || 0] as [number, number];
  }), [design]);

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Tunnel Profile Analyzer"
      icon={<SquareDashed className="h-4 w-4" />}
      iconColor={colors.industrialOrange}
      maxWidth="max-w-4xl"
      subtitle="Area + overbreak/underbreak"
      footerHint="SVG cross-section preview"
      actions={
        <>
        <DialogButton variant="primary" onClick={handleAnalyze}>Analyze</DialogButton>
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
        </>
      }
    >
          {/* Inputs */}
          <div className="space-y-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Chainage (m)</label>
              <input
                type="number"
                value={chainage}
                step="0.1"
                onChange={(e) => setChainage(e.target.value)}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-mining focus:outline-none"
              />
            </div>

            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                As-built Profile (width, height per line)
              </label>
              <textarea
                value={asBuilt}
                onChange={(e) => setAsBuilt(e.target.value)}
                rows={9}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-[10px] text-white focus:border-mining focus:outline-none"
                placeholder="-3.0,0.0"
              />
            </div>

            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Design Profile (optional)
              </label>
              <textarea
                value={design}
                onChange={(e) => setDesign(e.target.value)}
                rows={7}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-[10px] text-white focus:border-mining focus:outline-none"
              />
            </div>

            {error && (
              <div className="rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
                {error}
              </div>
            )}

            {result && (
              <div className="grid grid-cols-2 gap-2">
                <Stat label="Area" value={`${result.area.toFixed(2)} m²`} color={colors.mining} />
                <Stat label="Design Area" value={result.design_area != null ? `${result.design_area.toFixed(2)} m²` : "—"} color={colors.steelLight} />
                <Stat
                  label="Overbreak"
                  value={result.overbreak != null ? `${result.overbreak.toFixed(2)} m²` : "—"}
                  color={result.overbreak != null && result.overbreak > 0.1 ? colors.fail : colors.pass}
                />
                <Stat
                  label="Underbreak"
                  value={result.underbreak != null ? `${result.underbreak.toFixed(2)} m²` : "—"}
                  color={result.underbreak != null && result.underbreak > 0.1 ? colors.warn : colors.pass}
                />
                <Stat label="Max Width" value={`${result.max_width.toFixed(2)} m`} color={colors.steelLight} />
                <Stat label="Max Height" value={`${result.max_height.toFixed(2)} m`} color={colors.steelLight} />
              </div>
            )}
          </div>

          {/* SVG Preview */}
          <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3">
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Cross-Section Preview
            </div>
            <svg viewBox={`0 0 ${W} ${H}`} className="w-full">
              {/* Floor */}
              <line x1={pad} y1={H - pad} x2={W - pad} y2={H - pad} stroke={colors.steelLight} strokeWidth="1" strokeDasharray="2,2" />
              {/* Centerline */}
              <line x1={(W) / 2} y1={pad} x2={(W) / 2} y2={H - pad} stroke={colors.steelLight} strokeWidth="0.5" strokeDasharray="2,4" />

              {/* Design outline */}
              {designPts.length >= 3 && (
                <path d={buildPath(designPts)} fill={`${colors.marine}10`} stroke={colors.marine} strokeWidth="1.5" strokeDasharray="4,3" />
              )}

              {/* As-built outline */}
              {asBuiltPts.length >= 3 && (
                <path d={buildPath(asBuiltPts)} fill={`${colors.mining}15`} stroke={colors.mining} strokeWidth="2" />
              )}

              {/* As-built points */}
              {asBuiltPts.map(([w, h], i) => {
                const [x, y] = toSvg(w, h);
                return <circle key={i} cx={x} cy={y} r="2" fill={colors.mining} />;
              })}

              {/* Axis labels */}
              <text x={W / 2} y={H - 5} textAnchor="middle" fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">
                Width (m) — ±{maxW.toFixed(1)}
              </text>
              <text x={10} y={H / 2} textAnchor="middle" fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono"
                transform={`rotate(-90, 10, ${H / 2})`}>
                Height (m) — 0 to {maxH.toFixed(1)}
              </text>
            </svg>
            <div className="mt-2 flex gap-3 text-[9px] text-steel-gray">
              <span><span className="inline-block h-2 w-3 align-middle" style={{ background: colors.mining }} /> As-built</span>
              <span><span className="inline-block h-2 w-3 align-middle" style={{ background: colors.marine }} /> Design</span>
            </div>
          </div>
    </DialogShell>
  );
}

function Stat({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="card-enterprise rounded-md border p-2" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-xs font-bold text-white">{value}</div>
    </div>
  );
}
