// OBN (Ocean Bottom Node) IPC commands — 4DNav & Node Dashboard Automation.
//
// Exposes pre-plot parsing, as-laid fix parsing, geodetic delta calculation,
// tolerance gating, UKOOA SPS & IOGP P1/11 exporting, and shift DPR generation.

use metardu_core::marine::obn::{
    compute_obn_audit_hash, evaluate_obn_qc, generate_iogp_p1_11, generate_shift_dpr_summary,
    generate_ukooa_sps_r_records, parse_4dnav_aslaid, parse_node_dashboard_export,
    parse_planned_preplot, NodeOperationalStatus, ObnAsLaidNode, ObnHeader, ObnPlannedNode,
    ObnQcNode, ObnQcSummary, ObnShiftDpr,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ObnQcRequest {
    #[serde(rename = "plannedContent")]
    pub planned_content: String,
    #[serde(rename = "aslaidContent")]
    pub aslaid_content: String,
    #[serde(rename = "toleranceSpecM", default = "default_spec_tolerance")]
    pub tolerance_spec_m: f64,
    #[serde(rename = "sourceFormat", default = "default_source_format")]
    pub source_format: String,
}

fn default_spec_tolerance() -> f64 {
    5.0
}

fn default_source_format() -> String {
    "auto".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ObnExportRequest {
    pub nodes: Vec<ObnQcNode>,
    pub header: ObnHeader,
}

#[derive(Debug, Deserialize)]
pub struct ObnDprRequest {
    pub shift: ObnShiftDpr,
    #[serde(rename = "qcSummary")]
    pub qc_summary: ObnQcSummary,
}

#[derive(Debug, Serialize)]
pub struct ObnSampleDataset {
    pub planned_csv: String,
    pub aslaid_csv: String,
    pub default_header: ObnHeader,
}

/// Evaluates spatial tolerance and residuals between planned pre-plot and as-laid fixes.
#[tauri::command]
pub fn obn_evaluate_qc_cmd(request: ObnQcRequest) -> Result<ObnQcSummary, String> {
    let planned: Vec<ObnPlannedNode> = if !request.planned_content.trim().is_empty() {
        parse_planned_preplot(&request.planned_content)
            .map_err(|e| format!("Error parsing planned pre-plot: {e}"))?
    } else {
        Vec::new()
    };

    let aslaid: Vec<ObnAsLaidNode> = match request.source_format.to_lowercase().as_str() {
        "4dnav" => parse_4dnav_aslaid(&request.aslaid_content)
            .map_err(|e| format!("Error parsing 4DNav as-laid log: {e}"))?,
        "node_dashboard" => parse_node_dashboard_export(&request.aslaid_content)
            .map_err(|e| format!("Error parsing Node Dashboard export: {e}"))?,
        _ => {
            // Auto-detect format
            if request.aslaid_content.contains("NodeID") || request.aslaid_content.contains("SerialNo") {
                parse_4dnav_aslaid(&request.aslaid_content)
                    .map_err(|e| format!("Error parsing 4DNav as-laid log: {e}"))?
            } else {
                parse_node_dashboard_export(&request.aslaid_content)
                    .or_else(|_| parse_4dnav_aslaid(&request.aslaid_content))
                    .map_err(|e| format!("Error parsing as-laid fixes: {e}"))?
            }
        }
    };

    evaluate_obn_qc(&planned, &aslaid, request.tolerance_spec_m)
        .map_err(|e| format!("Error computing OBN QC metrics: {e}"))
}

/// Generates valid UKOOA SPS (Shell Processing Support) Rev 2.1 R-records.
#[tauri::command]
pub fn obn_export_sps_cmd(request: ObnExportRequest) -> Result<String, String> {
    generate_ukooa_sps_r_records(&request.nodes, &request.header)
        .map_err(|e| format!("Error generating UKOOA SPS records: {e}"))
}

/// Generates valid IOGP P1/11 Geophysical Position Data records.
#[tauri::command]
pub fn obn_export_p111_cmd(request: ObnExportRequest) -> Result<String, String> {
    generate_iogp_p1_11(&request.nodes, &request.header)
        .map_err(|e| format!("Error generating IOGP P1/11 file: {e}"))
}

/// Generates a comprehensive 12-Hour Shift Handover and Daily Progress Report (DPR).
#[tauri::command]
pub fn obn_generate_shift_dpr_cmd(request: ObnDprRequest) -> Result<String, String> {
    let mut shift = request.shift;
    if shift.audit_hash.is_empty() {
        shift.audit_hash = compute_obn_audit_hash(
            &request.qc_summary.nodes,
            &shift.shift_date,
            &shift.surveyor_name,
        );
    }

    generate_shift_dpr_summary(&shift, &request.qc_summary)
        .map_err(|e| format!("Error generating Shift DPR: {e}"))
}

/// Generates a realistic offshore 40-node deepwater OBN deployment dataset for demonstration and testing.
#[tauri::command]
pub fn obn_generate_sample_project() -> Result<ObnSampleDataset, String> {
    let mut planned_csv = String::from("Station_ID,Line_Name,Easting,Northing,Depth\n");
    let mut aslaid_csv = String::from(
        "NodeID,SerialNo,StationID,LineName,Easting,Northing,Depth,Timestamp,USBL_Sigma,WaterDepth,Status\n",
    );

    let base_e = 542000.0;
    let base_n = 6125000.0;
    let base_depth = 1285.0;

    let mut count = 0;
    for line_idx in 1..=4 {
        let line_name = format!("OBN_L{:02}", line_idx);
        let n_coord = base_n + (line_idx as f64 - 1.0) * 200.0;

        for stn_idx in 1..=10 {
            count += 1;
            let station_id = format!("STN_{:02}{:02}", line_idx, stn_idx);
            let e_coord = base_e + (stn_idx as f64 - 1.0) * 100.0;
            let depth = base_depth + ((stn_idx as f64) * 1.8);

            // Add to planned
            planned_csv.push_str(&format!(
                "{},{},{:.1},{:.1},{:.1}\n",
                station_id, line_name, e_coord, n_coord, depth
            ));

            // Generate realistic as-laid offset
            let (dx, dy, sigma, status_str) = match count {
                7 => (7.2, -3.1, 2.4, "Deployed"),  // warning ~7.8m
                14 => (5.4, 4.1, 1.9, "Deployed"),  // warning ~6.8m
                23 => (10.8, -4.5, 3.2, "Deployed"), // out of spec ~11.7m
                31 => (1.2, 0.8, 0.9, "ActiveRecording"),
                _ => {
                    // mostly in-spec (1.0 - 3.5m)
                    let sign = if count % 2 == 0 { 1.0 } else { -1.0 };
                    let jitter_e = sign * (1.2 + (count as f64 % 2.5));
                    let jitter_n = -sign * (0.8 + (count as f64 % 2.1));
                    (jitter_e, jitter_n, 1.1 + (count as f64 % 0.8), "Deployed")
                }
            };

            let aslaid_e = e_coord + dx;
            let aslaid_n = n_coord + dy;
            let aslaid_z = depth + (dx * 0.05);
            let time_str = format!("2026-10-01T{:02}:{:02}:00Z", 6 + (count / 6), (count * 9) % 60);
            let serial_no = format!("SN_{:04}", 7200 + count);

            aslaid_csv.push_str(&format!(
                "NODE_{:03},{},{},{},{:.2},{:.2},{:.2},{},{:.1},{:.1},{}\n",
                count, serial_no, station_id, line_name, aslaid_e, aslaid_n, aslaid_z, time_str, sigma, aslaid_z, status_str
            ));
        }
    }

    Ok(ObnSampleDataset {
        planned_csv,
        aslaid_csv,
        default_header: ObnHeader {
            survey_name: "Triton Deep OBN Seismic Scope".into(),
            client_name: "North Sea Petroleum Ltd".into(),
            contractor_name: "UTM Offshore Survey Team".into(),
            vessel_name: "MV Coastal Navigator".into(),
            crs_name: "ED50 / UTM Zone 31N".into(),
            epsg_code: 23031,
            date_str: "2026-10-01".into(),
        },
    })
}
