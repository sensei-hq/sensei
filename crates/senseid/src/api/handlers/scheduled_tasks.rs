//! `GET /api/tasks/scheduled` — background-task visibility (#96).
//!
//! Answers "what background workers exist and when did each last run?" — the
//! question Jerry hit when `get_ftr_daily` came back empty ("is the analyzer
//! even running?"). First version: a static registry of the schedulers the
//! daemon spawns at startup (`api/server.rs`), each surfaced with its
//! `last_run_at` read from the `sensei.config` watermark it already persists
//! (all written as epoch-millis, so parsing is uniform).
//!
//! Health fields (`last_ok`/`last_error`/`avg_ms`/`next_run_at`) are `null` for
//! now — the schedulers don't record run outcomes yet. Adding a shared
//! run-heartbeat is the tracked follow-up; this endpoint degrades honestly
//! rather than fabricating health. A worker with no watermark (log/activity
//! pruners, mcp discovery — they tick on an interval or run at startup, no
//! persisted cursor) reports `last_run_at: null`.

use crate::api::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};

/// One scheduled background worker: display name, what it does, and the
/// `sensei.config` key holding its last-run epoch-millis watermark (if any).
struct ScheduledTask {
    name: &'static str,
    description: &'static str,
    /// Config key with the epoch-millis last-run watermark, or `None` when the
    /// worker persists no cursor (ticks on interval / runs at startup only).
    watermark_key: Option<&'static str>,
}

/// The daemon's background workers (mirrors the `spawn(...)` calls in
/// `api/server.rs`). Registry, not reflection — keep in step when a worker is
/// added or its watermark key changes.
const TASKS: &[ScheduledTask] = &[
    ScheduledTask {
        name: "analyzer",
        description: "Session/log analyzer — findings, recommendations, learned memories",
        watermark_key: Some("analyzer.last_full_refresh"),
    },
    ScheduledTask {
        name: "advance_run",
        description: "Relay run scheduler — auto-resume due pauses + tick active runs",
        watermark_key: None,
    },
    ScheduledTask {
        name: "reconcile",
        description: "Folder/index reconcile — self-healing scan-drift repair",
        watermark_key: Some("reconcile.last_run"),
    },
    ScheduledTask {
        name: "index_audit",
        description: "Index integrity audit (read-only drift check)",
        watermark_key: Some("audit.last_run"),
    },
    ScheduledTask {
        name: "contribute",
        description: "Dōjō upstream contribute cadence",
        watermark_key: Some("collective.last_prepared"),
    },
    ScheduledTask {
        name: "log_pruner",
        description: "Structured-log TTL pruning",
        watermark_key: None,
    },
    ScheduledTask {
        name: "activity_pruner",
        description: "Activity-data GC (after analysis derives insights)",
        watermark_key: None,
    },
    ScheduledTask {
        name: "mcp_discovery",
        description: "MCP tool discovery (startup + on refresh)",
        watermark_key: None,
    },
];

/// Parse an epoch-millis config value into an RFC-3339 timestamp. `None` for a
/// missing or unparseable watermark (never fabricates a time).
fn watermark_to_rfc3339(raw: Option<String>) -> Option<String> {
    let ms: i64 = raw?.trim().parse().ok()?;
    chrono::DateTime::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339())
}

/// GET /api/tasks/scheduled — the background-task registry with last-run times.
pub(crate) async fn scheduled(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut rows = Vec::with_capacity(TASKS.len());
    for t in TASKS {
        // Fail closed: a config-read error must not mask as `last_run_at: null`
        // ("never ran") — that's the exact "is it even running?" ambiguity this
        // endpoint exists to remove. A genuine absent watermark is still None.
        let last_run_at = match t.watermark_key {
            Some(key) => watermark_to_rfc3339(
                state.pg.get_config(key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            ),
            None => None,
        };
        rows.push(serde_json::json!({
            "name": t.name,
            "description": t.description,
            "last_run_at": last_run_at,
            // Not tracked yet (honest degrade — see module docs). Follow-up: a
            // shared run-heartbeat writing outcome + latency per worker.
            "last_ok": serde_json::Value::Null,
            "last_error": serde_json::Value::Null,
            "next_run_at": serde_json::Value::Null,
            "interval_secs": serde_json::Value::Null,
            "avg_ms": serde_json::Value::Null,
        }));
    }
    Ok(Json(serde_json::json!({ "tasks": rows })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermark_parses_epoch_millis_and_rejects_junk() {
        // 1783995091406 → 2026-07-13T… (the analyzer watermark shape).
        let out = watermark_to_rfc3339(Some("1783995091406".into())).expect("valid epoch-ms");
        assert!(out.starts_with("2026-"), "epoch-ms → rfc3339, got {out}");
        assert_eq!(watermark_to_rfc3339(None), None);
        assert_eq!(watermark_to_rfc3339(Some("not-a-number".into())), None);
        assert_eq!(watermark_to_rfc3339(Some("  ".into())), None);
    }

    #[test]
    fn registry_covers_the_key_workers_with_unique_names() {
        let names: Vec<&str> = TASKS.iter().map(|t| t.name).collect();
        for expected in ["analyzer", "reconcile", "index_audit", "contribute"] {
            assert!(names.contains(&expected), "registry missing {expected}");
        }
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "task names must be unique");
    }
}
