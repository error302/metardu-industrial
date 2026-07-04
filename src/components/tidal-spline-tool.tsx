/**
 * Tidal Spline Interpolator — drop sonar + tide CSV → corrected depths.
 *
 * Single screen. No wizard. Two file inputs, one button, one result.
 */

import { useState } from "react";
import {
  X, Loader2, Waves, CheckCircle2, FileText, Download,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  runTidalCorrection,
  type TidalCorrectionResult,
} from "@/lib/tauri-ipc";
import { pickFile, pickSaveFile } from "@/lib/file-picker";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function TidalSplineTool({ open, onClose }: Props) {
  const [sonarPath, setSonarPath] = useState("");
  const [tidePath, setTidePath] = useState("");
  const [outputPath, setOutputPath] = useState("/tmp/corrected_depths.csv");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<TidalCorrectionResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  async function handleRun() {
    if (!sonarPath || !tidePath) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await runTidalCorrection({
        sonar_csv_path: sonarPath,
        tide_csv_path: tidePath,
        output_csv_path: outputPath,
      });
      if (r) {
        setResult(r);
      } else {
        setError("Browser mode — tidal correction requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Waves className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            Tidal Spline Interpolator
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Sonar CSV input */}
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Sonar depths CSV (timestamp, raw_depth_m)
            </label>
            <div className="flex items-center gap-2">
              <button
                onClick={async () => { const p = await pickFile({ extensions: ["csv"], filterName: "CSV", title: "Select sonar CSV" }); if (p) setSonarPath(p); }}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
              >
                <FileText className="h-3.5 w-3.5" /> Browse
              </button>
              <input
                type="text" value={sonarPath} onChange={(e) => setSonarPath(e.target.value)}
                placeholder="Or type a path…"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
              />
            </div>
            <p className="mt-1 text-[10px] text-steel-gray">Format: timestamp_unix_secs, raw_depth_m</p>
          </div>

          {/* Tide CSV input */}
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Tide gauge CSV (timestamp, tide_level_m)
            </label>
            <div className="flex items-center gap-2">
              <button
                onClick={async () => { const p = await pickFile({ extensions: ["csv"], filterName: "CSV", title: "Select tide CSV" }); if (p) setTidePath(p); }}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
              >
                <FileText className="h-3.5 w-3.5" /> Browse
              </button>
              <input
                type="text" value={tidePath} onChange={(e) => setTidePath(e.target.value)}
                placeholder="Or type a path…"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
              />
            </div>
            <p className="mt-1 text-[10px] text-steel-gray">Format: timestamp_unix_secs, tide_level_m (minimum 4 readings)</p>
          </div>

          {/* Output path */}
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Output corrected CSV path
            </label>
            <div className="flex items-center gap-2">
              <button
                onClick={async () => { const p = await pickSaveFile({ extensions: ["csv"], filterName: "CSV", title: "Save corrected CSV" }); if (p) setOutputPath(p); }}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
              >
                <Download className="h-3.5 w-3.5" /> Save As
              </button>
              <input
                type="text" value={outputPath} onChange={(e) => setOutputPath(e.target.value)}
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
              />
            </div>
          </div>

          {/* Run button */}
          <button
            onClick={handleRun}
            disabled={!sonarPath || !tidePath || running}
            className="flex items-center gap-2 rounded-md px-5 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
            style={{ background: colors.marineTurquoise, color: colors.navyBase }}
          >
            {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Waves className="h-4 w-4" />}
            {running ? "Correcting…" : "Correct Depths"}
          </button>

          {/* Results */}
          {result && (
            <div className="space-y-3">
              <div className="rounded-md border p-3"
                style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10` }}>
                <div className="flex items-center gap-2 mb-2">
                  <CheckCircle2 className="h-4 w-4" style={{ color: colors.pass }} />
                  <span className="text-sm font-bold text-white">Correction Complete</span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <Stat label="Pings Corrected" value={result.pings_corrected.toLocaleString()} />
                  <Stat label="Tide Readings" value={result.tide_readings.toLocaleString()} />
                  <Stat label="Tide Range" value={`${result.min_tide_m.toFixed(2)} – ${result.max_tide_m.toFixed(2)} m`} />
                  <Stat label="Mean Tide" value={`${result.mean_tide_m.toFixed(3)} m`} />
                  <Stat label="Corrected Depth Range" value={`${result.min_corrected_depth_m.toFixed(2)} – ${result.max_corrected_depth_m.toFixed(2)} m`} />
                  <Stat label="Output File" value={result.output_path.split(/[\\/]/).pop() ?? result.output_path} />
                </div>
              </div>

              {result.warnings.length > 0 && (
                <div className="rounded-md border p-2 text-[10px]"
                  style={{ borderColor: "#F59E0B40", background: "#F59E0B10", color: "#F59E0B" }}>
                  {result.warnings.map((w, i) => <div key={i}>⚠ {w}</div>)}
                </div>
              )}

              <div className="text-[10px] text-steel-gray">
                Output CSV format: timestamp_unix_secs, raw_depth_m, tide_level_m, corrected_depth_m
              </div>
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <span>Cubic spline interpolation — deterministic, no AI. Eliminates Excel tide-matching.</span>
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

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="font-mono text-white">{value}</div>
    </div>
  );
}
