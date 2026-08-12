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
pub mod mining;
pub mod ntrip;
pub mod triage;

// Re-export the key types and functions
pub use marine::cube::{
    generate_surface as generate_cube_surface, CubeError, CubeParams, CubeSurface, Sounding,
};
pub use marine::s44::{
    check_compliance as check_s44_compliance, S44CheckInput, S44ComplianceResult, S44Error,
    S44Failure, S44Order, S44Status,
};
pub use marine::s57::{write_s57, S57Attribute, S57Error, S57Feature, S57Geometry, S57ObjectClass};
pub use marine::segy::{read_segy_file, SegyDataset, SegyError};
pub use marine::patch_test::{
    solve_mbes_patch_test, PatchLineType, PatchSounding, PatchTestLine, PatchTestResults,
};
pub use marine::sounding_suppression::{
    select_shoal_biased_soundings, ChartSoundingSelectionParams, SelectedChartSoundings,
    SoundingPoint,
};
pub use marine::svp::{ray_trace_beam, RayTraceResult, SvpProfile};
pub use marine::tpu::{
    compute_tpu, SoundingTpuInput, TpuComponents, TpuContributions, TpuError, TpuResult,
};
pub use mining::cms::{analyze_cms_cavity, CmsAnalysisResult, CmsScanPoint, DesignTriMesh};
pub use mining::helmert::{
    solve_helmert_7param, transform_point_helmert, ControlPointPair, Helmert7Parameters,
};
pub use mining::m3c2::{
    compute_m3c2_distances, M3c2AnalysisReport, M3c2DistanceResult, M3c2Params, M3c2Point,
};
pub use ntrip::sensor_bus::{
    compute_helmsman_telemetry, parse_nmea_sentence, HelmsmanTelemetry, NmeaPosition,
};
pub use ntrip::usv_gateway::{
    parse_usv_telemetry, summarize_usv_fleet, FleetStatusReport, UsvTelemetryFrame,
};

/// Convenience: run CUBE on a tile of soundings (for the worker binary).
pub fn process_cube_tile(
    soundings: &[Sounding],
    params: &CubeParams,
) -> Result<CubeSurface, CubeError> {
    generate_cube_surface(soundings, params)
}

