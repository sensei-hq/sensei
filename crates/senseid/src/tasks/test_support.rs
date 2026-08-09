//! Shared test-only helpers for the task engine.
//!
//! DRY: `make_ctx` was copy-pasted verbatim across ~8 handler test modules
//! (a qlty duplication finding). One definition lives here; variants that need
//! the event-broadcast receiver (`scan`) or an `Option` return (`publish_run`,
//! `advance_run`) keep their own local builder.
//!
//! The `seed_metrics_*` / `cleanup_metrics_fixture` helpers are the shared metric
//! fixture the per-group compute-handler tests build on (`session_outcomes` and
//! the churn/duplication/autonomy/knowledge/tool groups that copy its template),
//! so the six groups don't each re-implement project/folder/session/turn seeding.

use std::sync::Arc;

use crate::db::pg_store::PgStore;

/// A [`TaskContext`](crate::tasks::executor::TaskContext) backed by a fresh
/// `TaskQueue`, the test `PgStore`, and a noop gateway — the standard fixture for
/// task-handler unit tests.
pub(crate) async fn make_ctx() -> Arc<crate::tasks::executor::TaskContext> {
    let queue = Arc::new(crate::tasks::queue::TaskQueue::new());
    let gateway = crate::api::gateway_init::init_gateway_test().await;
    let app_state = Arc::new(crate::api::state::SharedState {
        task_queue: queue.clone(),
        pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
        gateway,
        event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
        breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        provisioning: None,
    });
    Arc::new(crate::tasks::executor::TaskContext {
        queue,
        app_state,
        _graph_path: None,
        logger: sensei_logger::Logger::noop(),
    })
}

// ── Metrics compute fixtures ────────────────────────────────────────────────

/// Create a project + a folder wired to it under the fixed `/_test` watch root
/// (the same root-id convention the pg_store tests use). Returns
/// `(project_id, folder_id)`. `uniq` keeps `abs_path` collision-free across tests.
pub(crate) async fn seed_metrics_project_folder(
    pg: &PgStore,
    uniq: &uuid::Uuid,
) -> (uuid::Uuid, uuid::Uuid) {
    let pid = pg
        .create_project(&format!("_test:metrics:{uniq}"), None, None)
        .await
        .unwrap();
    pg.execute_raw(
        "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
         VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
         ON CONFLICT DO NOTHING",
    )
    .await
    .unwrap();
    let name = format!("metrics-{uniq}");
    let abs = format!("/_test/metrics-{uniq}");
    let (fid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) \
         VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2, $3) \
         ON CONFLICT(abs_path) DO UPDATE SET project_id = EXCLUDED.project_id RETURNING id",
    )
    .bind(&name)
    .bind(&abs)
    .bind(pid)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    (pid, fid)
}

/// Insert one `activity.sessions` row and return its id. `outcome`/`ftr` are
/// `Option` so tests can seed an in-flight session (`outcome = NULL`, `ftr = NULL`)
/// alongside measurable ones.
pub(crate) async fn seed_metrics_session(
    pg: &PgStore,
    fid: &uuid::Uuid,
    pid: &uuid::Uuid,
    outcome: Option<&str>,
    ftr: Option<bool>,
    corrections: i32,
    started_at: chrono::DateTime<chrono::Utc>,
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (folder_id, project_id, outcome, ftr, corrections, started_at) \
         VALUES ($1, $2, $3::sensei.session_outcome, $4, $5, $6) RETURNING id",
    )
    .bind(fid)
    .bind(pid)
    .bind(outcome)
    .bind(ftr)
    .bind(corrections)
    .bind(started_at)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

/// Attach one turn (carrying `tool_calls`) to a session — tool-calls live on
/// `activity.turns`, so this is how a session gets a measurable tool-call count.
pub(crate) async fn seed_metrics_turn(
    pg: &PgStore,
    sid: &uuid::Uuid,
    tool_calls: i32,
    started_at: chrono::DateTime<chrono::Utc>,
) {
    sqlx_core::query::query(
        "INSERT INTO activity.turns (session_id, turn_number, started_at, ended_at, tool_calls) \
         VALUES ($1, 1, $2, $2, $3)",
    )
    .bind(sid)
    .bind(started_at)
    .bind(tool_calls)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// Remove a metric fixture: the project (cascades its `project_metrics`) and, when
/// given, the folder (cascades its `sessions` → `turns`).
pub(crate) async fn cleanup_metrics_fixture(
    pg: &PgStore,
    pid: &uuid::Uuid,
    fid: Option<&uuid::Uuid>,
) {
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(pg.pool())
        .await
        .unwrap();
    if let Some(fid) = fid {
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .execute(pg.pool())
            .await
            .unwrap();
    }
}
