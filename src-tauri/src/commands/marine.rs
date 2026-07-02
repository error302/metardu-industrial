// Marine IPC commands — Phase 2 Marine MVP.
//
// Exposes CUBE surface generation, TPU, and S-44 compliance to the frontend.

use crate::marine::{
    check_s44_compliance, compute_tpu, generate_cube_surface, write_s57, CubeParams, S44CheckInput,
    S44Order, S57Feature, Sounding, SoundingTpuInput,
};
use serde::Deserialize;

/// Generate a CUBE surface from a batch of soundings.
#[tauri::command]
pub async fn generate_cube_surface_cmd(
    soundings: Vec<Sounding>,
    params: CubeParams,
) -> Result<crate::marine::CubeSurface, String> {
    generate_cube_surface(&soundings, &params).map_err(|e| e.to_string())
}

/// Compute TPU for a batch of soundings.
#[tauri::command]
pub async fn compute_tpu_batch(
    soundings: Vec<SoundingTpuInput>,
) -> Result<Vec<crate::marine::TpuResult>, String> {
    soundings
        .iter()
        .map(compute_tpu)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Check S-44 compliance for a batch of soundings.
#[derive(Debug, Deserialize)]
pub struct S44CheckRequest {
    pub soundings: Vec<S44CheckInput>,
    #[serde(rename = "targetOrder")]
    pub target_order: S44Order,
}

#[tauri::command]
pub async fn check_s44_compliance_cmd(
    request: S44CheckRequest,
) -> Result<crate::marine::S44ComplianceResult, String> {
    check_s44_compliance(&request.soundings, request.target_order).map_err(|e| e.to_string())
}

/// Export features to an S-57 .000 file.
#[tauri::command]
pub fn export_s57(features: Vec<S57Feature>, path: String) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(&path);
    write_s57(&path_buf, &features).map_err(|e| e.to_string())
}
