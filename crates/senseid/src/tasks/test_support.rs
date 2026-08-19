//! Shared test-only helpers for the task engine.
//!
//! DRY: `make_ctx` was copy-pasted verbatim across ~8 handler test modules
//! (a qlty duplication finding). One definition lives here; variants that need
//! the event-broadcast receiver (`scan`) or an `Option` return (`publish_run`,
//! `advance_run`) keep their own local builder.
//!
//! The `seed_metrics_*` / `cleanup_metrics_fixture` helpers are the shared metric
//! fixture the per-group compute-handler tests build on (`session_outcomes` and
//! the churn/quality/autonomy/knowledge/tool groups that copy its template),
//! so the groups don't each re-implement project/folder/session/turn seeding.

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

/// Ensure the fixed `/_test` watch root exists (idempotent) — the root every
/// metric-fixture folder hangs off (the same root-id convention the pg_store
/// tests use). Extracted so the synthetic- and git-path folder seeders share it.
async fn ensure_test_watch_root(pg: &PgStore) {
    pg.execute_raw(
        "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
         VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
         ON CONFLICT DO NOTHING",
    )
    .await
    .unwrap();
}

/// Create a project + a git-kind folder wired to it at an EXPLICIT `abs_path`
/// under the fixed `/_test` watch root. The generalized core of
/// [`seed_metrics_project_folder`] (which passes the synthetic `/_test/metrics-*`
/// path): churn's git tests pass a REAL temp-repo path so `project_root_path`
/// (the git churn source resolves the repo through it) points at an on-disk repo.
/// Also seeds a canonical `sensei.repositories` row (repo_key `test/{uniq}`) and
/// points `folders.repository_id` at it — every metric-fixture folder resolves to a
/// repository, the repo-grain metric key. Returns `(project_id, folder_id)`.
pub(crate) async fn seed_project_folder_at(
    pg: &PgStore,
    uniq: &uuid::Uuid,
    abs_path: &str,
) -> (uuid::Uuid, uuid::Uuid) {
    let pid = pg
        .create_project(&format!("_test:metrics:{uniq}"), None, None)
        .await
        .unwrap();
    ensure_test_watch_root(pg).await;
    let name = format!("metrics-{uniq}");
    let (fid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) \
         VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2, $3) \
         ON CONFLICT(abs_path) DO UPDATE SET project_id = EXCLUDED.project_id RETURNING id",
    )
    .bind(&name)
    .bind(abs_path)
    .bind(pid)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    // Repo-grain: resolve the fixture folder to a canonical repository (the metric
    // grain under `_v2`). Seed a local-only repo keyed on the collision-free
    // `test/{uniq}` and point `folders.repository_id` at it, so compute writes carry a
    // real `repository_id` (I-A) and read-backs pool per repository.
    // `cleanup_metrics_fixture` deletes these rows so the unique `repo_key` can't
    // collide across runs.
    let (rid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, $2) \
         ON CONFLICT(repo_key) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(format!("test/{uniq}"))
    .bind(&name)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $2 WHERE id = $1")
        .bind(fid)
        .bind(rid)
        .execute(pg.pool())
        .await
        .unwrap();
    (pid, fid)
}

/// Create a project + a folder wired to it under the fixed `/_test` watch root
/// (the same root-id convention the pg_store tests use). Returns
/// `(project_id, folder_id)`. `uniq` keeps `abs_path` collision-free across tests.
pub(crate) async fn seed_metrics_project_folder(
    pg: &PgStore,
    uniq: &uuid::Uuid,
) -> (uuid::Uuid, uuid::Uuid) {
    seed_project_folder_at(pg, uniq, &format!("/_test/metrics-{uniq}")).await
}

/// Create a project + a git-kind folder whose `abs_path` is a REAL on-disk git
/// repo (a fresh [`tempfile::TempDir`]), returning
/// `(project_id, folder_id, TempDir)`. The git-sourced churn computer resolves the
/// repo via `project_root_path`, so churn tests need the folder to point at an
/// actual repo. KEEP the returned `TempDir` bound for the test's duration —
/// dropping it removes the repo. The repo carries a local `user.name`/`user.email`
/// so commits succeed with no ambient git identity (CI).
pub(crate) async fn seed_git_project_folder(
    pg: &PgStore,
    uniq: &uuid::Uuid,
) -> (uuid::Uuid, uuid::Uuid, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    git_init_repo(dir.path());
    let root = dir.path().to_string_lossy().to_string();
    let (pid, fid) = seed_project_folder_at(pg, uniq, &root).await;
    (pid, fid, dir)
}

/// Read a folder's resolved `repository_id` — the repo-grain key the fixture seeded
/// onto it. Panics if the folder has none: a fixture folder always carries one after
/// [`seed_project_folder_at`] / [`seed_second_repository`], so a NULL means the
/// fixture is broken and should fail loud rather than silently mis-key a metric row.
/// Tests use it to pass the right `repository_id` to
/// [`PgStore::upsert_project_metric_repo`] when seeding rows to read back under `_v2`.
pub(crate) async fn repository_for_folder(pg: &PgStore, folder_id: &uuid::Uuid) -> uuid::Uuid {
    let (rid,): (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
            .bind(folder_id)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    rid
}

/// Seed a SECOND repository into an existing project: a git-kind checkout folder at a
/// distinct `abs_path` (`/_test/metrics-{uniq}-b`) with its own `sensei.repositories`
/// row (repo_key `test/{uniq}-b`), wired to `project_id`. The multi-repo pooling test
/// uses this so a project spans two repositories and the `project_metric_daily` view
/// pools their repo-grain rows (Σnum/Σden). Returns `(folder_id2, repository_id2)`.
/// [`cleanup_metrics_fixture`] deletes the repo row (it walks every folder in the
/// project); pass `folder_id2` as a fixture `fid` to also clear the folder.
pub(crate) async fn seed_second_repository(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    uniq: &uuid::Uuid,
) -> (uuid::Uuid, uuid::Uuid) {
    ensure_test_watch_root(pg).await;
    let name = format!("metrics-{uniq}-b");
    let abs_path = format!("/_test/metrics-{uniq}-b");
    let (fid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) \
         VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2, $3) \
         ON CONFLICT(abs_path) DO UPDATE SET project_id = EXCLUDED.project_id RETURNING id",
    )
    .bind(&name)
    .bind(&abs_path)
    .bind(project_id)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    let (rid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, $2) \
         ON CONFLICT(repo_key) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(format!("test/{uniq}-b"))
    .bind(&name)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $2 WHERE id = $1")
        .bind(fid)
        .bind(rid)
        .execute(pg.pool())
        .await
        .unwrap();
    (fid, rid)
}

/// `git init` a fresh repo at `dir` with a local commit identity and signing
/// disabled — the shared bootstrap for the git-sourced churn fixtures. Panics on
/// any git failure so a broken fixture fails loud rather than silently producing
/// no churn.
pub(crate) fn git_init_repo(dir: &std::path::Path) {
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "test@sensei.test"][..],
        &["config", "user.name", "Sensei Test"][..],
        &["config", "commit.gpgsign", "false"][..],
    ] {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed initializing the churn fixture repo");
    }
}

/// Commit `files` (name → content) into the repo at `dir` with the author AND
/// committer date pinned to `day` (`YYYY-MM-DD`), so `git log`'s committer-day
/// (`%cd --date=short` — the field churn buckets on) lands on `day`. Writes each
/// file, `git add -A`, then commits with `GIT_*_DATE` fixed to noon on `day`. Use
/// distinct content across commits so `--numstat` reflects real per-file line churn.
pub(crate) fn git_commit_on_day(dir: &std::path::Path, day: &str, files: &[(&str, &str)]) {
    for (name, content) in files {
        std::fs::write(dir.join(name), content).unwrap();
    }
    let add = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(add.success(), "git add -A failed in the churn fixture repo");
    let stamp = format!("{day}T12:00:00");
    let ok = std::process::Command::new("git")
        .args(["commit", "-q", "-m", "churn fixture"])
        .current_dir(dir)
        .env("GIT_AUTHOR_DATE", &stamp)
        .env("GIT_COMMITTER_DATE", &stamp)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git commit failed in the churn fixture repo (day {day})");
}

/// Insert one `activity.sessions` row (with `repo_folder_id = fid`, the durable repo
/// anchor `session_outcomes` groups on to resolve the session's repository) and return
/// its id. `outcome`/`ftr` are
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
        "INSERT INTO activity.sessions (folder_id, repo_folder_id, project_id, outcome, ftr, corrections, started_at) \
         VALUES ($1, $1, $2, $3::sensei.session_outcome, $4, $5, $6) RETURNING id",
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

/// Attach one turn with EXPLICIT timing + correction flag — for latency metrics
/// (`time_to_useful_result`), where `ended_at`, `turn_number`, and `is_correction`
/// all matter. The plain [`seed_metrics_turn`] fixes `ended_at = started_at`,
/// `turn_number = 1`, and `is_correction = false`, so it can't exercise them.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn seed_metrics_turn_ex(
    pg: &PgStore,
    sid: &uuid::Uuid,
    turn_number: i32,
    started_at: chrono::DateTime<chrono::Utc>,
    ended_at: chrono::DateTime<chrono::Utc>,
    is_correction: bool,
    tool_calls: i32,
) {
    sqlx_core::query::query(
        "INSERT INTO activity.turns (session_id, turn_number, started_at, ended_at, is_correction, tool_calls) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(sid)
    .bind(turn_number)
    .bind(started_at)
    .bind(ended_at)
    .bind(is_correction)
    .bind(tool_calls)
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

/// Insert one `sensei.memories` row for the `memory_promotion` numerator
/// (knowledge, 5.5) and return its id. `created_at` is set EXPLICITLY (not
/// defaulted to `now()`) so a test can place a memory inside or outside the
/// rolling window — memory_promotion windows the numerator on `created_at` (the
/// stable "learned" timestamp, distinct from `modified_at` which moves on every
/// reinforcement). Project-attributed via `project_id`; `type`/`title`/`content`
/// are the row's only other NOT-NULL-without-default columns. Cascades on the
/// project delete, so [`cleanup_metrics_fixture`] clears it.
pub(crate) async fn seed_memory(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.memories (project_id, type, title, content, created_at) \
         VALUES ($1, 'decision'::sensei.memory_type, 'test-memory', 'test-content', $2) RETURNING id",
    )
    .bind(project_id)
    .bind(created_at)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

/// Insert one `inference.detected_patterns` row with a CHOSEN `instance_count`
/// (the `memory_promotion` denominator counts patterns whose `instance_count >=
/// 3` as "eligible" for distillation). Reuses the production upsert
/// ([`PgStore::upsert_pattern`], which derives `instance_count` from the
/// `instances` array length) rather than re-implementing the SQL — so this seeds
/// an eligible pattern by handing it `instance_count` synthetic instance objects.
/// Distinct `name`s keep rows apart under `(project_id, name, is_anti_pattern)`.
/// Returns the pattern id. Companion to [`seed_detected_pattern`] (which always
/// seeds `instance_count = 0`, i.e. an INELIGIBLE pattern).
pub(crate) async fn seed_pattern_with_instances(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    folder_id: Option<&uuid::Uuid>,
    name: &str,
    is_anti: bool,
    instance_count: usize,
) -> uuid::Uuid {
    let instances: Vec<serde_json::Value> = (0..instance_count)
        .map(|i| serde_json::json!({ "file": format!("f{i}.rs"), "line": i }))
        .collect();
    pg.upsert_pattern(
        project_id,
        folder_id,
        name,
        is_anti,
        None,
        &serde_json::Value::Array(instances),
    )
    .await
    .unwrap()
}

/// Insert one `inference.corrections` row via the production upsert (DRY —
/// [`PgStore::upsert_correction`]) for the `memory_promotion` denominator. A
/// correction is a GLOBAL recurrence cluster keyed by `signature` (unique) with a
/// `count` recurrence tally and a `project_ids` array naming the projects it
/// appeared in; it is "eligible" for a project when `count >= 3` AND the project
/// is a member of `project_ids`. `count`/`project_ids` are the two the metric
/// reads. Returns the correction id. NOTE: corrections have NO project FK
/// (membership is the array), so they never cascade on a project delete — clear
/// them with [`purge_corrections`].
pub(crate) async fn seed_correction(
    pg: &PgStore,
    signature: &str,
    count: i32,
    project_ids: &[uuid::Uuid],
) -> uuid::Uuid {
    let row = crate::corrections::CorrectionRow {
        signature: signature.to_string(),
        text: "test correction".to_string(),
        suggestion: None,
        count,
        project_ids: project_ids.to_vec(),
        last_seen: chrono::Utc::now(),
        memory_id: None,
        instances: serde_json::json!([]),
    };
    pg.upsert_correction(&row).await.unwrap()
}

/// Delete every `inference.corrections` row for the given `signature`s. That
/// table attributes to projects through a `project_ids` array (NO FK), so it
/// never cascades on a project delete and must be cleared explicitly. Idempotent
/// — safe to call at the START of a test (pre-clean against a prior crashed run
/// that leaked rows) AND at the end. A no-op for an empty slice.
pub(crate) async fn purge_corrections(pg: &PgStore, signatures: &[&str]) {
    if signatures.is_empty() {
        return;
    }
    let sigs: Vec<String> = signatures.iter().map(|s| s.to_string()).collect();
    sqlx_core::query::query("DELETE FROM inference.corrections WHERE signature = ANY($1)")
        .bind(&sigs)
        .execute(pg.pool())
        .await
        .unwrap();
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
        "INSERT INTO activity.sessions (folder_id, project_id, repo_folder_id, client_session_id, started_at) \
         VALUES ($1, $2, $1, $3, $4) RETURNING id",
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
/// the client-clock `ts` (the true occurrence time the computer day-buckets on) and
/// the server-side insert `created_at`, so a test can place an event on a chosen day.
/// `family` defaults to `claude`, `payload` to `{}`.
pub(crate) async fn seed_assistant_event(
    pg: &PgStore,
    session_id: &str,
    event_type: &str,
    at: chrono::DateTime<chrono::Utc>,
) {
    seed_assistant_event_ex(pg, session_id, event_type, at, at).await;
}

/// Insert one `activity.assistant_events` row with the client-clock `ts` and the
/// server-side `created_at` set INDEPENDENTLY. This is how a test reproduces a
/// synthesized/back-dated event — a true occurrence time in the past (`ts`) that was
/// only inserted now (`created_at = now`). The autonomy anchor fix buckets
/// `interruption_rate` on `ts` (the occurrence time), NOT `created_at` (the insert
/// time), so a historical `ts` must file the row on its historical day even when
/// `created_at` is today. [`seed_assistant_event`] is the common case where the two
/// coincide.
pub(crate) async fn seed_assistant_event_ex(
    pg: &PgStore,
    session_id: &str,
    event_type: &str,
    ts: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
) {
    sqlx_core::query::query(
        "INSERT INTO activity.assistant_events (session_id, event_type, ts, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(session_id)
    .bind(event_type)
    .bind(ts.timestamp_millis())
    .bind(created_at)
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

/// Insert one `activity.sessions` row carrying BOTH a `client_session_id` (for
/// tool-verdict attribution) AND an `acp_id` (the assistant-family string) — the
/// tool (5.6) fixture. `unused_tools` scopes the global `assistant_tools` registry
/// to the families a project uses, and a project's families are `DISTINCT
/// sessions.acp_id` (the hook session-recorder stamps the harness family there, so
/// it aligns with `assistant_tools.assistant_family`). Distinct from
/// [`seed_metrics_client_session`], which leaves `acp_id` NULL. Returns the id.
pub(crate) async fn seed_tool_session(
    pg: &PgStore,
    fid: &uuid::Uuid,
    pid: &uuid::Uuid,
    client_session_id: &str,
    family: &str,
    started_at: chrono::DateTime<chrono::Utc>,
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (folder_id, project_id, client_session_id, acp_id, started_at) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(fid)
    .bind(pid)
    .bind(client_session_id)
    .bind(family)
    .bind(started_at)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

/// Register one tool in `sensei.assistant_tools` via the production upsert (DRY —
/// [`PgStore::upsert_assistant_tool`]) — the `unused_tools` registry (5.6). The
/// registry is GLOBAL per `assistant_family` (no project id); tests isolate by
/// using a UNIQUE `family` string per test (matched against `sessions.acp_id`).
/// `invoked_name` is the harness-qualified name that the verdict/event stream keys
/// on (`tool_call_verdicts.tool_name` / `assistant_events.tool_name`); use a bare
/// name for a built-in or an `mcp__…` name for MCP.
pub(crate) async fn seed_assistant_tool(
    pg: &PgStore,
    family: &str,
    source_type: &str,
    source_key: &str,
    tool_name: &str,
    invoked_name: &str,
) {
    pg.upsert_assistant_tool(family, source_type, source_key, tool_name, invoked_name, None, None)
        .await
        .unwrap();
}

/// Record one tool-call OUTCOME for the `unused_tools` fixture (5.6): a
/// `PostToolUse` event on `client_session_id` for `invoked_name` at time `at`, plus
/// the `tool_call_verdicts` row that classifies it. `verdict` is `used` (the
/// positive outcome), `partial`, or `ignored`. The verdict WINDOWS on the CALL time
/// — the event's `created_at` (set to `at`) reached via `event_id` — NOT
/// `classified_at`, which a re-classification resets to `now()`; so `at` is what a
/// test uses to place a call inside or outside the window. No FK cascade on either
/// table — clear with [`purge_assistant_events`] + [`purge_tool_verdicts`].
pub(crate) async fn seed_tool_verdict(
    pg: &PgStore,
    client_session_id: &str,
    invoked_name: &str,
    verdict: &str,
    at: chrono::DateTime<chrono::Utc>,
) {
    let (event_id,): (i64,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.assistant_events (session_id, event_type, tool_name, ts, created_at) \
         VALUES ($1, 'PostToolUse', $2, $3, $4) RETURNING id",
    )
    .bind(client_session_id)
    .bind(invoked_name)
    .bind(at.timestamp_millis())
    .bind(at)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.tool_call_verdicts (session_id, event_id, tool_name, verdict) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(client_session_id)
    .bind(event_id)
    .bind(invoked_name)
    .bind(verdict)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// Delete every `sensei.assistant_tools` row for the given `assistant_family`s.
/// The registry has no project/folder FK, so it never cascades and must be cleared
/// explicitly. Idempotent — safe at the START (pre-clean against a leaked run) and
/// end of a test. A no-op for an empty slice.
pub(crate) async fn purge_assistant_tools(pg: &PgStore, families: &[&str]) {
    if families.is_empty() {
        return;
    }
    let fams: Vec<String> = families.iter().map(|s| s.to_string()).collect();
    sqlx_core::query::query("DELETE FROM sensei.assistant_tools WHERE assistant_family = ANY($1)")
        .bind(&fams)
        .execute(pg.pool())
        .await
        .unwrap();
}

/// Delete every `sensei.tool_call_verdicts` row for the given `session_id`s. The
/// table's `session_id` is a plain text client id with NO FK, so it never cascades
/// and must be cleared explicitly. Idempotent — safe at the START (pre-clean) and
/// end of a test. A no-op for an empty slice. (The paired `assistant_events` are
/// cleared with [`purge_assistant_events`].)
pub(crate) async fn purge_tool_verdicts(pg: &PgStore, session_ids: &[&str]) {
    if session_ids.is_empty() {
        return;
    }
    let ids: Vec<String> = session_ids.iter().map(|s| s.to_string()).collect();
    sqlx_core::query::query("DELETE FROM sensei.tool_call_verdicts WHERE session_id = ANY($1)")
        .bind(&ids)
        .execute(pg.pool())
        .await
        .unwrap();
}

/// Remove a metric fixture: the project (cascades its `project_metrics` and
/// `detected_patterns`) and, when given, the folder (cascades its `sessions` →
/// `turns` and its `nodes`). `exec_folder_paths` names the `folder_path`s whose
/// `activity.task_executions` to purge (see [`purge_task_executions`]) — pass `&[]`
/// for fixtures that seed no executions. Also deletes the `sensei.repositories` rows
/// seeded onto this project's folders (by [`seed_project_folder_at`] /
/// [`seed_second_repository`]) so the unique `repo_key` (`test/{uniq}…`) can't collide
/// across runs.
pub(crate) async fn cleanup_metrics_fixture(
    pg: &PgStore,
    pid: &uuid::Uuid,
    fid: Option<&uuid::Uuid>,
    exec_folder_paths: &[&str],
) {
    // Capture the repositories seeded onto this project's folders BEFORE the project
    // delete nulls their `project_id` (folders.project_id is ON DELETE SET NULL).
    // Covers both the primary folder and any `seed_second_repository`; deleted at the
    // end (folders.repository_id is ON DELETE SET NULL, so the ordering is safe).
    let repo_ids: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
        "SELECT DISTINCT repository_id FROM sensei.folders \
          WHERE project_id = $1 AND repository_id IS NOT NULL",
    )
    .bind(pid)
    .fetch_all(pg.pool())
    .await
    .unwrap();
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
    let rids: Vec<uuid::Uuid> = repo_ids.into_iter().map(|(r,)| r).collect();
    if !rids.is_empty() {
        sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
            .bind(&rids)
            .execute(pg.pool())
            .await
            .unwrap();
    }
    purge_task_executions(pg, exec_folder_paths).await;
}

/// Daily local-user metric rows (`scope = 'user'`) for `pid`, keyed by
/// metric `key`: `(key, value, props)` ordered by key. The read-back the six
/// metric compute-handler test modules assert against — was copy-pasted verbatim
/// into each group's `#[cfg(test)]` module (a qlty duplication finding).
pub(crate) async fn daily_project_metric_rows(
    pg: &PgStore,
    pid: &uuid::Uuid,
) -> Vec<(String, f64, serde_json::Value)> {
    sqlx_core::query_as::query_as(
        "SELECT m.key, pm.value::float8, pm.props \
           FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
          WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.scope = 'user' \
          ORDER BY m.key",
    )
    .bind(pid)
    .fetch_all(pg.pool())
    .await
    .unwrap()
}

// `module_metric_rows` (per-module folder_id-set rows) was removed with the repo-grain
// cutover: no computer writes per-module rows anymore (folder_id is not part of the
// `project_metrics_identity` key), so the helper had no callers.
