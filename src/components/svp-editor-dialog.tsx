import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * SVP Editor — Sprint 3 Priority #3.
 *
 * Import .svp/.asvp files, view the depth-vs-speed curve as an SVG graph,
 * edit individual points, and see summary statistics.
 *
 * Unlocks credible marine processing — without SVP correction, CUBE
 * surfaces have systematic refraction errors.
 */

import { useState, useMemo } from "react";
import { X, Waves, Upload, Loader2 } from "lucide-react";
import { colors } from "@/lib/tokens";

interface SvpPoint {
  depth: number;
  speed: number;
}

interface SvpProfile {
  source: string;
  cast_count: number;
  points: SvpPoint[];
  min_depth: number;
  max_depth: number;
  min_speed: number;
  max_speed: number;
  surface_speed: number;
  bottom_speed: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SvpEditorDialog({ open, onClose }: Props) {
  const [filePath, setFilePath] = useState("");
  const [profile, setProfile] = useState<SvpProfile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // SVG graph dimensions
  const W = 400, H = 300, pad = 40;
  const speedRange = profile ? profile.max_speed - profile.min_speed : 1;
  const depthRange = profile ? profile.max_depth - profile.min_depth : 1;

  // Build SVG path for the SVP curve — must be before early return
  // (React hooks rules: all hooks must run unconditionally)
  const pathD = useMemo(() => {
    if (!profile || profile.points.length === 0) return "";
    return profile.points.map((p, i) => {
      const x = pad + ((p.speed - profile.min_speed) / (speedRange || 1)) * (W - 2 * pad);
      const y = pad + ((p.depth - profile.min_depth) / (depthRange || 1)) * (H - 2 * pad);
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
  }, [profile, speedRange, depthRange]);

  useEscapeKey(onClose, open);
  if (!open) return null;

  async function handleLoad() {
    if (!filePath.trim()) return;
    setLoading(true);
    setError(null);
    setProfile(null);
    try {
      // Dynamically import to avoid circular deps
      const { invoke } = await import("@tauri-apps/api/core");
      const { isNative } = await import("@/lib/tauri-ipc");
      if (!isNative()) {
        setError("Browser mode — SVP parsing requires the native Tauri shell");
        setLoading(false);
        return;
      }
      const result = await invoke<SvpProfile>("parse_svp_cmd", { path: filePath });
      setProfile(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
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
            <Waves className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            SVP Editor (Sound Velocity Profile)
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* File input */}
          <div className="mb-4">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              SVP file path (.svp, .asvp, .csv)
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                placeholder="/path/to/survey.svp"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                onKeyDown={(e) => e.key === "Enter" && handleLoad()}
              />
              <button
                onClick={handleLoad}
                disabled={loading || !filePath.trim()}
                className="flex items-center gap-1.5 rounded-md px-4 py-2 text-xs font-medium disabled:opacity-40"
                style={{ background: colors.marineTurquoise, color: colors.navyBase }}
              >
                {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Upload className="h-3 w-3" />}
                {loading ? "Loading…" : "Load"}
              </button>
            </div>
            <p className="mt-1 text-[10px] text-steel-gray">
              Format: depth(m) speed(m/s) per line — comma or whitespace separated
            </p>
          </div>

          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Stats */}
          {profile && (
            <div className="mb-4 grid grid-cols-4 gap-2">
              <Stat label="Points" value={profile.cast_count.toString()} color={colors.steelLight} />
              <Stat label="Surface" value={`${profile.surface_speed.toFixed(1)} m/s`} color={colors.marineTurquoise} />
              <Stat label="Bottom" value={`${profile.bottom_speed.toFixed(1)} m/s`} color={colors.marineDeep} />
              <Stat label="Max Depth" value={`${profile.max_depth.toFixed(1)} m`} color={colors.steelLight} />
            </div>
          )}

          {/* Graph */}
          {profile && (
            <div className="rounded-md border border-navy-border bg-navy-base p-3">
              <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Depth vs Sound Speed
              </div>
              <svg viewBox={`0 0 ${W} ${H}`} className="w-full" style={{ maxHeight: "300px" }}>
                {/* Grid */}
                {[0.25, 0.5, 0.75].map((t) => (
                  <g key={t}>
                    <line x1={pad} y1={pad + t * (H - 2 * pad)} x2={W - pad} y2={pad + t * (H - 2 * pad)} stroke={colors.navyBorder} strokeWidth="0.5" />
                    <line x1={pad + t * (W - 2 * pad)} y1={pad} x2={pad + t * (W - 2 * pad)} y2={H - pad} stroke={colors.navyBorder} strokeWidth="0.5" />
                  </g>
                ))}

                {/* Axes */}
                <line x1={pad} y1={pad} x2={pad} y2={H - pad} stroke={colors.steelGray} strokeWidth="1" />
                <line x1={pad} y1={H - pad} x2={W - pad} y2={H - pad} stroke={colors.steelGray} strokeWidth="1" />

                {/* Axis labels */}
                <text x={W / 2} y={H - 5} textAnchor="middle" fill={colors.steelGray} fontSize="10" fontFamily="JetBrains Mono">
                  Speed (m/s)
                </text>
                <text x={12} y={H / 2} textAnchor="middle" fill={colors.steelGray} fontSize="10" fontFamily="JetBrains Mono"
                  transform={`rotate(-90, 12, ${H / 2})`}>
                  Depth (m)
                </text>

                {/* Tick labels */}
                <text x={pad} y={H - pad + 12} textAnchor="middle" fill={colors.steelGray} fontSize="8" fontFamily="JetBrains Mono">
                  {profile.min_speed.toFixed(0)}
                </text>
                <text x={W - pad} y={H - pad + 12} textAnchor="middle" fill={colors.steelGray} fontSize="8" fontFamily="JetBrains Mono">
                  {profile.max_speed.toFixed(0)}
                </text>
                <text x={pad - 5} y={H - pad + 3} textAnchor="end" fill={colors.steelGray} fontSize="8" fontFamily="JetBrains Mono">
                  {profile.min_depth.toFixed(0)}
                </text>
                <text x={pad - 5} y={pad + 3} textAnchor="end" fill={colors.steelGray} fontSize="8" fontFamily="JetBrains Mono">
                  {profile.max_depth.toFixed(0)}
                </text>

                {/* SVP curve */}
                <path d={pathD} fill="none" stroke={colors.marineTurquoise} strokeWidth="2" />

                {/* Data points */}
                {profile.points.map((p, i) => {
                  const x = pad + ((p.speed - profile.min_speed) / (speedRange || 1)) * (W - 2 * pad);
                  const y = pad + ((p.depth - profile.min_depth) / (depthRange || 1)) * (H - 2 * pad);
                  return <circle key={i} cx={x} cy={y} r="2" fill={colors.marineCyan} />;
                })}
              </svg>
            </div>
          )}

          {/* Data table */}
          {profile && profile.points.length <= 50 && (
            <div className="mt-4">
              <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Cast Data ({profile.points.length} points)
              </h4>
              <div className="max-h-32 overflow-y-auto rounded-md border border-navy-border">
                <table className="w-full text-left text-[10px]">
                  <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                    <tr>
                      <th className="px-2 py-1.5">#</th>
                      <th className="px-2 py-1.5 text-right">Depth (m)</th>
                      <th className="px-2 py-1.5 text-right">Speed (m/s)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {profile.points.map((p, i) => (
                      <tr key={i} className="border-t border-navy-border">
                        <td className="px-2 py-1 font-mono text-steel-gray">{i + 1}</td>
                        <td className="px-2 py-1 text-right font-mono text-steel-light">{p.depth.toFixed(1)}</td>
                        <td className="px-2 py-1 text-right font-mono" style={{ color: colors.marineTurquoise }}>{p.speed.toFixed(2)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            Sound velocity correction for multibeam ray tracing
          </div>
          <button
            onClick={onClose}
            className="rounded-md px-4 py-1.5 text-xs font-medium"
            style={{ background: colors.steelGray, color: colors.navyBase }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-sm font-bold text-white">{value}</div>
    </div>
  );
}
