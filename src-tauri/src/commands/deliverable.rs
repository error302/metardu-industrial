// Deliverable Package IPC commands — Sprint 5 Revenue Feature #7.
//
// Exposes the deliverable package generator to the frontend.

use crate::deliverable::{
    generate_deliverable_package, DeliverablePackageRequest, DeliverablePackageResult,
};

/// Generate a survey deliverable package ZIP archive.
///
/// Bundles source files (GeoTIFF, S-57, S-44 PDF, etc.) with an
/// ISO 19115 metadata XML and a branded manifest HTML into a single ZIP.
#[tauri::command]
pub async fn generate_deliverable_package_cmd(
    request: DeliverablePackageRequest,
) -> Result<DeliverablePackageResult, String> {
    // Run the (potentially slow) packaging in a blocking task so we
    // don't stall the async runtime.
    tokio::task::spawn_blocking(move || {
        generate_deliverable_package(&request).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}
