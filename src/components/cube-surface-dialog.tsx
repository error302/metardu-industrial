/**
 * CUBE Surface Dialog — Phase 2 Marine MVP.
 *
 * Generate a CUBE bathymetric surface from sounding data and render it
 * as a colored raster overlay on the map (blue depth ramp).
 *
 * Input: CSV soundings (x, y, depth, uncertainty) or synthetic grid.
 * Output: CubeSurface with depth/uncertainty grids → rendered as raster.
 */

import { useState } from "react";
import { X, Waves, Loader2, Database } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  generateCubeSurface,
  type CubeParams,
  type CubeSurfaceRpc,
  type SoundingRpc,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
  /** Called when a CUBE surface is generated — the map will render it. */
  onSurfaceGenerated: (surface: CubeSurfaceRpc) => void;
}

const DEFAULT_PARAMS: CubeParams = {
  resolution: 1.0,
  capture_distance: 0.5,
  init_uncertainty: 0.3,
  max_hypotheses: 5,
  min_soundings: 3,
};

/** Generate synthetic soundings for testing without real .all/.s7k data. */
function generateSyntheticSoundings(): SoundingRpc[] {
  const soundings: SoundingRpc[] = [];
  // 100x100 grid over a 50m × 50m area, depth varies from 8m to 15m
  for (let i = 0; i < 100; i++) {
    for (let j = 0; j < 100; j++) {
      const x = i * 0.5;
      const y = j * 0.5;
      // Gentle slope from 8m (shallow) to 15m (deep) + a "shoal" bump
      const baseDepth = 8 + (x / 50) * 7;
      const shoal = Math.exp(-((x - 25) ** 2 + (y - 25) ** 2) / 50) * 3;
      const noise = (Math.random() - 0.5) * 0.2;
      soundings.push({
        x,
        y,
        depth: baseDepth - shoal + noise,
        uncertainty: 0.15,
      });
    }
  }
  return soundings;
}

export function CubeSurfaceDialog({ open, onClose, onSurfaceGenerated }: Props) {
  const [params, setParams] = useState<CubeParams>(DEFAULT_PARAMS);
  const [useSynthetic, setUseSynthetic] = useState(true);
  const [csvInput, setCsvInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<CubeSurfaceRpc | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  async function handleGenerate() {
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      let soundings: SoundingRpc[];
      if (useSynthetic) {
        soundings = generateSyntheticSoundings();
      } else {
        const lines = csvInput.trim().split("\n");
        soundings = lines
          .map((line) => {
            const parts = line.trim().split(",").map((s) => parseFloat(s.trim()));
            if (parts.length < 3 || parts.some(isNaN)) return null;
            return {
              x: parts[0],
              y: parts[1],
              depth: parts[2],
              uncertainty: parts[3] ?? 0.15,
            };
          })
          .filter((s): s is SoundingRpc => s !== null);
      }

      if (soundings.length < 10) {
        setError("Need at least 10 soundings. Use CSV: x,y,depth[,uncertainty]");
        setLoading(false);
        return;
      }

      const r = await generateCubeSurface(soundings, params);
      if (r) {
        setResult(r);
        onSurfaceGenerated(r);
      } else {
        setError("Browser mode — CUBE requires the native Tauri shell");
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
            <Waves className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            CUBE Surface Generation
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
          {/* Sounding source */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Sounding source
            </label>
            <div className="mb-2 flex gap-2">
              <button
                onClick={() => setUseSynthetic(true)}
                className="flex-1 rounded-md border px-3 py-1.5 text-xs font-medium transition-colors"
                style={{
                  borderColor: useSynthetic ? colors.marineTurquoise : colors.navyBorder,
                  background: useSynthetic ? `${colors.marineTurquoise}15` : colors.navyBase,
                  color: useSynthetic ? colors.white : colors.steelLight,
                }}
              >
                Synthetic data (10000 pts)
              </button>
              <button
                onClick={() => setUseSynthetic(false)}
                className="flex-1 rounded-md border px-3 py-1.5 text-xs font-medium transition-colors"
                style={{
                  borderColor: !useSynthetic ? colors.marineTurquoise : colors.navyBorder,
                  background: !useSynthetic ? `${colors.marineTurquoise}15` : colors.navyBase,
                  color: !useSynthetic ? colors.white : colors.steelLight,
                }}
              >
                CSV input
              </button>
            </div>
            {!useSynthetic && (
              <textarea
                value={csvInput}
                onChange={(e) => setCsvInput(e.target.value)}
                rows={4}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none"
                placeholder="x,y,depth,uncertainty&#10;0,0,10.0,0.15&#10;0.5,0,10.1,0.15"
              />
            )}
          </section>

          {/* CUBE params */}
          <div className="mb-5 grid grid-cols-2 gap-3">
            <ParamInput
              label="Grid resolution (m)"
              value={params.resolution}
              step="0.5"
              hint="Cell size for output grid"
              onChange={(v) => setParams({ ...params, resolution: v })}
            />
            <ParamInput
              label="Capture distance (m)"
              value={params.capture_distance}
              step="0.1"
              hint="Max distance for hypothesis merge"
              onChange={(v) => setParams({ ...params, capture_distance: v })}
            />
            <ParamInput
              label="Init uncertainty (m)"
              value={params.init_uncertainty}
              step="0.05"
              hint="Starting sigma per hypothesis"
              onChange={(v) => setParams({ ...params, init_uncertainty: v })}
            />
            <ParamInput
              label="Max hypotheses"
              value={params.max_hypotheses}
              step="1"
              hint="Per cell before pruning"
              onChange={(v) => setParams({ ...params, max_hypotheses: v })}
              isInt
            />
            <ParamInput
              label="Min soundings"
              value={params.min_soundings}
              step="1"
              hint="Per cell for inclusion"
              onChange={(v) => setParams({ ...params, min_soundings: v })}
              isInt
            />
          </div>

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
            <div className="space-y-3">
              <div className="grid grid-cols-4 gap-2">
                <ResultTile label="Total pts" value={result.total_soundings.toLocaleString()} color={colors.steelLight} />
                <ResultTile label="Valid cells" value={result.valid_cells.toLocaleString()} color={colors.pass} />
                <ResultTile label="Ambiguous" value={result.ambiguous_cells.toLocaleString()} color={colors.investigate} />
                <ResultTile
                  label="Grid"
                  value={`${result.dims[0]}×${result.dims[1]}`}
                  color={colors.marineCyan}
                />
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-light">
                <span className="flex items-center gap-1.5">
                  <Database className="h-3 w-3" style={{ color: colors.marineTurquoise }} />
                  Surface rendered on map · blue depth ramp
                </span>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            Calder &amp; Mayer (2003) — Bayesian hypothesis tracking
          </div>
          <button
            onClick={handleGenerate}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{
              background: loading ? colors.steelGray : colors.marineTurquoise,
              color: colors.navyBase,
            }}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Waves className="h-3 w-3" />}
            {loading ? "Generating…" : "Generate surface"}
          </button>
        </div>
      </div>
    </div>
  );
}

function ParamInput({
  label,
  value,
  step,
  hint,
  onChange,
  isInt = false,
}: {
  label: string;
  value: number;
  step: string;
  hint: string;
  onChange: (v: number) => void;
  isInt?: boolean;
}) {
  return (
    <div>
      <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
        {label}
      </label>
      <input
        type="number"
        step={step}
        value={value}
        onChange={(e) => {
          const v = isInt ? parseInt(e.target.value) || 0 : parseFloat(e.target.value) || 0;
          onChange(v);
        }}
        className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
      />
      <p className="mt-1 text-[10px] text-steel-gray">{hint}</p>
    </div>
  );
}

function ResultTile({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div
      className="rounded-md border p-2.5"
      style={{ borderColor: `${color}40`, background: `${color}10` }}
    >
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>
        {label}
      </div>
      <div className="mt-0.5 font-mono text-sm font-semibold text-white">{value}</div>
    </div>
  );
}
