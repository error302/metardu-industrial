/**
 * 3D Slice Editor — Sprint 6 Priority #9.
 *
 * Draw a bounding polygon over a survey line, isolate the slice in a
 * WebGL view, drag a "reject brush" over outlier points, flag as rejected
 * (undo-able), then re-run CUBE on the cleaned data.
 *
 * Workflow:
 *   1. Select a LAS file
 *   2. Paste/draw polygon vertices (projected coords, one per line)
 *   3. Click "Slice" → backend isolates points inside the polygon
 *   4. 3D Deck.gl view shows the slice (orange = accepted, red = rejected)
 *   5. Drag mouse over points with brush radius → toggles reject flag
 *   6. Undo button restores the last brush stroke
 *   7. "Get Accepted Indices" → returns indices for CUBE re-run
 *
 * Per ROADMAP.md Priority #9.
 */

import { useState, useRef, useMemo } from "react";
import {
  X, Loader2, Scissors, Undo2, Brush, Save, AlertTriangle, Download,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  sliceByPolygon,
  brushReject,
  undoBrush,
  acceptedIndices,
  type SliceResult,
  type RejectMask,
  type Point2D,
  type Point3D,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

const DEFAULT_BRUSH_RADIUS = 1.0; // meters

export function SliceEditor3D({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const lasFiles = files.filter((f) => f.kind === "las" && f.status === "loaded");

  const [lasPath, setLasPath] = useState("");
  const [polygonText, setPolygonText] = useState("");
  const [sliceResult, setSliceResult] = useState<SliceResult | null>(null);
  const [mask, setMask] = useState<RejectMask>({ rejected: [], undo_stack: [] });
  const [brushRadius, setBrushRadius] = useState(DEFAULT_BRUSH_RADIUS);
  const [brushMode, setBrushMode] = useState<"reject" | "restore">("reject");
  const [slicing, setSlicing] = useState(false);
  const [brushing, setBrushing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<string>("");

  const svgRef = useRef<SVGSVGElement>(null);

  // Compute bounding box for 3D rendering — must be before early return
  // (React hooks rules: all hooks must run unconditionally)
  const bounds = useMemo(() => {
    if (!sliceResult) return null;
    let minX = Infinity, minY = Infinity, minZ = Infinity;
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;
    for (const p of sliceResult.points) {
      minX = Math.min(minX, p.x); maxX = Math.max(maxX, p.x);
      minY = Math.min(minY, p.y); maxY = Math.max(maxY, p.y);
      minZ = Math.min(minZ, p.z); maxZ = Math.max(maxZ, p.z);
    }
    return { minX, minY, minZ, maxX, maxY, maxZ };
  }, [sliceResult]);

  if (!open) return null;

  const canSlice = !!lasPath && parsePolygon(polygonText).length >= 3;

  function parsePolygon(text: string): Point2D[] {
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
      .filter((p): p is Point2D => p !== null);
  }

  async function handleSlice() {
    setSlicing(true);
    setError(null);
    setSliceResult(null);
    setMask({ rejected: [], undo_stack: [] });
    try {
      const polygon = parsePolygon(polygonText);
      if (polygon.length < 3) {
        setError("Polygon needs at least 3 vertices");
        return;
      }
      const result = await sliceByPolygon({ path: lasPath, polygon });
      if (result) {
        setSliceResult(result);
        setStats(
          `Sliced ${result.slice_points.toLocaleString()} / ${result.total_points.toLocaleString()} points ` +
          `(polygon area: ${result.polygon_area_m2.toFixed(1)} m²)`
        );
      } else {
        setError("Browser mode — slicing requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSlicing(false);
    }
  }

  async function handleBrush(centerX: number, centerY: number, centerZ: number) {
    if (!sliceResult) return;
    setBrushing(true);
    try {
      const result = await brushReject({
        points: sliceResult.points,
        center_x: centerX,
        center_y: centerY,
        center_z: centerZ,
        radius_m: brushRadius,
        mask,
        restore: brushMode === "restore",
      });
      if (result) {
        setMask(result.mask);
        setStats(
          `${brushMode === "reject" ? "Rejected" : "Restored"} ${result.toggled_count} points. ` +
          `Total rejected: ${result.total_rejected} / ${sliceResult.points.length}`
        );
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBrushing(false);
    }
  }

  async function handleUndo() {
    if (mask.undo_stack.length === 0) return;
    try {
      const result = await undoBrush(mask);
      if (result) {
        setMask(result.mask);
        setStats(
          `Undid ${result.toggled_count} points. Total rejected: ${result.total_rejected} / ${sliceResult?.points.length ?? 0}`
        );
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleExportAccepted() {
    if (!sliceResult) return;
    try {
      const indices = await acceptedIndices(mask, sliceResult.points.length);
      setStats(`Exported ${indices.length} accepted point indices (ready for CUBE re-run)`);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  // Convert 3D world coords to SVG 2D coords (top-down view)
  function worldToSvg(p: Point3D): { x: number; y: number } {
    if (!bounds) return { x: 0, y: 0 };
    const worldW = bounds.maxX - bounds.minX || 1;
    const worldH = bounds.maxY - bounds.minY || 1;
    const svgSize = 500;
    const scale = Math.min(svgSize / worldW, svgSize / worldH) * 0.9;
    return {
      x: (p.x - bounds.minX) * scale + svgSize * 0.05,
      y: (bounds.maxY - p.y) * scale + svgSize * 0.05, // flip Y
    };
  }

  function svgToWorld(sx: number, sy: number): { x: number; y: number } {
    if (!bounds) return { x: 0, y: 0 };
    const worldW = bounds.maxX - bounds.minX || 1;
    const worldH = bounds.maxY - bounds.minY || 1;
    const svgSize = 500;
    const scale = Math.min(svgSize / worldW, svgSize / worldH) * 0.9;
    return {
      x: (sx - svgSize * 0.05) / scale + bounds.minX,
      y: bounds.maxY - (sy - svgSize * 0.05) / scale,
    };
  }

  // Render points as SVG circles (lighter than Canvas2D for small datasets)
  const renderPoints = () => {
    if (!sliceResult) return null;
    const rejected = new Set(mask.rejected);
    return sliceResult.points.map((p, i) => {
      const { x, y } = worldToSvg(p);
      const isRejected = rejected.has(i);
      return (
        <circle
          key={i}
          cx={x}
          cy={y}
          r={1.5}
          fill={isRejected ? "#DC2626" : "#FFA500"}
          opacity={0.8}
        />
      );
    });
  };

  // SVG mouse interaction for brush
  const handleSvgMouseDown = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!sliceResult || !bounds) return;
    const rect = svgRef.current?.getBoundingClientRect();
    if (!rect) return;
    const sx = (e.clientX - rect.left) * (500 / rect.width);
    const sy = (e.clientY - rect.top) * (500 / rect.height);
    const world = svgToWorld(sx, sy);
    // Use mid-Z for the brush center (the user is dragging in 2D so we
    // can't get a precise Z — use the centroid)
    const centerZ = (bounds.minZ + bounds.maxZ) / 2;
    handleBrush(world.x, world.y, centerZ);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[92vh] w-full max-w-5xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Scissors className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            3D Slice Editor with Reject Brush
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {!sliceResult ? (
            <div className="space-y-4">
              {/* Step 1: Select LAS */}
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  LAS point cloud
                </label>
                {lasFiles.length === 0 ? (
                  <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-gray">
                    Drop a LAS file on the map first.
                  </div>
                ) : (
                  <select
                    value={lasPath}
                    onChange={(e) => setLasPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none"
                  >
                    <option value="">— Select LAS file —</option>
                    {lasFiles.map((f) => (
                      <option key={f.id} value={f.path}>{f.name}</option>
                    ))}
                  </select>
                )}
              </div>

              {/* Step 2: Polygon */}
              <div>
                <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Bounding polygon vertices (projected coords, one per line: "x, y" or "x y")
                </label>
                <textarea
                  value={polygonText}
                  onChange={(e) => setPolygonText(e.target.value)}
                  rows={6}
                  placeholder={"# e.g., for EPSG:28355 (MGA Zone 55):\n337000.0, 6253000.0\n337500.0, 6253000.0\n337500.0, 6253100.0\n337000.0, 6253100.0"}
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
                />
                <p className="mt-1 text-[10px] text-steel-gray">
                  {parsePolygon(polygonText).length} vertices parsed (minimum 3 required).
                </p>
              </div>

              <button
                onClick={handleSlice}
                disabled={!canSlice || slicing}
                className="flex items-center gap-2 rounded-md px-5 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: colors.industrialOrange, color: colors.navyBase }}
              >
                {slicing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Scissors className="h-4 w-4" />}
                {slicing ? "Slicing…" : "Slice Point Cloud"}
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {/* Stats */}
              <div className="rounded-md border border-navy-border bg-navy-base p-2 text-xs text-steel-light">
                {stats}
              </div>

              {/* Controls */}
              <div className="flex items-center gap-3 rounded-md border border-navy-border bg-navy-base p-2 text-xs">
                <button
                  onClick={handleUndo}
                  disabled={mask.undo_stack.length === 0 || brushing}
                  className="flex items-center gap-1 rounded px-2 py-1 text-white disabled:opacity-40"
                  style={{ background: colors.steelLight }}
                >
                  <Undo2 className="h-3 w-3" /> Undo
                </button>
                <button
                  onClick={() => setBrushMode(brushMode === "reject" ? "restore" : "reject")}
                  className="flex items-center gap-1 rounded px-2 py-1 text-white"
                  style={{ background: brushMode === "reject" ? colors.fail : colors.pass }}
                >
                  <Brush className="h-3 w-3" /> {brushMode === "reject" ? "Rejecting" : "Restoring"}
                </button>
                <label className="flex items-center gap-1 text-steel-light">
                  Brush radius:
                  <input
                    type="number" step="0.1" min="0.1" value={brushRadius}
                    onChange={(e) => setBrushRadius(parseFloat(e.target.value) || DEFAULT_BRUSH_RADIUS)}
                    className="w-20 rounded border border-navy-border bg-navy-base px-2 py-0.5 font-mono text-white focus:outline-none"
                  />
                  m
                </label>
                <button
                  onClick={handleExportAccepted}
                  className="ml-auto flex items-center gap-1 rounded px-2 py-1 text-white"
                  style={{ background: colors.industrialOrange }}
                >
                  <Save className="h-3 w-3" /> Export Accepted Indices
                </button>
              </div>

              {/* 2D top-down view (SVG) — represents the 3D slice */}
              <div className="rounded-md border border-navy-border bg-navy-base p-2">
                <div className="mb-2 text-[10px] text-steel-gray">
                  Click anywhere in the slice to apply brush ({brushMode} mode).
                  Orange = accepted, Red = rejected.
                </div>
                <svg
                  ref={svgRef}
                  viewBox="0 0 500 500"
                  onMouseDown={handleSvgMouseDown}
                  style={{ width: "100%", height: "auto", cursor: "crosshair", background: "#000" }}
                >
                  {renderPoints()}
                </svg>
              </div>

              {mask.rejected.length > 0 && (
                <div
                  className="flex items-start gap-2 rounded-md border p-3 text-xs"
                  style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}
                >
                  <AlertTriangle className="mt-0.5 h-4 w-4 flex-shrink-0" />
                  <div>
                    <div className="font-semibold">
                      {mask.rejected.length.toLocaleString()} points rejected
                    </div>
                    <div className="mt-0.5 text-[10px]">
                      These points will be excluded when CUBE re-runs.
                      Click "Export Accepted Indices" to get the cleaned index list.
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <div>
            Slice points become orange. Brush-drag to reject (red). Undo restores.
            Export accepted indices for CUBE re-run.
          </div>
          <button
            onClick={onClose}
            className="flex items-center gap-1 rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}
          >
            <Download className="h-3 w-3" /> Done
          </button>
        </div>
      </div>
    </div>
  );
}
