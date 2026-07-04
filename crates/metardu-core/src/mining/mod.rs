// Mining survey module — UAV photogrammetry ingest, point cloud classification,
// volume calculation, EOM audit pipeline, and signed reporting.
//
// Pure Rust, no system dependencies.

pub mod csf;
pub mod dem;
pub mod dxf_import;
pub mod eom;
pub mod las;
pub mod license;
pub mod report;
pub mod report_counter;
pub mod volume;

pub use csf::{classify_ground, CsfError, CsfParams, CsfResult};
pub use dem::{rasterize_ground_to_dem, DemError, DemGrid, DemParams};
pub use dxf_import::{import_dxf_surface, rasterize_dxf_to_dem, DesignDem, DxfError, DxfSurface, Triangle};
pub use eom::{run_eom_pipeline, EomInput, EomOutput, EomPipelineError, EomProgress};
pub use las::{read_header, read_points, LasError, LasHeader};
pub use license::{
    check_status, generate_license_keypair, sign_license, verify_license, LicenseClaims,
    LicenseError, LicenseFile, LicenseStatus, MachineFingerprint,
};
pub use report::{generate_pdf_report, ReportData, ReportError};
pub use report_counter::{ReportCounter, TRIAL_REPORT_QUOTA};
pub use volume::{compute_volumes, BenchVolume, VolumeError, VolumeResult};
