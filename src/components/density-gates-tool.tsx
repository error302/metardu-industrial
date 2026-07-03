/**
 * Density Gates Tool — drag a folder → instant coverage heatmap.
 *
 * Single screen. No wizard. Drag folder, pick S-44 order, see green/red.
 */

import { useState, useCallback } from "react";
import {
  X, Loader2, Activity, AlertTriangle, FolderOpen, Map as MapIcon,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  runDensityGates,
  type CoverageReport,
  type CoverageStatus,
} from "@/lib/tauri-ipc";
import { pickFolder } from "@/lib/file-picker";

interface Props {
  open: boolean;
  onClose: () => void;
}

const STATUS_COLORS: Record<CoverageStatus, string> = {
  good: "#10B981",
  marginal: "#F59E0B",
  gap: "#EF4444",
  empty: "#1E293B",
};

const STATUS_LABELS: Record<CoverageStatus, string> = {
  good: "Good",
  marginal: "Marginal",
  gap: "Gap",
  empty: "Empty",
};

export function DensityGatesTool({ open, onClose }: Props) {
  const [folderPath, setFolderPath] = useState("");
  const [targetOrder, setTargetOrder] = useState("order_1a");
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<CoverageReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleBrowse = useCallback(async () => {
    const path = await pickFolder("Select sonar data folder");
    if (path) setFolderPath(path);
  }, []);

  const handleRun = useCallback(async () => {
    if (!folderPath) return;
    setRunning(true);
    setError(null);
    setReport(null);
    try {
      const r = await runDensityGates({
        folder_path: folderPath,
        target_order: targetOrder,
      });
      if (r) {
        setReport(r);
      } else {
        setError("Browser mode — density gates requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }, [folderPath, targetOrder]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[92vh] w-full max-w-4xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Activity className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            Density Gates — Coverage Validator
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body — single screen */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Input row — folder + order + run button */}
          <div className="flex items-end gap-3">
            <div className="flex-1">
              <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                Sonar data folder (.all / .s7k)
              </label>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleBrowse}
                  className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
                >
                  <FolderOpen className="h-3.5 w-3.5" /> Browse
                </button>
                <input
                  type="text"
                  value={folderPath}
                  onChange={(e) => setFolderPath(e.target.value)}
                  placeholder="Or type a folder path…"
                  className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
                />
              </div>
            </div>
            <div>
              <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                S-44 Order
              </label>
              <select
                value={targetOrder}
                onChange={(e) => setTargetOrder(e.target.value)}
                className="rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-white focus:outline-none"
              >
                <option value="special">Special (harbors)</option>
                <option value="order_1a">Order 1a (approaches)</option>
                <option value="order_1b">Order 1b (coastal)</option>
                <option value="order_2">Order 2 (open ocean)</option>
              </select>
            </div>
            <button
              onClick={handleRun}
              disabled={!folderPath || running}
              className="flex items-center gap-1 rounded-md px-4 py-2 text-sm font-bold transition-colors disabled:opacity-40"
              style={{ background: colors.marineTurquoise, color: colors.navyBase }}
            >
              {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Activity className="h-4 w-4" />}
              {running ? "Scanning…" : "Check Coverage"}
            </button>
          </div>

          {/* Results — coverage heatmap + stats */}
          {report && (
            <div className="space-y-3">
              {/* Summary stats */}
              <div className="grid grid-cols-5 gap-2">
                <StatTile label="Files" value={report.files_scanned.toString()} color={colors.steelLight} />
                <StatTile label="Total Pings" value={report.total_pings.toLocaleString()} color={colors.steelLight} />
                <StatTile label="Coverage" value={`${report.coverage_pct.toFixed(1)}%`}
                  color={report.coverage_pct > 90 ? colors.pass : report.coverage_pct > 70 ? "#F59E0B" : colors.fail} />
                <StatTile label="Gap Cells" value={report.gap_cells.toLocaleString()} color={colors.fail} />
                <StatTile label="Good Cells" value={report.good_cells.toLocaleString()} color={colors.pass} />
              </div>

              {/* Coverage heatmap — grid visualization */}
              <div>
                <div className="mb-2 flex items-center gap-2">
                  <MapIcon className="h-3.5 w-3.5 text-steel-gray" />
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Coverage Heatmap ({report.grid_cols}×{report.grid_rows} grid)
                  </span>
                </div>
                <div className="rounded-md border border-navy-border bg-navy-base p-3">
                  <CoverageHeatmap report={report} />
                </div>
              </div>

              {/* Legend */}
              <div className="flex items-center gap-4 text-[10px]">
                {(["good", "marginal", "gap", "empty"] as CoverageStatus[]).map((s) => (
                  <div key={s} className="flex items-center gap-1">
                    <span className="h-3 w-3 rounded-sm" style={{ background: STATUS_COLORS[s] }} />
                    <span className="text-steel-light">{STATUS_LABELS[s]}</span>
                  </div>
                ))}
              </div>

              {/* Warnings */}
              {report.warnings.length > 0 && (
                <div className="rounded-md border p-3 text-[10px]"
                  style={{ borderColor: "#F59E0B40", background: "#F59E0B10", color: "#F59E0B" }}>
                  <div className="flex items-center gap-1 font-semibold mb-1">
                    <AlertTriangle className="h-3 w-3" /> Warnings
                  </div>
                  <ul className="list-disc pl-4 space-y-0.5">
                    {report.warnings.map((w, i) => <li key={i}>{w}</li>)}
                  </ul>
                </div>
              )}

              {/* File summaries */}
              <div>
                <h4 className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Files Scanned ({report.file_summaries.length})
                </h4>
                <div className="max-h-32 overflow-y-auto rounded-md border border-navy-border">
                  <table className="w-full text-left text-[10px]">
                    <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                      <tr>
                        <th className="px-2 py-1">Filename</th>
                        <th className="px-2 py-1 text-right">Pings</th>
                        <th className="px-2 py-1 text-right">Est. Soundings</th>
                        <th className="px-2 py-1 text-right">Size</th>
                      </tr>
                    </thead>
                    <tbody>
                      {report.file_summaries.map((f, i) => (
                        <tr key={i} className="border-t border-navy-border">
                          <td className="px-2 py-1 font-mono text-steel-light truncate">{f.filename}</td>
                          <td className="px-2 py-1 text-right font-mono text-white">{f.pings.toLocaleString()}</td>
                          <td className="px-2 py-1 text-right font-mono text-steel-light">{f.est_soundings.toLocaleString()}</td>
                          <td className="px-2 py-1 text-right font-mono text-steel-gray">{(f.file_size_bytes / 1024 / 1024).toFixed(1)} MB</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          )}

          {!report && !running && (
            <div className="rounded-md border border-navy-border bg-navy-base p-8 text-center text-xs text-steel-gray">
              Drop a folder of .all or .s7k files above. The tool scans every file,
              extracts ping positions, and shows a coverage heatmap in seconds.
              <br />
              <span className="mt-2 block">Green = meets S-44 density · Yellow = marginal · Red = gap detected</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <span>Runs locally — no data leaves your machine. Catches coverage gaps while you're still on site.</span>
          <button onClick={onClose}
            className="rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

/** Render the coverage heatmap as a CSS grid of colored cells */
function CoverageHeatmap({ report }: { report: CoverageReport }) {
  const { cells, grid_cols, grid_rows } = report;
  // Cap display size to prevent rendering thousands of cells
  const maxDisplayCells = 2000;
  const step = Math.max(1, Math.ceil(cells.length / maxDisplayCells));
  const displayCells = cells.filter((_, i) => i % step === 0);
  const displayCols = Math.ceil(grid_cols / step);

  return (
    <div
      className="grid gap-px"
      style={{
        gridTemplateColumns: `repeat(${displayCols}, 1fr)`,
        aspectRatio: `${grid_cols} / ${grid_rows}`,
        maxWidth: "100%",
      }}
    >
      {displayCells.map((cell, i) => (
        <div
          key={i}
          className="rounded-sm"
          style={{
            background: STATUS_COLORS[cell.status],
            minHeight: "3px",
          }}
          title={`(${cell.center_lon.toFixed(5)}, ${cell.center_lat.toFixed(5)}) — ${cell.count} soundings — ${STATUS_LABELS[cell.status]}`}
        />
      ))}
    </div>
  );
}

function StatTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-2.5">
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="mt-0.5 font-mono text-sm font-bold" style={{ color }}>{value}</div>
    </div>
  );
}
