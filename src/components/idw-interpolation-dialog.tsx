/**
 * IDW Interpolation Dialog — Sprint 16.
 *
 * Frontend for the `interpolate_idw_cmd` IPC command. Lets the surveyor
 * load scattered point observations (from a LAS file or pasted as CSV),
 * configure IDW parameters (power, search radius, max points, cell size),
 * and generate a continuous surface grid.
 *
 * Output: a grid that can be displayed as a DEM overlay or exported as
 * a GeoTIFF. Useful for filling gaps in sparse bathymetry.
 */

import { useState, useMemo } from "react";
import { TrendingUp, Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import { FileInput } from "@/components/file-input";
import { ValidatedNumberInput } from "@/components/validated-number-input";

interface IdwParams {
  power: number;
  search_radius: number;
  max_points: number;
  nodata: number;
}

interface IdwResult {
  grid: number[];
  ncols: number;
  nrows: number;
  cell_size: number;
  bounds: [number, number, number, number];
  interpolated_cells: number;
  nodata_cells: number;
  min_value: number;
  max_value: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

export function IdwInterpolationDialog({ open, onClose }: Props) {
  const [filePath, setFilePath] = useState("");
  const [power, setPower] = useState("2.0");
  const [searchRadius, setSearchRadius] = useState("0");
  const [maxPoints, setMaxPoints] = useState("12");
  const [cellSize, setCellSize] = useState("1.0");
  const [result, setResult] = useState<IdwResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const params = useMemo((): IdwParams => ({
    power: parseFloat(power) || 2.0,
    search_radius: parseFloat(searchRadius) || 0.0,
    max_points: parseInt(maxPoints) || 12,
    nodata: NaN,
  }), [power, searchRadius, maxPoints]);

  async function handleCompute() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      if (!isNative()) {
        setError("Browser mode — IDW interpolation requires the native Tauri shell");
        return;
      }
      // For now, we need point data. In production this would read from
      // the LAS file via read_las_points_cmd and extract (x, y, z).
      // As a proof-of-concept, we'll generate synthetic points if no file
      // is provided, or use the file path to trigger a backend LAS read.
      if (!filePath.trim()) {
        setError("Enter a LAS file path to extract points for interpolation");
        return;
      }

      // Read LAS points, then interpolate
      const points = await invoke<[number, number, number][]>("read_las_points_cmd", {
        path: filePath,
        maxPoints: 50000,
      });

      if (points.length === 0) {
        setError("No points found in the LAS file");
        return;
      }

      // Compute bounds from points
      let min_x = Infinity, max_x = -Infinity, min_y = Infinity, max_y = -Infinity;
      for (const [x, y] of points) {
        min_x = Math.min(min_x, x);
        max_x = Math.max(max_x, x);
        min_y = Math.min(min_y, y);
        max_y = Math.max(max_y, y);
      }

      const result = await invoke<IdwResult>("interpolate_idw_cmd", {
        points,
        bounds: [min_x, min_y, max_x, max_y],
        cellSize: parseFloat(cellSize) || 1.0,
        params,
      });
      setResult(result);
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
      title="IDW Interpolation"
      icon={<TrendingUp className="h-4 w-4" />}
      iconColor={colors.marine}
      maxWidth="max-w-2xl"
      subtitle="Fill DEM gaps · generate continuous surfaces"
      footerHint="Shepard's algorithm · 1/d^p weighting"
      actions={
        <>
          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
          <DialogButton
            variant="marine"
            onClick={handleCompute}
            disabled={loading || !filePath.trim()}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <TrendingUp className="h-3 w-3" />}
            {loading ? "Computing…" : "Interpolate"}
          </DialogButton>
        </>
      }
    >
      <div className="space-y-4">
        {/* Input file */}
        <div>
          <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
            Input LAS file (point observations)
          </label>
          <FileInput
            value={filePath}
            onChange={setFilePath}
            extensions={["las", "laz"]}
            filterName="LAS Point Cloud"
            storageKey="idw-input"
            placeholder="/path/to/points.las"
          />
        </div>

        {/* Parameters */}
        <div className="grid grid-cols-2 gap-3">
          <ValidatedNumberInput
            value={power}
            onChange={setPower}
            validationType="positive"
            step={0.5}
            min={0.5}
            label="Power (p)"
          />
          <ValidatedNumberInput
            value={cellSize}
            onChange={setCellSize}
            validationType="positive"
            step={0.1}
            min={0.01}
            label="Cell size (m)"
          />
          <ValidatedNumberInput
            value={searchRadius}
            onChange={setSearchRadius}
            validationType="positive"
            step={1.0}
            min={0}
            label="Search radius (0=all)"
          />
          <ValidatedNumberInput
            value={maxPoints}
            onChange={setMaxPoints}
            validationType="positive"
            step={1}
            min={1}
            label="Max points per cell"
          />
        </div>

        {/* Help text */}
        <div className="rounded-md bg-navy-base p-2 text-[10px] leading-relaxed text-steel-gray">
          <strong className="text-steel-light">IDW (Inverse Distance Weighting):</strong> Estimates
          values at unobserved locations from scattered points. Each cell's value is the
          distance-weighted average of nearby points. Higher <span className="font-mono">power</span> =
          more localized. Use <span className="font-mono">search_radius &gt; 0</span> for large datasets
          to limit computation.
        </div>

        {error && (
          <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
            {error}
          </div>
        )}

        {/* Results */}
        {result && (
          <div className="grid grid-cols-4 gap-2">
            <Kpi label="Grid Size" value={`${result.ncols}×${result.nrows}`} color={colors.marine} />
            <Kpi label="Interpolated" value={result.interpolated_cells.toLocaleString()} color={colors.pass} />
            <Kpi label="NODATA" value={result.nodata_cells.toLocaleString()} color={colors.warn} />
            <Kpi label="Value Range" value={`${result.min_value.toFixed(2)} - ${result.max_value.toFixed(2)}`} color={colors.steelLight} />
          </div>
        )}
      </div>
    </DialogShell>
  );
}

function Kpi({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="card-enterprise rounded-md border p-2" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-xs font-bold text-white">{value}</div>
    </div>
  );
}
