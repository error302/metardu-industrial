// Distributed processing — Phase 4.
//
// Work-stealing scheduler for heavy processing jobs across multiple
// worker processes or machines. Designed for CUBE on 50M+ soundings,
// point cloud classification on 100M+ points, etc.

#[allow(dead_code)]
pub mod server;
//
// Architecture:
//   - Coordinator: the main MetaRDU process (this code)
//   - Workers: separate processes (metardu-worker binary) that connect
//     via TCP and request work chunks
//   - Work chunks: spatial partitions of the dataset (tiles, strips)
//
// Phase 4 scaffold: defines the coordinator trait, work chunk model,
// and TCP protocol. Actual worker binary is a separate Phase 4 task.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Mutex;

/// A unit of work that can be distributed to a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkChunk {
    pub id: String,
    pub chunk_type: WorkChunkType,
    /// Input data path or inline data reference
    pub input_path: String,
    /// Parameters for this chunk (serialized as JSON)
    pub params: serde_json::Value,
    /// Spatial bounds for this chunk (for spatial partitioning)
    pub bounds: Option<[f64; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkChunkType {
    /// Classify a portion of a point cloud
    ClassifyGround,
    /// Generate CUBE surface for a spatial tile
    CubeSurface,
    /// Compute volumes for a sub-region
    ComputeVolumes,
    /// Run 4D epoch diff for a sub-region
    EpochDiff,
}

/// Result of a completed work chunk.
#[derive(Debug, Clone, Serialize)]
pub struct WorkResult {
    pub chunk_id: String,
    pub status: WorkStatus,
    pub output: serde_json::Value,
    pub elapsed_seconds: f64,
    pub worker_id: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Complete,
    Failed,
}

/// Coordinator state — tracks work queue, active workers, results.
pub struct Coordinator {
    pub pending: VecDeque<WorkChunk>,
    pub in_progress: std::collections::HashMap<String, (WorkChunk, String)>, // chunk_id → (chunk, worker_id)
    pub completed: Vec<WorkResult>,
    pub workers: Vec<WorkerInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerInfo {
    pub id: String,
    pub address: SocketAddr,
    pub status: WorkerConnectionStatus,
    pub chunks_completed: usize,
    pub chunks_failed: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerConnectionStatus {
    Connected,
    Disconnected,
    Busy,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            in_progress: std::collections::HashMap::new(),
            completed: Vec::new(),
            workers: Vec::new(),
        }
    }

    /// Add work chunks to the queue.
    pub fn enqueue(&mut self, chunks: Vec<WorkChunk>) {
        for chunk in chunks {
            self.pending.push_back(chunk);
        }
    }

    /// Get the next work chunk for a worker.
    pub fn dispatch(&mut self, worker_id: &str) -> Option<WorkChunk> {
        let chunk = self.pending.pop_front()?;
        self.in_progress
            .insert(chunk.id.clone(), (chunk.clone(), worker_id.into()));
        Some(chunk)
    }

    /// Accept a result from a worker.
    pub fn complete(&mut self, result: WorkResult) {
        self.in_progress.remove(&result.chunk_id);
        self.completed.push(result);
    }

    /// Check if all work is done.
    pub fn is_done(&self) -> bool {
        self.pending.is_empty() && self.in_progress.is_empty()
    }

    /// Get progress (0.0–1.0).
    pub fn progress(&self) -> f64 {
        let total = self.pending.len() + self.in_progress.len() + self.completed.len();
        if total == 0 {
            return 1.0;
        }
        self.completed.len() as f64 / total as f64
    }

    /// Partition a spatial dataset into tiles for distributed processing.
    pub fn spatial_partition(
        bounds: [f64; 4],
        tile_size: f64,
        chunk_type: WorkChunkType,
        input_path: &str,
        params: serde_json::Value,
    ) -> Vec<WorkChunk> {
        let [min_x, min_y, max_x, max_y] = bounds;
        let mut chunks = Vec::new();
        let mut id_counter = 0;

        let mut y = min_y;
        while y < max_y {
            let mut x = min_x;
            while x < max_x {
                let tile_max_x = (x + tile_size).min(max_x);
                let tile_max_y = (y + tile_size).min(max_y);
                id_counter += 1;
                chunks.push(WorkChunk {
                    id: format!("chunk_{id_counter}"),
                    chunk_type: chunk_type.clone(),
                    input_path: input_path.into(),
                    params: params.clone(),
                    bounds: Some([x, y, tile_max_x, tile_max_y]),
                });
                x += tile_size;
            }
            y += tile_size;
        }

        chunks
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Global coordinator instance.
pub fn global_coordinator() -> &'static Mutex<Coordinator> {
    use std::sync::OnceLock;
    static COORD: OnceLock<Mutex<Coordinator>> = OnceLock::new();
    COORD.get_or_init(|| Mutex::new(Coordinator::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dispatch_complete() {
        let mut coord = Coordinator::new();
        coord.enqueue(vec![
            WorkChunk {
                id: "c1".into(),
                chunk_type: WorkChunkType::ClassifyGround,
                input_path: "/tmp/test.las".into(),
                params: serde_json::json!({}),
                bounds: Some([0.0, 0.0, 10.0, 10.0]),
            },
            WorkChunk {
                id: "c2".into(),
                chunk_type: WorkChunkType::ClassifyGround,
                input_path: "/tmp/test.las".into(),
                params: serde_json::json!({}),
                bounds: Some([10.0, 0.0, 20.0, 10.0]),
            },
        ]);

        assert!(!coord.is_done());

        let chunk = coord.dispatch("worker1").unwrap();
        assert_eq!(chunk.id, "c1");
        assert_eq!(coord.pending.len(), 1);
        assert_eq!(coord.in_progress.len(), 1);

        coord.complete(WorkResult {
            chunk_id: "c1".into(),
            status: WorkStatus::Complete,
            output: serde_json::json!({"ground_count": 100}),
            elapsed_seconds: 1.5,
            worker_id: "worker1".into(),
            error: None,
        });

        assert_eq!(coord.completed.len(), 1);
        assert!(!coord.is_done()); // c2 still pending

        coord.dispatch("worker1");
        coord.complete(WorkResult {
            chunk_id: "c2".into(),
            status: WorkStatus::Complete,
            output: serde_json::json!({"ground_count": 200}),
            elapsed_seconds: 1.2,
            worker_id: "worker1".into(),
            error: None,
        });

        assert!(coord.is_done());
        assert_eq!(coord.completed.len(), 2);
    }

    #[test]
    fn test_progress() {
        let mut coord = Coordinator::new();
        coord.enqueue(vec![WorkChunk {
            id: "c1".into(),
            chunk_type: WorkChunkType::ClassifyGround,
            input_path: "/tmp/test.las".into(),
            params: serde_json::json!({}),
            bounds: None,
        }]);
        assert_eq!(coord.progress(), 0.0);

        coord.dispatch("w1");
        assert_eq!(coord.progress(), 0.0);

        coord.complete(WorkResult {
            chunk_id: "c1".into(),
            status: WorkStatus::Complete,
            output: serde_json::json!({}),
            elapsed_seconds: 1.0,
            worker_id: "w1".into(),
            error: None,
        });
        assert_eq!(coord.progress(), 1.0);
    }

    #[test]
    fn test_spatial_partition() {
        let chunks = Coordinator::spatial_partition(
            [0.0, 0.0, 20.0, 10.0],
            10.0,
            WorkChunkType::ClassifyGround,
            "/tmp/test.las",
            serde_json::json!({}),
        );
        // 20m × 10m area, 10m tiles → 2×1 = 2 chunks
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].bounds, Some([0.0, 0.0, 10.0, 10.0]));
        assert_eq!(chunks[1].bounds, Some([10.0, 0.0, 20.0, 10.0]));
    }
}
