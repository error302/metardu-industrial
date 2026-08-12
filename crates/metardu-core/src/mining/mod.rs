// Mining survey module — UAV photogrammetry ingest, point cloud classification,
// volume calculation, EOM audit pipeline, and signed reporting.
//
// Pure Rust, no system dependencies.

pub mod cms;
pub mod csf;
pub mod dem;
pub mod dxf_import;
pub mod eom;
pub mod helmert;
pub mod las;
pub mod license;
pub mod m3c2;
pub mod report;
pub mod report_counter;
pub mod volume;

pub use cms::{analyze_cms_cavity, CmsAnalysisResult, CmsScanPoint, DesignTriMesh};
pub use csf::{classify_ground, CsfError, CsfParams, CsfResult};
pub use dem::{rasterize_ground_to_dem, DemError, DemGrid, DemParams};
pub use dxf_import::{
    import_dxf_surface, rasterize_dxf_to_dem, DesignDem, DxfError, DxfSurface, Triangle,
};
pub use eom::{run_eom_pipeline, EomInput, EomOutput, EomPipelineError, EomProgress};
pub use helmert::{
    solve_helmert_7param, transform_point_helmert, ControlPointPair, Helmert7Parameters,
    HelmertError,
};
pub use las::{read_header, read_points, LasError, LasHeader};
pub use license::{
    check_status, generate_license_keypair, sign_license, verify_license, LicenseClaims,
    LicenseError, LicenseFile, LicenseStatus, MachineFingerprint,
};
pub use m3c2::{
    compute_m3c2_distances, M3c2AnalysisReport, M3c2DistanceResult, M3c2Params, M3c2Point,
};

pub use report::{generate_pdf_report, ReportData, ReportError};
pub use report_counter::{ReportCounter, TRIAL_REPORT_QUOTA};
pub use volume::{compute_volumes, BenchVolume, VolumeError, VolumeResult};

