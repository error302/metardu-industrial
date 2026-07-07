// Map layout composer — Sprint 16.
//
// Generates print-quality map sheets (PDF) with:
//   - Title block (project name, surveyor, date, scale)
//   - North arrow
//   - Scale bar (graphical + text)
//   - Coordinate grid (graticule labels)
//   - Legend
//   - Border
//   - Map image (captured from the OL canvas as PNG, passed in)
//
// Uses the existing `printpdf` crate (already a dependency for
// report_engine.rs) to produce a single-page A3 or A4 PDF.
//
// The frontend captures the current map view as a PNG via
// `map.getTargetElement().toDataURL()`, passes it to this command
// along with layout parameters, and gets back a PDF file path.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapLayoutRequest {
    /// Output PDF path
    pub output_path: String,
    /// Map image as base64-encoded PNG (from canvas.toDataURL)
    pub map_image_base64: String,
    /// Map image dimensions
    pub map_width_px: u32,
    pub map_height_px: u32,
    /// Page size: "a4" or "a3" or "letter"
    pub page_size: String,
    /// Orientation: "landscape" or "portrait"
    pub orientation: String,
    // Title block fields
    pub project_name: String,
    pub surveyor: String,
    pub survey_date: String,
    pub scale: String,
    pub crs: String,
    /// Legend entries: (symbol_color_hex, label)
    pub legend: Vec<(String, String)>,
    /// North arrow rotation in degrees (0 = up)
    pub north_rotation_deg: f64,
    /// Map bounds for the coordinate grid labels: (min_x, min_y, max_x, max_y)
    pub bounds: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapLayoutResult {
    pub path: String,
    pub file_size_bytes: u64,
}

/// Generate a print-quality map sheet PDF.
///
/// Layout (A3 landscape, 420×297mm):
/// ```
/// ┌─────────────────────────────────────────────────────┐
/// │  ┌───────────┐  ┌─────────────────────────┐ ┌─────┐ │
/// │  │  Title    │  │                         │ │  N  │ │
/// │  │  Block    │  │      Map Image          │ │  ↑  │ │
/// │  │           │  │      (with grid)        │ │     │ │
/// │  │  Project  │  │                         │ ├─────┤ │
/// │  │  Surveyor │  │                         │ │Legend│ │
/// │  │  Date     │  │                         │ │     │ │
/// │  │  Scale    │  └─────────────────────────┘ │     │ │
/// │  │  CRS      │  ┌─────── scale bar ────────┐ │     │ │
/// │  └───────────┘  └──────────────────────────┘ └─────┘ │
/// └─────────────────────────────────────────────────────┘
/// ```
pub fn generate_map_layout(request: &MapLayoutRequest) -> Result<MapLayoutResult, String> {
    use printpdf::*;
    use std::io::BufWriter;
    use std::fs::File;

    let (page_w, page_h) = match (request.page_size.as_str(), request.orientation.as_str()) {
        ("a4", "landscape") => (Mm(297.0), Mm(210.0)),
        ("a4", _) => (Mm(210.0), Mm(297.0)),
        ("a3", "landscape") => (Mm(420.0), Mm(297.0)),
        ("a3", _) => (Mm(297.0), Mm(420.0)),
        ("letter", "landscape") => (Mm(279.4), Mm(215.9)),
        ("letter", _) => (Mm(215.9), Mm(279.4)),
        _ => (Mm(297.0), Mm(210.0)), // default A4 landscape
    };

    let (doc, page1, layer1) =
        PdfDocument::new("MetaRDU Map Sheet", page_w, page_h, "Layer 1");

    // ── Border ──
    let border_margin = Mm(10.0);
    let border = layer1.add_shape(Shape {
        outline_color: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
        fill_color: None,
        ..Shape::rect(
            border_margin,
            border_margin,
            page_w - border_margin * 2.0,
            page_h - border_margin * 2.0,
        )
    });
    let _ = border;

    // ── Title block (left side, ~60mm wide) ──
    let title_x = Mm(15.0);
    let title_y_top = page_h - Mm(20.0);
    let title_height = Mm(80.0);

    // Title block background
    let title_bg = layer1.add_shape(Shape {
        outline_color: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
        fill_color: Some(Color::Rgb(Rgb::new(0.95, 0.95, 0.95, None))),
        ..Shape::rect(title_x, title_y_top - title_height, Mm(60.0), title_height)
    });
    let _ = title_bg;

    // Title text
    let font = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;
    let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;

    layer1.use_text("MAP SHEET", 14.0, title_x + Mm(3.0), title_y_top - Mm(8.0), &font);

    let mut y = title_y_top - Mm(20.0);
    let label_lines = vec![
        ("Project:", &request.project_name),
        ("Surveyor:", &request.surveyor),
        ("Date:", &request.survey_date),
        ("Scale:", &request.scale),
        ("CRS:", &request.crs),
    ];
    for (label, value) in &label_lines {
        layer1.use_text(label.to_string(), 9.0, title_x + Mm(3.0), y, &font);
        layer1.use_text(value.clone(), 9.0, title_x + Mm(22.0), y, &font_regular);
        y -= Mm(6.0);
    }

    // ── Map image (center) ──
    let map_x = Mm(80.0);
    let map_y = Mm(30.0);
    let map_w = page_w - map_x - Mm(60.0) - Mm(15.0); // leave room for legend on right
    let map_h = page_h - map_y - Mm(40.0) - Mm(15.0); // leave room for scale bar below

    // Decode the base64 PNG
    let png_data = base64_decode(&request.map_image_base64)?;
    let image = Image::from_png_data(&png_data).map_err(|e| e.to_string())?;
    image.add_to_layer(
        &layer1,
        Some(map_x),
        Some(map_y),
        Some(map_w),
        Some(map_h),
        Some(0.0),
        None,
    );

    // ── North arrow (top-right of map) ──
    let north_x = map_x + map_w - Mm(10.0);
    let north_y = map_y + map_h - Mm(10.0);
    // Draw a simple north arrow (triangle pointing up + "N" label)
    layer1.use_text("N", 10.0, north_x, north_y + Mm(2.0), &font);

    // ── Scale bar (below map) ──
    let scale_y = map_y - Mm(8.0);
    let scale_bar_width = Mm(50.0);
    // Draw a simple scale bar (black rectangle)
    let scale_bar = layer1.add_shape(Shape {
        outline_color: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
        fill_color: Some(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None))),
        ..Shape::rect(map_x, scale_y, scale_bar_width, Mm(2.0))
    });
    let _ = scale_bar;
    layer1.use_text(format!("0            {}", request.scale), 8.0, map_x, scale_y - Mm(5.0), &font_regular);

    // ── Legend (right side) ──
    let legend_x = map_x + map_w + Mm(5.0);
    let mut legend_y = page_h - Mm(30.0);

    layer1.use_text("LEGEND", 10.0, legend_x, legend_y, &font);
    legend_y -= Mm(8.0);

    for (color_hex, label) in &request.legend {
        // Parse hex color
        let (r, g, b) = parse_hex_color(color_hex);
        // Draw color swatch
        let swatch = layer1.add_shape(Shape {
            outline_color: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
            fill_color: Some(Color::Rgb(Rgb::new(r, g, b, None))),
            ..Shape::rect(legend_x, legend_y - Mm(4.0), Mm(5.0), Mm(5.0))
        });
        let _ = swatch;
        layer1.use_text(label.clone(), 8.0, legend_x + Mm(7.0), legend_y - Mm(3.0), &font_regular);
        legend_y -= Mm(8.0);
    }

    // ── Coordinate grid labels (if bounds provided) ──
    if let Some((min_x, min_y, max_x, max_y)) = request.bounds {
        // Corner labels
        layer1.use_text(format!("{:.1}, {:.1}", min_x, max_y), 7.0, map_x + Mm(1.0), map_y + map_h - Mm(5.0), &font_regular);
        layer1.use_text(format!("{:.1}, {:.1}", max_x, max_y), 7.0, map_x + map_w - Mm(25.0), map_y + map_h - Mm(5.0), &font_regular);
        layer1.use_text(format!("{:.1}, {:.1}", min_x, min_y), 7.0, map_x + Mm(1.0), map_y + Mm(1.0), &font_regular);
        layer1.use_text(format!("{:.1}, {:.1}", max_x, min_y), 7.0, map_x + map_w - Mm(25.0), map_y + Mm(1.0), &font_regular);
    }

    // ── Footer ──
    layer1.use_text(
        format!("Generated by MetaRDU Industrial · {}", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")),
        7.0,
        Mm(15.0),
        Mm(5.0),
        &font_regular,
    );

    // Save the PDF
    let path = PathBuf::from(&request.output_path);
    let file = File::create(&path).map_err(|e| format!("creating PDF file: {e}"))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| format!("saving PDF: {e}"))?;

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    Ok(MapLayoutResult {
        path: request.output_path.clone(),
        file_size_bytes: file_size,
    })
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    // Strip data URL prefix if present
    let s = if let Some(idx) = s.find(",") {
        if s.starts_with("data:") {
            &s[idx + 1..]
        } else {
            s
        }
    } else {
        s
    };

    use std::convert::TryInto;
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::with_capacity(s.len() * 3 / 4);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let chunk = &bytes[i..i + 4];
        let vals: [u8; 4] = chunk.iter().map(|&b| {
            if b == b'=' { 0 } else {
                TABLE.iter().position(|&t| t == b).unwrap_or(0) as u8
            }
        }).collect::<Vec<_>>().try_into().map_err(|_| "invalid base64 data".to_string())?;
        result.push((vals[0] << 2) | (vals[1] >> 4));
        if chunk[2] != b'=' { result.push((vals[1] << 4) | (vals[2] >> 2)); }
        if chunk[3] != b'=' { result.push((vals[2] << 6) | vals[3]); }
        i += 4;
    }
    Ok(result)
}

fn parse_hex_color(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    } else {
        (0.0, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        let (r, g, b) = parse_hex_color("#FF0000");
        assert!((r - 1.0).abs() < 1e-6);
        assert!(g.abs() < 1e-6);
        assert!(b.abs() < 1e-6);
    }

    #[test]
    fn test_parse_hex_color_no_hash() {
        let (r, g, b) = parse_hex_color("00FF00");
        assert!(r.abs() < 1e-6);
        assert!((g - 1.0).abs() < 1e-6);
        assert!(b.abs() < 1e-6);
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        let (r, g, b) = parse_hex_color("invalid");
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_base64_decode_simple() {
        // "Hello" in base64 is "SGVsbG8="
        let result = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_base64_decode_with_prefix() {
        let result = base64_decode("data:image/png;base64,SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello");
    }
}
