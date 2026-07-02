// Marine survey module — CUBE surface generation, TPU, S-44 compliance.
//
// Per ARCHITECTURE.md §5 — Phase 2 Marine MVP scope:
//   - cube: CUBE surface generation (NOAA public-domain algorithm)
//   - tpu: Total Propagated Uncertainty calculation
//   - s44: IHO S-44 (6th edition) compliance checking

pub mod cube;
pub mod s44;
pub mod tpu;

pub use cube::{
    generate_surface as generate_cube_surface, CubeError, CubeParams, CubeSurface, Sounding,
};
pub use s44::{
    check_compliance as check_s44_compliance, S44CheckInput, S44ComplianceResult, S44Error,
    S44Order, S44Status,
};
pub use tpu::{compute_tpu, SoundingTpuInput, TpuComponents, TpuError, TpuResult};
