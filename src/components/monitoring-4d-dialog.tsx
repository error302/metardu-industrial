import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * 4D Monitoring Dialog — Phase 3.
 *
 * Compare two GeoTIFF DEMs from different survey epochs, compute
 * elevation differences, and show fill/cut/hotspot statistics.
 */

import { useState } from "react";
import { X, TrendingUp, Loader2, Activity, AlertTriangle } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  computeEpochDiff,
  type EpochDiff,
  type Monitoring4DParams,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function Monitoring4DDialog({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const demFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [prevPath, setPrevPath] = useState("");
  const [currPath, setCurrPath] = useState("");
  const [density, setDensity] = useState(2.7);
  const [hotspotThreshold, setHotspotThreshold] = useState(1.0);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<EpochDiff | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  const canCompute = prevPath && currPath && prevPath !== currPath;

  async function handleCompute() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const params: Monitoring4DParams = {
        cell_area: 1.0,
        density,
        hotspot_threshold: hotspotThreshold,
        active_threshold: 0.1,
      };
      const r = await computeEpochDiff(prevPath, currPath, params);
      if (r) {
        setResult(r);
      } else {
        setError("Browser mode — 4D monitoring requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const s = result?.summary;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[85vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <TrendingUp className="h-4 w-4" style={{ color: colors.miningYellow }} />
            4D Pit Monitoring
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5">
          {demFiles.length < 2 ? (
            <div className="rounded-md border border-navy-border bg-navy-base p-4 text-center text-xs text-steel-gray">
              Drop at least 2 GeoTIFF DEM files (previous + current survey) to compute differences.
            </div>
          ) : (
            <>
              <section className="mb-4">
                <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Previous survey (DEM)
                </label>
                <select
                  value={prevPath}
                  onChange={(e) => setPrevPath(e.target.value)}
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                >
                  <option value="">— Select previous —</option>
                  {demFiles.map((f) => (
                    <option key={f.id} value={f.path}>{f.name}</option>
                  ))}
                </select>
              </section>

              <section className="mb-4">
                <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Current survey (DEM)
                </label>
                <select
                  value={currPath}
                  onChange={(e) => setCurrPath(e.target.value)}
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
                >
                  <option value="">— Select current —</option>
                  {demFiles.filter((f) => f.path !== prevPath).map((f) => (
                    <option key={f.id} value={f.path}>{f.name}</option>
                  ))}
                </select>
              </section>

              <div className="mb-4 grid grid-cols-2 gap-3">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Rock density (t/m³)
                  </label>
                  <input
                    type="number" step="0.1" value={density}
                    onChange={(e) => setDensity(parseFloat(e.target.value) || 2.7)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Hotspot threshold (m)
                  </label>
                  <input
                    type="number" step="0.1" value={hotspotThreshold}
                    onChange={(e) => setHotspotThreshold(parseFloat(e.target.value) || 1.0)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                  />
                </div>
              </div>
            </>
          )}

          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {result && s && (
            <div className="space-y-3">
              <div className="grid grid-cols-3 gap-2">
                <StatTile label="Fill (m³)" value={s.total_fill_volume.toFixed(0)} color={colors.pass} />
                <StatTile label="Cut (m³)" value={s.total_cut_volume.toFixed(0)} color={colors.fail} />
                <StatTile label="Net (m³)" value={s.net_volume.toFixed(0)} color={colors.industrialOrange} />
              </div>
              <div className="grid grid-cols-3 gap-2">
                <StatTile label="Fill (t)" value={s.total_fill_tonnage.toFixed(0)} color={colors.pass} />
                <StatTile label="Cut (t)" value={s.total_cut_tonnage.toFixed(0)} color={colors.fail} />
                <StatTile label="Hotspots" value={result.hotspots.length.toString()} color={colors.investigate} />
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-light">
                <div className="grid grid-cols-2 gap-2">
                  <span>Fill cells: <span className="font-mono text-white">{s.fill_cells.toLocaleString()}</span></span>
                  <span>Cut cells: <span className="font-mono text-white">{s.cut_cells.toLocaleString()}</span></span>
                  <span>Stable: <span className="font-mono text-white">{s.stable_cells.toLocaleString()}</span></span>
                  <span>No-data: <span className="font-mono text-white">{s.nodata_cells.toLocaleString()}</span></span>
                  <span>Max fill: <span className="font-mono" style={{ color: colors.pass }}>{s.max_fill.toFixed(2)}m</span></span>
                  <span>Max cut: <span className="font-mono" style={{ color: colors.fail }}>{s.max_cut.toFixed(2)}m</span></span>
                  <span>Mean Δz: <span className="font-mono text-white">{s.mean_dz.toFixed(3)}m</span></span>
                  <span>RMS Δz: <span className="font-mono text-white">{s.rms_dz.toFixed(3)}m</span></span>
                </div>
              </div>
              {result.hotspots.length > 0 && (
                <div className="flex items-center gap-2 rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.investigate}40`, background: `${colors.investigate}10`, color: colors.investigate }}>
                  <AlertTriangle className="h-3 w-3" />
                  {result.hotspots.length} hotspot cells exceed ±{hotspotThreshold}m threshold
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">Multi-temporal surface differencing</div>
          <button
            onClick={handleCompute}
            disabled={!canCompute || loading}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{ background: canCompute && !loading ? colors.miningYellow : colors.steelGray, color: colors.navyBase }}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Activity className="h-3 w-3" />}
            {loading ? "Computing…" : "Compute diff"}
          </button>
        </div>
      </div>
    </div>
  );
}

function StatTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-sm font-semibold text-white">{value}</div>
    </div>
  );
}
