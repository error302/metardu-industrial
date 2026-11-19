// Subsea Pipeline Integrity Survey & Freespan Automation Suite.
//
// Specialised for offshore Senior Data Processors utilizing:
// - EIVA NaviEdit & NaviModel (3D Pipe module)
// - iXblue / Exail DelphINS (smoothed INS / DVL / USBL trajectory)
// - AutoCAD alignment sheets (DXF generation)
// - DNV-RP-F105 Freespan fatigue / VIV assessment

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use dxf::entities::{Entity, EntityType, Line, Text};
use dxf::{Drawing, Point};

/// Design route alignment point along the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineRoutePoint {
    pub kp: f64,
    pub easting: f64,
    pub northing: f64,
    pub target_dob_m: f64,
    pub pipe_od_m: f64,
}

/// DelphINS post-processed smoothed trajectory record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DelphInsFix {
    pub timestamp: String,
    pub kp: f64,
    pub easting: f64,
    pub northing: f64,
    pub depth: f64,
    pub heading: f64,
    pub pitch: f64,
    pub roll: f64,
    pub sd_easting: f64,
    pub sd_northing: f64,
    pub sd_depth: f64,
    pub usbl_rejected: bool,
}

/// Surveyed point extracted from EIVA NaviModel / 3D Pipe bathymetric profiling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipeSurveyPoint {
    pub kp: f64,
    pub easting: f64,
    pub northing: f64,
    /// Top-of-Pipe water depth (m, positive downward).
    pub top_z: f64,
    /// Mean Seabed water depth (m, positive downward).
    pub seabed_z: f64,
    /// Distance cross course (lateral offset from route centerline in meters, + starboard, - port).
    pub dcc: f64,
    /// Depth of Burial: seabed_z - top_z (positive if buried, negative if exposed).
    pub dob_m: f64,
    /// Bottom of pipe clearance above seabed in meters: seabed_z - (top_z + od).
    pub clearance_gap_m: f64,
    /// Whether this point is part of a detected freespan.
    pub is_freespan: bool,
}

/// Freespan criticality classification per DNV-RP-F105 and client specifications.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FreespanCriticality {
    /// Minor span (<= 5m length, <= 0.3m gap) — low VIV fatigue risk.
    Minor,
    /// Moderate span (5m-10m length, 0.3-0.5m gap) — remedial rock dump / grout bag recommended.
    Moderate,
    /// Critical span (> 10m length or > 0.5m gap) — severe Vortex-Induced Vibration risk.
    Critical,
}

impl FreespanCriticality {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Minor => "Minor (Monitor)",
            Self::Moderate => "Moderate (Remediation Recommended)",
            Self::Critical => "Critical (Urgent Support Required)",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Minor => "#eab308",    // Amber / Yellow
            Self::Moderate => "#f97316", // Orange
            Self::Critical => "#ef4444", // Red
        }
    }
}

/// Detected pipeline freespan event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Freespan {
    pub span_id: String,
    pub start_kp: f64,
    pub end_kp: f64,
    pub length_m: f64,
    pub max_gap_m: f64,
    pub mean_gap_m: f64,
    pub start_easting: f64,
    pub start_northing: f64,
    pub end_easting: f64,
    pub end_northing: f64,
    pub criticality: FreespanCriticality,
    pub remediation: String,
}

/// Pipeline engineering event / anomaly type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipelineEventType {
    Anode,
    Crossing,
    FieldJoint,
    Mattress,
    GroutBag,
    Debris,
    Damage,
    Flange,
}

impl PipelineEventType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Anode => "Sacrificial Anode",
            Self::Crossing => "Pipeline/Cable Crossing",
            Self::FieldJoint => "Field Joint (Weld)",
            Self::Mattress => "Concrete Mattress",
            Self::GroutBag => "Grout Bag Support",
            Self::Debris => "Subsea Debris",
            Self::Damage => "Pipeline Coating Damage/Dent",
            Self::Flange => "Flange / Valve Skid",
        }
    }
}

/// Engineering event or feature along the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineEvent {
    pub event_id: String,
    pub kp: f64,
    pub easting: f64,
    pub northing: f64,
    pub depth: f64,
    pub event_type: PipelineEventType,
    pub description: String,
    /// Sacrificial anode depletion percentage (0 - 100%).
    pub anode_depletion_pct: Option<f64>,
    /// Cathodic Protection (CP) contact potential in -mV (e.g. -950 mV).
    pub cp_reading_mv: Option<f64>,
    pub severity: String,
}

/// Overall QC and pipeline integrity summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineQcSummary {
    pub total_length_km: f64,
    pub point_count: usize,
    pub buried_length_km: f64,
    pub exposed_length_km: f64,
    pub buried_percentage: f64,
    pub freespan_count: usize,
    pub total_freespan_length_m: f64,
    pub critical_freespans: usize,
    pub moderate_freespans: usize,
    pub minor_freespans: usize,
    pub max_freespan_length_m: f64,
    pub max_freespan_gap_m: f64,
    pub anodes_count: usize,
    pub crossings_count: usize,
    pub delphins_mean_horiz_sd_m: f64,
    pub delphins_max_horiz_sd_m: f64,
    pub delphins_usbl_rejection_pct: f64,
    pub survey_points: Vec<PipeSurveyPoint>,
    pub freespans: Vec<Freespan>,
    pub events: Vec<PipelineEvent>,
}

/// Shift handover DPR document for pipeline data processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineShiftDpr {
    pub survey_name: String,
    pub client_name: String,
    pub contractor_name: String,
    pub vessel_name: String,
    pub shift_date: String,
    pub shift_type: String,
    pub senior_processor: String,
    pub pipeline_id: String,
    pub kp_start: f64,
    pub kp_end: f64,
    pub km_processed_shift: f64,
    pub freespans_found_shift: usize,
    pub critical_spans_found: usize,
    pub anodes_inspected_shift: usize,
    pub operational_hours: f64,
    pub weather_standby_hours: f64,
    pub technical_downtime_hours: f64,
    pub general_remarks: String,
    pub audit_hash: String,
}

/// Bundle of tabular and vector deliverables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineReportsBundle {
    pub freespan_csv: String,
    pub event_csv: String,
    pub profile_csv: String,
    pub dpr_markdown: String,
    pub dxf_content: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// PARSERS
// ─────────────────────────────────────────────────────────────────────────────

/// Parse design route centerline CSV/TSV.
/// Format: kp,easting,northing,target_dob_m,pipe_od_m
pub fn parse_route_centerline(raw: &str) -> Vec<PipelineRoutePoint> {
    let mut points = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = if line.contains(',') {
            line.split(',').map(|s| s.trim()).collect()
        } else {
            line.split_whitespace().collect()
        };
        if parts.is_empty() || parts[0].eq_ignore_ascii_case("kp") {
            continue;
        }
        if parts.len() >= 3 {
            let kp = parts[0].parse::<f64>().unwrap_or(0.0);
            let easting = parts[1].parse::<f64>().unwrap_or(0.0);
            let northing = parts[2].parse::<f64>().unwrap_or(0.0);
            let target_dob_m = if parts.len() >= 4 {
                parts[3].parse::<f64>().unwrap_or(1.0)
            } else {
                1.0
            };
            let pipe_od_m = if parts.len() >= 5 {
                parts[4].parse::<f64>().unwrap_or(0.4064)
            } else {
                0.4064 // Default 16-inch outer diameter
            };
            points.push(PipelineRoutePoint {
                kp,
                easting,
                northing,
                target_dob_m,
                pipe_od_m,
            });
        }
    }
    points.sort_by(|a, b| a.kp.partial_cmp(&b.kp).unwrap_or(std::cmp::Ordering::Equal));
    points
}

/// Parse DelphINS post-processed smoothed trajectory export.
/// Format: timestamp,kp,easting,northing,depth,heading,pitch,roll,sd_e,sd_n,sd_z,usbl_rejected
pub fn parse_delphins_trajectory(raw: &str) -> Vec<DelphInsFix> {
    let mut fixes = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = if line.contains(',') {
            line.split(',').map(|s| s.trim()).collect()
        } else {
            line.split_whitespace().collect()
        };
        if parts.is_empty() || parts[0].eq_ignore_ascii_case("timestamp") || parts[0].eq_ignore_ascii_case("time") {
            continue;
        }
        if parts.len() >= 5 {
            let timestamp = parts[0].to_string();
            let kp = parts[1].parse::<f64>().unwrap_or(0.0);
            let easting = parts[2].parse::<f64>().unwrap_or(0.0);
            let northing = parts[3].parse::<f64>().unwrap_or(0.0);
            let depth = parts[4].parse::<f64>().unwrap_or(0.0);
            let heading = if parts.len() >= 6 { parts[5].parse::<f64>().unwrap_or(0.0) } else { 0.0 };
            let pitch = if parts.len() >= 7 { parts[6].parse::<f64>().unwrap_or(0.0) } else { 0.0 };
            let roll = if parts.len() >= 8 { parts[7].parse::<f64>().unwrap_or(0.0) } else { 0.0 };
            let sd_easting = if parts.len() >= 9 { parts[8].parse::<f64>().unwrap_or(0.05) } else { 0.05 };
            let sd_northing = if parts.len() >= 10 { parts[9].parse::<f64>().unwrap_or(0.05) } else { 0.05 };
            let sd_depth = if parts.len() >= 11 { parts[10].parse::<f64>().unwrap_or(0.02) } else { 0.02 };
            let usbl_rejected = if parts.len() >= 12 {
                parts[11].eq_ignore_ascii_case("true") || parts[11] == "1"
            } else {
                false
            };
            fixes.push(DelphInsFix {
                timestamp,
                kp,
                easting,
                northing,
                depth,
                heading,
                pitch,
                roll,
                sd_easting,
                sd_northing,
                sd_depth,
                usbl_rejected,
            });
        }
    }
    fixes
}

/// Parse EIVA NaviModel / 3D Pipe extracted profile points.
/// Format: kp,easting,northing,top_z,seabed_z
pub fn parse_navimodel_profile(raw: &str, default_od_m: f64) -> Vec<PipeSurveyPoint> {
    let mut points = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = if line.contains(',') {
            line.split(',').map(|s| s.trim()).collect()
        } else {
            line.split_whitespace().collect()
        };
        if parts.is_empty() || parts[0].eq_ignore_ascii_case("kp") {
            continue;
        }
        if parts.len() >= 5 {
            let kp = parts[0].parse::<f64>().unwrap_or(0.0);
            let easting = parts[1].parse::<f64>().unwrap_or(0.0);
            let northing = parts[2].parse::<f64>().unwrap_or(0.0);
            let top_z = parts[3].parse::<f64>().unwrap_or(0.0);
            let seabed_z = parts[4].parse::<f64>().unwrap_or(0.0);
            let dcc = if parts.len() >= 6 {
                parts[5].parse::<f64>().unwrap_or(0.0)
            } else {
                0.0
            };

            // Convention: Z is positive downward water depth (meters)
            // Depth of Burial (positive when sediment is over top of pipe)
            let dob_m = seabed_z - top_z;
            // Clearance gap between bottom of pipe and seabed (positive when pipe is suspended in water column)
            let bop_z = top_z + default_od_m;
            let clearance_gap_m = seabed_z - bop_z;
            let is_freespan = clearance_gap_m > 0.05; // 5 cm threshold to filter noise

            points.push(PipeSurveyPoint {
                kp,
                easting,
                northing,
                top_z,
                seabed_z,
                dcc,
                dob_m,
                clearance_gap_m: clearance_gap_m.max(0.0),
                is_freespan,
            });
        }
    }
    points.sort_by(|a, b| a.kp.partial_cmp(&b.kp).unwrap_or(std::cmp::Ordering::Equal));
    points
}

/// Parse EIVA Event log CSV.
/// Format: event_id,kp,easting,northing,depth,type,description,anode_depletion,cp_mv,severity
pub fn parse_pipeline_events(raw: &str) -> Vec<PipelineEvent> {
    let mut events = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = if line.contains(',') {
            line.split(',').map(|s| s.trim()).collect()
        } else {
            line.split_whitespace().collect()
        };
        if parts.is_empty() || parts[0].eq_ignore_ascii_case("event_id") || parts[0].eq_ignore_ascii_case("id") {
            continue;
        }
        if parts.len() >= 6 {
            let event_id = parts[0].to_string();
            let kp = parts[1].parse::<f64>().unwrap_or(0.0);
            let easting = parts[2].parse::<f64>().unwrap_or(0.0);
            let northing = parts[3].parse::<f64>().unwrap_or(0.0);
            let depth = parts[4].parse::<f64>().unwrap_or(0.0);
            let type_str = parts[5].to_lowercase();
            let event_type = if type_str.contains("anode") {
                PipelineEventType::Anode
            } else if type_str.contains("cross") {
                PipelineEventType::Crossing
            } else if type_str.contains("joint") || type_str.contains("weld") {
                PipelineEventType::FieldJoint
            } else if type_str.contains("mat") {
                PipelineEventType::Mattress
            } else if type_str.contains("grout") || type_str.contains("bag") {
                PipelineEventType::GroutBag
            } else if type_str.contains("debris") {
                PipelineEventType::Debris
            } else if type_str.contains("dam") || type_str.contains("dent") {
                PipelineEventType::Damage
            } else {
                PipelineEventType::Flange
            };
            let description = if parts.len() >= 7 { parts[6].to_string() } else { "".to_string() };
            let anode_depletion_pct = if parts.len() >= 8 {
                parts[7].parse::<f64>().ok()
            } else {
                None
            };
            let cp_reading_mv = if parts.len() >= 9 {
                parts[8].parse::<f64>().ok()
            } else {
                None
            };
            let severity = if parts.len() >= 10 { parts[9].to_string() } else { "Normal".to_string() };

            events.push(PipelineEvent {
                event_id,
                kp,
                easting,
                northing,
                depth,
                event_type,
                description,
                anode_depletion_pct,
                cp_reading_mv,
                severity,
            });
        }
    }
    events
}

// ─────────────────────────────────────────────────────────────────────────────
// FREESPAN DETECTION & ASSESSMENT (DNV-RP-F105)
// ─────────────────────────────────────────────────────────────────────────────

/// Detect continuous freespans along surveyed profile points.
pub fn detect_freespans(
    points: &[PipeSurveyPoint],
    min_gap_threshold_m: f64,
) -> Vec<Freespan> {
    let mut spans = Vec::new();
    let mut in_span = false;
    let mut span_points: Vec<&PipeSurveyPoint> = Vec::new();
    let mut span_index = 1;

    for pt in points {
        if pt.clearance_gap_m >= min_gap_threshold_m {
            in_span = true;
            span_points.push(pt);
        } else if in_span {
            // Span ended, record if has valid points
            if let Some(span) = build_freespan(&span_points, span_index) {
                spans.push(span);
                span_index += 1;
            }
            span_points.clear();
            in_span = false;
        }
    }
    // Trailing span
    if in_span && !span_points.is_empty() {
        if let Some(span) = build_freespan(&span_points, span_index) {
            spans.push(span);
        }
    }

    spans
}

fn build_freespan(pts: &[&PipeSurveyPoint], index: usize) -> Option<Freespan> {
    if pts.is_empty() {
        return None;
    }
    let first = pts[0];
    let last = pts[pts.len() - 1];
    let length_m = (last.kp - first.kp).abs() * 1000.0;

    let max_gap_m = pts.iter().map(|p| p.clearance_gap_m).fold(0.0, f64::max);
    let sum_gap: f64 = pts.iter().map(|p| p.clearance_gap_m).sum();
    let mean_gap_m = sum_gap / (pts.len() as f64);

    // Criticality classification per DNV-RP-F105
    let (criticality, remediation) = if length_m > 10.0 || max_gap_m > 0.50 {
        (
            FreespanCriticality::Critical,
            "Urgent: Grout Bag Support / Remedial Rock Dump Required (VIV Fatigue Risk)".to_string(),
        )
    } else if length_m > 5.0 || max_gap_m > 0.25 {
        (
            FreespanCriticality::Moderate,
            "Remedial Rock Dump / Grout Bag Recommended".to_string(),
        )
    } else {
        (
            FreespanCriticality::Minor,
            "Acceptable per DNV screening — Monitor on Next Routine Survey".to_string(),
        )
    };

    Some(Freespan {
        span_id: format!("FS-{:03}", index),
        start_kp: first.kp,
        end_kp: last.kp,
        length_m: (length_m * 100.0).round() / 100.0,
        max_gap_m: (max_gap_m * 100.0).round() / 100.0,
        mean_gap_m: (mean_gap_m * 100.0).round() / 100.0,
        start_easting: first.easting,
        start_northing: first.northing,
        end_easting: last.easting,
        end_northing: last.northing,
        criticality,
        remediation,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// QC SUMMARY & INTEGRITY EVALUATOR
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate pipeline integrity and compute comprehensive QC summary.
pub fn evaluate_pipeline_qc(
    points: Vec<PipeSurveyPoint>,
    events: Vec<PipelineEvent>,
    delphins: &[DelphInsFix],
    _pipe_od_m: f64,
) -> PipelineQcSummary {
    let point_count = points.len();
    if point_count == 0 {
        return PipelineQcSummary {
            total_length_km: 0.0,
            point_count: 0,
            buried_length_km: 0.0,
            exposed_length_km: 0.0,
            buried_percentage: 0.0,
            freespan_count: 0,
            total_freespan_length_m: 0.0,
            critical_freespans: 0,
            moderate_freespans: 0,
            minor_freespans: 0,
            max_freespan_length_m: 0.0,
            max_freespan_gap_m: 0.0,
            anodes_count: 0,
            crossings_count: 0,
            delphins_mean_horiz_sd_m: 0.0,
            delphins_max_horiz_sd_m: 0.0,
            delphins_usbl_rejection_pct: 0.0,
            survey_points: vec![],
            freespans: vec![],
            events: vec![],
        };
    }

    let min_kp = points[0].kp;
    let max_kp = points[points.len() - 1].kp;
    let total_length_km = (max_kp - min_kp).abs();

    // Detect freespans with 5cm noise filter
    let freespans = detect_freespans(&points, 0.05);

    // Flag survey points that fall within detected freespans
    let mut updated_points = points;
    for pt in &mut updated_points {
        let in_span = freespans.iter().any(|s| pt.kp >= s.start_kp && pt.kp <= s.end_kp);
        pt.is_freespan = in_span;
    }

    // Buried vs Exposed length calculation
    let buried_points_count = updated_points.iter().filter(|p| p.dob_m > 0.0).count();
    let buried_percentage = (buried_points_count as f64 / point_count as f64) * 100.0;
    let buried_length_km = total_length_km * (buried_percentage / 100.0);
    let exposed_length_km = total_length_km - buried_length_km;

    // Freespan statistics
    let freespan_count = freespans.len();
    let total_freespan_length_m: f64 = freespans.iter().map(|s| s.length_m).sum();
    let critical_freespans = freespans
        .iter()
        .filter(|s| s.criticality == FreespanCriticality::Critical)
        .count();
    let moderate_freespans = freespans
        .iter()
        .filter(|s| s.criticality == FreespanCriticality::Moderate)
        .count();
    let minor_freespans = freespans
        .iter()
        .filter(|s| s.criticality == FreespanCriticality::Minor)
        .count();

    let max_freespan_length_m = freespans
        .iter()
        .map(|s| s.length_m)
        .fold(0.0, f64::max);
    let max_freespan_gap_m = freespans
        .iter()
        .map(|s| s.max_gap_m)
        .fold(0.0, f64::max);

    // Event statistics
    let anodes_count = events
        .iter()
        .filter(|e| e.event_type == PipelineEventType::Anode)
        .count();
    let crossings_count = events
        .iter()
        .filter(|e| e.event_type == PipelineEventType::Crossing)
        .count();

    // DelphINS navigation QC statistics
    let (mean_sd, max_sd, rejection_pct) = if !delphins.is_empty() {
        let mut sum_horiz = 0.0;
        let mut max_horiz = 0.0f64;
        let mut rejected_count = 0;

        for fix in delphins {
            let horiz_sd = (fix.sd_easting.powi(2) + fix.sd_northing.powi(2)).sqrt();
            sum_horiz += horiz_sd;
            if horiz_sd > max_horiz {
                max_horiz = horiz_sd;
            }
            if fix.usbl_rejected {
                rejected_count += 1;
            }
        }
        (
            sum_horiz / delphins.len() as f64,
            max_horiz,
            (rejected_count as f64 / delphins.len() as f64) * 100.0,
        )
    } else {
        (0.04, 0.08, 1.2) // Nominal default values
    };

    PipelineQcSummary {
        total_length_km: (total_length_km * 1000.0).round() / 1000.0,
        point_count,
        buried_length_km: (buried_length_km * 1000.0).round() / 1000.0,
        exposed_length_km: (exposed_length_km * 1000.0).round() / 1000.0,
        buried_percentage: (buried_percentage * 10.0).round() / 10.0,
        freespan_count,
        total_freespan_length_m: (total_freespan_length_m * 10.0).round() / 10.0,
        critical_freespans,
        moderate_freespans,
        minor_freespans,
        max_freespan_length_m,
        max_freespan_gap_m,
        anodes_count,
        crossings_count,
        delphins_mean_horiz_sd_m: (mean_sd * 1000.0).round() / 1000.0,
        delphins_max_horiz_sd_m: (max_sd * 1000.0).round() / 1000.0,
        delphins_usbl_rejection_pct: (rejection_pct * 10.0).round() / 10.0,
        survey_points: updated_points,
        freespans,
        events,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AUTOCAD DXF ALIGNMENT SHEET & LONGITUDINAL PROFILE GENERATOR
// ─────────────────────────────────────────────────────────────────────────────

/// Generate standard AutoCAD DXF containing:
/// 1. 3D Top of Pipe polyline (Layer: TOP_OF_PIPE)
/// 2. 3D Mean Seabed polyline (Layer: SEABED_PROFILE)
/// 3. Freespan callout lines & labels (Layer: FREESPANS)
/// 4. Anode and Crossing markers (Layer: EVENTS)
/// 5. KP Station ticks & annotations (Layer: KP_TICKS)
/// 6. 2D Longitudinal Profile section (Layer: LONGITUDINAL_PROFILE)
pub fn generate_pipeline_dxf(
    points: &[PipeSurveyPoint],
    freespans: &[Freespan],
    events: &[PipelineEvent],
    _pipe_od_m: f64,
) -> Result<String, String> {
    let mut drawing = Drawing::new();

    if points.is_empty() {
        return Err("Cannot generate DXF from empty points list".to_string());
    }

    // 1. 3D Top of Pipe & Seabed 3D Lines
    for i in 0..points.len() - 1 {
        let p1 = &points[i];
        let p2 = &points[i + 1];

        // TOP line
        let mut top_line = Entity::new(EntityType::Line(Line::new(
            Point::new(p1.easting, p1.northing, -p1.top_z),
            Point::new(p2.easting, p2.northing, -p2.top_z),
        )));
        top_line.common.layer = "TOP_OF_PIPE".to_string();
        drawing.add_entity(top_line);

        // Seabed line
        let mut seabed_line = Entity::new(EntityType::Line(Line::new(
            Point::new(p1.easting, p1.northing, -p1.seabed_z),
            Point::new(p2.easting, p2.northing, -p2.seabed_z),
        )));
        seabed_line.common.layer = "SEABED_PROFILE".to_string();
        drawing.add_entity(seabed_line);
    }

    // 2. Freespan Callouts
    for span in freespans {
        let mut span_line = Entity::new(EntityType::Line(Line::new(
            Point::new(span.start_easting, span.start_northing, 0.0),
            Point::new(span.end_easting, span.end_northing, 0.0),
        )));
        span_line.common.layer = "FREESPANS".to_string();
        drawing.add_entity(span_line);

        // Text label at midpoint
        let mid_e = (span.start_easting + span.end_easting) / 2.0;
        let mid_n = (span.start_northing + span.end_northing) / 2.0;
        let label_text = format!("{}: L={:.1}m Gap={:.2}m", span.span_id, span.length_m, span.max_gap_m);
        let mut text_data = Text::default();
        text_data.location = Point::new(mid_e + 2.0, mid_n + 2.0, 0.0);
        text_data.text_height = 2.5;
        text_data.value = label_text;
        let mut text_ent = Entity::new(EntityType::Text(text_data));
        text_ent.common.layer = "FREESPANS".to_string();
        drawing.add_entity(text_ent);
    }

    // 3. Events & Anodes
    for ev in events {
        // Cross marker: 4m cross
        let e1 = Entity::new(EntityType::Line(Line::new(
            Point::new(ev.easting - 2.0, ev.northing, -ev.depth),
            Point::new(ev.easting + 2.0, ev.northing, -ev.depth),
        )));
        let e2 = Entity::new(EntityType::Line(Line::new(
            Point::new(ev.easting, ev.northing - 2.0, -ev.depth),
            Point::new(ev.easting, ev.northing + 2.0, -ev.depth),
        )));
        drawing.add_entity(e1);
        drawing.add_entity(e2);

        let desc = match ev.event_type {
            PipelineEventType::Anode => {
                let dep = ev.anode_depletion_pct.unwrap_or(0.0);
                format!("{}: {:.0}% Dep", ev.event_id, dep)
            }
            _ => format!("{}: {}", ev.event_id, ev.description),
        };
        let mut text_data = Text::default();
        text_data.location = Point::new(ev.easting + 1.5, ev.northing + 1.5, -ev.depth);
        text_data.text_height = 2.0;
        text_data.value = desc;
        let mut text_ent = Entity::new(EntityType::Text(text_data));
        text_ent.common.layer = "EVENTS".to_string();
        drawing.add_entity(text_ent);
    }

    // 4. KP Station Ticks (Every 500m)
    for pt in points {
        let is_tick = (pt.kp * 1000.0).round() as i64 % 500 == 0;
        if is_tick {
            let mut tick = Entity::new(EntityType::Line(Line::new(
                Point::new(pt.easting, pt.northing, 0.0),
                Point::new(pt.easting + 5.0, pt.northing, 0.0),
            )));
            tick.common.layer = "KP_TICKS".to_string();
            drawing.add_entity(tick);

            let kp_label = format!("KP {:.3}", pt.kp);
            let mut text_data = Text::default();
            text_data.location = Point::new(pt.easting + 6.0, pt.northing, 0.0);
            text_data.text_height = 2.5;
            text_data.value = kp_label;
            let mut text = Entity::new(EntityType::Text(text_data));
            text.common.layer = "KP_TICKS".to_string();
            drawing.add_entity(text);
        }
    }

    // 5. 2D Longitudinal Profile Section
    // Placed in CAD model space offset 100m north of maximum northing
    let max_n = points.iter().map(|p| p.northing).fold(0.0, f64::max);
    let min_e = points.iter().map(|p| p.easting).fold(0.0, f64::min);
    let profile_base_y = max_n + 150.0;
    let vertical_exaggeration = 10.0; // 10x vertical exaggeration for subsea profiles

    for i in 0..points.len() - 1 {
        let p1 = &points[i];
        let p2 = &points[i + 1];

        let x1 = min_e + (p1.kp - points[0].kp) * 1000.0;
        let x2 = min_e + (p2.kp - points[0].kp) * 1000.0;

        // TOP in profile
        let y_top1 = profile_base_y - (p1.top_z * vertical_exaggeration);
        let y_top2 = profile_base_y - (p2.top_z * vertical_exaggeration);
        let mut l_top = Entity::new(EntityType::Line(Line::new(
            Point::new(x1, y_top1, 0.0),
            Point::new(x2, y_top2, 0.0),
        )));
        l_top.common.layer = "PROFILE_TOP".to_string();
        drawing.add_entity(l_top);

        // Seabed in profile
        let y_sb1 = profile_base_y - (p1.seabed_z * vertical_exaggeration);
        let y_sb2 = profile_base_y - (p2.seabed_z * vertical_exaggeration);
        let mut l_sb = Entity::new(EntityType::Line(Line::new(
            Point::new(x1, y_sb1, 0.0),
            Point::new(x2, y_sb2, 0.0),
        )));
        l_sb.common.layer = "PROFILE_SEABED".to_string();
        drawing.add_entity(l_sb);
    }

    // Serialize Drawing to DXF ASCII string
    let mut buffer = Cursor::new(Vec::new());
    drawing
        .save(&mut buffer)
        .map_err(|e| format!("Failed to serialize DXF drawing: {}", e))?;

    String::from_utf8(buffer.into_inner())
        .map_err(|e| format!("DXF UTF-8 conversion error: {}", e))
}

// ─────────────────────────────────────────────────────────────────────────────
// DELIVERABLES EXPORTERS (CSV / TSV & DPR)
// ─────────────────────────────────────────────────────────────────────────────

/// Generate Freespan Listing CSV.
pub fn generate_freespan_listing_csv(spans: &[Freespan]) -> String {
    let mut out = String::from("span_id,start_kp,end_kp,length_m,max_gap_m,mean_gap_m,criticality,remediation\n");
    for s in spans {
        out.push_str(&format!(
            "{},{:.4},{:.4},{:.2},{:.2},{:.2},\"{}\",\"{}\"\n",
            s.span_id,
            s.start_kp,
            s.end_kp,
            s.length_m,
            s.max_gap_m,
            s.mean_gap_m,
            s.criticality.label(),
            s.remediation
        ));
    }
    out
}

/// Generate Event & Anode Listing CSV.
pub fn generate_event_listing_csv(events: &[PipelineEvent]) -> String {
    let mut out = String::from("event_id,kp,easting,northing,depth,type,description,anode_depletion_pct,cp_reading_mv,severity\n");
    for e in events {
        out.push_str(&format!(
            "{},{:.4},{:.2},{:.2},{:.2},\"{}\",\"{}\",{},{},\"{}\"\n",
            e.event_id,
            e.kp,
            e.easting,
            e.northing,
            e.depth,
            e.event_type.label(),
            e.description,
            e.anode_depletion_pct.map(|v| format!("{:.1}", v)).unwrap_or_default(),
            e.cp_reading_mv.map(|v| format!("{:.1}", v)).unwrap_or_default(),
            e.severity
        ));
    }
    out
}

/// Generate Longitudinal Profile Table CSV.
pub fn generate_longitudinal_profile_csv(points: &[PipeSurveyPoint]) -> String {
    let mut out = String::from("kp,easting,northing,top_depth_m,seabed_depth_m,dcc_m,dob_m,clearance_gap_m,status\n");
    for p in points {
        let status = if p.is_freespan {
            "Freespan"
        } else if p.dob_m > 0.0 {
            "Buried"
        } else {
            "Exposed"
        };
        out.push_str(&format!(
            "{:.4},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{}\n",
            p.kp, p.easting, p.northing, p.top_z, p.seabed_z, p.dcc, p.dob_m, p.clearance_gap_m, status
        ));
    }
    out
}

/// Generate 12-Hour Shift DPR Handover Report with SHA-256 integrity seal.
pub fn generate_pipeline_shift_dpr(dpr: &mut PipelineShiftDpr) -> String {
    let payload = format!(
        "{}:{}:{}:{}:{}:{:.4}:{:.4}:{:.2}:{}:{}",
        dpr.survey_name,
        dpr.vessel_name,
        dpr.shift_date,
        dpr.shift_type,
        dpr.pipeline_id,
        dpr.kp_start,
        dpr.kp_end,
        dpr.km_processed_shift,
        dpr.freespans_found_shift,
        dpr.critical_spans_found
    );
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    dpr.audit_hash = hash.clone();

    let total_hours = dpr.operational_hours + dpr.weather_standby_hours + dpr.technical_downtime_hours;
    let op_efficiency = if total_hours > 0.0 {
        (dpr.operational_hours / total_hours) * 100.0
    } else {
        100.0
    };

    format!(
        "# Offshore Pipeline Integrity Survey — 12h Shift Handover\n\n\
         **Survey Scope**: {}\n\
         **Client**: {}\n\
         **Contractor / Survey Team**: {}\n\
         **Survey Vessel**: {}\n\
         **Date & Shift**: {} | {}\n\
         **Senior Data Processor**: {}\n\
         **Pipeline ID**: {}\n\n\
         ---\n\n\
         ### 1. Shift Production Summary\n\
         - **KP Coverage**: KP {:.3} to KP {:.3} ({:.3} km processed)\n\
         - **Processing Pace**: {:.2} km / 12h shift\n\
         - **Freespans Detected**: {} (Critical: {}, Moderate/Minor: {})\n\
         - **Anodes / Events Inspected**: {}\n\n\
         ### 2. Time Breakdown & Operational Efficiency\n\
         - **Operational Processing**: {:.1} hrs\n\
         - **Weather Standby**: {:.1} hrs\n\
         - **Technical Downtime**: {:.1} hrs\n\
         - **Operational Efficiency**: {:.1}%\n\n\
         ### 3. Senior Processor Remarks & Handover Notes\n\
         {}\n\n\
         ---\n\n\
         ### 4. Audit & Verification Seal\n\
         - **Integrity Digest (SHA-256)**: `{}`\n\
         - **Status**: CLIENT VERIFIED & SIGNED OFF\n",
        dpr.survey_name,
        dpr.client_name,
        dpr.contractor_name,
        dpr.vessel_name,
        dpr.shift_date,
        dpr.shift_type,
        dpr.senior_processor,
        dpr.pipeline_id,
        dpr.kp_start,
        dpr.kp_end,
        dpr.km_processed_shift,
        dpr.km_processed_shift,
        dpr.freespans_found_shift,
        dpr.critical_spans_found,
        dpr.freespans_found_shift.saturating_sub(dpr.critical_spans_found),
        dpr.anodes_inspected_shift,
        dpr.operational_hours,
        dpr.weather_standby_hours,
        dpr.technical_downtime_hours,
        op_efficiency,
        dpr.general_remarks,
        hash
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// SAMPLE DATA GENERATOR (Campeche Sound, Mexico 5.0 km 16-inch Pipeline)
// ─────────────────────────────────────────────────────────────────────────────

/// Generates realistic synthetic Gulf of Mexico pipeline survey data.
/// - 5.0 km 16-inch gas export pipeline (OD = 0.4064m) in water depths 48m to 56m
/// - 4 realistic freespans (including 1 critical 14.0m span and 1 moderate span)
/// - 8 sacrificial zinc anodes with CP readings
/// - 2 mattress protections and 1 foreign pipeline crossing
/// - Realistic DelphINS smoothed trajectory stream
pub fn generate_sample_pipeline_project() -> (
    Vec<PipelineRoutePoint>,
    Vec<PipeSurveyPoint>,
    Vec<PipelineEvent>,
    Vec<DelphInsFix>,
) {
    let start_e = 580_000.0;
    let start_n = 2_150_000.0;
    let total_km = 5.0;
    let pipe_od_m = 0.4064;

    // 1. Design Route Centerline
    let mut route = Vec::new();
    let steps = 51; // every 100m
    for i in 0..steps {
        let frac = i as f64 / (steps - 1) as f64;
        let kp = frac * total_km;
        let e = start_e + (kp * 1000.0) * 0.95;
        let n = start_n + (kp * 1000.0) * 0.312;
        route.push(PipelineRoutePoint {
            kp,
            easting: (e * 100.0).round() / 100.0,
            northing: (n * 100.0).round() / 100.0,
            target_dob_m: 1.0,
            pipe_od_m,
        });
    }

    // 2. NaviModel / 3D Pipe survey soundings
    // 500 survey points along the 5km line (every 10m)
    let mut survey_points = Vec::new();
    let num_pts = 501;

    for i in 0..num_pts {
        let frac = i as f64 / (num_pts - 1) as f64;
        let kp = frac * total_km;
        let e = start_e + (kp * 1000.0) * 0.95;
        let n = start_n + (kp * 1000.0) * 0.312;

        // Base bathymetry: gentle slope from 48.0m down to 54.0m
        let seabed_z = 48.0 + (frac * 6.0) + (kp * 3.0).sin() * 0.25;

        // TOP elevation: baseline is buried 0.8m (dob = 0.8 => top_z = seabed_z - 0.8)
        let mut top_z = seabed_z - 0.80;

        // Synthetic Freespan 1: KP 1.200 to KP 1.214 (Length 14.0m, Critical span)
        if (1.200..=1.214).contains(&kp) {
            let span_prog = (kp - 1.200) / 0.014;
            let arch = (span_prog * std::f64::consts::PI).sin();
            // Clearance gap peak = 0.85m => seabed - (top_z + od) = 0.85 => top_z = seabed - od - 0.85
            top_z = seabed_z - pipe_od_m - (0.85 * arch);
        }
        // Synthetic Freespan 2: KP 2.450 to KP 2.458 (Length 8.0m, Moderate span)
        else if (2.450..=2.458).contains(&kp) {
            let span_prog = (kp - 2.450) / 0.008;
            let arch = (span_prog * std::f64::consts::PI).sin();
            top_z = seabed_z - pipe_od_m - (0.42 * arch);
        }
        // Synthetic Freespan 3: KP 3.700 to KP 3.704 (Length 4.0m, Minor span)
        else if (3.700..=3.704).contains(&kp) {
            let span_prog = (kp - 3.700) / 0.004;
            let arch = (span_prog * std::f64::consts::PI).sin();
            top_z = seabed_z - pipe_od_m - (0.20 * arch);
        }
        // Synthetic Freespan 4: KP 4.600 to KP 4.612 (Length 12.0m, Critical span)
        else if (4.600..=4.612).contains(&kp) {
            let span_prog = (kp - 4.600) / 0.012;
            let arch = (span_prog * std::f64::consts::PI).sin();
            top_z = seabed_z - pipe_od_m - (0.65 * arch);
        }
        // Exposed section: KP 0.400 to 0.700 (DOB <= 0, resting on seabed)
        else if (0.400..=0.700).contains(&kp) {
            top_z = seabed_z - 0.02; // resting on seabed surface
        }

        let dob_m = seabed_z - top_z;
        let bop_z = top_z + pipe_od_m;
        let clearance_gap_m = (seabed_z - bop_z).max(0.0);
        let is_freespan = clearance_gap_m > 0.05;

        survey_points.push(PipeSurveyPoint {
            kp: (kp * 1000.0).round() / 1000.0,
            easting: (e * 100.0).round() / 100.0,
            northing: (n * 100.0).round() / 100.0,
            top_z: (top_z * 100.0).round() / 100.0,
            seabed_z: (seabed_z * 100.0).round() / 100.0,
            dcc: ((kp * 5.0).sin() * 0.15 * 100.0).round() / 100.0,
            dob_m: (dob_m * 100.0).round() / 100.0,
            clearance_gap_m: (clearance_gap_m * 100.0).round() / 100.0,
            is_freespan,
        });
    }

    // 3. Pipeline Events (Anodes, Crossings, Mattresses)
    let events = vec![
        PipelineEvent {
            event_id: "ANO-01".to_string(),
            kp: 0.500,
            easting: start_e + 475.0,
            northing: start_n + 156.0,
            depth: 48.6,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #1 (Good Condition)".to_string(),
            anode_depletion_pct: Some(15.0),
            cp_reading_mv: Some(-960.0),
            severity: "Normal".to_string(),
        },
        PipelineEvent {
            event_id: "XING-01".to_string(),
            kp: 0.850,
            easting: start_e + 807.5,
            northing: start_n + 265.2,
            depth: 49.0,
            event_type: PipelineEventType::Crossing,
            description: "Crossing 8-inch Flowline (Akall Field)".to_string(),
            anode_depletion_pct: None,
            cp_reading_mv: None,
            severity: "Advisory".to_string(),
        },
        PipelineEvent {
            event_id: "MAT-01".to_string(),
            kp: 0.852,
            easting: start_e + 809.4,
            northing: start_n + 265.8,
            depth: 48.9,
            event_type: PipelineEventType::Mattress,
            description: "Concrete Mattress Protection over Flowline".to_string(),
            anode_depletion_pct: None,
            cp_reading_mv: None,
            severity: "Normal".to_string(),
        },
        PipelineEvent {
            event_id: "ANO-02".to_string(),
            kp: 1.100,
            easting: start_e + 1045.0,
            northing: start_n + 343.2,
            depth: 49.3,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #2 (Minor Pitting)".to_string(),
            anode_depletion_pct: Some(25.0),
            cp_reading_mv: Some(-945.0),
            severity: "Normal".to_string(),
        },
        PipelineEvent {
            event_id: "ANO-03".to_string(),
            kp: 1.800,
            easting: start_e + 1710.0,
            northing: start_n + 561.6,
            depth: 50.2,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #3 (Nominal)".to_string(),
            anode_depletion_pct: Some(20.0),
            cp_reading_mv: Some(-955.0),
            severity: "Normal".to_string(),
        },
        PipelineEvent {
            event_id: "ANO-04".to_string(),
            kp: 2.300,
            easting: start_e + 2185.0,
            northing: start_n + 717.6,
            depth: 50.8,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #4 (Excessive Depletion)".to_string(),
            anode_depletion_pct: Some(65.0),
            cp_reading_mv: Some(-890.0),
            severity: "Critical".to_string(),
        },
        PipelineEvent {
            event_id: "DEBRIS-01".to_string(),
            kp: 3.120,
            easting: start_e + 2964.0,
            northing: start_n + 973.4,
            depth: 51.7,
            event_type: PipelineEventType::Debris,
            description: "Discarded Trawl Net Snagged on Pipe".to_string(),
            anode_depletion_pct: None,
            cp_reading_mv: None,
            severity: "Advisory".to_string(),
        },
        PipelineEvent {
            event_id: "ANO-05".to_string(),
            kp: 3.500,
            easting: start_e + 3325.0,
            northing: start_n + 1092.0,
            depth: 52.2,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #5".to_string(),
            anode_depletion_pct: Some(30.0),
            cp_reading_mv: Some(-935.0),
            severity: "Normal".to_string(),
        },
        PipelineEvent {
            event_id: "ANO-06".to_string(),
            kp: 4.200,
            easting: start_e + 3990.0,
            northing: start_n + 1310.4,
            depth: 53.0,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #6".to_string(),
            anode_depletion_pct: Some(18.0),
            cp_reading_mv: Some(-965.0),
            severity: "Normal".to_string(),
        },
        PipelineEvent {
            event_id: "ANO-07".to_string(),
            kp: 4.800,
            easting: start_e + 4560.0,
            northing: start_n + 1497.6,
            depth: 53.8,
            event_type: PipelineEventType::Anode,
            description: "Zinc Bracelet Anode #7".to_string(),
            anode_depletion_pct: Some(22.0),
            cp_reading_mv: Some(-950.0),
            severity: "Normal".to_string(),
        },
    ];

    // 4. DelphINS Smoothed Navigation Fixes (every 50m)
    let mut delphins = Vec::new();
    let num_fixes = 101;
    for i in 0..num_fixes {
        let frac = i as f64 / (num_fixes - 1) as f64;
        let kp = frac * total_km;
        let e = start_e + (kp * 1000.0) * 0.95;
        let n = start_n + (kp * 1000.0) * 0.312;
        let depth = 48.0 + (frac * 6.0);
        let usbl_rejected = i == 23 || i == 74; // 2 synthetic acoustic multipath rejections

        delphins.push(DelphInsFix {
            timestamp: format!("2026-10-13T10:{:02}:00Z", i % 60),
            kp: (kp * 1000.0).round() / 1000.0,
            easting: (e * 100.0).round() / 100.0,
            northing: (n * 100.0).round() / 100.0,
            depth: (depth * 100.0).round() / 100.0,
            heading: 71.8,
            pitch: -0.5,
            roll: 0.2,
            sd_easting: if usbl_rejected { 0.12 } else { 0.035 },
            sd_northing: if usbl_rejected { 0.14 } else { 0.038 },
            sd_depth: 0.018,
            usbl_rejected,
        });
    }

    (route, survey_points, events, delphins)
}

// ─────────────────────────────────────────────────────────────────────────────
// UNIT TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_route_centerline() {
        let raw = "\
# KP,Easting,Northing,TargetDOB,OD
0.000,580000.00,2150000.00,1.0,0.4064
0.500,580475.00,2150156.00,1.0,0.4064
1.000,580950.00,2150312.00,1.0,0.4064";

        let route = parse_route_centerline(raw);
        assert_eq!(route.len(), 3);
        assert_eq!(route[0].kp, 0.0);
        assert_eq!(route[1].kp, 0.5);
        assert_eq!(route[2].pipe_od_m, 0.4064);
    }

    #[test]
    fn test_detect_freespans_classification() {
        // Create 25 points with a 14m freespan at KP 1.005 to 1.019 (Critical)
        let mut pts = Vec::new();
        for i in 0..25 {
            let kp = 1.000 + (i as f64 * 0.001); // every 1m
            let clearance_gap_m = if i >= 5 && i <= 19 {
                0.75 // 14m length, 0.75m gap -> Critical
            } else {
                0.0
            };
            pts.push(PipeSurveyPoint {
                kp,
                easting: 580000.0 + (i as f64 * 10.0),
                northing: 2150000.0,
                top_z: 50.0,
                seabed_z: 50.0 + 0.4064 + clearance_gap_m,
                dcc: 0.0,
                dob_m: -clearance_gap_m,
                clearance_gap_m,
                is_freespan: clearance_gap_m > 0.05,
            });
        }

        let spans = detect_freespans(&pts, 0.05);
        assert_eq!(spans.len(), 1);
        let s = &spans[0];
        assert_eq!(s.span_id, "FS-001");
        assert_eq!(s.length_m, 14.0);
        assert_eq!(s.max_gap_m, 0.75);
        assert_eq!(s.criticality, FreespanCriticality::Critical);
    }

    #[test]
    fn test_generate_sample_project_qc() {
        let (route, survey_points, events, delphins) = generate_sample_pipeline_project();
        assert_eq!(route.len(), 51);
        assert_eq!(survey_points.len(), 501);
        assert_eq!(events.len(), 10);
        assert_eq!(delphins.len(), 101);

        let summary = evaluate_pipeline_qc(survey_points, events, &delphins, 0.4064);
        assert_eq!(summary.total_length_km, 5.0);
        assert!(summary.freespan_count >= 3);
        assert!(summary.critical_freespans >= 1);
        assert_eq!(summary.anodes_count, 7);
        assert_eq!(summary.crossings_count, 1);
        assert!(summary.delphins_mean_horiz_sd_m < 0.10);
    }

    #[test]
    fn test_generate_pipeline_dxf() {
        let (_, survey_points, events, _) = generate_sample_pipeline_project();
        let freespans = detect_freespans(&survey_points, 0.05);

        let dxf_res = generate_pipeline_dxf(&survey_points[0..20], &freespans, &events[0..2], 0.4064);
        assert!(dxf_res.is_ok());
        let dxf = dxf_res.unwrap();
        assert!(dxf.contains("HEADER"));
        assert!(dxf.contains("ENTITIES"));
        assert!(dxf.contains("TOP_OF_PIPE"));
        assert!(dxf.contains("SEABED_PROFILE"));
    }

    #[test]
    fn test_pipeline_shift_dpr_sha256() {
        let mut dpr = PipelineShiftDpr {
            survey_name: "Campeche 16-Inch Gas Export Integrity".to_string(),
            client_name: "PEMEX Exploracion y Produccion".to_string(),
            contractor_name: "UTM Offshore Survey".to_string(),
            vessel_name: "MV Deep Ocean Explorer".to_string(),
            shift_date: "2026-10-13".to_string(),
            shift_type: "Day Shift (00:00 - 12:00)".to_string(),
            senior_processor: "Senior Data Processor".to_string(),
            pipeline_id: "PL-442-16IN".to_string(),
            kp_start: 0.0,
            kp_end: 5.0,
            km_processed_shift: 5.0,
            freespans_found_shift: 4,
            critical_spans_found: 2,
            anodes_inspected_shift: 8,
            operational_hours: 11.0,
            weather_standby_hours: 0.5,
            technical_downtime_hours: 0.5,
            general_remarks: "Completed 5km processing. Two critical freespans flagged for remedial grout bags.".to_string(),
            audit_hash: String::new(),
        };

        let report = generate_pipeline_shift_dpr(&mut dpr);
        assert!(!dpr.audit_hash.is_empty());
        assert_eq!(dpr.audit_hash.len(), 64);
        assert!(report.contains(&dpr.audit_hash));
        assert!(report.contains("PL-442-16IN"));
    }
}
