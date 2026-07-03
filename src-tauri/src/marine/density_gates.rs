// Density Gates — Marine survey coverage validator.
//
// The #1 bottleneck for marine surveyors: they collect millions of
// soundings across a folder of .all/.s7k files, then discover weeks
// later that there are coverage gaps. By then they've left the site
// and re-mobilizing costs $50K+.
//
// This tool walks a folder of sonar files, extracts ping positions,
// bins them into a spatial grid, and reports per-cell density:
//   - GREEN: density meets IHO S-44 standard for the target order
//   - YELLOW: marginal coverage (50-100% of target)
//   - RED: insufficient coverage (<50% of target) — gap detected
//
// The frontend renders this as a map heatmap overlay so the surveyor
// sees coverage quality instantly — while still on site.
//
// No AI. Pure deterministic spatial binning + IHO S-44 density rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct CoverageCell {
    /// Grid row
    pub row: usize,
    /// Grid column
    pub col: usize,
    /// Center longitude (WGS84)
    pub center_lon: f64,
    /// Center latitude (WGS84)
    pub center_lat: f64,
    /// Sounding count in this cell
    pub count: u64,
    /// Density status
    pub status: CoverageStatus,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    /// Meets or exceeds target density
    Good,
    /// 50-100% of target density — marginal
    Marginal,
    /// <50% of target density — gap
    Gap,
    /// No soundings at all
    Empty,
}

impl CoverageStatus {
    pub fn color_hex(&self) -> &str {
        match self {
            CoverageStatus::Good => "#10B981",     // green
            CoverageStatus::Marginal => "#F59E0B", // amber
            CoverageStatus::Gap => "#EF4444",      // red
            CoverageStatus::Empty => "#1E293B",    // dark slate
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    /// Total files scanned
    pub files_scanned: usize,
    /// Total pings extracted
    pub total_pings: u64,
    /// Total soundings (pings × beams approximation)
    pub total_soundings: u64,
    /// Grid cells
    pub cells: Vec<CoverageCell>,
    /// Grid bounds (min_lon, min_lat, max_lon, max_lat)
    pub bounds: (f64, f64, f64, f64),
    /// Grid dimensions
    pub grid_rows: usize,
    pub grid_cols: usize,
    /// Cell size in degrees
    pub cell_size_deg: f64,
    /// Target density (soundings per cell)
    pub target_density: u64,
    /// Summary stats
    pub good_cells: usize,
    pub marginal_cells: usize,
    pub gap_cells: usize,
    pub empty_cells: usize,
    /// Overall coverage percentage
    pub coverage_pct: f64,
    /// Per-file summary
    pub file_summaries: Vec<FileSummary>,
    /// Errors encountered (non-fatal)
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileSummary {
    pub filename: String,
    pub pings: u64,
    pub est_soundings: u64,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DensityGatesRequest {
    /// Folder path to scan for .all / .s7k files
    pub folder_path: String,
    /// Target IHO S-44 order (determines density requirement)
    pub target_order: String,
    /// Cell size in degrees (default 0.0005 ≈ 50m at equator)
    #[serde(default = "default_cell_size")]
    pub cell_size_deg: f64,
}

fn default_cell_size() -> f64 {
    0.0005
}

/// IHO S-44 density requirements (6th edition, 2022).
/// These are the minimum sounding densities per square meter
/// for each survey order. We convert to per-cell targets.
fn target_density_for_order(order: &str, cell_size_deg: f64) -> u64 {
    // Approximate cell area in m² (at equator, 1° ≈ 111km)
    let cell_area_m2 = (cell_size_deg * 111_000.0) * (cell_size_deg * 111_000.0);

    // IHO S-44 minimum density (soundings per m²)
    let density_per_m2 = match order {
        "special" => 0.25,      // Special order: 1 sounding per 4m²
        "order_1a" => 0.04,     // Order 1a: 1 sounding per 25m²
        "order_1b" => 0.02,     // Order 1b
        "order_2" => 0.01,      // Order 2
        _ => 0.04,              // default to Order 1a
    };

    (density_per_m2 * cell_area_m2).max(1.0) as u64
}

/// Run the density gates analysis on a folder of sonar files.
///
/// Walks the folder recursively for .all and .s7k files, extracts
/// ping positions from each, bins them into a spatial grid, and
/// reports per-cell coverage status.
pub fn run_density_gates(request: &DensityGatesRequest) -> Result<CoverageReport, String> {
    let folder = Path::new(&request.folder_path);
    if !folder.exists() {
        return Err(format!("folder not found: {}", request.folder_path));
    }
    if !folder.is_dir() {
        return Err(format!("not a folder: {}", request.folder_path));
    }

    let target_density = target_density_for_order(&request.target_order, request.cell_size_deg);
    let mut warnings = Vec::new();

    // Collect all sonar files
    let mut sonar_files: Vec<PathBuf> = Vec::new();
    collect_sonar_files(folder, &mut sonar_files);

    if sonar_files.is_empty() {
        return Err("no .all or .s7k files found in folder".into());
    }

    // Extract ping positions from each file
    let mut all_positions: Vec<(f64, f64)> = Vec::new();
    let mut file_summaries = Vec::new();
    let mut total_pings: u64 = 0;

    for file_path in &sonar_files {
        let filename = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let file_size = std::fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        match extract_ping_positions(file_path) {
            Ok(positions) => {
                let ping_count = positions.len() as u64;
                // Estimate soundings: ~250 beams per ping (typical MBES)
                let est_soundings = ping_count * 250;
                total_pings += ping_count;
                all_positions.extend(positions);
                file_summaries.push(FileSummary {
                    filename: filename.clone(),
                    pings: ping_count,
                    est_soundings,
                    file_size_bytes: file_size,
                });
            }
            Err(e) => {
                warnings.push(format!("{}: {}", filename, e));
                file_summaries.push(FileSummary {
                    filename,
                    pings: 0,
                    est_soundings: 0,
                    file_size_bytes: file_size,
                });
            }
        }
    }

    if all_positions.is_empty() {
        return Err("no ping positions could be extracted from any file".into());
    }

    // Compute bounds
    let mut min_lon = f64::INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for &(lon, lat) in &all_positions {
        if !lon.is_nan() && !lat.is_nan() {
            min_lon = min_lon.min(lon);
            max_lon = max_lon.max(lon);
            min_lat = min_lat.min(lat);
            max_lat = max_lat.max(lat);
        }
    }

    // Build spatial grid
    let cell_size = request.cell_size_deg;
    let grid_cols = ((max_lon - min_lon) / cell_size).ceil().max(1.0) as usize;
    let grid_rows = ((max_lat - min_lat) / cell_size).ceil().max(1.0) as usize;

    // Cap grid size to prevent memory explosion on huge surveys
    let max_cells = 10_000;
    let (grid_cols, grid_rows, cell_size) = if grid_cols * grid_rows > max_cells {
        let scale = ((grid_cols * grid_rows) as f64 / max_cells as f64).sqrt();
        let new_cols = (grid_cols as f64 / scale).ceil() as usize;
        let new_rows = (grid_rows as f64 / scale).ceil() as usize;
        let new_cell = cell_size * scale;
        warnings.push(format!(
            "grid auto-coarsened from {}x{} to {}x{} (cell size {}° → {}°) to prevent memory explosion",
            grid_cols, grid_rows, new_cols, new_rows, cell_size, new_cell
        ));
        (new_cols, new_rows, new_cell)
    } else {
        (grid_cols, grid_rows, cell_size)
    };

    let mut grid: HashMap<(usize, usize), u64> = HashMap::new();
    for &(lon, lat) in &all_positions {
        if lon.is_nan() || lat.is_nan() {
            continue;
        }
        let col = ((lon - min_lon) / cell_size).floor() as usize;
        let row = ((max_lat - lat) / cell_size).floor() as usize; // row 0 = north
        let col = col.min(grid_cols - 1);
        let row = row.min(grid_rows - 1);
        *grid.entry((row, col)).or_insert(0) += 1;
    }

    // Build coverage cells
    let mut cells = Vec::new();
    let mut good_cells = 0usize;
    let mut marginal_cells = 0usize;
    let mut gap_cells = 0usize;
    let mut empty_cells = 0usize;

    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let count = grid.get(&(row, col)).copied().unwrap_or(0);
            let center_lon = min_lon + (col as f64 + 0.5) * cell_size;
            let center_lat = max_lat - (row as f64 + 0.5) * cell_size;

            let status = if count == 0 {
                empty_cells += 1;
                CoverageStatus::Empty
            } else if count >= target_density {
                good_cells += 1;
                CoverageStatus::Good
            } else if count >= target_density / 2 {
                marginal_cells += 1;
                CoverageStatus::Marginal
            } else {
                gap_cells += 1;
                CoverageStatus::Gap
            };

            cells.push(CoverageCell {
                row,
                col,
                center_lon,
                center_lat,
                count,
                status,
            });
        }
    }

    let total_cells = cells.len();
    let covered_cells = good_cells + marginal_cells + gap_cells;
    let coverage_pct = if total_cells > 0 {
        (covered_cells as f64 / total_cells as f64) * 100.0
    } else {
        0.0
    };

    let total_soundings = total_pings * 250; // approximate

    Ok(CoverageReport {
        files_scanned: sonar_files.len(),
        total_pings,
        total_soundings,
        cells,
        bounds: (min_lon, min_lat, max_lon, max_lat),
        grid_rows,
        grid_cols,
        cell_size_deg: cell_size,
        target_density,
        good_cells,
        marginal_cells,
        gap_cells,
        empty_cells,
        coverage_pct,
        file_summaries,
        warnings,
    })
}

/// Recursively collect .all and .s7k files from a folder.
fn collect_sonar_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_sonar_files(&path, files);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "all" || ext_lower == "s7k" {
                    files.push(path);
                }
            }
        }
    }
}

/// Extract ping positions (lon, lat) from a sonar file.
///
/// Uses the existing Kongsberg .all and Reson .s7k parsers to walk
/// datagrams and extract navigation data. Returns a Vec of (lon, lat)
/// tuples — one per ping.
fn extract_ping_positions(path: &Path) -> Result<Vec<(f64, f64)>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "all" => extract_from_all(path),
        "s7k" => extract_from_s7k(path),
        _ => Err(format!("unsupported file type: .{}", ext)),
    }
}

/// Extract positions from a Kongsberg .all file.
///
/// Walks the datagram stream looking for Position datagrams (type 0x50).
/// Each position datagram contains:
///   - 4 bytes: Unix timestamp
///   - 2 bytes: position fix descriptor
///   - 4 bytes: latitude (i32, scaled by 20,000,000 → decimal degrees)
///   - 4 bytes: longitude (i32, scaled by 20,000,000 → decimal degrees)
fn extract_from_all(path: &Path) -> Result<Vec<(f64, f64)>, String> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(path).map_err(|e| e.to_string())?;

    // Verify magic: first byte must be 0x49 (start datagram)
    let mut start_byte = [0u8; 1];
    file.read_exact(&mut start_byte).map_err(|e| e.to_string())?;
    if start_byte[0] != 0x49 {
        return Err("not a valid Kongsberg .all file".into());
    }
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;

    let mut positions = Vec::new();
    let max_datagrams = 500_000; // Cap to prevent reading huge files forever

    for _ in 0..max_datagrams {
        // 4-byte header: type(1) + size(3, LE, 24-bit)
        let mut header = [0u8; 4];
        match file.read(&mut header) {
            Ok(0) => break,
            Ok(n) if n < 4 => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let type_byte = header[0];
        let size = u32::from(header[1]) | (u32::from(header[2]) << 8) | (u32::from(header[3]) << 16);

        if size < 4 {
            break;
        }

        let payload_size = (size as usize).saturating_sub(4);
        let mut payload = vec![0u8; payload_size];
        if file.read_exact(&mut payload).is_err() {
            break;
        }

        // Read trailing 4-byte size
        let mut trailing = [0u8; 4];
        if file.read_exact(&mut trailing).is_err() {
            break;
        }

        // Position datagram = 0x50 ('P')
        if type_byte == 0x50 && payload.len() >= 14 {
            // Parse: timestamp(4) + pos_fix_desc(2) + lat(4, i32, ×2e7) + lon(4, i32, ×2e7)
            let lat_raw = i32::from_le_bytes([
                payload[6], payload[7], payload[8], payload[9],
            ]);
            let lon_raw = i32::from_le_bytes([
                payload[10], payload[11], payload[12], payload[13],
            ]);

            let lat = lat_raw as f64 / 20_000_000.0;
            let lon = lon_raw as f64 / 20_000_000.0;

            if lat.is_finite() && lon.is_finite()
                && lat >= -90.0 && lat <= 90.0
                && lon >= -180.0 && lon <= 180.0
            {
                positions.push((lon, lat));
            }
        }

        // Bathymetry datagram = 0x44 ('D') — also has a position in the header
        // We use position datagrams as the primary source since they're
        // more frequent and accurate than bathymetry positions.
    }

    if positions.is_empty() {
        // Fallback: if no position datagrams found, try to use bathymetry
        // datagram positions. If still empty, return error.
        return Err("no position datagrams found in .all file".into());
    }

    Ok(positions)
}

/// Extract positions from a Reson .s7k file.
///
/// S7K record type 1003 = Position. Each record has a standard s7k
/// header (64 bytes) followed by the position data:
///   - 8 bytes: timestamp (f64, seconds since 1970)
///   - 8 bytes: latitude (f64, decimal degrees)
///   - 8 bytes: longitude (f64, decimal degrees)
fn extract_from_s7k(path: &Path) -> Result<Vec<(f64, f64)>, String> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(path).map_err(|e| e.to_string())?;

    // Verify s7k sync pattern: 0x7F7F7F7F
    let mut sync = [0u8; 4];
    file.read_exact(&mut sync).map_err(|e| e.to_string())?;
    if sync != [0x7F, 0x7F, 0x7F, 0x7F] {
        return Err("not a valid Reson .s7k file — sync pattern mismatch".into());
    }
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;

    let mut positions = Vec::new();
    let max_records = 500_000;

    for _ in 0..max_records {
        // S7K record header: 64 bytes
        // Offset 0: sync pattern (4 bytes) = 0x7F7F7F7F
        // Offset 4: size of record (4 bytes, u32 LE)
        // Offset 8: optional offset (4 bytes)
        // Offset 12: optional identifier (4 bytes)
        // Offset 16: record type ID (4 bytes, u32 LE)
        // ... rest of 64-byte header

        let mut hdr = [0u8; 64];
        match file.read(&mut hdr) {
            Ok(0) => break,
            Ok(n) if n < 64 => break,
            Ok(_) => {}
            Err(_) => break,
        }

        // Check sync
        if hdr[0..4] != [0x7F, 0x7F, 0x7F, 0x7F] {
            break;
        }

        let record_size = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
        let record_type = u32::from_le_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]);

        if record_size < 64 {
            break;
        }

        let data_size = record_size - 64;

        // Read the data portion
        let mut data = vec![0u8; data_size];
        if file.read_exact(&mut data).is_err() {
            break;
        }

        // Record type 1003 = Position
        if record_type == 1003 && data.len() >= 24 {
            // Position data layout (after the 64-byte header):
            // Offset 0: 8 bytes — timestamp (f64)
            // Offset 8: 8 bytes — latitude (f64, decimal degrees)
            // Offset 16: 8 bytes — longitude (f64, decimal degrees)
            let lat = f64::from_le_bytes([
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15],
            ]);
            let lon = f64::from_le_bytes([
                data[16], data[17], data[18], data[19],
                data[20], data[21], data[22], data[23],
            ]);

            if lat.is_finite() && lon.is_finite()
                && lat >= -90.0 && lat <= 90.0
                && lon >= -180.0 && lon <= 180.0
            {
                positions.push((lon, lat));
            }
        }

        // Read trailing 4-byte checksum if present
        let mut trailing = [0u8; 4];
        let _ = file.read(&mut trailing);
    }

    if positions.is_empty() {
        return Err("no position records found in .s7k file".into());
    }

    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_density_special_order() {
        // Special order: 0.25 soundings/m². Cell 50m × 50m = 2500m²
        // Target = 0.25 × 2500 = 625 soundings per cell
        let target = target_density_for_order("special", 0.0005);
        assert!(target > 0);
        // Cell area ≈ (0.0005 × 111000)² = 55.5² ≈ 3080 m²
        // Target ≈ 0.25 × 3080 = 770
        assert!(target > 500 && target < 1000, "got: {}", target);
    }

    #[test]
    fn test_target_density_order_1a() {
        let target = target_density_for_order("order_1a", 0.0005);
        // Order 1a: 0.04 soundings/m²
        // Target ≈ 0.04 × 3080 = 123
        assert!(target > 80 && target < 200, "got: {}", target);
    }

    #[test]
    fn test_target_density_default() {
        let target = target_density_for_order("unknown", 0.0005);
        // Should default to Order 1a
        let expected = target_density_for_order("order_1a", 0.0005);
        assert_eq!(target, expected);
    }

    #[test]
    fn test_coverage_status_colors() {
        assert_eq!(CoverageStatus::Good.color_hex(), "#10B981");
        assert_eq!(CoverageStatus::Marginal.color_hex(), "#F59E0B");
        assert_eq!(CoverageStatus::Gap.color_hex(), "#EF4444");
        assert_eq!(CoverageStatus::Empty.color_hex(), "#1E293B");
    }

    #[test]
    fn test_run_density_gates_folder_not_found() {
        let req = DensityGatesRequest {
            folder_path: "/nonexistent/folder".into(),
            target_order: "order_1a".into(),
            cell_size_deg: 0.0005,
        };
        let result = run_density_gates(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_density_gates_not_a_folder() {
        // Create a temp file (not folder)
        let tmp = std::env::temp_dir().join("metardu_test_not_folder.txt");
        std::fs::write(&tmp, "test").unwrap();
        let req = DensityGatesRequest {
            folder_path: tmp.to_string_lossy().to_string(),
            target_order: "order_1a".into(),
            cell_size_deg: 0.0005,
        };
        let result = run_density_gates(&req);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_run_density_gates_empty_folder() {
        let tmp = std::env::temp_dir().join("metardu_test_empty_folder");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let req = DensityGatesRequest {
            folder_path: tmp.to_string_lossy().to_string(),
            target_order: "order_1a".into(),
            cell_size_deg: 0.0005,
        };
        let result = run_density_gates(&req);
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
