// ML IPC commands — Phase 3.
//
// Exposes classify_habitat and analyze_fragmentation to the frontend.

use crate::ml::{analyze_fragmentation, classify_habitat, BackscatterFeatures};
use serde::Deserialize;

#[tauri::command]
pub fn classify_habitat_cmd(
    features: BackscatterFeatures,
) -> Result<crate::ml::HabitatClassificationResult, String> {
    Ok(classify_habitat(&features))
}

#[derive(Debug, Deserialize)]
pub struct FragmentationRequest {
    pub sizes_mm: Vec<f64>,
}

#[tauri::command]
pub fn analyze_fragmentation_cmd(
    request: FragmentationRequest,
) -> Result<crate::ml::FragmentationResult, String> {
    analyze_fragmentation(&request.sizes_mm)
}
