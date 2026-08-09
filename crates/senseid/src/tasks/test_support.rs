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

/// Insert one `activity.task_executions` row — the churn source (Phase 5.2). Churn
/// counts `status = 'completed'` `process_file` executions, so `task_kind` is
/// usually `"process_file"` and `status` usually `"completed"` (pass `"failed"` to
/// exercise the retry-de-dup filter). `folder_path` is the git-folder abs path
/// churn is attributed through (`resolve_folder_by_path`, alias-aware), and `path`
/// the file the execution processed (`churn_concentration` keys files on it).
/// `started_at` fixes the day. `task_id` is a throwaway (bigint, no unique
/// constraint) derived from a fresh uuid so parallel seeds don't collide. Shared so
/// 5.3 can reuse it.
pub(crate) async fn seed_task_execution(
    pg: &PgStore,
    task_kind: &str,
    status: &str,
    folder_path: &str,
    path: &str,
    started_at: chrono::DateTime<chrono::Utc>,
) {
    let task_id = uuid::Uuid::new_v4().as_u128() as i64;
    sqlx_core::query::query(
        "INSERT INTO activity.task_executions (task_id, task_kind, folder_path, path, status, started_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(task_id)
    .bind(task_kind)
    .bind(folder_path)
    .bind(path)
    .bind(status)
    .bind(started_at)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// Insert one `inference.detected_patterns` row via the production upsert (DRY —
/// reuses [`PgStore::upsert_pattern`] rather than re-implementing the SQL). A
/// rework signal is `name = "rework: <file>"`, `is_anti = true`, `folder_id` = the
/// file's folder locus (what `rework_density` counts). Returns the pattern id.
/// Shared so the churn (5.2) and knowledge (5.3) group tests reuse it.
pub(crate) async fn seed_detected_pattern(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    folder_id: Option<&uuid::Uuid>,
    name: &str,
    is_anti: bool,
) -> uuid::Uuid {
    pg.upsert_pattern(project_id, folder_id, name, is_anti, None, &serde_json::json!([]))
        .await
        .unwrap()
}

/// Insert one file-kind `sensei.nodes` row — a "project file" for the
/// `rework_density` denominator (# project files = # `kind = 'file'` nodes in the
/// project's folders). `file_path` doubles as the node name and must be unique per
/// folder (governed by `nodes_unique_identity`); `ON CONFLICT DO NOTHING` keeps
/// re-seeds idempotent.
pub(crate) async fn seed_file_node(pg: &PgStore, folder_id: &uuid::Uuid, file_path: &str) {
    sqlx_core::query::query(
        "INSERT INTO sensei.nodes (folder_id, kind, name, file_path) \
         VALUES ($1, 'file'::sensei.node_kind, $2, $2) ON CONFLICT DO NOTHING",
    )
    .bind(folder_id)
    .bind(file_path)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// A 384-dim pgvector literal with every component equal to `val` (a SQL numeric
/// text, e.g. `"0.1"`). Two such vectors are IDENTICAL → cosine similarity 1.0, a
/// guaranteed near-duplicate for the `duplication_ratio` fixture. Emitted as a raw
/// SQL expression because pgvector has no sqlx bind type (inlined by
/// [`seed_symbol_node`]).
pub(crate) fn uniform_embedding_sql(val: &str) -> String {
    format!("(SELECT '['||string_agg('{val}', ',')||']' FROM generate_series(1,384))::vector")
}

/// A 384-dim ONE-HOT pgvector literal: `1` at 1-based position `pos`, `0` elsewhere
/// (`ORDER BY` makes the position deterministic). Two one-hots at DIFFERENT
/// positions are orthogonal (cosine 0), and any one-hot vs a
/// [`uniform_embedding_sql`] vector sits far below 0.92 — so one-hot symbols never
/// form a duplicate pair. The dissimilar counterpart to the uniform-vector duplicates.
pub(crate) fn onehot_embedding_sql(pos: u32) -> String {
    format!(
        "(SELECT '['||string_agg(CASE WHEN g = {pos} THEN '1' ELSE '0' END, ',' ORDER BY g)||']' \
         FROM generate_series(1,384) g)::vector"
    )
}

/// Insert one `sensei.nodes` symbol row for the `duplication_ratio` fixture and
/// return its id. `kind` is a `node_kind` literal (`"function"`/`"method"` are the
/// eligible kinds; anything else is deliberately ineligible). `embedding_sql`, when
/// `Some`, is a raw 384-dim pgvector expression (see [`uniform_embedding_sql`] /
/// [`onehot_embedding_sql`]) inlined into the INSERT; `None` leaves `embedding` NULL
/// (ineligible). `lines` is the `(line_start, line_end)` span — eligibility requires
/// `line_end - line_start >= 3`. Distinct `name`s keep rows apart under
/// `nodes_unique_identity`. Nodes cascade on folder delete, so
/// [`cleanup_metrics_fixture`] clears them.
pub(crate) async fn seed_symbol_node(
    pg: &PgStore,
    folder_id: &uuid::Uuid,
    kind: &str,
    name: &str,
    file_path: &str,
    lines: (i32, i32),
    embedding_sql: Option<&str>,
) -> uuid::Uuid {
    let (line_start, line_end) = lines;
    let embedding = embedding_sql.unwrap_or("NULL");
    let sql = format!(
        "INSERT INTO sensei.nodes (folder_id, kind, name, file_path, line_start, line_end, embedding) \
         VALUES ($1, '{kind}'::sensei.node_kind, $2, $3, $4, $5, {embedding}) RETURNING id"
    );
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(&sql)
        .bind(folder_id)
        .bind(name)
        .bind(file_path)
        .bind(line_start)
        .bind(line_end)
        .fetch_one(pg.pool())
        .await
        .unwrap();
    id
}

/// Insert one `activity.sessions` row carrying a `client_session_id` and return its
/// id — the autonomy (5.4) fixture. `assistant_events` are attributed to a project
/// via `sessions.client_session_id = assistant_events.session_id`, so an event
/// needs a session with this id set to be countable. Distinct from
/// [`seed_metrics_session`], which leaves `client_session_id` NULL (its outcome/ftr
/// path is what `session_outcomes` measures, not the hook-event stream).
pub(crate) async fn seed_metrics_client_session(
    pg: &PgStore,
    fid: &uuid::Uuid,
    pid: &uuid::Uuid,
    client_session_id: &str,
    started_at: chrono::DateTime<chrono::Utc>,
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (folder_id, project_id, client_session_id, started_at) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(fid)
    .bind(pid)
    .bind(client_session_id)
    .bind(started_at)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

/// Insert one `activity.assistant_events` row — the `interruption_rate` source
/// (autonomy, 5.4). `session_id` is the assistant's own session id string (matches a
/// [`seed_metrics_client_session`] `client_session_id`, NOT a DB uuid);
/// `event_type` is the hook name (`"Stop"` / `"UserPromptSubmit"`). `at` sets BOTH
/// the server-side `created_at` (the column the computer windows + day-buckets on)
/// and the client-clock `ts`, so a test can place an event on a chosen day. `family`
/// defaults to `claude`, `payload` to `{}`.
pub(crate) async fn seed_assistant_event(
    pg: &PgStore,
    session_id: &str,
    event_type: &str,
    at: chrono::DateTime<chrono::Utc>,
) {
    sqlx_core::query::query(
        "INSERT INTO activity.assistant_events (session_id, event_type, ts, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(session_id)
    .bind(event_type)
    .bind(at.timestamp_millis())
    .bind(at)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// Insert one `activity.runs` row — the `run_completion` source (autonomy, 5.4).
/// `status` is a `sensei.run_status` literal (`"done"` = reached completion; any
/// other value counts toward "started" but not "done"). `started_at` fixes the day
/// the computer windows + buckets on. Returns the run id.
pub(crate) async fn seed_run(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    status: &str,
    started_at: chrono::DateTime<chrono::Utc>,
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.runs (project_id, status, started_at) \
         VALUES ($1, $2::sensei.run_status, $3) RETURNING id",
    )
    .bind(project_id)
    .bind(status)
    .bind(started_at)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

/// Delete every `activity.assistant_events` row for the given `session_id`s. That
/// table's `session_id` is a plain text client id with NO FK to `sessions`, so it
/// never cascades on a session/folder/project delete and must be cleared explicitly.
/// Idempotent — safe to call at the START of a test (pre-clean against a prior
/// crashed run that leaked rows) AND at the end. A no-op for an empty slice.
pub(crate) async fn purge_assistant_events(pg: &PgStore, session_ids: &[&str]) {
    if session_ids.is_empty() {
        return;
    }
    let ids: Vec<String> = session_ids.iter().map(|s| s.to_string()).collect();
    sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = ANY($1)")
        .bind(&ids)
        .execute(pg.pool())
        .await
        .unwrap();
}

/// Delete every `activity.runs` row for the given `project_id`s (cascading their
/// `run_events`). `runs.project_id` is `ON DELETE SET NULL`, so a project delete
/// ORPHANS its runs rather than removing them — this must therefore run BEFORE
/// [`cleanup_metrics_fixture`] deletes the project (afterwards the runs' project_id
/// is NULL and no longer matches). Idempotent; a no-op for an empty slice.
pub(crate) async fn purge_runs(pg: &PgStore, project_ids: &[&uuid::Uuid]) {
    if project_ids.is_empty() {
        return;
    }
    let ids: Vec<uuid::Uuid> = project_ids.iter().map(|p| **p).collect();
    sqlx_core::query::query("DELETE FROM activity.runs WHERE project_id = ANY($1)")
        .bind(&ids)
        .execute(pg.pool())
        .await
        .unwrap();
}

/// Delete every `activity.task_executions` row under the given `folder_path`s. That
/// table is path-keyed with NO FK to the project/folder, so it never cascades and
/// must be cleared explicitly. Idempotent — safe to call at the START of a test
/// (pre-clean against a prior crashed run that leaked rows) AND at the end. A no-op
/// for an empty slice.
pub(crate) async fn purge_task_executions(pg: &PgStore, folder_paths: &[&str]) {
    if folder_paths.is_empty() {
        return;
    }
    let paths: Vec<String> = folder_paths.iter().map(|s| s.to_string()).collect();
    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = ANY($1)")
        .bind(&paths)
        .execute(pg.pool())
        .await
        .unwrap();
}

/// Remove a metric fixture: the project (cascades its `project_metrics` and
/// `detected_patterns`) and, when given, the folder (cascades its `sessions` →
/// `turns` and its `nodes`). `exec_folder_paths` names the `folder_path`s whose
/// `activity.task_executions` to purge (see [`purge_task_executions`]) — pass `&[]`
/// for fixtures that seed no executions.
pub(crate) async fn cleanup_metrics_fixture(
    pg: &PgStore,
    pid: &uuid::Uuid,
    fid: Option<&uuid::Uuid>,
    exec_folder_paths: &[&str],
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
    purge_task_executions(pg, exec_folder_paths).await;
}
