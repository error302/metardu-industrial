// Mining survey module — UAV photogrammetry ingest, point cloud classification,
// and volume calculation.
//
// Per ARCHITECTURE.md §4 — Phase 1 Mining MVP scope:
//   - drone_ingest: parse DJI MMC / FlightHub JSON / generic CSV manifests
//   - csf: pure-Rust Cloth Simulation Filter for ground extraction
//   - volume: pure-Rust fill/cut volume calculation with bench breakdown
//
// The actual SfM (structure-from-motion) processing is delegated to an
// external ODM (OpenDroneMap) Docker container — MetaRDU doesn't bundle
// ODM. The user installs ODM locally and MetaRDU shells out to it via
// the tauri-plugin-shell.

pub mod csf;
pub mod drone_ingest;
pub mod volume;

pub use csf::{classify_ground, CsfParams, CsfResult};
pub use drone_ingest::{parse_manifest, DroneManifest};
pub use volume::{compute_volumes, VolumeResult};
