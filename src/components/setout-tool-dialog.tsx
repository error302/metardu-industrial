/**
 * Setting Out & Markout Tool — Sprint 10 Mining Field Tool #1.
 *
 * Computes bearing, horizontal distance, slope distance, and slope angle
 * from a known reference peg to each design point. Used for blast-hole
 * collars, drill patterns, bench toes/crests, road centerlines, and peg
 * recovery.
 *
 * Workflow:
 *   1. Enter reference point coordinate (where the total station sits)
 *   2. Enter design points (or paste a CSV block)
 *   3. Click Compute → see bearing/distance/slope table
 *   4. Copy as markout sheet (CSV)
 */

import { useState } from "react";
import { X, MapPin, Crosshair, Loader2, Copy, Plus, Trash2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { useEscapeKey } from "@/lib/use-escape-key";

type SetoutPointType =
  | "blast_hole" | "peg" | "bench_toe" | "bench_crest"
  | "road_centerline" | "road_edge" | "drill_pattern"
  | "infrastructure" | "hazard_boundary" | "custom";

interface SetoutPoint {
  id: string;
  easting: number;
  northing: number;
  elevation: number;
  description: string;
  pointType: SetoutPointType;
}

interface SetoutResult {
  point: SetoutPoint;
  bearing_deg: number;
  distance_m: number;
  delta_z: number;
  slope_distance: number;
  slope_angle_deg: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

const POINT_TYPES: { value: SetoutPointType; label: string }[] = [
  { value: "blast_hole", label: "Blast Hole" },
  { value: "peg", label: "Survey Peg" },
  { value: "bench_toe", label: "Bench Toe" },
  { value: "bench_crest", label: "Bench Crest" },
  { value: "road_centerline", label: "Road Centerline" },
  { value: "road_edge", label: "Road Edge" },
  { value: "drill_pattern", label: "Drill Pattern" },
  { value: "infrastructure", label: "Infrastructure" },
  { value: "hazard_boundary", label: "Hazard Boundary" },
  { value: "custom", label: "Custom" },
];

export function SetoutToolDialog({ open, onClose }: Props) {
  const [refE, setRefE] = useState("1000.000");
  const [refN, setRefN] = useState("2000.000");
  const [refZ, setRefZ] = useState("50.000");
  const [points, setPoints] = useState<SetoutPoint[]>([
    { id: "P-001", easting: 1050, northing: 2050, elevation: 51.2, description: "Blast hole collar", pointType: "blast_hole" },
    { id: "P-002", easting: 1060, northing: 2045, elevation: 51.0, description: "Blast hole collar", pointType: "blast_hole" },
  ]);
  const [results, setResults] = useState<SetoutResult[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEscapeKey(onClose, open);
  if (!open) return null;

  function addPoint() {
    const idx = points.length + 1;
    setPoints([
      ...points,
      {
        id: `P-${String(idx).padStart(3, "0")}`,
        easting: 0,
        northing: 0,
        elevation: 0,
        description: "",
        pointType: "peg",
      },
    ]);
  }

  function removePoint(i: number) {
    setPoints(points.filter((_, idx) => idx !== i));
  }

  function updatePoint(i: number, patch: Partial<SetoutPoint>) {
    setPoints(points.map((p, idx) => (idx === i ? { ...p, ...patch } : p)));
  }

  async function handleCompute() {
    setLoading(true);
    setError(null);
    setResults(null);
    try {
      if (!isNative()) {
        setError("Browser mode — setout requires the native Tauri shell");
        return;
      }
      const refEasting = parseFloat(refE);
      const refNorthing = parseFloat(refN);
      const refElevation = parseFloat(refZ);
      if ([refEasting, refNorthing, refElevation].some((v) => Number.isNaN(v))) {
        throw new Error("Invalid reference coordinate — enter numeric values");
      }
      const result = await invoke<SetoutResult[]>("compute_setout_cmd", {
        points,
        refEasting,
        refNorthing,
        refElevation,
      });
      setResults(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function copyAsCsv() {
    if (!results) return;
    const header = "ID,Type,Easting,Northing,Elevation,Bearing(deg),HorizDist(m),DeltaZ(m),SlopeDist(m),SlopeAngle(deg),Description\n";
    const rows = results
      .map((r) =>
        [
          r.point.id,
          r.point.pointType,
          r.point.easting.toFixed(3),
          r.point.northing.toFixed(3),
          r.point.elevation.toFixed(3),
          r.bearing_deg.toFixed(2),
          r.distance_m.toFixed(3),
          r.delta_z.toFixed(3),
          r.slope_distance.toFixed(3),
          r.slope_angle_deg.toFixed(2),
          `"${r.point.description}"`,
        ].join(",")
      )
      .join("\n");
    navigator.clipboard.writeText(header + rows);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[90vh] w-full max-w-4xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Crosshair className="h-4 w-4" style={{ color: colors.mining }} />
            Setting Out &amp; Markout
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* Reference Point */}
          <div className="mb-4 rounded-md border p-3" style={{ borderColor: `${colors.mining}40`, background: `${colors.mining}08` }}>
            <div className="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.mining }}>
              <MapPin className="h-3 w-3" /> Reference Point (instrument station)
            </div>
            <div className="grid grid-cols-3 gap-2">
              <LabeledInput label="Easting (m)" value={refE} onChange={setRefE} />
              <LabeledInput label="Northing (m)" value={refN} onChange={setRefN} />
              <LabeledInput label="Elevation (m)" value={refZ} onChange={setRefZ} />
            </div>
          </div>

          {/* Design Points */}
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Design Points ({points.length})
            </h3>
            <button
              onClick={addPoint}
              className="flex items-center gap-1 rounded-md px-2.5 py-1 text-[10px] font-medium"
              style={{ background: colors.mining, color: colors.navyBase }}
            >
              <Plus className="h-3 w-3" /> Add Point
            </button>
          </div>

          <div className="mb-4 max-h-64 overflow-y-auto rounded-md border border-navy-border">
            <table className="w-full text-left text-[10px]">
              <thead className="sticky top-0 bg-navy-elevated text-steel-gray">
                <tr>
                  <th className="px-2 py-1.5">ID</th>
                  <th className="px-2 py-1.5">Type</th>
                  <th className="px-2 py-1.5 text-right">Easting</th>
                  <th className="px-2 py-1.5 text-right">Northing</th>
                  <th className="px-2 py-1.5 text-right">Elevation</th>
                  <th className="px-2 py-1.5">Description</th>
                  <th className="px-2 py-1.5"></th>
                </tr>
              </thead>
              <tbody>
                {points.map((p, i) => (
                  <tr key={i} className="border-t border-navy-border">
                    <td className="px-1 py-1">
                      <input
                        value={p.id}
                        onChange={(e) => updatePoint(i, { id: e.target.value })}
                        className="w-16 rounded border border-navy-border bg-navy-base px-1 py-0.5 font-mono text-white"
                      />
                    </td>
                    <td className="px-1 py-1">
                      <select
                        value={p.pointType}
                        onChange={(e) => updatePoint(i, { pointType: e.target.value as SetoutPointType })}
                        className="w-24 rounded border border-navy-border bg-navy-base px-1 py-0.5 text-steel-light"
                      >
                        {POINT_TYPES.map((t) => (
                          <option key={t.value} value={t.value}>{t.label}</option>
                        ))}
                      </select>
                    </td>
                    <td className="px-1 py-1">
                      <input
                        type="number"
                        value={p.easting}
                        step="0.001"
                        onChange={(e) => updatePoint(i, { easting: parseFloat(e.target.value) || 0 })}
                        className="w-20 rounded border border-navy-border bg-navy-base px-1 py-0.5 text-right font-mono text-white"
                      />
                    </td>
                    <td className="px-1 py-1">
                      <input
                        type="number"
                        value={p.northing}
                        step="0.001"
                        onChange={(e) => updatePoint(i, { northing: parseFloat(e.target.value) || 0 })}
                        className="w-20 rounded border border-navy-border bg-navy-base px-1 py-0.5 text-right font-mono text-white"
                      />
                    </td>
                    <td className="px-1 py-1">
                      <input
                        type="number"
                        value={p.elevation}
                        step="0.001"
                        onChange={(e) => updatePoint(i, { elevation: parseFloat(e.target.value) || 0 })}
                        className="w-16 rounded border border-navy-border bg-navy-base px-1 py-0.5 text-right font-mono text-white"
                      />
                    </td>
                    <td className="px-1 py-1">
                      <input
                        value={p.description}
                        onChange={(e) => updatePoint(i, { description: e.target.value })}
                        className="w-full rounded border border-navy-border bg-navy-base px-1 py-0.5 text-steel-light"
                      />
                    </td>
                    <td className="px-1 py-1">
                      <button
                        onClick={() => removePoint(i)}
                        className="rounded p-0.5 text-steel-gray hover:bg-fail/20 hover:text-fail"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Results */}
          {results && (
            <div>
              <div className="mb-2 flex items-center justify-between">
                <h3 className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Markout Sheet ({results.length} points)
                </h3>
                <button
                  onClick={copyAsCsv}
                  className="flex items-center gap-1 rounded-md px-2.5 py-1 text-[10px] font-medium"
                  style={{ background: colors.steelLight, color: colors.navyBase }}
                >
                  <Copy className="h-3 w-3" /> {copied ? "Copied!" : "Copy CSV"}
                </button>
              </div>
              <div className="max-h-64 overflow-y-auto rounded-md border border-navy-border">
                <table className="w-full text-left text-[10px]">
                  <thead className="sticky top-0 bg-navy-elevated text-steel-gray">
                    <tr>
                      <th className="px-2 py-1.5">ID</th>
                      <th className="px-2 py-1.5 text-right">Bearing (°)</th>
                      <th className="px-2 py-1.5 text-right">Horiz. Dist (m)</th>
                      <th className="px-2 py-1.5 text-right">ΔZ (m)</th>
                      <th className="px-2 py-1.5 text-right">Slope Dist (m)</th>
                      <th className="px-2 py-1.5 text-right">Slope (°)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {results.map((r, i) => (
                      <tr key={i} className="border-t border-navy-border">
                        <td className="px-2 py-1 font-mono text-steel-light">{r.point.id}</td>
                        <td className="px-2 py-1 text-right font-mono" style={{ color: colors.mining }}>{r.bearing_deg.toFixed(2)}</td>
                        <td className="px-2 py-1 text-right font-mono text-white">{r.distance_m.toFixed(3)}</td>
                        <td className="px-2 py-1 text-right font-mono text-steel-light">{r.delta_z >= 0 ? "+" : ""}{r.delta_z.toFixed(3)}</td>
                        <td className="px-2 py-1 text-right font-mono text-white">{r.slope_distance.toFixed(3)}</td>
                        <td className="px-2 py-1 text-right font-mono" style={{ color: colors.mining }}>{r.slope_angle_deg >= 0 ? "+" : ""}{r.slope_angle_deg.toFixed(2)}</td>
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
            Bearing 0°=N, clockwise. Slope angle positive = uphill from instrument.
          </div>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="rounded-md px-4 py-1.5 text-xs font-medium"
              style={{ background: colors.steelGray, color: colors.navyBase }}
            >
              Close
            </button>
            <button
              onClick={handleCompute}
              disabled={loading || points.length === 0}
              className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
              style={{ background: colors.mining, color: colors.navyBase }}
            >
              {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Crosshair className="h-3 w-3" />}
              {loading ? "Computing…" : "Compute Setout"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function LabeledInput({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <div>
      <label className="mb-0.5 block text-[9px] uppercase tracking-wider text-steel-gray">{label}</label>
      <input
        type="number"
        value={value}
        step="0.001"
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-mining focus:outline-none"
      />
    </div>
  );
}
