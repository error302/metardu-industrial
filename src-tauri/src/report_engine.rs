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
            ReportType::EomReconciliation => "MetaRDU Industrial — Production Reconciliation Report",
            ReportType::DredgeAudit => "MetaRDU Industrial — Dredge Volume Audit Report",
            ReportType::S44Compliance => "MetaRDU Industrial — IHO S-44 Compliance Certificate",
            ReportType::StockpileAudit => "MetaRDU Industrial — Stockpile Inventory Audit",
            ReportType::BlastReport => "MetaRDU Industrial — Blast Performance Report",
            ReportType::HighwallReport => "MetaRDU Industrial — Highwall Deformation Compliance Report",
            ReportType::DeliverablePackage => "MetaRDU Industrial — Survey Deliverable Package Manifest",
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
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn render_html(spec: &ReportSpec) -> String {
    let a = spec.report_type.accent();
    let now = std::time::{SystemTime, UNIX_EPOCH};
    let ts = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    let mut h = String::with_capacity(8192);

    h.push_str(&format!(
        "<!DOCTYPE html><html><head><meta charset='utf-8'><title>{t}</title>\
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
        <div class='hdr-r'><div><strong>{}</strong></div><div>Generated: epoch {ts}</div></div></div>",
        esc(&spec.title), esc(&spec.subtitle), esc(&spec.client)
    ));

    // Metadata
    if !spec.metadata.is_empty() {
        h.push_str("<div class='meta'>");
        for (k, v) in &spec.metadata {
            h.push_str(&format!("<div class='mi'><div class='ml'>{}</div><div class='mv'>{}</div></div>", esc(k), esc(v)));
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
        h.push_str(&format!("<div class='st'>{}</div><table><thead><tr>", esc(&t.title)));
        for hdr in &t.headers { h.push_str(&format!("<th>{}</th>", esc(hdr))); }
        h.push_str("</tr></thead><tbody>");
        for row in &t.rows {
            h.push_str("<tr>");
            for cell in row { h.push_str(&format!("<td>{}</td>", esc(cell))); }
            h.push_str("</tr>");
        }
        h.push_str("</tbody></table>");
    }

    // Map screenshot
    if let Some(ss) = &spec.map_screenshot {
        h.push_str(&format!("<div class='st'>Map Overview</div><img class='map' src='data:image/png;base64,{}' />", ss));
    }

    // Footer
    h.push_str(&format!("<div class='ftr'><div>{}</div>", spec.report_type.footer()));
    if let Some(hash) = &spec.provenance_hash {
        h.push_str(&format!("<div class='prov'>Provenance: {}</div>", hash));
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
                label: "Fill".into(), value: "1234".into(), unit: "m3".into(),
                color: Some("#10B981".into()),
            }],
            map_screenshot: None,
            provenance_hash: Some("abc123".into()),
            output_path: "/tmp/test_report.html".into(),
        };
        generate_report(&spec).unwrap();
        let content = std::fs::read_to_string("/tmp/test_report.html").unwrap();
        assert!(content.contains("Test Report"));
        assert!(content.contains("abc123"));
    }
}
