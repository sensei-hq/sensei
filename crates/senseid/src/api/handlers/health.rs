//! Health endpoints — basic liveness and full component status.

use axum::response::Json;
use serde::Serialize;
use sensei_bootstrap::{self as bootstrap};
use sensei_bootstrap::prereq::{CheckResult, checker::{Checker, PortChecker}};
use std::time::Instant;

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Initialize the start time. Call once at daemon startup.
pub(crate) fn init_uptime() {
    START_TIME.get_or_init(Instant::now);
}

fn uptime_seconds() -> u64 {
    START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0)
}

// ── GET /health ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    status: &'static str,
    name: &'static str,
    version: &'static str,
    uptime_seconds: u64,
    components: ComponentSummary,
}

#[derive(Serialize)]
struct ComponentSummary {
    postgresql: ComponentBrief,
    ollama: ComponentBrief,
    database: ComponentBrief,
    models: Vec<String>,
}

#[derive(Serialize)]
struct ComponentBrief {
    state: String,
    version: Option<String>,
}

impl From<&CheckResult> for ComponentBrief {
    fn from(r: &CheckResult) -> Self {
        Self {
            state: if r.ok { "ready".to_string() } else { "failed".to_string() },
            version: r.version.clone(),
        }
    }
}

impl From<&bootstrap::ComponentStatus> for ComponentBrief {
    fn from(s: &bootstrap::ComponentStatus) -> Self {
        let state = match &s.state {
            bootstrap::ComponentState::Ready => "ready",
            bootstrap::ComponentState::Failed { .. } => "failed",
            bootstrap::ComponentState::Skipped => "skipped",
            _ => "unknown",
        };
        Self { state: state.to_string(), version: s.version.clone() }
    }
}

pub(crate) async fn health() -> Json<HealthResponse> {
    let result = tokio::task::spawn_blocking(|| {
        let pg = PortChecker::new("postgresql", bootstrap::POSTGRES_PORT).check();
        let ollama = PortChecker::new("ollama", bootstrap::OLLAMA_PORT).check();
        let db = bootstrap::database::check();
        let models = bootstrap::models::list();
        (pg, ollama, db, models)
    }).await;

    let (pg, ollama, db, models) = match result {
        Ok(data) => data,
        Err(e) => {
            // Blocking task panicked or was aborted — return a degraded but
            // valid response rather than propagating the panic to the handler.
            tracing::error!("health check task failed: {e}");
            return Json(HealthResponse {
                status: "error",
                name: "senseid",
                version: env!("CARGO_PKG_VERSION"),
                uptime_seconds: uptime_seconds(),
                components: ComponentSummary {
                    postgresql: ComponentBrief { state: "unknown".to_string(), version: None },
                    ollama:     ComponentBrief { state: "unknown".to_string(), version: None },
                    database:   ComponentBrief { state: "unknown".to_string(), version: None },
                    models: vec![],
                },
            });
        }
    };

    Json(HealthResponse {
        status: if pg.ok && db.is_ready() { "healthy" } else { "degraded" },
        name: "senseid",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime_seconds(),
        components: ComponentSummary {
            postgresql: ComponentBrief::from(&pg),
            ollama: ComponentBrief::from(&ollama),
            database: ComponentBrief::from(&db),
            models,
        },
    })
}

// ── Watcher endpoints (unchanged) ───────────────────────────────────────────

pub(crate) async fn watcher_status() -> Json<serde_json::Value> {
    let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);

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
