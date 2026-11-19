// Marine survey module — re-exported from the shared metardu-core crate.
//
// This module provides CUBE surface generation, TPU calculation,
// S-44 compliance checking, S-57 export, and SVP parsing.
// The source files live in crates/metardu-core/src/marine/ and are
// the single source of truth shared between the desktop app and worker.

pub mod cube;
pub mod obn;
pub mod patch_test;
pub mod reservoir;
pub mod s44;
pub mod s57;
pub mod segy;
pub mod sounding_suppression;
pub mod subsea_pipeline;
pub mod svp;
pub mod tpu;

pub use cube::{
    generate_surface as generate_cube_surface, CubeError, CubeParams, CubeSurface, Sounding,
};
pub use obn::{
    compute_obn_audit_hash, evaluate_obn_qc, generate_iogp_p1_11, generate_shift_dpr_summary,
    generate_ukooa_sps_r_records, parse_4dnav_aslaid, parse_node_dashboard_export,
    parse_planned_preplot, NodeOperationalStatus, ObnAsLaidNode, ObnError, ObnHeader,
    ObnPlannedNode, ObnQcNode, ObnQcSummary, ObnShiftDpr, ToleranceStatus,
};
pub use patch_test::{
    solve_mbes_patch_test, PatchLineType, PatchSounding, PatchTestLine, PatchTestResults,
};
pub use s44::{
    check_compliance as check_s44_compliance, S44CheckInput, S44ComplianceResult, S44Error,
    S44Failure, S44Order, S44Status,
};
pub use s57::{write_s57, S57Attribute, S57Error, S57Feature, S57Geometry, S57ObjectClass};
pub use segy::{read_segy_file, SegyDataset, SegyError, SegyReelHeader, SegyTrace, SegyTraceHeader};
pub use sounding_suppression::{
    select_shoal_biased_soundings, ChartSoundingSelectionParams, SelectedChartSoundings,
    SoundingPoint,
};
pub use svp::{
    interpolate_speed, parse_svp, ray_trace_beam, RayTraceResult, SvpError, SvpPoint, SvpProfile,
};
pub use tpu::{
    compute_tpu, SoundingTpuInput, TpuComponents, TpuContributions, TpuError, TpuResult,
};
pub use subsea_pipeline::{
    detect_freespans, evaluate_pipeline_qc, generate_event_listing_csv, generate_freespan_listing_csv,
    generate_longitudinal_profile_csv, generate_pipeline_dxf, generate_pipeline_shift_dpr,
    generate_sample_pipeline_project, parse_delphins_trajectory, parse_navimodel_profile,
    parse_pipeline_events, parse_route_centerline, DelphInsFix, Freespan, FreespanCriticality,
    PipeSurveyPoint, PipelineEvent, PipelineEventType, PipelineQcSummary, PipelineReportsBundle,
    PipelineRoutePoint, PipelineShiftDpr,
};


