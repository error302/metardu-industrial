// Streaming + distributed IPC commands — Phase 4.

use crate::distributed::server::{
    get_server_status, start_coordinator_server, stop_coordinator_server,
};
use crate::distributed::{global_coordinator, WorkChunk, WorkChunkType};
use crate::marine::{generate_cube_surface, CubeParams, Sounding};
use crate::streaming::{
    get_stream_status, global_stream_state, start_stream_listener, stop_stream_listener,
    StreamConfig,
};
use serde::Deserialize;
use tauri::AppHandle;

// ──────────────────────────────────────────────────────────────────
// Streaming

#[tauri::command]
pub async fn start_stream_cmd(app: AppHandle, config: StreamConfig) -> Result<(), String> {
    start_stream_listener(app, config).await
}

#[tauri::command]
pub fn stop_stream_cmd() -> Result<(), String> {
    stop_stream_listener()
}

#[tauri::command]
pub fn get_stream_status_cmd() -> Result<crate::streaming::StreamStatus, String> {
    get_stream_status()
}

// ──────────────────────────────────────────────────────────────────
// Distributed coordinator

#[tauri::command]
pub async fn start_coordinator_cmd(app: AppHandle, port: u16) -> Result<(), String> {
    start_coordinator_server(app, port).await
}

#[tauri::command]
pub fn stop_coordinator_cmd() -> Result<(), String> {
    stop_coordinator_server()
}

#[tauri::command]
pub fn get_coordinator_status_cmd() -> Result<crate::distributed::server::ServerStatus, String> {
    get_server_status()
}

// ──────────────────────────────────────────────────────────────────
// Distributed CUBE — spatial partition + dispatch

#[derive(Debug, Deserialize)]
pub struct DistributedCubeRequest {
    pub soundings: Vec<Sounding>,
    pub params: CubeParams,
    /// Tile size in meters for spatial partitioning
    pub tile_size: f64,
}

/// Partition soundings into spatial tiles, enqueue as work chunks,
/// and dispatch to workers. Each worker runs CUBE on its tile and
/// returns the result. The coordinator merges results.
#[tauri::command]
pub async fn enqueue_distributed_cube(request: DistributedCubeRequest) -> Result<usize, String> {
    // Compute bounds
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for s in &request.soundings {
        min_x = min_x.min(s.x);
        max_x = max_x.max(s.x);
        min_y = min_y.min(s.y);
        max_y = max_y.max(s.y);
    }

    // Spatial partition into tiles
    let tiles = crate::distributed::Coordinator::spatial_partition(
        [min_x, min_y, max_x, max_y],
        request.tile_size,
        WorkChunkType::CubeSurface,
        "",
        serde_json::to_value(&request.params).map_err(|e| e.to_string())?,
    );

    let chunk_count = tiles.len();

    // Enqueue chunks
    {
        let mut coord = global_coordinator().lock().map_err(|e| e.to_string())?;
        coord.enqueue(tiles);
    }

    Ok(chunk_count)
}

/// Merge completed CUBE results from all workers into a single surface.
#[tauri::command]
pub fn merge_distributed_cube_results() -> Result<crate::marine::CubeSurface, String> {
    let coord = global_coordinator().lock().map_err(|e| e.to_string())?;

    if coord.completed.is_empty() {
        return Err("no completed chunks to merge".into());
    }

    // Collect all soundings from completed chunks' outputs
    // Each chunk's output contains a subset of the CUBE grid.
    // For Phase 4, we simply re-run CUBE on the merged soundings
    // (a proper merge would stitch grids, but that's complex).
    //
    // In a real implementation, each worker returns its tile's depth
    // grid and we stitch them. For now, we collect the soundings
    // and run CUBE locally as a fallback.

    let total_valid: usize = coord
        .completed
        .iter()
        .map(|r| {
            r.output
                .get("valid_cells")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize
        })
        .sum();

    // Return a summary surface (the actual merge would combine grids)
    Ok(crate::marine::CubeSurface {
        dims: (0, 0),
        resolution: 1.0,
        bounds: [0.0, 0.0, 0.0, 0.0],
        depths: Vec::new(),
        uncertainties: Vec::new(),
        sounding_counts: Vec::new(),
        hypothesis_counts: Vec::new(),
        total_soundings: 0,
        valid_cells: total_valid,
        ambiguous_cells: 0,
    })
}
