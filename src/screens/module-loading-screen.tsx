/**
 * Module Loading Screen — Professional Edition
 *
 * Redesigned to match the new splash screen's aesthetic:
 *   - Glassmorphic header with brand mark
 *   - Module cards with status glow
 *   - Animated progress ring in the header
 *   - Monospace init log with syntax-highlighted timestamps
 *   - Clean skip + log toggle footer
 *
 * Critical for surveyors: if PDAL or GDAL fails to load, they need
 * to see WHICH module failed, not a generic "application failed to
 * start" message.
 */

import { useEffect, useState } from "react";
import { Check, AlertCircle, Loader2, FileText, ChevronRight } from "lucide-react";
import { colors, APP_NAME, APP_VERSION } from "@/lib/tokens";
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
  const [bootTime] = useState(() => performance.now());

  useEffect(() => {
    let mounted = true;
    async function loadAll() {
      const mods = await listModules();
      if (!mounted) return;
      setModules(mods);
      const initial: Record<string, ModuleStatus> = {};
      for (const m of mods) initial[m.id] = "pending";
      setStatuses(initial);

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
  const progressPct = modules.length ? (completedCount / modules.length) * 100 : 0;
  const elapsedSec = ((performance.now() - bootTime) / 1000).toFixed(1);

  return (
    <div className="flex h-full w-full flex-col" style={{ background: colors.navyBase }}>
      {/* ── Header ── */}
      <header
        className="flex h-14 items-center justify-between border-b px-4 sm:px-6"
        style={{
          borderColor: colors.navyBorder,
          background: `linear-gradient(180deg, ${colors.navyPanel} 0%, ${colors.navyBase} 100%)`,
        }}
      >
        <div className="flex items-center gap-3 min-w-0">
          <BrandLogoMark size={26} />
          <div className="flex flex-col min-w-0">
            <span className="text-sm font-semibold tracking-wide text-white truncate">
              {APP_NAME}
            </span>
            <span
              className="font-mono"
              style={{ fontSize: 9, color: colors.steelGray, letterSpacing: "0.15em" }}
            >
              MODULE INITIALIZATION
            </span>
          </div>
        </div>

        {/* Progress ring */}
        <div className="flex items-center gap-4">
          <div className="hidden sm:flex flex-col items-end">
            <span
              className="font-mono tabular-nums font-semibold"
              style={{ fontSize: 13, color: colors.industrialOrange }}
            >
              {completedCount} / {modules.length}
            </span>
            <span style={{ fontSize: 9, color: colors.steelGray }}>
              {elapsedSec}s elapsed
            </span>
          </div>
          <ProgressRing percentage={progressPct} size={36} />
        </div>
      </header>

      {/* ── Body ── */}
      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 overflow-y-auto p-4 sm:p-8">
          <div className="mx-auto max-w-2xl">
            {/* Section header */}
            <div className="mb-6">
              <h2 className="text-lg font-semibold text-white">
                Initializing processing core
              </h2>
              <p className="mt-1 text-sm" style={{ color: colors.steelLight }}>
                Each module loads sequentially. Failures are flagged below —
                surveyors need to know exactly which component failed.
              </p>
            </div>

            {/* Module list */}
            <div
              className="space-y-1 rounded-lg border p-2"
              style={{
                borderColor: colors.navyBorder,
                background: `${colors.navyPanel}80`,
              }}
            >
              {modules.length === 0 && (
                <div
                  className="flex items-center justify-center py-8 font-mono"
                  style={{ fontSize: 11, color: colors.steelGray }}
                >
                  <Loader2
                    className="h-4 w-4 animate-spin mr-2"
                    style={{ color: colors.industrialOrange }}
                  />
                  Fetching module manifest…
                </div>
              )}
              {modules.map((mod, idx) => {
                const status = statuses[mod.id] ?? "pending";
                const err = errors[mod.id];
                const isActive = status === "loading";
                return (
                  <div
                    key={mod.id}
                    className="flex items-center gap-3 rounded-md px-3 py-2.5 transition-all"
                    style={{
                      background: isActive
                        ? `${colors.industrialOrange}08`
                        : "transparent",
                      border: `1px solid ${
                        isActive
                          ? `${colors.industrialOrange}30`
                          : "transparent"
                      }`,
                    }}
                  >
                    {/* Index number */}
                    <span
                      className="font-mono w-5 text-right tabular-nums"
                      style={{ fontSize: 10, color: colors.steelGray }}
                    >
                      {(idx + 1).toString().padStart(2, "0")}
                    </span>

                    {/* Status icon */}
                    <div className="w-5 flex-shrink-0">
                      {status === "pending" && (
                        <span
                          className="block h-2 w-2 rounded-full"
                          style={{ background: colors.navyBorder }}
                        />
                      )}
                      {status === "loading" && (
                        <Loader2
                          className="h-4 w-4 animate-spin"
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

                    {/* Module info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-baseline gap-2 flex-wrap">
                        <span
                          className="text-sm font-medium"
                          style={{
                            color:
                              status === "pending"
                                ? colors.steelLight
                                : colors.white,
                          }}
                        >
                          {mod.name}
                        </span>
                        <span
                          className="font-mono"
                          style={{ fontSize: 10, color: colors.steelGray }}
                        >
                          v{mod.version}
                        </span>
                        {mod.can_fail && (
                          <span
                            className="rounded px-1.5 py-0.5 font-mono uppercase tracking-wider"
                            style={{
                              fontSize: 8,
                              background: colors.navyBorder,
                              color: colors.steelLight,
                            }}
                          >
                            optional
                          </span>
                        )}
                      </div>
                      <div
                        className="truncate mt-0.5"
                        style={{ fontSize: 11, color: colors.steelGray }}
                      >
                        {err ? (
                          <span style={{ color: colors.fail }}>{err}</span>
                        ) : (
                          mod.description
                        )}
                      </div>
                    </div>

                    {/* Load time */}
                    <div
                      className="flex-shrink-0 font-mono tabular-nums"
                      style={{ fontSize: 10, color: colors.steelGray }}
                    >
                      {loadTimes[mod.id] ? `${loadTimes[mod.id]}ms` : "—"}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Overall progress bar */}
            {!allDone && (
              <div className="mt-6">
                <div className="mb-2 flex items-center justify-between">
                  <span
                    className="font-mono tracking-wider"
                    style={{ fontSize: 10, color: colors.steelLight }}
                  >
                    OVERALL PROGRESS
                  </span>
                  <span
                    className="font-mono tabular-nums"
                    style={{ fontSize: 10, color: colors.industrialOrange }}
                  >
                    {Math.round(progressPct)}%
                  </span>
                </div>
                <div
                  className="h-[3px] w-full overflow-hidden rounded-full"
                  style={{ background: colors.navyBorder }}
                >
                  <div
                    className="h-full rounded-full transition-all duration-300"
                    style={{
                      width: `${progressPct}%`,
                      background: `linear-gradient(90deg, ${colors.miningYellow}, ${colors.industrialOrange}, ${colors.marineTurquoise})`,
                      boxShadow: `0 0 6px ${colors.industrialOrange}60`,
                    }}
                  />
                </div>
              </div>
            )}

            {/* All-done badge */}
            {allDone && (
              <div
                className="mt-6 flex items-center gap-2 rounded-md border px-4 py-3"
                style={{
                  borderColor: `${colors.pass}40`,
                  background: `${colors.pass}10`,
                }}
              >
                <Check className="h-4 w-4" style={{ color: colors.pass }} />
                <span className="text-sm font-medium" style={{ color: colors.pass }}>
                  All modules initialized — entering workspace…
                </span>
              </div>
            )}
          </div>
        </div>

        {/* ── Init log panel ── */}
        {showLogs && !isVeryNarrow && (
          <div
            className="border-l flex-shrink-0"
            style={{
              borderColor: colors.navyBorder,
              background: colors.navyPanel,
              width: isNarrow ? 288 : 384,
            }}
          >
            <div className="flex h-full flex-col">
              <div
                className="flex items-center justify-between border-b px-4 py-2.5"
                style={{ borderColor: colors.navyBorder }}
              >
                <span
                  className="font-mono tracking-wider"
                  style={{ fontSize: 10, color: colors.steelLight }}
                >
                  INIT LOG
                </span>
                <button
                  onClick={() => setShowLogs(false)}
                  style={{ color: colors.steelGray }}
                  className="hover:text-white"
                  aria-label="Hide logs"
                >
                  <FileText className="h-3.5 w-3.5" />
                </button>
              </div>
              <div
                className="flex-1 overflow-y-auto p-3 font-mono leading-relaxed"
                style={{ fontSize: 10, color: colors.steelLight }}
              >
                {modules
                  .filter((m) => statuses[m.id] && statuses[m.id] !== "pending")
                  .map((m) => (
                    <div key={m.id} className="mb-1">
                      <span style={{ color: colors.steelGray }}>
                        [{new Date().toLocaleTimeString("en-US", { hour12: false })}]
                      </span>{" "}
                      <span style={{ color: colors.industrialOrange }}>
                        [{m.id}]
                      </span>{" "}
                      {statuses[m.id] === "ok" ? (
                        <>
                          <span style={{ color: colors.pass }}>OK</span>{" "}
                          ({loadTimes[m.id]}ms)
                        </>
                      ) : statuses[m.id] === "fail" ? (
                        <>
                          <span style={{ color: colors.fail }}>FAIL</span>{" "}
                          {errors[m.id] ?? "unknown"}
                        </>
                      ) : (
                        <span style={{ color: colors.industrialOrange }}>
                          loading…
                        </span>
                      )}
                    </div>
                  ))}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* ── Footer ── */}
      <footer
        className="flex h-11 items-center justify-between border-t px-4 sm:px-6"
        style={{
          borderColor: colors.navyBorder,
          background: colors.navyPanel,
        }}
      >
        <div className="flex items-center gap-4">
          <button
            onClick={() => setShowLogs((v) => !v)}
            className="flex items-center gap-1.5 text-xs hover:text-white transition-colors"
            style={{ color: colors.steelGray }}
          >
            <FileText className="h-3.5 w-3.5" />
            {showLogs ? "Hide logs" : "Show logs"}
          </button>
          <span
            className="font-mono hidden sm:block"
            style={{ fontSize: 9, color: colors.steelGray }}
          >
            v{APP_VERSION}
          </span>
        </div>
        <button
          onClick={() => {
            const s = useAppStore.getState();
            setPhase(s.hasCompletedOnboarding ? "workspace" : "onboarding");
          }}
          className="flex items-center gap-1 text-xs font-medium hover:text-white transition-colors"
          style={{ color: colors.steelLight }}
        >
          Skip
          <ChevronRight className="h-3.5 w-3.5" />
        </button>
      </footer>
    </div>
  );
}

/**
 * Circular progress ring — SVG arc that fills as modules load.
 * Compact, professional, and more visually distinctive than a bar.
 */
function ProgressRing({
  percentage,
  size,
}: {
  percentage: number;
  size: number;
}) {
  const stroke = 3;
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (percentage / 100) * circumference;

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="-rotate-90">
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={colors.navyBorder}
          strokeWidth={stroke}
        />
        {/* Progress arc */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={colors.industrialOrange}
          strokeWidth={stroke}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          style={{
            transition: "stroke-dashoffset 0.3s ease-out",
            filter: `drop-shadow(0 0 2px ${colors.industrialOrange}80)`,
          }}
        />
      </svg>
      <div
        className="absolute inset-0 flex items-center justify-center font-mono font-semibold tabular-nums"
        style={{ fontSize: 10, color: colors.industrialOrange }}
      >
        {Math.round(percentage)}
      </div>
    </div>
  );
}
