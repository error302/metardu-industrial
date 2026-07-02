// Coordinate reprojection — conditional on the 'geo' / 'geo-proj' feature flag.
//
// When the 'proj' crate is available (feature 'geo' or 'geo-proj'), we use
// it for real CRS transformations via PROJ 9.x. The crate requires
// libproj-dev at link time, so it's an opt-in feature.
//
// When neither feature is enabled, the module provides stub implementations
// that return an error — pure-Rust parsers cover Phase 1 needs without
// reprojection (LAS/GeoTIFF files in their native CRS, displayed as-is).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransformResult {
    pub coords: Vec<Coord>,
    pub from_crs: String,
    pub to_crs: String,
    pub method: TransformMethod,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum TransformMethod {
    /// Used the real proj crate (feature 'geo' or 'geo-proj' enabled)
    Proj,
    /// Identity transform — from_crs == to_crs
    Identity,
    /// No-op stub (proj feature not enabled)
    Unavailable,
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum GeodesyError {
    #[error("proj feature not enabled — rebuild with --features geo or geo-proj")]
    ProjNotEnabled,
    #[error("PROJ error: {0}")]
    Proj(String),
    #[error("unsupported CRS code: {0}")]
    UnsupportedCrs(String),
    #[error("identity transform requested but CRS codes differ: {0} vs {1}")]
    MismatchedCrs(String, String),
}

/// Transform a batch of coordinates from one CRS to another.
///
/// When the 'geo' or 'geo-proj' feature is enabled (which pulls in the proj
/// crate), uses real PROJ 9.x transformations. Otherwise returns an error.
pub fn transform_coords(
    coords: &[Coord],
    from_crs: &str,
    to_crs: &str,
) -> Result<TransformResult, GeodesyError> {
    // Identity fast-path
    if from_crs.eq_ignore_ascii_case(to_crs) {
        return Ok(TransformResult {
            coords: coords.to_vec(),
            from_crs: from_crs.into(),
            to_crs: to_crs.into(),
            method: TransformMethod::Identity,
        });
    }

    #[cfg(any(feature = "geo", feature = "geo-proj"))]
    {
        return transform_via_proj(coords, from_crs, to_crs);
    }

    #[cfg(not(any(feature = "geo", feature = "geo-proj")))]
    {
        let _ = coords;
        Err(GeodesyError::ProjNotEnabled)
    }
}

#[cfg(any(feature = "geo", feature = "geo-proj"))]
fn transform_via_proj(
    coords: &[Coord],
    from_crs: &str,
    to_crs: &str,
) -> Result<TransformResult, GeodesyError> {
    use proj::Proj;

    let definition = format!("{from_crs} {to_crs}");
    let proj = Proj::new(&definition).map_err(|e| GeodesyError::Proj(e.to_string()))?;

    let mut transformed = Vec::with_capacity(coords.len());
    for c in coords {
        let (x, y) = proj
            .project((c.x, c.y), false)
            .map_err(|e| GeodesyError::Proj(e.to_string()))?;
        transformed.push(Coord {
            x,
            y,
            z: c.z, // Z transform would need a vertical datum — Phase 2
        });
    }

    Ok(TransformResult {
        coords: transformed,
        from_crs: from_crs.into(),
        to_crs: to_crs.into(),
        method: TransformMethod::Proj,
    })
}

/// Check if real PROJ-backed reprojection is available.
pub fn is_proj_available() -> bool {
    cfg!(any(feature = "geo", feature = "geo-proj"))
}
