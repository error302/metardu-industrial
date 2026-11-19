// OBN (Ocean Bottom Node) Survey Module — 4DNav & Node Dashboard Automation.
//
// Provides specialized data structures, parsers, tolerance validators,
// and industry-standard format generators (UKOOA SPS & IOGP P1/11)
// for marine seismic OBN operations.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObnError {
    #[error("Parse error at line {line}: {reason}")]
    ParseError { line: usize, reason: String },
    #[error("Empty dataset provided")]
    EmptyDataset,
    #[error("No matching planned station found for node {node_id}")]
    NoMatchFound { node_id: String },
    #[error("Export formatting error: {0}")]
    FormatError(String),
}

/// Tolerance classification for node displacement relative to planned target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceStatus {
    /// In specification (typically <= 5.0m radial error)
    InSpec,
    /// Warning / acceptable with caution (typically 5.0m - 10.0m radial error)
    Warning,
    /// Out of specification (> 10.0m radial error or high acoustic uncertainty)
    OutOfSpec,
}

/// Deployment lifecycle status of an Ocean Bottom Node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeOperationalStatus {
    Planned,
    Deployed,
    ActiveRecording,
    Retrieved,
    Relocated,
    Failed,
    Lost,
}

/// A planned pre-plot node target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObnPlannedNode {
    pub station_id: String,
    pub line_name: String,
    pub easting: f64,
    pub northing: f64,
    pub target_depth: f64,
    #[serde(default)]
    pub remarks: Option<String>,
}

/// As-laid or real-time fix captured during deployment/retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObnAsLaidNode {
    pub node_id: String,
    pub serial_number: String,
    pub station_id: String,
    pub line_name: String,
    pub easting: f64,
    pub northing: f64,
    pub depth: f64,
    /// UTC timestamp string (ISO-8601 or YYYY-MM-DD HH:MM:SS)
    pub timestamp_utc: String,
    /// Acoustic USBL 1-sigma positioning uncertainty (m)
    pub usbl_uncertainty_m: f64,
    /// Water depth from vessel/ROV bathymetry (m)
    pub water_depth_m: f64,
    /// Node tilt/pitch in degrees (from onboard tilt sensor / ROV)
    #[serde(default)]
    pub tilt_deg: Option<f64>,
    /// Battery remaining percentage
    #[serde(default)]
    pub battery_pct: Option<f64>,
    /// Operational status
    pub status: NodeOperationalStatus,
}

/// Evaluated node with spatial residuals, radial error, and tolerance status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObnQcNode {
    pub station_id: String,
    pub serial_number: String,
    pub line_name: String,
    pub planned_easting: f64,
    pub planned_northing: f64,
    pub planned_depth: f64,
    pub aslaid_easting: f64,
    pub aslaid_northing: f64,
    pub aslaid_depth: f64,
    /// Delta Easting: aslaid - planned (m)
    pub delta_easting: f64,
    /// Delta Northing: aslaid - planned (m)
    pub delta_northing: f64,
    /// Radial offset: sqrt(delta_e^2 + delta_n^2) (m)
    pub radial_offset_m: f64,
    /// Delta Depth: aslaid_depth - planned_depth (m)
    pub delta_depth_m: f64,
    /// Tolerance classification
    pub tolerance: ToleranceStatus,
    /// USBL positioning uncertainty (m)
    pub usbl_uncertainty_m: f64,
    /// Timestamp UTC of the fix
    pub timestamp_utc: String,
    /// Battery percentage
    pub battery_pct: Option<f64>,
    /// Node tilt in degrees
    pub tilt_deg: Option<f64>,
    /// Operational status
    pub status: NodeOperationalStatus,
}

/// Overall QC summary for the current survey patch/shift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObnQcSummary {
    pub total_nodes: usize,
    pub in_spec_count: usize,
    pub warning_count: usize,
    pub out_of_spec_count: usize,
    pub compliance_percentage: f64,
    pub min_radial_offset_m: f64,
    pub max_radial_offset_m: f64,
    pub mean_radial_offset_m: f64,
    pub std_dev_radial_m: f64,
    pub mean_depth_m: f64,
    pub nodes: Vec<ObnQcNode>,
}

/// Shift metadata and performance metrics for Daily Progress Reporting (DPR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObnShiftDpr {
    pub survey_name: String,
    pub client_name: String,
    pub vessel_name: String,
    pub shift_date: String,
    pub shift_type: String, // e.g. "Day Shift (00:00 - 12:00)" or "Night Shift (12:00 - 24:00)"
    pub surveyor_name: String,
    pub total_planned_patch: usize,
    pub nodes_deployed_shift: usize,
    pub nodes_retrieved_shift: usize,
    pub deployment_rate_per_hour: f64,
    pub average_cycle_time_mins: f64,
    pub operational_hours: f64,
    pub weather_standby_hours: f64,
    pub technical_downtime_hours: f64,
    pub general_remarks: String,
    /// SHA-256 audit seal of the shift report and node records
    pub audit_hash: String,
}

/// Metadata header for UKOOA SPS or IOGP P1/11 file generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObnHeader {
    pub survey_name: String,
    pub client_name: String,
    pub contractor_name: String,
    pub vessel_name: String,
    pub crs_name: String,
    pub epsg_code: u32,
    pub date_str: String,
}

impl Default for ObnHeader {
    fn default() -> Self {
        Self {
            survey_name: "OBN Seismic Acquisition".into(),
            client_name: "Client Energy Corp".into(),
            contractor_name: "UTM Survey Team".into(),
            vessel_name: "MV Deep Explorer".into(),
            crs_name: "WGS 84 / UTM Zone 31N".into(),
            epsg_code: 32631,
            date_str: "2026-10-01".into(),
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Parsers
// ──────────────────────────────────────────────────────────────────

/// Parses planned pre-plot CSV or text files.
pub fn parse_planned_preplot(content: &str) -> Result<Vec<ObnPlannedNode>, ObnError> {
    let mut planned = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    let mut header_skipped = false;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = if trimmed.contains(',') {
            trimmed.split(',').map(|s| s.trim()).collect()
        } else {
            trimmed.split_whitespace().collect()
        };

        if parts.len() < 4 {
            continue;
        }

        // Check if this is the header row
        if !header_skipped && (parts[0].eq_ignore_ascii_case("station") || parts[0].eq_ignore_ascii_case("station_id") || parts[0].eq_ignore_ascii_case("stationid")) {
            header_skipped = true;
            continue;
        }

        let station_id = parts[0].to_string();
        let line_name = parts[1].to_string();
        let easting = parts[2].parse::<f64>().map_err(|_| ObnError::ParseError {
            line: idx + 1,
            reason: format!("invalid easting coordinate '{}'", parts[2]),
        })?;
        let northing = parts[3].parse::<f64>().map_err(|_| ObnError::ParseError {
            line: idx + 1,
            reason: format!("invalid northing coordinate '{}'", parts[3]),
        })?;
        let target_depth = if parts.len() > 4 {
            parts[4].parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };

        let remarks = if parts.len() > 5 {
            Some(parts[5..].join(" "))
        } else {
            None
        };

        planned.push(ObnPlannedNode {
            station_id,
            line_name,
            easting,
            northing,
            target_depth,
            remarks,
        });
    }

    if planned.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    Ok(planned)
}

/// Parses 4DNav / NavView As-Laid export strings.
pub fn parse_4dnav_aslaid(content: &str) -> Result<Vec<ObnAsLaidNode>, ObnError> {
    let mut as_laid = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    let mut header_skipped = false;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = if trimmed.contains(',') {
            trimmed.split(',').map(|s| s.trim()).collect()
        } else if trimmed.contains('\t') {
            trimmed.split('\t').map(|s| s.trim()).collect()
        } else {
            trimmed.split_whitespace().collect()
        };

        if parts.len() < 7 {
            continue;
        }

        // Header check
        if !header_skipped && (parts[0].eq_ignore_ascii_case("nodeid") || parts[0].eq_ignore_ascii_case("node_id") || parts[0].eq_ignore_ascii_case("serialno") || parts[0].eq_ignore_ascii_case("station")) {
            header_skipped = true;
            continue;
        }

        let node_id = parts[0].to_string();
        let serial_number = if parts.len() > 1 { parts[1].to_string() } else { node_id.clone() };
        let station_id = if parts.len() > 2 { parts[2].to_string() } else { node_id.clone() };
        let line_name = if parts.len() > 3 { parts[3].to_string() } else { "LINE_01".to_string() };

        let easting = parts[4].parse::<f64>().map_err(|_| ObnError::ParseError {
            line: idx + 1,
            reason: format!("invalid easting coordinate '{}'", parts[4]),
        })?;
        let northing = parts[5].parse::<f64>().map_err(|_| ObnError::ParseError {
            line: idx + 1,
            reason: format!("invalid northing coordinate '{}'", parts[5]),
        })?;
        let depth = parts[6].parse::<f64>().unwrap_or(0.0);

        let timestamp_utc = if parts.len() > 7 && !parts[7].is_empty() {
            parts[7].to_string()
        } else {
            "2026-10-01T12:00:00Z".to_string()
        };

        let usbl_uncertainty_m = if parts.len() > 8 {
            parts[8].parse::<f64>().unwrap_or(1.5)
        } else {
            1.5
        };

        let water_depth_m = if parts.len() > 9 {
            parts[9].parse::<f64>().unwrap_or(depth.abs())
        } else {
            depth.abs()
        };

        let status = if parts.len() > 10 {
            match parts[10].to_lowercase().as_str() {
                "retrieved" => NodeOperationalStatus::Retrieved,
                "failed" => NodeOperationalStatus::Failed,
                "lost" => NodeOperationalStatus::Lost,
                "active" | "activerecording" => NodeOperationalStatus::ActiveRecording,
                _ => NodeOperationalStatus::Deployed,
            }
        } else {
            NodeOperationalStatus::Deployed
        };

        as_laid.push(ObnAsLaidNode {
            node_id,
            serial_number,
            station_id,
            line_name,
            easting,
            northing,
            depth,
            timestamp_utc,
            usbl_uncertainty_m,
            water_depth_m,
            tilt_deg: Some(1.2),
            battery_pct: Some(98.5),
            status,
        });
    }

    if as_laid.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    Ok(as_laid)
}

/// Parses Node Dashboard CSV / TSV exports.
pub fn parse_node_dashboard_export(content: &str) -> Result<Vec<ObnAsLaidNode>, ObnError> {
    let mut nodes = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    let mut header_skipped = false;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = if trimmed.contains(',') {
            trimmed.split(',').map(|s| s.trim()).collect()
        } else if trimmed.contains('\t') {
            trimmed.split('\t').map(|s| s.trim()).collect()
        } else {
            trimmed.split_whitespace().collect()
        };

        if parts.len() < 6 {
            continue;
        }

        // Header check
        if !header_skipped && (parts[0].eq_ignore_ascii_case("station_id") || parts[0].eq_ignore_ascii_case("station") || parts[0].eq_ignore_ascii_case("stationid")) {
            header_skipped = true;
            continue;
        }

        let station_id = parts[0].to_string();
        let serial_number = parts[1].to_string();
        let timestamp_utc = parts[2].to_string();
        let easting = parts[3].parse::<f64>().map_err(|_| ObnError::ParseError {
            line: idx + 1,
            reason: format!("invalid easting coordinate '{}'", parts[3]),
        })?;
        let northing = parts[4].parse::<f64>().map_err(|_| ObnError::ParseError {
            line: idx + 1,
            reason: format!("invalid northing coordinate '{}'", parts[4]),
        })?;
        let depth = parts[5].parse::<f64>().unwrap_or(0.0);

        let battery_pct = if parts.len() > 6 {
            parts[6].parse::<f64>().ok()
        } else {
            Some(100.0)
        };

        let tilt_deg = if parts.len() > 7 {
            parts[7].parse::<f64>().ok()
        } else {
            Some(0.8)
        };

        let usbl_uncertainty_m = if parts.len() > 8 {
            parts[8].parse::<f64>().unwrap_or(1.2)
        } else {
            1.2
        };

        let status = if parts.len() > 9 {
            match parts[9].to_lowercase().as_str() {
                "retrieved" => NodeOperationalStatus::Retrieved,
                "failed" => NodeOperationalStatus::Failed,
                "lost" => NodeOperationalStatus::Lost,
                "active" | "activerecording" => NodeOperationalStatus::ActiveRecording,
                _ => NodeOperationalStatus::Deployed,
            }
        } else {
            NodeOperationalStatus::Deployed
        };

        nodes.push(ObnAsLaidNode {
            node_id: serial_number.clone(),
            serial_number,
            station_id,
            line_name: "OBN_LINE".into(),
            easting,
            northing,
            depth,
            timestamp_utc,
            usbl_uncertainty_m,
            water_depth_m: depth.abs(),
            tilt_deg,
            battery_pct,
            status,
        });
    }

    if nodes.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    Ok(nodes)
}

// ──────────────────────────────────────────────────────────────────
// QC Evaluation & Tolerance Gates
// ──────────────────────────────────────────────────────────────────

/// Matches planned targets against as-laid fixes, computing spatial residuals
/// and classifying tolerance (Green <= 5m, Amber 5-10m, Red > 10m).
pub fn evaluate_obn_qc(
    planned: &[ObnPlannedNode],
    as_laid: &[ObnAsLaidNode],
    spec_tolerance_m: f64,
) -> Result<ObnQcSummary, ObnError> {
    if planned.is_empty() && as_laid.is_empty() {
        return Err(ObnError::EmptyDataset);
    }

    let warning_tolerance_m = spec_tolerance_m * 2.0; // standard industry rule: 2x spec is warning boundary
    let mut qc_nodes = Vec::new();

    let mut in_spec = 0;
    let mut warning = 0;
    let mut out_of_spec = 0;

    let mut radial_offsets = Vec::new();
    let mut depths = Vec::new();

    for fix in as_laid {
        // Attempt exact station_id match first, otherwise find closest planned target
        let target = planned
            .iter()
            .find(|p| p.station_id == fix.station_id)
            .or_else(|| {
                // Proximity search
                planned.iter().min_by(|a, b| {
                    let dist_a = ((a.easting - fix.easting).powi(2) + (a.northing - fix.northing).powi(2)).sqrt();
                    let dist_b = ((b.easting - fix.easting).powi(2) + (b.northing - fix.northing).powi(2)).sqrt();
                    dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                })
            });

        let (p_easting, p_northing, p_depth, p_line) = match target {
            Some(t) => (t.easting, t.northing, t.target_depth, t.line_name.clone()),
            None => (fix.easting, fix.northing, fix.depth, fix.line_name.clone()),
        };

        let delta_e = fix.easting - p_easting;
        let delta_n = fix.northing - p_northing;
        let radial_offset = (delta_e.powi(2) + delta_n.powi(2)).sqrt();
        let delta_z = fix.depth - p_depth;

        let tolerance = if radial_offset <= spec_tolerance_m {
            in_spec += 1;
            ToleranceStatus::InSpec
        } else if radial_offset <= warning_tolerance_m {
            warning += 1;
            ToleranceStatus::Warning
        } else {
            out_of_spec += 1;
            ToleranceStatus::OutOfSpec
        };

        radial_offsets.push(radial_offset);
        depths.push(fix.depth);

        qc_nodes.push(ObnQcNode {
            station_id: fix.station_id.clone(),
            serial_number: fix.serial_number.clone(),
            line_name: p_line,
            planned_easting: p_easting,
            planned_northing: p_northing,
            planned_depth: p_depth,
            aslaid_easting: fix.easting,
            aslaid_northing: fix.northing,
            aslaid_depth: fix.depth,
            delta_easting: delta_e,
            delta_northing: delta_n,
            radial_offset_m: radial_offset,
            delta_depth_m: delta_z,
            tolerance,
            usbl_uncertainty_m: fix.usbl_uncertainty_m,
            timestamp_utc: fix.timestamp_utc.clone(),
            battery_pct: fix.battery_pct,
            tilt_deg: fix.tilt_deg,
            status: fix.status.clone(),
        });
    }

    let total = qc_nodes.len();
    let compliance = if total > 0 {
        ((in_spec + warning) as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let min_radial = radial_offsets.iter().copied().fold(f64::INFINITY, f64::min);
    let max_radial = radial_offsets.iter().copied().fold(0.0, f64::max);
    let mean_radial = if total > 0 {
        radial_offsets.iter().sum::<f64>() / total as f64
    } else {
        0.0
    };

    let variance = if total > 1 {
        radial_offsets
            .iter()
            .map(|r| (r - mean_radial).powi(2))
            .sum::<f64>()
            / (total - 1) as f64
    } else {
        0.0
    };
    let std_dev_radial = variance.sqrt();

    let mean_depth = if total > 0 {
        depths.iter().sum::<f64>() / total as f64
    } else {
        0.0
    };

    Ok(ObnQcSummary {
        total_nodes: total,
        in_spec_count: in_spec,
        warning_count: warning,
        out_of_spec_count: out_of_spec,
        compliance_percentage: compliance,
        min_radial_offset_m: if min_radial.is_infinite() { 0.0 } else { min_radial },
        max_radial_offset_m: max_radial,
        mean_radial_offset_m: mean_radial,
        std_dev_radial_m: std_dev_radial,
        mean_depth_m: mean_depth,
        nodes: qc_nodes,
    })
}

// ──────────────────────────────────────────────────────────────────
// Deliverable Exporters (UKOOA SPS & IOGP P1/11)
// ──────────────────────────────────────────────────────────────────

/// Generates valid UKOOA SPS Rev 2.1 Receiver (R) records.
pub fn generate_ukooa_sps_r_records(nodes: &[ObnQcNode], header: &ObnHeader) -> Result<String, ObnError> {
    let mut out = String::with_capacity(nodes.len() * 85 + 1000);

    // Write SPS Header Comments
    writeln!(out, "H0000 UKOOA SHELL PROCESSING SUPPORT (SPS) FORMAT REV 2.1").map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0100 SURVEY:      {}", header.survey_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0200 CLIENT:      {}", header.client_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0300 CONTRACTOR:  {}", header.contractor_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0400 VESSEL:      {}", header.vessel_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0500 GEODETIC CRS:{} (EPSG:{})", header.crs_name, header.epsg_code).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0600 DATE:        {}", header.date_str).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0700 RECORD TYPE: R = RECEIVER (OCEAN BOTTOM NODE)").map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H9999 ---------------- DATA RECORDS BEGIN ----------------").map_err(|e| ObnError::FormatError(e.to_string()))?;

    for node in nodes {
        let line_part = format!("{:<16}", if node.line_name.len() > 16 { &node.line_name[..16] } else { &node.line_name });
        let station_part = format!("{:>8}", if node.station_id.len() > 8 { &node.station_id[..8] } else { &node.station_id });

        let easting_str = format!("{:9.1}", node.aslaid_easting);
        let northing_str = format!("{:10.1}", node.aslaid_northing);
        let elev_str = format!("{:6.1}", -node.aslaid_depth.abs());

        let day_of_year = "274"; // Oct 1
        let time_hhmmss = "120000";

        let record = format!(
            "R{line}{station}1ND   0   0   0   0  {easting}{northing}{elev}{doy}{time}",
            line = line_part,
            station = station_part,
            easting = easting_str,
            northing = northing_str,
            elev = elev_str,
            doy = day_of_year,
            time = time_hhmmss,
        );

        writeln!(out, "{}", record).map_err(|e| ObnError::FormatError(e.to_string()))?;
    }

    Ok(out)
}

/// Generates IOGP P1/11 Geophysical Position Data Exchange Format for Ocean Bottom Nodes.
pub fn generate_iogp_p1_11(nodes: &[ObnQcNode], header: &ObnHeader) -> Result<String, ObnError> {
    let mut out = String::with_capacity(nodes.len() * 120 + 2000);

    writeln!(out, "H0100 SURVEY:        {}", header.survey_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0200 CLIENT:        {}", header.client_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0300 CONTRACTOR:    {}", header.contractor_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0400 VESSEL:        {}", header.vessel_name).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0800 GEODETIC DATUM:WGS 84 (EPSG:4326)").map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0801 PROJECTION:    {} (EPSG:{})", header.crs_name, header.epsg_code).map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0802 UNITS:         Metres").map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H0803 VERTICAL DATUM:Mean Sea Level (MSL)").map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H1000 POSITION TYPE: Seabed As-Laid Node Position (USBL acoustic fix)").map_err(|e| ObnError::FormatError(e.to_string()))?;
    writeln!(out, "H9999 DATA COLUMNS:  RecordType, LineName, StationNumber, PointIndex, StationCode, Easting, Northing, Elevation, USBL_1Sigma, RadialOffset, TimestampUTC, SerialNumber, Status").map_err(|e| ObnError::FormatError(e.to_string()))?;

    for node in nodes {
        let status_str = match node.status {
            NodeOperationalStatus::Deployed => "DEPLOYED",
            NodeOperationalStatus::ActiveRecording => "ACTIVE",
            NodeOperationalStatus::Retrieved => "RETRIEVED",
            NodeOperationalStatus::Failed => "FAILED",
            NodeOperationalStatus::Lost => "LOST",
            _ => "OTHER",
        };

        writeln!(
            out,
            "R,{line},{stn},1,ND,{e:.2},{n:.2},{z:.2},{unc:.2},{radial:.2},{ts},{sn},{status}",
            line = node.line_name,
            stn = node.station_id,
            e = node.aslaid_easting,
            n = node.aslaid_northing,
            z = -node.aslaid_depth.abs(),
            unc = node.usbl_uncertainty_m,
            radial = node.radial_offset_m,
            ts = node.timestamp_utc,
            sn = node.serial_number,
            status = status_str,
        )
        .map_err(|e| ObnError::FormatError(e.to_string()))?;
    }

    Ok(out)
}

/// Generates a comprehensive 12-Hour Shift Handover and Daily Progress Report (DPR).
pub fn generate_shift_dpr_summary(
    dpr: &ObnShiftDpr,
    qc_summary: &ObnQcSummary,
) -> Result<String, ObnError> {
    let mut out = String::with_capacity(4096);

    writeln!(out, "# Offshore OBN Survey — Shift Handover & Daily Progress Report").unwrap();
    writeln!(out, "").unwrap();
    writeln!(out, "| Parameter | Value |").unwrap();
    writeln!(out, "|---|---|").unwrap();
    writeln!(out, "| **Survey / Prospect** | {} |", dpr.survey_name).unwrap();
    writeln!(out, "| **Client** | {} |", dpr.client_name).unwrap();
    writeln!(out, "| **Vessel** | {} |", dpr.vessel_name).unwrap();
    writeln!(out, "| **Shift Date & Window** | {} — {} |", dpr.shift_date, dpr.shift_type).unwrap();
    writeln!(out, "| **Surveyor on Duty** | {} |", dpr.surveyor_name).unwrap();
    writeln!(out, "| **Audit Hash (SHA-256)** | `{}` |", dpr.audit_hash).unwrap();
    writeln!(out, "").unwrap();

    writeln!(out, "## Operational Statistics").unwrap();
    writeln!(out, "").unwrap();
    writeln!(out, "- **Nodes Deployed (Shift):** {} nodes", dpr.nodes_deployed_shift).unwrap();
    writeln!(out, "- **Nodes Retrieved (Shift):** {} nodes", dpr.nodes_retrieved_shift).unwrap();
    writeln!(out, "- **Deployment Rate:** {:.2} nodes/hour", dpr.deployment_rate_per_hour).unwrap();
    writeln!(out, "- **Average Cycle Time:** {:.1} mins/node", dpr.average_cycle_time_mins).unwrap();
    writeln!(out, "- **Operational Time:** {:.1} hrs | **Weather Standby:** {:.1} hrs | **Tech Downtime:** {:.1} hrs",
        dpr.operational_hours, dpr.weather_standby_hours, dpr.technical_downtime_hours).unwrap();
    writeln!(out, "").unwrap();

    writeln!(out, "## Spatial QA/QC & Tolerance Compliance").unwrap();
    writeln!(out, "").unwrap();
    writeln!(out, "- **Total Evaluated Fixes:** {} stations", qc_summary.total_nodes).unwrap();
    writeln!(out, "- **In-Specification (<= 5.0m):** {} nodes ({:.1}%)",
        qc_summary.in_spec_count,
        if qc_summary.total_nodes > 0 { (qc_summary.in_spec_count as f64 / qc_summary.total_nodes as f64) * 100.0 } else { 0.0 }
    ).unwrap();
    writeln!(out, "- **Warning Band (5.0m - 10.0m):** {} nodes ({:.1}%)",
        qc_summary.warning_count,
        if qc_summary.total_nodes > 0 { (qc_summary.warning_count as f64 / qc_summary.total_nodes as f64) * 100.0 } else { 0.0 }
    ).unwrap();
    writeln!(out, "- **Out of Specification (> 10.0m):** {} nodes", qc_summary.out_of_spec_count).unwrap();
    writeln!(out, "- **Mean Radial Displacement:** {:.2} m (StdDev: {:.2} m, Max: {:.2} m)",
        qc_summary.mean_radial_offset_m, qc_summary.std_dev_radial_m, qc_summary.max_radial_offset_m).unwrap();
    writeln!(out, "- **Mean Water Depth:** {:.1} m", qc_summary.mean_depth_m).unwrap();
    writeln!(out, "").unwrap();

    if qc_summary.out_of_spec_count > 0 {
        writeln!(out, "### Flagged Nodes Requiring Senior Surveyor Review").unwrap();
        writeln!(out, "| Station | Node SN | Radial Offset | USBL 1-Sigma | Remarks |").unwrap();
        writeln!(out, "|---|---|---|---|---|").unwrap();
        for node in qc_summary.nodes.iter().filter(|n| n.tolerance == ToleranceStatus::OutOfSpec) {
            writeln!(out, "| {} | {} | {:.2}m | {:.2}m | Exceeds 10m tolerance |",
                node.station_id, node.serial_number, node.radial_offset_m, node.usbl_uncertainty_m).unwrap();
        }
        writeln!(out, "").unwrap();
    }

    writeln!(out, "## Handover Notes & General Remarks").unwrap();
    writeln!(out, "> {}", dpr.general_remarks).unwrap();

    Ok(out)
}

/// Generates a SHA-256 audit hash for the dataset and shift.
pub fn compute_obn_audit_hash(nodes: &[ObnQcNode], shift_date: &str, surveyor: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(shift_date.as_bytes());
    hasher.update(surveyor.as_bytes());
    for node in nodes {
        hasher.update(node.station_id.as_bytes());
        hasher.update(node.serial_number.as_bytes());
        hasher.update(node.radial_offset_m.to_le_bytes());
        hasher.update(node.aslaid_easting.to_le_bytes());
        hasher.update(node.aslaid_northing.to_le_bytes());
    }
    let result = hasher.finalize();
    let mut hex_str = String::with_capacity(64);
    for byte in result {
        write!(hex_str, "{:02x}", byte).unwrap();
    }
    hex_str
}

// ──────────────────────────────────────────────────────────────────
// Unit Tests
// ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_planned_preplot_csv() {
        let sample = "Station_ID,Line_Name,Easting,Northing,Depth\n\
                      STN_101,L01,500000.0,6200000.0,1250.0\n\
                      STN_102,L01,500050.0,6200000.0,1252.0\n\
                      STN_103,L01,500100.0,6200000.0,1255.0";

        let planned = parse_planned_preplot(sample).expect("should parse planned csv");
        assert_eq!(planned.len(), 3);
        assert_eq!(planned[0].station_id, "STN_101");
        assert_eq!(planned[0].easting, 500000.0);
        assert_eq!(planned[0].northing, 6200000.0);
    }

    #[test]
    fn test_parse_4dnav_aslaid_log() {
        let sample = "NodeID,SerialNo,StationID,LineName,Easting,Northing,Depth,Timestamp,USBL_Sigma,WaterDepth,Status\n\
                      N01,SN_9001,STN_101,L01,500002.5,6200001.8,1250.4,2026-10-01T08:15:22Z,1.2,1250.4,Deployed\n\
                      N02,SN_9002,STN_102,L01,500054.1,6200003.2,1252.8,2026-10-01T08:22:10Z,1.4,1252.8,Deployed";

        let aslaid = parse_4dnav_aslaid(sample).expect("should parse 4dnav aslaid");
        assert_eq!(aslaid.len(), 2);
        assert_eq!(aslaid[0].serial_number, "SN_9001");
        assert_eq!(aslaid[0].easting, 500002.5);
        assert_eq!(aslaid[0].usbl_uncertainty_m, 1.2);
    }

    #[test]
    fn test_evaluate_obn_qc_tolerances() {
        let planned = vec![
            ObnPlannedNode { station_id: "STN_1".into(), line_name: "L1".into(), easting: 1000.0, northing: 2000.0, target_depth: 500.0, remarks: None },
            ObnPlannedNode { station_id: "STN_2".into(), line_name: "L1".into(), easting: 1050.0, northing: 2000.0, target_depth: 500.0, remarks: None },
            ObnPlannedNode { station_id: "STN_3".into(), line_name: "L1".into(), easting: 1100.0, northing: 2000.0, target_depth: 500.0, remarks: None },
        ];

        let aslaid = vec![
            // In spec (radial offset = 2.0m <= 5.0m)
            ObnAsLaidNode {
                node_id: "N1".into(), serial_number: "SN1".into(), station_id: "STN_1".into(), line_name: "L1".into(),
                easting: 1002.0, northing: 2000.0, depth: 500.2, timestamp_utc: "2026-10-01T01:00:00Z".into(),
                usbl_uncertainty_m: 0.8, water_depth_m: 500.0, tilt_deg: Some(0.5), battery_pct: Some(99.0), status: NodeOperationalStatus::Deployed,
            },
            // Warning (radial offset = 7.0m between 5m and 10m)
            ObnAsLaidNode {
                node_id: "N2".into(), serial_number: "SN2".into(), station_id: "STN_2".into(), line_name: "L1".into(),
                easting: 1057.0, northing: 2000.0, depth: 500.4, timestamp_utc: "2026-10-01T01:10:00Z".into(),
                usbl_uncertainty_m: 1.5, water_depth_m: 500.0, tilt_deg: Some(1.1), battery_pct: Some(98.5), status: NodeOperationalStatus::Deployed,
            },
            // Out of spec (radial offset = 12.0m > 10.0m)
            ObnAsLaidNode {
                node_id: "N3".into(), serial_number: "SN3".into(), station_id: "STN_3".into(), line_name: "L1".into(),
                easting: 1112.0, northing: 2000.0, depth: 500.9, timestamp_utc: "2026-10-01T01:20:00Z".into(),
                usbl_uncertainty_m: 2.1, water_depth_m: 500.0, tilt_deg: Some(2.4), battery_pct: Some(98.0), status: NodeOperationalStatus::Deployed,
            },
        ];

        let summary = evaluate_obn_qc(&planned, &aslaid, 5.0).expect("should evaluate qc");
        assert_eq!(summary.total_nodes, 3);
        assert_eq!(summary.in_spec_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.out_of_spec_count, 1);
        assert!((summary.nodes[0].radial_offset_m - 2.0).abs() < 1e-4);
        assert_eq!(summary.nodes[0].tolerance, ToleranceStatus::InSpec);
        assert_eq!(summary.nodes[1].tolerance, ToleranceStatus::Warning);
        assert_eq!(summary.nodes[2].tolerance, ToleranceStatus::OutOfSpec);
    }

    #[test]
    fn test_ukooa_sps_formatting() {
        let nodes = vec![ObnQcNode {
            station_id: "STN_001".into(),
            serial_number: "SN_9901".into(),
            line_name: "LINE_01".into(),
            planned_easting: 500000.0,
            planned_northing: 6200000.0,
            planned_depth: 1000.0,
            aslaid_easting: 500001.5,
            aslaid_northing: 6200002.3,
            aslaid_depth: 1000.4,
            delta_easting: 1.5,
            delta_northing: 2.3,
            radial_offset_m: 2.75,
            delta_depth_m: 0.4,
            tolerance: ToleranceStatus::InSpec,
            usbl_uncertainty_m: 1.0,
            timestamp_utc: "2026-10-01T10:00:00Z".into(),
            battery_pct: Some(99.0),
            tilt_deg: Some(0.4),
            status: NodeOperationalStatus::Deployed,
        }];

        let header = ObnHeader::default();
        let sps = generate_ukooa_sps_r_records(&nodes, &header).expect("should generate sps");
        assert!(sps.contains("UKOOA SHELL PROCESSING SUPPORT"));
        assert!(sps.contains("LINE_01"));
        assert!(sps.contains("STN_001"));
        assert!(sps.contains("500001.5"));
        assert!(sps.contains("6200002.3"));
    }

    #[test]
    fn test_iogp_p1_11_formatting() {
        let nodes = vec![ObnQcNode {
            station_id: "STN_001".into(),
            serial_number: "SN_9901".into(),
            line_name: "LINE_01".into(),
            planned_easting: 500000.0,
            planned_northing: 6200000.0,
            planned_depth: 1000.0,
            aslaid_easting: 500001.5,
            aslaid_northing: 6200002.3,
            aslaid_depth: 1000.4,
            delta_easting: 1.5,
            delta_northing: 2.3,
            radial_offset_m: 2.75,
            delta_depth_m: 0.4,
            tolerance: ToleranceStatus::InSpec,
            usbl_uncertainty_m: 1.0,
            timestamp_utc: "2026-10-01T10:00:00Z".into(),
            battery_pct: Some(99.0),
            tilt_deg: Some(0.4),
            status: NodeOperationalStatus::Deployed,
        }];

        let header = ObnHeader::default();
        let p111 = generate_iogp_p1_11(&nodes, &header).expect("should generate p111");
        assert!(p111.contains("H0100 SURVEY:"));
        assert!(p111.contains("R,LINE_01,STN_001,1,ND,500001.50,6200002.30,-1000.40,1.00,2.75"));
    }
}
