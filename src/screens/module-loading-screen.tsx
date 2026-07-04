/**
 * Module Loading Screen
 * Shows module-by-module initialization. Critical for surveyors — if PDAL
 * or GDAL fails to load, they need to see WHICH module failed, not a
 * generic "application failed to start" message.
 *
 * Uses the Tauri IPC layer (src/lib/tauri-ipc.ts) for real init calls when
 * running in the native shell. Falls back to simulated timings in browser.
 */

import { useEffect, useState } from "react";
import { Check, AlertCircle, Loader2, FileText } from "lucide-react";
import { colors, APP_NAME } from "@/lib/tokens";
import { BrandLogoMark } from "@/components/brand-logo";
import { useAppStore } from "@/stores/app-store";
import { useViewport } from "@/lib/use-viewport";
import {
  listModules,
  initModule,
  type ModuleInfo,
  type ModuleLoadResult,
  type ModuleStatus,
} from "@/lib/tauri-ipc";

export function ModuleLoadingScreen() {
  const setPhase = useAppStore((s) => s.setPhase);
  const hasCompletedOnboarding = useAppStore((s) => s.hasCompletedOnboarding);
  const hydrated = useAppStore((s) => s.hydrated);
  const { isNarrow, isVeryNarrow } = useViewport();

  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [statuses, setStatuses] = useState<Record<string, ModuleStatus>>({});
  const [loadTimes, setLoadTimes] = useState<Record<string, number>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [showLogs, setShowLogs] = useState(false);

  useEffect(() => {
    let mounted = true;
    async function loadAll() {
      // Fetch module list from Rust core (or browser stub)
      const mods = await listModules();
      if (!mounted) return;
      setModules(mods);
      const initial: Record<string, ModuleStatus> = {};
      for (const m of mods) initial[m.id] = "pending";
      setStatuses(initial);

      // Sequential load — preserves dependency order (geodesy before raster, etc.)
      for (const mod of mods) {
        if (!mounted) return;
        setStatuses((s) => ({ ...s, [mod.id]: "loading" }));
        try {
          const result: ModuleLoadResult = await initModule(mod.id);
          if (!mounted) return;
          setStatuses((s) => ({ ...s, [mod.id]: result.status }));
          setLoadTimes((t) => ({ ...t, [mod.id]: result.load_time_ms }));
          if (result.error) {
            setErrors((e) => ({ ...e, [mod.id]: result.error! }));
          }
        } catch (err) {
          if (!mounted) return;
          setStatuses((s) => ({ ...s, [mod.id]: "fail" }));
          setErrors((e) => ({
            ...e,
            [mod.id]: err instanceof Error ? err.message : String(err),
          }));
        }
      }

      // Brief beat, then advance — but only if hydration is done.
      // If hydrate() hasn't finished yet (rare but possible if Rust
      // IPC is slow on first call), keep polling until it does, so
      // we don't accidentally send the user back to onboarding when
      // they actually completed it last session.
      const tryAdvance = () => {
        if (!mounted) return;
        if (!useAppStore.getState().hydrated) {
          setTimeout(tryAdvance, 100);
          return;
        }
        setPhase(
          useAppStore.getState().hasCompletedOnboarding ? "workspace" : "onboarding",
        );
      };
      setTimeout(tryAdvance, 500);
    }
    loadAll();
    return () => {
      mounted = false;
    };
  }, [setPhase, hasCompletedOnboarding, hydrated]);

  const completedCount = Object.values(statuses).filter(
    (s) => s === "ok" || s === "fail",
  ).length;
  const allDone = modules.length > 0 && completedCount === modules.length;

  return (
    <div className="flex h-full w-full flex-col bg-navy-base">
      <header className="flex h-12 items-center justify-between border-b border-navy-border px-4 sm:px-6">
        <div className="flex items-center gap-3 min-w-0">
          <BrandLogoMark size={24} />
          <span className="text-sm font-medium tracking-wide text-white truncate">
            {APP_NAME}
          </span>
        </div>
        <div className="font-mono text-[10px] text-steel-gray whitespace-nowrap hidden sm:block">
          MODULE INITIALIZATION
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 overflow-y-auto p-4 sm:p-8">
          <div className="mx-auto max-w-2xl">
            <h2 className="mb-1 text-lg font-semibold text-white">
              Loading modules…
            </h2>
            <p className="mb-6 text-sm text-steel-light">
              Initializing processing core. Failures will be flagged below —
              surveyors need to know exactly which component failed.
            </p>

            <div className="space-y-1">
              {modules.map((mod) => {
                const status = statuses[mod.id] ?? "pending";
                const err = errors[mod.id];
                return (
                  <div
                    key={mod.id}
                    className="flex items-center gap-3 rounded-md border border-transparent px-3 py-2.5 transition-colors hover:border-navy-border hover:bg-navy-panel"
                  >
                    <div className="w-5 flex-shrink-0">
                      {status === "pending" && (
                        <span className="status-dot status-dot--pending" />
                      )}
                      {status === "loading" && (
                        <Loader2
                          className="h-3.5 w-3.5 animate-spin"
                          style={{ color: colors.industrialOrange }}
                        />
                      )}
                      {status === "ok" && (
                        <Check className="h-4 w-4" style={{ color: colors.pass }} />
                      )}
                      {status === "fail" && (
                        <AlertCircle
                          className="h-4 w-4"
                          style={{ color: colors.fail }}
                        />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-baseline gap-2 flex-wrap">
                        <span
                          className={`text-sm font-medium ${
                            status === "pending"
                              ? "text-steel-light"
                              : "text-white"
                          }`}
                        >
                          {mod.name}
                        </span>
                        <span className="font-mono text-[10px] text-steel-gray">
                          {mod.version}
                        </span>
                        {mod.can_fail && (
                          <span className="rounded-sm bg-navy-border px-1.5 py-0.5 text-[9px] uppercase tracking-wider text-steel-light">
                            optional
                          </span>
                        )}
                      </div>
                      <div className="truncate text-xs text-steel-gray">
                        {err ? (
                          <span style={{ color: colors.fail }}>{err}</span>
                        ) : (
                          mod.description
                        )}
                      </div>
                    </div>
                    <div className="flex-shrink-0 font-mono text-[10px] tabular-nums text-steel-gray">
                      {loadTimes[mod.id] ? `${loadTimes[mod.id]}ms` : "—"}
                    </div>
                  </div>
                );
              })}
            </div>

            {!allDone && (
              <div className="mt-6 flex items-center gap-2 text-xs text-steel-gray">
                <div className="h-1 flex-1 overflow-hidden rounded-full bg-navy-border">
                  <div
                    className="h-full transition-all duration-300"
                    style={{
                      width: `${modules.length ? (completedCount / modules.length) * 100 : 0}%`,
                      background: colors.industrialOrange,
                    }}
                  />
                </div>
                <span className="tabular-nums">
                  {completedCount} / {modules.length}
                </span>
              </div>
            )}
          </div>
        </div>

        {showLogs && !isVeryNarrow && (
          <div
            className={`border-l border-navy-border bg-navy-panel ${
              isNarrow ? "w-72" : "w-96"
            } flex-shrink-0`}
          >
            <div className="flex h-full flex-col">
              <div className="flex items-center justify-between border-b border-navy-border px-4 py-2">
                <span className="font-mono text-[10px] tracking-wider text-steel-light">
                  INIT LOG
                </span>
                <button
                  onClick={() => setShowLogs(false)}
                  className="text-steel-gray hover:text-white"
                  aria-label="Hide logs"
                >
                  <FileText className="h-3.5 w-3.5" />
                </button>
              </div>
              <div className="flex-1 overflow-y-auto p-3 font-mono text-[10px] leading-relaxed text-steel-light">
                {modules
                  .filter((m) => statuses[m.id] && statuses[m.id] !== "pending")
                  .map((m) => (
                    <div key={m.id}>
                      <span className="text-steel-gray">
                        {new Date().toLocaleTimeString()}{" "}
                      </span>
                      <span style={{ color: colors.industrialOrange }}>
                        [{m.id}]
                      </span>{" "}
                      {statuses[m.id] === "ok"
                        ? `loaded in ${loadTimes[m.id]}ms`
                        : statuses[m.id] === "fail"
                          ? `FAILED: ${errors[m.id] ?? "unknown"}`
                          : "loading…"}
                    </div>
                  ))}
              </div>
            </div>
          </div>
        )}
      </div>

      <footer className="flex h-10 items-center justify-between border-t border-navy-border px-4 sm:px-6">
        <button
          onClick={() => setShowLogs((v) => !v)}
          className="text-xs text-steel-gray hover:text-white"
        >
          {showLogs ? "Hide logs" : "Show logs"}
        </button>
        <button
          onClick={() => {
            // If hydration is somehow still pending, read the latest
            // state directly from the store rather than the stale
            // closure value.
            const s = useAppStore.getState();
            setPhase(s.hasCompletedOnboarding ? "workspace" : "onboarding");
          }}
          className="text-xs text-steel-gray hover:text-white"
        >
          Skip →
        </button>
      </footer>
    </div>
  );
}
