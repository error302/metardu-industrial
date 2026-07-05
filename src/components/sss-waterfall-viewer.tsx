/**
 * SSS Waterfall Viewer — Sprint 6 Priority #8.
 *
 * Custom Canvas2D scrolling waterfall for side-scan sonar data:
 *   - X axis = across-track samples (port on left, starboard on right)
 *   - Y axis = ping index (scrolls as new pings arrive)
 *   - Pixel intensity = backscatter amplitude (log-scaled)
 *
 * Workflow:
 *   1. Import XTF file → pings load in Rust
 *   2. Waterfall renders in Canvas2D, auto-scrolling
 *   3. Click target on waterfall → click shadow end → height computed
 *   4. Save measurement as georeferenced POI
 *
 * Per ROADMAP.md Priority #8.
 */

import { useState, useRef, useEffect, useCallback } from "react";
import {
  X, Loader2, Play, Pause, Waves, Download, Crosshair, Ruler, Save,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  readSssPings,
  computeTargetHeight,
  type SssData,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

interface ClickPoint {
  x: number;       // canvas pixel coords
  y: number;
  pingIdx: number; // ping index in the data
  sampleIdx: number; // sample index within the ping
  channel: "port" | "starboard";
}

const WATERFALL_WIDTH = 800;
const WATERFALL_HEIGHT = 500;
const PING_HEIGHT_PX = 2; // each ping row is 2px tall

export function SssWaterfallViewer({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);
  const xtfFiles = files.filter((f) => f.name.toLowerCase().endsWith(".xtf"));

  const [data, setData] = useState<SssData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [scrollOffset, setScrollOffset] = useState(0); // pings from the bottom
  const [gain, setGain] = useState(1.0); // brightness multiplier
  const [measuring, setMeasuring] = useState(false);
  const [targetClick, setTargetClick] = useState<ClickPoint | null>(null);
  const [shadowClick, setShadowClick] = useState<ClickPoint | null>(null);
  const [targetHeightM, setTargetHeightM] = useState<number | null>(null);
  const [poiList, setPoiList] = useState<string[]>([]);

  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Load XTF file
  const loadXtf = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    setData(null);
    try {
      const result = await readSssPings({ path, maxPings: 1000 });
      if (result) {
        setData(result);
        setScrollOffset(0);
        setTargetClick(null);
        setShadowClick(null);
        setTargetHeightM(null);
      } else {
        setError("Browser mode — XTF parsing requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  // Render the waterfall to canvas
  useEffect(() => {
    if (!data || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Clear
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, WATERFALL_WIDTH, WATERFALL_HEIGHT);

    // Determine visible ping range based on scrollOffset
    const maxVisiblePings = Math.floor(WATERFALL_HEIGHT / PING_HEIGHT_PX);
    const endPing = Math.max(0, data.pings.length - scrollOffset);
    const startPing = Math.max(0, endPing - maxVisiblePings);

    // Render each ping as a horizontal strip
    for (let p = startPing; p < endPing; p++) {
      const ping = data.pings[p];
      const yPx = (p - startPing) * PING_HEIGHT_PX;

      // Port side: rendered on left half (reversed so nadir is at center)
      const portHalfWidth = WATERFALL_WIDTH / 2;
      const portLen = ping.port_samples.length;
      for (let s = 0; s < portLen && s < portHalfWidth; s++) {
        const intensity = applyGain(ping.port_samples[portLen - 1 - s], gain);
        ctx.fillStyle = `rgb(${intensity},${intensity},${intensity})`;
        ctx.fillRect(portHalfWidth - s - 1, yPx, 1, PING_HEIGHT_PX);
      }

      // Starboard side: rendered on right half
      const starLen = ping.starboard_samples.length;
      for (let s = 0; s < starLen && s < portHalfWidth; s++) {
        const intensity = applyGain(ping.starboard_samples[s], gain);
        ctx.fillStyle = `rgb(${intensity},${intensity},${intensity})`;
        ctx.fillRect(portHalfWidth + s, yPx, 1, PING_HEIGHT_PX);
      }
    }

    // Draw nadir line (center)
    ctx.strokeStyle = "rgba(255, 165, 0, 0.4)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(WATERFALL_WIDTH / 2, 0);
    ctx.lineTo(WATERFALL_WIDTH / 2, WATERFALL_HEIGHT);
    ctx.stroke();

    // Draw measurement markers
    if (targetClick) {
      ctx.strokeStyle = "#10B981";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(targetClick.x, targetClick.y, 6, 0, 2 * Math.PI);
      ctx.stroke();
      ctx.fillStyle = "#10B981";
      ctx.font = "10px monospace";
      ctx.fillText("T", targetClick.x - 3, targetClick.y - 8);
    }
    if (shadowClick) {
      ctx.strokeStyle = "#DC2626";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(shadowClick.x, shadowClick.y, 6, 0, 2 * Math.PI);
      ctx.stroke();
      ctx.fillStyle = "#DC2626";
      ctx.font = "10px monospace";
      ctx.fillText("S", shadowClick.x - 3, shadowClick.y - 8);

      // Draw line between target and shadow
      if (targetClick) {
        ctx.strokeStyle = "rgba(255, 255, 0, 0.6)";
        ctx.setLineDash([4, 2]);
        ctx.beginPath();
        ctx.moveTo(targetClick.x, targetClick.y);
        ctx.lineTo(shadowClick.x, shadowClick.y);
        ctx.stroke();
        ctx.setLineDash([]);
      }
    }
  }, [data, scrollOffset, gain, targetClick, shadowClick]);

  // Auto-scroll: increment scrollOffset (no — actually we want to show
  // the latest pings, so scrollOffset = 0 means "show most recent").
  // For pre-loaded XTF data we just show from the end.
  useEffect(() => {
    if (!autoScroll || !data) return;
    setScrollOffset(0);
  }, [autoScroll, data]);

  // Compute the height of a target above the seafloor from a
  // target/shadow click pair. Wrapped in useCallback so the
  // handleCanvasClick closure below always has a fresh reference
  // (otherwise it would capture a stale `data` and compute against
  // the wrong ping).
  const computeHeight = useCallback(async (target: ClickPoint, shadow: ClickPoint) => {
    if (!data) return;
    const ping = data.pings[target.pingIdx];
    if (!ping) return;

    // Sample index → slant range (meters)
    // slant_range = sample_index × sound_speed × sample_interval / 2
    const slantRange = target.sampleIdx * ping.sound_speed_mps * ping.sample_interval_secs / 2;
    // Shadow length: difference in sample indices × same conversion
    const shadowSamples = Math.abs(shadow.sampleIdx - target.sampleIdx);
    const shadowLengthM = shadowSamples * ping.sound_speed_mps * ping.sample_interval_secs / 2;

    const height = await computeTargetHeight({
      fishAltitudeM: ping.altitude_m,
      slantRangeToTargetM: slantRange,
      shadowLengthM,
    });
    setTargetHeightM(height);
  }, [data]);

  // Handle canvas click for measurement
  const handleCanvasClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!data) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Convert canvas Y to ping index
    const maxVisiblePings = Math.floor(WATERFALL_HEIGHT / PING_HEIGHT_PX);
    const endPing = Math.max(0, data.pings.length - scrollOffset);
    const startPing = Math.max(0, endPing - maxVisiblePings);
    const pingIdx = startPing + Math.floor(y / PING_HEIGHT_PX);
    if (pingIdx < 0 || pingIdx >= data.pings.length) return;

    // Convert canvas X to sample index (port on left reversed, starboard on right)
    const portHalfWidth = WATERFALL_WIDTH / 2;
    const channel: "port" | "starboard" = x < portHalfWidth ? "port" : "starboard";
    const sampleIdx = channel === "port"
      ? Math.floor(portHalfWidth - x)
      : Math.floor(x - portHalfWidth);

    const click: ClickPoint = { x, y, pingIdx, sampleIdx, channel };

    if (measuring) {
      if (!targetClick) {
        setTargetClick(click);
      } else if (!shadowClick) {
        setShadowClick(click);
        // Compute target height
        void computeHeight(targetClick, click);
      } else {
        // Start new measurement
        setTargetClick(click);
        setShadowClick(null);
        setTargetHeightM(null);
      }
    }
  }, [data, scrollOffset, measuring, targetClick, shadowClick, computeHeight]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[92vh] w-full max-w-5xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Waves className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            SSS Waterfall Viewer
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* File picker */}
          {!data && (
            <div className="space-y-3">
              <label className="block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                Select XTF (side-scan sonar) file
              </label>
              {xtfFiles.length === 0 ? (
                <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-gray">
                  Drop an XTF file on the map first. XTF is the standard format from
                  EdgeTech, Klein, Marine Sonic, and other side-scan sonars.
                </div>
              ) : (
                <div className="grid gap-2">
                  {xtfFiles.map((f) => (
                    <button
                      key={f.id}
                      onClick={() => loadXtf(f.path)}
                      disabled={loading}
                      className="flex items-center justify-between rounded-md border border-navy-border bg-navy-base px-3 py-2 text-left text-xs text-white hover:bg-navy-elevated disabled:opacity-40"
                    >
                      <span>{f.name}</span>
                      <span className="font-mono text-steel-gray">{(f.size / 1024 / 1024).toFixed(1)} MB</span>
                    </button>
                  ))}
                </div>
              )}
              {loading && (
                <div className="flex items-center gap-2 text-xs text-steel-light">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" /> Loading XTF pings…
                </div>
              )}
            </div>
          )}

          {/* Waterfall */}
          {data && (
            <div className="space-y-3">
              {/* Controls */}
              <div className="flex items-center gap-3 rounded-md border border-navy-border bg-navy-base p-2 text-xs">
                <button
                  onClick={() => setAutoScroll((v) => !v)}
                  className="flex items-center gap-1 rounded px-2 py-1 text-white"
                  style={{ background: autoScroll ? colors.pass : colors.steelGray }}
                >
                  {autoScroll ? <Pause className="h-3 w-3" /> : <Play className="h-3 w-3" />}
                  {autoScroll ? "Auto" : "Manual"}
                </button>
                <label className="flex items-center gap-1 text-steel-light">
                  Gain:
                  <input
                    type="range" min="0.5" max="3" step="0.1" value={gain}
                    onChange={(e) => setGain(parseFloat(e.target.value))}
                    className="w-24"
                  />
                  <span className="font-mono">{gain.toFixed(1)}×</span>
                </label>
                <button
                  onClick={() => setMeasuring((v) => !v)}
                  className="flex items-center gap-1 rounded px-2 py-1 text-white"
                  style={{ background: measuring ? colors.industrialOrange : colors.steelGray }}
                >
                  <Ruler className="h-3 w-3" /> Measure
                </button>
                <div className="ml-auto flex items-center gap-2 text-steel-gray">
                  <span>{data.total_pings.toLocaleString()} pings</span>
                  <span>•</span>
                  <span>{data.max_samples_per_channel} samples/ping</span>
                  <span>•</span>
                  <span>{data.header.sonar_name}</span>
                </div>
              </div>

              {/* Canvas */}
              <div className="rounded-md border border-navy-border bg-black overflow-hidden">
                <canvas
                  ref={canvasRef}
                  width={WATERFALL_WIDTH}
                  height={WATERFALL_HEIGHT}
                  onClick={handleCanvasClick}
                  style={{
                    width: "100%",
                    height: "auto",
                    cursor: measuring ? "crosshair" : "default",
                  }}
                />
              </div>

              {/* Measurement panel */}
              {measuring && (
                <div className="rounded-md border p-3 text-xs"
                  style={{ borderColor: `${colors.industrialOrange}40`, background: `${colors.industrialOrange}10` }}>
                  <div className="mb-2 font-semibold" style={{ color: colors.industrialOrange }}>
                    Target Height Measurement
                  </div>
                  {!targetClick && (
                    <div className="text-steel-light">
                      Click on the target (top of shadow) in the waterfall.
                    </div>
                  )}
                  {targetClick && !shadowClick && (
                    <div className="text-steel-light">
                      Target marked at ping {targetClick.pingIdx}, sample {targetClick.sampleIdx} ({targetClick.channel}).
                      Now click on the END of the shadow.
                    </div>
                  )}
                  {targetClick && shadowClick && (
                    <div className="space-y-1">
                      <div className="text-white">
                        Target: ping {targetClick.pingIdx}, sample {targetClick.sampleIdx} ({targetClick.channel})
                      </div>
                      <div className="text-white">
                        Shadow end: ping {shadowClick.pingIdx}, sample {shadowClick.sampleIdx} ({shadowClick.channel})
                      </div>
                      {targetHeightM !== null && (
                        <div className="text-lg font-bold" style={{ color: colors.pass }}>
                          Estimated target height: {targetHeightM.toFixed(2)} m
                        </div>
                      )}
                      <button
                        onClick={() => {
                          const poi = `Target @ ping ${targetClick.pingIdx}, ${targetClick.channel}, height ${targetHeightM?.toFixed(2)}m`;
                          setPoiList([...poiList, poi]);
                          setTargetClick(null);
                          setShadowClick(null);
                          setTargetHeightM(null);
                        }}
                        className="mt-2 flex items-center gap-1 rounded-md px-3 py-1 text-xs font-medium"
                        style={{ background: colors.pass, color: colors.navyBase }}
                      >
                        <Save className="h-3 w-3" /> Save as POI
                      </button>
                    </div>
                  )}
                </div>
              )}

              {/* Saved POIs */}
              {poiList.length > 0 && (
                <div>
                  <h4 className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Saved POIs ({poiList.length})
                  </h4>
                  <div className="space-y-1">
                    {poiList.map((poi, i) => (
                      <div key={i} className="rounded border border-navy-border bg-navy-base px-2 py-1 font-mono text-[10px] text-steel-light">
                        {poi}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <div className="flex items-center gap-2">
            <Crosshair className="h-3 w-3" />
            <span>Click target + shadow to measure height (similar-triangles method)</span>
          </div>
          <button
            onClick={onClose}
            className="flex items-center gap-1 rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}
          >
            <Download className="h-3 w-3" /> Done
          </button>
        </div>
      </div>
    </div>
  );
}

/** Apply log-scaled gain to a u8 backscatter sample. */
function applyGain(sample: number, gain: number): number {
  // Log scale + gain, clamped to 0-255
  const v = Math.log(1 + sample) * gain * 50;
  return Math.min(255, Math.max(0, Math.round(v)));
}
