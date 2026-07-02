// Processing pipelines — subprocess managers for external tools.
//
// Phase 1: ODM (OpenDroneMap) for UAV photogrammetry.
// Future: PDRF filter for marine data, ML model runners, etc.

pub mod odm;

pub use odm::{check_odm, estimate_progress, run_odm, OdmConfig};
