// Branded PDF Report Engine — generates professional survey reports.
// Per ROADMAP.md Revenue Feature #0 — foundation for all revenue features.
//
// Writes structured HTML with print-ready CSS. Open in browser → Ctrl+P → PDF.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSpec {
    pub report_type: ReportType,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub client: String,
    /// Surveyor name (from the user profile, Sprint 20).
    /// Shown in the report title block for chain-of-custody.
    #[serde(default)]
    pub surveyor_name: String,
    /// Surveyor's company / organization (from the user profile).
    #[serde(default)]
    pub surveyor_company: String,
    /// Surveyor's registration number (from the user profile, if any).
    #[serde(default)]
    pub surveyor_registration: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub tables: Vec<ReportTable>,
    #[serde(default)]
    pub summary: Vec<ReportStat>,
    #[serde(default)]
    pub map_screenshot: Option<String>,
    #[serde(default)]
    pub provenance_hash: Option<String>,
    pub output_path: String,
    /// Datum + epoch note shown in the report footer, e.g.
    /// "Datum: GDA2020 / Epoch 2020.0" or "Datum: WGS 84".
    /// For survey plans this is a legal compliance field — many
    /// jurisdictions (AU, US, EU) require the datum to be stated
    /// explicitly on every plan. Frontend is responsible for
    /// formatting via `formatDatumNote()` in src/lib/crs-quickpicks.ts.
    #[serde(default)]
    pub datum_note: Option<String>,
    /// CRIRSCO-aligned reporting code the plan references, if any.
    /// One of: JORC (Australia), SAMREC (South Africa), CIM (Canada),
    /// SME (US), PERC (Europe), or None. Stored as a free string so
    /// future codes don't require a Rust change. The frontend should
    /// constrain to the known set via a dropdown.
    #[serde(default)]
    pub reporting_code: Option<String>,
    /// Jurisdiction tag for the report footer, e.g. "Australia — NSW"
    /// or "South Africa — offshore". Used by compliance reviewers to
    /// route the plan to the right regulator checklist.
    #[serde(default)]
    pub jurisdiction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    EomReconciliation,
    DredgeAudit,
    S44Compliance,
    StockpileAudit,
    BlastReport,
    HighwallReport,
    DeliverablePackage,
    CrossSection,
    Generic,
}

impl ReportType {
    fn accent(&self) -> &str {
        match self {
            ReportType::EomReconciliation => "#FFA500",
            ReportType::DredgeAudit => "#20B2AA",
            ReportType::S44Compliance => "#20B2AA",
            ReportType::StockpileAudit => "#FFC107",
            ReportType::BlastReport => "#FF6B35",
            ReportType::HighwallReport => "#DC2626",
            ReportType::DeliverablePackage => "#6366F1",
            ReportType::CrossSection => "#0EA5E9",
            ReportType::Generic => "#FFA500",
        }
    }
    fn footer(&self) -> &str {
        match self {
            ReportType::EomReconciliation => {
                "MetaRDU Industrial — Production Reconciliation Report"
            }
            ReportType::DredgeAudit => "MetaRDU Industrial — Dredge Volume Audit Report",
            ReportType::S44Compliance => "MetaRDU Industrial — IHO S-44 Compliance Certificate",
            ReportType::StockpileAudit => "MetaRDU Industrial — Stockpile Inventory Audit",
            ReportType::BlastReport => "MetaRDU Industrial — Blast Performance Report",
            ReportType::HighwallReport => {
                "MetaRDU Industrial — Highwall Deformation Compliance Report"
            }
            ReportType::DeliverablePackage => {
                "MetaRDU Industrial — Survey Deliverable Package Manifest"
            }
            ReportType::CrossSection => "MetaRDU Industrial — Cross-Section Profile Report",
            ReportType::Generic => "MetaRDU Industrial — Survey Report",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTable {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportStat {
    pub label: String,
    pub value: String,
    pub unit: String,
    pub color: Option<String>,
}

pub fn generate_report(spec: &ReportSpec) -> Result<(), ReportError> {
    let html = render_html(spec);
    std::fs::write(&spec.output_path, html)?;
    Ok(())
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_html(spec: &ReportSpec) -> String {
    let a = spec.report_type.accent();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut h = String::with_capacity(8192);

    h.push_str(&format!(
        "<!DOCTYPE html><html><head><meta charset='utf-8'><title>{t}</title>\
        <meta name='generated' content='epoch {ts}' />\
        <style>\
        @page{{size:A4;margin:20mm 15mm}}\
        body{{font-family:Inter,system-ui,sans-serif;color:#0A192F;background:#fff;margin:0;font-size:11pt;line-height:1.5}}\
        .hdr{{border-bottom:3px solid {a};padding:12px 0;margin-bottom:20px;display:flex;justify-content:space-between;align-items:center}}\
        .hdr-l{{display:flex;align-items:center;gap:12px}}\
        .logo{{width:36px;height:36px;border-radius:50%;border:3px solid {a};display:flex;align-items:center;justify-content:center;font-weight:900;font-size:18px;color:{a}}}\
        .hdr-t{{font-size:16pt;font-weight:800}}\
        .hdr-s{{font-size:10pt;color:#64748B;margin-top:2px}}\
        .hdr-r{{text-align:right;font-size:9pt;color:#64748B}}\
        .meta{{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-bottom:20px}}\
        .mi{{background:#F8FAFC;border:1px solid #E2E8F0;border-radius:4px;padding:8px 12px}}\
        .ml{{font-size:8pt;color:#64748B;text-transform:uppercase;letter-spacing:0.5px}}\
        .mv{{font-size:11pt;font-weight:600}}\
        .comp{{display:flex;flex-wrap:wrap;gap:0;margin-bottom:20px;border:1px solid {a};border-radius:4px;overflow:hidden;background:{a}08}}\
        .cmp-i{{flex:1 1 33%;padding:8px 12px;border-right:1px solid {a}40;min-width:140px}}\
        .cmp-i:last-child{{border-right:0}}\
        .cmp-l{{display:block;font-size:7pt;color:{a};font-weight:700;text-transform:uppercase;letter-spacing:0.8px;margin-bottom:2px}}\
        .cmp-v{{display:block;font-size:11pt;font-weight:600;color:#0A192F}}\
        .stats{{display:grid;grid-template-columns:repeat(4,1fr);gap:8px;margin-bottom:20px}}\
        .sc{{border-radius:6px;padding:12px;text-align:center}}\
        .sl{{font-size:8pt;text-transform:uppercase;letter-spacing:0.5px}}\
        .sv{{font-size:18pt;font-weight:800;margin-top:4px}}\
        table{{width:100%;border-collapse:collapse;margin-bottom:20px}}\
        th{{background:#F1F5F9;text-align:left;padding:8px;font-size:9pt;color:#475569;border-bottom:2px solid #CBD5E1}}\
        td{{padding:6px 8px;border-bottom:1px solid #E2E8F0;font-size:10pt}}\
        tr:nth-child(even) td{{background:#FAFAFA}}\
        .st{{font-size:12pt;font-weight:700;border-bottom:1px solid #E2E8F0;padding-bottom:4px;margin:20px 0 10px}}\
        .map{{width:100%;border:1px solid #E2E8F0;border-radius:4px;margin-bottom:20px}}\
        .ftr{{border-top:2px solid #E2E8F0;padding-top:10px;margin-top:30px;font-size:8pt;color:#94A3B8;display:flex;justify-content:space-between}}\
        .prov{{font-family:JetBrains Mono,monospace;font-size:7pt}}\
        @media print{{body{{font-size:10pt}}.no-print{{display:none}}}}\
        </style></head><body>",
        t = esc(&spec.title), a = a, ts = ts
    ));

    // Header
    h.push_str(&format!(
        "<div class='hdr'><div class='hdr-l'><div class='logo'>M</div><div>\
        <div class='hdr-t'>{}</div><div class='hdr-s'>{}</div></div></div>\
        <div class='hdr-r'><div><strong>{}</strong></div>\
        <div>Surveyor: {surveyor}</div>\
        <div>{company}</div>\
        <div>Generated: epoch {ts}</div></div></div>",
        esc(&spec.title), esc(&spec.subtitle), esc(&spec.client),
        surveyor = if spec.surveyor_name.is_empty() { "—".into() } else { esc(&spec.surveyor_name) },
        company = if spec.surveyor_company.is_empty() { String::new() } else { esc(&spec.surveyor_company) },
        ts = ts
    ));

    // Surveyor registration number (if provided)
    if let Some(ref reg) = spec.surveyor_registration {
        if !reg.is_empty() {
            h.push_str(&format!(
                "<div class='meta'><div><strong>Surveyor Registration:</strong> {}</div></div>",
                esc(reg)
            ));
        }
    }

    // Metadata
    if !spec.metadata.is_empty() {
        h.push_str("<div class='meta'>");
        for (k, v) in &spec.metadata {
            h.push_str(&format!(
                "<div class='mi'><div class='ml'>{}</div><div class='mv'>{}</div></div>",
                esc(k),
                esc(v)
            ));
        }
        h.push_str("</div>");
    }

    // ── Compliance strip ─────────────────────────────────────────────
    // A single colored bar showing datum + reporting code + jurisdiction.
    // This is the field a compliance reviewer scans first when validating
    // a plan — burying it in metadata would be a real audit friction point.
    // Only render if at least one of the three fields is set.
    let has_compliance =
        spec.datum_note.is_some() || spec.reporting_code.is_some() || spec.jurisdiction.is_some();
    if has_compliance {
        h.push_str("<div class='comp'>");
        if let Some(d) = &spec.datum_note {
            h.push_str(&format!(
                "<div class='cmp-i'><span class='cmp-l'>DATUM</span><span class='cmp-v'>{}</span></div>",
                esc(d)
            ));
        }
        if let Some(rc) = &spec.reporting_code {
            h.push_str(&format!(
                "<div class='cmp-i'><span class='cmp-l'>REPORTING CODE</span><span class='cmp-v'>{}</span></div>",
                esc(rc)
            ));
        }
        if let Some(j) = &spec.jurisdiction {
            h.push_str(&format!(
                "<div class='cmp-i'><span class='cmp-l'>JURISDICTION</span><span class='cmp-v'>{}</span></div>",
                esc(j)
            ));
        }
        h.push_str("</div>");
    }

    // Summary stats
    if !spec.summary.is_empty() {
        h.push_str("<div class='stats'>");
        for s in &spec.summary {
            let c = s.color.as_deref().unwrap_or("#0A192F");
            h.push_str(&format!(
                "<div class='sc' style='background:{}15;border:1px solid {}40'>\
                <div class='sl' style='color:{}'>{}</div>\
                <div class='sv' style='color:{}'>{} <span style='font-size:9pt;font-weight:400'>{}</span></div></div>",
                c, c, c, esc(&s.label), c, esc(&s.value), esc(&s.unit)
            ));
        }
        h.push_str("</div>");
    }

    // Tables
    for t in &spec.tables {
        h.push_str(&format!(
            "<div class='st'>{}</div><table><thead><tr>",
            esc(&t.title)
        ));
        for hdr in &t.headers {
            h.push_str(&format!("<th>{}</th>", esc(hdr)));
        }
        h.push_str("</tr></thead><tbody>");
        for row in &t.rows {
            h.push_str("<tr>");
            for cell in row {
                h.push_str(&format!("<td>{}</td>", esc(cell)));
            }
            h.push_str("</tr>");
        }
        h.push_str("</tbody></table>");
    }

    // Map screenshot
    if let Some(ss) = &spec.map_screenshot {
        h.push_str(&format!(
            "<div class='st'>Map Overview</div><img class='map' src='data:image/png;base64,{}' />",
            ss
        ));
    }

    // Footer
    h.push_str(&format!(
        "<div class='ftr'><div>{}</div>",
        spec.report_type.footer()
    ));
    if let Some(hash) = &spec.provenance_hash {
        h.push_str(&format!(
            "<div class='prov'>Provenance: {}</div>",
            esc(hash)
        ));
    }
    h.push_str("</div></body></html>");
    h
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_report() {
        let spec = ReportSpec {
            report_type: ReportType::Generic,
            title: "Test Report".into(),
            subtitle: "June 2026".into(),
            client: "Test Mine".into(),
            metadata: HashMap::from([("CRS".into(), "EPSG:28355".into())]),
            tables: vec![ReportTable {
                title: "Volumes".into(),
                headers: vec!["Bench".into(), "Fill".into()],
                rows: vec![vec!["100-105".into(), "1234".into()]],
            }],
            summary: vec![ReportStat {
                label: "Fill".into(),
                value: "1234".into(),
                unit: "m3".into(),
                color: Some("#10B981".into()),
            }],
            map_screenshot: None,
            provenance_hash: Some("abc123".into()),
            output_path: "/tmp/test_report.html".into(),
            datum_note: None,
            reporting_code: None,
            jurisdiction: None,
        };
        generate_report(&spec).unwrap();
        let content = std::fs::read_to_string("/tmp/test_report.html").unwrap();
        assert!(content.contains("Test Report"));
        assert!(content.contains("abc123"));
    }

    #[test]
    fn test_compliance_strip_renders_all_three_fields() {
        // Verify that a report with all 3 compliance fields set renders
        // them in the colored strip near the top of the report. This is
        // the field a compliance reviewer scans first when validating
        // a plan — it MUST be visible without scrolling.
        let spec = ReportSpec {
            report_type: ReportType::EomReconciliation,
            title: "June 2026 EOM Reconciliation".into(),
            subtitle: "Test Mine — Pit A".into(),
            client: "Test Mine Pty Ltd".into(),
            metadata: HashMap::new(),
            tables: vec![],
            summary: vec![],
            map_screenshot: None,
            provenance_hash: None,
            output_path: "/tmp/test_compliance.html".into(),
            datum_note: Some("Datum: GDA2020 / Epoch 2020.0".into()),
            reporting_code: Some("JORC 2012 Edition".into()),
            jurisdiction: Some("Australia — NSW".into()),
        };
        generate_report(&spec).unwrap();
        let content = std::fs::read_to_string("/tmp/test_compliance.html").unwrap();
        // The compliance strip wrapper div must be present.
        assert!(content.contains("class='comp'"), "missing compliance strip");
        // All three labels + values must render.
        assert!(content.contains("DATUM"), "missing DATUM label");
        assert!(
            content.contains("Datum: GDA2020 / Epoch 2020.0"),
            "missing datum note value"
        );
        assert!(
            content.contains("REPORTING CODE"),
            "missing REPORTING CODE label"
        );
        assert!(
            content.contains("JORC 2012 Edition"),
            "missing reporting code value"
        );
        assert!(
            content.contains("JURISDICTION"),
            "missing JURISDICTION label"
        );
        assert!(
            content.contains("Australia — NSW"),
            "missing jurisdiction value"
        );
    }

    #[test]
    fn test_compliance_strip_skipped_when_all_fields_none() {
        // If no compliance fields are set, the strip must not render at
        // all — otherwise we'd show an empty colored box.
        let spec = ReportSpec {
            report_type: ReportType::Generic,
            title: "Plain Report".into(),
            subtitle: "".into(),
            client: "".into(),
            metadata: HashMap::new(),
            tables: vec![],
            summary: vec![],
            map_screenshot: None,
            provenance_hash: None,
            output_path: "/tmp/test_no_compliance.html".into(),
            datum_note: None,
            reporting_code: None,
            jurisdiction: None,
        };
        generate_report(&spec).unwrap();
        let content = std::fs::read_to_string("/tmp/test_no_compliance.html").unwrap();
        assert!(
            !content.contains("class='comp'"),
            "compliance strip should not render"
        );
        assert!(!content.contains("DATUM"), "DATUM label should not render");
    }

    #[test]
    fn test_compliance_strip_renders_partial_fields() {
        // If only datum is set, only the DATUM cell renders — no empty
        // REPORTING CODE or JURISDICTION cells.
        let spec = ReportSpec {
            report_type: ReportType::Generic,
            title: "Partial Compliance Report".into(),
            subtitle: "".into(),
            client: "".into(),
            metadata: HashMap::new(),
            tables: vec![],
            summary: vec![],
            map_screenshot: None,
            provenance_hash: None,
            output_path: "/tmp/test_partial_compliance.html".into(),
            datum_note: Some("Datum: WGS 84".into()),
            reporting_code: None,
            jurisdiction: None,
        };
        generate_report(&spec).unwrap();
        let content = std::fs::read_to_string("/tmp/test_partial_compliance.html").unwrap();
        assert!(content.contains("class='comp'"));
        assert!(content.contains("Datum: WGS 84"));
        assert!(!content.contains("REPORTING CODE"));
        assert!(!content.contains("JURISDICTION"));
    }
}
