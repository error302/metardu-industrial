// metardu-core — shared geospatial processing core.
//
// This crate contains the actual marine and mining processing modules
// used by both the MetaRDU Industrial desktop app and the metardu-worker
// distributed processing binary.
//
// By extracting these into a shared crate, the worker binary gets the
// FULL CUBE algorithm (not a simplified version), the full TPU
// calculation, S-44 compliance checking, and S-57 export — all from
// a single source of truth.

pub mod marine;

// Re-export the key types and functions
pub use marine::cube::{
    generate_surface as generate_cube_surface, CubeError, CubeParams, CubeSurface, Sounding,
};
pub use marine::s44::{
    check_compliance as check_s44_compliance, S44CheckInput, S44ComplianceResult, S44Error,
    S44Failure, S44Order, S44Status,
};
pub use marine::s57::{write_s57, S57Attribute, S57Error, S57Feature, S57Geometry, S57ObjectClass};
pub use marine::tpu::{
    compute_tpu, SoundingTpuInput, TpuComponents, TpuContributions, TpuError, TpuResult,
};

/// Convenience: run CUBE on a tile of soundings (for the worker binary).
pub fn process_cube_tile(
    soundings: &[Sounding],
    params: &CubeParams,
) -> Result<CubeSurface, CubeError> {
    generate_cube_surface(soundings, params)
}
