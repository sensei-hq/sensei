//! Health endpoint — thin wrapper over `sensei_bootstrap::check`.
//!
//! The daemon does NOT reshape the response. Every consumer (Tauri app,
//! external clients) gets the same `HealthPayload` shape the bootstrap
//! crate produces.

use axum::{extract::State, response::Json};
use sensei_bootstrap::{self as bootstrap, HealthPayload};
use std::time::Instant;
use crate::api::state::AppState;

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

pub(crate) fn init_uptime() {
    START_TIME.get_or_init(Instant::now);
}

fn uptime_seconds() -> u64 {
    START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0)
}

pub(crate) async fn health() -> Json<HealthPayload> {
    let mut payload = tokio::task::spawn_blocking(|| {
        bootstrap::check(env!("CARGO_PKG_VERSION"))
    }).await.expect("health check task panicked");
    payload.uptime_seconds = uptime_seconds();
    Json(payload)
}

// ── Watcher endpoints (unchanged) ───────────────────────────────────────────

pub(crate) async fn watcher_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(state.task_queue.clone());

    if let Ok(w) = watcher.lock() {
        let status = format!("{:?}", w.status());
        let roots: Vec<serde_json::Value> = w.roots().iter().map(|(path, root)| {
            serde_json::json!({
                "path": path.to_string_lossy(),
                "excluded": root.excluded,
            })
        }).collect();
        Json(serde_json::json!({ "status": status, "roots": roots }))
    } else {
        Json(serde_json::json!({ "status": "error", "message": "lock poisoned" }))
    }
}

pub(crate) async fn watcher_unregister(
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() {
        return Json(serde_json::json!({ "error": "path required" }));
    }

    let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);

    if let Ok(mut w) = watcher.lock() {
        w.unregister(&std::path::PathBuf::from(path));
        Json(serde_json::json!({ "ok": true, "unregistered": path }))
    } else {
        Json(serde_json::json!({ "error": "lock poisoned" }))
    }
}
