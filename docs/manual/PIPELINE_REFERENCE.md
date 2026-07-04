# MetaRDU Industrial — YAML Pipeline Reference

**Version**: 0.1.0-beta.1  
**Total actions**: 11

---

## Pipeline Schema

```yaml
name: string                    # Required — pipeline name
description: string             # Optional — human-readable description
steps:                          # Required — ordered list of steps
  - id: string                  # Required — unique step identifier
    action: <action_name>       # Required — one of the 11 actions below
    params:                     # Optional — action-specific parameters
      key: value
    outputs:                    # Optional — output variable declarations
      output_name: "{{steps.<id>.<key>}}"
watch_folders:                  # Optional — directories to watch
  - "/path/to/watch"
schedule: string                # Optional — cron expression (Phase 4+)
```

## Template Variables

Template variables allow step-to-step data flow. They are resolved before each step executes.

| Variable | Description | Example |
|---|---|---|
| `{{input.<key>}}` | Pipeline input parameter | `{{input.dir}}` → `/data/survey` |
| `{{steps.<id>.<key>}}` | Output from a previous step | `{{steps.ingest.las_path}}` → `/output/result.las` |

---

## Actions

### 1. `probe_file`

Reads a file header and returns metadata.

**Params**:
| Param | Type | Required | Description |
|---|---|---|---|
| `path` | string | Yes | File path |

**Outputs** (LAS):
| Key | Type | Description |
|---|---|---|
| `point_count` | u64 | Total points in cloud |
| `min_x`, `min_y`, `max_x`, `max_y` | f64 | Geographic bounds |
| `las_version` | string | e.g., "1.4" |

**Outputs** (GeoTIFF):
| Key | Type | Description |
|---|---|---|
| `width`, `height` | u32 | Image dimensions |
| `epsg` | Option<u16> | EPSG code if available |
| `min_x`, `min_y`, `max_x`, `max_y` | f64 | Geographic bounds |

**Example**:
```yaml
- id: probe
  action: probe_file
  params:
    path: "{{input.file_path}}"
```

---

### 2. `classify_ground`

Runs CSF (Cloth Simulation Filter) ground extraction on a LAS point cloud.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `path` | string | — | LAS file path (required) |
| `cloth_resolution` | f64 | 0.5 | Grid spacing for cloth (m) |
| `classification_threshold` | f64 | 0.5 | Max distance from cloth for ground (m) |
| `max_iterations` | u32 | 500 | Cap on simulation steps |
| `max_points` | u64 | 0 | Point limit (0 = all) |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `ground_count` | usize | Points classified as ground |
| `non_ground_count` | usize | Points classified as non-ground |
| `total_points` | usize | Total points processed |
| `iterations` | u32 | Actual iterations run |

**Example**:
```yaml
- id: classify
  action: classify_ground
  params:
    path: "{{steps.ingest.las_path}}"
    cloth_resolution: 0.5
    classification_threshold: 0.5
```

---

### 3. `compute_volumes`

Computes fill/cut volumes by differencing two DEM surfaces.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `current_path` | string | — | Current survey GeoTIFF (required) |
| `reference_path` | string | — | Reference: file path or `flat:Z` (required) |
| `bench_interval` | f64 | 5.0 | Bench band width (m), 0 = skip |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `fill_volume` | f64 | Fill volume (m³) |
| `cut_volume` | f64 | Cut volume (m³) |
| `net_volume` | f64 | Net = fill - cut (m³) |
| `fill_cells` | usize | Cells with fill |
| `cut_cells` | usize | Cells with cut |
| `bench_count` | usize | Number of bench bands |

**Example**:
```yaml
- id: volume
  action: compute_volumes
  params:
    current_path: "{{steps.ingest.las_path}}"
    reference_path: "flat:100.0"
    bench_interval: 5.0
```

---

### 4. `generate_report`

Writes an HTML report summarizing pipeline outputs.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `output_path` | string | `/tmp/report.html` | Output file path |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `report_path` | string | Path to written report |

---

### 5. `odm_pipeline`

Runs OpenDroneMap via Docker to produce a point cloud from drone photos.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `images_dir` | string | — | Directory of JPEG/TIFF images (required) |
| `feature_quality` | string | "high" | ultra/high/medium/low/lowest |
| `max_concurrency` | u32 | 4 | CPU cores |
| `pc_type` | string | "las" | Output format: las/laz/ply/csv |
| `skip_3dmodel` | bool | true | Skip 3D mesh generation |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `las_path` | string | Path to resulting point cloud |

---

### 6. `generate_cube_surface`

Generates a CUBE bathymetric surface from soundings.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `soundings` | JSON array | — | Array of {x, y, depth, uncertainty} (required) |
| `resolution` | f64 | 1.0 | Grid cell size (m) |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `valid_cells` | usize | Cells with valid depth |
| `ambiguous_cells` | usize | Cells with multiple hypotheses |
| `total_soundings` | usize | Soundings processed |

---

### 7. `check_s44_compliance`

Checks IHO S-44 compliance for a batch of soundings.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `soundings` | JSON array | — | Array of {depth, vertical_tpu_95, horizontal_tpu_95} |
| `order` | string | "order_1a" | special/order_1a/order_1b/order_2 |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `pass_rate` | f64 | 0.0–1.0 |
| `passing` | usize | Passing sounding count |
| `failing` | usize | Failing sounding count |
| `status` | string | pass/investigate/fail |

---

### 8. `export_s57`

Writes S-57 features to a .000 file.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `features` | JSON array | — | Array of S57Feature objects |
| `path` | string | `/tmp/export.000` | Output file path |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `export_path` | string | Path to .000 file |
| `feature_count` | usize | Features exported |

---

### 9. `compute_epoch_diff`

Computes elevation difference between two DEM epochs (4D monitoring).

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `previous_path` | string | — | Previous survey GeoTIFF (required) |
| `current_path` | string | — | Current survey GeoTIFF (required) |
| `density` | f64 | 2.7 | Rock density (t/m³) |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `fill_volume` | f64 | Fill volume (m³) |
| `cut_volume` | f64 | Cut volume (m³) |
| `net_volume` | f64 | Net volume (m³) |
| `fill_tonnage` | f64 | Fill tonnage (t) |
| `cut_tonnage` | f64 | Cut tonnage (t) |
| `hotspots` | usize | Hotspot cell count |
| `max_fill` | f64 | Maximum fill (m) |
| `max_cut` | f64 | Maximum cut (m) |

---

### 10. `shell_command`

Executes an arbitrary shell command.

**Params**:
| Param | Type | Default | Description |
|---|---|---|---|
| `command` | string | — | Shell command to execute (required) |

**Outputs**:
| Key | Type | Description |
|---|---|---|
| `exit_code` | i32 | Process exit code |
| `stdout` | string | Captured stdout |

---

### 11. `noop`

No-op step for testing. Always succeeds.

**Params**: none  
**Outputs**: none

---

## Complete Example: Mining Daily Workflow

```yaml
name: "Daily Mine Survey"
description: "Ingest drone survey → classify → volume → report"
steps:
  - id: probe
    action: probe_file
    params:
      path: "{{input.survey_file}}"
  - id: classify
    action: classify_ground
    params:
      path: "{{input.survey_file}}"
      cloth_resolution: 0.5
      max_points: 5000000
  - id: volume
    action: compute_volumes
    params:
      current_path: "{{input.dem_file}}"
      reference_path: "flat:105.0"
      bench_interval: 5.0
  - id: report
    action: generate_report
    params:
      output_path: "{{input.output_dir}}/daily_report.html"
```

## Complete Example: Marine QC Pipeline

```yaml
name: "Marine Daily QC"
description: "Ingest MbES → CUBE → S-44 → report"
steps:
  - id: probe
    action: probe_file
    params:
      path: "{{input.all_file}}"
  - id: cube
    action: generate_cube_surface
    params:
      soundings: "{{input.soundings}}"
      resolution: 1.0
  - id: s44
    action: check_s44_compliance
    params:
      soundings: "{{input.soundings}}"
      order: "order_1a"
  - id: report
    action: generate_report
    params:
      output_path: "{{input.output_dir}}/marine_qc.html"
```
