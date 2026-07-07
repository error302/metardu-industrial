// Shapefile reader/writer — Sprint 15.
//
// Pure-Rust parser for ESRI Shapefiles (.shp + .shx + .dbf). The #1
// interchange format for mining plans, cadastral data, and engineering
// drawings. Surveyors get Shapefiles from mine planning software
// (Surpac, Datamine, Vulcan) and need to overlay them on the map.
//
// Supported shape types:
//   - Point (1)
//   - Polyline (3) — multi-part lines
//   - Polygon (5) — multi-part polygons with holes
//   - MultiPoint (8)
//
// Not supported (niche, defer to customer request):
//   - PointZ (11), PolylineZ (13), PolygonZ (15), MultiPointZ (18)
//   - PointM (21), PolylineM (23), PolygonM (25), MultiPointM (28)
//   - MultiPatch (31)
//
// The .dbf (dBase III+) parser reads attribute columns. Writing .dbf
// is limited to the most common column types (Numeric, Character, Date).
//
// References:
//   - ESRI Shapefile Technical Description (July 1998)
//   - dBase III Plus file format specification

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

// ──────────────────────────────────────────────────────────────────
// Shapefile types
// ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeType {
    Null = 0,
    Point = 1,
    Polyline = 3,
    Polygon = 5,
    MultiPoint = 8,
}

impl ShapeType {
    fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(ShapeType::Null),
            1 => Some(ShapeType::Point),
            3 => Some(ShapeType::Polyline),
            5 => Some(ShapeType::Polygon),
            8 => Some(ShapeType::MultiPoint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Point { x: f64, y: f64 },
    Polyline { parts: Vec<Vec<[f64; 2]>> },
    Polygon { rings: Vec<Vec<[f64; 2]>> },
    MultiPoint { points: Vec<[f64; 2]> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapefileFeature {
    pub geometry: Shape,
    /// Attributes from the .dbf file (column name → value as string)
    pub attributes: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shapefile {
    pub shape_type: ShapeType,
    pub features: Vec<ShapefileFeature>,
    pub bounds: (f64, f64, f64, f64), // min_x, min_y, max_x, max_y
}

// ──────────────────────────────────────────────────────────────────
// Reader
// ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ShapefileError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("invalid shapefile: {0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported shape type: {0}")]
    UnsupportedShapeType(u32),
}

/// Read a Shapefile from a .shp path.
///
/// The .shp path is the primary file; the function also reads the
/// .dbf file (same name, .dbf extension) for attributes. If the .dbf
/// is missing, features will have empty attribute maps.
pub fn read_shapefile(shp_path: &Path) -> Result<Shapefile, ShapefileError> {
    // Read .shp
    let shp_data = std::fs::read(shp_path).map_err(|e| {
        ShapefileError::Io(e)
    })?;
    let (shape_type, features, bounds) = parse_shp(&shp_data)?;

    // Read .dbf for attributes (if it exists)
    let dbf_path = shp_path.with_extension("dbf");
    let attributes_list = if dbf_path.exists() {
        let dbf_data = std::fs::read(&dbf_path)?;
        parse_dbf(&dbf_data)?
    } else {
        vec![std::collections::HashMap::new(); features.len()]
    };

    // Merge attributes into features
    let features: Vec<ShapefileFeature> = features
        .into_iter()
        .enumerate()
        .map(|(i, geom)| ShapefileFeature {
            geometry: geom,
            attributes: attributes_list.get(i).cloned().unwrap_or_default(),
        })
        .collect();

    Ok(Shapefile {
        shape_type,
        features,
        bounds,
    })
}

/// Parse the .shp binary format.
fn parse_shp(data: &[u8]) -> Result<(ShapeType, Vec<Shape>, (f64, f64, f64, f64)), ShapefileError> {
    if data.len() < 100 {
        return Err(ShapefileError::Invalid("file too short for header".to_string()));
    }

    // Header (100 bytes)
    // File code (big-endian): bytes 0-3, should be 9994
    let file_code = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    if file_code != 9994 {
        return Err(ShapefileError::Invalid(format!(
            "invalid file code: {} (expected 9994)",
            file_code
        )));
    }

    // Shape type (little-endian): bytes 32-35
    let shape_type_raw = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
    let shape_type = ShapeType::from_u32(shape_type_raw)
        .ok_or(ShapefileError::UnsupportedShapeType(shape_type_raw))?;

    // Bounds: bytes 36-67 (min_x, min_y, max_x, max_y — little-endian f64)
    let min_x = f64::from_le_bytes(data[36..44].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [36..44]".to_string()))?);
    let min_y = f64::from_le_bytes(data[44..52].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [44..52]".to_string()))?);
    let max_x = f64::from_le_bytes(data[52..60].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [52..60]".to_string()))?);
    let max_y = f64::from_le_bytes(data[60..68].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [60..68]".to_string()))?);

    // Records start at byte 100
    let mut offset = 100usize;
    let mut features = Vec::new();

    while offset + 12 <= data.len() {
        // Record header: record number (big-endian i32) + content length (big-endian i32)
        let _record_num = i32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        let content_len_words = i32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        let content_len_bytes = content_len_words * 2;
        offset += 8;

        if offset + content_len_bytes > data.len() {
            break; // Truncated record
        }

        let record_data = &data[offset..offset + content_len_bytes];
        if let Some(shape) = parse_record(record_data, shape_type)? {
            features.push(shape);
        }
        offset += content_len_bytes;
    }

    Ok((shape_type, features, (min_x, min_y, max_x, max_y)))
}

fn parse_record(data: &[u8], expected_type: ShapeType) -> Result<Option<Shape>, ShapefileError> {
    if data.len() < 4 {
        return Ok(None);
    }
    let rec_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if rec_type == 0 {
        return Ok(None); // Null shape
    }

    let shape = match expected_type {
        ShapeType::Point => {
            if data.len() < 20 {
                return Err(ShapefileError::Invalid("Point record too short".to_string()));
            }
            let x = f64::from_le_bytes(data[4..12].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [4..12]".to_string()))?);
            let y = f64::from_le_bytes(data[12..20].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [12..20]".to_string()))?);
            Shape::Point { x, y }
        }
        ShapeType::MultiPoint => {
            if data.len() < 40 {
                return Err(ShapefileError::Invalid("MultiPoint record too short".to_string()));
            }
            let num_points = i32::from_le_bytes(data[36..40].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [36..40]".to_string()))?) as usize;
            let mut points = Vec::with_capacity(num_points);
            for i in 0..num_points {
                let base = 40 + i * 16;
                if base + 16 > data.len() {
                    break;
                }
                let x = f64::from_le_bytes(data[base..base + 8].try_into().map_err(|_| ShapefileError::Invalid("truncated point data".to_string()))?);
                let y = f64::from_le_bytes(data[base + 8..base + 16].try_into().map_err(|_| ShapefileError::Invalid("truncated point data".to_string()))?);
                points.push([x, y]);
            }
            Shape::MultiPoint { points }
        }
        ShapeType::Polyline | ShapeType::Polygon => {
            if data.len() < 44 {
                return Err(ShapefileError::Invalid("Polyline/Polygon record too short".to_string()));
            }
            let num_parts = i32::from_le_bytes(data[36..40].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [36..40]".to_string()))?) as usize;
            let num_points = i32::from_le_bytes(data[40..44].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [40..44]".to_string()))?) as usize;

            // Parts array: num_parts × i32 (start index of each part)
            let parts_end = 44 + num_parts * 4;
            if parts_end + num_points * 16 > data.len() {
                return Err(ShapefileError::Invalid("Polyline/Polygon points truncated".to_string()));
            }
            let mut part_starts = Vec::with_capacity(num_parts);
            for i in 0..num_parts {
                let base = 44 + i * 4;
                part_starts.push(i32::from_le_bytes(data[base..base + 4].try_into().map_err(|_| ShapefileError::Invalid("truncated part data".to_string()))?) as usize);
            }

            // Points array: num_points × (f64 x, f64 y)
            let mut all_points = Vec::with_capacity(num_points);
            for i in 0..num_points {
                let base = parts_end + i * 16;
                let x = f64::from_le_bytes(data[base..base + 8].try_into().map_err(|_| ShapefileError::Invalid("truncated point data".to_string()))?);
                let y = f64::from_le_bytes(data[base + 8..base + 16].try_into().map_err(|_| ShapefileError::Invalid("truncated point data".to_string()))?);
                all_points.push([x, y]);
            }

            // Split points into parts
            let parts: Vec<Vec<[f64; 2]>> = part_starts
                .iter()
                .enumerate()
                .map(|(i, &start)| {
                    let end = if i + 1 < part_starts.len() {
                        part_starts[i + 1]
                    } else {
                        num_points
                    };
                    all_points[start..end].to_vec()
                })
                .collect();

            if expected_type == ShapeType::Polyline {
                Shape::Polyline { parts }
            } else {
                Shape::Polygon { rings: parts }
            }
        }
        ShapeType::Null => return Ok(None),
    };

    Ok(Some(shape))
}

// ──────────────────────────────────────────────────────────────────
// .dbf (dBase III+) parser
// ──────────────────────────────────────────────────────────────────

fn parse_dbf(data: &[u8]) -> Result<Vec<std::collections::HashMap<String, String>>, ShapefileError> {
    if data.len() < 32 {
        return Err(ShapefileError::Invalid("dbf too short for header".to_string()));
    }

    // Header
    let num_records = u32::from_le_bytes(data[4..8].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [4..8]".to_string()))?) as usize;
    let header_size = u16::from_le_bytes(data[8..10].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [8..10]".to_string()))?) as usize;
    let record_size = u16::from_le_bytes(data[10..12].try_into().map_err(|_| ShapefileError::Invalid("truncated data at offset [10..12]".to_string()))?) as usize;

    // Field descriptors start at byte 32, each is 32 bytes, terminated by 0x0D
    let mut fields: Vec<(String, char, usize)> = Vec::new(); // (name, type, length)
    let mut offset = 32;
    while offset < header_size && offset < data.len() {
        if data[offset] == 0x0D {
            break;
        }
        // Field name: 11 bytes, null-terminated
        let name_end = data[offset..offset + 11].iter().position(|&b| b == 0).unwrap_or(11);
        let name = String::from_utf8_lossy(&data[offset..offset + name_end]).trim().to_string();
        // Field type: byte 11
        let field_type = data[offset + 11] as char;
        // Field length: byte 16
        let field_len = data[offset + 16] as usize;
        fields.push((name, field_type, field_len));
        offset += 32;
    }

    // Records start at header_size
    let mut records = Vec::with_capacity(num_records);
    let mut rec_offset = header_size;
    for _ in 0..num_records {
        if rec_offset + record_size > data.len() {
            break;
        }
        let rec_data = &data[rec_offset..rec_offset + record_size];
        // First byte: deletion flag (0x20 = active, 0x2A = deleted)
        if rec_data[0] == 0x2A {
            rec_offset += record_size;
            continue; // Skip deleted records
        }

        let mut attrs = std::collections::HashMap::new();
        let mut field_offset = 1; // Skip deletion flag
        for (name, _ftype, flen) in &fields {
            if field_offset + flen > rec_data.len() {
                break;
            }
            let val = String::from_utf8_lossy(&rec_data[field_offset..field_offset + flen])
                .trim()
                .to_string();
            attrs.insert(name.clone(), val);
            field_offset += flen;
        }
        records.push(attrs);
        rec_offset += record_size;
    }

    Ok(records)
}

// ──────────────────────────────────────────────────────────────────
// Writer (minimal — Point and Polyline only, for export)
// ──────────────────────────────────────────────────────────────────

/// Write a Shapefile (Point or Polyline) to a .shp path.
///
/// Also writes the .shx (index) and .dbf (attributes) files. Attributes
/// are all written as Character type (simplification — real Shapefiles
/// use Numeric, Date, etc. per column).
pub fn write_shapefile(
    shp_path: &Path,
    features: &[ShapefileFeature],
) -> Result<(), ShapefileError> {
    if features.is_empty() {
        return Err(ShapefileError::Invalid("no features to write".to_string()));
    }

    // Determine shape type from first feature
    let shape_type = match &features[0].geometry {
        Shape::Point { .. } => ShapeType::Point,
        Shape::Polyline { .. } => ShapeType::Polyline,
        Shape::Polygon { .. } => ShapeType::Polygon,
        Shape::MultiPoint { .. } => ShapeType::MultiPoint,
    };

    // Compute bounds
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for f in features {
        let pts: Vec<[f64; 2]> = match &f.geometry {
            Shape::Point { x, y } => vec![[*x, *y]],
            Shape::Polyline { parts } => parts.iter().flatten().cloned().collect(),
            Shape::Polygon { rings } => rings.iter().flatten().cloned().collect(),
            Shape::MultiPoint { points } => points.clone(),
        };
        for [x, y] in pts {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    // Build .shp data
    let mut shp_data = Vec::new();
    // Header (100 bytes) — we'll fill file length at the end
    shp_data.extend_from_slice(&9994i32.to_be_bytes()); // File code
    shp_data.extend_from_slice(&[0u8; 20]); // Unused
    shp_data.extend_from_slice(&0u32.to_be_bytes()); // File length (fill later)
    shp_data.extend_from_slice(&1000u32.to_le_bytes()); // Version
    shp_data.extend_from_slice(&(shape_type as u32).to_le_bytes());
    shp_data.extend_from_slice(&min_x.to_le_bytes());
    shp_data.extend_from_slice(&min_y.to_le_bytes());
    shp_data.extend_from_slice(&max_x.to_le_bytes());
    shp_data.extend_from_slice(&max_y.to_le_bytes());
    shp_data.extend_from_slice(&0f64.to_le_bytes()); // Z bounds (unused)
    shp_data.extend_from_slice(&0f64.to_le_bytes());
    shp_data.extend_from_slice(&0f64.to_le_bytes()); // M bounds (unused)
    shp_data.extend_from_slice(&0f64.to_le_bytes());

    // Records
    let mut shx_data = Vec::new();
    for (i, f) in features.iter().enumerate() {
        let record_offset = shp_data.len();
        // Record header
        shp_data.extend_from_slice(&((i + 1) as i32).to_be_bytes()); // Record number
        let record_content_start = shp_data.len();
        shp_data.extend_from_slice(&0i32.to_be_bytes()); // Content length (fill later)

        let content_start = shp_data.len();
        // Shape type
        shp_data.extend_from_slice(&(shape_type as u32).to_le_bytes());

        match &f.geometry {
            Shape::Point { x, y } => {
                shp_data.extend_from_slice(&x.to_le_bytes());
                shp_data.extend_from_slice(&y.to_le_bytes());
            }
            Shape::Polyline { parts } | Shape::Polygon { rings: parts } => {
                shp_data.extend_from_slice(&min_x.to_le_bytes()); // Bounds
                shp_data.extend_from_slice(&min_y.to_le_bytes());
                shp_data.extend_from_slice(&max_x.to_le_bytes());
                shp_data.extend_from_slice(&max_y.to_le_bytes());
                shp_data.extend_from_slice(&(parts.len() as i32).to_le_bytes()); // Num parts
                let total_points: usize = parts.iter().map(|p| p.len()).sum();
                shp_data.extend_from_slice(&(total_points as i32).to_le_bytes()); // Num points
                // Part starts
                let mut start = 0i32;
                for part in parts {
                    shp_data.extend_from_slice(&start.to_le_bytes());
                    start += part.len() as i32;
                }
                // Points
                for part in parts {
                    for &[x, y] in part {
                        shp_data.extend_from_slice(&x.to_le_bytes());
                        shp_data.extend_from_slice(&y.to_le_bytes());
                    }
                }
            }
            Shape::MultiPoint { points } => {
                shp_data.extend_from_slice(&min_x.to_le_bytes());
                shp_data.extend_from_slice(&min_y.to_le_bytes());
                shp_data.extend_from_slice(&max_x.to_le_bytes());
                shp_data.extend_from_slice(&max_y.to_le_bytes());
                shp_data.extend_from_slice(&(points.len() as i32).to_le_bytes());
                for &[x, y] in points {
                    shp_data.extend_from_slice(&x.to_le_bytes());
                    shp_data.extend_from_slice(&y.to_le_bytes());
                }
            }
        }

        let content_len_bytes = shp_data.len() - content_start;
        let content_len_words = (content_len_bytes / 2) as i32;
        // Patch content length
        let content_len_offset = record_content_start;
        shp_data[content_len_offset..content_len_offset + 4].copy_from_slice(&content_len_words.to_be_bytes());

        // .shx index entry: offset (in 16-bit words) + length (in 16-bit words)
        let offset_words = (record_offset / 2) as i32;
        shx_data.extend_from_slice(&offset_words.to_be_bytes());
        shx_data.extend_from_slice(&content_len_words.to_be_bytes());
    }

    // Patch file length (in 16-bit words)
    let file_len_words = (shp_data.len() / 2) as i32;
    shp_data[24..28].copy_from_slice(&file_len_words.to_be_bytes());

    // Write .shp
    std::fs::write(shp_path, &shp_data)?;

    // Write .shx (index file)
    let shx_path = shp_path.with_extension("shx");
    let mut shx = Vec::new();
    shx.extend_from_slice(&9994i32.to_be_bytes()); // File code
    shx.extend_from_slice(&[0u8; 20]); // Unused
    shx.extend_from_slice(&((50 + shx_data.len() / 8) as i32).to_be_bytes()); // File length
    shx.extend_from_slice(&1000u32.to_le_bytes()); // Version
    shx.extend_from_slice(&(shape_type as u32).to_le_bytes());
    shx.extend_from_slice(&min_x.to_le_bytes());
    shx.extend_from_slice(&min_y.to_le_bytes());
    shx.extend_from_slice(&max_x.to_le_bytes());
    shx.extend_from_slice(&max_y.to_le_bytes());
    shx.extend_from_slice(&0f64.to_le_bytes()); // Z bounds
    shx.extend_from_slice(&0f64.to_le_bytes());
    shx.extend_from_slice(&0f64.to_le_bytes()); // M bounds
    shx.extend_from_slice(&0f64.to_le_bytes());
    shx.extend_from_slice(&shx_data);
    std::fs::write(&shx_path, &shx)?;

    // Write .dbf (attributes)
    let dbf_path = shp_path.with_extension("dbf");
    write_dbf(&dbf_path, features)?;

    Ok(())
}

fn write_dbf(dbf_path: &Path, features: &[ShapefileFeature]) -> Result<(), ShapefileError> {
    // Collect all attribute keys
    let mut all_keys: Vec<String> = Vec::new();
    for f in features {
        for k in f.attributes.keys() {
            if !all_keys.contains(k) {
                all_keys.push(k.clone());
            }
        }
    }

    // Truncate field names to 10 chars (dBase limit)
    let fields: Vec<(String, usize)> = all_keys
        .iter()
        .map(|k| {
            let name = if k.len() > 10 { k[..10].to_string() } else { k.clone() };
            (name, 80usize) // All Character type, 80 chars wide
        })
        .collect();

    let record_size: usize = 1 + fields.iter().map(|(_, l)| l).sum::<usize>();
    let num_records = features.len();
    let header_size = 32 + fields.len() * 32 + 1; // header + field descriptors + terminator

    let mut dbf = Vec::new();
    // Header
    dbf.push(0x03); // dBase III version
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days_since_1970 = secs / 86400;
    let year = 1970 + (days_since_1970 / 365);
    let day_of_year = days_since_1970 % 365;
    dbf.push((year - 1900) as u8);
    dbf.push((day_of_year / 30 + 1) as u8); // Month (approx)
    dbf.push((day_of_year % 30 + 1) as u8); // Day (approx)
    dbf.extend_from_slice(&(num_records as u32).to_le_bytes());
    dbf.extend_from_slice(&(header_size as u16).to_le_bytes());
    dbf.extend_from_slice(&(record_size as u16).to_le_bytes());
    dbf.extend_from_slice(&[0u8; 20]); // Reserved

    // Field descriptors
    for (name, len) in &fields {
        let name_bytes = name.as_bytes();
        let mut name_field = [0u8; 11];
        name_field[..name_bytes.len()].copy_from_slice(name_bytes);
        dbf.extend_from_slice(&name_field);
        dbf.push(b'C'); // Character type
        dbf.extend_from_slice(&[0u8; 4]); // Reserved
        dbf.push(*len as u8); // Field length
        dbf.push(0); // Decimal count
        dbf.extend_from_slice(&[0u8; 14]); // Reserved
    }
    dbf.push(0x0D); // Header terminator

    // Records
    for f in features {
        dbf.push(0x20); // Active record
        for (i, (_, len)) in fields.iter().enumerate() {
            let key = &all_keys[i];
            let val = f.attributes.get(key).map(|s| s.as_str()).unwrap_or("");
            let val_bytes = val.as_bytes();
            let mut field_data = vec![b' '; *len];
            let copy_len = val_bytes.len().min(*len);
            field_data[..copy_len].copy_from_slice(&val_bytes[..copy_len]);
            dbf.extend_from_slice(&field_data);
        }
    }

    dbf.push(0x1A); // EOF marker
    std::fs::write(dbf_path, &dbf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_test_shp(path: &Path) {
        // Minimal Point shapefile with 2 points
        let mut f = File::create(path).unwrap();
        // Header (100 bytes)
        f.write_all(&9994i32.to_be_bytes()).unwrap(); // File code
        f.write_all(&[0u8; 20]).unwrap(); // Unused
        f.write_all(&50i32.to_be_bytes()).unwrap(); // File length (50 words = 100 bytes... but we write more)
        f.write_all(&1000u32.to_le_bytes()).unwrap(); // Version
        f.write_all(&1u32.to_le_bytes()).unwrap(); // Shape type = Point
        f.write_all(&0f64.to_le_bytes()).unwrap(); // min_x
        f.write_all(&0f64.to_le_bytes()).unwrap(); // min_y
        f.write_all(&10f64.to_le_bytes()).unwrap(); // max_x
        f.write_all(&10f64.to_le_bytes()).unwrap(); // max_y
        f.write_all(&0f64.to_le_bytes()).unwrap(); // z_min
        f.write_all(&0f64.to_le_bytes()).unwrap(); // z_max
        f.write_all(&0f64.to_le_bytes()).unwrap(); // m_min
        f.write_all(&0f64.to_le_bytes()).unwrap(); // m_max

        // Record 1: point at (1, 2)
        f.write_all(&1i32.to_be_bytes()).unwrap(); // Record number
        f.write_all(&10i32.to_be_bytes()).unwrap(); // Content length (10 words = 20 bytes)
        f.write_all(&1u32.to_le_bytes()).unwrap(); // Shape type = Point
        f.write_all(&1f64.to_le_bytes()).unwrap(); // x
        f.write_all(&2f64.to_le_bytes()).unwrap(); // y

        // Record 2: point at (10, 10)
        f.write_all(&2i32.to_be_bytes()).unwrap(); // Record number
        f.write_all(&10i32.to_be_bytes()).unwrap(); // Content length
        f.write_all(&1u32.to_le_bytes()).unwrap(); // Shape type = Point
        f.write_all(&10f64.to_le_bytes()).unwrap(); // x
        f.write_all(&10f64.to_le_bytes()).unwrap(); // y

        // Patch file length (50 + 2 records × 14 words = 78 words... actually let me compute)
        // Header = 50 words, each record = 7 words header + 10 words content = 14 words... 
        // Actually: record header = 4 words (8 bytes), content = 10 words (20 bytes), total = 14 words per record
        // 2 records = 28 words, + 50 header = 78 words
        f.seek(SeekFrom::Start(24)).unwrap();
        f.write_all(&78i32.to_be_bytes()).unwrap();
    }

    #[test]
    fn test_read_point_shapefile() {
        let dir = std::env::temp_dir();
        let shp_path = dir.join("test_point.shp");
        write_test_shp(&shp_path);

        let shp = read_shapefile(&shp_path).unwrap();
        assert_eq!(shp.shape_type, ShapeType::Point);
        assert_eq!(shp.features.len(), 2);
        match &shp.features[0].geometry {
            Shape::Point { x, y } => {
                assert!((*x - 1.0).abs() < 1e-6);
                assert!((*y - 2.0).abs() < 1e-6);
            }
            _ => panic!("expected Point"),
        }

        let _ = std::fs::remove_file(&shp_path);
    }

    #[test]
    fn test_write_then_read_point() {
        let dir = std::env::temp_dir();
        let shp_path = dir.join("test_write_point.shp");
        let features = vec![
            ShapefileFeature {
                geometry: Shape::Point { x: 100.0, y: 200.0 },
                attributes: [("NAME".to_string(), "Point A".to_string())].into_iter().collect(),
            },
            ShapefileFeature {
                geometry: Shape::Point { x: 300.0, y: 400.0 },
                attributes: [("NAME".to_string(), "Point B".to_string())].into_iter().collect(),
            },
        ];
        write_shapefile(&shp_path, &features).unwrap();
        let shp = read_shapefile(&shp_path).unwrap();
        assert_eq!(shp.features.len(), 2);
        assert_eq!(shp.shape_type, ShapeType::Point);

        let _ = std::fs::remove_file(&shp_path);
        let _ = std::fs::remove_file(shp_path.with_extension("shx"));
        let _ = std::fs::remove_file(shp_path.with_extension("dbf"));
    }

    #[test]
    fn test_read_nonexistent() {
        let result = read_shapefile(Path::new("/nonexistent/file.shp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_shape_type_from_u32() {
        assert_eq!(ShapeType::from_u32(1), Some(ShapeType::Point));
        assert_eq!(ShapeType::from_u32(3), Some(ShapeType::Polyline));
        assert_eq!(ShapeType::from_u32(5), Some(ShapeType::Polygon));
        assert_eq!(ShapeType::from_u32(99), None);
    }
}
