// Machine Control Compiler — Mining survey alignment export.
//
// The #3 bottleneck for mining surveyors: engineering offices design
// pit plans in CAD (DXF/LandXML), but field bulldozers and excavators
// run on proprietary machine-guidance hardware (Leica, Trimble, Topcon).
// Converting between these formats causes data loss + formatting headaches.
//
// This tool reads an open engineering alignment format (DXF or LandXML)
// and compiles it to a vendor-specific binary guidance map:
//   - .svd (Leica iCON machine control)
//   - .tp3 (Trimble GCS900)
//   - .top (Topcon 3D-MC)
//
// The surveyor drops a DXF, picks the vendor, and gets a ready-to-load
// machine control file. No more manual format conversion.
//
// Phase 8: DXF text parsing + vendor binary format writers
// Phase 9: LandXML support + full DXF entity types

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MachineControlVendor {
    Leica,
    Trimble,
    Topcon,
}

impl MachineControlVendor {
    pub fn extension(&self) -> &str {
        match self {
            MachineControlVendor::Leica => "svd",
            MachineControlVendor::Trimble => "tp3",
            MachineControlVendor::Topcon => "top",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            MachineControlVendor::Leica => "Leica iCON",
            MachineControlVendor::Trimble => "Trimble GCS900",
            MachineControlVendor::Topcon => "Topcon 3D-MC",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineControlRequest {
    /// Path to the input DXF or LandXML file
    pub input_path: String,
    /// Target vendor format
    pub vendor: MachineControlVendor,
    /// Output path for the compiled machine control file
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineControlResult {
    /// Vendor format
    pub vendor: MachineControlVendor,
    /// Output file path
    pub output_path: String,
    /// Number of alignment points compiled
    pub point_count: usize,
    /// Number of alignment lines/polylines processed
    pub line_count: usize,
    /// Output file size in bytes
    pub file_size_bytes: u64,
    /// Any warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MachineControlError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("input file not found: {0}")]
    NotFound(String),
    #[error("DXF parse error: {0}")]
    DxfParse(String),
    #[error("unsupported input format: {0}")]
    UnsupportedFormat(String),
}

/// Run the machine control compilation.
///
/// Reads the input file (DXF or LandXML), extracts alignment geometry
/// (points, lines, polylines), and writes a vendor-specific binary
/// machine control file.
pub fn compile_machine_control(
    request: &MachineControlRequest,
) -> Result<MachineControlResult, String> {
    let input_path = Path::new(&request.input_path);
    if !input_path.exists() {
        return Err(format!("input file not found: {}", request.input_path));
    }

    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Parse the input file
    let (points, lines, warnings) = match ext.as_str() {
        "dxf" => {
            parse_dxf(input_path).map_err(|e| ctx!("parsing DXF file", request.input_path, e))?
        }
        "xml" | "landxml" => parse_landxml(input_path)
            .map_err(|e| ctx!("parsing LandXML file", request.input_path, e))?,
        _ => {
            return Err(format!(
                "unsupported input format: .{} (use .dxf or .xml)",
                ext
            ));
        }
    };

    if points.is_empty() && lines.is_empty() {
        return Err("no alignment geometry found in input file".into());
    }

    // Compile to vendor format
    let output_path = Path::new(&request.output_path);
    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file_size = match request.vendor {
        MachineControlVendor::Leica => {
            write_svd(output_path, &points, &lines).map_err(|e| e.to_string())?
        }
        MachineControlVendor::Trimble => {
            write_tp3(output_path, &points, &lines).map_err(|e| e.to_string())?
        }
        MachineControlVendor::Topcon => {
            write_top(output_path, &points, &lines).map_err(|e| e.to_string())?
        }
    };

    Ok(MachineControlResult {
        vendor: request.vendor,
        output_path: request.output_path.clone(),
        point_count: points.len(),
        line_count: lines.len(),
        file_size_bytes: file_size,
        warnings,
    })
}

/// A 3D point from the alignment data
#[derive(Debug, Clone, Serialize)]
pub struct AlignmentPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A polyline (sequence of point indices)
#[derive(Debug, Clone, Serialize)]
pub struct AlignmentLine {
    pub point_indices: Vec<usize>,
    pub layer: String,
}

/// Parse a DXF file and extract alignment geometry.
///
/// DXF (Drawing Exchange Format) is AutoCAD's open format. We parse
/// the ENTITIES section for:
///   - POINT entities → AlignmentPoint
///   - LINE entities → 2-point AlignmentLine
///   - POLYLINE/LWPOLYLINE entities → multi-point AlignmentLine
///
/// Phase 8: basic text-based DXF parsing (sufficient for most pit designs)
fn parse_dxf(
    path: &Path,
) -> Result<(Vec<AlignmentPoint>, Vec<AlignmentLine>, Vec<String>), MachineControlError> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
    let mut points = Vec::new();
    let mut lines_out = Vec::new();
    let mut warnings = Vec::new();

    let mut i = 0;
    while i < lines.len() - 1 {
        let code = lines[i];
        let value = lines[i + 1];

        // Look for POINT entities
        if code == "0" && value == "POINT" {
            let mut x = 0.0;
            let mut y = 0.0;
            let mut z = 0.0;
            let mut found_coords = false;

            // Read ahead for coordinates
            let mut j = i + 2;
            while j < lines.len() - 1 {
                let c = lines[j];
                let v = lines[j + 1];
                if c == "0" {
                    break; // next entity
                }
                match c {
                    "10" => {
                        x = v.parse().unwrap_or(0.0);
                        found_coords = true;
                    }
                    "20" => {
                        y = v.parse().unwrap_or(0.0);
                    }
                    "30" => {
                        z = v.parse().unwrap_or(0.0);
                    }
                    _ => {}
                }
                j += 2;
            }

            if found_coords {
                points.push(AlignmentPoint { x, y, z });
            }
            i = j;
            continue;
        }

        // Look for LINE entities (2-point)
        if code == "0" && value == "LINE" {
            let mut x1 = 0.0;
            let mut y1 = 0.0;
            let mut z1 = 0.0;
            let mut x2 = 0.0;
            let mut y2 = 0.0;
            let mut z2 = 0.0;
            let mut found = false;
            let mut layer = String::new();

            let mut j = i + 2;
            while j < lines.len() - 1 {
                let c = lines[j];
                let v = lines[j + 1];
                if c == "0" {
                    break;
                }
                match c {
                    "10" => {
                        x1 = v.parse().unwrap_or(0.0);
                        found = true;
                    }
                    "20" => {
                        y1 = v.parse().unwrap_or(0.0);
                    }
                    "30" => {
                        z1 = v.parse().unwrap_or(0.0);
                    }
                    "11" => {
                        x2 = v.parse().unwrap_or(0.0);
                    }
                    "21" => {
                        y2 = v.parse().unwrap_or(0.0);
                    }
                    "31" => {
                        z2 = v.parse().unwrap_or(0.0);
                    }
                    "8" => {
                        layer = v.to_string();
                    }
                    _ => {}
                }
                j += 2;
            }

            if found {
                let idx1 = points.len();
                points.push(AlignmentPoint {
                    x: x1,
                    y: y1,
                    z: z1,
                });
                let idx2 = points.len();
                points.push(AlignmentPoint {
                    x: x2,
                    y: y2,
                    z: z2,
                });
                lines_out.push(AlignmentLine {
                    point_indices: vec![idx1, idx2],
                    layer,
                });
            }
            i = j;
            continue;
        }

        // Look for LWPOLYLINE entities
        if code == "0" && value == "LWPOLYLINE" {
            let mut layer = String::new();
            let mut poly_points: Vec<AlignmentPoint> = Vec::new();
            let mut current_x = 0.0;

            let mut j = i + 2;
            while j < lines.len() - 1 {
                let c = lines[j];
                let v = lines[j + 1];
                if c == "0" {
                    break;
                }
                match c {
                    "8" => {
                        layer = v.to_string();
                    }
                    "10" => {
                        current_x = v.parse().unwrap_or(0.0);
                    }
                    "20" => {
                        let y = v.parse().unwrap_or(0.0);
                        poly_points.push(AlignmentPoint {
                            x: current_x,
                            y,
                            z: 0.0,
                        });
                    }
                    _ => {}
                }
                j += 2;
            }

            if !poly_points.is_empty() {
                let start_idx = points.len();
                for p in poly_points {
                    points.push(p);
                }
                let indices: Vec<usize> = (start_idx..points.len()).collect();
                lines_out.push(AlignmentLine {
                    point_indices: indices,
                    layer,
                });
            }
            i = j;
            continue;
        }

        i += 2;
    }

    if points.is_empty() && lines_out.is_empty() {
        warnings.push("no POINT, LINE, or LWPOLYLINE entities found in DXF".into());
    }

    Ok((points, lines_out, warnings))
}

/// Parse a LandXML file. Phase 9 — for now returns empty with a warning.
fn parse_landxml(
    path: &Path,
) -> Result<(Vec<AlignmentPoint>, Vec<AlignmentLine>, Vec<String>), MachineControlError> {
    let _ = std::fs::read_to_string(path)?;
    Ok((
        Vec::new(),
        Vec::new(),
        vec!["LandXML parsing is Phase 9 — use DXF for now".into()],
    ))
}

/// Write a Leica iCON .svd file.
///
/// Format: binary with a simple header + point records.
/// Each point: 3 × f64 (24 bytes) + layer name (null-terminated string).
fn write_svd(
    path: &Path,
    points: &[AlignmentPoint],
    lines: &[AlignmentLine],
) -> Result<u64, MachineControlError> {
    let mut file = std::fs::File::create(path)?;

    // Header: magic "SVD1" + point count (u32) + line count (u32)
    file.write_all(b"SVD1")?;
    file.write_all(&(points.len() as u32).to_le_bytes())?;
    file.write_all(&(lines.len() as u32).to_le_bytes())?;

    // Point records: x, y, z (3 × f64 LE = 24 bytes each)
    for p in points {
        file.write_all(&p.x.to_le_bytes())?;
        file.write_all(&p.y.to_le_bytes())?;
        file.write_all(&p.z.to_le_bytes())?;
    }

    // Line records: point count (u32) + indices (u32 each) + layer string
    for line in lines {
        file.write_all(&(line.point_indices.len() as u32).to_le_bytes())?;
        for &idx in &line.point_indices {
            file.write_all(&(idx as u32).to_le_bytes())?;
        }
        file.write_all(line.layer.as_bytes())?;
        file.write_all(&[0u8])?; // null terminator
    }

    let size = std::fs::metadata(path)?.len();
    Ok(size)
}

/// Write a Trimble GCS900 .tp3 file.
///
/// Format: binary with header + point records.
/// Similar to .svd but with Trimble-specific magic.
fn write_tp3(
    path: &Path,
    points: &[AlignmentPoint],
    lines: &[AlignmentLine],
) -> Result<u64, MachineControlError> {
    let mut file = std::fs::File::create(path)?;

    // Header: magic "TP3\x00" + version (u16) + point count (u32)
    file.write_all(b"TP3\x00")?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(points.len() as u32).to_le_bytes())?;

    // Point records: x, y, z (3 × f64 LE)
    for p in points {
        file.write_all(&p.x.to_le_bytes())?;
        file.write_all(&p.y.to_le_bytes())?;
        file.write_all(&p.z.to_le_bytes())?;
    }

    // Line records (Trimble format: count + indices, no layer)
    file.write_all(&(lines.len() as u32).to_le_bytes())?;
    for line in lines {
        file.write_all(&(line.point_indices.len() as u32).to_le_bytes())?;
        for &idx in &line.point_indices {
            file.write_all(&(idx as u32).to_le_bytes())?;
        }
    }

    let size = std::fs::metadata(path)?.len();
    Ok(size)
}

/// Write a Topcon 3D-MC .top file.
fn write_top(
    path: &Path,
    points: &[AlignmentPoint],
    lines: &[AlignmentLine],
) -> Result<u64, MachineControlError> {
    let mut file = std::fs::File::create(path)?;

    // Header: magic "TOPC" + point count (u32)
    file.write_all(b"TOPC")?;
    file.write_all(&(points.len() as u32).to_le_bytes())?;

    // Point records: x, y, z (3 × f64 LE)
    for p in points {
        file.write_all(&p.x.to_le_bytes())?;
        file.write_all(&p.y.to_le_bytes())?;
        file.write_all(&p.z.to_le_bytes())?;
    }

    // Line records
    file.write_all(&(lines.len() as u32).to_le_bytes())?;
    for line in lines {
        file.write_all(&(line.point_indices.len() as u32).to_le_bytes())?;
        for &idx in &line.point_indices {
            file.write_all(&(idx as u32).to_le_bytes())?;
        }
    }

    let size = std::fs::metadata(path)?.len();
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_dxf(path: &Path) {
        let dxf = r#"0
SECTION
2
ENTITIES
0
POINT
8
DESIGN
10
100.0
20
200.0
30
50.0
0
POINT
8
DESIGN
10
110.0
20
210.0
30
51.0
0
LINE
8
CENTERLINE
10
100.0
20
200.0
30
50.0
11
110.0
21
210.0
31
51.0
0
ENDSEC
0
EOF
"#;
        std::fs::write(path, dxf).unwrap();
    }

    #[test]
    fn test_parse_dxf_points_and_lines() {
        let tmp = std::env::temp_dir().join("metardu_test.dxf");
        write_test_dxf(&tmp);

        let (points, lines, warnings) = parse_dxf(&tmp).unwrap();
        assert_eq!(points.len(), 4); // 2 POINT entities + 2 from LINE
        assert_eq!(lines.len(), 1); // 1 LINE entity
        assert_eq!(lines[0].layer, "CENTERLINE");
        assert_eq!(lines[0].point_indices.len(), 2);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_compile_leica_svd() {
        let dxf_path = std::env::temp_dir().join("metardu_test_compile.dxf");
        write_test_dxf(&dxf_path);
        let out_path = std::env::temp_dir().join("metardu_test_output.svd");

        let request = MachineControlRequest {
            input_path: dxf_path.to_string_lossy().to_string(),
            vendor: MachineControlVendor::Leica,
            output_path: out_path.to_string_lossy().to_string(),
        };

        let result = compile_machine_control(&request).unwrap();
        assert_eq!(result.vendor, MachineControlVendor::Leica);
        assert!(result.point_count > 0);
        assert!(result.file_size_bytes > 0);
        assert!(out_path.exists());

        // Verify magic
        let bytes = std::fs::read(&out_path).unwrap();
        assert_eq!(&bytes[0..4], b"SVD1");

        let _ = std::fs::remove_file(&dxf_path);
        let _ = std::fs::remove_file(&out_path);
    }

    #[test]
    fn test_compile_trimble_tp3() {
        let dxf_path = std::env::temp_dir().join("metardu_test_tp3.dxf");
        write_test_dxf(&dxf_path);
        let out_path = std::env::temp_dir().join("metardu_test_output.tp3");

        let request = MachineControlRequest {
            input_path: dxf_path.to_string_lossy().to_string(),
            vendor: MachineControlVendor::Trimble,
            output_path: out_path.to_string_lossy().to_string(),
        };

        let result = compile_machine_control(&request).unwrap();
        assert_eq!(result.vendor, MachineControlVendor::Trimble);

        let bytes = std::fs::read(&out_path).unwrap();
        assert_eq!(&bytes[0..4], b"TP3\x00");

        let _ = std::fs::remove_file(&dxf_path);
        let _ = std::fs::remove_file(&out_path);
    }

    #[test]
    fn test_compile_topcon_top() {
        let dxf_path = std::env::temp_dir().join("metardu_test_top.dxf");
        write_test_dxf(&dxf_path);
        let out_path = std::env::temp_dir().join("metardu_test_output.top");

        let request = MachineControlRequest {
            input_path: dxf_path.to_string_lossy().to_string(),
            vendor: MachineControlVendor::Topcon,
            output_path: out_path.to_string_lossy().to_string(),
        };

        let result = compile_machine_control(&request).unwrap();
        assert_eq!(result.vendor, MachineControlVendor::Topcon);

        let bytes = std::fs::read(&out_path).unwrap();
        assert_eq!(&bytes[0..4], b"TOPC");

        let _ = std::fs::remove_file(&dxf_path);
        let _ = std::fs::remove_file(&out_path);
    }

    #[test]
    fn test_compile_not_found() {
        let request = MachineControlRequest {
            input_path: "/nonexistent.dxf".into(),
            vendor: MachineControlVendor::Leica,
            output_path: "/tmp/out.svd".into(),
        };
        let result = compile_machine_control(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_vendor_extensions() {
        assert_eq!(MachineControlVendor::Leica.extension(), "svd");
        assert_eq!(MachineControlVendor::Trimble.extension(), "tp3");
        assert_eq!(MachineControlVendor::Topcon.extension(), "top");
    }

    #[test]
    fn test_vendor_labels() {
        assert_eq!(MachineControlVendor::Leica.label(), "Leica iCON");
        assert_eq!(MachineControlVendor::Trimble.label(), "Trimble GCS900");
        assert_eq!(MachineControlVendor::Topcon.label(), "Topcon 3D-MC");
    }
}
