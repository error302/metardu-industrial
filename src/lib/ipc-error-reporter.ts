/**
 * IPC Error Reporter — last-resort safety net for Tauri invoke failures.
 *
 * The existing <ErrorBoundary> catches React *render* errors. But when a
 * Rust command panics or returns `Err(String)`, Tauri rejects the promise
 * from `invoke()`. If a component forgot a try/catch (or if a future
 * refactor introduces one), the rejection bubbles up as an
 * "unhandledrejection" event and the UI silently hangs — no spinner
 * stops, no error shows, the surveyor has no idea why nothing happens.
 *
 * This module:
 *   1. Listens to `window.addEventListener("unhandledrejection", …)`.
 *   2. Extracts a human-readable message from any Tauri / Rust payload
 *      (handles plain strings, Error, {message}, {error}, and unknown).
 *   3. Pushes it into a tiny Zustand store that any component can
 *      subscribe to. The <IpcErrorToast/> component renders the latest
 *      error as a dismissible banner at the top of the screen.
 *
 * This is purely additive — it does NOT change how any existing dialog
 * handles its own try/catch. It only catches the ones that slip through.
 */

import { create } from "zustand";

export interface IpcError {
  /** Unique id (timestamp) so React keys stay stable. */
  id: number;
  /** Short label, e.g. "IPC Error" or "Unhandled Rejection". */
  kind: "ipc" | "unknown";
  /** Human-readable message extracted from the rejection reason. */
  message: string;
  /** When it happened (epoch ms). */
  ts: number;
}

interface IpcErrorState {
  errors: IpcError[];
  push: (e: Omit<IpcError, "id" | "ts">) => void;
  dismiss: (id: number) => void;
  clear: () => void;
}

export const useIpcErrors = create<IpcErrorState>((set) => ({
  errors: [],
  push: (e) =>
    set((s) => ({
      // Keep at most 5 most-recent errors so the banner doesn't grow
      // unbounded if a polling loop starts throwing.
      errors: [...s.errors, { ...e, id: Date.now() + Math.random(), ts: Date.now() }].slice(-5),
    })),
  dismiss: (id) =>
    set((s) => ({ errors: s.errors.filter((e) => e.id !== id) })),
  clear: () => set({ errors: [] }),
}));

/**
 * Best-effort extraction of a human-readable message from a Tauri / Rust
 * rejection payload. Tauri's `invoke()` rejects with whatever the Rust
 * side returned from `Err(...)` — usually a plain string, but custom
 * error types serialize to objects with a `message` or `error` field.
 */
function describeReason(reason: unknown): string {
  if (reason == null) return "(no error payload)";
  if (typeof reason === "string") return reason;
  if (reason instanceof Error) return reason.message || reason.name;
  if (typeof reason === "object") {
    const r = reason as Record<string, unknown>;
    if (typeof r.message === "string" && r.message) return r.message;
    if (typeof r.error === "string" && r.error) return r.error;
    if (typeof r.kind === "string" && r.kind) return `${r.kind}: ${JSON.stringify(reason)}`;
    try {
      return JSON.stringify(reason);
    } catch {
      return String(reason);
    }
  }
  return String(reason);
}

/**
 * Heuristic: does this rejection look like it came from a Tauri `invoke()`
 * call? Tauri's error payloads are either strings (from `Err(String)`)
 * or objects with a `message` field. We can't be 100% certain, but the
 * `kind` field on our IpcError type lets the UI label it appropriately.
 */
function looksLikeTauri(reason: unknown): boolean {
  if (typeof reason === "string") return true;
  if (reason && typeof reason === "object") {
    const r = reason as Record<string, unknown>;
    return typeof r.message === "string" || typeof r.error === "string";
  }
  return false;
}

let installed = false;

/**
 * Install the global unhandledrejection listener. Safe to call multiple
 * times — only the first call wires up the listener.
 */
export function installIpcErrorReporter(): void {
  if (installed) return;
  installed = true;

  if (typeof window === "undefined") return;

  window.addEventListener("unhandledrejection", (event) => {
    const reason = event.reason;
    const message = describeReason(reason);
    const kind: IpcError["kind"] = looksLikeTauri(reason) ? "ipc" : "unknown";

    // Always log to console so devs can see the full reason + stack
    // even if the UI banner is dismissed.
    console.error("[MetaRDU] Unhandled rejection:", reason);

    useIpcErrors.getState().push({ kind, message });

    // Prevent the browser's default "unhandled rejection" console noise
    // since we've already captured and surfaced it.
    event.preventDefault();
  });

  // Also catch synchronous errors that escape React's error boundary
  // (e.g. errors in setTimeout / fetch callbacks that aren't in a
  // React render cycle).
  window.addEventListener("error", (event) => {
    const msg = event.error instanceof Error
      ? event.error.message
      : event.message || "Unknown error";
    console.error("[MetaRDU] Uncaught error:", event.error ?? event.message);
    useIpcErrors.getState().push({ kind: "unknown", message: msg });
    event.preventDefault();
  });
}
