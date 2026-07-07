// Tidal datum management — convert between tidal datums for marine surveys.
//
// Tidal datums are vertical reference surfaces used for bathymetric data:
//   - MLLW: Mean Lower Low Water (common US chart datum)
//   - MSL: Mean Sea Level
//   - CD: Chart Datum (IHO, varies by region)
//   - LAT: Lowest Astronomical Tide
//   - MLW: Mean Low Water
//   - MHHW: Mean Higher High Water
//
// The conversion uses published offsets between datums. These offsets
// are location-specific (tidal constituents vary by geography) and
// are typically obtained from NOAA tide stations or local port
// authorities.
//
// Usage:
//   1. Surveyor knows their data is in MLLW
//   2. Surveyor needs to convert to CD (for IHO S-44 compliance)
//   3. They enter the MLLW→CD offset (e.g., +0.45m from NOAA)
//   4. The module applies the offset to all depths

use serde::{Deserialize, Serialize};

/// Known tidal datums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TidalDatum {
    /// Mean Lower Low Water — common US chart datum
    Mllw,
    /// Mean Sea Level
    Msl,
    /// Chart Datum — IHO standard, varies by region
    Cd,
    /// Lowest Astronomical Tide — used in UK/Europe
    Lat,
    /// Mean Low Water
    Mlw,
    /// Mean Higher High Water
    Mhhw,
    /// Navd88 — North American Vertical Datum of 1988
    Navd88,
}

impl TidalDatum {
    pub fn label(&self) -> &'static str {
        match self {
            TidalDatum::Mllw => "MLLW (Mean Lower Low Water)",
            TidalDatum::Msl => "MSL (Mean Sea Level)",
            TidalDatum::Cd => "CD (Chart Datum)",
            TidalDatum::Lat => "LAT (Lowest Astronomical Tide)",
            TidalDatum::Mlw => "MLW (Mean Low Water)",
            TidalDatum::Mhhw => "MHHW (Mean Higher High Water)",
            TidalDatum::Navd88 => "NAVD88",
        }
    }
}

/// A tidal datum conversion — applies a vertical offset to depths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalDatumConversion {
    /// Source datum
    pub from: TidalDatum,
    /// Target datum
    pub to: TidalDatum,
    /// Vertical offset in meters (positive = target datum is ABOVE source)
    /// E.g., MLLW→CD offset = +0.45 means CD is 0.45m above MLLW,
    /// so a depth of 10.0m MLLW becomes 10.45m CD.
    pub offset_m: f64,
    /// Source of the offset (NOAA station, tide gauge, etc.)
    pub source: String,
}

/// Apply a tidal datum conversion to a depth value.
///
/// Returns the depth in the target datum.
pub fn convert_depth(depth_m: f64, conversion: &TidalDatumConversion) -> f64 {
    depth_m + conversion.offset_m
}

/// Apply a tidal datum conversion to an array of depths.
pub fn convert_depths(depths: &[f64], conversion: &TidalDatumConversion) -> Vec<f64> {
    depths.iter().map(|d| convert_depth(*d, conversion)).collect()
}

/// Common datum offsets for well-known regions. These are approximate
/// — always verify with local tide gauge data for survey-grade work.
pub fn common_offsets(from: TidalDatum, to: TidalDatum) -> Option<f64> {
    // These are rough approximations for common US coastal conversions.
    // Real survey work requires location-specific offsets from NOAA.
    match (from, to) {
        (TidalDatum::Mllw, TidalDatum::Msl) => Some(-0.15),  // MSL is ~0.15m below MLLW
        (TidalDatum::Msl, TidalDatum::Mllw) => Some(0.15),
        (TidalDatum::Mllw, TidalDatum::Navd88) => Some(-0.15),
        (TidalDatum::Navd88, TidalDatum::Mllw) => Some(0.15),
        (TidalDatum::Msl, TidalDatum::Navd88) => Some(0.0),   // ~equal on US coasts
        (TidalDatum::Navd88, TidalDatum::Msl) => Some(0.0),
        _ => None, // Unknown — user must provide offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_depth() {
        let conv = TidalDatumConversion {
            from: TidalDatum::Mllw,
            to: TidalDatum::Cd,
            offset_m: 0.45,
            source: "NOAA Station 8443970".to_string(),
        };
        assert!((convert_depth(10.0, &conv) - 10.45).abs() < 1e-10);
        assert!((convert_depth(0.0, &conv) - 0.45).abs() < 1e-10);
    }

    #[test]
    fn test_convert_depths() {
        let conv = TidalDatumConversion {
            from: TidalDatum::Mllw,
            to: TidalDatum::Cd,
            offset_m: 0.5,
            source: "test".to_string(),
        };
        let depths = vec![10.0, 20.0, 30.0];
        let result = convert_depths(&depths, &conv);
        assert_eq!(result, vec![10.5, 20.5, 30.5]);
    }

    #[test]
    fn test_common_offsets() {
        assert!(common_offsets(TidalDatum::Mllw, TidalDatum::Msl).is_some());
        assert!(common_offsets(TidalDatum::Mllw, TidalDatum::Cd).is_none());
    }
}
