/**
 * Volume Calculator Dialog — Phase 1 Mining MVP.
 *
 * Lets the surveyor pick two GeoTIFF DEMs (current + reference) or a
 * flat reference plane, set a bench interval, and compute fill/cut
 * volumes via the Rust core. Shows the result with bench-by-bench
 * breakdown.
 *
 * Reference can be:
 *   - Another GeoTIFF (previous survey or design surface)
 *   - "flat:Z" — a flat plane at elevation Z (for stockpile volumes
 *     against a known base elevation)
 */

import { useState } from "react";
import { X, Calculator, Loader2, TrendingUp, TrendingDown } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  computeVolumes,
  type VolumeResultRpc,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function VolumeCalcDialog({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const geotiffFiles = files.filter((f) => f.kind === "geotiff" && f.status === "loaded");

  const [currentPath, setCurrentPath] = useState<string>("");
  const [referenceMode, setReferenceMode] = useState<"file" | "flat">("flat");
  const [referencePath, setReferencePath] = useState<string>("");
  const [flatElevation, setFlatElevation] = useState<number>(0);
  const [benchInterval, setBenchInterval] = useState<number>(5);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<VolumeResultRpc | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const canCompute =
    currentPath !== "" &&
    (referenceMode === "flat" || referencePath !== "");

  async function handleCompute() {
    setLoading(true);
    setError(null);
    setResult(null);
    const refPath =
      referenceMode === "flat"
        ? `flat:${flatElevation}`
        : referencePath;
    try {
      const r = await computeVolumes(currentPath, refPath, benchInterval);
      if (r) {
        setResult(r);
      } else {
        setError("Browser mode — volume calc requires the native Tauri shell");
      }
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
        className="flex max-h-[85vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Calculator className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Volume Calculator
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
          {/* Current survey */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Current survey (DEM)
            </label>
            {geotiffFiles.length === 0 ? (
              <div className="rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-steel-gray">
                Drop a GeoTIFF DEM file on the map first.
              </div>
            ) : (
              <select
                value={currentPath}
                onChange={(e) => setCurrentPath(e.target.value)}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
              >
                <option value="">— Select current survey —</option>
                {geotiffFiles.map((f) => (
                  <option key={f.id} value={f.path}>
                    {f.name}
                  </option>
                ))}
              </select>
            )}
          </section>

          {/* Reference surface */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Reference surface
            </label>
            <div className="mb-2 flex gap-2">
              <button
                onClick={() => setReferenceMode("flat")}
                className="flex-1 rounded-md border px-3 py-1.5 text-xs font-medium transition-colors"
                style={{
                  borderColor: referenceMode === "flat" ? colors.industrialOrange : colors.navyBorder,
                  background: referenceMode === "flat" ? `${colors.industrialOrange}15` : colors.navyBase,
                  color: referenceMode === "flat" ? colors.white : colors.steelLight,
                }}
              >
                Flat plane
              </button>
              <button
                onClick={() => setReferenceMode("file")}
                className="flex-1 rounded-md border px-3 py-1.5 text-xs font-medium transition-colors"
                style={{
                  borderColor: referenceMode === "file" ? colors.industrialOrange : colors.navyBorder,
                  background: referenceMode === "file" ? `${colors.industrialOrange}15` : colors.navyBase,
                  color: referenceMode === "file" ? colors.white : colors.steelLight,
                }}
              >
                Previous survey DEM
              </button>
            </div>

            {referenceMode === "flat" ? (
              <div className="flex items-center gap-2">
                <label className="text-xs text-steel-light">Elevation (m):</label>
                <input
                  type="number"
                  step="0.1"
                  value={flatElevation}
                  onChange={(e) => setFlatElevation(parseFloat(e.target.value) || 0)}
                  className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-1.5 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
                />
              </div>
            ) : (
              <select
                value={referencePath}
                onChange={(e) => setReferencePath(e.target.value)}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
              >
                <option value="">— Select reference survey —</option>
                {geotiffFiles
                  .filter((f) => f.path !== currentPath)
                  .map((f) => (
                    <option key={f.id} value={f.path}>
                      {f.name}
                    </option>
                  ))}
              </select>
            )}
          </section>

          {/* Bench interval */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Bench interval (m)
            </label>
            <input
              type="number"
              step="0.5"
              min="0"
              value={benchInterval}
              onChange={(e) => setBenchInterval(parseFloat(e.target.value) || 0)}
              className="w-32 rounded-md border border-navy-border bg-navy-base px-3 py-1.5 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
            />
            <p className="mt-1 text-[10px] text-steel-gray">
              Set to 0 to skip bench-by-bench breakdown.
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
              <div className="grid grid-cols-3 gap-2">
                <ResultTile
                  label="Fill"
                  value={result.fill_volume}
                  unit="m³"
                  icon={<TrendingUp className="h-3.5 w-3.5" />}
                  color={colors.pass}
                />
                <ResultTile
                  label="Cut"
                  value={result.cut_volume}
                  unit="m³"
                  icon={<TrendingDown className="h-3.5 w-3.5" />}
                  color={colors.fail}
                />
                <ResultTile
                  label="Net"
                  value={result.net_volume}
                  unit="m³"
                  icon={<Calculator className="h-3.5 w-3.5" />}
                  color={colors.industrialOrange}
                />
              </div>

              <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-light">
                Cell area: <span className="font-mono">{result.cell_area.toFixed(2)} m²</span>
                {" · "}
                Fill cells: <span className="font-mono">{result.fill_cells.toLocaleString()}</span>
                {" · "}
                Cut cells: <span className="font-mono">{result.cut_cells.toLocaleString()}</span>
              </div>

              {/* Bench breakdown */}
              {result.benches.length > 0 && (
                <div>
                  <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Bench Breakdown
                  </h4>
                  <div className="max-h-48 overflow-y-auto rounded-md border border-navy-border">
                    <table className="w-full text-left text-[10px]">
                      <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                        <tr>
                          <th className="px-2 py-1.5 font-mono">Bench</th>
                          <th className="px-2 py-1.5 text-right">Fill (m³)</th>
                          <th className="px-2 py-1.5 text-right">Cut (m³)</th>
                          <th className="px-2 py-1.5 text-right">Net (m³)</th>
                        </tr>
                      </thead>
                      <tbody>
                        {result.benches.map((b, i) => (
                          <tr key={i} className="border-t border-navy-border">
                            <td className="px-2 py-1.5 font-mono text-steel-light">
                              {b.z_min.toFixed(1)}–{b.z_max.toFixed(1)}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono" style={{ color: colors.pass }}>
                              {b.fill_volume > 0 ? b.fill_volume.toFixed(1) : "—"}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono" style={{ color: colors.fail }}>
                              {b.cut_volume > 0 ? b.cut_volume.toFixed(1) : "—"}
                            </td>
                            <td className="px-2 py-1.5 text-right font-mono text-white">
                              {b.net_volume !== 0 ? b.net_volume.toFixed(1) : "—"}
                            </td>
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
            Phase 1: requires both DEMs to have identical dimensions.
          </div>
          <button
            onClick={handleCompute}
            disabled={!canCompute || loading}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{
              background: canCompute && !loading ? colors.industrialOrange : colors.steelGray,
              color: colors.navyBase,
            }}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Calculator className="h-3 w-3" />}
            {loading ? "Computing…" : "Compute"}
          </button>
        </div>
      </div>
    </div>
  );
}

function ResultTile({
  label,
  value,
  unit,
  icon,
  color,
}: {
  label: string;
  value: number;
  unit: string;
  icon: React.ReactNode;
  color: string;
}) {
  return (
    <div
      className="rounded-md border p-3"
      style={{ borderColor: `${color}40`, background: `${color}10` }}
    >
      <div className="flex items-center gap-1.5 text-[10px] uppercase tracking-wider" style={{ color }}>
        {icon}
        {label}
      </div>
      <div className="mt-1 font-mono text-lg font-semibold text-white">
        {value.toLocaleString(undefined, { maximumFractionDigits: 1 })}
        <span className="ml-1 text-xs font-normal text-steel-gray">{unit}</span>
      </div>
    </div>
  );
}
