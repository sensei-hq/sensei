//! Following queued work — the read side of the task queue.
//!
//! Endpoints that hand back a task id (the backfills, any future long job) are
//! only half a contract without this: the caller gets an id it has no way to
//! resolve. These are the endpoints that make an id answerable.
//!
//! ## Why the durable log and not only the live stream
//!
//! The queue broadcasts [`TaskEvent`](crate::tasks::progress::TaskEvent)s, and
//! `/api/tasks/progress` streams them. That is genuinely live, but it is also
//! ephemeral and unaddressed: a subscriber who connects one second after its
//! task finished sees nothing at all, forever, because the events already fired.
//! That is the worst failure mode for a follower — indistinguishable from "still
//! running".
//!
//! So the durable `activity.task_executions` log is the source of truth and the
//! stream is an accelerator. [`get_task`] answers from the log and therefore
//! works before, during, and long after the run. [`task_events`] opens with a
//! snapshot from that same log and only then attaches the live stream, so a late
//! subscriber is caught up rather than stranded.
//!
//! ## Dispatchers
//!
//! A dispatcher (`IngestCaptures` queues one task per transcript) completes
//! in milliseconds having done none of the work. Reporting only its own row
//! would tell a follower "completed" while thousands of children had barely
//! started, so both endpoints carry the children and aggregate them.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{
        Json,
        sse::{Event, Sse},
    },
};
use tokio_stream::StreamExt;

use crate::api::state::AppState;

/// The window `/api/tasks/abandoned` reads when none is asked for: a week.
const ABANDONED_DEFAULT_DAYS: i32 = 7;

/// How many offenders `/api/tasks/abandoned` names.
const ABANDONED_WORST: i64 = 10;

#[derive(serde::Deserialize)]
pub struct AbandonedQuery {
    days: Option<i32>,
    limit: Option<i64>,
}

/// How many per-task abandonment rows `GET /api/tasks/abandoned` names.
const ABANDONED_TASK_LIMIT: i64 = 50;

/// `GET /api/tasks/abandoned?days=N&limit=M` — the tasks the watchdog abandoned
/// (#188), per day and kind, with the worst offenders named AND the per-task
/// rows behind them. A task that times out stops existing; this is where
/// "slow" becomes visible before it is "missing".
pub async fn abandoned(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<AbandonedQuery>,
) -> Result<Json<crate::db::pg_store::Abandonments>, (StatusCode, Json<serde_json::Value>)> {
    let days = q.days.unwrap_or(ABANDONED_DEFAULT_DAYS);
    let limit = q.limit.unwrap_or(ABANDONED_TASK_LIMIT);
    if days < 1 {
        return Err(super::err(StatusCode::BAD_REQUEST, "days must be a positive number of days"));
    }
    if !(1..=500).contains(&limit) {
        return Err(super::err(StatusCode::BAD_REQUEST, "limit must be between 1 and 500"));
    }
    let mut out = state
        .pg
        .watchdog_abandonments(days, ABANDONED_WORST)
        .await
        .map_err(|e| super::err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    out.tasks = state
        .pg
        .abandoned_task_list(days, limit)
        .await
        .map_err(|e| super::err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(out))
}

#[derive(serde::Deserialize)]
pub struct FailedQuery {
    days: Option<i32>,
    limit: Option<i64>,
}

/// How many failures `GET /api/tasks/failed` names by default.
const FAILED_DEFAULT_LIMIT: i64 = 100;

/// `GET /api/tasks/failed?days=N&limit=M` — terminal failures newest-first
/// with kind, path, and reason: the failures feed. Completed and
/// still-running rows are never failures.
pub async fn failed(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<FailedQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let days = q.days.unwrap_or(ABANDONED_DEFAULT_DAYS);
    let limit = q.limit.unwrap_or(FAILED_DEFAULT_LIMIT);
    if days < 1 {
        return Err(super::err(StatusCode::BAD_REQUEST, "days must be a positive number of days"));
    }
    if !(1..=500).contains(&limit) {
        return Err(super::err(StatusCode::BAD_REQUEST, "limit must be between 1 and 500"));
    }
    let tasks = state
        .pg
        .failed_tasks(days, limit)
        .await
        .map_err(|e| super::err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "tasks": tasks })))
}

#[derive(serde::Deserialize)]
pub struct QueueQuery {
    limit: Option<usize>,
}

/// `GET /api/tasks/queue?limit=N` — the queue listing: named pending,
/// blocked, and running tasks with waits, backoff-pending retries, exact
/// totals, and per-kind depth. Lists cap (default 200 each); totals never
/// lie and `truncated` says when a list was cut.
pub async fn queue_listing(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<QueueQuery>,
) -> Result<Json<crate::tasks::queue::QueueListing>, (StatusCode, Json<serde_json::Value>)> {
    let limit = q.limit.unwrap_or(crate::tasks::queue::QUEUE_LIST_LIMIT);
    if limit < 1 {
        return Err(super::err(StatusCode::BAD_REQUEST, "limit must be a positive number"));
    }
    Ok(Json(state.task_queue.list_queue(limit).await))
}

/// Roll the children of a dispatcher into the counts a follower actually wants.
///
/// Pure so the aggregation is testable without a queue or a database.
pub(crate) fn summarize_children(children: &[serde_json::Value]) -> serde_json::Value {
    let count = |s: &str| children.iter().filter(|c| c["status"].as_str() == Some(s)).count();
    let (completed, failed, running) = (count("completed"), count("failed"), count("running"));
    serde_json::json!({
        "total": children.len(),
        "completed": completed,
        "failed": failed,
        "running": running,
        // The question a follower is really asking. A dispatcher's own row says
        // `completed` the moment it finishes queueing, so "is the WORK done" can
        // only be answered by the children.
        "settled": completed + failed == children.len(),
    })
}

/// The live-queue view of a task, for one that has been queued but has not yet
/// started — it has no execution row, so the log alone would 404 a task that
/// genuinely exists and is about to run.
async fn queued_state(state: &AppState, task_id: u64) -> Option<serde_json::Value> {
    state.task_queue.find_task(task_id).await.map(|t| {
        serde_json::json!({
            "taskId": task_id,
            "kind": t.kind.to_string(),
            "pipeline": t.kind.pipeline(),
            "stage": t.kind.stage(),
            "folderPath": t.folder_path,
            "path": t.path,
            "status": "queued",
        })
    })
}

/// `GET /api/tasks/{id}` — what happened to one queued task.
///
/// Answers from the durable log, so it stays valid long after the run finishes —
/// which is the point, since the live stream tells a late subscriber nothing.
///
/// Scoped to the CURRENT daemon session, because a task id is only meaningful
/// there: ids restart at 1 on every boot, so an unscoped lookup for id 1 returns
/// a heap of unrelated rows from previous sessions. That is not a limitation to
/// work around — after a restart the in-memory queue is gone and the task no
/// longer exists in any followable sense (boot reconcile fails its row).
///
/// A task still waiting in the queue has no execution row yet (the executor
/// writes one on start) and is reported as `queued` from the live queue.
///
/// 404 means genuinely unknown: not in the queue and never executed in this
/// session. Fail-closed on a read error (500) — never an empty "not found",
/// which a caller would read as "my task vanished".
pub(crate) async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let since = state.task_queue.session_start();
    let attempts = state.pg.task_execution_attempts(id as i64, since).await.map_err(|e| {
        tracing::warn!(error = %e, task_id = id, "get_task: attempts read failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let children = state.pg.child_task_executions(id as i64, since).await.map_err(|e| {
        tracing::warn!(error = %e, task_id = id, "get_task: children read failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if attempts.is_empty() {
        return match queued_state(&state, id).await {
            Some(q) => Ok(Json(q)),
            None => Err(StatusCode::NOT_FOUND),
        };
    }

    // The newest attempt is the task's current state; the rest are history a
    // follower needs to see a retry for what it was (see
    // `task_execution_attempts`).
    Ok(Json(serde_json::json!({
        "taskId": id,
        "current": attempts[0],
        "attempts": attempts,
        "children": children,
        "childSummary": summarize_children(&children),
    })))
}

/// `GET /api/tasks/{id}/events` — SSE for ONE task and its children.
///
/// Opens with a snapshot frame carrying the same payload as [`get_task`], so a
/// subscriber that attaches late (or after completion) is immediately correct.
/// Live frames follow only for the task and its snapshot-named children:
/// folder rollups and stage events carry no task id, so they cannot be
/// attributed to one follower — a stage is not a queue task, and minting an
/// id to make it fit would attribute it to a follower it has nothing to do
/// with. The frame key carries this rule now (`job` entity + numeric key).
/// instead of waiting on events that already fired. Live `TaskEvent`s follow,
/// filtered to this task and the children known at subscribe time.
///
/// Subscribing BEFORE the snapshot read is deliberate: the reverse order has a
/// window where an event fires after the read and before the subscription, and
/// is lost.
pub(crate) async fn task_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.streams.task_tx.subscribe();
    let cursor = crate::api::stream::parse_cursor(&headers);
    let since = state.task_queue.session_start();

    // One error path for the whole snapshot. Reading children separately and
    // defaulting them to empty would emit a snapshot claiming a dispatcher has no
    // children — indistinguishable from one that genuinely has none, and the
    // follower would conclude the work was done.
    let snapshot_read = async {
        let children = state.pg.child_task_executions(id as i64, since).await?;
        let attempts = state.pg.task_execution_attempts(id as i64, since).await?;
        Ok::<_, String>((attempts, children))
    }
    .await;

    let (snapshot, children) = match snapshot_read {
        Ok((attempts, children)) => (
            crate::api::events::StateEvent::new(
                crate::api::events::Action::Set,
                "job",
                id.to_string(),
                serde_json::json!({
                    "taskId": id,
                    "current": attempts.first(),
                    "attempts": attempts,
                    "children": children,
                    "childSummary": summarize_children(&children),
                }),
            ),
            children,
        ),
        Err(e) => {
            // Say so explicitly rather than sending an empty-looking snapshot: a
            // read failure and "nothing has happened yet" must not look alike.
            tracing::warn!(error = %e, task_id = id, "task_events: snapshot read failed");
            (
                crate::api::events::StateEvent::new(
                    crate::api::events::Action::Set,
                    "job",
                    id.to_string(),
                    serde_json::json!({ "taskId": id }),
                )
                .failed("snapshot-failed", &e),
                Vec::new(),
            )
        }
    };

    let mut followed: std::collections::HashSet<u64> = std::collections::HashSet::new();
    followed.insert(id);
    for c in &children {
        if let Some(t) = c["taskId"].as_i64() {
            followed.insert(t as u64);
        }
    }

    // Snapshot head first (cursor 0: the full state), then framed live
    // frames for this task and the children the snapshot named.
    let head_frame = crate::api::stream::StreamFrame::live(0, snapshot);
    let head = tokio_stream::once(Ok(
        Event::default().data(serde_json::to_string(&head_frame).unwrap_or_default())
    ));
    let live =
        crate::api::stream::framed_stream(rx, state.streams.task_log.clone(), cursor, move |f| {
            f.entity() == "job"
                && f.key().parse::<u64>().map(|tid| followed.contains(&tid)).unwrap_or(false)
        });

    crate::api::stream::heartbeat(head.chain(live))
}

/// `POST /api/tasks/{id}/retry` — re-queue a failed task as a new task id.
///
/// Reads the latest attempt from the durable log and enqueues a fresh task
/// with its kind, paths, and parent (so the retry rejoins its wave's
/// barrier). Plain [`enqueue`](crate::tasks::queue::TaskQueue::enqueue), never
/// `enqueue_unique` — a manual retry is explicit and must not be deduped away.
///
/// 404 when the id never ran in this session; 409 when the latest attempt did
/// not fail, or the kind is not retryable (the same
/// [`is_retryable`](crate::tasks::TaskKind::is_retryable) rule the workers
/// apply, so the button and the log agree). Fail-closed 500 on a read error.
///
/// Fidelity: the log carries kind, paths, and parent only, so the retry runs
/// with default extras (`Scope::Full`, no `as_of`/`url`/`branch`/`force`) —
/// a superset re-walk, never less than the failed work. The url-carrying
/// library kinds are `retryable: false`, so no retry can enqueue a task whose
/// inputs the log cannot reconstruct.
pub(crate) async fn retry_task(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let since = state.task_queue.session_start();
    let attempts = state.pg.task_execution_attempts(id as i64, since).await.map_err(|e| {
        tracing::warn!(error = %e, task_id = id, "retry_task: attempts read failed");
        super::err(StatusCode::INTERNAL_SERVER_ERROR, "attempts read failed")
    })?;
    let latest =
        attempts.first().ok_or_else(|| super::err(StatusCode::NOT_FOUND, "unknown task"))?;
    if latest["status"].as_str() != Some("failed") {
        return Err(super::err(StatusCode::CONFLICT, "only a failed task can be retried"));
    }
    let kind_name = latest["kind"].as_str().unwrap_or_default();
    let kind = crate::tasks::TaskKind::from_name(kind_name).ok_or_else(|| {
        tracing::warn!(task_id = id, kind = kind_name, "retry_task: log names an unknown kind");
        super::err(StatusCode::INTERNAL_SERVER_ERROR, "unknown task kind")
    })?;
    if !kind.is_retryable() {
        return Err(super::err(StatusCode::CONFLICT, format!("{kind_name} must not be re-run")));
    }
    let mut task = crate::tasks::Task::new(
        kind,
        latest["folderPath"].as_str().unwrap_or_default(),
        latest["path"].as_str().unwrap_or_default(),
    );
    task.parent_task_id = latest["parentTaskId"].as_u64();
    let new_id = state.task_queue.enqueue(task).await;
    Ok(Json(serde_json::json!({ "taskId": new_id })))
}

#[derive(serde::Deserialize)]
pub struct WavesQuery {
    wave: Option<String>,
    limit: Option<i64>,
}

/// `GET /api/tasks/waves?wave=&limit=N` — recent wave runs, newest first:
/// open waves, done terminals, and boot-abandoned runs. History for the
/// center beside the live stream; capped (default 50).
pub(crate) async fn waves_listing(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<WavesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let limit = q.limit.unwrap_or(50);
    if limit < 1 {
        return Err(super::err(StatusCode::BAD_REQUEST, "limit must be a positive number"));
    }
    let waves = state.pg.list_wave_runs(q.wave.as_deref(), limit).await.map_err(|e| {
        tracing::warn!(error = %e, "waves_listing: read failed");
        super::err(StatusCode::INTERNAL_SERVER_ERROR, "waves read failed")
    })?;
    Ok(Json(serde_json::json!({ "waves": waves })))
}

/// `GET /api/tasks/kinds` — the task catalogue, grouped by pipeline.
///
/// Exists because the pipelines were previously convention only: which stage a
/// kind belonged to lived in reviewers' heads and in the order of a `match`.
/// Publishing the grouping means the app, the docs and a human debugging a
/// stalled queue all read the same answer from one place.
///
/// Pure: derived entirely from the kind descriptors, so it cannot drift from the
/// behaviour it describes.
pub(crate) async fn list_kinds() -> Json<serde_json::Value> {
    use std::collections::BTreeMap;
    let mut by_pipeline: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for k in crate::tasks::TaskKind::ALL {
        let i = k.info();
        by_pipeline.entry(format!("{:?}", i.pipeline).to_lowercase()).or_default().push(
            serde_json::json!({
                "kind": i.name,
                "stage": format!("{:?}", i.stage).to_lowercase(),
                "budgetSecs": i.budget_secs,
                "highPriority": i.high_priority,
                "retryable": i.retryable,
            }),
        );
    }
    Json(serde_json::json!({ "pipelines": by_pipeline }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::progress::TaskEvent;

    fn child(status: &str) -> serde_json::Value {
        serde_json::json!({ "taskId": 1, "status": status })
    }

    #[test]
    fn a_dispatcher_is_not_settled_while_a_child_still_runs() {
        // The whole reason children are aggregated: the dispatcher's OWN row says
        // `completed` the instant it finishes queueing, so only the children can
        // answer "is the work done".
        let s = summarize_children(&[child("completed"), child("running")]);
        assert_eq!(s["settled"], false);
        assert_eq!(s["total"], 2);
        assert_eq!(s["running"], 1);
    }

    #[test]
    fn a_failed_child_still_counts_as_settled() {
        // Settled means "no longer moving", not "succeeded" — a follower that
        // waited for completed == total would hang forever on a failure.
        let s = summarize_children(&[child("completed"), child("failed")]);
        assert_eq!(s["settled"], true);
        assert_eq!(s["failed"], 1);
    }

    #[test]
    fn no_children_is_settled_not_stuck() {
        // A leaf task has no children; reporting it unsettled would strand any
        // follower that waits on the summary.
        let s = summarize_children(&[]);
        assert_eq!(s["settled"], true);
        assert_eq!(s["total"], 0);
    }

    /// The frame key carries the old `event_task_id` rule: job frames key
    /// by task id (followable), folder rollups and stages do not (their
    /// keys never parse as the followed task).
    /// The shared `AppState` fixture. DB-backed: `make_ctx` connects to
    /// `sensei_test` and panics if it cannot, so every async test below
    /// needs the daemon database running — there is no graceful skip.
    async fn state() -> crate::api::state::AppState {
        crate::tasks::test_support::make_ctx().await.app_state.clone()
    }

    /// Throwaway execution-row ids. Queue ids restart at 1 per context, so
    /// seeded rows sit far above any real id; each test takes its own.
    static RETRY_TEST_ID: std::sync::atomic::AtomicI64 =
        std::sync::atomic::AtomicI64::new(1_000_000_000);

    fn retry_test_id() -> i64 {
        RETRY_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    async fn seed_execution(
        state: &crate::api::state::AppState,
        task_id: i64,
        kind: &str,
        status: &str,
    ) {
        sqlx_core::query::query(
            "INSERT INTO activity.task_executions(task_id, parent_task_id, task_kind, folder_path, path, status, error_message) \
             VALUES ($1, 4242, $2::sensei.task_execution_kind, '/r', '/r/a.ts', $3, 'boom')",
        )
        .bind(task_id)
        .bind(kind)
        .bind(status)
        .execute(state.pg.pool())
        .await
        .expect("seed execution row");
    }

    #[tokio::test]
    async fn retry_requeues_a_failed_task_with_its_identity() {
        let state = state().await;
        let id = retry_test_id();
        seed_execution(&state, id, "process_file", "failed").await;
        let body = retry_task(State(state.clone()), Path(id as u64))
            .await
            .expect("a failed retryable task re-queues");
        let new_id = body.0["taskId"].as_u64().expect("the new task id");
        assert_ne!(new_id, id as u64, "a retry is a new task, not the old id");
        let task = state.task_queue.find_task(new_id).await.expect("the re-queued task");
        assert_eq!(task.kind.to_string(), "process_file");
        assert_eq!(task.path, "/r/a.ts");
        assert_eq!(task.parent_task_id, Some(4242), "the retry rejoins its wave");
    }

    #[tokio::test]
    async fn retry_refuses_a_task_that_never_ran() {
        let state = state().await;
        let err = retry_task(State(state), Path(retry_test_id() as u64)).await.expect_err("404");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn retry_refuses_a_task_that_did_not_fail() {
        let state = state().await;
        let id = retry_test_id();
        seed_execution(&state, id, "process_file", "completed").await;
        let err = retry_task(State(state), Path(id as u64)).await.expect_err("409");
        assert_eq!(err.0, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn waves_list_recent_runs_filterable_by_wave() {
        let state = state().await;
        let key = format!("waves-test-{}", uuid::Uuid::new_v4());
        state.pg.open_wave_run("transcript", &key, None, 3).await.unwrap();
        state.pg.close_wave_run("transcript", &key, 3, 3, "done").await.unwrap();
        let body = waves_listing(
            State(state.clone()),
            axum::extract::Query(WavesQuery { wave: Some("transcript".into()), limit: None }),
        )
        .await
        .expect("waves list");
        let runs = body.0["waves"].as_array().expect("waves array");
        let row = runs.iter().find(|r| r["key"] == key).expect("the closed wave lists");
        assert_eq!(row["outcome"], "done");
        assert_eq!(row["settled"], 3);
        let empty = waves_listing(
            State(state),
            axum::extract::Query(WavesQuery { wave: Some("metric".into()), limit: None }),
        )
        .await
        .expect("waves list");
        assert!(
            !empty.0["waves"].as_array().expect("waves array").iter().any(|r| r["key"] == key),
            "the wave filter narrows"
        );
    }

    #[tokio::test]
    async fn waves_listing_refuses_a_non_positive_limit() {
        let state = state().await;
        let err = waves_listing(
            State(state),
            axum::extract::Query(WavesQuery { wave: None, limit: Some(0) }),
        )
        .await
        .expect_err("limit 0 is a 400");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn retry_refuses_a_kind_that_must_not_rerun() {
        let state = state().await;
        let id = retry_test_id();
        seed_execution(&state, id, "import_lib", "failed").await;
        let err = retry_task(State(state), Path(id as u64)).await.expect_err("409");
        assert_eq!(err.0, StatusCode::CONFLICT);
    }

    #[test]
    fn frame_keys_route_followers_and_exclude_rollups() {
        use crate::api::stream::StreamFrame;
        let job = StreamFrame::from_task_event(
            1,
            &TaskEvent::Queued {
                task_id: 7,
                kind: "process_file".into(),
                folder_path: "/r".into(),
                path: "/r/a".into(),
            },
        );
        assert_eq!(job.key().parse::<u64>().ok(), Some(7));
        let rollup = StreamFrame::from_task_event(
            2,
            &TaskEvent::FolderQueued {
                folder_path: "/x".into(),
                files_total: 3,
                files_expected: Some(9),
            },
        );
        assert_eq!(rollup.entity(), "folder");
        assert!(rollup.key().parse::<u64>().is_err());
        let stage = StreamFrame::from_task_event(
            3,
            &TaskEvent::StageStarted { stage: "scan_root".into(), path: "/x".into() },
        );
        assert!(stage.key().parse::<u64>().is_err());
    }
}
