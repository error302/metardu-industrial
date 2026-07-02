# MetaRDU Industrial — IPC Command Reference

**Version**: 0.1.0-beta.1  
**Total commands**: 39

All commands are invoked via Tauri's `invoke()` function from the frontend. Each command is a Rust function annotated with `#[tauri::command]` and registered in `lib.rs` via `generate_handler!`.

---

## Core (6 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `ping` | — | `String` | Health check — returns "metardu-industrial-core-online" |
| `app_version` | — | `String` | Rust core semantic version |
| `init_module` | `id: String` | `ModuleLoadResult` | Initialize a processing module by ID |
| `list_modules` | — | `Vec<ModuleInfo>` | List all 8 processing modules with metadata |
| `get_settings` | — | `AppSettings` | Read persisted user settings |
| `save_settings` | `settings: AppSettings` | `()` | Persist user settings to disk |

## File Ingest (3 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `probe_file` | `path: String` | `FileProbeResult` | Read file header — returns LAS/GeoTIFF/.all/.s7k metadata |
| `read_las_points_cmd` | `path: String, max_points: u64` | `Vec<(f64,f64,f64)>` | Read LAS point data as (x,y,z) tuples |
| `sample_profile` | `path, start_lon, start_lat, end_lon, end_lat, num_samples` | `ProfileSampleResult` | Sample elevation along a line in a GeoTIFF DEM (bilinear) |

## Mining (5 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `parse_drone_manifest` | `path: String` | `DroneManifest` | Parse DJI MMC/FlightHub/CSV manifest |
| `classify_ground` | `path, params: CsfParams, max_points` | `CsfResult` | Run CSF ground extraction on LAS |
| `compute_volumes_cmd` | `request: ComputeVolumesRequest` | `VolumeResult` | Compute fill/cut volumes between two DEMs |
| `compute_epoch_diff_cmd` | `request: EpochDiffRequest` | `EpochDiff` | 4D monitoring — diff two DEM epochs |
| `compute_progression_cmd` | `request: ProgressionRequest` | `ProgressionReport` | Cumulative progression across N epochs |

## Marine (5 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `generate_cube_surface_cmd` | `soundings, params: CubeParams` | `CubeSurface` | Generate CUBE bathymetric surface |
| `compute_tpu_batch` | `soundings: Vec<SoundingTpuInput>` | `Vec<TpuResult>` | Compute TPU for batch of soundings |
| `check_s44_compliance_cmd` | `request: S44CheckRequest` | `S44ComplianceResult` | Check IHO S-44 compliance |
| `export_s57` | `features: Vec<S57Feature>, path: String` | `()` | Write S-57 .000 file |

## ML (2 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `classify_habitat_cmd` | `features: BackscatterFeatures` | `HabitatClassificationResult` | Classify seafloor habitat from backscatter |
| `analyze_fragmentation_cmd` | `request: FragmentationRequest` | `FragmentationResult` | Analyze blast fragmentation from sizes |

## Pipelines / ODM (3 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `check_odm_availability` | `image: Option<String>` | `OdmCheckResult` | Check Docker + ODM image |
| `run_odm_pipeline` | `config: OdmConfig` | `String` (LAS path) | Run ODM via Docker, stream progress events |
| `get_odm_status` | — | `Option<OdmRunStatus>` | Get last ODM run status |

## Geodesy (2 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `is_proj_available` | — | `bool` | Check if PROJ crate is compiled in (geo feature) |
| `transform_coords_cmd` | `coords, from_crs, to_crs` | `TransformResult` | Transform coordinates between CRSs |

## Plugins (2 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `list_plugins` | — | `Vec<PluginInfo>` | List all registered plugins |
| `get_supported_extensions` | — | `Vec<String>` | Get file extensions from plugins |

## Automation (11 commands)

| Command | Params | Returns | Description |
|---|---|---|---|
| `parse_pipeline_cmd` | `yaml: String` | `Pipeline` | Parse YAML pipeline definition |
| `serialize_pipeline_cmd` | `pipeline: Pipeline` | `String` | Serialize pipeline to YAML |
| `run_pipeline_cmd` | `pipeline, input` | `PipelineRunResult` | Execute pipeline — streams `pipeline://progress` events |
| `add_watch_folder` | `folder: WatchFolder` | `()` | Register a watch folder |
| `remove_watch_folder` | `id: String` | `()` | Remove a watch folder |
| `list_watch_folders` | — | `Vec<WatchFolderStatus>` | List all watch folders + stats |
| `scan_watch_folders` | — | `Vec<(String,String,String)>` | Scan for new files — returns (folder_id, pipeline, file_path) |
| `add_scheduled_job` | `job: ScheduledJob` | `()` | Register a scheduled job |
| `remove_scheduled_job` | `id: String` | `()` | Remove a scheduled job |
| `list_scheduled_jobs` | — | `Vec<ScheduledJobStatus>` | List all jobs + stats |
| `check_due_jobs` | — | `Vec<String>` | Check which jobs are due to run |

---

## Events

| Event | Payload | Description |
|---|---|---|
| `odm://progress` | `OdmRunStatus` | ODM pipeline progress (log lines, phase, errors) |
| `pipeline://progress` | JSON object | Pipeline step progress (step_id, action, status, log_lines, error) |
