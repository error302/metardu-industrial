// Marine survey module — CUBE surface generation, TPU, S-44 compliance,
// S-57 export.
//
// Per ARCHITECTURE.md §5 — Phase 2 Marine MVP scope:
//   - cube: CUBE surface generation (NOAA public-domain algorithm)
//   - tpu: Total Propagated Uncertainty calculation
//   - s44: IHO S-44 (6th edition) compliance checking
//   - s57: S-57 ENC export (ISO 8211 binary writer)

pub mod cube;
pub mod s44;
pub mod s57;
pub mod tpu;

pub use cube::{generate_surface as generate_cube_surface, CubeParams, CubeSurface, Sounding};
pub use s44::{
    check_compliance as check_s44_compliance, S44CheckInput, S44ComplianceResult, S44Order,
};
pub use s57::{write_s57, S57Feature};
pub use tpu::{compute_tpu, SoundingTpuInput, TpuResult};
