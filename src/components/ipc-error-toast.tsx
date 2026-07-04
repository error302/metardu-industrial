/**
 * IPC Error Toast — renders the latest unhandled IPC / rejection errors
 * as a dismissible banner at the top of the screen.
 *
 * Mounted once at the App root (inside <ErrorBoundary/>) so it sits
 * above every screen and dialog. The banner auto-dismisses after 8s
 * for non-IPC errors (likely a transient blip) but stays put for IPC
 * errors (which usually mean a Rust panic — the user needs to know).
 */

import { useEffect } from "react";
import { AlertTriangle, X } from "lucide-react";
import { colors } from "@/lib/tokens";
import { useIpcErrors } from "@/lib/ipc-error-reporter";

export function IpcErrorToast() {
  const errors = useIpcErrors((s) => s.errors);
  const dismiss = useIpcErrors((s) => s.dismiss);

  // Auto-dismiss non-IPC errors after 8s — they're usually transient.
  // IPC errors stay until the user dismisses them, because they almost
  // always indicate a real Rust-side problem the user should report.
  useEffect(() => {
    if (errors.length === 0) return;
    const timers = errors
      .filter((e) => e.kind !== "ipc")
      .map((e) => window.setTimeout(() => dismiss(e.id), 8000));
    return () => {
      for (const t of timers) window.clearTimeout(t);
    };
  }, [errors, dismiss]);

  if (errors.length === 0) return null;

  return (
    <div
      className="pointer-events-none fixed top-0 left-0 right-0 z-[200] flex flex-col items-center gap-1.5 px-4 pt-3"
      aria-live="assertive"
      aria-atomic="true"
    >
      {errors.map((e) => (
        <div
          key={e.id}
          className="pointer-events-auto flex w-full max-w-2xl items-start gap-3 rounded-md border px-4 py-3 shadow-2xl backdrop-blur-sm"
          style={{
            background: e.kind === "ipc" ? "rgba(120, 20, 20, 0.95)" : "rgba(60, 40, 10, 0.95)",
            borderColor: e.kind === "ipc" ? colors.fail : colors.industrialOrange,
            color: "white",
          }}
          role="alert"
        >
          <AlertTriangle
            className="mt-0.5 h-4 w-4 flex-shrink-0"
            style={{ color: e.kind === "ipc" ? "#FFB4B4" : colors.industrialOrange }}
          />
          <div className="min-w-0 flex-1">
            <div className="text-[11px] font-semibold uppercase tracking-wider opacity-80">
              {e.kind === "ipc" ? "Processing Error" : "Application Error"}
            </div>
            <div className="mt-0.5 break-words font-mono text-[12px] leading-snug">
              {e.message}
            </div>
            <div className="mt-1 text-[10px] opacity-60">
              {new Date(e.ts).toLocaleTimeString()} · click × to dismiss
            </div>
          </div>
          <button
            onClick={() => dismiss(e.id)}
            className="flex-shrink-0 rounded p-1 text-white/80 transition-colors hover:bg-white/10 hover:text-white"
            aria-label="Dismiss error"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      ))}
    </div>
  );
}
