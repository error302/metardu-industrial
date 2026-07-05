// Sprint 8.5 — High-value bottleneck tools IPC commands.
//
// Density Gates + Tidal Spline + Machine Control Compiler
// These are the tools surveyors actually struggle with.

use crate::dem_render::{self, DemRenderRequest, DemRenderResult};
use crate::marine::density_gates::{self, CoverageReport, DensityGatesRequest};
use crate::marine::tidal_spline::{self, TidalCorrectionRequest, TidalCorrectionResult};
use crate::mining::machine_control::{self, MachineControlRequest, MachineControlResult};

/// Run density gates analysis on a folder of sonar files.
#[tauri::command]
pub async fn run_density_gates_cmd(request: DensityGatesRequest) -> Result<CoverageReport, String> {
    tokio::task::spawn_blocking(move || {
        density_gates::run_density_gates(&request)
            .map_err(|e| ctx_no_input!("running density gates", e))
    })
    .await
    .map_err(|e| format!("run_density_gates_cmd: task join error: {e}"))?
}

/// Run tidal spline correction.
#[tauri::command]
pub async fn run_tidal_correction_cmd(
    request: TidalCorrectionRequest,
) -> Result<TidalCorrectionResult, String> {
    let sonar_label = request.sonar_csv_path.clone();
    tokio::task::spawn_blocking(move || {
        tidal_spline::run_tidal_correction(&request)
            .map_err(|e| ctx!("correcting tides", sonar_label, e))
    })
    .await
    .map_err(|e| format!("run_tidal_correction_cmd: task join error: {e}"))?
}

/// Compile machine control file from DXF/LandXML.
#[tauri::command]
pub async fn compile_machine_control_cmd(
    request: MachineControlRequest,
) -> Result<MachineControlResult, String> {
    let input_label = request.input_path.clone();
    tokio::task::spawn_blocking(move || {
        machine_control::compile_machine_control(&request)
            .map_err(|e| ctx!("compiling machine control", input_label, e))
    })
    .await
    .map_err(|e| format!("compile_machine_control_cmd: task join error: {e}"))?
}

/// Render a GeoTIFF DEM as a hillshaded color-ramp RGBA image.
///
/// Returns packed RGBA bytes + geographic bounds for OpenLayers overlay.
#[tauri::command]
pub async fn render_dem_cmd(request: DemRenderRequest) -> Result<DemRenderResult, String> {
    let path_label = request.path.clone();
    tokio::task::spawn_blocking(move || {
        dem_render::render_dem(&request).map_err(|e| ctx!("rendering DEM", path_label, e))
    })
    .await
    .map_err(|e| format!("render_dem_cmd: task join error: {e}"))?
}
