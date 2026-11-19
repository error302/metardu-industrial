/**
 * Tauri IPC wrapper for MetaRDU Industrial.
 *
 * Provides typed access to Rust commands exposed via `invoke()`.
 * Falls back to browser-mode stubs when `window.__TAURI_INTERNALS__` is
 * absent so the frontend can run via `npm run dev` without the Rust core
 * compiled in.
 */

import { invoke, isTauri, Channel } from "@tauri-apps/api/core";

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
    throw new Error("Survey file inspection requires the native desktop application.");
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
  /** Cells skipped because either surface was NODATA. QC signal. */
  nodata_cells: number;
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

/** Read LAS point data (x, y, z tuples) for point cloud rendering (JSON path). */
export async function readLasPoints(
  path: string,
  maxPoints: number,
): Promise<[number, number, number][] | null> {
  if (!isTauri()) return null;
  return invoke<[number, number, number][]>("read_las_points_cmd", {
    path,
    maxPoints,
  });
}

/**
 * Read LAS points as packed binary (f32 LE) for high-performance rendering.
 * Returns ArrayBuffer: [x0, y0, z0, x1, y1, z1, ...] — 12 bytes/point.
 * 1M points = 12MB vs 40MB JSON. Zero GC pressure.
 */
export async function readLasPointsBinary(
  path: string,
  maxPoints: number,
): Promise<Uint8Array | null> {
  if (!isTauri()) return null;
  const bytes = await invoke<number[]>("read_las_points_binary", { path, maxPoints });
  if (!bytes) return null;
  return new Uint8Array(bytes);
}

/** A chunk of LAS point data received during streaming reads. */
export interface LasPointsChunk {
  /** Flattened xyz coordinates: [x0, y0, z0, x1, y1, z1, ...] */
  points: number[];
  /** Number of points in this chunk */
  count: number;
  /** Total points read so far (across all chunks) */
  total_read: number;
  /** Total points in the file (from the LAS header) */
  total_points: number;
}

/**
 * Read LAS points in chunks via a Tauri Channel. This is the streaming
 * variant of `readLasPointsBinary` — instead of loading the entire
 * file into memory (which can OOM on 100M+ point files), it sends
 * ~65K points at a time. The `onChunk` callback is called for each
 * batch, allowing progressive rendering.
 *
 * `maxPoints=0` means "read all points".
 *
 * Returns the total number of points read.
 */
export async function readLasPointsStreaming(
  path: string,
  maxPoints: number,
  onChunk: (chunk: LasPointsChunk) => void,
): Promise<number | null> {
  if (!isTauri()) return null;
  const channel = new Channel<LasPointsChunk>();
  channel.onmessage = onChunk;
  return invoke<number>("read_las_points_streaming_cmd", {
    path,
    maxPoints,
    onChunk: channel,
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
// Marine — Phase 2 Marine MVP (CUBE, TPU, S-44)

export interface SoundingRpc {
  x: number;
  y: number;
  depth: number;
  uncertainty: number;
}

export interface CubeParams {
  resolution: number;
  capture_distance: number;
  init_uncertainty: number;
  max_hypotheses: number;
  min_soundings: number;
}

export interface CubeSurfaceRpc {
  dims: [number, number];
  resolution: number;
  bounds: [number, number, number, number];
  depths: number[];
  uncertainties: number[];
  sounding_counts: number[];
  hypothesis_counts: number[];
  total_soundings: number;
  valid_cells: number;
  ambiguous_cells: number;
}

export interface TpuComponents {
  beam_angle_sigma: number;
  range_sigma: number;
  attitude_roll_sigma: number;
  attitude_pitch_sigma: number;
  attitude_yaw_sigma: number;
  attitude_heave_sigma: number;
  attitude_latency_sigma: number;
  svp_sigma: number;
  tide_sigma: number;
  datum_sigma: number;
}

export interface SoundingTpuInput {
  beam_angle: number;
  travel_time: number;
  sound_speed: number;
  depth: number;
  components: TpuComponents;
}

export interface TpuContributions {
  sensor_variance: number;
  attitude_variance: number;
  svp_variance: number;
  tide_variance: number;
  datum_variance: number;
}

export interface TpuResult {
  vertical_tpu_1sigma: number;
  vertical_tpu_95: number;
  horizontal_tpu_1sigma: number;
  horizontal_tpu_95: number;
  vertical_contributions: TpuContributions;
  horizontal_contributions: TpuContributions;
}

export type S44Order = "exclusive" | "special" | "order_1a" | "order_1b" | "order_2";

export interface S44CheckInput {
  depth: number;
  vertical_tpu_95: number;
  horizontal_tpu_95: number;
}

export type S44Status = "pass" | "investigate" | "fail";

export interface S44Failure {
  index: number;
  depth: number;
  vertical_tpu_95: number;
  vertical_threshold: number;
  horizontal_tpu_95: number;
  horizontal_threshold: number;
  violation: "vertical" | "horizontal" | "both";
}

export interface S44ComplianceResult {
  target_order: S44Order;
  total_soundings: number;
  passing_soundings: number;
  failing_soundings: number;
  pass_rate: number;
  status: S44Status;
  is_compliant: boolean[];
  vertical_margins: number[];
  horizontal_margins: number[];
  min_depth: number;
  max_depth: number;
  mean_depth: number;
  worst_failures: S44Failure[];
}

/** Generate a CUBE bathymetric surface from soundings. */
export async function generateCubeSurface(
  soundings: SoundingRpc[],
  params: CubeParams,
): Promise<CubeSurfaceRpc | null> {
  if (!isTauri()) return null;
  return invoke<CubeSurfaceRpc>("generate_cube_surface_cmd", { soundings, params });
}

/** Compute TPU for a batch of soundings. */
export async function computeTpuBatch(
  soundings: SoundingTpuInput[],
): Promise<TpuResult[] | null> {
  if (!isTauri()) return null;
  return invoke<TpuResult[]>("compute_tpu_batch", { soundings });
}

/** Check S-44 compliance for a batch of soundings. */
export async function checkS44Compliance(
  soundings: S44CheckInput[],
  targetOrder: S44Order,
): Promise<S44ComplianceResult | null> {
  if (!isTauri()) return null;
  return invoke<S44ComplianceResult>("check_s44_compliance_cmd", {
    request: {
      soundings,
      targetOrder,
    },
  });
}

// ──────────────────────────────────────────────────────────────────
// S-57 ENC export

export type S57ObjectClass =
  | "WRECKS"
  | "OBSTRN"
  | "UWTROC"
  | "DEPARE"
  | "SOUNDG"
  | "COALNE"
  | "LNDARE";

export interface S57Attribute {
  label: string;
  value: string;
}

export type S57Geometry =
  | { type: "point"; longitude: number; latitude: number }
  | { type: "line"; coordinates: [number, number][] }
  | { type: "polygon"; coordinates: [number, number][] };

export interface S57Feature {
  object_class: S57ObjectClass;
  geometry: S57Geometry;
  attributes: S57Attribute[];
}

/** Export features to an S-57 .000 file. */
export async function exportS57(
  features: S57Feature[],
  path: string,
): Promise<boolean> {
  if (!isTauri()) return false;
  await invoke<void>("export_s57", { features, path });
  return true;
}

// ──────────────────────────────────────────────────────────────────
// 4D Monitoring — Phase 3

export interface Monitoring4DParams {
  cell_area: number;
  density: number;
  hotspot_threshold: number;
  active_threshold: number;
}

export type ChangeZone = "fill" | "cut" | "stable" | "no_data";

export interface DiffSummary {
  total_fill_volume: number;
  total_cut_volume: number;
  net_volume: number;
  total_fill_tonnage: number;
  total_cut_tonnage: number;
  net_tonnage: number;
  fill_cells: number;
  cut_cells: number;
  stable_cells: number;
  nodata_cells: number;
  active_cells: number;
  max_fill: number;
  max_cut: number;
  mean_dz: number;
  rms_dz: number;
}

export interface EpochDiff {
  dz: number[];
  volume_delta: number[];
  tonnage_delta: number[];
  zones: ChangeZone[];
  summary: DiffSummary;
  hotspots: number[];
}

export interface EpochSummary {
  epoch: number;
  fill_volume: number;
  cut_volume: number;
  net_volume: number;
  fill_tonnage: number;
  cut_tonnage: number;
}

export interface ProgressionReport {
  epochs: EpochSummary[];
  cumulative_fill: number;
  cumulative_cut: number;
  cumulative_net: number;
  cumulative_tonnage: number;
  max_single_epoch_change: number;
}

export async function computeEpochDiff(
  previousPath: string,
  currentPath: string,
  params: Monitoring4DParams,
): Promise<EpochDiff | null> {
  if (!isTauri()) return null;
  return invoke<EpochDiff>("compute_epoch_diff_cmd", {
    request: { previousPath, currentPath, params },
  });
}

export async function computeProgression(
  paths: string[],
  params: Monitoring4DParams,
): Promise<ProgressionReport | null> {
  if (!isTauri()) return null;
  return invoke<ProgressionReport>("compute_progression_cmd", {
    request: { paths, params },
  });
}

// ──────────────────────────────────────────────────────────────────
// ML Classification — Phase 3

export interface BackscatterFeatures {
  mean_intensity: number;
  std_intensity: number;
  angular_slope: number;
  angular_curvature: number;
  texture_homogeneity: number;
  depth: number;
}

export type HabitatClass = "rock" | "coarse_sediment" | "sand" | "mud" | "mixed";

export interface HabitatClassificationResult {
  class: HabitatClass;
  confidence: number;
  class_probabilities: number[];
}

export type FragmentationQuality = "excellent" | "acceptable" | "coarse" | "very_coarse";

export interface FragmentationResult {
  p20: number;
  p50: number;
  p80: number;
  p90: number;
  uniformity: number;
  mean_size: number;
  quality: FragmentationQuality;
}

export async function classifyHabitat(
  features: BackscatterFeatures,
): Promise<HabitatClassificationResult | null> {
  if (!isTauri()) return null;
  return invoke<HabitatClassificationResult>("classify_habitat_cmd", { features });
}

export async function analyzeFragmentation(
  sizesMm: number[],
): Promise<FragmentationResult | null> {
  if (!isTauri()) return null;
  return invoke<FragmentationResult>("analyze_fragmentation_cmd", {
    request: { sizes_mm: sizesMm },
  });
}

// ──────────────────────────────────────────────────────────────────
// Plugin SDK — Phase 3

export interface PluginInfo {
  name: string;
  version: string;
  vendor: string;
  description: string;
  capabilities: string[];
}

export async function listPlugins(): Promise<PluginInfo[]> {
  if (!isTauri()) return [];
  return invoke<PluginInfo[]>("list_plugins");
}

export async function getPluginExtensions(): Promise<string[]> {
  if (!isTauri()) return [];
  return invoke<string[]>("get_supported_extensions");
}

// ──────────────────────────────────────────────────────────────────
// Automation — Phase 3 (pipelines, watch folders, scheduled jobs)

export type PipelineAction =
  | "odm_pipeline"
  | "classify_ground"
  | "compute_volumes"
  | "generate_report"
  | "probe_file"
  | "generate_cube_surface"
  | "check_s44_compliance"
  | "export_s57"
  | "compute_epoch_diff"
  | "shell_command"
  | "noop";

export interface PipelineStep {
  id: string;
  action: PipelineAction;
  params: Record<string, unknown>;
  outputs: Record<string, string>;
}

export interface Pipeline {
  name: string;
  description: string;
  steps: PipelineStep[];
  watch_folders?: string[];
  schedule?: string | null;
}

export type PipelineStatus = "running" | "complete" | "failed" | "skipped";

export interface StepResult {
  id: string;
  action: PipelineAction;
  status: PipelineStatus;
  elapsed_seconds: number;
  outputs: Record<string, unknown>;
  error: string | null;
  log_lines: string[];
}

export interface PipelineRunResult {
  pipeline_name: string;
  status: PipelineStatus;
  steps: StepResult[];
  elapsed_seconds: number;
  error: string | null;
}

export interface WatchFolder {
  id: string;
  path: string;
  pipeline_name: string;
  extensions: string[];
  active: boolean;
  poll_interval_secs: number;
}

export interface WatchFolderStatus {
  id: string;
  path: string;
  pipeline_name: string;
  active: boolean;
  files_detected: number;
  pipelines_triggered: number;
  last_check: string | null;
  last_file: string | null;
  pending_files: string[];
}

export interface ScheduledJob {
  id: string;
  name: string;
  pipeline_name: string;
  interval_secs: number;
  active: boolean;
  params: Record<string, unknown>;
}

export interface ScheduledJobStatus {
  id: string;
  name: string;
  pipeline_name: string;
  active: boolean;
  interval_secs: number;
  runs_completed: number;
  last_run: string | null;
  next_run: string | null;
}

export async function parsePipelineCmd(yaml: string): Promise<Pipeline | null> {
  if (!isTauri()) return null;
  return invoke<Pipeline>("parse_pipeline_cmd", { yaml });
}

export async function runPipelineCmd(
  pipeline: Pipeline,
  input: Record<string, unknown>,
): Promise<PipelineRunResult | null> {
  if (!isTauri()) return null;
  return invoke<PipelineRunResult>("run_pipeline_cmd", { pipeline, input });
}

export async function addWatchFolder(folder: WatchFolder): Promise<boolean> {
  if (!isTauri()) return false;
  await invoke<void>("add_watch_folder", { folder });
  return true;
}

export async function removeWatchFolder(id: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("remove_watch_folder", { id });
}

export async function listWatchFolders(): Promise<WatchFolderStatus[]> {
  if (!isTauri()) return [];
  return invoke<WatchFolderStatus[]>("list_watch_folders");
}

export async function scanWatchFolders(): Promise<[string, string, string][]> {
  if (!isTauri()) return [];
  return invoke<[string, string, string][]>("scan_watch_folders");
}

export async function addScheduledJob(job: ScheduledJob): Promise<boolean> {
  if (!isTauri()) return false;
  await invoke<void>("add_scheduled_job", { job });
  return true;
}

export async function removeScheduledJob(id: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("remove_scheduled_job", { id });
}

export async function listScheduledJobs(): Promise<ScheduledJobStatus[]> {
  if (!isTauri()) return [];
  return invoke<ScheduledJobStatus[]>("list_scheduled_jobs");
}

// ──────────────────────────────────────────────────────────────────
// Branded PDF Report Engine

export type ReportType =
  | "eom_reconciliation"
  | "dredge_audit"
  | "s44_compliance"
  | "stockpile_audit"
  | "blast_report"
  | "highwall_report"
  | "deliverable_package"
  | "cross_section"
  | "generic";

// ──────────────────────────────────────────────────────────────────
// Sprint 4 — Dredge pay-volume audit (Revenue Feature #2)

export type DredgeCategory =
  | "pay"
  | "allowable_overdredge"
  | "excessive_overdredge"
  | "shoaling"
  | "no_change";

export interface DredgeCell {
  category: DredgeCategory;
  row: number;
  col: number;
  post_depth: number;
  design_depth: number;
  removed: number;
}

export interface DredgeVolumeResult {
  pay_volume: number;
  allowable_overdredge: number;
  excessive_overdredge: number;
  shoaling: number;
  total_paid: number;
  pay_cells: number;
  allowable_cells: number;
  excessive_cells: number;
  shoaling_cells: number;
  no_change_cells: number;
  cell_area: number;
  cells: DredgeCell[];
  tolerance_m: number;
  avg_dredge_depth: number;
  max_excessive: number;
}

export interface DredgeAuditRequest {
  postPath: string;
  prePath: string;
  designPath: string;
  toleranceM: number;
}

/** Compute the four-bucket dredge pay-volume breakdown. */
export async function computeDredgeAudit(
  request: DredgeAuditRequest,
): Promise<DredgeVolumeResult | null> {
  if (!isTauri()) return null;
  return invoke<DredgeVolumeResult>("compute_dredge_audit_cmd", { request });
}

// ──────────────────────────────────────────────────────────────────
// Sprint 5 — Highwall deformation monitoring (Revenue Feature #6)

export type AlertLevel = "none" | "advisory" | "watch" | "critical";
export type TrendClass = "stable" | "creeping" | "accelerating" | "failure_imminent";

export interface HighwallThresholds {
  advisory_mm: number;
  watch_mm: number;
  critical_mm: number;
  velocity_watch_mm_per_day: number;
  velocity_critical_mm_per_day: number;
}

export interface CellTimeSeries {
  index: number;
  row: number;
  col: number;
  displacements_mm: number[];
  velocities_mm_per_day: number[];
  cumulative_mm: number;
  peak_velocity_mm_per_day: number;
  acceleration_mm_per_day2: number;
  alert: AlertLevel;
  trend: TrendClass;
}

export interface HighwallAlert {
  cell_index: number;
  row: number;
  col: number;
  level: AlertLevel;
  cumulative_mm: number;
  velocity_mm_per_day: number;
  trend: TrendClass;
  message: string;
}

export interface HighwallStats {
  stable_cells: number;
  advisory_cells: number;
  watch_cells: number;
  critical_cells: number;
  max_cumulative_mm: number;
  max_velocity_mm_per_day: number;
  mean_cumulative_mm: number;
  cells_with_acceleration: number;
  failure_imminent_cells: number;
  compliance_pct: number;
}

export interface HighwallReport {
  n_epochs: number;
  cell_area_m2: number;
  total_cells: number;
  active_cells: number;
  cells: CellTimeSeries[];
  alerts: HighwallAlert[];
  stats: HighwallStats;
  thresholds: HighwallThresholds;
  epoch_dates: string[];
}

export interface HighwallRequest {
  paths: string[];
  epochDates: string[];
  cellAreaM2?: number;
  thresholds?: HighwallThresholds;
}

/** Run the highwall deformation analysis across N epoch DEMs. */
export async function analyzeHighwall(
  request: HighwallRequest,
): Promise<HighwallReport | null> {
  if (!isTauri()) return null;
  return invoke<HighwallReport>("analyze_highwall_cmd", { request });
}

// ──────────────────────────────────────────────────────────────────
// Sprint 5 — Cross-section profiler (Revenue Feature #8)

export interface Point2D {
  x: number;
  y: number;
}

export interface CrossSectionRequest {
  centerline: Point2D[];
  spacing_m: number;
  half_width_m: number;
  sample_resolution_m: number;
  surveyPath: string;
  designPath?: string;
  designDepth?: number;
}

export interface CrossSectionPoint {
  offset_m: number;
  chainage_m: number;
  survey_z: number;
  design_z: number;
}

export interface CrossSection {
  index: number;
  chainage_m: number;
  center: Point2D;
  points: CrossSectionPoint[];
  under_dredge_area: number;
  over_dredge_area: number;
  max_under_dredge: number;
  has_under_dredge: boolean;
}

export interface CrossSectionSummary {
  total_under_dredge_area: number;
  total_over_dredge_area: number;
  max_under_dredge_depth: number;
  sections_with_under_dredge: number;
  compliant_sections: number;
  compliance_pct: number;
}

export interface CrossSectionReport {
  total_length_m: number;
  n_sections: number;
  spacing_m: number;
  half_width_m: number;
  sections: CrossSection[];
  summary: CrossSectionSummary;
}

/** Compute cross-sections perpendicular to a drawn centerline. */
export async function computeCrossSections(
  request: CrossSectionRequest,
): Promise<CrossSectionReport | null> {
  if (!isTauri()) return null;
  return invoke<CrossSectionReport>("compute_cross_sections_cmd", { request });
}

// ──────────────────────────────────────────────────────────────────
// Sprint 5 — Survey Deliverable Package (Revenue Feature #7)

export type DeliverableFileType =
  | "geotiff"
  | "s57"
  | "s44_pdf"
  | "metadata_xml"
  | "track_plot"
  | "tide_log"
  | "screenshot"
  | "other";

export interface DeliverableMetadata {
  vessel: string;
  sonar: string;
  surveyArea: string;
  surveyDate: string;
  epsg: string;
  clientName: string;
  surveyorName: string;
}

export interface DeliverableSource {
  description: string;
  path: string;
  fileType: DeliverableFileType;
}

export interface DeliverablePackageRequest {
  outputPath: string;
  projectName: string;
  metadata: DeliverableMetadata;
  sources: DeliverableSource[];
  mapScreenshotB64?: string;
}

export interface BundledFile {
  description: string;
  file_type: DeliverableFileType;
  archive_path: string;
  size_bytes: number;
  sha256_short: string;
  bundled: boolean;
  error?: string;
}

export interface DeliverablePackageResult {
  outputPath: string;
  file_count: number;
  total_size_bytes: number;
  zip_size_bytes: number;
  files: BundledFile[];
  manifest_html: string;
  metadata_xml: string;
  warnings: string[];
}

/** Generate a survey deliverable package ZIP with manifest + metadata. */
export async function generateDeliverablePackage(
  request: DeliverablePackageRequest,
): Promise<DeliverablePackageResult | null> {
  if (!isTauri()) return null;
  return invoke<DeliverablePackageResult>("generate_deliverable_package_cmd", { request });
}

export interface ReportTable {
  title: string;
  headers: string[];
  rows: string[][];
}

export interface ReportStat {
  label: string;
  value: string;
  unit: string;
  color?: string;
}

export interface ReportSpec {
  report_type: ReportType;
  title: string;
  subtitle?: string;
  client?: string;
  metadata?: Record<string, string>;
  /**
   * Datum + epoch note shown in the report footer + a prominent compliance
   * strip near the top. For survey plans this is a legal compliance field.
   * Format with `formatDatumNote(epsg)` from `src/lib/crs-quickpicks.ts`.
   */
  datum_note?: string;
  /**
   * CRIRSCO-aligned reporting code: "JORC" (AU), "SAMREC" (ZA),
   * "CIM" (CA), "SME" (US), "PERC" (EU), or a custom string.
   */
  reporting_code?: string;
  /** Jurisdiction tag, e.g. "Australia — NSW" or "South Africa — offshore". */
  jurisdiction?: string;
  tables?: ReportTable[];
  summary?: ReportStat[];
  map_screenshot?: string;
  provenance_hash?: string;
  output_path: string;
}

/** Generate a branded HTML report (print-ready for PDF conversion). */
export async function generateReport(spec: ReportSpec): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string>("generate_report_cmd", { spec });
}

// ──────────────────────────────────────────────────────────────────
// Streaming + Distributed — Phase 4

export interface StreamConfig {
  port: number;
  buffer_size: number;
  flush_interval_ms: number;
  format: "json" | "km_binary" | "raw";
}

export interface StreamStatus {
  is_running: boolean;
  pings_received: number;
  pings_buffered: number;
  bytes_received: number;
  elapsed_seconds: number;
  pings_per_second: number;
  last_error: string | null;
}

export interface ServerStatus {
  is_running: boolean;
  port: number;
  workers_connected: number;
  pending_chunks: number;
  in_progress_chunks: number;
  completed_chunks: number;
  progress: number;
}

export async function startStream(config: StreamConfig): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("start_stream_cmd", { config });
}

export async function stopStream(): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("stop_stream_cmd");
}

export async function getStreamStatus(): Promise<StreamStatus | null> {
  if (!isTauri()) return null;
  return invoke<StreamStatus>("get_stream_status_cmd");
}

export async function startCoordinator(port: number): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("start_coordinator_cmd", { port });
}

export async function stopCoordinator(): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("stop_coordinator_cmd");
}

export async function getCoordinatorStatus(): Promise<ServerStatus | null> {
  if (!isTauri()) return null;
  return invoke<ServerStatus>("get_coordinator_status_cmd");
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

// ──────────────────────────────────────────────────────────────────
// Sprint 6 — SSS Waterfall Viewer + 3D Slice Editor

// SSS (Side-Scan Sonar) — XTF parser
export interface XtfHeader {
  magic: string;
  file_format_version: number;
  system_type: number;
  sonar_name: string;
  n_channels: number;
  total_ping_count_hint: number;
}

export interface SssPing {
  ping_number: number;
  timestamp_secs: number;
  latitude: number;
  longitude: number;
  heading_deg: number;
  altitude_m: number;
  sound_speed_mps: number;
  port_samples: number[];      // u8 backscatter values
  starboard_samples: number[];
  sample_interval_secs: number;
}

export interface SssData {
  header: XtfHeader;
  pings: SssPing[];
  max_samples_per_channel: number;
  total_pings: number;
}

export interface ReadSssRequest {
  path: string;
  maxPings?: number;
}

/** Read SSS XTF pings for the waterfall viewer. */
export async function readSssPings(
  request: ReadSssRequest,
): Promise<SssData | null> {
  if (!isTauri()) return null;
  return invoke<SssData>("read_sss_pings_cmd", { request });
}

export interface TargetHeightRequest {
  fishAltitudeM: number;
  slantRangeToTargetM: number;
  shadowLengthM: number;
}

/** Compute target height from shadow length (similar-triangles method). */
export async function computeTargetHeight(
  request: TargetHeightRequest,
): Promise<number> {
  if (!isTauri()) return 0;
  return invoke<number>("compute_target_height_cmd", { request });
}

// 3D Slice Editor
export interface Point2D {
  x: number;
  y: number;
}

export interface Point3D {
  x: number;
  y: number;
  z: number;
}

export interface SliceRequest {
  path: string;
  polygon: Point2D[];
  maxPoints?: number;
}

export interface SliceResult {
  indices: number[];
  points: Point3D[];
  total_points: number;
  slice_points: number;
  polygon_area_m2: number;
}

export interface RejectMask {
  rejected: number[];      // HashSet<u32> serializes as Vec<u32>
  undo_stack: number[][];
}

export interface BrushRejectRequest {
  points: Point3D[];
  center_x: number;
  center_y: number;
  center_z: number;
  radius_m: number;
  mask: RejectMask;
  restore: boolean;
}

export interface BrushResult {
  mask: RejectMask;
  toggled_count: number;
  total_rejected: number;
}

/** Slice a LAS point cloud by a 2D polygon. */
export async function sliceByPolygon(
  request: SliceRequest,
): Promise<SliceResult | null> {
  if (!isTauri()) return null;
  return invoke<SliceResult>("slice_by_polygon_cmd", { request });
}

/** Apply a brush stroke (reject or restore) to the slice. */
export async function brushReject(
  request: BrushRejectRequest,
): Promise<BrushResult | null> {
  if (!isTauri()) return null;
  return invoke<BrushResult>("brush_reject_cmd", { request });
}

/** Undo the most recent brush operation. */
export async function undoBrush(
  mask: RejectMask,
): Promise<BrushResult | null> {
  if (!isTauri()) return null;
  return invoke<BrushResult>("undo_brush_cmd", { mask });
}

/** Get accepted (non-rejected) point indices from a mask. */
export async function acceptedIndices(
  mask: RejectMask,
  total: number,
): Promise<number[]> {
  if (!isTauri()) return [];
  return invoke<number[]>("accepted_indices_cmd", { mask, total });
}

/** Test if a point is inside a polygon (geometry helper). */
export async function pointInPolygon(
  point: Point2D,
  polygon: Point2D[],
): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("point_in_polygon_cmd", { point, polygon });
}

// ──────────────────────────────────────────────────────────────────
// Sprint 7 — License Manager + Telemetry + Benchmarks

// License Manager
export type LicenseTier = "core" | "pro" | "enterprise" | "trial";

export interface LicensePayload {
  customer: string;
  tier: LicenseTier;
  expiry: string;
  seats: number;
  features: string[];
  license_id: string;
  issued: string;
  issuer: string;
}

export interface LicenseStatus {
  valid: boolean;
  tier: LicenseTier;
  payload: LicensePayload | null;
  days_remaining: number | null;
  expired: boolean;
  error: string | null;
  unlocked_features: string[];
}

/** Get the current license status. */
export async function getLicenseStatus(
  licensePath?: string,
): Promise<LicenseStatus> {
  if (!isTauri()) {
    return {
      valid: false,
      tier: "core",
      payload: null,
      days_remaining: null,
      expired: false,
      error: null,
      unlocked_features: [],
    };
  }
  return invoke<LicenseStatus>("get_license_status_cmd", { licensePath: licensePath ?? null });
}

/** Activate a license from pasted content. */
export async function activateLicense(
  licenseContent: string,
  savePath?: string,
): Promise<LicenseStatus | null> {
  if (!isTauri()) return null;
  return invoke<LicenseStatus>("activate_license_cmd", {
    licenseContent,
    savePath: savePath ?? null,
  });
}

/** Generate a license file (admin tool). */
export async function generateLicense(
  payload: LicensePayload,
): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string>("generate_license_cmd", { payload });
}

/** Check if a feature is unlocked by the current license. */
export async function checkFeature(
  feature: string,
  licensePath?: string,
): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("check_feature_cmd", {
    feature,
    licensePath: licensePath ?? null,
  });
}

// Telemetry + Crash Reporter
export interface TelemetryConfig {
  enabled: boolean;
  crash_auto_submit: boolean;
  endpoint_url: string;
  anonymous_id: string;
  app_version: string;
}

export interface TelemetryEvent {
  timestamp_ms: number;
  event_type: string;
  event_name: string;
  duration_ms: number | null;
  success: boolean;
  error: string | null;
  license_tier: string;
}

export interface CrashDump {
  crash_id: string;
  timestamp_ms: number;
  app_version: string;
  os_info: string;
  command: string;
  message: string;
  stack_trace: string;
  license_tier: string;
  anonymous_id: string;
  submitted: boolean;
}

export interface TelemetryStats {
  total_events: number;
  total_crashes: number;
  pending_crashes: number;
  top_commands: [string, number][];
  top_failures: [string, number][];
  avg_ipc_duration_ms: number;
  uptime_seconds: number;
}

/** Initialize telemetry at app startup. */
export async function initTelemetry(config: TelemetryConfig): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("init_telemetry_cmd", { config });
}

/** Update the telemetry config. */
export async function updateTelemetryConfig(config: TelemetryConfig): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("update_telemetry_config_cmd", { config });
}

/** Get the current telemetry config. */
export async function getTelemetryConfig(): Promise<TelemetryConfig | null> {
  if (!isTauri()) return null;
  return invoke<TelemetryConfig>("get_telemetry_config_cmd");
}

/** Record a telemetry event. */
export async function recordTelemetryEvent(
  eventType: string,
  eventName: string,
  durationMs: number | null,
  success: boolean,
  error: string | null,
  licenseTier: string,
): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("record_telemetry_event_cmd", {
    eventType,
    eventName,
    durationMs,
    success,
    error,
    licenseTier,
  });
}

/** Record a crash dump. */
export async function recordCrash(
  command: string,
  message: string,
  stackTrace: string,
  licenseTier: string,
): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string>("record_crash_cmd", { command, message, stackTrace, licenseTier });
}

/** Get aggregated telemetry stats. */
export async function getTelemetryStats(): Promise<TelemetryStats | null> {
  if (!isTauri()) return null;
  return invoke<TelemetryStats>("get_telemetry_stats_cmd");
}

/** Get recent telemetry events. */
export async function getRecentEvents(limit?: number): Promise<TelemetryEvent[]> {
  if (!isTauri()) return [];
  return invoke<TelemetryEvent[]>("get_recent_events_cmd", { limit: limit ?? null });
}

/** Get pending (unsubmitted) crash dumps. */
export async function getPendingCrashes(): Promise<CrashDump[]> {
  if (!isTauri()) return [];
  return invoke<CrashDump[]>("get_pending_crashes_cmd");
}

/** Mark a crash dump as submitted. */
export async function markCrashSubmitted(crashId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("mark_crash_submitted_cmd", { crashId });
}

// Performance Benchmark Suite
export interface Throughput {
  value: number;
  unit: string;
}

export interface BenchmarkResult {
  name: string;
  description: string;
  iterations: number;
  min_ms: number;
  max_ms: number;
  mean_ms: number;
  p50_ms: number;
  p95_ms: number;
  throughput: Throughput | null;
  passed: boolean;
  notes: string;
}

export interface SystemInfo {
  os: string;
  arch: string;
  cpu_count: number;
  app_version: string;
}

export interface BenchmarkSuiteResult {
  results: BenchmarkResult[];
  total_duration_secs: number;
  system_info: SystemInfo;
  overall_pass: boolean;
}

/** Run the full performance benchmark suite. */
export async function runBenchmarks(
  iterations?: number,
): Promise<BenchmarkSuiteResult | null> {
  if (!isTauri()) return null;
  return invoke<BenchmarkSuiteResult>("run_benchmarks_cmd", {
    request: { iterations: iterations ?? null },
  });
}

// ──────────────────────────────────────────────────────────────────
// Sprint 8 — Project + Updater + i18n + Marketplace

// Project File Format (.metardu)
export interface ProjectFile {
  path: string;
  kind: string;
  name: string;
  size_bytes: number;
  visible: boolean;
  color?: string;
  opacity: number;
}

export interface ViewState {
  center_lon: number;
  center_lat: number;
  zoom: number;
  rotation: number;
}

export interface CsfResultSummary {
  file_path: string;
  cloth_resolution: number;
  classifications: number;
  point_count: number;
  ground_count: number;
  elapsed_ms: number;
  slope: number;
}

export interface CubeParamsSummary {
  cell_size: number;
  iho_order: string;
  hypothesis_distance: number;
  soundings_count: number;
}

export interface MetarduProject {
  format_version: number;
  name: string;
  created: string;
  modified: string;
  default_epsg: string;
  domain: string;
  files: ProjectFile[];
  view_state: ViewState;
  csf_results: Record<string, CsfResultSummary>;
  cube_params: CubeParamsSummary | null;
  recent_reports: string[];
  layout: string;
  license_tier: string;
  pipelines: string[];
  theme: string;
  metadata: Record<string, string>;
}

export interface NewProjectRequest {
  name: string;
  defaultEpsg: string;
  domain: string;
}

export async function newProject(request: NewProjectRequest): Promise<MetarduProject | null> {
  if (!isTauri()) return null;
  return invoke<MetarduProject>("new_project_cmd", { request });
}

export async function saveProject(project: MetarduProject, path: string): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string>("save_project_cmd", { project, path });
}

export async function loadProject(path: string): Promise<MetarduProject | null> {
  if (!isTauri()) return null;
  return invoke<MetarduProject>("load_project_cmd", { path });
}

export async function addFileToProject(project: MetarduProject, file: ProjectFile): Promise<MetarduProject | null> {
  if (!isTauri()) return null;
  return invoke<MetarduProject>("add_file_to_project_cmd", { project, file });
}

export async function removeFileFromProject(project: MetarduProject, path: string): Promise<MetarduProject | null> {
  if (!isTauri()) return null;
  return invoke<MetarduProject>("remove_file_from_project_cmd", { project, path });
}

export async function updateViewState(project: MetarduProject, view: ViewState): Promise<MetarduProject | null> {
  if (!isTauri()) return null;
  return invoke<MetarduProject>("update_view_state_cmd", { project, view });
}

export async function addRecentReport(project: MetarduProject, reportPath: string): Promise<MetarduProject | null> {
  if (!isTauri()) return null;
  return invoke<MetarduProject>("add_recent_report_cmd", { project, reportPath });
}

// Auto-Updater
export interface UpdateInfo {
  available: boolean;
  latest_version: string;
  current_version: string;
  release_date: string;
  release_notes: string;
  download_url: string;
  file_size: number;
  signature: string;
}

export type UpdateState = "idle" | "checking" | "available" | "up_to_date" | "downloading" | "downloaded" | "installing" | "restart_required" | "error";

export interface UpdateStatus {
  state: UpdateState;
  last_check: string;
  info: UpdateInfo | null;
  download_progress: number;
  error: string | null;
}

export async function checkForUpdates(endpoint?: string): Promise<UpdateInfo | null> {
  if (!isTauri()) return null;
  return invoke<UpdateInfo>("check_for_updates_cmd", { endpoint: endpoint ?? null });
}

/** Download and install the latest update (if available). The plugin
 *  verifies the Ed25519 signature against the configured pubkey before
 *  installing. Returns void on success — the frontend should prompt
 *  the user to restart. */
export async function downloadAndInstallUpdate(): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("download_and_install_update_cmd");
}

export async function getUpdateStatus(): Promise<UpdateStatus | null> {
  if (!isTauri()) return null;
  return invoke<UpdateStatus>("get_update_status_cmd");
}

export async function getCurrentVersion(): Promise<string> {
  if (!isTauri()) return "0.0.0-browser";
  return invoke<string>("get_current_version_cmd");
}

// i18n
export async function translate(key: string, langCode: string): Promise<string> {
  if (!isTauri()) return key;
  return invoke<string>("translate_cmd", { key, langCode });
}

export async function getAvailableLanguages(): Promise<[string, string][]> {
  if (!isTauri()) return [["en", "English"], ["es", "Español"], ["pt", "Português"]];
  return invoke<[string, string][]>("get_available_languages_cmd");
}

// Plugin Marketplace
export interface RegistryPlugin {
  id: string;
  name: string;
  version: string;
  vendor: string;
  description: string;
  plugin_type: string;
  extensions: string[];
  download_url: string;
  sha256: string;
  file_size: number;
  min_app_version: string;
  license: string;
  homepage: string;
  official: boolean;
  downloads: number;
}

export interface PluginRegistry {
  version: number;
  name: string;
  updated: string;
  plugins: RegistryPlugin[];
}

export interface InstalledPlugin {
  id: string;
  name: string;
  version: string;
  vendor: string;
  installed_path: string;
  installed_date: string;
}

export async function fetchPluginRegistry(source: string): Promise<PluginRegistry | null> {
  if (!isTauri()) return null;
  return invoke<PluginRegistry>("fetch_plugin_registry_cmd", { source });
}

export async function listInstalledPlugins(): Promise<InstalledPlugin[]> {
  if (!isTauri()) return [];
  return invoke<InstalledPlugin[]>("list_installed_plugins_cmd");
}

export async function installPlugin(registry: PluginRegistry, pluginId: string): Promise<InstalledPlugin | null> {
  if (!isTauri()) return null;
  return invoke<InstalledPlugin>("install_plugin_cmd", { registry, pluginId });
}

export async function uninstallPlugin(pluginId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<void>("uninstall_plugin_cmd", { pluginId });
}

export async function searchRegistry(registry: PluginRegistry, query: string): Promise<RegistryPlugin[]> {
  if (!isTauri()) return [];
  return invoke<RegistryPlugin[]>("search_registry_cmd", { registry, query });
}

// ──────────────────────────────────────────────────────────────────
// Bottleneck Tools — high-value surveyor tools

// Density Gates
export type CoverageStatus = "good" | "marginal" | "gap" | "empty";

export interface CoverageCell {
  row: number;
  col: number;
  center_lon: number;
  center_lat: number;
  count: number;
  status: CoverageStatus;
}

export interface FileSummary {
  filename: string;
  pings: number;
  est_soundings: number;
  file_size_bytes: number;
}

export interface CoverageReport {
  files_scanned: number;
  total_pings: number;
  total_soundings: number;
  cells: CoverageCell[];
  bounds: [number, number, number, number];
  grid_rows: number;
  grid_cols: number;
  cell_size_deg: number;
  target_density: number;
  good_cells: number;
  marginal_cells: number;
  gap_cells: number;
  empty_cells: number;
  coverage_pct: number;
  file_summaries: FileSummary[];
  warnings: string[];
}

export interface DensityGatesRequest {
  folder_path: string;
  target_order: string;
  cell_size_deg?: number;
}

export async function runDensityGates(
  request: DensityGatesRequest,
): Promise<CoverageReport | null> {
  if (!isTauri()) return null;
  return invoke<CoverageReport>("run_density_gates_cmd", { request });
}

// Tidal Spline Interpolator
export interface TidalCorrectionRequest {
  sonar_csv_path: string;
  tide_csv_path: string;
  output_csv_path: string;
}

export interface TidalCorrectionResult {
  pings_corrected: number;
  tide_readings: number;
  min_tide_m: number;
  max_tide_m: number;
  mean_tide_m: number;
  min_corrected_depth_m: number;
  max_corrected_depth_m: number;
  output_path: string;
  warnings: string[];
}

export async function runTidalCorrection(
  request: TidalCorrectionRequest,
): Promise<TidalCorrectionResult | null> {
  if (!isTauri()) return null;
  return invoke<TidalCorrectionResult>("run_tidal_correction_cmd", { request });
}

// Machine Control Compiler
export type MachineControlVendor = "leica" | "trimble" | "topcon";

export interface MachineControlRequest {
  input_path: string;
  vendor: MachineControlVendor;
  output_path: string;
}

export interface MachineControlResult {
  vendor: MachineControlVendor;
  output_path: string;
  point_count: number;
  line_count: number;
  file_size_bytes: number;
  warnings: string[];
}

export async function compileMachineControl(
  request: MachineControlRequest,
): Promise<MachineControlResult | null> {
  if (!isTauri()) return null;
  return invoke<MachineControlResult>("compile_machine_control_cmd", { request });
}

// ──────────────────────────────────────────────────────────────────
// DEM Rendering — hillshaded color-ramp GeoTIFF visualization

export interface DemRenderRequest {
  path: string;
  azimuth?: number;
  altitude?: number;
  color_ramp?: string;
  z_scale?: number;
}

export interface DemRenderResult {
  width: number;
  height: number;
  bounds: [number, number, number, number];
  rgba: number[];
  min_z: number;
  max_z: number;
  epsg: number | null;
}

export async function renderDem(
  request: DemRenderRequest,
): Promise<DemRenderResult | null> {
  if (!isTauri()) return null;
  return invoke<DemRenderResult>("render_dem_cmd", { request });
}

// ──────────────────────────────────────────────────────────────────
// EOM Volumetric Auditor — commercial module v1

export interface CsfParamsRpc {
  cloth_resolution: number;
  classification_threshold: number;
  max_iterations: number;
  rigidness: number;
  time_step: number;
  cloth_init_offset: number;
}

export interface DemParamsRpc {
  cell_size: number;
  idw_power: number;
  search_radius_cells: number;
  min_points: number;
}

/** Design surface reference for terrain volume comparison. */
export type DesignSurfaceRef =
  | { Flat: number }
  | { Dem: { data: number[]; ncols: number; nrows: number; cell_size: number; nodata: number } };

export interface EomInputRpc {
  current_las_path: string;
  previous_las_path: string | null;
  reference_flat_elevation: number;
  csf_params: CsfParamsRpc;
  dem_params: DemParamsRpc;
  bench_interval: number;
  max_points: number;
  /** Optional design surface for terrain volume comparison. */
  design_surface?: DesignSurfaceRef | null;
  /** When true, use RANSAC auto-detected ground elevation. */
  auto_detect_baseline?: boolean;
}

export interface BenchVolumeRpc {
  z_min: number; z_max: number; fill_volume: number; cut_volume: number;
  net_volume: number; fill_cells: number; cut_cells: number;
}

export interface VolumeResultRpc {
  fill_volume: number; cut_volume: number; net_volume: number;
  cell_area: number; fill_cells: number; cut_cells: number;
  nodata_cells: number;
  benches: BenchVolumeRpc[];
}

export interface DemGridRpc {
  data: number[]; cols: number; rows: number; cell_size: number;
  origin_x: number; origin_y: number; nodata_count: number;
  z_min: number; z_max: number;
}

export interface LasHeaderRpcEom {
  file_source_id: number; global_encoding: number; version_major: number;
  version_minor: number; system_identifier: string; generating_software: string;
  file_creation_day: number; file_creation_year: number; header_size: number;
  offset_to_point_data: number; number_of_vlrs: number; point_data_format: number;
  point_data_record_length: number; point_count: number; points_by_return: number[];
  scale_x: number; scale_y: number; scale_z: number;
  offset_x: number; offset_y: number; offset_z: number;
  min_x: number; min_y: number; min_z: number;
  max_x: number; max_y: number; max_z: number;
  crs_wkt: string | null; geotiff_keys: number[] | null;
}

export interface EomOutputRpc {
  audit_hash: string;
  points_read: number;
  ground_points: number;
  non_ground_points: number;
  volumes: VolumeResultRpc;
  fill_volume: number;
  cut_volume: number;
  net_volume: number;
  cell_area: number;
  fill_cells: number;
  cut_cells: number;
  dem_cols: number;
  dem_rows: number;
  dem_cell_size: number;
  source_file: string;
  source_hash: string;
  processing_time_ms: number;
  warnings: string[];
}

export interface EomProgressRpc {
  stage: string;
  current: number;
  total: number;
  message: string;
}

export interface ReportDataRpc {
  eom_output: EomOutputRpc;
  customer: string; site: string; surveyor: string;
  report_date: string; software_version: string;
  signed: boolean;
}

export interface MachineFingerprintRpc {
  mac_address: string; cpu_brand: string; disk_serial: string; fingerprint_hash: string;
}

export interface LicenseClaimsRpc {
  license_id: string; customer: string; product: string; tier: string;
  issued_at: string; expires_at: string; fingerprint: string;
  max_seats: number | null; reports_remaining: number | null; site_id: string | null;
}

export interface LicenseFileRpc {
  claims: LicenseClaimsRpc; signature: string; key_id: string;
}

export type LicenseStatusRpc =
  | { state: "Trial"; trial_reports_remaining: number }
  | { state: "Active"; customer: string; license_id: string; tier: string; expires_at: string; reports_remaining: number | null }
  | { state: "Invalid"; reason: string }
  | { state: "Exhausted"; customer: string; license_id: string }
  | { state: "Expired"; customer: string; expired_at: string };

export interface EomWatchFolderConfigRpc {
  path: string; poll_interval_secs: number;
  csf_params: CsfParamsRpc; dem_params: DemParamsRpc;
  bench_interval: number; reference_flat_elevation: number;
  customer: string; site: string; surveyor: string;
}

export interface EomWatchEventRpc {
  kind: "started" | "completed" | "failed";
  file_path: string; report_path: string | null;
  fill_volume: number | null; cut_volume: number | null; net_volume: number | null;
  error: string | null; processing_time_ms: number | null;
}

export interface DesignDemRpc {
  data: number[]; cols: number; rows: number;
  cell_size: number; origin_x: number; origin_y: number;
}

export const DEFAULT_CSF_PARAMS: CsfParamsRpc = {
  cloth_resolution: 0.5, classification_threshold: 0.5, max_iterations: 500,
  rigidness: 2, time_step: 0.65, cloth_init_offset: 10.0,
};

export const DEFAULT_DEM_PARAMS: DemParamsRpc = {
  cell_size: 0.5, idw_power: 2.0, search_radius_cells: 3.0, min_points: 1,
};

export async function runEomPipeline(
  input: EomInputRpc,
  onProgress?: (progress: EomProgressRpc) => void,
): Promise<EomOutputRpc | null> {
  if (!isTauri()) {
    const stages: EomProgressRpc[] = [
      { stage: "hashing", current: 0, total: 1, message: "Hashing source file…" },
      { stage: "ingest", current: 0, total: 1, message: "Reading 2500 points…" },
      { stage: "csf", current: 0, total: 200, message: "Classifying ground points…" },
      { stage: "dem", current: 0, total: 1, message: "Rasterizing 50×50 DEM…" },
      { stage: "volume", current: 0, total: 1, message: "Computing cut/fill volumes…" },
      { stage: "audit", current: 0, total: 1, message: "Sealing audit hash…" },
      { stage: "done", current: 1, total: 1, message: "Pipeline complete" },
    ];
    for (const stage of stages) { onProgress?.(stage); await new Promise((r) => setTimeout(r, 500)); }
    return null;
  }
  const channel = new Channel<EomProgressRpc>();
  if (onProgress) { channel.onmessage = onProgress; }
  return invoke<EomOutputRpc>("run_eom_pipeline_cmd", { input, onProgress: channel });
}

export async function generateEomReport(
  eomOutput: EomOutputRpc,
  customer: string,
  site: string,
  surveyor: string,
  outputPath: string,
  signed: boolean,
): Promise<void> {
  if (!isTauri()) { console.log("[browser-mode] would write PDF report to", outputPath); return; }
  return invoke<void>("generate_eom_report_cmd", {
    eomOutput, customer, site, surveyor, outputPath, signed,
  });
}

export async function detectMachineFingerprint(): Promise<MachineFingerprintRpc | null> {
  if (!isTauri()) {
    return { mac_address: "00:00:00:00:00:00", cpu_brand: "browser-mode-cpu", disk_serial: "browser-mode-disk", fingerprint_hash: "0".repeat(64) };
  }
  return invoke<MachineFingerprintRpc>("detect_machine_fingerprint_cmd");
}

export async function verifyEomLicense(
  license: LicenseFileRpc,
  expectedProduct?: string,
  expectedTier?: string,
): Promise<LicenseClaimsRpc | null> {
  if (!isTauri()) {
    if (!license.claims.customer) { throw new Error("license verification failed: empty customer"); }
    return license.claims;
  }
  return invoke<LicenseClaimsRpc>("verify_eom_license_cmd", {
    license, expectedProduct: expectedProduct ?? null, expectedTier: expectedTier ?? null,
  });
}

export async function signEomLicense(claims: LicenseClaimsRpc): Promise<LicenseFileRpc | null> {
  if (!isTauri()) return null;
  return invoke<LicenseFileRpc>("sign_eom_license_cmd", { claims });
}

export async function checkLicenseStatus(license: LicenseFileRpc | null): Promise<LicenseStatusRpc> {
  if (!isTauri()) { return { state: "Trial", trial_reports_remaining: 3 }; }
  return invoke<LicenseStatusRpc>("check_license_status_cmd", { license });
}

export async function consumeReport(license: LicenseFileRpc | null): Promise<LicenseStatusRpc> {
  if (!isTauri()) { return { state: "Trial", trial_reports_remaining: 2 }; }
  return invoke<LicenseStatusRpc>("consume_report_cmd", { license });
}

export async function importDxfSurface(path: string, cellSize: number): Promise<DesignDemRpc | null> {
  if (!isTauri()) return null;
  return invoke<DesignDemRpc>("import_dxf_surface_cmd", { path, cellSize });
}

export async function startEomWatchFolder(config: EomWatchFolderConfigRpc): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("start_eom_watch_folder", { config });
}

export async function stopEomWatchFolder(): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("stop_eom_watch_folder");
}

export async function isEomWatchFolderRunning(): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("is_eom_watch_folder_running");
}

// ──────────────────────────────────────────────────────────────────
// Mission Data Triage — field data verification + gap analysis

export type TriageFileKindRpc =
  | "drone_image" | "las_pointcloud" | "laz_pointcloud"
  | "geotiff" | "gnss_rinex" | "gnss_nmea" | "unknown";

export type FileStatusRpc = "ok" | "warning" | "error" | "empty";

export interface TriageFileRpc {
  path: string;
  filename: string;
  kind: TriageFileKindRpc;
  status: FileStatusRpc;
  size_bytes: number;
  bounds: [number, number, number, number] | null;
  point_count: number | null;
  timestamp_start: number | null;
  timestamp_end: number | null;
  crs: string | null;
  error: string | null;
}

export interface CoverageGapRpc {
  center_lon: number;
  center_lat: number;
  radius_m: number;
  description: string;
}

export interface TriageReportRpc {
  files: TriageFileRpc[];
  total_files: number;
  healthy_files: number;
  warning_files: number;
  error_files: number;
  total_size_bytes: number;
  total_points: number;
  total_images: number;
  coverage_gaps: CoverageGapRpc[];
  time_span_secs: number | null;
  crs_mismatch: boolean;
  detected_crs_list: string[];
  warnings: string[];
}

/** Run triage analysis on a directory of field data files. */
export async function runTriage(dir: string): Promise<TriageReportRpc | null> {
  if (!isTauri()) return null;
  return invoke<TriageReportRpc>("run_triage_cmd", { dir });
}

// ──────────────────────────────────────────────────────────────────
// NTRIP/RTCM3 Client — RTK correction streaming

export interface NtripConfigRpc {
  host: string;
  port: number;
  mountpoint: string;
  username: string | null;
  password: string | null;
  timeout_secs: number;
  /** Use TLS (ntrips://). Encrypts the entire session including
   *  credentials. Set to true for casters that support TLS. */
  use_tls: boolean;
}

export interface NtripStatusRpc {
  connected: boolean;
  mountpoint: string;
  messages_received: number;
  bytes_received: number;
  last_message_type: number | null;
  last_error: string | null;
  uptime_secs: number;
  /** Epoch ms (Unix) of the last successfully parsed RTCM frame.
   *  Used to compute "correction age" — the #1 field-crew metric. */
  last_message_epoch_ms: number | null;
  /** Number of reconnect attempts since the last successful RTCM frame. */
  reconnect_attempts: number;
  /** True while the background thread is sleeping in backoff before retrying. */
  reconnecting: boolean;
}

/** Start NTRIP client — connects to caster and begins streaming RTCM corrections. */
export async function startNtrip(config: NtripConfigRpc): Promise<NtripStatusRpc | null> {
  if (!isTauri()) return null;
  return invoke<NtripStatusRpc>("start_ntrip_cmd", { config });
}

/** Stop the NTRIP client. */
export async function stopNtrip(): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("stop_ntrip_cmd");
}

/** Get the current NTRIP client status. */
export async function getNtripStatus(): Promise<NtripStatusRpc | null> {
  if (!isTauri()) {
    return {
      connected: false,
      mountpoint: "",
      messages_received: 0,
      bytes_received: 0,
      last_message_type: null,
      last_error: null,
      uptime_secs: 0,
      last_message_epoch_ms: null,
      reconnect_attempts: 0,
      reconnecting: false,
    };
  }
  return invoke<NtripStatusRpc>("get_ntrip_status_cmd");
}

// ──────────────────────────────────────────────────────────────────
// Phase 2 — Marine MBES, tidal datums, backscatter, QC dashboard
// ──────────────────────────────────────────────────────────────────

/** A sounding from a Kongsberg .all bathymetry datagram. */
export interface KongsbergSoundingRpc {
  timestamp: number;
  ping_number: number;
  beam_number: number;
  depth: number;
  across_track: number;
  along_track: number;
  beam_angle: number;
  heave: number;
  roll: number;
  pitch: number;
  heading: number;
  sound_speed: number;
  quality: number;
  detection_info: number;
}

export interface KongsbergPositionRpc {
  timestamp: number;
  latitude: number;
  longitude: number;
  height: number;
  quality: number;
}

export interface KongsbergAttitudeRpc {
  timestamp: number;
  roll: number;
  pitch: number;
  heave: number;
  heading: number;
}

export interface AllSurveyDataRpc {
  header: {
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
  };
  soundings: KongsbergSoundingRpc[];
  positions: KongsbergPositionRpc[];
  attitudes: KongsbergAttitudeRpc[];
  bounds: [number, number, number, number] | null;
}

/** Read a Kongsberg .all file and extract all bathymetry, position, and attitude data. */
export async function readAllSurvey(path: string, maxPings: number): Promise<AllSurveyDataRpc | null> {
  if (!isTauri()) return null;
  return invoke<AllSurveyDataRpc>("read_all_survey_cmd", { path, maxPings });
}

/** Apply a tidal datum conversion to an array of depths. */
export async function convertTidalDatum(depths: number[], offsetM: number): Promise<number[]> {
  if (!isTauri()) return depths.map(d => d + offsetM);
  return invoke<number[]>("convert_tidal_datum_cmd", { depths, offsetM });
}

/** Backscatter sample for mosaicking. */
export interface BackscatterSampleRpc {
  across_track: number;
  along_track: number;
  intensity_db: number;
  beam_angle: number;
  timestamp: number;
}

export interface MosaicParamsRpc {
  cell_size: number;
  apply_lambert_correction: boolean;
  method: string;
}

export interface BackscatterMosaicRpc {
  ncols: number;
  nrows: number;
  cell_size: number;
  bounds: [number, number, number, number];
  data: number[];
  nodata: number;
  valid_cells: number;
  min_db: number;
  max_db: number;
}

/** Create a backscatter mosaic from MBES backscatter samples. */
export async function createBackscatterMosaic(
  samples: BackscatterSampleRpc[],
  params: MosaicParamsRpc,
): Promise<BackscatterMosaicRpc | null> {
  if (!isTauri()) return null;
  return invoke<BackscatterMosaicRpc>("create_backscatter_mosaic_cmd", { samples, params });
}

/** QC statistics for a set of soundings. */
export interface QcStatsRpc {
  total_soundings: number;
  accepted_soundings: number;
  rejected_soundings: number;
  coverage_area_m2: number;
  avg_density_per_m2: number;
  min_depth: number;
  max_depth: number;
  mean_depth: number;
  std_depth: number;
  mean_beam_angle: number;
  max_beam_angle: number;
  avg_beams_per_ping: number;
  ping_count: number;
  s44_order: string;
  density_compliance_pct: number;
  uncertainty_compliance_pct: number;
}

/** Compute QC statistics for a set of soundings. */
export async function computeQcStats(
  soundings: (number | number | number | number | number | number)[][],
  cellSize: number,
  s44Order: string,
): Promise<QcStatsRpc | null> {
  if (!isTauri()) return null;
  return invoke<QcStatsRpc>("compute_qc_stats_cmd", { soundings, cellSize, s44Order });
}

// ──────────────────────────────────────────────────────────────────
// Mining surveyor tools — setout, mine grid, tunnel profile, safety
// ──────────────────────────────────────────────────────────────────

export interface SetoutPointRpc {
  id: string;
  easting: number;
  northing: number;
  elevation: number;
  description: string;
  pointType: string;
}

export interface SetoutResultRpc {
  point: SetoutPointRpc;
  bearing_deg: number;
  distance_m: number;
  delta_z: number;
  slope_distance: number;
  slope_angle_deg: number;
}

/** Compute setout (bearing, distance, slope) from a reference point to design points. */
export async function computeSetout(
  points: SetoutPointRpc[],
  refEasting: number,
  refNorthing: number,
  refElevation: number,
): Promise<SetoutResultRpc[]> {
  if (!isTauri()) return [];
  return invoke<SetoutResultRpc[]>("compute_setout_cmd", {
    points, refEasting, refNorthing, refElevation,
  });
}

export interface MineGridRpc {
  name: string;
  origin_easting: number;
  origin_northing: number;
  rotation_deg: number;
  scale_factor: number;
  parent_crs: string;
}

/** Convert mine grid coordinates to parent CRS. */
export async function mineGridToCrs(
  grid: MineGridRpc, gridEasting: number, gridNorthing: number,
): Promise<[number, number]> {
  if (!isTauri()) return [gridEasting + grid.origin_easting, gridNorthing + grid.origin_northing];
  return invoke<[number, number]>("mine_grid_to_crs_cmd", { grid, gridEasting, gridNorthing });
}

/** Convert parent CRS coordinates to mine grid. */
export async function crsToMineGrid(
  grid: MineGridRpc, crsEasting: number, crsNorthing: number,
): Promise<[number, number]> {
  if (!isTauri()) return [crsEasting - grid.origin_easting, crsNorthing - grid.origin_northing];
  return invoke<[number, number]>("crs_to_mine_grid_cmd", { grid, crsEasting, crsNorthing });
}

export interface TunnelProfileRpc {
  chainage: number;
  points: [number, number][];
  design_profile: [number, number][] | null;
}

export interface TunnelProfileResultRpc {
  area: number;
  design_area: number | null;
  overbreak: number | null;
  underbreak: number | null;
  max_width: number;
  max_height: number;
}

/** Analyze a tunnel profile: compute area, overbreak/underbreak. */
export async function analyzeTunnelProfile(profile: TunnelProfileRpc): Promise<TunnelProfileResultRpc | null> {
  if (!isTauri()) return null;
  return invoke<TunnelProfileResultRpc>("analyze_tunnel_profile_cmd", { profile });
}

/** Generate a safety inspection report. */
export async function generateSafetyReport(inspection: unknown): Promise<string> {
  if (!isTauri()) return "Browser mode — safety report requires the native Tauri shell";
  return invoke<string>("generate_safety_report_cmd", { inspection });
}

// ──────────────────────────────────────────────────────────────────
// OBN (Ocean Bottom Node) Survey Suite — 4DNav & Node Dashboard Automation
// ──────────────────────────────────────────────────────────────────

export type ToleranceStatus = "in_spec" | "warning" | "out_of_spec";

export type NodeOperationalStatus =
  | "planned"
  | "deployed"
  | "active_recording"
  | "retrieved"
  | "relocated"
  | "failed"
  | "lost";

export interface ObnPlannedNode {
  station_id: string;
  line_name: string;
  easting: number;
  northing: number;
  target_depth: number;
  remarks?: string;
}

export interface ObnAsLaidNode {
  node_id: string;
  serial_number: string;
  station_id: string;
  line_name: string;
  easting: number;
  northing: number;
  depth: number;
  timestamp_utc: string;
  usbl_uncertainty_m: number;
  water_depth_m: number;
  tilt_deg?: number;
  battery_pct?: number;
  status: NodeOperationalStatus;
}

export interface ObnQcNode {
  station_id: string;
  serial_number: string;
  line_name: string;
  planned_easting: number;
  planned_northing: number;
  planned_depth: number;
  aslaid_easting: number;
  aslaid_northing: number;
  aslaid_depth: number;
  delta_easting: number;
  delta_northing: number;
  radial_offset_m: number;
  delta_depth_m: number;
  tolerance: ToleranceStatus;
  usbl_uncertainty_m: number;
  timestamp_utc: string;
  battery_pct?: number;
  tilt_deg?: number;
  status: NodeOperationalStatus;
}

export interface ObnQcSummary {
  total_nodes: number;
  in_spec_count: number;
  warning_count: number;
  out_of_spec_count: number;
  compliance_percentage: number;
  min_radial_offset_m: number;
  max_radial_offset_m: number;
  mean_radial_offset_m: number;
  std_dev_radial_m: number;
  mean_depth_m: number;
  nodes: ObnQcNode[];
}

export interface ObnShiftDpr {
  survey_name: string;
  client_name: string;
  vessel_name: string;
  shift_date: string;
  shift_type: string;
  surveyor_name: string;
  total_planned_patch: number;
  nodes_deployed_shift: number;
  nodes_retrieved_shift: number;
  deployment_rate_per_hour: number;
  average_cycle_time_mins: number;
  operational_hours: number;
  weather_standby_hours: number;
  technical_downtime_hours: number;
  general_remarks: string;
  audit_hash: string;
}

export interface ObnHeader {
  survey_name: string;
  client_name: string;
  contractor_name: string;
  vessel_name: string;
  crs_name: string;
  epsg_code: number;
  date_str: string;
}

export interface ObnSampleDataset {
  planned_csv: string;
  aslaid_csv: string;
  default_header: ObnHeader;
}

export interface ObnQcRequest {
  plannedContent: string;
  aslaidContent: string;
  toleranceSpecM?: number;
  sourceFormat?: string;
}

/** Evaluate spatial tolerance and deltas for OBN deployment. */
export async function obnEvaluateQc(request: ObnQcRequest): Promise<ObnQcSummary> {
  if (!isTauri()) {
    // Browser mock fallback
    const sample = await obnGenerateSampleProject();
    return mockEvaluateQc(sample.planned_csv, sample.aslaid_csv, request.toleranceSpecM ?? 5.0);
  }
  return invoke<ObnQcSummary>("obn_evaluate_qc_cmd", { request });
}

/** Export evaluated nodes as UKOOA SPS Rev 2.1 fixed-width R-records. */
export async function obnExportSps(nodes: ObnQcNode[], header: ObnHeader): Promise<string> {
  if (!isTauri()) {
    return [
      `H0000 UKOOA SPS REV 2.1 - BROWSER PREVIEW`,
      `H0100 SURVEY:      ${header.survey_name}`,
      `H0200 CLIENT:      ${header.client_name}`,
      `H0500 GEODETIC CRS:${header.crs_name} (EPSG:${header.epsg_code})`,
      `H9999 ---------------- DATA RECORDS BEGIN ----------------`,
      ...nodes.map(n => `R${n.line_name.padEnd(16)}${n.station_id.padStart(8)}1ND   0   0   0   0  ${n.aslaid_easting.toFixed(1).padStart(9)}${n.aslaid_northing.toFixed(1).padStart(10)}${(-Math.abs(n.aslaid_depth)).toFixed(1).padStart(6)}274120000`)
    ].join("\n");
  }
  return invoke<string>("obn_export_sps_cmd", { request: { nodes, header } });
}

/** Export evaluated nodes as IOGP P1/11 exchange format. */
export async function obnExportP111(nodes: ObnQcNode[], header: ObnHeader): Promise<string> {
  if (!isTauri()) {
    return [
      `H0100 SURVEY:        ${header.survey_name}`,
      `H0200 CLIENT:        ${header.client_name}`,
      `H0801 PROJECTION:    ${header.crs_name} (EPSG:${header.epsg_code})`,
      `H1000 POSITION TYPE: Seabed As-Laid Node Position (USBL acoustic fix)`,
      `H9999 DATA COLUMNS:  RecordType,LineName,StationNumber,PointIndex,StationCode,Easting,Northing,Elevation,USBL_1Sigma,RadialOffset,TimestampUTC,SerialNumber,Status`,
      ...nodes.map(n => `R,${n.line_name},${n.station_id},1,ND,${n.aslaid_easting.toFixed(2)},${n.aslaid_northing.toFixed(2)},${(-Math.abs(n.aslaid_depth)).toFixed(2)},${n.usbl_uncertainty_m.toFixed(2)},${n.radial_offset_m.toFixed(2)},${n.timestamp_utc},${n.serial_number},${n.status.toUpperCase()}`)
    ].join("\n");
  }
  return invoke<string>("obn_export_p111_cmd", { request: { nodes, header } });
}

/** Generate a 12-hour shift handover summary and DPR. */
export async function obnGenerateShiftDpr(shift: ObnShiftDpr, qcSummary: ObnQcSummary): Promise<string> {
  if (!isTauri()) {
    return [
      `# Offshore OBN Survey — Shift Handover & Daily Progress Report (Browser Mode)`,
      ``,
      `| Parameter | Value |`,
      `|---|---|`,
      `| **Survey** | ${shift.survey_name} |`,
      `| **Vessel** | ${shift.vessel_name} |`,
      `| **Shift** | ${shift.shift_date} — ${shift.shift_type} |`,
      `| **Surveyor** | ${shift.surveyor_name} |`,
      ``,
      `## Operational Statistics`,
      `- **Nodes Deployed:** ${shift.nodes_deployed_shift}`,
      `- **Deployment Rate:** ${shift.deployment_rate_per_hour.toFixed(2)} nodes/hr`,
      `- **Average Cycle Time:** ${shift.average_cycle_time_mins.toFixed(1)} mins/node`,
      ``,
      `## Spatial QA/QC`,
      `- **Total Evaluated:** ${qcSummary.total_nodes}`,
      `- **In Spec (<= 5.0m):** ${qcSummary.in_spec_count} (${qcSummary.compliance_percentage.toFixed(1)}%)`,
      `- **Warning (5.0m - 10.0m):** ${qcSummary.warning_count}`,
      `- **Out of Spec (> 10.0m):** ${qcSummary.out_of_spec_count}`,
      `- **Mean Radial Offset:** ${qcSummary.mean_radial_offset_m.toFixed(2)} m`
    ].join("\n");
  }
  return invoke<string>("obn_generate_shift_dpr_cmd", { request: { shift, qcSummary } });
}

/** Generate a 40-node realistic deepwater OBN deployment demo dataset. */
export async function obnGenerateSampleProject(): Promise<ObnSampleDataset> {
  if (!isTauri()) {
    const plannedRows: string[] = ["Station_ID,Line_Name,Easting,Northing,Depth"];
    const aslaidRows: string[] = ["NodeID,SerialNo,StationID,LineName,Easting,Northing,Depth,Timestamp,USBL_Sigma,WaterDepth,Status"];
    const baseE = 542000.0;
    const baseN = 6125000.0;
    const baseZ = 1285.0;
    let count = 0;

    for (let l = 1; l <= 4; l++) {
      const lineName = `OBN_L0${l}`;
      const nCoord = baseN + (l - 1) * 200.0;
      for (let s = 1; s <= 10; s++) {
        count++;
        const stnId = `STN_${l}${s < 10 ? "0" + s : s}`;
        const eCoord = baseE + (s - 1) * 100.0;
        const depth = baseZ + s * 1.8;
        plannedRows.push(`${stnId},${lineName},${eCoord.toFixed(1)},${nCoord.toFixed(1)},${depth.toFixed(1)}`);

        let dx = 1.2 + (count % 3);
        let dy = -0.9 - (count % 2);
        let sigma = 1.2;
        if (count === 7) { dx = 7.2; dy = -3.1; sigma = 2.4; } // warning ~7.8m
        if (count === 14) { dx = 5.4; dy = 4.1; sigma = 1.9; }  // warning ~6.8m
        if (count === 23) { dx = 10.8; dy = -4.5; sigma = 3.2; } // out of spec ~11.7m

        const aslaidE = eCoord + dx;
        const aslaidN = nCoord + dy;
        const aslaidZ = depth + dx * 0.05;
        const timeStr = `2026-10-01T${String(6 + Math.floor(count / 6)).padStart(2, "0")}:${String((count * 9) % 60).padStart(2, "0")}:00Z`;
        const sn = `SN_${7200 + count}`;
        aslaidRows.push(`NODE_${String(count).padStart(3, "0")},${sn},${stnId},${lineName},${aslaidE.toFixed(2)},${aslaidN.toFixed(2)},${aslaidZ.toFixed(2)},${timeStr},${sigma},${aslaidZ.toFixed(1)},Deployed`);
      }
    }

    return {
      planned_csv: plannedRows.join("\n"),
      aslaid_csv: aslaidRows.join("\n"),
      default_header: {
        survey_name: "Triton Deep OBN Seismic Scope",
        client_name: "North Sea Petroleum Ltd",
        contractor_name: "UTM Offshore Survey Team",
        vessel_name: "MV Coastal Navigator",
        crs_name: "ED50 / UTM Zone 31N",
        epsg_code: 23031,
        date_str: "2026-10-01",
      },
    };
  }
  return invoke<ObnSampleDataset>("obn_generate_sample_project");
}

function mockEvaluateQc(plannedCsv: string, aslaidCsv: string, toleranceSpecM: number): ObnQcSummary {
  const pLines = plannedCsv.split("\n").filter(l => l.trim() && !l.startsWith("Station"));
  const aLines = aslaidCsv.split("\n").filter(l => l.trim() && !l.startsWith("NodeID"));

  const plannedMap = new Map<string, { e: number; n: number; z: number; line: string }>();
  for (const line of pLines) {
    const parts = line.split(",").map(s => s.trim());
    if (parts.length >= 5) {
      plannedMap.set(parts[0], {
        line: parts[1],
        e: parseFloat(parts[2]),
        n: parseFloat(parts[3]),
        z: parseFloat(parts[4]),
      });
    }
  }

  const nodes: ObnQcNode[] = [];
  let inSpec = 0;
  let warning = 0;
  let outOfSpec = 0;
  let sumRadial = 0;

  for (const line of aLines) {
    const p = line.split(",").map(s => s.trim());
    if (p.length < 7) continue;
    const serial = p[1];
    const stn = p[2];
    const aslaidE = parseFloat(p[4]);
    const aslaidN = parseFloat(p[5]);
    const aslaidZ = parseFloat(p[6]);
    const ts = p[7] || "2026-10-01T12:00:00Z";
    const usblUnc = parseFloat(p[8]) || 1.2;

    const target = plannedMap.get(stn) || { e: aslaidE, n: aslaidN, z: aslaidZ, line: "L01" };
    const de = aslaidE - target.e;
    const dn = aslaidN - target.n;
    const radial = Math.sqrt(de * de + dn * dn);
    const dz = aslaidZ - target.z;

    let tol: ToleranceStatus = "in_spec";
    if (radial <= toleranceSpecM) {
      inSpec++;
      tol = "in_spec";
    } else if (radial <= toleranceSpecM * 2.0) {
      warning++;
      tol = "warning";
    } else {
      outOfSpec++;
      tol = "out_of_spec";
    }

    sumRadial += radial;
    nodes.push({
      station_id: stn,
      serial_number: serial,
      line_name: target.line,
      planned_easting: target.e,
      planned_northing: target.n,
      planned_depth: target.z,
      aslaid_easting: aslaidE,
      aslaid_northing: aslaidN,
      aslaid_depth: aslaidZ,
      delta_easting: de,
      delta_northing: dn,
      radial_offset_m: radial,
      delta_depth_m: dz,
      tolerance: tol,
      usbl_uncertainty_m: usblUnc,
      timestamp_utc: ts,
      battery_pct: 98.5,
      tilt_deg: 0.8,
      status: "deployed",
    });
  }

  const total = nodes.length;
  const meanRadial = total > 0 ? sumRadial / total : 0;
  return {
    total_nodes: total,
    in_spec_count: inSpec,
    warning_count: warning,
    out_of_spec_count: outOfSpec,
    compliance_percentage: total > 0 ? ((inSpec + warning) / total) * 100 : 0,
    min_radial_offset_m: nodes.reduce((m, n) => Math.min(m, n.radial_offset_m), 9999),
    max_radial_offset_m: nodes.reduce((m, n) => Math.max(m, n.radial_offset_m), 0),
    mean_radial_offset_m: meanRadial,
    std_dev_radial_m: 1.85,
    mean_depth_m: 1295.0,
    nodes,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Subsea Pipeline Integrity (EIVA NaviModel, 3D Pipe, DelphINS, AutoCAD)
// ─────────────────────────────────────────────────────────────────────────────

export interface PipelineRoutePoint {
  kp: number;
  easting: number;
  northing: number;
  target_dob_m: number;
  pipe_od_m: number;
}

export interface DelphInsFix {
  timestamp: string;
  kp: number;
  easting: number;
  northing: number;
  depth: number;
  heading: number;
  pitch: number;
  roll: number;
  sd_easting: number;
  sd_northing: number;
  sd_depth: number;
  usbl_rejected: boolean;
}

export interface PipeSurveyPoint {
  kp: number;
  easting: number;
  northing: number;
  top_z: number;
  seabed_z: number;
  dcc: number;
  dob_m: number;
  clearance_gap_m: number;
  is_freespan: boolean;
}

export type FreespanCriticality = "Minor" | "Moderate" | "Critical";

export interface Freespan {
  span_id: string;
  start_kp: number;
  end_kp: number;
  length_m: number;
  max_gap_m: number;
  mean_gap_m: number;
  start_easting: number;
  start_northing: number;
  end_easting: number;
  end_northing: number;
  criticality: FreespanCriticality;
  remediation: string;
}

export type PipelineEventType =
  | "Anode"
  | "Crossing"
  | "FieldJoint"
  | "Mattress"
  | "GroutBag"
  | "Debris"
  | "Damage"
  | "Flange";

export interface PipelineEvent {
  event_id: string;
  kp: number;
  easting: number;
  northing: number;
  depth: number;
  event_type: PipelineEventType;
  description: string;
  anode_depletion_pct: number | null;
  cp_reading_mv: number | null;
  severity: string;
}

export interface PipelineQcSummary {
  total_length_km: number;
  point_count: number;
  buried_length_km: number;
  exposed_length_km: number;
  buried_percentage: number;
  freespan_count: number;
  total_freespan_length_m: number;
  critical_freespans: number;
  moderate_freespans: number;
  minor_freespans: number;
  max_freespan_length_m: number;
  max_freespan_gap_m: number;
  anodes_count: number;
  crossings_count: number;
  delphins_mean_horiz_sd_m: number;
  delphins_max_horiz_sd_m: number;
  delphins_usbl_rejection_pct: number;
  survey_points: PipeSurveyPoint[];
  freespans: Freespan[];
  events: PipelineEvent[];
}

export interface PipelineShiftDpr {
  survey_name: string;
  client_name: string;
  contractor_name: string;
  vessel_name: string;
  shift_date: string;
  shift_type: string;
  senior_processor: string;
  pipeline_id: string;
  kp_start: number;
  kp_end: number;
  km_processed_shift: number;
  freespans_found_shift: number;
  critical_spans_found: number;
  anodes_inspected_shift: number;
  operational_hours: number;
  weather_standby_hours: number;
  technical_downtime_hours: number;
  general_remarks: string;
  audit_hash: string;
}

export interface PipelineReportsBundle {
  freespan_csv: string;
  event_csv: string;
  profile_csv: string;
  dpr_markdown: string;
  dxf_content: string;
}

export interface PipelineSampleDataset {
  route_csv: string;
  profile_csv: string;
  events_csv: string;
  delphins_csv: string;
  default_dpr: PipelineShiftDpr;
  summary: PipelineQcSummary;
}

/** Evaluate pipeline integrity, DOB, and freespans via Tauri IPC with mock fallback. */
export async function subseaPipelineEvaluateQc(request: {
  routeContent: string;
  profileContent: string;
  eventsContent: string;
  delphinsContent: string;
  pipeOdM?: number;
}): Promise<PipelineQcSummary> {
  if (isTauri()) {
    return invoke<PipelineQcSummary>("subsea_pipeline_evaluate_qc_cmd", { request });
  }

  // Fallback simulator for browser preview
  const demo = getMockPipelineSample();
  return demo.summary;
}

/** Export standard AutoCAD DXF vector alignment sheet. */
export async function subseaPipelineExportDxf(
  points: PipeSurveyPoint[],
  freespans: Freespan[],
  events: PipelineEvent[],
  pipeOdM?: number,
): Promise<string> {
  if (isTauri()) {
    return invoke<string>("subsea_pipeline_export_dxf_cmd", {
      request: {
        points,
        freespans,
        events,
        pipeOdM: pipeOdM ?? 0.4064,
      },
    });
  }

  return "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF";
}

/** Generate CSV deliverables and DXF alignment sheet. */
export async function subseaPipelineExportReports(
  points: PipeSurveyPoint[],
  freespans: Freespan[],
  events: PipelineEvent[],
  dpr: PipelineShiftDpr,
  pipeOdM?: number,
): Promise<PipelineReportsBundle> {
  if (isTauri()) {
    return invoke<PipelineReportsBundle>("subsea_pipeline_export_reports_cmd", {
      request: {
        points,
        freespans,
        events,
        dpr,
        pipeOdM: pipeOdM ?? 0.4064,
      },
    });
  }

  return {
    freespan_csv: "span_id,start_kp,end_kp,length_m,max_gap_m,mean_gap_m,criticality,remediation\nFS-001,1.200,1.214,14.00,0.85,0.54,\"Critical\",\"Urgent: Grout Bag Support\"",
    event_csv: "event_id,kp,easting,northing,depth,type,description,anode_depletion_pct,cp_reading_mv,severity\nANO-01,0.500,580475.00,2150156.00,48.60,\"Sacrificial Anode\",\"Zinc Bracelet\",15.0,-960.0,\"Normal\"",
    profile_csv: "kp,easting,northing,top_depth_m,seabed_depth_m,dcc_m,dob_m,clearance_gap_m,status\n0.000,580000.00,2150000.00,47.20,48.00,0.00,0.80,0.00,Buried",
    dpr_markdown: "# Offshore Pipeline Integrity Survey — 12h Shift Handover\n\n**Survey Scope**: Campeche Sound 16-Inch Gas Export Pipeline Integrity\n",
    dxf_content: "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF",
  };
}

/** Format 12h Shift DPR handover with SHA-256 seal. */
export async function subseaPipelineGenerateShiftDpr(dpr: PipelineShiftDpr): Promise<PipelineShiftDpr> {
  if (isTauri()) {
    return invoke<PipelineShiftDpr>("subsea_pipeline_generate_shift_dpr_cmd", { dpr });
  }

  return {
    ...dpr,
    audit_hash: "3a88f0bc21dc89b91c14041b63ef26a1112467b4515598dcd6e866e476c5e219",
  };
}

/** Pre-populate realistic Campeche Sound, Mexico 5.0 km 16-inch pipeline inspection dataset. */
export async function subseaPipelineGenerateSampleProject(): Promise<PipelineSampleDataset> {
  if (isTauri()) {
    return invoke<PipelineSampleDataset>("subsea_pipeline_generate_sample_project");
  }

  return getMockPipelineSample();
}

function getMockPipelineSample(): PipelineSampleDataset {
  const points: PipeSurveyPoint[] = [];
  const totalKm = 5.0;
  const numPts = 101;
  const pipeOdM = 0.4064;

  for (let i = 0; i < numPts; i++) {
    const frac = i / (numPts - 1);
    const kp = frac * totalKm;
    const e = 580000.0 + kp * 1000.0 * 0.95;
    const n = 2150000.0 + kp * 1000.0 * 0.312;
    const seabedZ = 48.0 + frac * 6.0 + Math.sin(kp * 3.0) * 0.25;
    let topZ = seabedZ - 0.8;

    if (kp >= 1.2 && kp <= 1.214) {
      const arch = Math.sin(((kp - 1.2) / 0.014) * Math.PI);
      topZ = seabedZ - pipeOdM - 0.85 * arch;
    } else if (kp >= 2.45 && kp <= 2.458) {
      const arch = Math.sin(((kp - 2.45) / 0.008) * Math.PI);
      topZ = seabedZ - pipeOdM - 0.42 * arch;
    }

    const dobM = seabedZ - topZ;
    const clearanceGapM = Math.max(0, seabedZ - (topZ + pipeOdM));
    points.push({
      kp,
      easting: e,
      northing: n,
      top_z: topZ,
      seabed_z: seabedZ,
      dcc: Math.sin(kp * 5) * 0.15,
      dob_m: dobM,
      clearance_gap_m: clearanceGapM,
      is_freespan: clearanceGapM > 0.05,
    });
  }

  const freespans: Freespan[] = [
    {
      span_id: "FS-001",
      start_kp: 1.2,
      end_kp: 1.214,
      length_m: 14.0,
      max_gap_m: 0.85,
      mean_gap_m: 0.54,
      start_easting: 581140.0,
      start_northing: 2150374.4,
      end_easting: 581153.3,
      end_northing: 2150378.8,
      criticality: "Critical",
      remediation: "Urgent: Grout Bag Support Required (VIV Fatigue Risk)",
    },
    {
      span_id: "FS-002",
      start_kp: 2.45,
      end_kp: 2.458,
      length_m: 8.0,
      max_gap_m: 0.42,
      mean_gap_m: 0.27,
      start_easting: 582327.5,
      start_northing: 2150764.4,
      end_easting: 582335.1,
      end_northing: 2150766.9,
      criticality: "Moderate",
      remediation: "Remedial Rock Dump / Grout Bag Recommended",
    },
  ];

  const events: PipelineEvent[] = [
    {
      event_id: "ANO-01",
      kp: 0.5,
      easting: 580475.0,
      northing: 2150156.0,
      depth: 48.6,
      event_type: "Anode",
      description: "Zinc Bracelet Anode #1",
      anode_depletion_pct: 15.0,
      cp_reading_mv: -960.0,
      severity: "Normal",
    },
    {
      event_id: "XING-01",
      kp: 0.85,
      easting: 580807.5,
      northing: 2150265.2,
      depth: 49.0,
      event_type: "Crossing",
      description: "Crossing 8-inch Flowline",
      anode_depletion_pct: null,
      cp_reading_mv: null,
      severity: "Advisory",
    },
  ];

  const summary: PipelineQcSummary = {
    total_length_km: 5.0,
    point_count: points.length,
    buried_length_km: 4.15,
    exposed_length_km: 0.85,
    buried_percentage: 83.0,
    freespan_count: freespans.length,
    total_freespan_length_m: 22.0,
    critical_freespans: 1,
    moderate_freespans: 1,
    minor_freespans: 0,
    max_freespan_length_m: 14.0,
    max_freespan_gap_m: 0.85,
    anodes_count: 1,
    crossings_count: 1,
    delphins_mean_horiz_sd_m: 0.038,
    delphins_max_horiz_sd_m: 0.085,
    delphins_usbl_rejection_pct: 1.8,
    survey_points: points,
    freespans,
    events,
  };

  const default_dpr: PipelineShiftDpr = {
    survey_name: "Campeche Sound 16-Inch Gas Export Pipeline Integrity",
    client_name: "PEMEX Exploracion y Produccion",
    contractor_name: "UTM Offshore Survey Team",
    vessel_name: "MV Deep Ocean Explorer",
    shift_date: "2026-10-13",
    shift_type: "Day Shift (00:00 - 12:00)",
    senior_processor: "Senior Data Processor",
    pipeline_id: "PL-442-16IN (Akall to Dos Bocas)",
    kp_start: 0.0,
    kp_end: 5.0,
    km_processed_shift: 5.0,
    freespans_found_shift: 2,
    critical_spans_found: 1,
    anodes_inspected_shift: 1,
    operational_hours: 11.0,
    weather_standby_hours: 0.5,
    technical_downtime_hours: 0.5,
    general_remarks: "Completed 5.0 km processing section. Critical freespan FS-001 (14m) flagged for client review.",
    audit_hash: "3a88f0bc21dc89b91c14041b63ef26a1112467b4515598dcd6e866e476c5e219",
  };

  return {
    route_csv: "kp,easting,northing,target_dob_m,pipe_od_m\n0.000,580000.00,2150000.00,1.00,0.4064\n5.000,584750.00,2151560.00,1.00,0.4064",
    profile_csv: "kp,easting,northing,top_depth_m,seabed_depth_m,dcc_m\n0.000,580000.00,2150000.00,47.20,48.00,0.00",
    events_csv: "event_id,kp,easting,northing,depth,type,description,anode_depletion_pct,cp_reading_mv,severity\nANO-01,0.500,580475.00,2150156.00,48.60,\"Sacrificial Anode\",\"Zinc Bracelet\",15.0,-960.0,\"Normal\"",
    delphins_csv: "timestamp,kp,easting,northing,depth,heading,pitch,roll,sd_easting,sd_northing,sd_depth,usbl_rejected\n2026-10-13T10:00:00Z,0.000,580000.00,2150000.00,48.00,71.8,-0.5,0.2,0.035,0.038,0.018,0",
    default_dpr,
    summary,
  };
}


