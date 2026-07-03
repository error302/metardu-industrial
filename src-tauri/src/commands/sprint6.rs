// Sprint 6 — SSS waterfall viewer + 3D slice editor IPC commands.

use crate::formats::{read_xtf_pings, SssData};
use crate::slice_editor::{slice_by_polygon, Point2D, Point3D, RejectMask, SliceResult};
use serde::Deserialize;

// ──────────────────────────────────────────────────────────────────
// SSS Waterfall Viewer (Priority #8)

#[derive(Debug, Deserialize)]
pub struct ReadSssRequest {
    /// Path to the XTF file
    pub path: String,
    /// Maximum pings to load (0 = all)
    #[serde(default)]
    pub max_pings: Option<usize>,
}

/// Read XTF pings for the waterfall viewer.
///
/// The frontend uses the returned ping samples to render a Canvas2D
/// scrolling waterfall (X = across-track samples, Y = ping index,
/// pixel intensity = backscatter amplitude).
#[tauri::command]
pub async fn read_sss_pings_cmd(
    request: ReadSssRequest,
) -> Result<SssData, String> {
    let path = std::path::PathBuf::from(&request.path);
    let max_pings = request.max_pings.unwrap_or(0);
    // XTF parsing is potentially slow (multi-GB files) — run in blocking task
    tokio::task::spawn_blocking(move || {
        read_xtf_pings(&path, max_pings).map_err(|e| ctx!("reading SSS XTF pings", request.path, e))
    })
    .await
    .map_err(|e| format!("read_sss_pings_cmd: task join error: {e}"))?
}

#[derive(Debug, Deserialize)]
pub struct TargetHeightRequest {
    /// Fish altitude above seabed (meters)
    pub fish_altitude_m: f64,
    /// Slant range from fish to target (meters)
    pub slant_range_to_target_m: f64,
    /// Shadow length on the waterfall (meters)
    pub shadow_length_m: f64,
}

/// Compute target height from shadow length using similar-triangles.
///
/// Used when the surveyor clicks target + shadow on the waterfall and
/// the system measures the shadow length in meters (from across-track
/// distance = sample_index × sound_speed × sample_interval / 2).
#[tauri::command]
pub fn compute_target_height_cmd(
    request: TargetHeightRequest,
) -> f64 {
    crate::formats::compute_target_height_from_shadow(
        request.fish_altitude_m,
        request.slant_range_to_target_m,
        request.shadow_length_m,
    )
}

// ──────────────────────────────────────────────────────────────────
// 3D Slice Editor (Priority #9)

/// Slice a LAS point cloud by a 2D polygon (projected coords).
///
/// Returns the indices + point coordinates that fall inside the polygon.
/// The frontend renders these in a 3D Deck.gl view for the reject-brush
/// workflow.
#[tauri::command]
pub async fn slice_by_polygon_cmd(
    request: crate::slice_editor::SliceRequest,
) -> Result<SliceResult, String> {
    // LAS reading is potentially slow — run in blocking task
    let path_label = request.path.clone();
    tokio::task::spawn_blocking(move || {
        slice_by_polygon(&request).map_err(|e| ctx!("slicing LAS by polygon", path_label, e))
    })
    .await
    .map_err(|e| format!("slice_by_polygon_cmd: task join error: {e}"))?
}

#[derive(Debug, Deserialize)]
pub struct BrushRejectRequest {
    /// The slice points (already loaded from slice_by_polygon_cmd)
    pub points: Vec<Point3D>,
    /// Brush center X (projected, meters)
    pub center_x: f64,
    /// Brush center Y
    pub center_y: f64,
    /// Brush center Z
    pub center_z: f64,
    /// Brush radius (meters)
    pub radius_m: f64,
    /// Current reject mask (modified in-place by this operation)
    pub mask: RejectMask,
    /// If true, RESTORE points in brush (un-reject). If false, REJECT them.
    pub restore: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct BrushResult {
    /// Updated mask (clone with new state)
    pub mask: RejectMask,
    /// Count of points toggled in this brush stroke
    pub toggled_count: u32,
    /// Total rejected points after this op
    pub total_rejected: u32,
}

/// Apply a brush stroke (reject or restore) to the slice.
///
/// The mask is returned by-value so the frontend can manage it statefully
/// — the React component stores the latest mask in useState and passes
/// it back on the next brush stroke.
#[tauri::command]
pub fn brush_reject_cmd(request: BrushRejectRequest) -> BrushResult {
    let mut mask = request.mask;
    let toggled = if request.restore {
        mask.brush_restore(
            &request.points,
            request.center_x,
            request.center_y,
            request.center_z,
            request.radius_m,
        )
    } else {
        mask.brush_reject(
            &request.points,
            request.center_x,
            request.center_y,
            request.center_z,
            request.radius_m,
        )
    };
    let total_rejected = mask.rejected_count();
    BrushResult {
        mask,
        toggled_count: toggled,
        total_rejected,
    }
}

#[derive(Debug, Deserialize)]
pub struct UndoRequest {
    pub mask: RejectMask,
}

/// Undo the most recent brush operation.
///
/// Returns the updated mask + count of points toggled back.
/// Returns count=0 if the undo stack is empty.
#[tauri::command]
pub fn undo_brush_cmd(request: UndoRequest) -> BrushResult {
    let mut mask = request.mask;
    let toggled = mask.undo().unwrap_or(0);
    let total_rejected = mask.rejected_count();
    BrushResult {
        mask,
        toggled_count: toggled,
        total_rejected,
    }
}

/// Get the indices of accepted (non-rejected) points from a mask.
///
/// Used by the frontend when re-running CUBE on the cleaned cloud —
/// only the accepted points are passed to the CUBE surface generator.
#[tauri::command]
pub fn accepted_indices_cmd(
    mask: RejectMask,
    total: u32,
) -> Vec<u32> {
    mask.accepted_indices(total)
}

/// Standalone geometry helper — exposed for the frontend to test
/// whether a drawn polygon contains a point without invoking the
/// full slice pipeline.
#[tauri::command]
pub fn point_in_polygon_cmd(
    point: Point2D,
    polygon: Vec<Point2D>,
) -> bool {
    crate::slice_editor::point_in_polygon_2d_test(point, &polygon)
}
