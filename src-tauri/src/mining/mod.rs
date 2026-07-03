// Mining survey module — UAV photogrammetry ingest, point cloud classification,
// volume calculation, and 4D monitoring.
//
// Per ARCHITECTURE.md §4 — Phase 1-3 Mining scope:
//   - drone_ingest: parse DJI MMC / FlightHub JSON / generic CSV manifests
//   - csf: pure-Rust Cloth Simulation Filter for ground extraction
//   - volume: pure-Rust fill/cut volume calculation with bench breakdown
//   - monitoring_4d: multi-temporal surface differencing for pit progression
//
// The actual SfM (structure-from-motion) processing is delegated to an
// external ODM (OpenDroneMap) Docker container — MetaRDU doesn't bundle
// ODM. The user installs ODM locally and MetaRDU shells out to it via
// the tauri-plugin-shell.

#[allow(dead_code)]
pub mod csf;
#[allow(dead_code)]
pub mod drone_ingest;
#[allow(dead_code)]
pub mod highwall;
#[allow(dead_code)]
pub mod monitoring_4d;
#[allow(dead_code)]
pub mod volume;

pub use csf::{classify_ground, CsfParams, CsfResult};
pub use drone_ingest::{parse_manifest, DroneManifest};
pub use highwall::{analyze_highwall, HighwallThresholds};
pub use monitoring_4d::{compute_epoch_diff, compute_progression, Monitoring4DParams};
pub use volume::{compute_volumes, VolumeResult};
