import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * CSF Classification Dialog — Phase 1.
 *
 * Run Cloth Simulation Filter ground extraction on a loaded LAS point
 * cloud. Tunable params: cloth resolution, classification threshold,
 * max iterations, rigidness.
 *
 * Result shows ground/non-ground counts and can be used to filter the
 * point cloud for DEM generation.
 */

import { useState } from "react";
import { X, Loader2, Layers, Activity } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  classifyGround,
  type CsfParams,
  type CsfResult,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
  /** Called when classification completes — passes the result so the
   * point cloud layer can color points by ground/non-ground. */
  onClassified?: (result: CsfResult) => void;
}

const DEFAULT_PARAMS: CsfParams = {
  cloth_resolution: 0.5,
  classification_threshold: 0.5,
  max_iterations: 500,
  rigidness: 2,
  time_step: 0.65,
  cloth_init_offset: 10.0,
};

export function CsfClassificationDialog({ open, onClose, onClassified }: Props) {
  const files = useSurveyStore((s) => s.files);
  const lasFiles = files.filter((f) => f.kind === "las" && f.status === "loaded");

  const [lasPath, setLasPath] = useState<string>("");
  const [params, setParams] = useState<CsfParams>(DEFAULT_PARAMS);
  const [maxPoints, setMaxPoints] = useState<number>(0); // 0 = unlimited
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<CsfResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  const canRun = lasPath !== "" && !loading;

  async function handleClassify() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const r = await classifyGround(lasPath, params, maxPoints > 0 ? maxPoints : undefined);
      if (r) {
        setResult(r);
        onClassified?.(r);
      } else {
        setError("Browser mode — CSF requires the native Tauri shell");
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
            <Layers className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Point Cloud Classification (CSF)
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
          {/* LAS source */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Source point cloud (LAS)
            </label>
            {lasFiles.length === 0 ? (
              <div className="rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-steel-gray">
                Drop a LAS file on the map first.
              </div>
            ) : (
              <select
                value={lasPath}
                onChange={(e) => setLasPath(e.target.value)}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
              >
                <option value="">— Select LAS —</option>
                {lasFiles.map((f) => (
                  <option key={f.id} value={f.path}>
                    {f.name} {f.pointCount ? `· ${f.pointCount.toLocaleString()} pts` : ""}
                  </option>
                ))}
              </select>
            )}
          </section>

          {/* CSF parameters */}
          <div className="mb-5 grid grid-cols-2 gap-3">
            <ParamInput
              label="Cloth resolution (m)"
              value={params.cloth_resolution}
              step="0.1"
              hint="Grid spacing for cloth particles"
              onChange={(v) => setParams({ ...params, cloth_resolution: v })}
            />
            <ParamInput
              label="Classification threshold (m)"
              value={params.classification_threshold}
              step="0.1"
              hint="Max distance from cloth for ground"
              onChange={(v) => setParams({ ...params, classification_threshold: v })}
            />
            <ParamInput
              label="Max iterations"
              value={params.max_iterations}
              step="50"
              hint="Cap on simulation steps"
              onChange={(v) => setParams({ ...params, max_iterations: v })}
              isInt
            />
            <div>
              <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Terrain rigidness
              </label>
              <select
                value={params.rigidness}
                onChange={(e) =>
                  setParams({ ...params, rigidness: parseInt(e.target.value) })
                }
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
              >
                <option value="1">1 — Gentle terrain</option>
                <option value="2">2 — Sloped (default)</option>
                <option value="3">3 — Cliff / steep</option>
              </select>
              <p className="mt-1 text-[10px] text-steel-gray">Higher = stiffer cloth</p>
            </div>
            <ParamInput
              label="Time step"
              value={params.time_step}
              step="0.05"
              hint="Simulation dt (default 0.65)"
              onChange={(v) => setParams({ ...params, time_step: v })}
            />
            <ParamInput
              label="Max points (0 = all)"
              value={maxPoints}
              step="1000"
              hint="Limit for very large clouds"
              onChange={setMaxPoints}
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
              <div className="grid grid-cols-3 gap-2">
                <ResultTile
                  label="Total"
                  value={result.point_count}
                  color={colors.steelLight}
                />
                <ResultTile
                  label="Ground"
                  value={result.ground_count}
                  color={colors.pass}
                />
                <ResultTile
                  label="Non-ground"
                  value={result.non_ground_count}
                  color={colors.fail}
                />
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-light">
                Iterations: <span className="font-mono">{result.iterations_run}</span>
                {" · "}Cloth grid:{" "}
                <span className="font-mono">
                  {result.cloth_dims[0]}×{result.cloth_dims[1]}
                </span>
                {" · "}Cloth Z range:{" "}
                <span className="font-mono">
                  {result.cloth_z_min.toFixed(2)} – {result.cloth_z_max.toFixed(2)} m
                </span>
              </div>
              {result.point_count > 0 && (
                <div>
                  <div className="mb-1 flex items-center justify-between text-[10px] text-steel-gray">
                    <span>Ground ratio</span>
                    <span className="font-mono">
                      {((result.ground_count / result.point_count) * 100).toFixed(1)}%
                    </span>
                  </div>
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-navy-border">
                    <div
                      className="h-full"
                      style={{
                        width: `${(result.ground_count / result.point_count) * 100}%`,
                        background: colors.pass,
                      }}
                    />
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            Zhang et al. (2016) — pure Rust implementation
          </div>
          <button
            onClick={handleClassify}
            disabled={!canRun}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{
              background: canRun ? colors.industrialOrange : colors.steelGray,
              color: colors.navyBase,
            }}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Activity className="h-3 w-3" />}
            {loading ? "Classifying…" : "Classify"}
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
  value: number;
  color: string;
}) {
  return (
    <div
      className="rounded-md border p-3"
      style={{ borderColor: `${color}40`, background: `${color}10` }}
    >
      <div className="text-[10px] uppercase tracking-wider" style={{ color }}>
        {label}
      </div>
      <div className="mt-1 font-mono text-lg font-semibold text-white">
        {value.toLocaleString()}
      </div>
    </div>
  );
}
