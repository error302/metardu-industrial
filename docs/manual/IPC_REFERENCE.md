# IPC Reference — MetaRDU Industrial

> **108 commands** across 10 modules. This document is the complete
> reference for every `#[tauri::command]` exposed to the frontend via
> `tauri::generate_handler![]` in `src-tauri/src/lib.rs`.

## Quick Reference

| Module | Commands | Sprint | Purpose |
|--------|----------|--------|---------|
| Core | 11 | 0-1 | Modules, settings, file probing, LAS reading, profiles, reports, CRS |
| Mining | 3 | 1 | Drone manifest, ground classification, volume calc |
| Marine | 6 | 2-3 | CUBE surface, S-44, S-57 export, SVP, dredge, cross-sections |
| Pipelines | 3 | 4 | ODM (OpenDroneMap) availability + execution |
| Bottleneck Tools | 4 | 5 | Density gates, tidal correction, machine control, DEM render |
| Sprint 6 | 7 | 6 | SSS (side-scan sonar), slice editor, brush reject |
| Sprint 7 | 13 | 7 | License manager, telemetry, benchmarks |
| Sprint 8 | 17 | 8 | Projects, updater, i18n, plugin marketplace |
| Sprint 9 (EOM) | 15 | 9 | EOM auditor, NTRIP, triage, watch folder, DXF import |
| Automation | 9 | 8 | Pipeline runner, watch folders, scheduled jobs |
| Streaming | 8 | 5 | Live sonar stream, distributed CUBE coordinator |
| Deliverable | 1 | 5 | Survey deliverable package |
| Monitoring | 3 | 8 | Highwall, 4D epoch diff, progression |
| ML | 2 | 8 | Habitat classification, blast fragmentation |
| Plugins | 2 | 4 | Plugin listing, supported extensions |
| **Total** | **108** | | |

---

## Core Commands (`src-tauri/src/commands/mod.rs`)

### `ping`
- **Params:** none
- **Returns:** `string` — always `"pong"`
- **Purpose:** Health check — confirms the IPC bridge is alive.

### `app_version`
- **Params:** none
- **Returns:** `string` — e.g. `"0.1.0"`
- **Purpose:** Get the Rust core semantic version.

### `init_module`
- **Params:** `id: string`
- **Returns:** `{ id: string, status: "pending"|"loading"|"ok"|"fail", load_time_ms: number, error: string|null }`
- **Purpose:** Initialize a processing module by id (PROJ, GDAL, PDAL, etc.).

### `list_modules`
- **Params:** none
- **Returns:** `[{ id, name, version, description, can_fail }]`
- **Purpose:** List all known processing modules for the boot screen.

### `get_settings`
- **Params:** none
- **Returns:** `{ defaultDomain, defaultEpsg, density, reducedMotion }`
- **Purpose:** Read persisted user settings from `app_config_dir/settings.json`.

### `save_settings`
- **Params:** `settings: AppSettingsRpc`
- **Returns:** `void`
- **Purpose:** Persist user settings to disk.

### `probe_file`
- **Params:** `path: string`
- **Returns:** `FileProbeResult` (tagged union: `Las | Geotiff | KongsbergAll | ResonS7k | MbEs`)
- **Purpose:** Read a survey file's header and return metadata + bounds. This is the entry point for the drag-and-drop workflow.
- **Security:** Path validated via `validate_path()` — rejects paths into `~/.ssh`, `~/.aws`, browser dirs, etc.

### `read_las_points_binary`
- **Params:** `path: string, max_points: u64`
- **Returns:** `Vec<u8>` — packed f32 LE: `[x0, y0, z0, x1, y1, z1, ...]`
- **Purpose:** Read LAS points as packed binary for Deck.gl. 12 bytes/point. `max_points=0` means "all".
- **Security:** Path validated + OOM clamp (`file_size / record_length`).

### `read_las_points_cmd`
- **Params:** `path: string, max_points: u64`
- **Returns:** `Vec<(f64, f64, f64)>` — JSON array of `[x, y, z]` tuples
- **Purpose:** Read LAS points as JSON (backward compat, prefer binary for >100K points).

### `read_las_points_streaming_cmd`
- **Params:** `path: string, max_points: u64, on_chunk: Channel<LasPointsChunk>`
- **Returns:** `u64` — total points read
- **Purpose:** Stream LAS points in 65K-point chunks via a Tauri Channel. Prevents OOM on 100M+ point files.
- **Chunk format:** `{ points: number[], count: number, total_read: number, total_points: number }`

### `sample_profile`
- **Params:** `path: string, start: [number, number], end: [number, number], samples: number`
- **Returns:** `{ elevations: number[], distances: number[], min: number, max: number }`
- **Purpose:** Sample elevation values along a line from a GeoTIFF DEM.

### `generate_report_cmd`
- **Params:** `spec: ReportSpec`
- **Returns:** `string` — path to generated HTML report
- **Purpose:** Generate an HTML compliance report.

### `is_proj_available`
- **Params:** none
- **Returns:** `bool`
- **Purpose:** Check if PROJ (CRS reprojection) is available. Always true with the pure-Rust UTM fallback.

### `transform_coords_cmd`
- **Params:** `coords: [{x, y, z?}], from_crs: string, to_crs: string`
- **Returns:** `{ coords: [{x, y, z?}], method: "PureRustUtm"|"Proj"|"Identity" }`
- **Purpose:** Transform coordinates between CRS (e.g. EPSG:4326 → EPSG:32756).

---

## Mining Commands (`src-tauri/src/commands/mining.rs`)

### `parse_drone_manifest`
- **Params:** `path: string`
- **Returns:** `DroneManifest` — image metadata, bounds, GPS positions
- **Purpose:** Parse DJI MMC / FlightHub JSON / generic CSV drone manifests.

### `classify_ground`
- **Params:** `path: string, params: CsfParams, max_points: Option<u64>`
- **Returns:** `CsfResult` — per-point ground/non-ground classification
- **Purpose:** Run CSF (Cloth Simulation Filter) ground extraction on a LAS point cloud.

### `compute_volumes_cmd`
- **Params:** `request: ComputeVolumesRequest`
- **Returns:** `VolumeResult` — fill/cut/net volumes with bench breakdown
- **Purpose:** Compute fill/cut volumes by differencing two DEM surfaces.

---

## Marine Commands (`src-tauri/src/commands/marine.rs`)

### `generate_cube_surface_cmd`
- **Params:** `soundings: [{x, y, z, uncertainty?}], cell_size: number, domain: string`
- **Returns:** `CubeSurface` — CUBE hypothesis surface (depths, uncertainties, hypothesis counts)
- **Purpose:** Generate a CUBE (Combined Uncertainty and Bathymetry Estimation) surface.

### `compute_tpu_batch`
- **Params:** `soundings: [...], vessel: VesselConfig, environment: EnvironmentConfig`
- **Returns:** `Vec<TpuResult>` — Total Propagated Uncertainty per sounding
- **Purpose:** Compute TPU for IHO S-44 compliance.

### `check_s44_compliance_cmd`
- **Params:** `soundings: [...], order: S44Order, cell_size: number`
- **Returns:** `S44ComplianceResult` — per-cell compliance status
- **Purpose:** Check bathymetric data against IHO S-44 orders (Special, 1a, 1b, 2).

### `export_s57`
- **Params:** `request: S57ExportRequest`
- **Returns:** `string` — path to exported .000 file
- **Purpose:** Export survey data to S-57 ENC format.

### `parse_svp_cmd`
- **Params:** `path: string`
- **Returns:** `SvpProfile` — sound velocity profile (depth vs speed)
- **Purpose:** Parse .svp/.asvp sound velocity profile files.

### `compute_dredge_audit_cmd`
- **Params:** `request: DredgeAuditRequest`
- **Returns:** `DredgeAuditResult` — pre/post/design volumes, over-dredge, under-dredge
- **Purpose:** Compute dredge pay volumes against a design surface.

### `compute_cross_sections_cmd`
- **Params:** `request: CrossSectionRequest`
- **Returns:** `CrossSectionResult` — channel cross-section profiles
- **Purpose:** Generate cross-section profiles for channel design verification.

---

## Pipeline Commands (`src-tauri/src/commands/pipelines.rs`)

### `check_odm_availability`
- **Params:** `image: Option<string>`
- **Returns:** `{ available: bool, version: string|null, error: string|null }`
- **Purpose:** Check if OpenDroneMap Docker image is available.

### `run_odm_pipeline`
- **Params:** `config: OdmConfig, on_progress: Channel<OdmProgress>`
- **Returns:** `OdmRunResult` — output paths, processing time
- **Purpose:** Run the ODM (OpenDroneMap) photogrammetry pipeline via Docker.

### `get_odm_status`
- **Params:** none
- **Returns:** `OdmRunStatus | null` — current pipeline status
- **Purpose:** Poll the ODM pipeline status.

---

## Bottleneck Tools (`src-tauri/src/commands/bottleneck_tools.rs`)

### `run_density_gates_cmd`
- **Params:** `request: DensityGatesRequest`
- **Returns:** `CoverageReport` — S-44 density gate compliance
- **Purpose:** Validate survey coverage against S-44 density requirements.

### `run_tidal_correction_cmd`
- **Params:** `request: TidalCorrectionRequest`
- **Returns:** `TidalCorrectionResult` — tide-corrected depths
- **Purpose:** Apply tidal corrections to bathymetric data via cubic spline.

### `compile_machine_control_cmd`
- **Params:** `request: MachineControlRequest`
- **Returns:** `MachineControlResult` — Leica .svd / Trimble .tp3 / Topcon .top file path
- **Purpose:** Compile a design surface to a machine control file.

### `render_dem_cmd`
- **Params:** `request: DemRenderRequest`
- **Returns:** `DemRenderResult` — hillshaded RGBA image + bounds
- **Purpose:** Render a GeoTIFF DEM as a hillshaded color-ramp image. Parallelized with rayon.

---

## Sprint 6 — SSS + Slice Editor (`src-tauri/src/commands/sprint6.rs`)

### `read_sss_pings_cmd`
- **Params:** `path: string, max_pings: u64`
- **Returns:** `SssData` — side-scan sonar pings with backscatter samples
- **Purpose:** Read XTF (side-scan sonar) files.

### `compute_target_height_cmd`
- **Params:** `fish_altitude_m, slant_range_to_target_m, shadow_length_m`
- **Returns:** `f64` — target height above seafloor (meters)
- **Purpose:** Compute target height from slant range + shadow length.

### `slice_by_polygon_cmd`
- **Params:** `request: SliceRequest`
- **Returns:** `SliceResult` — points inside the polygon + total/slice counts
- **Purpose:** Extract points inside a polygon from a LAS file.

### `brush_reject_cmd`
- **Params:** `request: BrushRejectRequest`
- **Returns:** `BrushResult` — accepted/rejected point indices
- **Purpose:** Apply a brush reject to a point cloud slice.

### `undo_brush_cmd`
- **Params:** `request: UndoRequest`
- **Returns:** `BrushResult` — restored point indices
- **Purpose:** Undo the last brush reject.

### `accepted_indices_cmd`
- **Params:** `request: AcceptedIndicesRequest`
- **Returns:** `Vec<u32>` — accepted point indices
- **Purpose:** Get the accepted point indices after all brush operations.

### `point_in_polygon_cmd`
- **Params:** `point: [number, number], polygon: [[number, number]]`
- **Returns:** `bool`
- **Purpose:** Ray-casting point-in-polygon test.

---

## Sprint 7 — License + Telemetry + Benchmarks (`src-tauri/src/commands/sprint7.rs`)

### `get_license_status_cmd`
- **Params:** `license_path: Option<string>`
- **Returns:** `LicenseStatus`
- **Purpose:** Check the HMAC license system status (Pro/Enterprise tiers).

### `activate_license_cmd`
- **Params:** `license_content: string, save_path: Option<string>`
- **Returns:** `LicenseStatus`
- **Purpose:** Activate a license from pasted content. Saves to `app_data_dir/license.json` (NOT the user-supplied path — security).
- **Note:** `generate_license_cmd` is NOT exposed via IPC — it's a forge oracle.

### `check_feature_cmd`
- **Params:** `feature: string, license_path: Option<string>`
- **Returns:** `bool`
- **Purpose:** Check if a specific feature is unlocked by the current license.

### `init_telemetry_cmd`
- **Params:** `config: TelemetryConfig`
- **Returns:** `void`
- **Purpose:** Initialize the telemetry system.

### `update_telemetry_config_cmd`
- **Params:** `config: TelemetryConfig`
- **Returns:** `void`
- **Purpose:** Update telemetry config (e.g. toggle opt-in).

### `get_telemetry_config_cmd`
- **Params:** none
- **Returns:** `TelemetryConfig`
- **Purpose:** Get the current telemetry config.

### `record_telemetry_event_cmd`
- **Params:** `event_type, event_name, duration_ms, success, error, license_tier`
- **Returns:** `void`
- **Purpose:** Record a telemetry event. File paths in error strings are sanitized to `<redacted>`.

### `record_crash_cmd`
- **Params:** `crash: CrashDump`
- **Returns:** `void`
- **Purpose:** Record a crash dump for later submission.

### `get_telemetry_stats_cmd`
- **Params:** none
- **Returns:** `TelemetryStats`
- **Purpose:** Get aggregated telemetry statistics.

### `get_recent_events_cmd`
- **Params:** `limit: Option<usize>`
- **Returns:** `Vec<TelemetryEvent>`
- **Purpose:** Get recent telemetry events.

### `get_pending_crashes_cmd`
- **Params:** none
- **Returns:** `Vec<CrashDump>`
- **Purpose:** Get crash dumps waiting for submission.

### `mark_crash_submitted_cmd`
- **Params:** `crash_id: string`
- **Returns:** `void`
- **Purpose:** Mark a crash dump as submitted.

### `run_benchmarks_cmd`
- **Params:** `iterations: Option<u32>`
- **Returns:** `BenchmarkSuiteResult`
- **Purpose:** Run performance benchmarks (volume calc, CSF, CUBE, etc.).

---

## Sprint 8 — Project + Updater + i18n + Marketplace (`src-tauri/src/commands/sprint8.rs`)

### `new_project_cmd`
- **Params:** `request: NewProjectRequest`
- **Returns:** `MetarduProject`
- **Purpose:** Create a new project file.

### `save_project_cmd`
- **Params:** `project: MetarduProject, path: string`
- **Returns:** `string` — saved path
- **Purpose:** Save a project to disk.

### `load_project_cmd`
- **Params:** `path: string`
- **Returns:** `MetarduProject`
- **Purpose:** Load a project from disk.

### `add_file_to_project_cmd`
- **Params:** `project, file_path, file_kind`
- **Returns:** `MetarduProject`
- **Purpose:** Add a file to a project.

### `remove_file_from_project_cmd`
- **Params:** `project, file_path`
- **Returns:** `MetarduProject`
- **Purpose:** Remove a file from a project.

### `update_view_state_cmd`
- **Params:** `project, view_state`
- **Returns:** `MetarduProject`
- **Purpose:** Update the saved view state (center, zoom, layers).

### `add_recent_report_cmd`
- **Params:** `project, report_path`
- **Returns:** `MetarduProject`
- **Purpose:** Add a report to the project's recent reports list.

### `check_for_updates_cmd`
- **Params:** `app: AppHandle, endpoint: Option<string>` (endpoint ignored — configured in tauri.conf.json)
- **Returns:** `UpdateInfo` — `{ available, latest_version, current_version, release_date, release_notes, ... }`
- **Purpose:** Check for app updates via tauri-plugin-updater. Ed25519 signature verification.
- **Note:** Returns `NotConfigured` error if pubkey/endpoints are empty in tauri.conf.json.

### `download_and_install_update_cmd`
- **Params:** `app: AppHandle`
- **Returns:** `void`
- **Purpose:** Download + verify + install the latest update. Frontend should prompt restart on success.

### `get_update_status_cmd`
- **Params:** none
- **Returns:** `UpdateStatus`
- **Purpose:** Get the current updater state.

### `get_current_version_cmd`
- **Params:** none
- **Returns:** `string`
- **Purpose:** Get the app version from Cargo.toml.

### `translate_cmd`
- **Params:** `key: string, lang_code: string`
- **Returns:** `string`
- **Purpose:** Translate a key to the given language.

### `get_available_languages_cmd`
- **Params:** none
- **Returns:** `Vec<(code, label)>` — e.g. `[("en", "English"), ("es", "Español")]`
- **Purpose:** List available languages.

### `fetch_plugin_registry_cmd`
- **Params:** `source: string` — URL or file path
- **Returns:** `PluginRegistry`
- **Purpose:** Fetch a plugin registry from a URL or local file.

### `list_installed_plugins_cmd`
- **Params:** `app: AppHandle`
- **Returns:** `Vec<InstalledPlugin>`
- **Purpose:** List installed plugins.

### `install_plugin_cmd`
- **Params:** `app: AppHandle, registry, plugin_id`
- **Returns:** `InstalledPlugin`
- **Purpose:** Install a plugin from the registry.

### `uninstall_plugin_cmd`
- **Params:** `app: AppHandle, plugin_id`
- **Returns:** `void`
- **Purpose:** Uninstall a plugin.

### `search_registry_cmd`
- **Params:** `registry, query`
- **Returns:** `Vec<PluginEntry>`
- **Purpose:** Search a plugin registry.

---

## Sprint 9 — EOM + NTRIP + Triage (`src-tauri/src/commands/eom.rs`)

### `run_eom_pipeline_cmd`
- **Params:** `input: EomInputAdapter, on_progress: Channel<EomProgress>`
- **Returns:** `EomOutputAdapter` — volumes, DEM, audit hash, chain of custody
- **Purpose:** Run the full EOM pipeline: LAS → CSF → DEM → volumes → signed PDF.
- **Note:** `sign_eom_license_cmd` is NOT exposed via IPC — it's a signing oracle.

### `generate_eom_report_cmd`
- **Params:** `eom_output, customer, site, surveyor, output_path, signed`
- **Returns:** `void`
- **Purpose:** Generate a signed PDF report from EOM pipeline output.

### `detect_machine_fingerprint_cmd`
- **Params:** none
- **Returns:** `FingerprintAdapter` — machine_id + site_id + fingerprint_hash
- **Purpose:** Compute the machine fingerprint for license node-locking.

### `verify_eom_license_cmd`
- **Params:** `license: LicenseFile`
- **Returns:** `LicenseClaims`
- **Purpose:** Verify an EOM license file's RSA-PSS signature.

### `check_license_status_cmd`
- **Params:** `license: Option<LicenseFile>`
- **Returns:** `LicenseStatusAdapter` — tagged union: `{ state: "Trial", trial_reports_remaining }` | `{ state: "Active", customer, ... }` | ...
- **Purpose:** Check the EOM license status. Returns a tagged union matching the TS contract.

### `consume_report_cmd`
- **Params:** `license: Option<LicenseFile>`
- **Returns:** `LicenseStatusAdapter`
- **Purpose:** Consume a report from the per-report quota.

### `import_dxf_surface_cmd`
- **Params:** `path: string, cell_size: f64`
- **Returns:** `DesignDem` — rasterized design surface
- **Purpose:** Import a DXF TIN surface and rasterize to a regular DEM grid.

### `run_triage_cmd`
- **Params:** `dir: string`
- **Returns:** `TriageReport` — file health, bounds, CRS mismatches, temporal span
- **Purpose:** Run field data triage on a directory (EXIF, RINEX, NMEA, LAS headers).

### `start_ntrip_cmd`
- **Params:** `config: NtripConfig` (includes `use_tls: bool`)
- **Returns:** `NtripStatus`
- **Purpose:** Start the NTRIP/RTCM3 client. Supports TLS (ntrips://) via rustls.

### `stop_ntrip_cmd`
- **Params:** none
- **Returns:** `void`
- **Purpose:** Stop the NTRIP client.

### `get_ntrip_status_cmd`
- **Params:** none
- **Returns:** `NtripStatus`
- **Purpose:** Get the current NTRIP client status (connected, correction age, messages received, etc.).

### `start_eom_watch_folder`
- **Params:** `config: EomWatchFolderConfig`
- **Returns:** `void`
- **Purpose:** Start zero-touch EOM pipeline on a watch folder.

### `stop_eom_watch_folder`
- **Params:** none
- **Returns:** `void`
- **Purpose:** Stop the EOM watch folder.

### `is_eom_watch_folder_running`
- **Params:** none
- **Returns:** `bool`
- **Purpose:** Check if the EOM watch folder is running.

---

## Automation (`src-tauri/src/commands/automation.rs`)

### `parse_pipeline_cmd`
- **Params:** `yaml: string`
- **Returns:** `Pipeline`
- **Purpose:** Parse a YAML pipeline definition.

### `serialize_pipeline_cmd`
- **Params:** `pipeline: Pipeline`
- **Returns:** `string` — YAML
- **Purpose:** Serialize a pipeline to YAML.

### `run_pipeline_cmd`
- **Params:** `pipeline: Pipeline, input: PipelineInput, on_log: Channel<string>`
- **Returns:** `PipelineRunResult` — status, outputs, logs, error
- **Purpose:** Run a pipeline. `ShellCommand` action is disabled for security.

### `add_watch_folder`
- **Params:** `path: string, pipeline_name: string, extensions: Vec<string>`
- **Returns:** `void`
- **Purpose:** Add a watch folder that auto-runs a pipeline on new files.

### `remove_watch_folder`
- **Params:** `path: string`
- **Returns:** `void`

### `list_watch_folders`
- **Params:** none
- **Returns:** `Vec<WatchFolderStatus>`

### `scan_watch_folders`
- **Params:** none
- **Returns:** `void`
- **Purpose:** Manually trigger a scan of all watch folders.

### `add_scheduled_job`
- **Params:** `name: string, pipeline_name: string, interval_secs: u64`
- **Returns:** `void`

### `remove_scheduled_job`
- **Params:** `name: string`
- **Returns:** `void`

### `list_scheduled_jobs`
- **Params:** none
- **Returns:** `Vec<ScheduledJobStatus>`

### `check_due_jobs`
- **Params:** none
- **Returns:** `void`
- **Purpose:** Check for and run due scheduled jobs.

---

## Streaming + Distributed CUBE (`src-tauri/src/commands/streaming.rs`)

### `start_stream_cmd`
- **Params:** `app: AppHandle, config: StreamConfig`
- **Returns:** `void`
- **Purpose:** Start the UDP sonar stream listener. Binds to 127.0.0.1 only.

### `stop_stream_cmd`
- **Params:** none
- **Returns:** `void`

### `get_stream_status_cmd`
- **Params:** none
- **Returns:** `StreamStatus`

### `start_coordinator_cmd`
- **Params:** `app: AppHandle, port: u16`
- **Returns:** `void`
- **Purpose:** Start the distributed CUBE coordinator. Binds to 127.0.0.1 only.

### `stop_coordinator_cmd`
- **Params:** none
- **Returns:** `void`

### `get_coordinator_status_cmd`
- **Params:** none
- **Returns:** `ServerStatus`

### `enqueue_distributed_cube`
- **Params:** `chunks: Vec<WorkChunk>`
- **Returns:** `void`

### `merge_distributed_cube_results`
- **Params:** `results: Vec<WorkResult>`
- **Returns:** `CubeSurface`

---

## Deliverable (`src-tauri/src/commands/deliverable.rs`)

### `generate_deliverable_package_cmd`
- **Params:** `request: DeliverableRequest`
- **Returns:** `DeliverableResult` — ZIP path, manifest, bundled files
- **Purpose:** Generate a survey deliverable package (ZIP with manifest, data, metadata).

---

## Monitoring (`src-tauri/src/commands/monitoring.rs`)

### `compute_epoch_diff_cmd`
- **Params:** `epoch1, epoch2`
- **Returns:** `EpochDiffResult` — per-cell elevation change

### `compute_progression_cmd`
- **Params:** `epochs: [...], params`
- **Returns:** `ProgressionResult` — volume progression over time

### `analyze_highwall_cmd`
- **Params:** `surfaces: [...], dates: [...], cell_size, thresholds`
- **Returns:** `HighwallAnalysisResult` — displacement, velocity, compliance %
- **Note:** Date parsing uses proleptic Gregorian calendar (year-boundary safe).

---

## ML (`src-tauri/src/commands/ml.rs`)

### `classify_habitat_cmd`
- **Params:** `features: HabitatFeatures`
- **Returns:** `HabitatClassificationResult` — class + confidence + probabilities
- **Purpose:** Classify seafloor habitat from backscatter features.

### `analyze_fragmentation_cmd`
- **Params:** `sizes: Vec<f64>` — fragment sizes in mm
- **Returns:** `FragmentationResult` — P20/P50/P80/P90, mean, uniformity, quality
- **Purpose:** Analyze blast fragmentation from drone imagery of muck pile.

---

## Plugins (`src-tauri/src/plugins/mod.rs`)

### `list_plugins`
- **Params:** none
- **Returns:** `Vec<PluginInfo>`
- **Purpose:** List loaded file-reader plugins.

### `get_supported_extensions`
- **Params:** none
- **Returns:** `Vec<string>` — e.g. `["las", "laz", "tif", "all", "s7k"]`
- **Purpose:** Get all file extensions supported by loaded plugins.

---

## Security Notes

- All `path: string` parameters are validated via `validate_path()` before
  filesystem access. Paths into `~/.ssh`, `~/.aws`, `~/.gnupg`, browser dirs,
  and shell rc files are rejected. See `src-tauri/src/path_validation.rs`.
- `generate_license_cmd` and `sign_eom_license_cmd` are NOT exposed via IPC —
  they're signing oracles. See `SECURITY.md`.
- NTRIP supports TLS (`use_tls: true`) via rustls with the Mozilla root CA store.
- The auto-updater verifies Ed25519 signatures before installing.
- Plugin loading requires a `.sig` sidecar with an RSA-PSS signature over the
  plugin binary's SHA-256 hash.
