/**
 * ODM Pipeline Dialog — Phase 1.
 *
 * Lets the surveyor run OpenDroneMap (via Docker) on a directory of
 * drone images to produce a classified point cloud. Streams progress
 * via Tauri events.
 *
 * Workflow:
 *   1. Pick an images directory (the user already dropped a manifest
 *      OR types a path)
 *   2. Check Docker + ODM image availability
 *   3. Configure: max concurrency, feature quality, skip 3D model
 *   4. Click Run → progress streams, log tail visible
 *   5. On completion, the resulting LAS path is added to the survey store
 */

import { useEffect, useState } from "react";
import { X, Play, Loader2, CheckCircle2, AlertCircle, Terminal } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { colors } from "@/lib/tokens";
import {
  checkOdmAvailability,
  runOdmPipeline,
  type OdmCheckResult,
  type OdmConfig,
  type OdmRunStatus,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

const DEFAULT_CONFIG: OdmConfig = {
  image: "opendronemap/odm:latest",
  images_dir: "",
  output_dir: null,
  max_concurrency: 4,
  feature_quality: "high",
  skip_3dmodel: true,
  pc_type: "las",
};

export function OdmPipelineDialog({ open, onClose }: Props) {
  const [config, setConfig] = useState<OdmConfig>(DEFAULT_CONFIG);
  const [check, setCheck] = useState<OdmCheckResult | null>(null);
  const [checking, setChecking] = useState(false);
  const [status, setStatus] = useState<OdmRunStatus | null>(null);
  const [running, setRunning] = useState(false);
  const [logLines, setLogLines] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const addFileFromPath = useSurveyStore((s) => s.addFileFromPath);

  // Subscribe to ODM progress events
  useEffect(() => {
    if (!open) return;
    const unlisten = listen<OdmRunStatus>("odm://progress", (event) => {
      setStatus(event.payload);
      if (event.payload.last_log_line) {
        setLogLines((prev) =>
          [...prev, event.payload.last_log_line].slice(-200),
        );
      }
      if (!event.payload.running) {
        setRunning(false);
        if (event.payload.error) {
          setError(event.payload.error);
        } else if (event.payload.output_las_path) {
          // Auto-add the resulting LAS to the survey store
          addFileFromPath(event.payload.output_las_path, 0);
        }
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [open, addFileFromPath]);

  // Check Docker + ODM on dialog open
  useEffect(() => {
    if (!open) return;
    setChecking(true);
    setError(null);
    checkOdmAvailability(config.image)
      .then((r) => {
        setCheck(r);
        setChecking(false);
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setChecking(false);
      });
  }, [open, config.image]);

  if (!open) return null;

  const canRun =
    check?.docker_available &&
    check?.image_pulled &&
    config.images_dir !== "" &&
    !running;

  function handleRun() {
    setRunning(true);
    setError(null);
    setLogLines([]);
    runOdmPipeline(config).catch((e: unknown) => {
      setError(e instanceof Error ? e.message : String(e));
      setRunning(false);
    });
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
            <Terminal className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            ODM Pipeline Runner
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
          {/* Docker check */}
          <section className="mb-5">
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Environment
            </h3>
            <div className="flex items-center gap-3 rounded-md border border-navy-border bg-navy-base p-3">
              {checking ? (
                <Loader2 className="h-4 w-4 animate-spin" style={{ color: colors.industrialOrange }} />
              ) : check?.docker_available && check.image_pulled ? (
                <CheckCircle2 className="h-4 w-4" style={{ color: colors.pass }} />
              ) : (
                <AlertCircle className="h-4 w-4" style={{ color: colors.fail }} />
              )}
              <div className="flex-1 text-xs">
                {checking ? (
                  <span className="text-steel-light">Checking Docker…</span>
                ) : check?.docker_available && check.image_pulled ? (
                  <span style={{ color: colors.pass }}>
                    Docker ready · image <span className="font-mono">{check.image_name}</span> available
                  </span>
                ) : check?.docker_available && !check.image_pulled ? (
                  <span style={{ color: colors.investigate }}>
                    Docker available but image not pulled. Run:
                    <code className="ml-2 rounded bg-navy-elevated px-1.5 py-0.5 font-mono text-[10px]">
                      docker pull {check.image_name}
                    </code>
                  </span>
                ) : (
                  <span style={{ color: colors.fail }}>
                    Docker not found. Install Docker Desktop or docker engine.
                  </span>
                )}
              </div>
            </div>
          </section>

          {/* Images dir */}
          <section className="mb-5">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Images directory
            </label>
            <input
              type="text"
              value={config.images_dir}
              onChange={(e) => setConfig({ ...config, images_dir: e.target.value })}
              placeholder="/path/to/drone/images"
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
            />
            <p className="mt-1 text-[10px] text-steel-gray">
              Directory containing JPEG/TIFF images from the drone flight.
            </p>
          </section>

          {/* ODM options */}
          <div className="mb-5 grid grid-cols-2 gap-3">
            <section>
              <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Max concurrency
              </label>
              <input
                type="number"
                min="1"
                max="32"
                value={config.max_concurrency}
                onChange={(e) =>
                  setConfig({ ...config, max_concurrency: parseInt(e.target.value) || 4 })
                }
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
              />
            </section>
            <section>
              <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Feature quality
              </label>
              <select
                value={config.feature_quality}
                onChange={(e) => setConfig({ ...config, feature_quality: e.target.value })}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
              >
                <option value="ultra">Ultra (slowest, best)</option>
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
                <option value="lowest">Lowest (fastest)</option>
              </select>
            </section>
            <section>
              <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Output format
              </label>
              <select
                value={config.pc_type}
                onChange={(e) => setConfig({ ...config, pc_type: e.target.value })}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none"
              >
                <option value="las">LAS</option>
                <option value="laz">LAZ (compressed)</option>
                <option value="ply">PLY</option>
                <option value="csv">CSV</option>
              </select>
            </section>
            <section>
              <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Skip 3D model
              </label>
              <label className="flex h-[38px] cursor-pointer items-center gap-2 rounded-md border border-navy-border bg-navy-base px-3">
                <input
                  type="checkbox"
                  checked={config.skip_3dmodel}
                  onChange={(e) => setConfig({ ...config, skip_3dmodel: e.target.checked })}
                  className="h-4 w-4"
                  style={{ accentColor: colors.industrialOrange }}
                />
                <span className="text-xs text-steel-light">Skip (faster)</span>
              </label>
            </section>
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

          {/* Progress + log */}
          {status && (
            <div className="mb-4">
              <div className="mb-2 flex items-center justify-between">
                <span
                  className="text-[10px] font-semibold uppercase tracking-wider"
                  style={{
                    color: status.phase === "error" ? colors.fail :
                           status.phase === "complete" ? colors.pass :
                           colors.industrialOrange,
                  }}
                >
                  {status.phase}
                </span>
                {status.output_las_path && (
                  <span className="font-mono text-[10px]" style={{ color: colors.pass }}>
                    ✓ {status.output_las_path.split(/[\\/]/).pop()}
                  </span>
                )}
              </div>
              <div className="h-1 w-full overflow-hidden rounded-full bg-navy-border">
                <div
                  className="h-full transition-all duration-300"
                  style={{
                    width: status.phase === "complete" ? "100%" : "60%",
                    background: status.phase === "error" ? colors.fail :
                                status.phase === "complete" ? colors.pass :
                                colors.industrialOrange,
                  }}
                />
              </div>
              {logLines.length > 0 && (
                <div className="mt-2 max-h-32 overflow-y-auto rounded-md border border-navy-border bg-black/50 p-2 font-mono text-[10px] leading-relaxed text-steel-light">
                  {logLines.slice(-20).map((line, i) => (
                    <div key={i}>{line}</div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            Requires Docker + opendronemap/odm image.
          </div>
          <button
            onClick={handleRun}
            disabled={!canRun}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{
              background: canRun ? colors.industrialOrange : colors.steelGray,
              color: colors.navyBase,
            }}
          >
            {running ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
            {running ? "Running…" : "Run pipeline"}
          </button>
        </div>
      </div>
    </div>
  );
}
