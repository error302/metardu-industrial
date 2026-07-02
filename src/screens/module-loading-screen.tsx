/**
 * Module Loading Screen
 * Shows module-by-module initialization. Critical for surveyors — if PDAL
 * or GDAL fails to load, they need to see WHICH module failed, not a
 * generic "application failed to start" message.
 */

import { useEffect, useState } from "react";
import { Check, AlertCircle, Loader2, FileText } from "lucide-react";
import { colors, APP_NAME } from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";

type ModuleStatus = "pending" | "loading" | "ok" | "fail";

interface ModuleEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  loadTimeMs: number;
  canFail?: boolean; // optional modules won't block boot
}

const MODULES: ModuleEntry[] = [
  {
    id: "geodesy",
    name: "Geodesy engine",
    version: "PROJ 9.4",
    description: "Coordinate transforms, CRS management, datum shifts",
    loadTimeMs: 700,
  },
  {
    id: "raster",
    name: "Raster I/O",
    version: "GDAL 3.8",
    description: "GeoTIFF/COG read, warp, mosaic, reprojection",
    loadTimeMs: 900,
  },
  {
    id: "pointcloud",
    name: "Point cloud engine",
    version: "PDAL 2.6",
    description: "LAS/LAZ ingest, classification, ground extraction",
    loadTimeMs: 800,
  },
  {
    id: "spatialite",
    name: "Spatial index",
    version: "SpatiaLite 5.1",
    description: "Embedded local cache, project metadata, search",
    loadTimeMs: 350,
  },
  {
    id: "coord-reg",
    name: "Coordinate registry",
    version: "internal",
    description: "Least-squares adjustment, deformation tracking",
    loadTimeMs: 500,
  },
  {
    id: "marine",
    name: "Marine sonar readers",
    version: ".all / .s7k / .bsf",
    description: "Kongsberg, Reson, R2Sonic multibeam ingest",
    loadTimeMs: 600,
    canFail: true,
  },
  {
    id: "mining",
    name: "Mining drone pipelines",
    version: "DJI / SenseFly",
    description: "UAV photogrammetry ingest, ODM bindings",
    loadTimeMs: 650,
    canFail: true,
  },
  {
    id: "reporting",
    name: "Reporting engine",
    version: "internal",
    description: "PDF, KML, DXF, S-57, GeoTIFF export",
    loadTimeMs: 400,
  },
];

export function ModuleLoadingScreen() {
  const setPhase = useAppStore((s) => s.setPhase);
  const hasCompletedOnboarding = useAppStore(
    (s) => s.hasCompletedOnboarding,
  );
  const [statuses, setStatuses] = useState<Record<string, ModuleStatus>>(
    Object.fromEntries(MODULES.map((m) => [m.id, "pending"])),
  );
  const [loadTimes, setLoadTimes] = useState<Record<string, number>>({});
  const [showLogs, setShowLogs] = useState(false);

  useEffect(() => {
    let mounted = true;
    async function loadModules() {
      for (const mod of MODULES) {
        if (!mounted) return;
        setStatuses((s) => ({ ...s, [mod.id]: "loading" }));
        const start = performance.now();
        // Simulate load — real impl calls Tauri command `init_module`
        await new Promise((r) => setTimeout(r, mod.loadTimeMs));
        if (!mounted) return;
        const elapsed = Math.round(performance.now() - start);
        setLoadTimes((t) => ({ ...t, [mod.id]: elapsed }));
        setStatuses((s) => ({ ...s, [mod.id]: "ok" }));
      }
      if (!mounted) return;
      // All loaded — proceed to onboarding or workspace
      setTimeout(() => {
        if (!mounted) return;
        setPhase(hasCompletedOnboarding ? "workspace" : "onboarding");
      }, 400);
    }
    loadModules();
    return () => {
      mounted = false;
    };
  }, [setPhase, hasCompletedOnboarding]);

  const allDone = Object.values(statuses).every((s) => s === "ok" || s === "fail");

  return (
    <div className="flex h-full w-full flex-col bg-navy-base">
      {/* Header */}
      <header className="flex h-12 items-center justify-between border-b border-navy-border px-6">
        <div className="flex items-center gap-3">
          <div
            className="flex h-7 w-7 items-center justify-center rounded font-bold"
            style={{
              background: colors.industrialOrange,
              color: colors.navyBase,
            }}
          >
            M
          </div>
          <span className="text-sm font-medium tracking-wide text-white">
            {APP_NAME}
          </span>
        </div>
        <div className="font-mono text-[10px] text-steel-gray">
          MODULE INITIALIZATION
        </div>
      </header>

      {/* Body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Module list */}
        <div className="flex-1 overflow-y-auto p-8">
          <div className="mx-auto max-w-2xl">
            <h2 className="mb-1 text-lg font-semibold text-white">
              Loading modules…
            </h2>
            <p className="mb-6 text-sm text-steel-light">
              Initializing processing core. Failures will be flagged below —
              surveyors need to know exactly which component failed.
            </p>

            <div className="space-y-1">
              {MODULES.map((mod) => {
                const status = statuses[mod.id];
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
                        <Check
                          className="h-4 w-4"
                          style={{ color: colors.pass }}
                        />
                      )}
                      {status === "fail" && (
                        <AlertCircle
                          className="h-4 w-4"
                          style={{ color: colors.fail }}
                        />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-baseline gap-2">
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
                        {mod.canFail && (
                          <span className="rounded-sm bg-navy-border px-1.5 py-0.5 text-[9px] uppercase tracking-wider text-steel-light">
                            optional
                          </span>
                        )}
                      </div>
                      <div className="truncate text-xs text-steel-gray">
                        {mod.description}
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
                      width: `${
                        (Object.values(statuses).filter(
                          (s) => s === "ok" || s === "fail",
                        ).length /
                          MODULES.length) *
                        100
                      }%`,
                      background: colors.industrialOrange,
                    }}
                  />
                </div>
                <span>
                  {Object.values(statuses).filter(
                    (s) => s === "ok" || s === "fail",
                  ).length}{" "}
                  / {MODULES.length}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Log panel */}
        {showLogs && (
          <div className="w-96 border-l border-navy-border bg-navy-panel">
            <div className="flex h-full flex-col">
              <div className="flex items-center justify-between border-b border-navy-border px-4 py-2">
                <span className="font-mono text-[10px] tracking-wider text-steel-light">
                  INIT LOG
                </span>
                <button
                  onClick={() => setShowLogs(false)}
                  className="text-steel-gray hover:text-white"
                >
                  <FileText className="h-3.5 w-3.5" />
                </button>
              </div>
              <div className="flex-1 overflow-y-auto p-3 font-mono text-[10px] leading-relaxed text-steel-light">
                {MODULES.filter((m) => statuses[m.id] !== "pending").map(
                  (m) => (
                    <div key={m.id}>
                      <span className="text-steel-gray">
                        {new Date().toLocaleTimeString()}{" "}
                      </span>
                      <span style={{ color: colors.industrialOrange }}>
                        [{m.id}]
                      </span>{" "}
                      {statuses[m.id] === "ok"
                        ? `loaded in ${loadTimes[m.id]}ms`
                        : "loading…"}
                    </div>
                  ),
                )}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <footer className="flex h-10 items-center justify-between border-t border-navy-border px-6">
        <button
          onClick={() => setShowLogs((v) => !v)}
          className="text-xs text-steel-gray hover:text-white"
        >
          {showLogs ? "Hide logs" : "Show logs"}
        </button>
        <button
          onClick={() =>
            setPhase(hasCompletedOnboarding ? "workspace" : "onboarding")
          }
          className="text-xs text-steel-gray hover:text-white"
        >
          Skip →
        </button>
      </footer>
    </div>
  );
}
