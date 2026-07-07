/**
 * ProgressBar — indeterminate + determinate progress indicator (Sprint 14).
 *
 * Replaces the bare "Loading..." text that was the #3 critical friction
 * point in the UX Researcher audit. Surveyors couldn't tell if a 30-second
 * CSF classification was 10% or 90% done, or if it had hung.
 *
 * Two modes:
 *   - Determinate: caller provides progress (0-100) + optional ETA
 *   - Indeterminate: animated bar for when total work is unknown
 *
 * Shows:
 *   - Progress bar (fill width = progress%)
 *   - Percentage text
 *   - Elapsed time (mm:ss)
 *   - Optional ETA (mm:ss) — caller computes from progress + elapsed
 *   - Optional status message ("Classifying ground points...", "Writing PDF...")
 *
 * Usage:
 * ```tsx
 * <ProgressBar
 *   progress={65}
 *   status="Classifying ground points..."
 *   startTime={Date.now()}
 *   etaSeconds={12}
 * />
 * ```
 */

import { useState, useEffect } from "react";
import { Loader2 } from "lucide-react";
import { colors } from "@/lib/tokens";

interface ProgressBarProps {
  /** Progress 0-100. If undefined, shows indeterminate animation. */
  progress?: number;
  /** Status message shown above the bar. */
  status?: string;
  /** Start time (Unix ms) for elapsed time calculation. */
  startTime?: number;
  /** Estimated time remaining (seconds). Caller computes from rate. */
  etaSeconds?: number;
  /** If true, show a cancel button. */
  cancellable?: boolean;
  onCancel?: () => void;
}

export function ProgressBar({
  progress,
  status,
  startTime,
  etaSeconds,
  cancellable = false,
  onCancel,
}: ProgressBarProps) {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (!startTime) return;
    const timer = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);
    return () => clearInterval(timer);
  }, [startTime]);

  const isDeterminate = progress != null;
  const pct = isDeterminate ? Math.max(0, Math.min(100, progress)) : 0;

  return (
    <div className="w-full">
      {/* Status message */}
      {status && (
        <div className="mb-1.5 flex items-center gap-1.5 text-xs text-steel-light">
          <Loader2 className="h-3 w-3 animate-spin" style={{ color: colors.accent }} />
          <span>{status}</span>
        </div>
      )}

      {/* Progress bar */}
      <div className="relative h-3 w-full overflow-hidden rounded-full bg-navy-elevated">
        {isDeterminate ? (
          <div
            className="h-full rounded-full transition-all duration-300 ease-out"
            style={{
              width: `${pct}%`,
              background: `linear-gradient(90deg, ${colors.accent}, ${colors.accentLight})`,
            }}
          />
        ) : (
          // Indeterminate — animated sliding bar
          <div
            className="h-full w-1/3 rounded-full"
            style={{
              background: `linear-gradient(90deg, transparent, ${colors.accent}, transparent)`,
              animation: "progress-indeterminate 1.5s ease-in-out infinite",
            }}
          />
        )}
      </div>

      {/* Stats row */}
      <div className="mt-1 flex items-center justify-between text-[10px] font-mono text-steel-gray">
        <span>
          {isDeterminate ? `${pct.toFixed(0)}%` : "Working..."}
        </span>
        {startTime && (
          <span>
            {formatTime(elapsed)}
            {etaSeconds != null && etaSeconds > 0 && (
              <span className="ml-2" style={{ color: colors.textMuted ?? colors.steelGray }}>
                ETA: {formatTime(etaSeconds)}
              </span>
            )}
          </span>
        )}
        {cancellable && onCancel && (
          <button
            onClick={onCancel}
            className="rounded px-2 py-0.5 text-[10px] font-medium"
            style={{ background: colors.fail, color: colors.white }}
          >
            Cancel
          </button>
        )}
      </div>

      <style>{`
        @keyframes progress-indeterminate {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(400%); }
        }
      `}</style>
    </div>
  );
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}
