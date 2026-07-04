// PDF report generation with chain-of-custody appendix.
//
// Uses `printpdf` to lay out a single-page A4 PDF report containing the
// volume-calculation summary plus a visible "Chain of Custody" section.
// The full chain-of-custody record (a JSON blob) is also embedded in the
// PDF's `/Keywords` metadata field, so downstream auditors can extract
// the provenance record without re-parsing the visible text.
//
// The `report_hash` field of `ChainOfCustody` is the SHA-256 of the CoC
// JSON with the hash field itself left empty (a canonical pre-image) —
// this lets a verifier re-derive the hash deterministically from the
// PDF's Keywords metadata.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use printpdf::{BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfLayerReference};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Software version string written into the report's metadata and visible
/// footer.
pub const SOFTWARE_VERSION: &str = "MetaRDU Industrial 1.0.0";

/// 22-field chain-of-custody record. `report_hash` is the SHA-256 of the
/// canonical CoC JSON with `report_hash` left as an empty string.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainOfCustody {
    /// 1. Unique custody identifier (UUID or sequential ID).
    pub custody_id: String,
    /// 2. Unix timestamp when the custody record was created.
    pub created_at: u64,
    /// 3. Name of the custodian / operator who generated the report.
    pub custodian: String,
    /// 4. Source file path (the input point cloud or LAS file).
    pub source_file: String,
    /// 5. SHA-256 of the source file contents.
    pub source_hash: String,
    /// 6. Total number of points read from the source file.
    pub point_count: u64,
    /// 7. Number of points classified as ground by CSF.
    pub ground_count: u64,
    /// 8. CSF cloth resolution (metres).
    pub csf_cloth_resolution: f64,
    /// 9. CSF classification threshold (metres).
    pub csf_classification_threshold: f64,
    /// 10. CSF iterations actually run.
    pub csf_iterations: u32,
    /// 11. DEM cell size (metres).
    pub dem_cell_size: f64,
    /// 12. DEM bounds — minimum X.
    pub dem_min_x: f64,
    /// 13. DEM bounds — minimum Y.
    pub dem_min_y: f64,
    /// 14. DEM bounds — maximum X.
    pub dem_max_x: f64,
    /// 15. DEM bounds — maximum Y.
    pub dem_max_y: f64,
    /// 16. Fill volume (cubic metres).
    pub fill_volume: f64,
    /// 17. Cut volume (cubic metres).
    pub cut_volume: f64,
    /// 18. Net volume (cubic metres).
    pub net_volume: f64,
    /// 19. License ID under which the report was generated.
    pub license_id: String,
    /// 20. Machine fingerprint of the generating host.
    pub machine_id: String,
    /// 21. Site ID for the operation.
    pub site_id: String,
    /// 22. SHA-256 of the canonical CoC JSON (with this field blank).
    pub report_hash: String,
}

impl ChainOfCustody {
    /// Compute the SHA-256 of the CoC JSON with `report_hash` left empty,
    /// and store it back into `self.report_hash`. Returns the hash hex.
    pub fn seal(&mut self) -> String {
        // Stash the existing hash, set to empty, serialise, hash, restore.
        let original = std::mem::take(&mut self.report_hash);
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let digest = hasher.finalize();
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        self.report_hash = hex.clone();
        // `original` is intentionally discarded — once sealed, the hash is
        // the canonical one derived from the empty-hash pre-image.
        let _ = original;
        hex
    }
}

/// Data required to generate a PDF report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    /// Report title (shown at the top of the page).
    pub title: String,
    /// Optional subtitle (project name, etc.).
    pub subtitle: String,
    /// Author / operator name.
    pub author: String,
    /// Project identifier.
    pub project: String,
    /// Site name.
    pub site: String,
    /// Unix timestamp the report was generated.
    pub created_at: u64,
    /// Whether the report was generated under a signed license.
    pub signed: bool,
    /// Free-form summary text (multi-line; newline-separated).
    pub summary: String,
    /// The chain-of-custody record.
    pub chain_of_custody: ChainOfCustody,
    /// Software version string.
    pub software_version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF generation error: {0}")]
    Pdf(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Generate a single-page A4 PDF report at `path` containing the report
/// title, summary, a visible Chain-of-Custody section, and the full CoC
/// JSON embedded in the `/Keywords` metadata field.
pub fn generate_pdf_report(path: &Path, data: &ReportData) -> Result<(), ReportError> {
    // 1. Seal the chain-of-custody record (compute report_hash).
    let mut coc = data.chain_of_custody.clone();
    let _hash = coc.seal();

    // 2. Serialise the CoC JSON for the Keywords metadata field.
    let coc_json = serde_json::to_string(&coc)?;

    // 3. Build the PDF document.
    let (doc, page1, layer1) = PdfDocument::new(
        data.title.clone(),
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    let bold_font = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;

    let layer = doc.get_page(page1).get_layer(layer1);

    // A4 portrait: 210mm x 297mm. Origin is bottom-left, y grows upward.
    // Layout: header at y=280, body sections descending to footer at y=20.
    // All coordinates are f32 because printpdf's `Mm(pub f32)` tuple struct
    // expects single-precision millimetre values.
    let mut y = 280.0f32;
    let left_margin = 20.0f32;
    let right_limit = 190.0f32;
    let line_height = 6.0f32;
    let section_gap = 4.0f32;

    // --- Header ---------------------------------------------------------
    layer.use_text(&data.title, 22.0, Mm(left_margin), Mm(y), &bold_font);
    y -= 10.0;
    if !data.subtitle.is_empty() {
        layer.use_text(&data.subtitle, 12.0, Mm(left_margin), Mm(y), &font);
        y -= line_height + 2.0;
    }
    layer.use_text(
        format!(
            "Project: {}    Site: {}    Author: {}",
            data.project, data.site, data.author
        ),
        10.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );
    y -= line_height;
    layer.use_text(
        format!(
            "Generated: {}    Software: {}    Signed: {}",
            format_unix_ts(data.created_at),
            data.software_version,
            if data.signed { "yes" } else { "no" }
        ),
        10.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );
    y -= line_height + section_gap;
    draw_horizontal_rule(&layer, left_margin, right_limit, y, &font);
    y -= line_height + section_gap;

    // --- Summary section ------------------------------------------------
    layer.use_text("Summary", 14.0, Mm(left_margin), Mm(y), &bold_font);
    y -= line_height + 1.0;
    for line in data.summary.lines() {
        if y < 60.0 {
            break;
        }
        layer.use_text(line, 10.0, Mm(left_margin), Mm(y), &font);
        y -= line_height;
    }
    y -= section_gap;

    // --- Volume summary -------------------------------------------------
    layer.use_text("Volume Summary", 14.0, Mm(left_margin), Mm(y), &bold_font);
    y -= line_height + 1.0;
    layer.use_text(
        format!("Fill volume:  +{:.2} m³", coc.fill_volume),
        10.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );
    y -= line_height;
    layer.use_text(
        format!("Cut volume:   -{:.2} m³", coc.cut_volume),
        10.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );
    y -= line_height;
    layer.use_text(
        format!("Net volume:   {:+.2} m³", coc.net_volume),
        10.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );
    y -= line_height + section_gap;

    // --- Chain of Custody section --------------------------------------
    layer.use_text(
        "Chain of Custody",
        14.0,
        Mm(left_margin),
        Mm(y),
        &bold_font,
    );
    y -= line_height + 1.0;
    let coc_lines = vec![
        format!("Custody ID:    {}", coc.custody_id),
        format!("Created:       {}", format_unix_ts(coc.created_at)),
        format!("Custodian:     {}", coc.custodian),
        format!("Source file:   {}", coc.source_file),
        format!("Source SHA-256: {}", coc.source_hash),
        format!(
            "Points:        {} total, {} ground",
            coc.point_count, coc.ground_count
        ),
        format!(
            "CSF:           cloth={} m, threshold={} m, {} iters",
            coc.csf_cloth_resolution, coc.csf_classification_threshold, coc.csf_iterations
        ),
        format!(
            "DEM:           cell={} m, bounds=({:.2}, {:.2}, {:.2}, {:.2})",
            coc.dem_cell_size,
            coc.dem_min_x,
            coc.dem_min_y,
            coc.dem_max_x,
            coc.dem_max_y
        ),
        format!("License ID:    {}", coc.license_id),
        format!("Machine ID:    {}", coc.machine_id),
        format!("Site ID:       {}", coc.site_id),
        format!("Report hash:   {}", coc.report_hash),
    ];
    for line in coc_lines {
        if y < 40.0 {
            break;
        }
        layer.use_text(&line, 9.0, Mm(left_margin), Mm(y), &font);
        y -= line_height - 0.5;
    }
    y -= section_gap;
    layer.use_text(
        "The full chain-of-custody record is embedded in this PDF's",
        8.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );
    y -= line_height - 1.0;
    layer.use_text(
        "Keywords metadata (PDF → Properties → Keywords).",
        8.0,
        Mm(left_margin),
        Mm(y),
        &font,
    );

    // --- Footer ---------------------------------------------------------
    layer.use_text(
        format!("{} — {}", data.software_version, format_unix_ts(data.created_at)),
        8.0,
        Mm(left_margin),
        Mm(15.0),
        &font,
    );

    // 4. Embed the CoC JSON in the PDF's Keywords metadata.
    let doc_with_keywords = doc.with_keywords(vec![coc_json.clone()]);

    // 5. Save the PDF.
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    doc_with_keywords
        .save(&mut writer)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    writer.flush()?;
    Ok(())
}

/// Draw a horizontal rule at y by rendering a row of underscore
/// characters in the supplied font. This is the simplest portable way to
/// draw a divider line in printpdf 0.7 without bringing in the full
/// vector-graphics API.
fn draw_horizontal_rule(
    layer: &PdfLayerReference,
    left: f32,
    right: f32,
    y: f32,
    font: &IndirectFontRef,
) {
    let span = (right - left) as usize;
    let dashes: String = std::iter::repeat('_').take(span).collect();
    layer.use_text(dashes, 8.0, Mm(left), Mm(y), font);
}

/// Helper to format a Unix timestamp as a human-readable UTC string.
fn format_unix_ts(ts: u64) -> String {
    // Lightweight UTC formatter — no chrono dependency required.
    let days_since_epoch = (ts / 86_400) as i64;
    let secs_of_day = (ts % 86_400) as u64;
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    let (year, month, day) = days_to_ymd(days_since_epoch);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, month, day, hour, minute, second
    )
}

/// Convert a day count since 1970-01-01 into (year, month, day) using the
/// proleptic Gregorian calendar. Algorithm: Howard Hinnant, "date.h".
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146097]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

// (No additional helper traits — `draw_horizontal_rule` takes an
// `&IndirectFontRef` directly.)

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_coc() -> ChainOfCustody {
        ChainOfCustody {
            custody_id: "CUSTODY-001".to_string(),
            created_at: 1_700_000_000,
            custodian: "J. Surveyor".to_string(),
            source_file: "/data/survey_001.las".to_string(),
            source_hash: "abcdef0123456789".to_string(),
            point_count: 12_345,
            ground_count: 11_000,
            csf_cloth_resolution: 0.5,
            csf_classification_threshold: 0.5,
            csf_iterations: 500,
            dem_cell_size: 1.0,
            dem_min_x: 500_000.0,
            dem_min_y: 4_000_000.0,
            dem_max_x: 500_100.0,
            dem_max_y: 4_000_100.0,
            fill_volume: 1234.5,
            cut_volume: 567.8,
            net_volume: 666.7,
            license_id: "LIC-001".to_string(),
            machine_id: "MACHINE-ABC".to_string(),
            site_id: "SITE-001".to_string(),
            report_hash: String::new(),
        }
    }

    fn sample_report(signed: bool) -> ReportData {
        ReportData {
            title: "MetaRDU Volume Report".to_string(),
            subtitle: "Survey 2024-001".to_string(),
            author: "J. Surveyor".to_string(),
            project: "Stockpile Audit".to_string(),
            site: "North Pit".to_string(),
            created_at: 1_700_000_000,
            signed,
            summary: "This report summarises the volume\nchange between two surveys.".to_string(),
            chain_of_custody: sample_coc(),
            software_version: SOFTWARE_VERSION.to_string(),
        }
    }

    #[test]
    fn test_generate_pdf_writes_nonempty_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let data = sample_report(true);
        generate_pdf_report(tmp.path(), &data).unwrap();
        let bytes = std::fs::read(tmp.path()).unwrap();
        assert!(bytes.len() > 1000, "PDF should be non-trivial");
        assert!(&bytes[0..5] == b"%PDF-");
        // The CoC JSON should be embedded in the keywords metadata.
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("report_hash"));
    }

    #[test]
    fn test_chain_of_custody_has_22_fields() {
        let coc = sample_coc();
        let json = serde_json::to_value(&coc).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(
            obj.len(),
            22,
            "ChainOfCustody must have exactly 22 fields, got {}",
            obj.len()
        );
        assert!(obj.contains_key("report_hash"));
    }

    #[test]
    fn test_seal_sets_nonempty_report_hash() {
        let mut coc = sample_coc();
        assert!(coc.report_hash.is_empty());
        let hash = coc.seal();
        assert!(!hash.is_empty());
        assert_eq!(coc.report_hash.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_unsigned_report_also_compiles() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let data = sample_report(false);
        generate_pdf_report(tmp.path(), &data).unwrap();
        let bytes = std::fs::read(tmp.path()).unwrap();
        assert!(&bytes[0..5] == b"%PDF-");
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
    }
}
