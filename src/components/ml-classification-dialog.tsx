/**
 * ML Classification Dialog — Phase 3.
 *
 * Two tools: seafloor habitat classification from backscatter features,
 * and blast fragmentation analysis from particle size data.
 */

import { useState } from "react";
import { X, Brain, Loader2, Waves, Bomb } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  analyzeFragmentation,
  classifyHabitat,
  type FragmentationResult,
  type HabitatClassificationResult,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Tab = "habitat" | "fragmentation";

export function MlClassificationDialog({ open, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("habitat");

  if (!open) return null;

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
            <Brain className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            ML Classification
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Tab switcher */}
        <div className="flex border-b border-navy-border px-5 pt-3">
          <TabButton active={tab === "habitat"} onClick={() => setTab("habitat")} icon={<Waves className="h-3 w-3" />} label="Seafloor Habitat" accent={colors.marineTurquoise} />
          <TabButton active={tab === "fragmentation"} onClick={() => setTab("fragmentation")} icon={<Bomb className="h-3 w-3" />} label="Blast Fragmentation" accent={colors.miningYellow} />
        </div>

        <div className="flex-1 overflow-y-auto p-5">
          {tab === "habitat" ? <HabitatTab /> : <FragmentationTab />}
        </div>
      </div>
    </div>
  );
}

function TabButton({ active, onClick, icon, label, accent }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string; accent: string }) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 border-b-2 px-3 py-2 text-xs font-medium transition-colors ${
        active ? "text-white" : "text-steel-gray hover:text-steel-light"
      }`}
      style={active ? { borderColor: accent } : { borderColor: "transparent" }}
    >
      {icon}
      {label}
    </button>
  );
}

function HabitatTab() {
  const [meanIntensity, setMeanIntensity] = useState(-15.0);
  const [stdIntensity, setStdIntensity] = useState(3.0);
  const [angularSlope, setAngularSlope] = useState(0.3);
  const [textureHomogeneity, setTextureHomogeneity] = useState(0.5);
  const [depth, setDepth] = useState(20.0);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<HabitatClassificationResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleClassify() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const r = await classifyHabitat({
        mean_intensity: meanIntensity,
        std_intensity: stdIntensity,
        angular_slope: angularSlope,
        angular_curvature: 0.01,
        texture_homogeneity: textureHomogeneity,
        depth,
      });
      if (r) setResult(r);
      else setError("Browser mode — ML requires the native Tauri shell");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const classLabels: Record<string, string> = {
    rock: "Rock / Hard substrate",
    coarse_sediment: "Coarse sediment (gravel)",
    sand: "Sand",
    mud: "Mud / Fine sediment",
    mixed: "Mixed substrate",
  };

  return (
    <div>
      <p className="mb-4 text-[11px] text-steel-gray">Enter backscatter features extracted from multibeam data to classify the seafloor habitat.</p>
      <div className="mb-4 grid grid-cols-2 gap-3">
        <NumInput label="Mean intensity (dB)" value={meanIntensity} step="0.5" onChange={setMeanIntensity} />
        <NumInput label="Std intensity (dB)" value={stdIntensity} step="0.1" onChange={setStdIntensity} />
        <NumInput label="Angular slope (dB/°)" value={angularSlope} step="0.01" onChange={setAngularSlope} />
        <NumInput label="Texture homogeneity" value={textureHomogeneity} step="0.05" onChange={setTextureHomogeneity} />
        <NumInput label="Depth (m)" value={depth} step="0.5" onChange={setDepth} />
      </div>

      {error && <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>{error}</div>}

      {result && (
        <div className="space-y-3">
          <div className="flex items-center gap-3 rounded-md border p-4" style={{ borderColor: `${colors.marineTurquoise}40`, background: `${colors.marineTurquoise}10` }}>
            <div className="flex-1">
              <div className="text-sm font-semibold text-white">{classLabels[result.class] || result.class}</div>
              <div className="text-xs text-steel-light">Confidence: <span className="font-mono">{(result.confidence * 100).toFixed(1)}%</span></div>
            </div>
            <div className="font-mono text-2xl font-bold" style={{ color: colors.marineTurquoise }}>
              {(result.confidence * 100).toFixed(0)}%
            </div>
          </div>
          <div>
            <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Class Probabilities</h4>
            {(["rock", "coarse_sediment", "sand", "mud", "mixed"] as const).map((cls, i) => (
              <div key={cls} className="mb-1 flex items-center gap-2 text-[10px]">
                <span className="w-32 text-steel-light">{classLabels[cls]}</span>
                <div className="h-2 flex-1 overflow-hidden rounded-full bg-navy-border">
                  <div className="h-full" style={{ width: `${result.class_probabilities[i] * 100}%`, background: colors.marineTurquoise }} />
                </div>
                <span className="w-10 text-right font-mono text-steel-gray">{(result.class_probabilities[i] * 100).toFixed(1)}%</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="mt-4 flex justify-end">
        <button onClick={handleClassify} disabled={loading} className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40" style={{ background: loading ? colors.steelGray : colors.marineTurquoise, color: colors.navyBase }}>
          {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Brain className="h-3 w-3" />}
          {loading ? "Classifying…" : "Classify"}
        </button>
      </div>
    </div>
  );
}

function FragmentationTab() {
  const [csvInput, setCsvInput] = useState(
    Array.from({ length: 100 }, (_, i) => 50 + (i % 150)).join("\n"),
  );
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<FragmentationResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleAnalyze() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const sizes = csvInput.trim().split("\n").map((s) => parseFloat(s.trim())).filter((n) => !isNaN(n));
      if (sizes.length < 10) {
        setError("Need at least 10 fragment sizes. One per line, in mm.");
        setLoading(false);
        return;
      }
      const r = await analyzeFragmentation(sizes);
      if (r) setResult(r);
      else setError("Browser mode — ML requires the native Tauri shell");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const qualityColors: Record<string, string> = {
    excellent: colors.pass,
    acceptable: colors.info,
    coarse: colors.investigate,
    very_coarse: colors.fail,
  };
  const qualityLabels: Record<string, string> = {
    excellent: "Excellent (P80 < 300mm)",
    acceptable: "Acceptable (P80 300-500mm)",
    coarse: "Coarse (P80 500-800mm)",
    very_coarse: "Very Coarse (P80 > 800mm) — adjust blast",
  };

  return (
    <div>
      <p className="mb-4 text-[11px] text-steel-gray">Enter fragment sizes (mm) extracted from drone imagery of the muck pile. One per line.</p>
      <textarea
        value={csvInput}
        onChange={(e) => setCsvInput(e.target.value)}
        rows={6}
        className="mb-4 w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-industrial-orange focus:outline-none"
      />

      {error && <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>{error}</div>}

      {result && (
        <div className="space-y-3">
          <div className="flex items-center gap-3 rounded-md border p-4" style={{ borderColor: `${qualityColors[result.quality]}40`, background: `${qualityColors[result.quality]}10` }}>
            <div className="flex-1">
              <div className="text-sm font-semibold text-white">{qualityLabels[result.quality]}</div>
              <div className="text-xs text-steel-light">Mean size: <span className="font-mono">{result.mean_size.toFixed(1)}mm</span></div>
            </div>
          </div>
          <div className="grid grid-cols-4 gap-2">
            <FragTile label="P20" value={`${result.p20.toFixed(0)}mm`} />
            <FragTile label="P50" value={`${result.p50.toFixed(0)}mm`} />
            <FragTile label="P80" value={`${result.p80.toFixed(0)}mm`} />
            <FragTile label="P90" value={`${result.p90.toFixed(0)}mm`} />
          </div>
          <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-light">
            Uniformity coefficient (P60/P10): <span className="font-mono text-white">{result.uniformity.toFixed(2)}</span>
          </div>
        </div>
      )}

      <div className="mt-4 flex justify-end">
        <button onClick={handleAnalyze} disabled={loading} className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40" style={{ background: loading ? colors.steelGray : colors.miningYellow, color: colors.navyBase }}>
          {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Bomb className="h-3 w-3" />}
          {loading ? "Analyzing…" : "Analyze"}
        </button>
      </div>
    </div>
  );
}

function NumInput({ label, value, step, onChange }: { label: string; value: number; step: string; onChange: (v: number) => void }) {
  return (
    <div>
      <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">{label}</label>
      <input
        type="number" step={step} value={value}
        onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
        className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
      />
    </div>
  );
}

function FragTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-2.5">
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="mt-0.5 font-mono text-sm font-semibold text-white">{value}</div>
    </div>
  );
}
