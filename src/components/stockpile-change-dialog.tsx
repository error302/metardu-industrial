/**
 * Stockpile Change Detection — Sprint 10 Volumetric Tool.
 *
 * Compare two LAS surveys of the same stockpile from different epochs
 * and produce a per-cell cut/fill report. Used for monthly inventory
 * reconciliation, progress claims, and hotspot detection (data errors
 * or unexpected material movement).
 *
 * Workflow:
 *   1. Pick current (newer) and previous (baseline) LAS files
 *   2. Set grid cell size + hotspot threshold
 *   3. Click Compute → see cut/fill volumes, net change, heatmap, hotspot list
 */

import { useState, useMemo } from "react";
import { History, AlertTriangle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { FileInput } from "@/components/file-input";
import { ValidatedNumberInput } from "@/components/validated-number-input";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

interface ChangeDetectionResult {
  cut_volume_m3: number;
  fill_volume_m3: number;
  net_change_m3: number;
  cut_cells: number;
  fill_cells: number;
  compared_cells: number;
  no_overlap_cells: number;
  cell_size_m: number;
  grid_dims: [number, number];
  bounds: [number, number, number, number];
  delta_grid: number[];
  hotspots: [number, number, number][]; // [row, col, delta_z]
  mean_delta: number;
  std_delta: number;
  max_fill: number;
  max_cut: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

export function StockpileChangeDialog({ open, onClose }: Props) {
  const [currentPath, setCurrentPath] = useState("");
  const [previousPath, setPreviousPath] = useState("");
  const [cellSize, setCellSize] = useState("1.0");
  const [hotspotThreshold, setHotspotThreshold] = useState("2.0");
  const [result, setResult] = useState<ChangeDetectionResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);


  async function handleCompute() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      if (!isNative()) {
        setError("Browser mode — change detection requires the native Tauri shell");
        return;
      }
      const r = await invoke<ChangeDetectionResult>("compute_stockpile_change_cmd", {
        currentPath,
        previousPath,
        cellSizeM: parseFloat(cellSize) || 1.0,
        hotspotThresholdM: parseFloat(hotspotThreshold) || 2.0,
      });
      setResult(r);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  // Render the delta heatmap as an SVG (capped at ~600x600 cells)
  const heatmap = useMemo(() => {
    if (!result) return null;
    const [ncols, nrows] = result.grid_dims;
    // Cap dimensions for visualization — sample if grid is too large
    const maxDim = 500;
    const step = Math.max(1, Math.ceil(Math.max(ncols, nrows) / maxDim));
    const dispCols = Math.ceil(ncols / step);
    const dispRows = Math.ceil(nrows / step);
    const cellPx = Math.max(2, Math.min(8, Math.floor(500 / Math.max(dispCols, dispRows))));

    // Find min/max delta for color scale
    let minD = Infinity, maxD = -Infinity;
    for (let i = 0; i < result.delta_grid.length; i++) {
      const v = result.delta_grid[i];
      if (!Number.isNaN(v)) {
        if (v < minD) minD = v;
        if (v > maxD) maxD = v;
      }
    }
    if (!Number.isFinite(minD)) { minD = -1; maxD = 1; }
    const range = Math.max(0.001, Math.max(Math.abs(minD), Math.abs(maxD)) * 2);

    // Build cells
    const cells: { x: number; y: number; color: string }[] = [];
    for (let r = 0; r < dispRows; r++) {
      for (let c = 0; c < dispCols; c++) {
        // Sample the underlying grid — average of `step × step` block
        let sum = 0, count = 0;
        for (let dr = 0; dr < step && r * step + dr < nrows; dr++) {
          for (let dc = 0; dc < step && c * step + dc < ncols; dc++) {
            const v = result.delta_grid[(r * step + dr) * ncols + (c * step + dc)];
            if (!Number.isNaN(v)) { sum += v; count++; }
          }
        }
        if (count === 0) continue;
        const avg = sum / count;
        // Color scale: red (cut) → white (no change) → green (fill)
        // const t = (avg + range / 2) / range; // 0..1 — unused; intensity computed directly below
        let color: string;
        if (avg > 0.01) {
          // Green for fill
          const intensity = Math.min(1, avg / (range / 2));
          color = `rgba(34, 197, 94, ${0.3 + 0.7 * intensity})`;
        } else if (avg < -0.01) {
          // Red for cut
          const intensity = Math.min(1, -avg / (range / 2));
          color = `rgba(239, 68, 68, ${0.3 + 0.7 * intensity})`;
        } else {
          color = "rgba(100, 116, 139, 0.2)";
        }
        cells.push({ x: c * cellPx, y: r * cellPx, color });
      }
    }
    return { cells, dispCols, dispRows, cellPx, minD, maxD };
  }, [result]);

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Stockpile Change Detection"
      icon={<History className="h-4 w-4" />}
      iconColor={colors.industrialOrange}
      maxWidth="max-w-5xl"
      subtitle="Cut/fill heat map"
      footerHint="Median rasterization"
      actions={
        <>
        <DialogButton variant="primary" onClick={handleCompute}>Compute</DialogButton>
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
        </>
      }
    >
          {/* Left: inputs + stats */}
          <div className="space-y-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Current Survey (newer)
              </label>
              <FileInput
                value={currentPath}
                onChange={setCurrentPath}
                extensions={["las", "laz"]}
                filterName="LAS Point Cloud"
                storageKey="stockpile-change-current"
                placeholder="/path/to/month-2.las"
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Previous Survey (baseline)
              </label>
              <FileInput
                value={previousPath}
                onChange={setPreviousPath}
                extensions={["las", "laz"]}
                filterName="LAS Point Cloud"
                storageKey="stockpile-change-previous"
                placeholder="/path/to/month-1.las"
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <ValidatedNumberInput
                value={cellSize}
                onChange={setCellSize}
                validationType="positive"
                step={0.1}
                min={0.1}
                label="Cell Size (m)"
              />
              <ValidatedNumberInput
                value={hotspotThreshold}
                onChange={setHotspotThreshold}
                validationType="positive"
                step={0.1}
                min={0.1}
                label="Hotspot (m)"
              />
            </div>

            {error && (
              <div className="rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
                {error}
              </div>
            )}

            {result && (
              <>
                {/* KPIs */}
                <div className="grid grid-cols-2 gap-1.5">
                  <Kpi label="Cut Volume" value={`${result.cut_volume_m3.toFixed(1)} m³`} color={colors.fail} />
                  <Kpi label="Fill Volume" value={`${result.fill_volume_m3.toFixed(1)} m³`} color={colors.pass} />
                  <Kpi
                    label="Net Change"
                    value={`${result.net_change_m3 >= 0 ? "+" : ""}${result.net_change_m3.toFixed(1)} m³`}
                    color={result.net_change_m3 >= 0 ? colors.pass : colors.fail}
                  />
                  <Kpi label="Grid" value={`${result.grid_dims[0]}×${result.grid_dims[1]}`} color={colors.steelLight} />
                  <Kpi label="Cut Cells" value={result.cut_cells.toLocaleString()} color={colors.fail} />
                  <Kpi label="Fill Cells" value={result.fill_cells.toLocaleString()} color={colors.pass} />
                  <Kpi label="Max Fill" value={`+${result.max_fill.toFixed(2)} m`} color={colors.pass} />
                  <Kpi label="Max Cut" value={`${result.max_cut.toFixed(2)} m`} color={colors.fail} />
                  <Kpi label="Mean Δ" value={`${result.mean_delta >= 0 ? "+" : ""}${result.mean_delta.toFixed(3)} m`} color={colors.steelLight} />
                  <Kpi label="Std Δ" value={`±${result.std_delta.toFixed(3)} m`} color={colors.steelLight} />
                </div>

                {/* Hotspots */}
                {result.hotspots.length > 0 && (
                  <div>
                    <div className="mb-1 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.warn }}>
                      <AlertTriangle className="h-3 w-3" /> Hotspots ({result.hotspots.length})
                    </div>
                    <div className="max-h-32 overflow-y-auto rounded-md border border-navy-border bg-navy-base">
                      <table className="w-full text-left text-[10px]">
                        <thead className="sticky top-0 bg-navy-elevated text-steel-gray">
                          <tr>
                            <th className="px-2 py-1">#</th>
                            <th className="px-2 py-1 text-right">Row</th>
                            <th className="px-2 py-1 text-right">Col</th>
                            <th className="px-2 py-1 text-right">ΔZ (m)</th>
                          </tr>
                        </thead>
                        <tbody>
                          {result.hotspots.slice(0, 50).map((h, i) => (
                            <tr key={i} className="border-t border-navy-border">
                              <td className="px-2 py-0.5 text-steel-gray">{i + 1}</td>
                              <td className="px-2 py-0.5 text-right font-mono text-steel-light">{h[0]}</td>
                              <td className="px-2 py-0.5 text-right font-mono text-steel-light">{h[1]}</td>
                              <td className="px-2 py-0.5 text-right font-mono" style={{ color: h[2] >= 0 ? colors.pass : colors.fail }}>
                                {h[2] >= 0 ? "+" : ""}{h[2].toFixed(2)}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                      {result.hotspots.length > 50 && (
                        <div className="border-t border-navy-border p-1 text-center text-[9px] text-steel-gray">
                          +{(result.hotspots.length - 50).toLocaleString()} more
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </>
            )}
          </div>

          {/* Right: heatmap */}
          <div className="rounded-md border border-navy-border bg-navy-base p-3">
            <div className="mb-2 flex items-center justify-between">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Cut / Fill Heat Map</span>
              {heatmap && (
                <span className="font-mono text-[10px] text-steel-gray">
                  range: {heatmap.minD.toFixed(2)} to {heatmap.maxD.toFixed(2)} m
                </span>
              )}
            </div>
            {heatmap ? (
              <div className="overflow-auto">
                <svg
                  width={heatmap.dispCols * heatmap.cellPx}
                  height={heatmap.dispRows * heatmap.cellPx}
                  style={{ imageRendering: "pixelated", maxHeight: "460px" }}
                >
                  {heatmap.cells.map((cell, i) => (
                    <rect
                      key={i}
                      x={cell.x}
                      y={cell.y}
                      width={heatmap.cellPx}
                      height={heatmap.cellPx}
                      fill={cell.color}
                    />
                  ))}
                </svg>
                <div className="mt-2 flex items-center gap-3 text-[9px] text-steel-gray">
                  <span className="flex items-center gap-1">
                    <span className="inline-block h-2 w-4" style={{ background: colors.fail }} /> Cut (material removed)
                  </span>
                  <span className="flex items-center gap-1">
                    <span className="inline-block h-2 w-4" style={{ background: colors.pass }} /> Fill (material added)
                  </span>
                </div>
              </div>
            ) : (
              <div className="flex h-64 items-center justify-center text-[10px] text-steel-gray">
                {loading ? "Computing…" : "Heat map will render here after computation."}
              </div>
            )}
          </div>
    </DialogShell>
  );
}

function Kpi({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-xs font-bold text-white">{value}</div>
    </div>
  );
}
