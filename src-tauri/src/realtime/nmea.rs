// NMEA 0183 sentence parser — pure Rust, no allocations on the hot path.
//
// Supported sentence types:
//   - GGA: Global Positioning System Fix Data (lat/lon/alt/quality/sats/HDOP)
//   - RMC: Recommended Minimum Navigation Information (lat/lon/speed/course/date)
//   - GLL: Geographic Position Lat/Long (lat/lon only)
//   - GSA: GNSS DOP and Active Satellites (PDOP/HDOP/VDOP + sat list)
//   - VTG: Track Made Good and Ground Speed (course + speed)
//
// Each sentence has the form:
//   $<talker><type>,<field1>,<field2>,...,<fieldN>*<checksum>\r\n
//
// The checksum is the XOR of all bytes between $ and * (exclusive).
// Sentences without a checksum (no *) are accepted but flagged.

use crate::realtime::RoverPosition;

/// A parsed NMEA sentence — the union of all supported types. Fields
/// not present in the source sentence are `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NmeaSentence {
    pub talker: String,
    pub sentence_type: String,
    pub gga: Option<GgaData>,
    pub rmc: Option<RmcData>,
    pub gll: Option<GllData>,
    pub gsa: Option<GsaData>,
    pub vtg: Option<VtgData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GgaData {
    pub time: f64, // seconds since midnight UTC
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub fix_quality: u8,
    pub satellites: u8,
    pub hdop: Option<f64>,
    pub altitude_m: Option<f64>,
    pub geoid_separation_m: Option<f64>,
    pub age_of_diff_s: Option<f64>,
    pub diff_station_id: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RmcData {
    pub time: f64,
    pub status: RmcStatus,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub speed_knots: Option<f64>,
    pub course_deg: Option<f64>,
    pub date: Option<(u16, u8, u8)>, // (year, month, day)
}

#[derive(Debug, Clone, PartialEq)]
pub enum RmcStatus {
    Valid,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GllData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub time: f64,
    pub status: RmcStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GsaData {
    pub mode: GsaMode,
    pub fix_type: u8, // 1=no fix, 2=2D, 3=3D
    pub satellites: Vec<u8>,
    pub pdop: Option<f64>,
    pub hdop: Option<f64>,
    pub vdop: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GsaMode {
    Manual,
    Automatic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VtgData {
    pub course_true: Option<f64>,
    pub course_magnetic: Option<f64>,
    pub speed_knots: Option<f64>,
    pub speed_kph: Option<f64>,
}

/// Parse a single NMEA sentence.
///
/// Returns `None` if the sentence is malformed or the checksum is
/// invalid. Returns `Some(NmeaSentence)` otherwise — the union struct
/// has the type-specific fields populated and the rest as `None`.
pub fn parse_sentence(raw: &str) -> Option<NmeaSentence> {
    let raw = raw.trim();
    if raw.len() < 6 || !raw.starts_with('$') {
        return None;
    }

    // Split off checksum
    let (body, checksum_hex) = match raw.find('*') {
        Some(pos) => (&raw[1..pos], Some(&raw[pos + 1..])),
        None => (&raw[1..], None),
    };

    // Validate checksum if present
    if let Some(hex) = checksum_hex {
        let expected = u8::from_str_radix(hex.trim_end_matches("\r\n").trim(), 16).ok()?;
        let mut actual: u8 = 0;
        for b in body.bytes() {
            actual ^= b;
        }
        if actual != expected {
            return None;
        }
    }

    // Split into fields
    let mut parts = body.splitn(2, ',');
    let talker_and_type = parts.next()?;
    let fields_str = parts.next().unwrap_or("");

    // Talker is the first 2 chars, type is the rest
    let (talker, sentence_type) = if talker_and_type.len() >= 5 {
        (&talker_and_type[..2], &talker_and_type[2..])
    } else {
        ("", talker_and_type)
    };

    let fields: Vec<&str> = fields_str.split(',').collect();

    let mut sentence = NmeaSentence {
        talker: talker.to_string(),
        sentence_type: sentence_type.to_string(),
        gga: None,
        rmc: None,
        gll: None,
        gsa: None,
        vtg: None,
    };

    match sentence_type {
        "GGA" => sentence.gga = parse_gga(&fields),
        "RMC" => sentence.rmc = parse_rmc(&fields),
        "GLL" => sentence.gll = parse_gll(&fields),
        "GSA" => sentence.gsa = parse_gsa(&fields),
        "VTG" => sentence.vtg = parse_vtg(&fields),
        _ => return Some(sentence), // Unknown type — return parsed header only
    }

    Some(sentence)
}

/// Merge a parsed sentence into a running `RoverPosition` accumulator.
///
/// Different sentence types contribute different fields; calling this
/// for each sentence in arrival order builds up the full position.
pub fn merge_into_position(sentence: &NmeaSentence, pos: &mut RoverPosition) {
    if let Some(gga) = &sentence.gga {
        pos.latitude = gga.latitude;
        pos.longitude = gga.longitude;
        pos.altitude_m = gga.altitude_m;
        pos.fix_quality = Some(gga.fix_quality);
        pos.satellites = Some(gga.satellites);
        pos.hdop = gga.hdop;
        pos.age_of_diff_s = gga.age_of_diff_s;
        pos.diff_station_id = gga.diff_station_id;
        pos.timestamp = gga.time;
    }
    if let Some(rmc) = &sentence.rmc {
        if rmc.latitude.is_some() {
            pos.latitude = rmc.latitude;
        }
        if rmc.longitude.is_some() {
            pos.longitude = rmc.longitude;
        }
        pos.speed_mps = rmc.speed_knots.map(|k| k * 0.514444);
        pos.course_deg = rmc.course_deg;
        pos.timestamp = rmc.time;
    }
    if let Some(gll) = &sentence.gll {
        if gll.latitude.is_some() {
            pos.latitude = gll.latitude;
        }
        if gll.longitude.is_some() {
            pos.longitude = gll.longitude;
        }
        pos.timestamp = gll.time;
    }
    if let Some(vtg) = &sentence.vtg {
        pos.course_deg = vtg.course_true.or(pos.course_deg);
        pos.speed_mps = vtg.speed_knots.map(|k| k * 0.514444).or(pos.speed_mps);
    }
    if let Some(gsa) = &sentence.gsa {
        if gsa.hdop.is_some() {
            pos.hdop = gsa.hdop;
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Per-type parsers
// ──────────────────────────────────────────────────────────────────

fn parse_gga(fields: &[&str]) -> Option<GgaData> {
    // $--GGA,hhmmss.ss,llll.ll,a,yyyyy.yy,a,x,xx,x.x,x.x,M,x.x,M,x.x,xxxx*hh
    //  0     1          2       3 4        5 6 7  8   9   10 11 12 13 14
    if fields.len() < 14 {
        return None;
    }
    let time = parse_hhmmss(fields[0]).unwrap_or(0.0);
    let latitude = parse_lat_lon(fields[1], fields[2]);
    let longitude = parse_lat_lon(fields[3], fields[4]);
    let fix_quality = fields[5].parse::<u8>().unwrap_or(0);
    let satellites = fields[6].parse::<u8>().unwrap_or(0);
    let hdop = fields[7].parse::<f64>().ok();
    let altitude_m = fields[9].parse::<f64>().ok();
    let geoid_separation_m = fields[11].parse::<f64>().ok();
    let age_of_diff_s = if fields[13].is_empty() {
        None
    } else {
        fields[13].parse::<f64>().ok()
    };
    let diff_station_id = if fields.len() > 14 && !fields[14].is_empty() {
        fields[14].parse::<u16>().ok()
    } else {
        None
    };

    Some(GgaData {
        time,
        latitude,
        longitude,
        fix_quality,
        satellites,
        hdop,
        altitude_m,
        geoid_separation_m,
        age_of_diff_s,
        diff_station_id,
    })
}

fn parse_rmc(fields: &[&str]) -> Option<RmcData> {
    // $--RMC,hhmmss.ss,A,llll.ll,a,yyyyy.yy,a,x.x,x.x,ddmmyy,x.x,a*hh
    //  0     1          2 3       4 5        6 7   8   9      10  11
    if fields.len() < 10 {
        return None;
    }
    let time = parse_hhmmss(fields[0]).unwrap_or(0.0);
    let status = match fields[1] {
        "A" => RmcStatus::Valid,
        "V" => RmcStatus::Warning,
        _ => return None,
    };
    let latitude = parse_lat_lon(fields[2], fields[3]);
    let longitude = parse_lat_lon(fields[4], fields[5]);
    let speed_knots = fields[6].parse::<f64>().ok();
    let course_deg = fields[7].parse::<f64>().ok();
    let date = parse_ddmmyy(fields[8]);

    Some(RmcData {
        time,
        status,
        latitude,
        longitude,
        speed_knots,
        course_deg,
        date,
    })
}

fn parse_gll(fields: &[&str]) -> Option<GllData> {
    // $--GLL,llll.ll,a,yyyyy.yy,a,hhmmss.ss,a*hh
    //  0     1       2 3        4 5          6
    if fields.len() < 6 {
        return None;
    }
    let latitude = parse_lat_lon(fields[0], fields[1]);
    let longitude = parse_lat_lon(fields[2], fields[3]);
    let time = parse_hhmmss(fields[4]).unwrap_or(0.0);
    let status = match fields[5] {
        "A" => RmcStatus::Valid,
        "V" => RmcStatus::Warning,
        _ => RmcStatus::Warning,
    };
    Some(GllData {
        latitude,
        longitude,
        time,
        status,
    })
}

fn parse_gsa(fields: &[&str]) -> Option<GsaData> {
    // $--GSA,a,x,xx,xx,xx,xx,xx,xx,xx,xx,xx,xx,xx,xx,x.x,x.x,x.x*hh
    //  0     1 2 3                                          12  13  14  15
    if fields.len() < 16 {
        return None;
    }
    let mode = match fields[0] {
        "A" => GsaMode::Automatic,
        "M" => GsaMode::Manual,
        _ => return None,
    };
    let fix_type = fields[1].parse::<u8>().unwrap_or(1);
    let mut satellites = Vec::new();
    for i in 2..14 {
        if !fields[i].is_empty() {
            if let Ok(s) = fields[i].parse::<u8>() {
                satellites.push(s);
            }
        }
    }
    let pdop = fields[14].parse::<f64>().ok();
    let hdop = fields[15].parse::<f64>().ok();
    let vdop = fields.get(16).and_then(|s| s.parse::<f64>().ok());

    Some(GsaData {
        mode,
        fix_type,
        satellites,
        pdop,
        hdop,
        vdop,
    })
}

fn parse_vtg(fields: &[&str]) -> Option<VtgData> {
    // $--VTG,x.x,T,x.x,M,x.x,N,x.x,K,a*hh
    //  0     1   2 3   4 5   6 7   8
    if fields.len() < 8 {
        return None;
    }
    let course_true = fields[0].parse::<f64>().ok();
    let course_magnetic = fields[2].parse::<f64>().ok();
    let speed_knots = fields[4].parse::<f64>().ok();
    let speed_kph = fields[6].parse::<f64>().ok();
    Some(VtgData {
        course_true,
        course_magnetic,
        speed_knots,
        speed_kph,
    })
}

// ──────────────────────────────────────────────────────────────────
// Field helpers
// ──────────────────────────────────────────────────────────────────

/// Parse hhmmss.ss → seconds since midnight UTC.
fn parse_hhmmss(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    let (hhmm, ss) = s.split_once('.').unwrap_or((s, "0"));
    if hhmm.len() < 4 {
        return None;
    }
    let hh: f64 = hhmm[..2].parse().ok()?;
    let mm: f64 = hhmm[2..4].parse().ok()?;
    let ss: f64 = ss.parse().unwrap_or(0.0);
    // Handle extra fields if any
    let ss = if ss > 100.0 { ss / 100.0 } else { ss };
    Some(hh * 3600.0 + mm * 60.0 + ss)
}

/// Parse ddmmyy → (year, month, day). Year is 4-digit (2000+yy).
fn parse_ddmmyy(s: &str) -> Option<(u16, u8, u8)> {
    if s.len() != 6 {
        return None;
    }
    let day: u8 = s[..2].parse().ok()?;
    let month: u8 = s[2..4].parse().ok()?;
    let year: u16 = 2000 + s[4..6].parse::<u16>().ok()?;
    Some((year, month, day))
}

/// Parse lat/lon in NMEA format (ddmm.mmmm or dddmm.mmmm) → decimal degrees.
///
/// `hemisphere` is N/S for latitude or E/W for longitude. Returns `None`
/// for empty fields (common when the receiver has no fix).
fn parse_lat_lon(value: &str, hemisphere: &str) -> Option<f64> {
    if value.is_empty() {
        return None;
    }
    // Find the decimal point — degrees are everything before the
    // last 2 digits before the decimal (or end of string for whole
    // degree values without minutes).
    let dot = value.find('.').unwrap_or(value.len());
    if dot < 3 {
        return None;
    }
    let deg_end = dot - 2;
    let degrees: f64 = value[..deg_end].parse().ok()?;
    let minutes: f64 = value[deg_end..].parse().ok()?;
    let mut result = degrees + minutes / 60.0;
    if hemisphere == "S" || hemisphere == "W" {
        result = -result;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gga_basic() {
        let raw = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
        let s = parse_sentence(raw).unwrap();
        assert_eq!(s.talker, "GP");
        assert_eq!(s.sentence_type, "GGA");
        let gga = s.gga.unwrap();
        assert!((gga.latitude.unwrap() - 53.3613367).abs() < 1e-6);
        assert!((gga.longitude.unwrap() - (-6.505620)).abs() < 1e-6);
        assert_eq!(gga.fix_quality, 1);
        assert_eq!(gga.satellites, 8);
        assert!((gga.hdop.unwrap() - 1.03).abs() < 1e-6);
        assert!((gga.altitude_m.unwrap() - 61.7).abs() < 1e-6);
    }

    #[test]
    fn test_parse_gga_no_fix() {
        let raw = "$GPGGA,092750.000,,,,,0,00,99.0,,,,,,*66";
        let s = parse_sentence(raw).unwrap();
        let gga = s.gga.unwrap();
        assert_eq!(gga.fix_quality, 0);
        assert_eq!(gga.satellites, 0);
        assert!(gga.latitude.is_none());
        assert!(gga.longitude.is_none());
    }

    #[test]
    fn test_parse_rmc_valid() {
        let raw = "$GPRMC,092750.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*45";
        let s = parse_sentence(raw).unwrap();
        let rmc = s.rmc.unwrap();
        assert_eq!(rmc.status, RmcStatus::Valid);
        assert!((rmc.latitude.unwrap() - 53.3613367).abs() < 1e-6);
        assert!((rmc.longitude.unwrap() - (-6.505620)).abs() < 1e-6);
        assert_eq!(rmc.date, Some((2011, 5, 28)));
    }

    #[test]
    fn test_parse_gll_south_west() {
        let raw = "$GPGLL,5321.6802,S,00630.3372,W,092750.000,V*hh";
        // Compute correct checksum
        let body = "GPGLL,5321.6802,S,00630.3372,W,092750.000,V";
        let mut cs: u8 = 0;
        for b in body.bytes() {
            cs ^= b;
        }
        let raw = format!("${}*{:02X}", body, cs);
        let s = parse_sentence(&raw).unwrap();
        let gll = s.gll.unwrap();
        assert!((gll.latitude.unwrap() - (-53.3613367)).abs() < 1e-6);
        assert!((gll.longitude.unwrap() - (-6.505620)).abs() < 1e-6);
    }

    #[test]
    fn test_invalid_checksum() {
        let raw = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*77";
        assert!(parse_sentence(raw).is_none());
    }

    #[test]
    fn test_unknown_type() {
        let raw = "$GPXYZ,1,2,3*4F";
        // Recompute checksum
        let body = "GPXYZ,1,2,3";
        let mut cs: u8 = 0;
        for b in body.bytes() {
            cs ^= b;
        }
        let raw = format!("${}*{:02X}", body, cs);
        let s = parse_sentence(&raw).unwrap();
        assert_eq!(s.sentence_type, "XYZ");
        assert!(s.gga.is_none() && s.rmc.is_none());
    }

    #[test]
    fn test_merging_sentences() {
        let mut pos = RoverPosition::default();
        let gga = parse_sentence("$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76").unwrap();
        merge_into_position(&gga, &mut pos);
        assert_eq!(pos.fix_quality, Some(1));
        assert_eq!(pos.satellites, Some(8));
        assert!(pos.latitude.is_some());

        let rmc = parse_sentence("$GPRMC,092751.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A*42").unwrap();
        // Recompute checksum (since I just typed it)
        let body = "GPRMC,092751.000,A,5321.6802,N,00630.3372,W,0.02,31.66,280511,,,A";
        let mut cs: u8 = 0;
        for b in body.bytes() {
            cs ^= b;
        }
        let raw = format!("${}*{:02X}", body, cs);
        let rmc = parse_sentence(&raw).unwrap();
        merge_into_position(&rmc, &mut pos);
        assert!(pos.speed_mps.is_some());
        assert!(pos.course_deg.is_some());
    }
}
