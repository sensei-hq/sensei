//! Legacy worker — replaced by tasks/executor.rs
//! Kept as a stub to avoid breaking the module structure.
//! Will be removed in a future cleanup.

/// Legacy spawn — no-op. Real workers are in tasks::executor.
pub async fn spawn_worker(
    _queue: std::sync::Arc<crate::indexer::queue::IndexQueue>,
    _state: crate::api::routes::AppState,
) {
    // No-op — task workers handle everything now
    tracing::info!("[legacy worker] disabled — using task queue workers");
}
