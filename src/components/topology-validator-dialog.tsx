/**
 * Topology Validator Dialog — Sprint 16.
 *
 * Frontend for the `validate_polygons_cmd` and `validate_lines_cmd` IPC
 * commands. Lets the surveyor validate GIS topology for quality assurance:
 *   - Self-intersection
 *   - Polygon overlap
 *   - Polygon gap
 *   - Dangles (line endpoints not connected)
 *   - Slivers (tiny polygons)
 *   - Null geometry, unclosed rings, too few points
 *
 * Activated the GIS QA Engineer agent methodology.
 */

import { useState, useMemo } from "react";
import { ShieldCheck, Loader2, AlertCircle, AlertTriangle, CheckCircle2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton, EmptyState } from "@/components/dialog-shell";
import { ValidatedNumberInput } from "@/components/validated-number-input";

interface TopologyError {
  rule: "self_intersection" | "polygon_overlap" | "polygon_gap" | "dangle" | "sliver" | "null_geometry" | "too_few_points" | "not_closed";
  severity: "error" | "warning";
  message: string;
  location: [number, number];
  feature_indices: number[];
}

interface TopologyReport {
  errors: TopologyError[];
  total_features: number;
  error_count: number;
  warning_count: number;
  passed: boolean;
}

interface TopologyParams {
  min_polygon_area: number;
  max_gap_width: number;
  tolerance: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

type GeometryType = "polygon" | "line";

const RULE_LABELS: Record<string, string> = {
  self_intersection: "Self-Intersection",
  polygon_overlap: "Polygon Overlap",
  polygon_gap: "Polygon Gap",
  dangle: "Dangle",
  sliver: "Sliver",
  null_geometry: "Null Geometry",
  too_few_points: "Too Few Points",
  not_closed: "Not Closed",
};

export function TopologyValidatorDialog({ open, onClose }: Props) {
  const [geoType, setGeoType] = useState<GeometryType>("polygon");
  const [inputText, setInputText] = useState("");
  const [minArea, setMinArea] = useState("1.0");
  const [maxGap, setMaxGap] = useState("0.5");
  const [tolerance, setTolerance] = useState("0.001");
  const [report, setReport] = useState<TopologyReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const params = useMemo((): TopologyParams => ({
    min_polygon_area: parseFloat(minArea) || 1.0,
    max_gap_width: parseFloat(maxGap) || 0.5,
    tolerance: parseFloat(tolerance) || 0.001,
  }), [minArea, maxGap, tolerance]);

  function parseInput(): { polygons?: number[][][][]; lines?: number[][][] } | null {
    // Expected format for polygons:
    //   Feature 1: (x1,y1) (x2,y2) (x3,y3) (x1,y1)
    //   Feature 2: ...
    // For lines: same but each line is one feature
    try {
      const lines = inputText.trim().split("\n").filter((l) => l.trim() && !l.trim().startsWith("#"));
      if (geoType === "polygon") {
        const polygons: number[][][][] = lines.map((line) => {
          const ring = line.trim().match(/\((-?\d+\.?\d*),\s*(-?\d+\.?\d*)\)/g);
          if (!ring) throw new Error(`Invalid format on line: ${line}`);
          return [ring.map((m) => {
            const [x, y] = m.slice(1, -1).split(",").map((s) => parseFloat(s.trim()));
            return [x, y];
          })];
        });
        return { polygons };
      } else {
        const linesData: number[][][] = lines.map((line) => {
          const pts = line.trim().match(/\((-?\d+\.?\d*),\s*(-?\d+\.?\d*)\)/g);
          if (!pts) throw new Error(`Invalid format on line: ${line}`);
          return pts.map((m) => {
            const [x, y] = m.slice(1, -1).split(",").map((s) => parseFloat(s.trim()));
            return [x, y];
          });
        });
        return { lines: linesData };
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to parse input");
      return null;
    }
  }

  async function handleValidate() {
    setLoading(true);
    setError(null);
    setReport(null);
    try {
      if (!isNative()) {
        setError("Browser mode — topology validation requires the native Tauri shell");
        return;
      }
      const parsed = parseInput();
      if (!parsed) return;

      if (geoType === "polygon" && parsed.polygons) {
        const result = await invoke<TopologyReport>("validate_polygons_cmd", {
          polygons: parsed.polygons,
          params,
        });
        setReport(result);
      } else if (geoType === "line" && parsed.lines) {
        const result = await invoke<TopologyReport>("validate_lines_cmd", {
          lines: parsed.lines,
          params,
        });
        setReport(result);
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Topology Validator"
      icon={<ShieldCheck className="h-4 w-4" />}
      iconColor={colors.pass}
      maxWidth="max-w-3xl"
      subtitle="GIS QA — validate polygon + line topology"
      footerHint="Checks: self-intersection, overlap, gap, dangle, sliver, unclosed rings"
      actions={
        <>
          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
          <DialogButton
            variant="success"
            onClick={handleValidate}
            disabled={loading || !inputText.trim()}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <ShieldCheck className="h-3 w-3" />}
            {loading ? "Validating…" : "Validate"}
          </DialogButton>
        </>
      }
    >
      <div className="space-y-4">
        {/* Geometry type toggle */}
        <div className="flex gap-1 rounded-md border border-navy-border bg-navy-base p-1">
          <button
            onClick={() => setGeoType("polygon")}
            className={`flex-1 rounded px-3 py-1.5 text-xs font-medium ${geoType === "polygon" ? "text-navy-base" : "text-steel-gray"}`}
            style={{ background: geoType === "polygon" ? colors.pass : "transparent" }}
          >
            Polygons
          </button>
          <button
            onClick={() => setGeoType("line")}
            className={`flex-1 rounded px-3 py-1.5 text-xs font-medium ${geoType === "line" ? "text-navy-base" : "text-steel-gray"}`}
            style={{ background: geoType === "line" ? colors.pass : "transparent" }}
          >
            Lines
          </button>
        </div>

        {/* Parameters */}
        <div className="grid grid-cols-3 gap-2">
          <ValidatedNumberInput
            value={minArea}
            onChange={setMinArea}
            validationType="positive"
            step={0.1}
            min={0.01}
            label="Min area (m²)"
          />
          <ValidatedNumberInput
            value={maxGap}
            onChange={setMaxGap}
            validationType="positive"
            step={0.1}
            min={0.01}
            label="Max gap (m)"
          />
          <ValidatedNumberInput
            value={tolerance}
            onChange={setTolerance}
            validationType="positive"
            step={0.001}
            min={0.0001}
            label="Tolerance (m)"
          />
        </div>

        {/* Input text area */}
        <div>
          <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
            {geoType === "polygon" ? "Polygon rings (one feature per line)" : "Line vertices (one line per feature)"}
          </label>
          <textarea
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            rows={6}
            placeholder={geoType === "polygon"
              ? "(0,0) (10,0) (10,10) (0,10) (0,0)\n(20,20) (30,20) (30,30) (20,30) (20,20)"
              : "(0,0) (10,0) (10,10)\n(10,10) (20,10) (20,20)"
            }
            className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-[10px] text-white focus:border-pass focus:outline-none"
          />
          <p className="mt-1 text-[9px] text-steel-gray">
            Format: one {geoType} per line, vertices as <span className="font-mono">(x,y)</span> pairs. Polygons must be closed (first point = last point).
          </p>
        </div>

        {error && (
          <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
            {error}
          </div>
        )}

        {/* Results */}
        {report && (
          <div className="space-y-3">
            {/* Summary */}
            <div className="flex items-center gap-3 rounded-md border p-3" style={{
              borderColor: report.passed ? `${colors.pass}40` : `${colors.fail}40`,
              background: report.passed ? `${colors.pass}08` : `${colors.fail}08`,
            }}>
              {report.passed ? (
                <CheckCircle2 className="h-5 w-5" style={{ color: colors.pass }} />
              ) : (
                <AlertCircle className="h-5 w-5" style={{ color: colors.fail }} />
              )}
              <div className="flex-1">
                <div className="text-sm font-semibold text-white">
                  {report.passed ? "All checks passed" : `${report.error_count} errors, ${report.warning_count} warnings`}
                </div>
                <div className="text-[10px] text-steel-gray">
                  {report.total_features} features validated
                </div>
              </div>
            </div>

            {/* Error list */}
            {report.errors.length > 0 && (
              <div className="max-h-48 overflow-y-auto rounded-md border border-navy-border">
                <table className="table-enterprise w-full text-left text-[10px]">
                  <thead className="sticky top-0 bg-navy-elevated text-steel-gray">
                    <tr>
                      <th className="px-2 py-1.5">Severity</th>
                      <th className="px-2 py-1.5">Rule</th>
                      <th className="px-2 py-1.5">Message</th>
                      <th className="px-2 py-1.5">Features</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.errors.map((e, i) => (
                      <tr key={i} className="border-t border-navy-border">
                        <td className="px-2 py-1">
                          {e.severity === "error" ? (
                            <AlertCircle className="h-3 w-3" style={{ color: colors.fail }} />
                          ) : (
                            <AlertTriangle className="h-3 w-3" style={{ color: colors.warn }} />
                          )}
                        </td>
                        <td className="px-2 py-1 font-mono text-steel-light">{RULE_LABELS[e.rule] ?? e.rule}</td>
                        <td className="px-2 py-1 text-steel-light">{e.message}</td>
                        <td className="px-2 py-1 font-mono text-steel-gray">{e.feature_indices.join(", ")}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}

        {!report && !loading && !error && (
          <EmptyState
            icon={<ShieldCheck className="h-8 w-8" />}
            title="No validation run yet"
            description="Enter polygon or line coordinates above, then click Validate to check for topology errors."
          />
        )}
      </div>
    </DialogShell>
  );
}
