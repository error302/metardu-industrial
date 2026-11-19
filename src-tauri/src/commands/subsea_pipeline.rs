// Subsea Pipeline Integrity IPC commands — EIVA NaviModel, 3D Pipe, DelphINS, & AutoCAD.
//
// Exposes route centerline parsing, 3D pipe profile ingestion, DNV-RP-F105 freespan
// assessment, AutoCAD DXF export, CSV deliverables generation, and 12h Shift DPR handover.

use metardu_core::marine::subsea_pipeline::{
    detect_freespans, evaluate_pipeline_qc, generate_event_listing_csv, generate_freespan_listing_csv,
    generate_longitudinal_profile_csv, generate_pipeline_dxf, generate_pipeline_shift_dpr,
    generate_sample_pipeline_project, parse_delphins_trajectory, parse_navimodel_profile,
    parse_pipeline_events, parse_route_centerline, DelphInsFix, Freespan, PipeSurveyPoint,
    PipelineEvent, PipelineQcSummary, PipelineReportsBundle, PipelineRoutePoint, PipelineShiftDpr,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PipelineQcRequest {
    #[serde(rename = "routeContent")]
    pub route_content: String,
    #[serde(rename = "profileContent")]
    pub profile_content: String,
    #[serde(rename = "eventsContent")]
    pub events_content: String,
    #[serde(rename = "delphinsContent")]
    pub delphins_content: String,
    #[serde(rename = "pipeOdM", default = "default_pipe_od")]
    pub pipe_od_m: f64,
}

fn default_pipe_od() -> f64 {
    0.4064 // 16-inch
}

#[derive(Debug, Deserialize)]
pub struct PipelineDxfRequest {
    pub points: Vec<PipeSurveyPoint>,
    pub freespans: Vec<Freespan>,
    pub events: Vec<PipelineEvent>,
    #[serde(rename = "pipeOdM", default = "default_pipe_od")]
    pub pipe_od_m: f64,
}

#[derive(Debug, Deserialize)]
pub struct PipelineReportsRequest {
    pub points: Vec<PipeSurveyPoint>,
    pub freespans: Vec<Freespan>,
    pub events: Vec<PipelineEvent>,
    pub dpr: PipelineShiftDpr,
    #[serde(rename = "pipeOdM", default = "default_pipe_od")]
    pub pipe_od_m: f64,
}

#[derive(Debug, Serialize)]
pub struct PipelineSampleDataset {
    pub route_csv: String,
    pub profile_csv: String,
    pub events_csv: String,
    pub delphins_csv: String,
    pub default_dpr: PipelineShiftDpr,
    pub summary: PipelineQcSummary,
}

/// Evaluates pipeline integrity, depth of burial, freespans, and DelphINS navigation precision.
#[tauri::command]
pub fn subsea_pipeline_evaluate_qc_cmd(request: PipelineQcRequest) -> Result<PipelineQcSummary, String> {
    let od = request.pipe_od_m;

    let points: Vec<PipeSurveyPoint> = if !request.profile_content.trim().is_empty() {
        parse_navimodel_profile(&request.profile_content, od)
    } else {
        Vec::new()
    };

    let events: Vec<PipelineEvent> = if !request.events_content.trim().is_empty() {
        parse_pipeline_events(&request.events_content)
    } else {
        Vec::new()
    };

    let delphins: Vec<DelphInsFix> = if !request.delphins_content.trim().is_empty() {
        parse_delphins_trajectory(&request.delphins_content)
    } else {
        Vec::new()
    };

    if points.is_empty() {
        return Err("No valid EIVA NaviModel / 3D Pipe survey points provided".to_string());
    }

    Ok(evaluate_pipeline_qc(points, events, &delphins, od))
}

/// Detects and classifies freespans along surveyed profile points using custom clearance thresholds.
#[tauri::command]
pub fn subsea_pipeline_detect_freespans_cmd(
    points: Vec<PipeSurveyPoint>,
    min_gap_m: Option<f64>,
) -> Result<Vec<Freespan>, String> {
    let threshold = min_gap_m.unwrap_or(0.05);
    Ok(detect_freespans(&points, threshold))
}

/// Generates standard AutoCAD DXF vector alignment sheet and longitudinal profile.
#[tauri::command]
pub fn subsea_pipeline_export_dxf_cmd(request: PipelineDxfRequest) -> Result<String, String> {
    generate_pipeline_dxf(&request.points, &request.freespans, &request.events, request.pipe_od_m)
}

/// Generates the complete bundle of CSV deliverables and the DXF file.
#[tauri::command]
pub fn subsea_pipeline_export_reports_cmd(
    mut request: PipelineReportsRequest,
) -> Result<PipelineReportsBundle, String> {
    let freespan_csv = generate_freespan_listing_csv(&request.freespans);
    let event_csv = generate_event_listing_csv(&request.events);
    let profile_csv = generate_longitudinal_profile_csv(&request.points);
    let dpr_markdown = generate_pipeline_shift_dpr(&mut request.dpr);
    let dxf_content = generate_pipeline_dxf(
        &request.points,
        &request.freespans,
        &request.events,
        request.pipe_od_m,
    )?;

    Ok(PipelineReportsBundle {
        freespan_csv,
        event_csv,
        profile_csv,
        dpr_markdown,
        dxf_content,
    })
}

/// Formats 12h Shift DPR Handover report with SHA-256 seal.
#[tauri::command]
pub fn subsea_pipeline_generate_shift_dpr_cmd(
    mut dpr: PipelineShiftDpr,
) -> Result<PipelineShiftDpr, String> {
    let _ = generate_pipeline_shift_dpr(&mut dpr);
    Ok(dpr)
}

/// Pre-loads a complete realistic 5.0 km 16-inch offshore pipeline inspection dataset in Mexico (Campeche Sound).
#[tauri::command]
pub fn subsea_pipeline_generate_sample_project() -> Result<PipelineSampleDataset, String> {
    let (route, survey_points, events, delphins) = generate_sample_pipeline_project();

    // Serialize route to CSV
    let mut route_csv = String::from("kp,easting,northing,target_dob_m,pipe_od_m\n");
    for r in &route {
        route_csv.push_str(&format!(
            "{:.3},{:.2},{:.2},{:.2},{:.4}\n",
            r.kp, r.easting, r.northing, r.target_dob_m, r.pipe_od_m
        ));
    }

    // Serialize profile to CSV
    let mut profile_csv = String::from("kp,easting,northing,top_depth_m,seabed_depth_m,dcc_m\n");
    for p in &survey_points {
        profile_csv.push_str(&format!(
            "{:.3},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
            p.kp, p.easting, p.northing, p.top_z, p.seabed_z, p.dcc
        ));
    }

    // Serialize events to CSV
    let events_csv = generate_event_listing_csv(&events);

    // Serialize DelphINS to CSV
    let mut delphins_csv = String::from(
        "timestamp,kp,easting,northing,depth,heading,pitch,roll,sd_easting,sd_northing,sd_depth,usbl_rejected\n",
    );
    for f in &delphins {
        delphins_csv.push_str(&format!(
            "{},{:.3},{:.2},{:.2},{:.2},{:.1},{:.1},{:.1},{:.3},{:.3},{:.3},{}\n",
            f.timestamp,
            f.kp,
            f.easting,
            f.northing,
            f.depth,
            f.heading,
            f.pitch,
            f.roll,
            f.sd_easting,
            f.sd_northing,
            f.sd_depth,
            if f.usbl_rejected { 1 } else { 0 }
        ));
    }

    let summary = evaluate_pipeline_qc(survey_points, events, &delphins, 0.4064);

    let default_dpr = PipelineShiftDpr {
        survey_name: "Campeche Sound 16-Inch Gas Export Pipeline Integrity".to_string(),
        client_name: "PEMEX Exploracion y Produccion".to_string(),
        contractor_name: "UTM Offshore Survey Team".to_string(),
        vessel_name: "MV Deep Ocean Explorer".to_string(),
        shift_date: "2026-10-13".to_string(),
        shift_type: "Day Shift (00:00 - 12:00)".to_string(),
        senior_processor: "Senior Data Processor".to_string(),
        pipeline_id: "PL-442-16IN (Akall to Dos Bocas)".to_string(),
        kp_start: 0.0,
        kp_end: 5.0,
        km_processed_shift: 5.0,
        freespans_found_shift: summary.freespan_count,
        critical_spans_found: summary.critical_freespans,
        anodes_inspected_shift: summary.anodes_count,
        operational_hours: 11.0,
        weather_standby_hours: 0.5,
        technical_downtime_hours: 0.5,
        general_remarks: "Completed 5.0 km section from KP 0.000 to KP 5.000. Two critical freespans (FS-001 at KP 1.200, FS-004 at KP 4.600) flagged for urgent client review and remedial grout bag support. DelphINS USBL rejection rate 2.0% within specification.".to_string(),
        audit_hash: String::new(),
    };

    Ok(PipelineSampleDataset {
        route_csv,
        profile_csv,
        events_csv,
        delphins_csv,
        default_dpr,
        summary,
    })
}
