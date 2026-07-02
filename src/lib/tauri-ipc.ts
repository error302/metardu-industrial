/**
 * Tauri IPC wrapper for MetaRDU Industrial.
 *
 * Provides typed access to Rust commands exposed via `invoke()`.
 * Falls back to browser-mode stubs when `window.__TAURI_INTERNALS__` is
 * absent so the frontend can run via `npm run dev` without the Rust core
 * compiled in.
 */

import { invoke, isTauri } from "@tauri-apps/api/core";

export interface ModuleInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  can_fail: boolean;
}

export type ModuleStatus = "pending" | "loading" | "ok" | "fail";

export interface ModuleLoadResult {
  id: string;
  status: ModuleStatus;
  load_time_ms: number;
  error: string | null;
}

export interface AppSettingsRpc {
  defaultDomain: string;
  defaultEpsg: string;
  density: string;
  reducedMotion: boolean;
}

/** True when running inside the Tauri native shell. */
export const isNative = (): boolean => isTauri();

/** Health check — confirms IPC bridge is alive. */
export async function ping(): Promise<string> {
  if (!isTauri()) return "browser-mode-stub";
  return invoke<string>("ping");
}

/** Get the Rust core semantic version. */
export async function appVersion(): Promise<string> {
  if (!isTauri()) return "0.1.0-browser";
  return invoke<string>("app_version");
}

/** List all known processing modules. */
export async function listModules(): Promise<ModuleInfo[]> {
  if (!isTauri()) return BROWSER_MODULE_STUBS;
  return invoke<ModuleInfo[]>("list_modules");
}

/** Initialize a single module by id. */
export async function initModule(id: string): Promise<ModuleLoadResult> {
  if (!isTauri()) {
    // Simulate load in browser mode
    const start = performance.now();
    const loadMs = BROWSER_LOAD_MS[id] ?? 500;
    await new Promise((r) => setTimeout(r, loadMs));
    return {
      id,
      status: "ok",
      load_time_ms: Math.round(performance.now() - start),
      error: null,
    };
  }
  return invoke<ModuleLoadResult>("init_module", { id });
}

/** Read persisted user settings. */
export async function getSettings(): Promise<AppSettingsRpc | null> {
  if (!isTauri()) return null;
  return invoke<AppSettingsRpc>("get_settings");
}

/** Persist user settings. */
export async function saveSettings(
  settings: AppSettingsRpc,
): Promise<void> {
  if (!isTauri()) {
    // Browser fallback — persist to localStorage so refreshes keep state
    localStorage.setItem("metardu.settings", JSON.stringify(settings));
    return;
  }
  return invoke<void>("save_settings", { settings });
}

// ──────────────────────────────────────────────────────────────────
// Browser-mode stubs — mirror the registry in src-tauri/src/modules/registry.rs

const BROWSER_MODULE_STUBS: ModuleInfo[] = [
  {
    id: "geodesy",
    name: "Geodesy engine",
    version: "PROJ 9.4",
    description: "Coordinate transforms, CRS management, datum shifts",
    can_fail: false,
  },
  {
    id: "raster",
    name: "Raster I/O",
    version: "GDAL 3.8",
    description: "GeoTIFF/COG read, warp, mosaic, reprojection",
    can_fail: false,
  },
  {
    id: "pointcloud",
    name: "Point cloud engine",
    version: "PDAL 2.6",
    description: "LAS/LAZ ingest, classification, ground extraction",
    can_fail: false,
  },
  {
    id: "spatialite",
    name: "Spatial index",
    version: "SpatiaLite 5.1",
    description: "Embedded local cache, project metadata, search",
    can_fail: false,
  },
  {
    id: "coord-reg",
    name: "Coordinate registry",
    version: "internal",
    description: "Least-squares adjustment, deformation tracking",
    can_fail: false,
  },
  {
    id: "marine",
    name: "Marine sonar readers",
    version: ".all / .s7k / .bsf",
    description: "Kongsberg, Reson, R2Sonic multibeam ingest",
    can_fail: true,
  },
  {
    id: "mining",
    name: "Mining drone pipelines",
    version: "DJI / SenseFly",
    description: "UAV photogrammetry ingest, ODM bindings",
    can_fail: true,
  },
  {
    id: "reporting",
    name: "Reporting engine",
    version: "internal",
    description: "PDF, KML, DXF, S-57, GeoTIFF export",
    can_fail: false,
  },
];

const BROWSER_LOAD_MS: Record<string, number> = {
  geodesy: 700,
  raster: 900,
  pointcloud: 800,
  spatialite: 350,
  "coord-reg": 500,
  marine: 600,
  mining: 650,
  reporting: 400,
};
