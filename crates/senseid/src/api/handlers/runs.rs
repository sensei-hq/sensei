//! Relay-engine (P3.2) — the run observability API.
//!
//! Read-only endpoints over the daemon-owned `activity.runs` / `run_events`:
//!
//! - `GET /api/runs`       → `{ runs: [Run] }` — the active runs (running /
//!   paused / stalled), newest-started first.
//! - `GET /api/runs/{id}`  → `{ run: Run, events: [RunEvent] }` — one run plus
//!   its latest cadence events; `404` for a well-formed id with no row, `400`
//!   for a non-UUID id (mirrors `sessions::get_session`).
//! - `POST /api/runs`      → `{ run: Run }` (201) — create a daemon-owned run
//!   (P3.8 run-control; the MCP `start_run` tool + the desktop app kick runs off
//!   here). The scheduler picks it up on the next tick.
//!
//! The reads render the same durable state the scheduler/handler write, so the
//! console/phone can show a run's status + cadence without touching the queue.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};

use crate::api::state::AppState;
use crate::runs::{NewRun, RunEventKind};
use dojo_protocol::relay::{RelayRunStatus, SegmentState};

/// Cap on the cadence events returned for a single run — the recent tail is what
/// the timeline renders; older events page in later if we need them.
const RUN_EVENTS_LIMIT: i64 = 20;

/// GET /api/runs — the active runs (running / paused / stalled), newest-started
/// first. Returns `{ runs: [Run] }`; a DB error is a 500.
pub(crate) async fn list_runs(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.pg.list_active_runs().await {
        Ok(runs) => Ok(Json(serde_json::json!({ "runs": runs }))),
        Err(e) => {
            tracing::error!(error = %e, "list_runs failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/runs/{id} — one run plus its latest ~20 cadence events. A well-formed
/// UUID with no row yields 404 (not 500); a non-UUID id yields 400. Returns
/// `{ run: Run, events: [RunEvent] }`.
pub(crate) async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let run = match state.pg.get_run(&uuid).await {
        Ok(Some(run)) => run,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!(error = %e, run = %id, "get_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let events = match state.pg.list_run_events(&uuid, RUN_EVENTS_LIMIT).await {
        Ok(events) => events,
        Err(e) => {
            tracing::error!(error = %e, run = %id, "get_run events lookup failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    Ok(Json(serde_json::json!({ "run": run, "events": events })))
}

/// Resolve `body["project"]` (name-or-uuid) → a project id, and stamp the run's
/// git author from that project's repo root (`user.name`/`user.email`, git's
/// local→global precedence — the identity key that matches the commit author +
/// the Dōjō sign-in). Shared by `create_run` + `register_plan` (DRY). `Ok((None,
/// None, None))` when no project is given; `Err(400)` when a **given** project is
/// unresolvable (intent is never silently dropped). An unresolvable git identity
/// (not a repo) leaves the author columns NULL, not an error.
async fn resolve_project_and_author(
    state: &AppState,
    body: &serde_json::Value,
) -> Result<(Option<uuid::Uuid>, Option<String>, Option<String>), StatusCode> {
    let project_id = match body["project"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        // resolve fails closed: a DB error propagates as 500; a genuine miss is a 400.
        Some(p) => match crate::api::util::resolve_project_uuid(state, p).await? {
            Some(id) => Some(id),
            None => return Err(StatusCode::BAD_REQUEST),
        },
        None => None,
    };
    let (author_name, author_email) = match project_id {
        Some(pid) => match state.pg.project_root_path(&pid).await.ok().flatten() {
            Some(dir) => {
                let user = crate::git_identity::read_git_user(std::path::Path::new(&dir));
                (user.name, user.email)
            }
            None => (None, None),
        },
        None => (None, None),
    };
    Ok((project_id, author_name, author_email))
}

/// Build the run-creation response `{ run, track_url }` (201). `track_url` is the
/// auth-gated Dōjō run-detail link (`<registry>/you/runs/<id>`) at the dōjō the
/// run federates to — set ONLY when the run resolves to exactly one dōjō (its
/// bound membership, or a single enabled one). With several enabled dōjōs and no
/// binding it's ambiguous which to link, so `track_url` is `None` rather than an
/// arbitrary org's URL. `None` too when no dōjō is connected or a resolve errors —
/// the URL is a courtesy, never a reason to fail run creation.
async fn run_created_response(
    state: &AppState,
    run: crate::runs::Run,
) -> (StatusCode, Json<serde_json::Value>) {
    let memberships =
        crate::tasks::handlers::resolve_run_memberships(&state.pg, &run).await.unwrap_or_default();
    let track_url = crate::resolution::Resolution::from_unique(memberships.iter())
        .resolved()
        .map(|m| crate::dojo::memberships::dojo_run_url(&m.registry_url, &run.id));
    (StatusCode::CREATED, Json(serde_json::json!({ "run": run, "track_url": track_url })))
}

/// POST /api/runs — create a daemon-owned run (P3.8 run-control; the MCP
/// `start_run` tool and the desktop app kick runs off here). Body:
/// `{ "goal": string (required, non-empty), "project"?: name-or-uuid,
///    "plan_ref"?: string, "max_concurrency"?: int }`.
///
/// - An empty/absent `goal` is a 400 — a run with nothing to drive is a no-op.
/// - `project` resolves name-OR-uuid → id ([`crate::api::util::resolve_project_uuid`]);
///   absent means no project (the drive Flags "no cwd" rather than spawning
///   loose), while a **given-but-unresolvable** project is a 400 so the caller's
///   intent is never silently dropped.
///
/// Returns `{ run: Run }` with `201 Created`. The scheduler advances it on the
/// next tick; whether it actually drives an agent still depends on the
/// OFF-by-default `SENSEI_RUN_DRIVE`.
pub(crate) async fn create_run(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let goal = body["goal"].as_str().map(str::trim).unwrap_or("");
    if goal.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (project_id, author_name, author_email) = resolve_project_and_author(&state, &body).await?;
    let plan_ref =
        body["plan_ref"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let max_concurrency = body["max_concurrency"].as_i64().map(|n| n as i32);

    let new = NewRun {
        project_id,
        plan_ref,
        goal: Some(goal.to_string()),
        dojo_session_id: None,
        max_concurrency,
        author_name,
        author_email,
        // start_run creates a graph-less run; register_plan is the verb that seeds
        // an authored plan graph.
        plan_graph: None,
    };
    let id = match state.pg.create_run(&new).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "create_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    match state.pg.get_run(&id).await {
        Ok(Some(run)) => Ok(run_created_response(&state, run).await),
        // Created then vanished between INSERT and read-back — shouldn't happen.
        Ok(None) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        Err(e) => {
            tracing::error!(error = %e, "create_run read-back failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/runs/pause — mark a run **paused until a reset time** (a limit-wait),
/// so it reads `paused` (not `stalled`) and `resume_due_runs` auto-resumes it at
/// the reset. This is the distinct resumable state for "waiting for a usage
/// limit". Body: `{ "until": RFC-3339 (required), "reason"?: string,
/// "run_id"?: uuid, "project"?: name-or-uuid }` — targets an explicit `run_id`,
/// else the active (running/stalled) run for the project.
pub(crate) async fn pause_run(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(until) = body["until"].as_str().map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    // Validate the instant here so a bad value is a clean 400, not a DB-cast 500.
    if chrono::DateTime::parse_from_rfc3339(until).is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let reason = body["reason"].as_str().map(str::trim).filter(|s| !s.is_empty());

    // Target: an explicit run_id, else the active run for the (given/cwd) project.
    let run_id = match body["run_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
        Some(id) => Some(id),
        None => match body["project"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
            Some(p) => match crate::api::util::resolve_project_uuid(&state, p).await? {
                Some(pid) => state
                    .pg
                    .active_run_for_project(&pid)
                    .await
                    .map_err(|e| {
                        tracing::warn!(error = %e, "pause_run: active_run_for_project failed");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .map(|r| r.id),
                None => None,
            },
            None => None,
        },
    };
    let Some(run_id) = run_id else {
        return Err(StatusCode::NOT_FOUND);
    };

    if let Err(e) =
        state.pg.update_run_status(&run_id, RelayRunStatus::Paused, Some(until), reason).await
    {
        tracing::error!(error = %e, "pause_run failed");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    // Cadence marker so the feed shows the pause + when it resumes. Best-effort.
    let _ = state
        .pg
        .append_run_event(
            &run_id,
            RunEventKind::PausedOnLimit,
            None,
            None,
            &serde_json::json!({ "until": until, "reason": reason, "via": "pause_run" }),
        )
        .await;

    match state.pg.get_run(&run_id).await {
        Ok(Some(run)) => Ok(Json(serde_json::json!({ "run": run }))),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/runs/plan — register an authored plan graph as a daemon-owned run
/// (AR-2; the MCP `register_plan` tool fronts this). Body: `{ "goal": string
/// (required), "plan": PlanGraph (required), "project"?: name-or-uuid,
/// "plan_ref"?: string, "max_concurrency"?: int }`.
///
/// The graph is validated (non-empty, unique task ids, deps resolve, DAG) and
/// stored in `activity.runs.plan_graph`; the background `publish_run` then authors
/// the relay outline from it (phases→tasks with agent/model/spec_ref). A bad graph
/// is a 400 — never a persisted broken run. Returns `{ run: Run }` (201).
pub(crate) async fn register_plan(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    // Reject with a MESSAGE (in the 400 body), not a bare status — the caller
    // (register_plan MCP tool / an executor) needs to know WHAT to fix.
    let bad =
        |msg: String| Ok((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": msg }))));

    let goal = body["goal"].as_str().map(str::trim).unwrap_or("");
    if goal.is_empty() {
        return bad("goal is required".into());
    }
    let Some(plan_val) = body.get("plan").filter(|v| !v.is_null()) else {
        return bad("plan is required and must be a valid JSON object".into());
    };
    let graph: crate::plan_graph::PlanGraph = match serde_json::from_value(plan_val.clone()) {
        Ok(g) => g,
        Err(e) => return bad(format!("invalid plan structure: {e}")),
    };
    // A plan with no tasks is a no-op; a malformed DAG (dup ids / dangling / cycle)
    // is a 400 with the reason rather than a persisted broken run an executor
    // can't schedule.
    if graph.task_count() == 0 {
        return bad("plan has no tasks".into());
    }
    if let Err(e) = crate::plan_graph::validate(&graph) {
        return bad(format!("invalid plan graph: {e}"));
    }

    let (project_id, author_name, author_email) = resolve_project_and_author(&state, &body).await?;
    let plan_ref =
        body["plan_ref"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let max_concurrency = body["max_concurrency"].as_i64().map(|n| n as i32);

    // Normalize (fills the default per-task state) before storing the jsonb.
    let normalized = match serde_json::to_value(&graph) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "register_plan: graph re-serialize failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let new = NewRun {
        project_id,
        plan_ref,
        goal: Some(goal.to_string()),
        dojo_session_id: None,
        max_concurrency,
        author_name,
        author_email,
        plan_graph: Some(normalized),
    };
    let id = match state.pg.create_run(&new).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "register_plan create_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    match state.pg.get_run(&id).await {
        Ok(Some(run)) => Ok(run_created_response(&state, run).await),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/runs/{id}/tasks/{task_id} — flip a plan task's state as the executor
/// works the graph (AR-3; the MCP `update_task_status` tool fronts this). Body:
/// `{ "state": segment_state (required), "note"?: string }`.
///
/// Updates the task's state in `activity.runs.plan_graph` (so the authored relay
/// outline re-projects with the new state) AND appends a feature-class cadence
/// event so the run's PROGRESS clock stays fresh — it never touches `heartbeat_at`
/// (that stays daemon-owned so the watchdog can still catch a real stall). 404 if
/// the run has no plan graph or the task id is unknown; 400 for a bad state.
pub(crate) async fn update_task_status(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let run_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let Some(new_state) = body["state"].as_str().and_then(SegmentState::from_db_str) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let note = body["note"].as_str().map(str::trim).filter(|s| !s.is_empty());

    let raw = match state.pg.run_plan_graph(&run_id).await {
        Ok(Some(raw)) => raw,
        Ok(None) => return Err(StatusCode::NOT_FOUND), // no plan → nothing to flip
        Err(e) => {
            tracing::error!(error = %e, run = %id, "update_task_status: plan_graph read failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let mut graph: crate::plan_graph::PlanGraph = match serde_json::from_value(raw) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(error = %e, run = %id, "update_task_status: plan_graph parse failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    if !crate::plan_graph::set_task_state(&mut graph, &task_id, new_state) {
        return Err(StatusCode::NOT_FOUND); // unknown task id — never a silent no-op
    }
    let updated = match serde_json::to_value(&graph) {
        Ok(v) => v,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    if let Err(e) = state.pg.set_run_plan_graph(&run_id, &updated).await {
        tracing::error!(error = %e, run = %id, "update_task_status: plan_graph write failed");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Feed the progress clock + timeline. Best-effort: the state write above is the
    // authoritative change; a missed cadence marker only delays a stall reset.
    if let Some(kind) = task_event_kind(new_state) {
        let _ = state
            .pg
            .append_run_event(
                &run_id,
                kind,
                None,
                Some(&task_id),
                &serde_json::json!({ "state": new_state.as_db_str(), "note": note, "via": "update_task_status" }),
            )
            .await;
    }
    Ok(Json(serde_json::json!({ "ok": true, "task_id": task_id, "state": new_state.as_db_str() })))
}

/// Map a task's new state to the cadence-event kind that records it (timeline +
/// progress clock). `Pending` emits nothing (the initial state, not a
/// transition). A task-level failure/block is a `Flagged` marker — NOT a run-level
/// `Failed` (only `report_run_outcome` fails a run).
fn task_event_kind(state: SegmentState) -> Option<RunEventKind> {
    match state {
        SegmentState::Active => Some(RunEventKind::FeatureStarted),
        SegmentState::Done | SegmentState::Skipped => Some(RunEventKind::FeatureDone),
        SegmentState::Failed | SegmentState::Blocked | SegmentState::NeedsReview => {
            Some(RunEventKind::Flagged)
        }
        SegmentState::Pending => None,
    }
}

/// POST /api/runs/{id}/outcome — mark a run terminal (AR-3; the MCP
/// `report_run_outcome` tool fronts this). Body: `{ "outcome": "done"|"failed"
/// (required), "summary"?: string }`. This is the ONE terminal transition an
/// external coordinator may set — the watchdog keeps independent stall/crash
/// authority. 400 for anything but done/failed; 404 for an unknown run.
pub(crate) async fn report_run_outcome(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let run_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let status = match body["outcome"].as_str().map(str::trim) {
        Some("done") => RelayRunStatus::Done,
        Some("failed") => RelayRunStatus::Failed,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let summary = body["summary"].as_str().map(str::trim).filter(|s| !s.is_empty());

    // The run must exist — a clean 404, not a silent no-op UPDATE.
    match state.pg.get_run(&run_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!(error = %e, run = %id, "report_run_outcome: get_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    if let Err(e) = state.pg.complete_run(&run_id, status).await {
        tracing::error!(error = %e, run = %id, "report_run_outcome: complete_run failed");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let kind =
        if status == RelayRunStatus::Done { RunEventKind::Done } else { RunEventKind::Failed };
    let _ = state
        .pg
        .append_run_event(
            &run_id,
            kind,
            None,
            None,
            &serde_json::json!({ "summary": summary, "via": "report_run_outcome" }),
        )
        .await;

    // A terminal run drops out of `list_active_runs`, so the scheduler stops
    // federating it. Enqueue ONE final PublishRun so the terminal status
    // (done/failed) reaches Dōjō — otherwise the last-federated "running" stays
    // stale on the phone/console.
    crate::tasks::advance_run_scheduler::enqueue_publish_run(&state.task_queue, &run_id).await;

    match state.pg.get_run(&run_id).await {
        Ok(Some(run)) => Ok(Json(serde_json::json!({ "run": run }))),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/runs/{id}/nudges — the pending human→agent steer for a run (AR-3; the
/// MCP `get_pending_nudges` tool fronts this). The agent-PULL side of the "daemon
/// initiates a check" contract: the executor polls it each loop and acts on any
/// human nudge/chat surfaced from the run's Dōjō inbox. Read-only + **fail-soft** —
/// no enrolled dojo, a membership-resolve failure, or an inbox-poll failure all
/// return `{ nudges: [] }`, never a 500, so a transient dojo hiccup can't wedge the
/// executor (it retries next poll). 404 for an unknown run; 400 for a non-UUID id.
///
/// STEER, not drive: this only surfaces the human's message; `SENSEI_RUN_DRIVE`
/// stays OFF. Zero-knowledge: only code-free logical fields (id/kind/text/time).
pub(crate) async fn get_pending_nudges(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let run_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let run = match state.pg.get_run(&run_id).await {
        Ok(Some(run)) => run,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!(error = %e, run = %id, "get_pending_nudges: get_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let empty = || Ok(Json(serde_json::json!({ "nudges": [] })));
    let memberships = match crate::tasks::handlers::resolve_run_memberships(&state.pg, &run).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, run = %id, "get_pending_nudges: membership resolve failed (fail-soft)");
            return empty();
        }
    };
    // Poll THIS run's dōjō only. Exactly one resolved → use it; ambiguous
    // (unbound run, several enabled dōjōs) → surface nothing rather than an
    // arbitrary tenant's inbox.
    let m = match crate::resolution::Resolution::from_unique(memberships.iter()) {
        crate::resolution::Resolution::Resolved(m) => m,
        crate::resolution::Resolution::Ambiguous { count } => {
            tracing::debug!(run = %id, count, "get_pending_nudges: {count} enabled memberships for an unbound run — no steer surfaced (ambiguous dōjō)");
            return empty();
        }
        crate::resolution::Resolution::Unresolved => return empty(),
    };
    let client = crate::dojo::client::DojoClient::for_membership(m);
    let nudges = match client.poll_inbox(0).await {
        Ok(pull) => crate::dojo::relay_nudge::pickup_nudges(&pull.items, &run_id.to_string()),
        Err(e) => {
            tracing::warn!(error = %e, run = %id, "get_pending_nudges: inbox poll failed (fail-soft)");
            Vec::new()
        }
    };
    // `Nudge` is code-free logical fields; expose them explicitly (it is not
    // Serialize, and this keeps the wire shape stable + zero-knowledge).
    let out: Vec<serde_json::Value> = nudges
        .iter()
        .map(|n| serde_json::json!({ "id": n.id, "kind": n.kind, "text": n.text, "answered_at": n.answered_at }))
        .collect();
    Ok(Json(serde_json::json!({ "nudges": out })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::state::SharedState;
    use crate::runs::{NewRun, RunEventKind};
    use crate::tasks::queue::TaskQueue;
    use std::sync::Arc;

    async fn make_state() -> Option<AppState> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        Some(Arc::new(SharedState {
            task_queue: queue,
            pg,
            gateway,
            event_tx: {
                let (tx, _) = tokio::sync::broadcast::channel(16);
                tx
            },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        }))
    }

    async fn del(state: &AppState, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id)
            .execute(state.pg.pool())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn get_run_bad_uuid_is_400() {
        let Some(state) = make_state().await else {
            return;
        };
        let err = get_run(State(state), Path("not-a-uuid".into())).await.unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_run_unknown_is_404() {
        let Some(state) = make_state().await else {
            return;
        };
        let missing = uuid::Uuid::new_v4().to_string();
        let err = get_run(State(state), Path(missing)).await.unwrap_err();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_run_empty_goal_is_400() {
        let Some(state) = make_state().await else {
            return;
        };
        // No DB row created — the guard rejects before any insert.
        let err =
            create_run(State(state), Json(serde_json::json!({ "goal": "   " }))).await.unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_run_unresolvable_project_is_400() {
        let Some(state) = make_state().await else {
            return;
        };
        // A given project that resolves to nothing is a 400 (intent not dropped).
        let err = create_run(
            State(state),
            Json(serde_json::json!({ "goal": "do a thing", "project": "no-such-project-xyzzy" })),
        )
        .await
        .unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_run_happy_path_returns_201_and_running_run() {
        let Some(state) = make_state().await else {
            return;
        };
        let (status, Json(body)) = create_run(
            State(state.clone()),
            Json(serde_json::json!({ "goal": "ship the thing", "plan_ref": "docs/plan/x.md" })),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["run"]["goal"], serde_json::json!("ship the thing"));
        assert_eq!(body["run"]["plan_ref"], serde_json::json!("docs/plan/x.md"));
        assert_eq!(body["run"]["status"], serde_json::json!("running"));
        // No project given → project_id is null (drive will Flag "no cwd").
        assert!(body["run"]["project_id"].is_null());
        // The response carries the track_url handoff field (a string when a dōjō is
        // connected, else null — robust to whatever memberships the DB holds).
        assert!(body.get("track_url").is_some(), "response carries the track_url field");

        let id = uuid::Uuid::parse_str(body["run"]["id"].as_str().unwrap()).unwrap();
        del(&state, &id).await;
    }

    #[tokio::test]
    async fn list_and_get_return_run_with_events() {
        let Some(state) = make_state().await else {
            return;
        };
        let id = state.pg.create_run(&NewRun::default()).await.unwrap();
        state
            .pg
            .append_run_event(
                &id,
                RunEventKind::Housekeeping,
                None,
                None,
                &serde_json::json!({ "tick": true }),
            )
            .await
            .unwrap();

        // list_runs includes the active run.
        let Json(list) = list_runs(State(state.clone())).await.unwrap();
        let ids: Vec<&str> =
            list["runs"].as_array().unwrap().iter().filter_map(|r| r["id"].as_str()).collect();
        assert!(ids.contains(&id.to_string().as_str()), "active run listed");

        // get_run returns the run + its events.
        let Json(one) = get_run(State(state.clone()), Path(id.to_string())).await.unwrap();
        assert_eq!(one["run"]["id"].as_str(), Some(id.to_string().as_str()));
        assert_eq!(one["run"]["status"], serde_json::json!("running"));
        let events = one["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["kind"], serde_json::json!("housekeeping"));

        del(&state, &id).await;
    }

    #[tokio::test]
    async fn register_plan_rejects_bad_input() {
        let Some(state) = make_state().await else {
            return;
        };
        // Bad input surfaces as `Ok((400, {error}))` — the daemon reports WHY the
        // graph was rejected in the body (see the sibling success test) — NOT an
        // `Err(StatusCode)`. Assert the status on the returned tuple.
        // Missing plan → 400.
        let (status, _) =
            register_plan(State(state.clone()), Json(serde_json::json!({ "goal": "g" })))
                .await
                .unwrap();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // Empty goal → 400.
        let (status, _) = register_plan(
            State(state.clone()),
            Json(serde_json::json!({ "goal": "  ", "plan": { "phases": [] } })),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // A cyclic graph is rejected (validate gate).
        let cyclic = serde_json::json!({ "goal": "g", "plan": { "phases": [{ "title": "P", "tasks": [
            { "id": "a", "title": "a", "deps": ["b"] },
            { "id": "b", "title": "b", "deps": ["a"] }
        ]}]}});
        let (status, _) = register_plan(State(state), Json(cyclic)).await.unwrap();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn register_plan_stores_graph_and_returns_running_run() {
        let Some(state) = make_state().await else {
            return;
        };
        let plan = serde_json::json!({ "goal": "ship register_plan", "plan": { "phases": [
            { "title": "Schema", "tasks": [{ "id": "t1", "title": "columns", "agent": "general-purpose", "model": "sonnet" }] },
            { "title": "Daemon", "tasks": [{ "id": "t2", "title": "handler", "model": "opus", "deps": ["t1"] }] }
        ]}});
        let (status, Json(body)) = register_plan(State(state.clone()), Json(plan)).await.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["run"]["status"], serde_json::json!("running"));
        // register_plan surfaces the same track_url handoff field as start_run.
        assert!(
            body.get("track_url").is_some(),
            "register_plan response carries the track_url field"
        );
        let id = uuid::Uuid::parse_str(body["run"]["id"].as_str().unwrap()).unwrap();
        // The authored graph is persisted (publish_run will project it), with the
        // default per-task state filled in on normalize.
        let g = state.pg.run_plan_graph(&id).await.unwrap().expect("graph stored");
        assert_eq!(g["phases"][1]["tasks"][0]["id"], serde_json::json!("t2"));
        assert_eq!(g["phases"][0]["tasks"][0]["state"], serde_json::json!("pending"));
        del(&state, &id).await;
    }

    #[tokio::test]
    async fn update_task_status_flips_state_bad_state_and_unknown_task() {
        let Some(state) = make_state().await else {
            return;
        };
        let (_s, Json(body)) = register_plan(
            State(state.clone()),
            Json(serde_json::json!({ "goal": "g", "plan": { "phases": [
                { "title": "P", "tasks": [{ "id": "t1", "title": "x" }] }
            ]}})),
        )
        .await
        .unwrap();
        let id = uuid::Uuid::parse_str(body["run"]["id"].as_str().unwrap()).unwrap();

        // Bad state → 400.
        let e = update_task_status(
            State(state.clone()),
            Path((id.to_string(), "t1".into())),
            Json(serde_json::json!({ "state": "not-a-state" })),
        )
        .await
        .unwrap_err();
        assert_eq!(e, StatusCode::BAD_REQUEST);
        // Unknown task → 404 (never a silent no-op).
        let e = update_task_status(
            State(state.clone()),
            Path((id.to_string(), "ghost".into())),
            Json(serde_json::json!({ "state": "active" })),
        )
        .await
        .unwrap_err();
        assert_eq!(e, StatusCode::NOT_FOUND);
        // Happy: flip t1 → done, persisted in the graph.
        let Json(ok) = update_task_status(
            State(state.clone()),
            Path((id.to_string(), "t1".into())),
            Json(serde_json::json!({ "state": "done" })),
        )
        .await
        .unwrap();
        assert_eq!(ok["state"], serde_json::json!("done"));
        let g = state.pg.run_plan_graph(&id).await.unwrap().unwrap();
        assert_eq!(g["phases"][0]["tasks"][0]["state"], serde_json::json!("done"));
        del(&state, &id).await;
    }

    #[tokio::test]
    async fn update_task_status_404_when_run_has_no_graph() {
        let Some(state) = make_state().await else {
            return;
        };
        let id = state.pg.create_run(&NewRun::default()).await.unwrap(); // graph-less run
        let e = update_task_status(
            State(state.clone()),
            Path((id.to_string(), "t1".into())),
            Json(serde_json::json!({ "state": "active" })),
        )
        .await
        .unwrap_err();
        assert_eq!(e, StatusCode::NOT_FOUND);
        del(&state, &id).await;
    }

    #[tokio::test]
    async fn report_run_outcome_marks_terminal_or_rejects() {
        let Some(state) = make_state().await else {
            return;
        };
        let id = state.pg.create_run(&NewRun::default()).await.unwrap();
        // Bad outcome → 400.
        let e = report_run_outcome(
            State(state.clone()),
            Path(id.to_string()),
            Json(serde_json::json!({ "outcome": "meh" })),
        )
        .await
        .unwrap_err();
        assert_eq!(e, StatusCode::BAD_REQUEST);
        // Unknown run → 404.
        let e = report_run_outcome(
            State(state.clone()),
            Path(uuid::Uuid::new_v4().to_string()),
            Json(serde_json::json!({ "outcome": "done" })),
        )
        .await
        .unwrap_err();
        assert_eq!(e, StatusCode::NOT_FOUND);
        // Happy: done, terminal + completed_at stamped.
        let Json(body) = report_run_outcome(
            State(state.clone()),
            Path(id.to_string()),
            Json(serde_json::json!({ "outcome": "done", "summary": "all green" })),
        )
        .await
        .unwrap();
        assert_eq!(body["run"]["status"], serde_json::json!("done"));
        assert!(body["run"]["completed_at"].is_string(), "completed_at stamped");
        // Terminal runs leave list_active_runs, so completion must enqueue a final
        // federation push — else dojo keeps the stale "running" status.
        assert!(
            state
                .task_queue
                .has_pending_kind_path(crate::tasks::TaskKind::PublishRun, &id.to_string())
                .await,
            "report_run_outcome enqueues a final PublishRun for the terminal status"
        );
        del(&state, &id).await;
    }

    #[tokio::test]
    async fn get_pending_nudges_validates_and_fails_soft_without_dojo() {
        let Some(state) = make_state().await else {
            return;
        };
        // Bad uuid → 400.
        let e = get_pending_nudges(State(state.clone()), Path("nope".into())).await.unwrap_err();
        assert_eq!(e, StatusCode::BAD_REQUEST);
        // Unknown run → 404.
        let e = get_pending_nudges(State(state.clone()), Path(uuid::Uuid::new_v4().to_string()))
            .await
            .unwrap_err();
        assert_eq!(e, StatusCode::NOT_FOUND);
        // A real run with no enrolled dojo → empty nudges (fail-soft), not a 500.
        let id = state.pg.create_run(&NewRun::default()).await.unwrap();
        let Json(body) =
            get_pending_nudges(State(state.clone()), Path(id.to_string())).await.unwrap();
        assert_eq!(body["nudges"], serde_json::json!([]));
        del(&state, &id).await;
    }
}
