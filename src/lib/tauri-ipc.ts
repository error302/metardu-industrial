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
// File probing — returns metadata + bounds for a survey file

export interface LasHeaderRpc {
  file_source_id: number;
  global_encoding: number;
  version_major: number;
  version_minor: number;
  system_identifier: string;
  generating_software: string;
  file_creation_day: number;
  file_creation_year: number;
  header_size: number;
  offset_to_point_data: number;
  number_of_vlrs: number;
  point_data_format: number;
  point_data_record_length: number;
  point_count: number;
  points_by_return: number[];
  scale_x: number;
  scale_y: number;
  scale_z: number;
  offset_x: number;
  offset_y: number;
  offset_z: number;
  min_x: number;
  min_y: number;
  min_z: number;
  max_x: number;
  max_y: number;
  max_z: number;
  crs_wkt: string | null;
  geotiff_keys: number[] | null;
}

export interface GeoTiffHeaderRpc {
  width: number;
  length: number;
  bits_per_sample: number;
  samples_per_pixel: number;
  compression: number;
  photometric: number;
  is_tiled: boolean;
  strip_count: number;
  model_pixel_scale: [number, number, number] | null;
  model_tiepoint: [number, number, number, number, number, number] | null;
  epsg: number | null;
  geo_ascii: string | null;
  bounds: [number, number, number, number] | null; // min_x, min_y, max_x, max_y
}

export interface KongsbergAllHeaderRpc {
  model: string;
  model_id: number;
  date: string;
  seconds_since_epoch: number;
  ping_count: number;
  position_count: number;
  attitude_count: number;
  svp_count: number;
  runtime_count: number;
  total_datagrams: number;
  first_timestamp: number | null;
  last_timestamp: number | null;
}

export interface ResonS7kHeaderRpc {
  model: string;
  version: number;
  date: string;
  seconds_since_epoch: number;
  bathymetry_count: number;
  position_count: number;
  attitude_count: number;
  svp_count: number;
  sonar_settings_count: number;
  side_scan_count: number;
  snippet_count: number;
  total_records: number;
  first_timestamp: number | null;
  last_timestamp: number | null;
}

export type FileProbeResult =
  | { kind: "las"; path: string; header: LasHeaderRpc }
  | { kind: "geo-tiff"; path: string; header: GeoTiffHeaderRpc }
  | { kind: "kongsberg-all"; path: string; header: KongsbergAllHeaderRpc }
  | { kind: "reson-s7k"; path: string; header: ResonS7kHeaderRpc }
  | { kind: "mb-es"; path: string; vendor: string; size_bytes: number }
  | { kind: "other"; path: string; size_bytes: number; note: string };

/** Probe a survey file by path. Returns header + bounds metadata. */
export async function probeFile(path: string): Promise<FileProbeResult> {
  if (!isTauri()) {
    // Browser fallback — synthesize a small placeholder
    return {
      kind: "other",
      path,
      size_bytes: 0,
      note: "browser-mode stub",
    };
  }
  return invoke<FileProbeResult>("probe_file", { path });
}

// ──────────────────────────────────────────────────────────────────
// Elevation profile — sample real elevation from a loaded GeoTIFF DEM

export interface ProfileSampleResult {
  /** Elevation samples (in DEM units — usually meters) */
  elevations: number[];
  /** Distance per sample in meters (haversine, lon/lat assumption) */
  distances: number[];
  /** Min elevation across the samples */
  min_elevation: number;
  /** Max elevation across the samples */
  max_elevation: number;
  /** True if from real DEM data, false if synthesized */
  from_real_dem: boolean;
}

/**
 * Sample elevation along a profile line in a loaded GeoTIFF DEM.
 * Returns null in browser mode (no real DEM access).
 */
export async function sampleProfile(
  path: string,
  startLon: number,
  startLat: number,
  endLon: number,
  endLat: number,
  numSamples: number,
): Promise<ProfileSampleResult | null> {
  if (!isTauri()) return null;
  return invoke<ProfileSampleResult>("sample_profile", {
    path,
    startLon,
    startLat,
    endLon,
    endLat,
    numSamples,
  });
}

// ──────────────────────────────────────────────────────────────────
// Mining — Phase 1 Mining MVP commands

export interface DroneImageRpc {
  filename: string;
  longitude: number;
  latitude: number;
  altitude: number;
  yaw: number;
  pitch: number;
  roll: number;
  timestamp: number;
}

export interface DroneManifestRpc {
  source: string;
  format: string;
  image_count: number;
  bounds: [number, number, number, number] | null; // min_lon, min_lat, max_lon, max_lat
  min_altitude: number;
  max_altitude: number;
  drone_model: string | null;
  camera_model: string | null;
  images: DroneImageRpc[];
}

export interface BenchVolumeRpc {
  z_min: number;
  z_max: number;
  fill_volume: number;
  cut_volume: number;
  net_volume: number;
  fill_cells: number;
  cut_cells: number;
}

export interface VolumeResultRpc {
  fill_volume: number;
  cut_volume: number;
  net_volume: number;
  cell_area: number;
  fill_cells: number;
  cut_cells: number;
  benches: BenchVolumeRpc[];
}

/** Parse a drone manifest (.mrk / .json / .csv) → image metadata. */
export async function parseDroneManifest(
  path: string,
): Promise<DroneManifestRpc | null> {
  if (!isTauri()) return null;
  return invoke<DroneManifestRpc>("parse_drone_manifest", { path });
}

/**
 * Compute fill/cut volumes by differencing two DEM surfaces.
 * Reference path can be either a GeoTIFF path or "flat:Z" for a flat
 * plane at elevation Z (useful for stockpile-to-base-plane volumes).
 */
export async function computeVolumes(
  currentPath: string,
  referencePath: string,
  benchInterval: number,
): Promise<VolumeResultRpc | null> {
  if (!isTauri()) return null;
  return invoke<VolumeResultRpc>("compute_volumes_cmd", {
    request: {
      current_path: currentPath,
      reference_path: referencePath,
      benchInterval,
    },
  });
}

// ──────────────────────────────────────────────────────────────────
// CSF point cloud ground classification

export interface CsfParams {
  cloth_resolution: number;
  classification_threshold: number;
  max_iterations: number;
  rigidness: number;
  time_step: number;
  cloth_init_offset: number;
}

export interface CsfResult {
  point_count: number;
  ground_count: number;
  non_ground_count: number;
  is_ground: boolean[];
  iterations_run: number;
  cloth_dims: [number, number];
  cloth_z_min: number;
  cloth_z_max: number;
}

/** Run CSF ground extraction on a LAS point cloud. */
export async function classifyGround(
  path: string,
  params: CsfParams,
  maxPoints?: number,
): Promise<CsfResult | null> {
  if (!isTauri()) return null;
  return invoke<CsfResult>("classify_ground", {
    path,
    params,
    maxPoints: maxPoints ?? null,
  });
}

// ──────────────────────────────────────────────────────────────────
// ODM (OpenDroneMap) subprocess integration

export interface OdmConfig {
  image: string;
  images_dir: string;
  output_dir: string | null;
  max_concurrency: number;
  feature_quality: string;
  skip_3dmodel: boolean;
  pc_type: string;
}

export interface OdmCheckResult {
  docker_available: boolean;
  image_pulled: boolean;
  image_name: string;
}

export interface OdmRunStatus {
  phase: string;
  last_log_line: string;
  output_las_path: string | null;
  error: string | null;
  running: boolean;
}

/** Check if Docker + ODM image are available. */
export async function checkOdmAvailability(
  image?: string,
): Promise<OdmCheckResult | null> {
  if (!isTauri()) return null;
  return invoke<OdmCheckResult>("check_odm_availability", {
    image: image ?? null,
  });
}

/** Run the ODM pipeline. Listen to 'odm://progress' events for updates. */
export async function runOdmPipeline(
  config: OdmConfig,
): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string>("run_odm_pipeline", { config });
}

/** Get the latest ODM status (for refreshing after window reload). */
export async function getOdmStatus(): Promise<OdmRunStatus | null> {
  if (!isTauri()) return null;
  return invoke<OdmRunStatus>("get_odm_status");
}

// ──────────────────────────────────────────────────────────────────
// Coordinate reprojection (geo feature flag)

export interface Coord {
  x: number;
  y: number;
  z: number | null;
}

export type TransformMethod = "proj" | "identity" | "unavailable";

export interface TransformResult {
  coords: Coord[];
  from_crs: string;
  to_crs: string;
  method: TransformMethod;
}

/** Check if real PROJ-backed reprojection is available in this build. */
export async function isProjAvailable(): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("is_proj_available");
}

/** Transform a batch of coordinates from one CRS to another. */
export async function transformCoords(
  coords: Coord[],
  fromCrs: string,
  toCrs: string,
): Promise<TransformResult | null> {
  if (!isTauri()) return null;
  return invoke<TransformResult>("transform_coords_cmd", {
    coords,
    fromCrs,
    toCrs,
  });
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
