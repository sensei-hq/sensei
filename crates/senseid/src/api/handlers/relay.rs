//! Relay lanes slice 1, item 2 — the daemon HTTP surface (phase A part 2).
//!
//! Lane routes under `/api/relay/…` (MCP tools call these; the paths extend
//! the part-1 pins) and person routes exactly as `relay-api.ts` calls them
//! (`/relay/now`, `/relay/queue`, `/relay/tasks/{crew}/{task}`,
//! `/relay/messages`, `/relay/answers`, `/relay/backlog`).
//!
//! URL paths are L3's to publish (Q9). The full table:
//!
//! | Caller | Method + path | Op |
//! |---|---|---|
//! | lane | `POST /api/relay/register` | register (mints the token; no auth) |
//! | lane | `POST /api/relay/heartbeat` | renew the lease |
//! | lane | `POST /api/relay/reregister` | resume with the checkpoint |
//! | lane | `POST /api/relay/unregister` | leave |
//! | lane | `POST /api/relay/limit_pause` | report quota exhaustion |
//! | lane | `GET /api/relay/tasks?lane_id=` | the envelope, now |
//! | lane | `GET /api/relay/tasks/wait?lane_id=&timeout_ms=` | the envelope, held open ≤ 50 s |
//! | lane | `POST /api/relay/tasks/plan` | plan an item (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/assign` | hand to a lane (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/report` | a work/rework turn |
//! | lane | `POST /api/relay/tasks/{id}/review` | a review turn |
//! | lane | `POST /api/relay/decisions` | ask |
//! | lane | `POST /api/relay/decisions/{id}/answer` | answer (lane asked) |
//! | lane | `POST /api/relay/messages` | message (lane sent) |
//! | lane | `POST /api/relay/tasks/{id}/merge` | merge (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/cancel` | cancel (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/handover` | handover (coordinator) |
//! | lane | `POST /api/relay/tasks/{id}/steps` | step update (holder) |
//! | screen | `GET /api/relay/events` | SSE: task + step + relay-op events |
//! | lane | `POST /api/relay/crews/{id}/close` | close the crew (coordinator) |
//! | person | `GET /relay/now`, `GET /relay/queue`, `GET /relay/tasks/{crew}/{task}` | reads |
//! | person | `POST /relay/messages`, `POST /relay/answers`, `POST /relay/backlog` | writes |
//! | person | `POST /relay/triage` | add a manual triage item (derived: nothing
//! | | else creates one in slice 1 — the tracker pull is slice 2) |
//!
//! Lane calls authenticate over the `Authorization: Bearer <lane_token>`
//! header and name their `lane_id` in the query (GET) or body (POST); the
//! token never appears anywhere else. Person calls ride the daemon's
//! localhost trust; the daemon user comes from `relay_current_user` (the
//! stable machine-local id — local identity, Q19 superseded).
//!
//! Phase A: types and reds only — the handlers and the `routes.rs`
//! registration land in phase B, so every red below fails on the missing
//! route (404 with an empty body) or the missing store op, never on setup.
//! Failure tests assert the `relay_*` error code in the body, so none can
//! pass vacuously on the router's bare 404.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
};
use serde::Deserialize;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::api::events::{LaneChange, LaneReason, RunChange, StateEvent};
use crate::api::state::AppState;
use crate::db::pg_store::relay::{
    RelayAnswerInput, RelayAnswerOutput, RelayAskInput, RelayAskedOf, RelayAssignInput,
    RelayCaller, RelayCheckpointInput, RelayClaim, RelayClaimedReview, RelayClaimedTask,
    RelayCreateRunInput, RelayDecisionOption, RelayFileInput, RelayFindingInput, RelayGateInput,
    RelayHandoverInput, RelayHeartbeatOutput, RelayLaneSession, RelayLimitPauseInput,
    RelayMergeInput, RelayMessageInput, RelayMessageTo, RelayPlanInput, RelayReadFilter,
    RelayRegisterInput, RelayReleaseInput, RelayReportInput, RelayResourceRequest,
    RelayReviewInput, RelayStepInput, RelayStepOutput, RelayTicketInput, RelayTriageInput,
};
use crate::relay::envelope::{LaneEnvelope, empty_envelope};
use crate::relay::rows::{
    decision_asked, item_added, item_leaving, message_sent, with_lane_row, with_run_row,
    with_task_row,
};
use crate::relay::{
    DepsStatus, RelayError, RelayFindingSeverity, RelayItemKind, RelayLimitScope, RelayRole,
    RelayStepState, RelaySurface, RelayTaskPhase, RelayTicketProvider, ReviewVerdict,
    clamp_wait_timeout,
};

type RelayResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

fn err(e: RelayError) -> (StatusCode, Json<serde_json::Value>) {
    let status = StatusCode::from_u16(e.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    // Internal detail stays in the log; the wire carries the code only.
    let message = match &e {
        RelayError::Internal(detail) => {
            tracing::warn!(detail = %detail, "relay handler: internal error");
            "internal error".to_string()
        }
        RelayError::Unauthorized(m)
        | RelayError::Forbidden(m)
        | RelayError::NotFound(m)
        | RelayError::Gone(m)
        | RelayError::Conflict(m)
        | RelayError::Validation(m)
        | RelayError::NotImplemented(m)
        | RelayError::DepsUnmet(m)
        | RelayError::PlanInvalid(m) => m.clone(),
        RelayError::SignInRequired => "this machine never signed into a dōjō".to_string(),
        RelayError::NoActiveCrew => {
            "no active crew to join: a coordinator mints one, or name crew_id".to_string()
        }
        RelayError::CrewAmbiguous => "more than one active crew: name crew_id".to_string(),
    };
    (status, Json(serde_json::json!({ "error": e.code(), "message": message })))
}

fn ok(value: serde_json::Value) -> RelayResult {
    Ok(Json(value))
}

/// A `relay_*` word off the wire, or 422 when it names nothing.
fn parse_word<T>(text: &str, what: &str) -> Result<T, RelayError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str::<T>(&format!("\"{text}\""))
        .map_err(|_| RelayError::Validation(format!("unknown {what}: {text}")))
}

fn parse_uuid(text: &str, what: &str) -> Result<Uuid, RelayError> {
    Uuid::parse_str(text).map_err(|_| RelayError::Validation(format!("bad {what}: {text}")))
}

/// A lane call: `lane_id` (query or body) plus the `Authorization: Bearer`
/// token. Unknown tokens and missing headers are 401; a bad lane id is 422.
async fn auth_lane(
    state: &AppState,
    lane_id: &str,
    headers: &HeaderMap,
) -> Result<RelayLaneSession, RelayError> {
    let lane_id = parse_uuid(lane_id, "lane_id")?;
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| RelayError::Unauthorized("missing bearer token".into()))?;
    state.pg.relay_auth_lane(lane_id, token).await
}

/// The person behind a person route: the stable machine-local daemon
/// user, always (local identity, Q19 superseded — no sign-in gate).
async fn person_user(state: &AppState) -> Result<Uuid, RelayError> {
    state.pg.relay_current_user().await
}

fn emit(state: &AppState, op: &str) {
    let _ = state.event_tx.send(StateEvent::relay(op));
}

/// A task changed: the screen re-reads that task's view (F8).
/// The event carries the task's header (#418).
async fn emit_task(state: &AppState, op: &str, task_id: &Uuid) {
    let event = with_task_row(&state.pg, StateEvent::relay_task(op, task_id), task_id).await;
    let _ = state.event_tx.send(event);
}

/// A step as the wire carries it (the step route's answer and the step event).
fn step_json(out: &RelayStepOutput) -> serde_json::Value {
    serde_json::json!({
        "key": out.key,
        "title": out.title,
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
        "reason": out.reason,
        "updated_at": out.updated_at,
    })
}

/// A step was created or restated: the screen applies it without re-reading (F8).
fn emit_step(state: &AppState, task_id: &Uuid, out: &RelayStepOutput) {
    let _ = state.event_tx.send(StateEvent::relay_step(task_id, &step_json(out), out.created));
}

/// A lane's crew membership changed: the Lanes section follows.
/// A run reached a lifecycle point: the event carries its band row (#418).
async fn emit_run(state: &AppState, change: RunChange, run_id: &Uuid, crew_id: &Uuid) {
    let event =
        with_run_row(&state.pg, StateEvent::relay_run(change, run_id, crew_id), run_id).await;
    let _ = state.event_tx.send(event);
}

/// The event carries the lane's Now row (#418); a removal carries its id.
async fn emit_lane(state: &AppState, change: LaneChange, lane_id: &Uuid, crew_id: &Uuid) {
    let removed = matches!(change, LaneChange::Unregistered);
    let event = StateEvent::relay_lane(change, lane_id, crew_id);
    let event = if removed { event } else { with_lane_row(&state.pg, event, lane_id).await };
    let _ = state.event_tx.send(event);
}

// ─── Wire bodies ───

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterBody {
    /// Absent means contributor (#402).
    role: Option<String>,
    /// The name the assistant proposes for itself: the lane's title.
    suggested_name: Option<String>,
    family: String,
    version: Option<String>,
    model: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    repo: Option<String>,
    worktree: Option<String>,
    crew_id: Option<String>,
    goal: Option<String>,
    repo_key: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LaneBody {
    lane_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LimitPauseBody {
    lane_id: String,
    scope: String,
    resumes_at: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ResourceBody {
    lane_id: String,
    resource: String,
}

/// F11/#404: a misspelt or stray query key is refused (400), never ignored.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TasksQuery {
    lane_id: String,
}

/// F11/#404: strict keys; the hold is `timeout_ms`, or `timeout_seconds`
/// when `timeout_ms` is absent — the same clamp either way.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WaitQuery {
    lane_id: String,
    timeout_ms: Option<u64>,
    timeout_seconds: Option<u64>,
}

impl WaitQuery {
    fn hold_ms(&self) -> Option<u64> {
        self.timeout_ms.or(self.timeout_seconds.map(|s| s.saturating_mul(1000)))
    }
}

/// `POST /api/relay/tasks/{id}/checkpoint` (F07): an in-turn save by the
/// holder.
#[derive(Debug, Deserialize)]
pub(crate) struct CheckpointBody {
    lane_id: String,
    current_step: Option<String>,
    notes: Option<String>,
    #[serde(default)]
    steps: Vec<CheckpointStepBody>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CheckpointStepBody {
    key: String,
    title: String,
    state: String,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/relay/tasks/{id}/release`: a claim given back.
#[derive(Debug, Deserialize)]
pub(crate) struct ReleaseBody {
    lane_id: String,
    reason: Option<String>,
}

/// `POST /relay/runs` (person): a sprint run over backlog items.
#[derive(Debug, Deserialize)]
pub(crate) struct CreateRunBody {
    crew_id: Option<String>,
    title: String,
    goal: String,
    #[serde(default)]
    item_ids: Vec<String>,
    repo_key: Option<String>,
    workspace_id: Option<String>,
    config: Option<RunConfigBody>,
}

/// The gate's knobs (F11: an unknown knob is a caller bug, not a silent
/// default).
#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunConfigBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    base_checkout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify_cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_merge: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlanBody {
    lane_id: String,
    item_id: Option<String>,
    title: Option<String>,
    phase: String,
    repo_key: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BriefBody {
    summary: String,
    #[serde(default)]
    docs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TicketBody {
    provider: String,
    key: String,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssignBody {
    lane_id: String,
    to_lane: String,
    title: String,
    phase: String,
    ticket: Option<TicketBody>,
    #[serde(default)]
    tags: Vec<String>,
    brief: BriefBody,
    #[serde(default)]
    acceptance: Vec<String>,
    #[serde(default)]
    owned_paths: Vec<String>,
    #[serde(default)]
    deps: Vec<String>,
    #[serde(default)]
    resources: Vec<String>,
    lane_title: Option<String>,
}

/// `POST /api/relay/tasks/{id}/claim_review` (queue cutover).
#[derive(Debug, Deserialize)]
pub(crate) struct ClaimReviewBody {
    lane_id: String,
}

// ─── Queue cutover (2.9): the claim loop ───

/// What a lane may do on a claim-early task until its deps are done.
const EARLY_CLAIM_GUIDANCE: &str = "An upstream task is not done yet. You may scaffold \
interfaces and write tests against them, but your full report is refused \
(relay_upstream_deps_unmet) until every dependency is done.";

/// The §6.3 `task` shape for a claimed task.
fn claimed_task_json(task: &RelayClaimedTask) -> serde_json::Value {
    let mut out = serde_json::json!({
        "id": task.task_id.to_string(),
        "run_id": task.run_id.map(|id| id.to_string()),
        "title": task.title,
        "phase": serde_json::to_value(task.phase).unwrap_or(serde_json::Value::Null),
        "round": task.round,
        "repo_key": task.repo_key,
        "deps_status": serde_json::to_value(task.deps_status).unwrap_or(serde_json::Value::Null),
        "unmet_deps": task.unmet_deps.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        "owned_paths": task.owned_paths,
        "acceptance": task.acceptance,
        "checkpoint": task.checkpoint,
    });
    if task.deps_status == DepsStatus::EarlyClaimGated {
        out["guidance"] = serde_json::Value::String(EARLY_CLAIM_GUIDANCE.into());
    }
    out
}

/// The §6.3 `task` shape for a claimed review (the `claim_review` route's
/// own answer is the same shape, flat).
fn claimed_review_json(review: &RelayClaimedReview) -> serde_json::Value {
    serde_json::json!({
        "id": review.task_id.to_string(),
        "title": review.title,
        "author_lane_code": review.author_lane_code,
        "phase": serde_json::to_value(review.phase).unwrap_or(serde_json::Value::Null),
        "round": review.round,
        "summary": review.summary,
        "report_md": review.report_md,
        "branch": review.branch,
        "files": review.files,
    })
}

pub(crate) async fn claim_next(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let claim = state
        .pg
        .relay_claim_next(session.user_id, lane_id, chrono::Utc::now())
        .await
        .map_err(err)?;
    let (claim_type, task) = match &claim {
        RelayClaim::Review(review) => {
            emit_task(&state, "claim_review", &review.task_id).await;
            ("review", claimed_review_json(review))
        }
        RelayClaim::Task(task) => {
            emit_task(&state, "claim", &task.task_id).await;
            ("task", claimed_task_json(task))
        }
        RelayClaim::None => ("none", serde_json::Value::Null),
    };
    ok(serde_json::json!({ "claim_type": claim_type, "task": task }))
}

pub(crate) async fn claim_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let task = state
        .pg
        .relay_claim_task(session.user_id, lane_id, task_id, chrono::Utc::now())
        .await
        .map_err(err)?;
    emit_task(&state, "claim", &task_id).await;
    ok(serde_json::json!({ "claim_type": "task", "task": claimed_task_json(&task) }))
}

pub(crate) async fn checkpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<CheckpointBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let steps = body
        .steps
        .into_iter()
        .map(|s| -> Result<RelayStepInput, RelayError> {
            Ok(RelayStepInput {
                key: s.key,
                title: s.title,
                state: parse_word::<RelayStepState>(&s.state, "steps[].state")?,
                reason: s.reason,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let out = state
        .pg
        .relay_checkpoint_task(
            session.user_id,
            lane_id,
            task_id,
            RelayCheckpointInput { current_step: body.current_step, notes: body.notes, steps },
        )
        .await
        .map_err(err)?;
    for step in &out.steps {
        emit_step(&state, &task_id, step);
    }
    emit_task(&state, "checkpoint", &task_id).await;
    ok(serde_json::json!({
        "task_id": task_id.to_string(),
        "checkpointed_at": iso(out.checkpointed_at),
    }))
}

pub(crate) async fn release(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<ReleaseBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let out = state
        .pg
        .relay_release_task(RelayReleaseInput {
            user_id: session.user_id,
            lane_id,
            task_id,
            reason: body.reason,
            now: chrono::Utc::now(),
        })
        .await
        .map_err(err)?;
    emit_task(&state, "release", &task_id).await;
    ok(serde_json::json!({
        "task_id": out.task_id.to_string(),
        "kind": serde_json::to_value(out.kind).unwrap_or(serde_json::Value::Null),
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) async fn limit_resume(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let resumed = state.pg.relay_limit_resume(lane_id).await.map_err(err)?;
    // Only a pause that ended is announced; a no-op stays quiet.
    if resumed {
        emit_lane(
            &state,
            LaneChange::Resumed(LaneReason::LimitPauseEnded),
            &lane_id,
            &session.crew_id,
        )
        .await;
    }
    ok(serde_json::json!({ "resumed": resumed }))
}

/// `GET /api/relay/queue?lane_id=`: the crew's claimable work, read-only.
pub(crate) async fn lane_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TasksQuery>,
) -> RelayResult {
    let session = auth_lane(&state, &query.lane_id, &headers).await.map_err(err)?;
    let queue = state.pg.relay_lane_queue(session.user_id, session.crew_id).await.map_err(err)?;
    let entries: Vec<serde_json::Value> = queue
        .iter()
        .map(|q| {
            serde_json::json!({
                "task_id": q.task_id.to_string(),
                "run_id": q.run_id.map(|id| id.to_string()),
                "title": q.title,
                "phase": serde_json::to_value(q.phase).unwrap_or(serde_json::Value::Null),
                "state": serde_json::to_value(q.state).unwrap_or(serde_json::Value::Null),
                "round": q.round,
                "repo_key": q.repo_key,
                "owned_paths": q.owned_paths,
                "acceptance": q.acceptance,
                "author_lane_id": q.author_lane_id.map(|id| id.to_string()),
                "review_claimed_by": q.review_claimed_by.map(|id| id.to_string()),
                "claim_eligibility": q.claim_eligibility,
                "unmet_deps": q.unmet_deps.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                "state_changed_at": iso(q.state_changed_at),
            })
        })
        .collect();
    ok(serde_json::json!({ "queue": entries }))
}

/// `POST /relay/runs` (person): 201 with the run, Task 0, and the crew.
pub(crate) async fn create_run(
    State(state): State<AppState>,
    Json(body): Json<CreateRunBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let user_id = person_user(&state).await.map_err(err)?;
    let item_ids = body
        .item_ids
        .iter()
        .map(|id| parse_uuid(id, "item_ids[]"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    // Read before the write: each remove names the id the screen holds.
    let mut leaving = Vec::with_capacity(item_ids.len());
    for item_id in &item_ids {
        leaving.extend(item_leaving(&state.pg, item_id, "planned").await);
    }
    let config = match body.config {
        Some(config) => serde_json::to_value(config)
            .map_err(|e| err(RelayError::Internal(format!("run config: {e}"))))?,
        None => serde_json::json!({}),
    };
    let out = state
        .pg
        .relay_create_run(
            user_id,
            RelayCreateRunInput {
                crew_id: body
                    .crew_id
                    .as_deref()
                    .map(|c| parse_uuid(c, "crew_id"))
                    .transpose()
                    .map_err(err)?,
                title: body.title,
                goal: body.goal,
                item_ids,
                repo_key: body.repo_key,
                workspace_id: body
                    .workspace_id
                    .as_deref()
                    .map(|w| parse_uuid(w, "workspace_id"))
                    .transpose()
                    .map_err(err)?,
                config,
            },
            chrono::Utc::now(),
        )
        .await
        .map_err(err)?;
    emit_run(&state, RunChange::Created, &out.run_id, &out.crew_id).await;
    // The picked items leave the queue (#418).
    for event in leaving {
        let _ = state.event_tx.send(event);
    }
    emit_task(&state, "plan", &out.planning_task_id).await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "run_id": out.run_id.to_string(),
            "planning_task_id": out.planning_task_id.to_string(),
            "crew_id": out.crew_id.to_string(),
            "status": "planning",
        })),
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct SummaryBody {
    changed: String,
    look_at: Option<String>,
    flags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileBody {
    path: String,
    added: i64,
    removed: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GateBody {
    cmd: String,
    exit: i32,
    counts: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReportBody {
    lane_id: String,
    phase: String,
    round: i32,
    /// F1: partial progress — records the action, moves nothing. Omitted
    /// (or explicit null) means a full report.
    partial: Option<bool>,
    summary: SummaryBody,
    report_md: Option<String>,
    base: Option<String>,
    branch: Option<String>,
    #[serde(default)]
    commits: Vec<String>,
    #[serde(default)]
    files: Vec<FileBody>,
    #[serde(default)]
    gates: Vec<GateBody>,
    #[serde(default)]
    telemetry: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FindingBody {
    severity: String,
    class: String,
    text: String,
    file: Option<String>,
    line: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewBody {
    lane_id: String,
    verdict: String,
    summary: String,
    #[serde(default)]
    findings: Vec<FindingBody>,
    probes: Option<i64>,
    #[serde(default)]
    telemetry: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OptionBody {
    id: String,
    label: String,
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AskBody {
    lane_id: String,
    task_id: Option<String>,
    question: String,
    context: Option<String>,
    #[serde(default)]
    options: Vec<OptionBody>,
    recommended: Option<String>,
    #[serde(default)]
    blocks: Vec<String>,
    asked_of: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LaneAnswerBody {
    lane_id: String,
    option: String,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PersonAnswerBody {
    decision_id: String,
    option: String,
    note: Option<String>,
    surface: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ToBody {
    kind: String,
    lane_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PersonMessageBody {
    crew_id: Option<String>,
    to: ToBody,
    task_id: Option<String>,
    text: String,
    surface: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LaneMessageBody {
    lane_id: String,
    to: ToBody,
    task_id: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MergeBody {
    lane_id: String,
    #[serde(default)]
    commits: Vec<String>,
    into: String,
    #[serde(default)]
    gates: Vec<GateBody>,
    note: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HandoverBody {
    lane_id: String,
    to_lane: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StepBody {
    lane_id: String,
    key: String,
    title: String,
    state: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReadQuery {
    workspace_id: Option<String>,
    repo_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TriageBody {
    title: String,
    kind: String,
    repo_key: Option<String>,
    workspace_id: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BacklogBody {
    item_id: Option<String>,
    ticket_provider: Option<String>,
    ticket_key: Option<String>,
}

fn read_filter(query: &ReadQuery) -> Result<RelayReadFilter, RelayError> {
    Ok(RelayReadFilter {
        workspace_id: query
            .workspace_id
            .as_deref()
            .map(|w| parse_uuid(w, "workspace_id"))
            .transpose()?,
        repo_key: query.repo_key.clone(),
    })
}

fn message_to(to: &ToBody) -> Result<RelayMessageTo, RelayError> {
    match to.kind.as_str() {
        "coordinator" => Ok(RelayMessageTo::Coordinator),
        "everyone" => Ok(RelayMessageTo::Everyone),
        "person" => Ok(RelayMessageTo::Person),
        "lane" => {
            let lane_id = to.lane_id.as_deref().ok_or_else(|| {
                RelayError::Validation("to.lane_id is required for kind lane".into())
            })?;
            Ok(RelayMessageTo::Lane { lane_id: parse_uuid(lane_id, "to.lane_id")? })
        }
        other => Err(RelayError::Validation(format!("unknown to.kind {other}"))),
    }
}

fn iso(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

// ─── Lane routes ───

pub(crate) async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let out = state
        .pg
        .relay_register(
            user_id,
            RelayRegisterInput {
                role: match body.role.as_deref() {
                    Some(role) => parse_word::<RelayRole>(role, "role").map_err(err)?,
                    None => RelayRole::Contributor,
                },
                family: body.family,
                version: body.version,
                model: body.model,
                capabilities: body.capabilities,
                repo: body.repo,
                worktree: body.worktree,
                crew_id: body
                    .crew_id
                    .as_deref()
                    .map(|c| parse_uuid(c, "crew_id"))
                    .transpose()
                    .map_err(err)?,
                goal: body.goal,
                repo_key: body.repo_key,
                workspace_id: body
                    .workspace_id
                    .as_deref()
                    .map(|w| parse_uuid(w, "workspace_id"))
                    .transpose()
                    .map_err(err)?,
                suggested_name: body.suggested_name,
            },
            chrono::Utc::now(),
        )
        .await
        .map_err(err)?;
    emit_lane(&state, LaneChange::Registered, &out.lane_id, &out.crew_id).await;
    ok(serde_json::json!({
        "lane_token": out.lane_token,
        "lane_id": out.lane_id.to_string(),
        "lane_title": out.lane_title,
        "crew_id": out.crew_id.to_string(),
        "guidance": {
            "version": out.guidance.version,
            "generic": out.guidance.generic,
            "family_notes": out.guidance.family_notes,
        },
    }))
}

pub(crate) async fn request_resource(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ResourceBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let out = state
        .pg
        .relay_request_resource(
            session.user_id,
            caller,
            RelayResourceRequest { resource: body.resource.clone(), floors: None },
            chrono::Utc::now(),
        )
        .await
        .map_err(err)?;
    emit(&state, "resources.request");
    ok(serde_json::json!({
        "lease_id": out.lease_id.to_string(),
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
        "resource": out.resource,
        "disk_free_gb": out.disk_free_gb,
        "swap_free_gb": out.swap_free_gb,
        "disk_floor_gb": out.disk_floor_gb,
        "swap_floor_gb": out.swap_floor_gb,
        "granted_at": out.granted_at.map(iso),
        "expires_at": out.expires_at.map(iso),
        "position": out.position,
    }))
}

pub(crate) async fn release_resource(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ResourceBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let out = state
        .pg
        .relay_release_resource(session.user_id, caller, &body.resource, chrono::Utc::now())
        .await
        .map_err(err)?;
    emit(&state, "resources.release");
    ok(serde_json::json!({
        "lease_id": out.lease_id.to_string(),
        "resource": out.resource,
        "state": "released",
        "granted_next": out.granted_next.map(|id| id.to_string()),
    }))
}

pub(crate) async fn heartbeat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let RelayHeartbeatOutput { expires_at, regranted } =
        state.pg.relay_heartbeat(lane_id, chrono::Utc::now()).await.map_err(err)?;
    // A routine renewal changes nothing the screen shows; only a beat
    // that brings the lane back to work is announced. A beat never ends a
    // limit pause (F03): the sweeper announces that at resumes_at.
    if regranted {
        emit_lane(
            &state,
            LaneChange::Resumed(LaneReason::LeaseRegranted),
            &lane_id,
            &session.crew_id,
        )
        .await;
    }
    ok(serde_json::json!({ "expires_at": iso(expires_at) }))
}

pub(crate) async fn reregister(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    // `Gone` is the resume path (valid token, unregistered lane); a live
    // lane resumes idempotently. Only `Unauthorized` refuses.
    match auth_lane(&state, &body.lane_id, &headers).await {
        Ok(_) | Err(RelayError::Gone(_)) => {}
        Err(e) => return Err(err(e)),
    }
    let out = state
        .pg
        .relay_reregister(parse_uuid(&body.lane_id, "lane_id").map_err(err)?, chrono::Utc::now())
        .await
        .map_err(err)?;
    emit_lane(&state, LaneChange::Reregistered, &out.lane_id, &out.crew_id).await;
    ok(serde_json::json!({
        "lane_id": out.lane_id.to_string(),
        "crew_id": out.crew_id.to_string(),
        "checkpoint": out.checkpoint,
        "expires_at": iso(out.expires_at),
    }))
}

pub(crate) async fn unregister(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let out = state.pg.relay_unregister(lane_id).await.map_err(err)?;
    emit(&state, "unregister");
    emit_lane(&state, LaneChange::Unregistered, &lane_id, &session.crew_id).await;
    if let Some(task_id) = out.interrupted {
        emit_task(&state, "interrupt", &task_id).await;
    }
    ok(serde_json::json!({ "lane_id": body.lane_id }))
}

pub(crate) async fn limit_pause(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LimitPauseBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let resumes_at = body
        .resumes_at
        .as_deref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.to_utc())
                .map_err(|_| RelayError::Validation(format!("bad resumes_at: {s}")))
        })
        .transpose()
        .map_err(err)?;
    let out = state
        .pg
        .relay_report_limit_pause(
            lane_id,
            RelayLimitPauseInput {
                scope: parse_word::<RelayLimitScope>(&body.scope, "scope").map_err(err)?,
                resumes_at,
                note: body.note,
            },
            chrono::Utc::now(),
        )
        .await
        .map_err(err)?;
    emit_lane(&state, LaneChange::Suspended(LaneReason::LimitPause), &lane_id, &session.crew_id)
        .await;
    ok(serde_json::json!({
        "scope": out.scope,
        "resumes_at": out.resumes_at.map(iso),
    }))
}

type EnvelopeResult = Result<Json<LaneEnvelope>, (StatusCode, Json<serde_json::Value>)>;

// The envelope answers serialize the STRUCT, never a `Value`: only the
// struct keeps the ruling's key order (messages, answers, reviews,
// assignments) — a `Value` map sorts its keys.

pub(crate) async fn get_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TasksQuery>,
) -> EnvelopeResult {
    let session = auth_lane(&state, &query.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&query.lane_id, "lane_id").map_err(err)?;
    let envelope = state.pg.relay_envelope(session.user_id, lane_id).await.map_err(err)?;
    Ok(Json(envelope))
}

pub(crate) async fn wait_for_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WaitQuery>,
) -> EnvelopeResult {
    let session = auth_lane(&state, &query.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&query.lane_id, "lane_id").map_err(err)?;
    let hold = clamp_wait_timeout(query.hold_ms());
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(hold);
    // F06: the hold is a waiter row for its duration — opened before the
    // first read, released on every exit path (the sweeper nudges by it
    // and evicts dead lanes' rows).
    state.pg.relay_waiter_hold(session.user_id, lane_id, session.crew_id).await.map_err(err)?;
    let held = wait_held(&state, &session, lane_id, deadline).await;
    let released = state.pg.relay_waiter_release(session.user_id, lane_id).await.map_err(err);
    let envelope = held?;
    released?;
    Ok(Json(envelope))
}

/// The hold itself: re-read on every relay event until the envelope has
/// something or the deadline passes.
async fn wait_held(
    state: &AppState,
    session: &RelayLaneSession,
    lane_id: Uuid,
    deadline: std::time::Instant,
) -> Result<LaneEnvelope, (StatusCode, Json<serde_json::Value>)> {
    // Subscribe before the first read: an event between the read and the
    // subscribe would otherwise sleep through the hold.
    let mut rx = state.event_tx.subscribe();
    loop {
        let envelope = state.pg.relay_envelope(session.user_id, lane_id).await.map_err(err)?;
        if !(envelope.messages.is_empty()
            && envelope.answers.is_empty()
            && envelope.reviews.is_empty()
            && envelope.assignments.is_empty())
        {
            return Ok(envelope);
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Ok(empty_envelope());
        }
        // Any event (or lag, or a closed sender) re-checks; only the
        // deadline returns empty. Spurious wakes are a re-read, never wrong.
        let _ = tokio::time::timeout(remaining, rx.recv()).await;
    }
}

pub(crate) async fn plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PlanBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let out = state
        .pg
        .relay_plan_item(
            session.user_id,
            caller,
            RelayPlanInput {
                item_id: body
                    .item_id
                    .as_deref()
                    .map(|i| parse_uuid(i, "item_id"))
                    .transpose()
                    .map_err(err)?,
                title: body.title,
                phase: parse_word::<RelayTaskPhase>(&body.phase, "phase").map_err(err)?,
                repo_key: body.repo_key,
                workspace_id: body
                    .workspace_id
                    .as_deref()
                    .map(|w| parse_uuid(w, "workspace_id"))
                    .transpose()
                    .map_err(err)?,
            },
        )
        .await
        .map_err(err)?;
    ok(serde_json::json!({ "task_id": out.task_id.to_string() }))
}

pub(crate) async fn assign(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<AssignBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let deps = body
        .deps
        .iter()
        .map(|d| parse_uuid(d, "deps[]"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let out = state
        .pg
        .relay_assign_task(
            session.user_id,
            caller,
            task_id,
            RelayAssignInput {
                to_lane: parse_uuid(&body.to_lane, "to_lane").map_err(err)?,
                title: body.title,
                phase: parse_word::<RelayTaskPhase>(&body.phase, "phase").map_err(err)?,
                ticket: body
                    .ticket
                    .map(|t| -> Result<RelayTicketInput, RelayError> {
                        Ok(RelayTicketInput {
                            provider: parse_word::<RelayTicketProvider>(
                                &t.provider,
                                "ticket.provider",
                            )?,
                            key: t.key,
                            url: t.url,
                        })
                    })
                    .transpose()
                    .map_err(err)?,
                tags: body.tags,
                brief_summary: body.brief.summary,
                brief_docs: body.brief.docs,
                acceptance: body.acceptance,
                owned_paths: body.owned_paths,
                deps,
                resources: body.resources,
                lane_title: body.lane_title,
                now: chrono::Utc::now(),
            },
        )
        .await
        .map_err(err)?;
    emit_task(&state, "assign", &task_id).await;
    ok(serde_json::json!({
        "task_id": out.task_id.to_string(),
        "round": out.round,
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) async fn report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<ReportBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let out = state
        .pg
        .relay_report_task(
            session.user_id,
            caller,
            task_id,
            RelayReportInput {
                phase: parse_word::<RelayTaskPhase>(&body.phase, "phase").map_err(err)?,
                round: body.round,
                partial: body.partial.unwrap_or(false),
                changed: body.summary.changed,
                look_at: body.summary.look_at,
                flags: body.summary.flags,
                report_md: body.report_md,
                base: body.base,
                branch: body.branch,
                commits: body.commits,
                files: body
                    .files
                    .into_iter()
                    .map(|f| RelayFileInput { path: f.path, added: f.added, removed: f.removed })
                    .collect(),
                gates: body
                    .gates
                    .into_iter()
                    .map(|g| RelayGateInput { cmd: g.cmd, exit: g.exit, counts: g.counts })
                    .collect(),
                telemetry: if body.telemetry.is_null() {
                    serde_json::json!({})
                } else {
                    body.telemetry
                },
            },
        )
        .await
        .map_err(err)?;
    emit_task(&state, "report", &task_id).await;
    ok(serde_json::json!({
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
        "round": out.round,
    }))
}

pub(crate) async fn claim_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<ClaimReviewBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let out = state
        .pg
        .relay_claim_review(session.user_id, caller, task_id, chrono::Utc::now())
        .await
        .map_err(err)?;
    emit_task(&state, "claim_review", &task_id).await;
    ok(serde_json::json!({
        "task_id": out.task_id.to_string(),
        "title": out.title,
        "phase": serde_json::to_value(out.phase).unwrap_or(serde_json::Value::Null),
        "round": out.round,
        "author_lane_code": out.author_lane_code,
        "summary": out.summary,
        "report_md": out.report_md,
        "branch": out.branch,
        "files": out.files,
    }))
}

pub(crate) async fn review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<ReviewBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let findings = body
        .findings
        .into_iter()
        .map(|f| -> Result<RelayFindingInput, RelayError> {
            Ok(RelayFindingInput {
                severity: parse_word::<RelayFindingSeverity>(&f.severity, "findings[].severity")?,
                class: f.class,
                text: f.text,
                file: f.file,
                line: f.line,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let out = state
        .pg
        .relay_submit_review(
            session.user_id,
            caller,
            task_id,
            RelayReviewInput {
                verdict: parse_word::<ReviewVerdict>(&body.verdict, "verdict").map_err(err)?,
                summary: body.summary,
                findings,
                probes: body.probes,
                telemetry: if body.telemetry.is_null() {
                    serde_json::json!({})
                } else {
                    body.telemetry
                },
            },
        )
        .await
        .map_err(err)?;
    emit_task(&state, "review", &task_id).await;
    // An approved Task 0 unpacked its plan: the queue grew and the run is
    // active — both announced beside the review itself.
    if let Some(run_id) = out.run_activated {
        emit_task(&state, "unpack", &task_id).await;
        emit_run(&state, RunChange::Activated, &run_id, &session.crew_id).await;
    }
    ok(serde_json::json!({
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
        "round": out.round,
    }))
}

pub(crate) async fn ask(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AskBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let asked_of = if body.asked_of == "person" {
        RelayAskedOf::Person
    } else {
        RelayAskedOf::Lane(parse_uuid(&body.asked_of, "asked_of").map_err(err)?)
    };
    let blocks = body
        .blocks
        .iter()
        .map(|b| parse_uuid(b, "blocks[]"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let task_id =
        body.task_id.as_deref().map(|t| parse_uuid(t, "task_id")).transpose().map_err(err)?;
    let out = state
        .pg
        .relay_ask_decision(
            session.user_id,
            caller,
            RelayAskInput {
                task_id,
                question: body.question,
                context: body.context,
                options: body
                    .options
                    .into_iter()
                    .map(|o| RelayDecisionOption { id: o.id, label: o.label, detail: o.detail })
                    .collect(),
                recommended: body.recommended,
                blocks,
                asked_of,
            },
        )
        .await
        .map_err(err)?;
    match task_id {
        Some(task_id) => emit_task(&state, "ask", &task_id).await,
        None => emit(&state, "ask"),
    }
    // Asked of the person: Needs you gains the row (#418).
    if let Some(event) = decision_asked(&state.pg, &out.decision_id, &session.crew_id).await {
        let _ = state.event_tx.send(event);
    }
    ok(serde_json::json!({ "decision_id": out.decision_id.to_string() }))
}

pub(crate) async fn lane_answer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(decision_id): Path<String>,
    Json(body): Json<LaneAnswerBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let decision_id = parse_uuid(&decision_id, "decision_id").map_err(err)?;
    let out: RelayAnswerOutput = state
        .pg
        .relay_answer_decision(
            session.user_id,
            RelayCaller::Lane { lane_id: caller },
            decision_id,
            RelayAnswerInput { option: body.option, note: body.note },
        )
        .await
        .map_err(err)?;
    match out.task_id {
        Some(task_id) => emit_task(&state, "answer", &task_id).await,
        None => emit(&state, "answer"),
    }
    ok(serde_json::json!({ "action_id": out.action_id.to_string() }))
}

pub(crate) async fn lane_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneMessageBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id =
        body.task_id.as_deref().map(|t| parse_uuid(t, "task_id")).transpose().map_err(err)?;
    let to = message_to(&body.to).map_err(err)?;
    let out = state
        .pg
        .relay_send_message(
            session.user_id,
            RelayCaller::Lane { lane_id: caller },
            RelayMessageInput { to: to.clone(), task_id, text: body.text },
        )
        .await
        .map_err(err)?;
    match task_id {
        Some(task_id) => emit_task(&state, "message", &task_id).await,
        None => emit(&state, "message"),
    }
    // To the person: Needs you gains the row (#418).
    if matches!(to, RelayMessageTo::Person)
        && let Some(event) = message_sent(&state.pg, &out.message_id, &session.crew_id).await
    {
        let _ = state.event_tx.send(event);
    }
    ok(serde_json::json!({ "message_id": out.message_id.to_string() }))
}

pub(crate) async fn merge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<MergeBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    state
        .pg
        .relay_merge_task(
            session.user_id,
            caller,
            task_id,
            RelayMergeInput {
                commits: body.commits,
                into: body.into,
                gates: body
                    .gates
                    .into_iter()
                    .map(|g| RelayGateInput { cmd: g.cmd, exit: g.exit, counts: g.counts })
                    .collect(),
                note: body.note,
            },
        )
        .await
        .map_err(err)?;
    emit_task(&state, "merge", &task_id).await;
    ok(serde_json::json!({ "task_id": task_id.to_string() }))
}

pub(crate) async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let out = state.pg.relay_cancel_task(session.user_id, caller, task_id).await.map_err(err)?;
    emit_task(&state, "cancel", &task_id).await;
    ok(serde_json::json!({
        "task_id": out.task_id.to_string(),
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
    }))
}

pub(crate) async fn handover(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<HandoverBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let to_lane = parse_uuid(&body.to_lane, "to_lane").map_err(err)?;
    let out = state
        .pg
        .relay_handover_task(
            session.user_id,
            caller,
            task_id,
            RelayHandoverInput { to_lane, note: body.note },
        )
        .await
        .map_err(err)?;
    emit_task(&state, "handover", &task_id).await;
    ok(serde_json::json!({
        "task_id": out.task_id.to_string(),
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
        "round": out.round,
    }))
}

pub(crate) async fn post_step(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(body): Json<StepBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let step_state = parse_word::<RelayStepState>(&body.state, "state").map_err(err)?;
    let out = state
        .pg
        .relay_post_step(
            session.user_id,
            caller,
            task_id,
            RelayStepInput {
                key: body.key,
                title: body.title,
                state: step_state,
                reason: body.reason,
            },
        )
        .await
        .map_err(err)?;
    emit_step(&state, &task_id, &out);
    ok(serde_json::json!({
        "task_id": task_id.to_string(),
        "step": step_json(&out),
    }))
}

/// `GET /api/relay/events` — SSE stream of the relay family's events
/// (F8): task notifications carry the task id (the screen re-reads the
/// view), step events carry the step itself (applied without re-reading),
/// op-only relay events cover the rest (unregister interrupts). Same
/// BroadcastStream-over-`event_tx` shape as the scan socket.
///
/// `?crew_id=` narrows the stream to one crew (#418): an event naming
/// another crew is dropped; one naming no crew (a relay op, or a task whose
/// row could not be read) still arrives, and the screen re-reads its own
/// crew's view on it. A malformed
/// id is a 400 from the query extractor.
pub(crate) async fn relay_events_sse(
    State(state): State<AppState>,
    Query(filter): Query<RelayEventsFilter>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |result| match result {
        Ok(event)
            if StateEvent::is_relay_family(&event.entity)
                && event.admits_crew(filter.crew_id.as_ref()) =>
        {
            let data = serde_json::to_string(&event).unwrap_or_default();
            Some(Ok(Event::default().data(data)))
        }
        _ => None,
    });
    Sse::new(stream)
}

#[derive(Debug, Deserialize)]
pub(crate) struct RelayEventsFilter {
    crew_id: Option<Uuid>,
}

pub(crate) async fn close_crew(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(crew_id): Path<String>,
    Json(body): Json<LaneBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let crew_id = parse_uuid(&crew_id, "crew_id").map_err(err)?;
    let out = state.pg.relay_close_crew(session.user_id, caller, crew_id).await.map_err(err)?;
    // A crew's lifecycle event: the crew id stands as the run id (§3k).
    emit_run(&state, RunChange::Completed, &out.crew_id, &out.crew_id).await;
    ok(serde_json::json!({
        "crew_id": out.crew_id.to_string(),
        "status": out.status,
    }))
}

// ─── Person routes ───

pub(crate) async fn now(
    State(state): State<AppState>,
    Query(query): Query<ReadQuery>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let now = state.pg.relay_now(user_id, read_filter(&query).map_err(err)?).await.map_err(err)?;
    ok(now)
}

pub(crate) async fn queue(
    State(state): State<AppState>,
    Query(query): Query<ReadQuery>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let queue =
        state.pg.relay_queue(user_id, read_filter(&query).map_err(err)?).await.map_err(err)?;
    ok(queue)
}

pub(crate) async fn task_view(
    State(state): State<AppState>,
    Path((crew_id, task_id)): Path<(String, String)>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let crew_id = parse_uuid(&crew_id, "crew_id").map_err(err)?;
    let task_id = parse_uuid(&task_id, "task_id").map_err(err)?;
    let view = state.pg.relay_task_view(user_id, crew_id, task_id).await.map_err(err)?;
    ok(view)
}

pub(crate) async fn person_message(
    State(state): State<AppState>,
    Json(body): Json<PersonMessageBody>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    // `crew_id` is the viewing context, accepted but unscoped in slice 1:
    // delivery is by address (the store derives the row's crew).
    let _ = body.crew_id.as_deref().map(|c| parse_uuid(c, "crew_id")).transpose().map_err(err)?;
    let surface = parse_word::<RelaySurface>(&body.surface, "surface").map_err(err)?;
    let task_id =
        body.task_id.as_deref().map(|t| parse_uuid(t, "task_id")).transpose().map_err(err)?;
    let out = state
        .pg
        .relay_send_message(
            user_id,
            RelayCaller::Person { surface },
            RelayMessageInput { to: message_to(&body.to).map_err(err)?, task_id, text: body.text },
        )
        .await
        .map_err(err)?;
    match task_id {
        Some(task_id) => emit_task(&state, "message", &task_id).await,
        None => emit(&state, "message"),
    }
    ok(serde_json::json!({ "message_id": out.message_id.to_string() }))
}

pub(crate) async fn person_answer(
    State(state): State<AppState>,
    Json(body): Json<PersonAnswerBody>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let surface = parse_word::<RelaySurface>(&body.surface, "surface").map_err(err)?;
    let decision_id = parse_uuid(&body.decision_id, "decision_id").map_err(err)?;
    let out: RelayAnswerOutput = state
        .pg
        .relay_answer_decision(
            user_id,
            RelayCaller::Person { surface },
            decision_id,
            RelayAnswerInput { option: body.option, note: body.note },
        )
        .await
        .map_err(err)?;
    match out.task_id {
        Some(task_id) => emit_task(&state, "answer", &task_id).await,
        None => emit(&state, "answer"),
    }
    // Answered: the row leaves Needs you (#418).
    let _ = state.event_tx.send(StateEvent::relay_decision(true, &decision_id, &out.crew_id));
    ok(serde_json::json!({ "action_id": out.action_id.to_string() }))
}

pub(crate) async fn triage(
    State(state): State<AppState>,
    Json(body): Json<TriageBody>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let surface = RelaySurface::Desktop;
    let id = state
        .pg
        .relay_triage_add(
            user_id,
            RelayCaller::Person { surface },
            RelayTriageInput {
                title: body.title,
                kind: parse_word::<RelayItemKind>(&body.kind, "kind").map_err(err)?,
                repo_key: body.repo_key,
                workspace_id: body
                    .workspace_id
                    .as_deref()
                    .map(|w| parse_uuid(w, "workspace_id"))
                    .transpose()
                    .map_err(err)?,
                tags: body.tags,
            },
        )
        .await
        .map_err(err)?;
    // The queue gains the row (#418).
    if let Some(event) = item_added(&state.pg, &id, "triaged").await {
        let _ = state.event_tx.send(event);
    }
    ok(serde_json::json!({ "item_id": id.to_string() }))
}

pub(crate) async fn backlog(
    State(state): State<AppState>,
    Json(body): Json<BacklogBody>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let surface = RelaySurface::Desktop;
    match (body.item_id, body.ticket_provider, body.ticket_key) {
        (Some(item_id), None, None) => {
            let item_id = parse_uuid(&item_id, "item_id").map_err(err)?;
            // Read before the write: the remove names the id the screen holds.
            let leaving = item_leaving(&state.pg, &item_id, "backlogged").await;
            let id = state
                .pg
                .relay_backlog_add(
                    user_id,
                    RelayCaller::Person { surface },
                    item_id,
                )
                .await
                .map_err(err)?;
            // Out of triage, into the backlog with its row (#418).
            let added = item_added(&state.pg, &id, "backlogged").await;
            for event in leaving.into_iter().chain(added) {
                let _ = state.event_tx.send(event);
            }
            ok(serde_json::json!({ "item_id": id.to_string() }))
        }
        (None, Some(_), Some(_)) => Err(err(RelayError::NotImplemented(
            "the tracker pull is slice 2; a ticketed add names no title the daemon could honestly store".into(),
        ))),
        _ => Err(err(RelayError::Validation("send item_id or a ticket, not both or neither".into()))),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    struct Harness {
        app: axum::Router,
        state: crate::api::state::AppState,
    }

    async fn harness() -> Harness {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let state = ctx.app_state.clone();
        // A linked default persona is present — and must not matter: the
        // daemon user is machine-local (local identity, user ruling
        // 2026-11-09; Q19 superseded). The whole suite running green on
        // this harness is the linked-state arm of that proof.
        sqlx_core::query::query(
            "insert into sensei.personas (label, is_chosen_default, principal_id)
             values ('default', true, $1)",
        )
        .bind(uuid::Uuid::now_v7())
        .execute(state.pg.pool())
        .await
        .unwrap();
        Harness { app: crate::api::routes::create_router(state.clone()), state }
    }

    /// The same daemon on an unlinked machine: zero personas rows. Local
    /// identity resolves the machine-local user anyway — no 401.
    async fn harness_bare() -> Harness {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let state = ctx.app_state.clone();
        Harness { app: crate::api::routes::create_router(state.clone()), state }
    }

    /// One request through the real router, from this machine by default.
    async fn call(
        h: &Harness,
        method: &str,
        uri: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut b = Request::builder().method(method).uri(uri).header("host", "127.0.0.1:7744");
        if let Some(t) = token {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        let req = match body {
            Some(v) => b
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        };
        let resp = h.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json_body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json_body)
    }

    fn lane_body(lane_id: &str, extra: Value) -> Value {
        let mut obj = extra.as_object().cloned().unwrap_or_default();
        obj.insert("lane_id".into(), json!(lane_id));
        Value::Object(obj)
    }

    /// Register a coordinator (minting its crew) and return
    /// (lane_id, token, crew_id). Every lane-flow test starts here.
    async fn register_coordinator(h: &Harness) -> (String, String, String) {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "claude-code"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (
            body["lane_id"].as_str().unwrap().to_string(),
            body["lane_token"].as_str().unwrap().to_string(),
            body["crew_id"].as_str().unwrap().to_string(),
        )
    }

    async fn register_contributor(h: &Harness, crew_id: &str) -> (String, String) {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "contributor", "family": "claude-code", "crew_id": crew_id})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (
            body["lane_id"].as_str().unwrap().to_string(),
            body["lane_token"].as_str().unwrap().to_string(),
        )
    }

    #[tokio::test]
    async fn register_on_unlinked_machine_succeeds() {
        // Local identity (user ruling 2026-11-09; Q19 superseded):
        // register mints on the machine-local user — zero personas rows,
        // no relay_signin_required.
        let h = harness_bare().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "test"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["lane_id"].is_string(), "{body}");
    }

    #[tokio::test]
    async fn person_now_on_unlinked_machine_succeeds() {
        // Local identity (user ruling 2026-11-09; Q19 superseded): the
        // person routes share the one choke point (`person_user`), so one
        // route proving 200 unlinked proves the shape — the rest is the
        // same call.
        let h = harness_bare().await;
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    /// Coordinator plans + assigns a tests-phase task to the lane; returns
    /// the task id.
    async fn assigned_task(h: &Harness, coord: &(String, String, String), lane_id: &str) -> String {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": "T", "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let task_id = body["task_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "tests",
                    "brief": {"summary": "Do it.", "docs": []},
                    "acceptance": ["green"], "owned_paths": ["app/**"],
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        task_id
    }

    // ── register / heartbeat ──

    #[tokio::test]
    async fn register_mint_shape_q16rec() {
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "claude-code", "goal": "Ship"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // Q16-REC: the mint response carries the crew for the paste line.
        assert!(body["lane_token"].is_string());
        assert!(body["lane_id"].is_string());
        // F2: register mints the lane title (code + family), never null.
        assert_eq!(body["lane_title"], "L1 · claude-code");
        assert!(body["crew_id"].is_string());
        assert!(uuid::Uuid::parse_str(body["crew_id"].as_str().unwrap()).is_ok());
    }

    #[tokio::test]
    async fn register_mints_title_each_role_and_census_reflects() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "contributor", "family": "muse", "crew_id": coord.2})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // F2: the minted title carries the lane code, each role.
        assert_eq!(body["lane_title"], "L2 · muse");
        // …and census reflects both persisted titles.
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        let titles: Vec<&str> =
            now["lanes"].as_array().unwrap().iter().map(|l| l["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"L1 · claude-code"), "{titles:?}");
        assert!(titles.contains(&"L2 · muse"), "{titles:?}");
    }

    #[tokio::test]
    async fn register_empty_family_title_is_bare_code() {
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": ""})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // F2: no dangling separator on an empty family.
        assert_eq!(body["lane_title"], "L1");
    }

    #[tokio::test]
    async fn register_contributor_without_crew_joins_the_single_active_crew_or_is_409() {
        // #402 (queue cutover): a contributor naming no crew joins the
        // user's single active crew; with none to join it is a 409 of
        // its own code, never a guess.
        let h = harness().await;
        let body = json!({"role": "contributor", "family": "claude-code"});
        let (status, out) = call(&h, "POST", "/api/relay/register", Some(body.clone()), None).await;
        assert_eq!(status, StatusCode::CONFLICT, "{out}");
        assert_eq!(out["error"], "relay_no_active_crew");
        let coord = register_coordinator(&h).await;
        let (status, out) = call(&h, "POST", "/api/relay/register", Some(body), None).await;
        assert_eq!(status, StatusCode::OK, "{out}");
        assert_eq!(out["crew_id"], coord.2);
    }

    #[tokio::test]
    async fn resources_request_shape_matches_verdict() {
        // The verdict follows the machine (ambient readings vs the ruled
        // floors — no wire seam by design, decision 7), so the test pins
        // the full wire shape for whichever verdict came back. The handler
        // maps states uniformly (one json! block, no per-state branch), so
        // a mapping regression dies on the common granted branch.
        let h = harness().await;
        let (lane_id, token, _) = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/resources/request",
            Some(json!({"lane_id": lane_id, "resource": "rust-build"})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["resource"], "rust-build");
        assert!(body["lease_id"].is_string(), "{body}");
        assert!(body["disk_floor_gb"].as_u64() == Some(25), "{body}");
        assert!(body["swap_floor_gb"].as_u64() == Some(1), "{body}");
        assert!(body.get("swap_free_gb").is_some(), "swap key present: {body}");
        assert!(body["position"].is_null(), "position only while queued: {body}");
        match body["state"].as_str() {
            Some("granted") => {
                assert!(body["disk_free_gb"].is_number(), "{body}");
                assert!(body["granted_at"].is_string(), "{body}");
                assert!(body["expires_at"].is_string(), "{body}");
            }
            Some("refused") => {
                assert!(body["disk_free_gb"].is_number(), "{body}");
                assert!(body["granted_at"].is_null(), "{body}");
                assert!(body["expires_at"].is_null(), "{body}");
            }
            s => panic!("a free slot never queues: {s:?} ({body})"),
        }
    }

    #[tokio::test]
    async fn resources_two_lane_flow_per_verdict() {
        // The first verdict follows the machine (no wire seam, decision 7):
        // a grant runs the full queue-then-release flow; a refusal leaves
        // the slot free for the second lane's own verdict. Either way the
        // exclusion invariant holds — never two grants on one slot.
        let h = harness().await;
        let (coord_id, coord_token, crew_id) = register_coordinator(&h).await;
        let (contrib_id, contrib_token) = register_contributor(&h, &crew_id).await;
        let (status, first) = call(
            &h,
            "POST",
            "/api/relay/resources/request",
            Some(json!({"lane_id": coord_id, "resource": "rust-build"})),
            Some(&coord_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{first}");
        assert!(matches!(first["state"].as_str(), Some("granted") | Some("refused")), "{first}");
        let (status, second) = call(
            &h,
            "POST",
            "/api/relay/resources/request",
            Some(json!({"lane_id": contrib_id, "resource": "rust-build"})),
            Some(&contrib_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{second}");
        assert_ne!(first["lease_id"], second["lease_id"], "distinct rows");
        assert!(
            !(first["state"] == "granted" && second["state"] == "granted"),
            "mutual exclusion on the wire: {first} vs {second}"
        );
        if first["state"] == "granted" {
            assert_eq!(second["state"], "queued");
            assert_eq!(second["position"], 1);
            assert!(second["granted_at"].is_null(), "{second}");
            let (status, released) = call(
                &h,
                "POST",
                "/api/relay/resources/release",
                Some(json!({"lane_id": coord_id, "resource": "rust-build"})),
                Some(&coord_token),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{released}");
            assert_eq!(released["state"], "released");
            assert_eq!(released["lease_id"], first["lease_id"]);
            assert_eq!(released["granted_next"], second["lease_id"]);
            // The waiter re-requests: the same row, now granted.
            let (status, granted) = call(
                &h,
                "POST",
                "/api/relay/resources/request",
                Some(json!({"lane_id": contrib_id, "resource": "rust-build"})),
                Some(&contrib_token),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{granted}");
            assert_eq!(granted["state"], "granted");
            assert_eq!(granted["lease_id"], second["lease_id"]);
        } else {
            assert!(
                matches!(second["state"].as_str(), Some("granted") | Some("refused")),
                "{second}"
            );
        }
    }

    #[tokio::test]
    async fn resources_request_without_token_is_401() {
        let h = harness().await;
        let (lane_id, _, _) = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/resources/request",
            Some(json!({"lane_id": lane_id, "resource": "rust-build"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "relay_unauthorized");
    }

    #[tokio::test]
    async fn resources_non_holder_release_is_403_or_404() {
        // Whether the slot is held follows the machine (no wire seam,
        // decision 7): a grant makes the non-holder's release 403, a
        // refusal leaves nothing live for 404. Either way the status and
        // the code agree — and a 500 dies on every run.
        let h = harness().await;
        let (coord_id, coord_token, crew_id) = register_coordinator(&h).await;
        let (contrib_id, contrib_token) = register_contributor(&h, &crew_id).await;
        let (status, first) = call(
            &h,
            "POST",
            "/api/relay/resources/request",
            Some(json!({"lane_id": coord_id, "resource": "rust-build"})),
            Some(&coord_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{first}");
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/resources/release",
            Some(json!({"lane_id": contrib_id, "resource": "rust-build"})),
            Some(&contrib_token),
        )
        .await;
        if first["state"] == "granted" {
            assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
            assert_eq!(body["error"], "relay_forbidden");
        } else {
            assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
            assert_eq!(body["error"], "relay_not_found");
        }
    }

    #[tokio::test]
    async fn resources_release_unknown_is_404() {
        let h = harness().await;
        let (lane_id, token, _) = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/resources/release",
            Some(json!({"lane_id": lane_id, "resource": "e2e"})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"], "relay_not_found");
    }

    #[tokio::test]
    async fn heartbeat_renews_and_auth_is_checked() {
        let h = harness().await;
        let (lane_id, token, _) = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["expires_at"].is_string());
        // No header, wrong token: 401 with the code (never the bare 404).
        let (status, body) =
            call(&h, "POST", "/api/relay/heartbeat", Some(json!({"lane_id": lane_id})), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "relay_unauthorized");
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some("tok-bogus"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "relay_unauthorized");
    }

    // ── get_tasks / wait_for_task ──

    #[tokio::test]
    async fn get_tasks_envelope_keys_and_assignment() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) =
            call(&h, "GET", &format!("/api/relay/tasks?lane_id={lane_id}"), None, Some(&token))
                .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // Keys in ruling order; the assignment carries the frozen brief.
        let obj = body.as_object().unwrap();
        let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["messages", "answers", "reviews", "assignments"]);
        assert_eq!(body["assignments"][0]["task_id"], task_id);
        assert_eq!(body["assignments"][0]["brief"]["summary"], "Do it.");
    }

    #[tokio::test]
    async fn wait_for_task_timeout_returns_empty_envelope() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks/wait?lane_id={lane_id}&timeout_ms=10"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["messages"], json!([]));
        assert_eq!(body["answers"], json!([]));
        assert_eq!(body["reviews"], json!([]));
        assert_eq!(body["assignments"], json!([]));
    }

    #[tokio::test]
    async fn wait_for_task_wakes_on_message() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        // A person message 100 ms in must wake the 5 s hold promptly.
        let app = h.app.clone();
        let crew = coord.2.clone();
        let lane = lane_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let req = Request::builder()
                .method("POST")
                .uri("/relay/messages")
                .header("host", "127.0.0.1:7744")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "crew_id": crew,
                        "to": {"kind": "lane", "lane_id": lane},
                        "task_id": Value::Null, "text": "ping", "surface": "desktop",
                    })
                    .to_string(),
                ))
                .unwrap();
            let _ = app.oneshot(req).await.unwrap();
        });
        let start = std::time::Instant::now();
        let (status, body) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks/wait?lane_id={lane_id}&timeout_ms=5000"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["messages"][0]["text"], "ping");
        assert!(start.elapsed() < std::time::Duration::from_millis(4000), "woke, not timed out");
    }

    // ── report / review / merge ──

    /// A full report naming a branch: approval holds the task for the
    /// merge (a report with nothing to merge ends it done at once — the
    /// store's pin).
    fn report_body(lane_id: &str) -> Value {
        lane_body(
            lane_id,
            json!({
                "phase": "tests", "round": 1,
                "summary": {"changed": "Wired it.", "look_at": "app/", "flags": null},
                "files": [{"path": "app/a.ts", "added": 1, "removed": 0}],
                "branch": "lane/l2",
            }),
        )
    }

    /// Claim a floating review as `lane`.
    async fn claim_review(
        h: &Harness,
        task_id: &str,
        lane_id: &str,
        token: &str,
    ) -> (StatusCode, Value) {
        call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/claim_review"),
            Some(lane_body(lane_id, json!({}))),
            Some(token),
        )
        .await
    }

    #[tokio::test]
    async fn report_then_claim_then_review_then_merge_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        // The holder's report is its first write (F01: it starts the task);
        // the report moves to in_review and floats on the review queue
        // (queue cutover: nobody is picked)…
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&lane_id)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "in_review");
        assert!(body.get("reviewer_lane_id").is_none(), "nothing is picked: {body}");
        // …the coordinator claims the review and reads the report…
        let (status, body) = claim_review(&h, &task_id, &coord.0, &coord.1).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task_id"], task_id);
        assert_eq!(body["round"], 1);
        assert_eq!(body["phase"], "tests");
        assert!(body["author_lane_code"].is_string(), "{body}");
        assert_eq!(body["summary"], "Wired it.");
        // …approves…
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/review"),
            Some(lane_body(&coord.0, json!({"verdict": "approved", "summary": "Clean."}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "approved");
        // …and merges.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/merge"),
            Some(lane_body(&coord.0, json!({"commits": ["abc"], "into": "develop", "note": "M."}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task"]["state"], "done");
    }

    // ── Fix gate F1: partial reports ──

    fn partial_report_body(lane_id: &str) -> Value {
        lane_body(
            lane_id,
            json!({
                "phase": "tests", "round": 1, "partial": true,
                "summary": {"changed": "Halfway.", "look_at": null, "flags": null},
            }),
        )
    }

    #[tokio::test]
    async fn report_partial_stays_working_no_reviewer_pick() {
        // F1: a partial report records progress without moving the task:
        // still working, same round, no reviewer picked, and the
        // reported action carries props.partial=true.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let _ = call(&h, "GET", &format!("/api/relay/tasks?lane_id={lane_id}"), None, Some(&token))
            .await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(partial_report_body(&lane_id)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "working");
        assert!(body["reviewer_lane_id"].is_null(), "{body}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "working");
        assert_eq!(view["task"]["round"], 1);
        let reported =
            view["history"].as_array().unwrap().iter().find(|a| a["kind"] == "reported").unwrap();
        assert_eq!(reported["props"]["partial"], true);
    }

    #[tokio::test]
    async fn report_full_after_partial_lands_in_review() {
        // F1: a full report after partials moves to in_review and picks
        // the reviewer, on the same round the partials kept.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let _ = call(&h, "GET", &format!("/api/relay/tasks?lane_id={lane_id}"), None, Some(&token))
            .await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(partial_report_body(&lane_id)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&lane_id)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "in_review");
        assert_eq!(body["round"], 1);
        // Queue cutover: the review floats; the response names no reviewer.
        assert!(body.get("reviewer_lane_id").is_none(), "{body}");
    }

    #[tokio::test]
    async fn claim_review_refuses_author_second_claim_and_paused_lane() {
        // Queue cutover: a floating review is claimed by any live, unpaused,
        // non-author lane — once. The author is refused (422), a second
        // claimant finds it taken (409), a limit-paused lane cannot claim
        // (409).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (holder, token) = register_contributor(&h, &coord.2).await;
        let (other, other_token) = register_contributor(&h, &coord.2).await;
        let (paused, paused_token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &holder).await;
        let _ = call(&h, "GET", &format!("/api/relay/tasks?lane_id={holder}"), None, Some(&token))
            .await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&holder)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // The author never reviews its own task.
        let (status, body) = claim_review(&h, &task_id, &holder, &token).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
        // A paused lane sits out.
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/limit_pause",
            Some(lane_body(&paused, json!({"scope": "daily"}))),
            Some(&paused_token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = claim_review(&h, &task_id, &paused, &paused_token).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        // The first live claimant takes it; the next finds it taken.
        let (status, body) = claim_review(&h, &task_id, &other, &other_token).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = claim_review(&h, &task_id, &coord.0, &coord.1).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        // The claim is visible on the task view as the reviewer.
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["reviewer"]["id"], other, "{view}");
        assert!(view["task"]["reviewer"]["code"].is_string(), "{view}");
    }

    // ── Fix gate F9: register guidance ──

    #[tokio::test]
    async fn register_returns_versioned_guidance() {
        // F9(a): every register carries a guidance block — a version plus
        // self-sufficient generic contract lines (TTL/cadence, poll paths,
        // report flow, re-offer/wait pickup, no-transcript rule).
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "muse"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let guidance = &body["guidance"];
        assert_eq!(guidance["version"], "2", "{body}");
        let generic = guidance["generic"].as_str().unwrap();
        for line in ["300", "heartbeat", "tasks/wait", "report", "transcript", "claim_next"] {
            assert!(generic.contains(line), "generic lacks {line:?}: {generic}");
        }
    }

    // ── queue cutover (2.9): the claim loop over HTTP ──

    /// Plan a task into the pool (coordinator) and return its id.
    async fn pool_task(h: &Harness, coord: &(String, String, String), title: &str) -> String {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": title, "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["task_id"].as_str().unwrap().to_string()
    }

    async fn claim_next(h: &Harness, lane_id: &str, token: &str) -> (StatusCode, Value) {
        call(
            h,
            "POST",
            "/api/relay/tasks/claim_next",
            Some(lane_body(lane_id, json!({}))),
            Some(token),
        )
        .await
    }

    async fn claim_task(
        h: &Harness,
        task_id: &str,
        lane_id: &str,
        token: &str,
    ) -> (StatusCode, Value) {
        call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/claim"),
            Some(lane_body(lane_id, json!({}))),
            Some(token),
        )
        .await
    }

    /// A triage item (person) for a run.
    async fn triage_item(h: &Harness) -> String {
        let (status, body) = call(
            h,
            "POST",
            "/relay/triage",
            Some(json!({"title": "Gap", "kind": "gap", "tags": []})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["item_id"].as_str().unwrap().to_string()
    }

    /// A queue row exactly as `/relay/queue` shows it, in `stage`, found by
    /// its item id.
    async fn queue_row(h: &Harness, stage: &str, item_id: &str) -> Value {
        let (status, queue) = call(h, "GET", "/relay/queue", None, None).await;
        assert_eq!(status, StatusCode::OK, "{queue}");
        queue[stage]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["item_id"] == item_id)
            .cloned()
            .unwrap_or_else(|| panic!("{item_id} not in {stage}: {queue}"))
    }

    fn item_event(action: &str, op: &str, stage: &str, id: &str, row: Option<Value>) -> Value {
        let mut data = row.unwrap_or_else(|| json!({}));
        data["op"] = json!(op);
        data["id"] = json!(id);
        data["stage"] = json!(stage);
        json!({"action": action, "entity": "item", "data": data})
    }

    #[tokio::test]
    async fn queue_writes_ride_the_stream_as_item_rows() {
        // #418: each queue write is confirmed by the item events it causes —
        // the row added to triage; moved to the backlog (out of triage, into
        // the backlog with its row); taken into a sprint (out of the queue).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let mut rx = h.state.event_tx.subscribe();
        let item = triage_item(&h).await;
        let added = serde_json::to_value(event_for_entity(&mut rx, "item").await).unwrap();
        let row = queue_row(&h, "triage", &item).await;
        assert_eq!(added, item_event("add", "triaged", "triage", &item, Some(row)));
        let (status, body) =
            call(&h, "POST", "/relay/backlog", Some(json!({"item_id": item})), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let out = serde_json::to_value(event_for_entity(&mut rx, "item").await).unwrap();
        let into = serde_json::to_value(event_for_entity(&mut rx, "item").await).unwrap();
        assert_eq!(out, item_event("remove", "backlogged", "triage", &item, None));
        let row = queue_row(&h, "backlog", &item).await;
        assert_eq!(into, item_event("add", "backlogged", "backlog", &item, Some(row)));
        let run = json!({
            "title": "Sprint 1", "goal": "Ship", "item_ids": [item],
            "repo_key": "github.com/o/r", "config": {"auto_merge": false},
        });
        let (status, body) = call(&h, "POST", "/relay/runs", Some(run), None).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(body["crew_id"], coord.2);
        let planned = serde_json::to_value(event_for_entity(&mut rx, "item").await).unwrap();
        assert_eq!(planned, item_event("remove", "planned", "backlog", &item, None));
    }

    fn task_op(event: &crate::api::events::StateEvent) -> (String, String, String) {
        (
            event.entity.clone(),
            event.data["op"].as_str().unwrap_or("").to_string(),
            event.data["task_id"].as_str().unwrap_or("").to_string(),
        )
    }

    #[tokio::test]
    async fn claim_next_answers_review_then_task_then_none_with_the_design_shape() {
        // §6.3: a ready task comes back as the task shape, a floating review
        // as the review shape, nothing as `none`; each claim is announced.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (a, token_a) = register_contributor(&h, &coord.2).await;
        let (b, token_b) = register_contributor(&h, &coord.2).await;
        let task_id = pool_task(&h, &coord, "Wire it").await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = claim_next(&h, &a, &token_a).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["claim_type"], "task");
        let task = &body["task"];
        assert_eq!(task["id"], task_id);
        assert_eq!(task["title"], "Wire it");
        assert_eq!(task["phase"], "tests");
        assert_eq!(task["round"], 1);
        assert_eq!(task["deps_status"], "satisfied");
        assert_eq!(task["unmet_deps"], json!([]));
        assert!(task["owned_paths"].is_array() && task["acceptance"].is_array(), "{task}");
        assert_eq!(
            task_op(&next_event(&mut rx).await),
            ("task".into(), "claim".into(), task_id.clone())
        );
        // A reports: the review floats; B takes it as the review shape.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&a)),
            Some(&token_a),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = claim_next(&h, &b, &token_b).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["claim_type"], "review");
        let review = &body["task"];
        assert_eq!(review["id"], task_id);
        assert_eq!(review["title"], "Wire it");
        assert!(review["author_lane_code"].is_string(), "{review}");
        assert_eq!(review["phase"], "tests");
        assert_eq!(review["round"], 1);
        assert_eq!(review["summary"], "Wired it.");
        assert_eq!(review["branch"], "lane/l2");
        // B holds the review and nothing else is claimable: none.
        let (status, body) = claim_next(&h, &b, &token_b).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["claim_type"], "none");
        assert!(body["task"].is_null(), "{body}");
    }

    #[tokio::test]
    async fn claim_task_by_id_and_release_back_to_the_pool() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (a, token_a) = register_contributor(&h, &coord.2).await;
        let (b, token_b) = register_contributor(&h, &coord.2).await;
        let first = pool_task(&h, &coord, "First").await;
        let second = pool_task(&h, &coord, "Second").await;
        let (status, body) = claim_task(&h, &second, &a, &token_a).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["claim_type"], "task");
        assert_eq!(body["task"]["id"], second);
        assert_eq!(body["task"]["deps_status"], "satisfied");
        // A busy lane takes no second task.
        let (status, body) = claim_task(&h, &first, &a, &token_a).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        // Release: back to the pool, the reason on the row, announced; B
        // takes it.
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{second}/release"),
            Some(lane_body(&a, json!({"reason": "Blocked on the schema."}))),
            Some(&token_a),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task_id"], second);
        assert_eq!(body["kind"], "task");
        assert_eq!(body["state"], "ready");
        assert_eq!(
            task_op(&next_event(&mut rx).await),
            ("task".into(), "release".into(), second.clone())
        );
        let (status, body) = claim_task(&h, &second, &b, &token_b).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{second}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        let kinds: Vec<&str> =
            view["history"].as_array().unwrap().iter().filter_map(|a| a["kind"].as_str()).collect();
        assert!(kinds.contains(&"claimed") && kinds.contains(&"released"), "{kinds:?}");
    }

    #[tokio::test]
    async fn checkpoint_saves_in_turn_progress_and_answers_checkpointed_at() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        let save = json!({
            "current_step": "running clippy",
            "notes": "Three warnings left.",
            "steps": [
                {"key": "ddl", "title": "Apply the schema", "state": "done"},
                {"key": "clippy", "title": "Run clippy", "state": "open"},
            ],
        });
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/checkpoint"),
            Some(lane_body(&holder, save.clone())),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task_id"], task_id);
        assert!(body["checkpointed_at"].is_string(), "{body}");
        // #418: each step lands as a step event carrying the stored step, so the
        // screen applies it without re-reading; then the task notification.
        for (key, state) in [("ddl", "done"), ("clippy", "open")] {
            let step = next_event(&mut rx).await;
            assert_eq!(step.entity, "step", "{:?}", step.data);
            assert_eq!(serde_json::to_value(&step.action).unwrap(), "add");
            assert_eq!(step.data["task_id"], task_id);
            assert_eq!(step.data["step"]["key"], key);
            assert_eq!(step.data["step"]["state"], state);
            assert!(step.data["step"]["updated_at"].is_string(), "{:?}", step.data);
        }
        assert_eq!(
            task_op(&next_event(&mut rx).await),
            ("task".into(), "checkpoint".into(), task_id.clone())
        );
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        let steps = view["steps"].as_array().unwrap();
        assert!(steps.iter().any(|s| s["key"] == "ddl" && s["state"] == "done"), "{view}");
        assert!(steps.iter().any(|s| s["key"] == "clippy" && s["state"] == "open"), "{view}");
        // Only the holder saves.
        let (other, other_token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/checkpoint"),
            Some(lane_body(&other, save)),
            Some(&other_token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    }

    #[tokio::test]
    async fn now_lane_task_reads_its_current_step_and_step_counts() {
        // #418: the Now lane row says where the holder is — the checkpoint's
        // current step and the checklist's settled/total counts (done and
        // skipped are settled). Nothing reported reads null, never a guess.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let lane_task = |now: &Value| {
            now["lanes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["task"]["id"] == task_id)
                .map(|l| l["task"].clone())
                .expect("the holder's lane carries the task")
        };
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        let task = lane_task(&now);
        assert_eq!(task["current_step"], Value::Null, "{task}");
        assert_eq!(task["steps_done"], 0, "{task}");
        assert_eq!(task["steps_total"], 1, "{task}");
        let save = json!({
            "current_step": "running clippy",
            "steps": [
                {"key": "ddl", "title": "Apply the schema", "state": "done"},
                {"key": "lint", "title": "Lint", "state": "skipped", "reason": "clippy covers it"},
            ],
        });
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/checkpoint"),
            Some(lane_body(&holder, save)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (_, now) = call(&h, "GET", "/relay/now", None, None).await;
        let task = lane_task(&now);
        assert_eq!(task["current_step"], "running clippy", "{task}");
        assert_eq!(task["steps_done"], 2, "{task}");
        assert_eq!(task["steps_total"], 3, "{task}");
    }

    #[tokio::test]
    async fn limit_resume_ends_a_pause_early_and_emits_resumed() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/limit_pause",
            Some(lane_body(&lane, json!({"scope": "daily"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/limit_resume",
            Some(lane_body(&lane, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["resumed"], true);
        let event = next_event(&mut rx).await;
        assert_eq!(event.entity, "lane");
        assert_eq!(event.data["op"], "resumed");
        assert_eq!(event.data["reason"], "limit_pause_ended");
        assert_eq!(event.data["lane_id"], lane);
        // Not paused: nothing to end, nothing announced.
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/limit_resume",
            Some(lane_body(&lane, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["resumed"], false);
        let quiet = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(quiet.is_err(), "no second event: {quiet:?}");
    }

    #[tokio::test]
    async fn queue_lists_the_crews_claimable_work_read_only() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (a, token_a) = register_contributor(&h, &coord.2).await;
        let t1 = pool_task(&h, &coord, "One").await;
        let t2 = pool_task(&h, &coord, "Two").await;
        // A works and reports t1: a floating review beside a ready task.
        let (status, body) = claim_task(&h, &t1, &a, &token_a).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{t1}/report"),
            Some(report_body(&a)),
            Some(&token_a),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) =
            call(&h, "GET", &format!("/api/relay/queue?lane_id={a}"), None, Some(&token_a)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let queue = body["queue"].as_array().unwrap();
        assert_eq!(queue.len(), 2, "{queue:?}");
        let review = queue.iter().find(|q| q["task_id"] == t1).unwrap();
        assert_eq!(review["claim_eligibility"], "review_floating");
        assert_eq!(review["state"], "in_review");
        assert_eq!(review["author_lane_id"], a);
        let ready = queue.iter().find(|q| q["task_id"] == t2).unwrap();
        assert_eq!(ready["claim_eligibility"], "ready_unblocked");
        assert_eq!(ready["unmet_deps"], json!([]));
        assert_eq!(ready["title"], "Two");
        assert_eq!(ready["phase"], "tests");
        // Reading claimed nothing: A (its own report is not its to review)
        // takes t2 as a task.
        let (status, body) = claim_next(&h, &a, &token_a).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["claim_type"], "task");
        assert_eq!(body["task"]["id"], t2);
        // Another crew's lane sees its own, empty queue.
        let other = register_coordinator(&h).await;
        let (status, body) =
            call(&h, "GET", &format!("/api/relay/queue?lane_id={}", other.0), None, Some(&other.1))
                .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["queue"], json!([]));
    }

    /// #418: a run event carries the run's band row exactly as `/relay/now`
    /// shows it, named by `id` so the screen applies it in place.
    async fn assert_carries_run_row(h: &Harness, data: &Value, run_id: &str) {
        let (status, now) = call(h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        let row = now["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["run_id"] == run_id)
            .unwrap_or_else(|| panic!("run {run_id} not in the band: {now}"));
        for (key, value) in row.as_object().unwrap() {
            assert_eq!(&data[key], value, "{key} in {data}");
        }
        assert_eq!(data["id"], run_id, "{data}");
    }

    #[tokio::test]
    async fn create_run_is_201_with_task0_and_emits_run_created() {
        let h = harness().await;
        let item_id = triage_item(&h).await;
        let run_body = json!({
            "title": "Sprint 1", "goal": "Ship the queue", "item_ids": [item_id],
            "repo_key": "github.com/o/r", "config": {"auto_merge": false},
        });
        // No active crew: nothing to run in.
        let (status, body) = call(&h, "POST", "/relay/runs", Some(run_body.clone()), None).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_no_active_crew");
        let coord = register_coordinator(&h).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(&h, "POST", "/relay/runs", Some(run_body), None).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(body["status"], "planning");
        assert_eq!(body["crew_id"], coord.2);
        let run_id = body["run_id"].as_str().unwrap().to_string();
        let task0 = body["planning_task_id"].as_str().unwrap().to_string();
        assert_ne!(run_id, task0);
        let event = next_event(&mut rx).await;
        assert_eq!(event.entity, "run");
        assert_eq!(event.data["op"], "created");
        assert_eq!(event.data["run_id"], run_id);
        assert_eq!(event.data["crew_id"], coord.2);
        assert_carries_run_row(&h, &event.data, &run_id).await;
        // Task 0 waits in the pool for a claim.
        let (lane, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = claim_next(&h, &lane, &token).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["claim_type"], "task");
        assert_eq!(body["task"]["id"], task0);
        assert_eq!(body["task"]["phase"], "planning");
        // An unknown config knob is a caller bug (F11).
        let (status, body) = call(
            &h,
            "POST",
            "/relay/runs",
            Some(json!({"title": "S", "goal": "G", "item_ids": [], "config": {"bogus": 1}})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }

    #[tokio::test]
    async fn approving_task0_emits_unpack_and_run_activated() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (a, token_a) = register_contributor(&h, &coord.2).await;
        let (b, token_b) = register_contributor(&h, &coord.2).await;
        let item_id = triage_item(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/relay/runs",
            Some(json!({
                "title": "Sprint 1", "goal": "Ship", "item_ids": [item_id],
                "repo_key": "github.com/o/r", "config": {"auto_merge": false},
            })),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let run_id = body["run_id"].as_str().unwrap().to_string();
        let task0 = body["planning_task_id"].as_str().unwrap().to_string();
        let (status, body) = claim_next(&h, &a, &token_a).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task"]["id"], task0);
        let plan = json!({"tasks": [
            {"key": "t1", "title": "Apply schema deltas", "phase": "impl",
             "owned_paths": ["database/ddl/table/relay/runs.ddl"],
             "acceptance": ["psql -f runs.ddl succeeds"], "deps": []},
            {"key": "t2", "title": "Add claim handlers", "phase": "impl",
             "owned_paths": ["crates/senseid/src/api/handlers/relay.rs"],
             "acceptance": ["cargo check succeeds"], "deps": ["t1"]},
        ]});
        let report = lane_body(
            &a,
            json!({
                "phase": "planning", "round": 1,
                "summary": {"changed": "Planned the sprint."},
                "report_md": format!("Plan:\n\n```json\n{plan}\n```\n"),
            }),
        );
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task0}/report"),
            Some(report),
            Some(&token_a),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = claim_review(&h, &task0, &b, &token_b).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task0}/review"),
            Some(lane_body(&b, json!({"verdict": "approved", "summary": "Sound."}))),
            Some(&token_b),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "done");
        let mut seen = Vec::new();
        for _ in 0..3 {
            let event = next_event(&mut rx).await;
            seen.push((event.entity.clone(), event.data.clone()));
        }
        assert!(
            seen.iter().any(|(entity, data)| entity == "task"
                && data["op"] == "unpack"
                && data["task_id"] == task0),
            "{seen:?}"
        );
        let activated = seen
            .iter()
            .find(|(entity, data)| entity == "run" && data["op"] == "activated")
            .map(|(_, data)| data.clone())
            .unwrap_or_else(|| panic!("no run/activated: {seen:?}"));
        assert_eq!(activated["run_id"], run_id);
        assert_eq!(activated["crew_id"], coord.2);
        assert_carries_run_row(&h, &activated, &run_id).await;
        let (status, body) =
            call(&h, "GET", &format!("/api/relay/queue?lane_id={a}"), None, Some(&token_a)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["queue"].as_array().unwrap().len(), 2, "{body}");
    }

    #[tokio::test]
    async fn register_defaults_role_to_contributor_and_keeps_suggested_name() {
        // #402: no role means contributor; no crew_id joins the single
        // active crew; the suggested name is the lane's title.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"family": "claude-code", "suggested_name": "worker-alpha"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["crew_id"], coord.2);
        assert_eq!(body["lane_title"], "worker-alpha");
        // A contributor: planning is the coordinator's.
        let lane = body["lane_id"].as_str().unwrap().to_string();
        let token = body["lane_token"].as_str().unwrap().to_string();
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(&lane, json!({"title": "T", "phase": "tests"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    }

    #[tokio::test]
    async fn wait_query_refuses_unknown_keys_and_takes_timeout_seconds() {
        // F11/#404: a misspelt query key is refused, never ignored; the
        // hold takes seconds as well as milliseconds.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane, token) = register_contributor(&h, &coord.2).await;
        let (status, _) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks/wait?lane_id={lane}&timeout_ms=10&bogus=1"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks?lane_id={lane}&bogus=1"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let start = std::time::Instant::now();
        let (status, body) = call(
            &h,
            "GET",
            &format!("/api/relay/tasks/wait?lane_id={lane}&timeout_seconds=1"),
            None,
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "timeout_seconds bounds the hold: {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn wait_for_task_holds_a_waiter_row_while_waiting_and_drops_it_after() {
        // F06: the hold is a row in relay.lane_waiters for its duration.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane, token) = register_contributor(&h, &coord.2).await;
        let app = h.app.clone();
        let (lane2, token2) = (lane.clone(), token.clone());
        let waiting = tokio::spawn(async move {
            let req = Request::builder()
                .method("GET")
                .uri(format!("/api/relay/tasks/wait?lane_id={lane2}&timeout_ms=1500"))
                .header("host", "127.0.0.1:7744")
                .header("authorization", format!("Bearer {token2}"))
                .body(Body::empty())
                .unwrap();
            app.oneshot(req).await.unwrap().status()
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let lane_uuid = uuid::Uuid::parse_str(&lane).unwrap();
        let held: i64 = sqlx_core::query_scalar::query_scalar(
            "select count(*) from relay.lane_waiters where lane_id = $1",
        )
        .bind(lane_uuid)
        .fetch_one(h.state.pg.pool())
        .await
        .unwrap();
        assert_eq!(held, 1, "the hold is a waiter row");
        assert_eq!(waiting.await.unwrap(), StatusCode::OK);
        let held: i64 = sqlx_core::query_scalar::query_scalar(
            "select count(*) from relay.lane_waiters where lane_id = $1",
        )
        .bind(lane_uuid)
        .fetch_one(h.state.pg.pool())
        .await
        .unwrap();
        assert_eq!(held, 0, "the row goes with the hold");
    }

    #[tokio::test]
    async fn register_known_family_gets_override() {
        // F9(b): a known family gets generic PLUS its override note.
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "claude-code"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let notes = body["guidance"]["family_notes"].as_str().unwrap();
        assert!(notes.contains("claude"), "{notes}");
    }

    #[tokio::test]
    async fn register_unknown_family_gets_generic_only() {
        // F9(c): an unknown family (codex is a documented family outside
        // the four) gets a full generic block and null notes — no crash,
        // no empty block.
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "coordinator", "family": "codex"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let guidance = &body["guidance"];
        assert!(!guidance["generic"].as_str().unwrap().is_empty(), "{body}");
        assert!(guidance["family_notes"].is_null(), "{body}");
    }

    // ── Fix gate F1: force-handover ──

    async fn handover(
        h: &Harness,
        coord: &(String, String, String),
        task_id: &str,
        to_lane: &str,
    ) -> (StatusCode, Value) {
        call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/handover"),
            Some(lane_body(&coord.0, json!({"to_lane": to_lane}))),
            Some(&coord.1),
        )
        .await
    }

    async fn working_task(
        h: &Harness,
        coord: &(String, String, String),
    ) -> (String, String, String) {
        // A held working task: returns (task_id, holder_lane, holder_token).
        // F01 (queue cutover): reading the envelope starts nothing; the
        // holder's first write — a step here — does.
        let (holder, token) = register_contributor(h, &coord.2).await;
        let task_id = assigned_task(h, coord, &holder).await;
        let _ =
            call(h, "GET", &format!("/api/relay/tasks?lane_id={holder}"), None, Some(&token)).await;
        let (status, body) = call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/steps"),
            Some(lane_body(&holder, json!({"key": "start", "title": "Start", "state": "open"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (task_id, holder, token)
    }

    #[tokio::test]
    async fn handover_happy_path_reassigns_with_note() {
        // F1: the coordinator pulls a working task off its holder — the
        // task lands assigned on to_lane with a fresh round, the brief
        // frozen from the prior brief plus a handoff note, and a
        // handed_over event in history.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, _) = working_task(&h, &coord).await;
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = handover(&h, &coord, &task_id, &to_lane).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "assigned");
        assert_eq!(body["round"], 2);
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "assigned");
        assert_eq!(view["task"]["round"], 2);
        let history = view["history"].as_array().unwrap();
        let handoff = history.iter().find(|a| a["kind"] == "handed_over").unwrap();
        assert!(handoff["body_md"].as_str().unwrap().contains("Handed over"), "{handoff:?}");
        // The event names both parties plus the state left (shaped, not raw).
        assert_eq!(handoff["props"]["from_lane"], holder);
        assert_eq!(handoff["props"]["from_state"], "working");
        assert_eq!(handoff["props"]["to_lane"], to_lane);
        // History reads newest-first: the first `assigned` is the fresh
        // round-2 brief.
        let brief = history.iter().find(|a| a["kind"] == "assigned").unwrap();
        assert_eq!(brief["round"], 2);
        assert!(brief["body_md"].as_str().unwrap().contains("Handed over"), "{brief:?}");
    }

    #[tokio::test]
    async fn handover_requires_coordinator() {
        // RC-1 (beige-aster r1): handover is coordinator-only — a
        // contributor caller 403s, like every sibling route pins.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, _, _) = working_task(&h, &coord).await;
        let (caller, caller_token) = register_contributor(&h, &coord.2).await;
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/handover"),
            Some(lane_body(&caller, json!({"to_lane": to_lane}))),
            Some(&caller_token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "relay_forbidden");
    }

    // ── F8-B: step progress ──

    async fn post_step(
        h: &Harness,
        lane_id: &str,
        token: &str,
        task_id: &str,
        extra: Value,
    ) -> (StatusCode, Value) {
        call(
            h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/steps"),
            Some(lane_body(lane_id, extra)),
            Some(token),
        )
        .await
    }

    #[tokio::test]
    async fn step_create_posts_open_and_view_serves_it() {
        // F8: the holder posts a step; the task view serves the checklist.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["step"]["key"], "scaffold");
        assert_eq!(body["step"]["state"], "open");
        assert!(body["step"]["updated_at"].is_string(), "{body}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        // Beside the starting step the helper posted (F01).
        let steps = view["steps"].as_array().unwrap();
        let step =
            steps.iter().find(|s| s["key"] == "scaffold").unwrap_or_else(|| panic!("{view}"));
        assert_eq!(step["title"], "Scaffold the crate");
        assert_eq!(step["state"], "open");
    }

    #[tokio::test]
    async fn step_update_open_to_done() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, _) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "done"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["step"]["state"], "done");
    }

    #[tokio::test]
    async fn step_skipped_needs_reason() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "bench", "title": "Benchmark", "state": "skipped"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "bench", "title": "Benchmark", "state": "skipped", "reason": "No perf gate this slice"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["step"]["reason"], "No perf gate this slice");
    }

    #[tokio::test]
    async fn step_done_is_terminal_but_idempotent() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, _) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "done"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Re-posting done is a no-op…
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "done"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // …but done never reopens.
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open"}),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn step_non_holder_is_403() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, _, _) = working_task(&h, &coord).await;
        let (other, other_token) = register_contributor(&h, &coord.2).await;
        let (status, body) = post_step(
            &h,
            &other,
            &other_token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open"}),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "relay_forbidden");
    }

    #[tokio::test]
    async fn step_wrong_state_is_409() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        // In review: the turn is over, steps with it. (An assigned task
        // takes the holder's first step — it starts the work (F01), the
        // store's pin. Cancelled needs no pin: cancel refuses a live holder
        // while steps require one. Done shares this same state gate.)
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&holder)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open"}),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    // ── F8-C: relay SSE ──

    async fn next_event(
        rx: &mut tokio::sync::broadcast::Receiver<crate::api::events::StateEvent>,
    ) -> crate::api::events::StateEvent {
        tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("event arrives")
            .expect("channel open")
    }

    #[tokio::test]
    async fn relay_report_emits_task_event() {
        // F8: a task mutation emits a task-entity event naming the task.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&holder)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let event = next_event(&mut rx).await;
        assert_eq!(event.entity, "task");
        assert_eq!(event.data["op"], "report");
        assert_eq!(event.data["task_id"], task_id);
    }

    /// #418: a task event carries the task's header exactly as the task view
    /// serves it, named by `id`.
    async fn assert_carries_task_row(h: &Harness, data: &Value, crew_id: &str, task_id: &str) {
        let (status, view) =
            call(h, "GET", &format!("/relay/tasks/{crew_id}/{task_id}"), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        for (key, value) in view["task"].as_object().unwrap() {
            assert_eq!(&data[key], value, "{key} in {data}");
        }
        assert_eq!(data["id"], task_id, "{data}");
    }

    #[tokio::test]
    async fn task_events_carry_the_task_header() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&holder)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let reported = event_for(&mut rx, "task", "report").await;
        assert_eq!(reported["data"]["task_id"], task_id);
        assert_carries_task_row(&h, &reported["data"], &coord.2, &task_id).await;
        // The sweep's events carry it too: a lapsed lease interrupts the task.
        let (task_id, _, _) = working_task(&h, &coord).await;
        let past_lease = chrono::Utc::now() + chrono::Duration::seconds(301);
        crate::tasks::relay_lease_sweep::sweep_at(&h.state.pg, &h.state.event_tx, past_lease)
            .await
            .unwrap();
        let interrupted = loop {
            let e = event_for(&mut rx, "task", "interrupt").await;
            if e["data"]["task_id"] == task_id {
                break e;
            }
        };
        assert_carries_task_row(&h, &interrupted["data"], &coord.2, &task_id).await;
    }

    #[tokio::test]
    async fn relay_step_post_emits_step_event() {
        // F8: step posts stream the step itself (add on create, update on
        // restate) — the screen applies them without re-reading.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let event = next_event(&mut rx).await;
        assert_eq!(event.entity, "step");
        assert!(matches!(event.action, crate::api::events::Action::Add));
        assert_eq!(event.data["task_id"], task_id);
        assert_eq!(event.data["step"]["key"], "scaffold");
        let (status, _) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "done"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let event = next_event(&mut rx).await;
        assert!(matches!(event.action, crate::api::events::Action::Update));
        assert_eq!(event.data["step"]["state"], "done");
    }

    #[tokio::test]
    async fn relay_answer_emits_task_event() {
        // F8: answering a task-bound decision names the task (the store
        // already reads it — the event just carries it).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &holder,
                json!({
                    "task_id": task_id, "question": "Where?",
                    "options": [{"id": "db", "label": "Db"}],
                    "blocks": [], "asked_of": "person",
                }),
            )),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let decision_id = body["decision_id"].as_str().unwrap().to_string();
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            "/relay/answers",
            Some(json!({"decision_id": decision_id, "option": "db", "note": null, "surface": "desktop"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let event = next_event(&mut rx).await;
        assert_eq!(event.entity, "task");
        assert_eq!(event.data["op"], "answer");
        assert_eq!(event.data["task_id"], task_id);
    }

    #[tokio::test]
    async fn person_decision_rides_the_stream_as_its_row() {
        // #418: asked of the person, the decision arrives with its Needs-you
        // row (exactly as /relay/now shows it); answered, it leaves by id.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &holder,
                json!({
                    "task_id": task_id, "question": "Where?",
                    "options": [{"id": "db", "label": "Db"}],
                    "blocks": [], "asked_of": "person",
                }),
            )),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let decision_id = body["decision_id"].as_str().unwrap().to_string();
        let asked = serde_json::to_value(event_for_entity(&mut rx, "decision").await).unwrap();
        let (_, now) = call(&h, "GET", "/relay/now", None, None).await;
        let mut row = now["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == decision_id)
            .cloned()
            .unwrap_or_else(|| panic!("decision not on Now: {now}"));
        row["op"] = json!("asked");
        assert_eq!(asked, json!({"action": "add", "entity": "decision", "data": row}));
        let (status, body) = call(
            &h,
            "POST",
            "/relay/answers",
            Some(json!({"decision_id": decision_id, "option": "db", "note": null, "surface": "desktop"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let answered = serde_json::to_value(event_for_entity(&mut rx, "decision").await).unwrap();
        assert_eq!(
            answered,
            json!({"action": "remove", "entity": "decision",
                   "data": {"op": "answered", "id": decision_id, "crew_id": coord.2}})
        );
    }

    #[tokio::test]
    async fn a_decision_asked_of_a_lane_sends_no_decision_event() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &holder,
                json!({
                    "task_id": task_id, "question": "Which?",
                    "options": [{"id": "a", "label": "A"}],
                    "blocks": [], "asked_of": coord.0,
                }),
            )),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // The task notification arrives; nothing names a decision.
        assert_eq!(event_for(&mut rx, "task", "ask").await["data"]["task_id"], task_id);
        while let Ok(event) = rx.try_recv() {
            assert_ne!(event.entity, "decision", "{:?}", event.data);
        }
    }

    /// The next event naming `entity`, skipping others (5 s).
    async fn event_for_entity(
        rx: &mut tokio::sync::broadcast::Receiver<crate::api::events::StateEvent>,
        entity: &str,
    ) -> crate::api::events::StateEvent {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut seen = Vec::new();
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Ok(event)) if event.entity == entity => return event,
                Ok(Ok(event)) => seen.push(event.entity),
                _ => panic!("no {entity} event; saw {seen:?}"),
            }
        }
    }

    #[tokio::test]
    async fn relay_events_endpoint_streams_event_stream() {
        // F8: GET /api/relay/events speaks SSE (headers only — the stream
        // never ends, so the test never reads the body).
        let h = harness().await;
        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/api/relay/events")
            .header("host", "127.0.0.1:7744")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = h.app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp.headers()["content-type"].to_str().unwrap().to_string();
        assert!(content_type.starts_with("text/event-stream"), "{content_type}");
    }

    /// The SSE frames' data, read off a stream until `n` arrive.
    async fn sse_frames(resp: axum::response::Response, n: usize) -> Vec<Value> {
        use tokio_stream::StreamExt;
        let mut stream = resp.into_body().into_data_stream();
        let mut buf = String::new();
        let mut out = Vec::new();
        while out.len() < n {
            let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
                .await
                .expect("a frame arrives")
                .expect("stream open")
                .unwrap();
            buf.push_str(std::str::from_utf8(&chunk).unwrap());
            while let Some(end) = buf.find("\n\n") {
                let frame: String = buf.drain(..end + 2).collect();
                if let Some(data) = frame.lines().find_map(|l| l.strip_prefix("data:")) {
                    out.push(serde_json::from_str(data.trim()).unwrap());
                }
            }
        }
        out
    }

    #[tokio::test]
    async fn relay_events_crew_filter_drops_other_crews() {
        // #418: `?crew_id=` narrows the stream to one crew. An event naming
        // another crew never arrives; one naming no crew (a task op) does —
        // the screen re-reads its own crew's view on it.
        let h = harness().await;
        let (crew_a, crew_b) = (uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/relay/events?crew_id={crew_a}"))
            .header("host", "127.0.0.1:7744")
            .body(Body::empty())
            .unwrap();
        let resp = h.app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let lane = uuid::Uuid::now_v7();
        let tx = &h.state.event_tx;
        use crate::api::events::{LaneChange, StateEvent};
        tx.send(StateEvent::relay_lane(LaneChange::Registered, &lane, &crew_b)).unwrap();
        tx.send(StateEvent::relay_lane(LaneChange::Registered, &lane, &crew_a)).unwrap();
        tx.send(StateEvent::relay_task("report", &lane)).unwrap();
        let frames = sse_frames(resp, 2).await;
        assert_eq!(frames[0]["entity"], "lane");
        assert_eq!(frames[0]["data"]["crew_id"], crew_a.to_string());
        assert_eq!(frames[1]["entity"], "task");
    }

    #[tokio::test]
    async fn relay_events_without_crew_filter_carry_every_crew() {
        let h = harness().await;
        let (crew_a, crew_b) = (uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
        let req = Request::builder()
            .method("GET")
            .uri("/api/relay/events")
            .header("host", "127.0.0.1:7744")
            .body(Body::empty())
            .unwrap();
        let resp = h.app.clone().oneshot(req).await.unwrap();
        let lane = uuid::Uuid::now_v7();
        use crate::api::events::{LaneChange, StateEvent};
        h.state
            .event_tx
            .send(StateEvent::relay_lane(LaneChange::Registered, &lane, &crew_b))
            .unwrap();
        h.state
            .event_tx
            .send(StateEvent::relay_lane(LaneChange::Registered, &lane, &crew_a))
            .unwrap();
        let frames = sse_frames(resp, 2).await;
        assert_eq!(frames[0]["data"]["crew_id"], crew_b.to_string());
        assert_eq!(frames[1]["data"]["crew_id"], crew_a.to_string());
    }

    #[tokio::test]
    async fn relay_events_bad_crew_id_is_400() {
        let h = harness().await;
        let req = Request::builder()
            .method("GET")
            .uri("/api/relay/events?crew_id=nope")
            .header("host", "127.0.0.1:7744")
            .body(Body::empty())
            .unwrap();
        let resp = h.app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn step_reason_only_on_skipped() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, body) = post_step(
            &h,
            &holder,
            &token,
            &task_id,
            json!({"key": "scaffold", "title": "Scaffold the crate", "state": "open", "reason": "why"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
    }

    #[tokio::test]
    async fn handover_terminal_task_is_409() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let task_id = planned_task(&h, &coord).await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(lane_body(&coord.0, json!({}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = handover(&h, &coord, &task_id, &to_lane).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn handover_to_dead_lane_is_409() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, _, _) = working_task(&h, &coord).await;
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        sqlx_core::query::query("update relay.leases set expires_at = $1 where lane_id = $2")
            .bind(chrono::Utc::now() - chrono::Duration::seconds(1))
            .bind(to_lane.parse::<uuid::Uuid>().unwrap())
            .execute(h.state.pg.pool())
            .await
            .unwrap();
        let (status, body) = handover(&h, &coord, &task_id, &to_lane).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn handover_cross_crew_lane_is_409() {
        // Cross-crew reads as not-found at assign; handover says 409 with
        // a specific error instead (F1 brief).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, _, _) = working_task(&h, &coord).await;
        let coord2 = register_coordinator(&h).await;
        let (other_lane, _) = register_contributor(&h, &coord2.2).await;
        let (status, body) = handover(&h, &coord, &task_id, &other_lane).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn handover_to_claimed_reviewer_is_409() {
        // A task never reviews itself: the lane holding the review claim
        // cannot take over the task.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (reviewer, reviewer_token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/report"),
            Some(report_body(&holder)),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = claim_review(&h, &task_id, &reviewer, &reviewer_token).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = handover(&h, &coord, &task_id, &reviewer).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn handover_interrupted_task_is_409() {
        // Interrupted tasks re-offer through the sweeper; handover is refused.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(lane_body(&holder, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = handover(&h, &coord, &task_id, &to_lane).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn handover_rejects_non_handoff_states() {
        // F1 interpretation: handover pulls from working/in_review/rework
        // only — assigned (no holder), blocked, approved, and same-holder
        // handoffs all 409 with specific errors. One holder per case (one
        // active task per lane).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        // Assigned: nothing held yet.
        let (holder_a, _) = register_contributor(&h, &coord.2).await;
        let fresh = assigned_task(&h, &coord, &holder_a).await;
        let (status, body) = handover(&h, &coord, &fresh, &to_lane).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        // Blocked: resolve it first.
        let (holder_b, token_b) = register_contributor(&h, &coord.2).await;
        let blocked = assigned_task(&h, &coord, &holder_b).await;
        let _ =
            call(&h, "GET", &format!("/api/relay/tasks?lane_id={holder_b}"), None, Some(&token_b))
                .await;
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": blocked, "question": "Where?",
                    "options": [{"id": "db", "label": "Db"}],
                    "blocks": [blocked], "asked_of": "person",
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = handover(&h, &coord, &blocked, &to_lane).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        // Same holder: nothing to pull.
        let (holder_c, token_c) = register_contributor(&h, &coord.2).await;
        let working = assigned_task(&h, &coord, &holder_c).await;
        let _ =
            call(&h, "GET", &format!("/api/relay/tasks?lane_id={holder_c}"), None, Some(&token_c))
                .await;
        let (status, body) = handover(&h, &coord, &working, &holder_c).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        // Approved: the work is done; merge it. Own two-lane crew; the
        // coordinator claims the floating review and approves.
        let coord_b = register_coordinator(&h).await;
        let (worker_b, token_wb) = register_contributor(&h, &coord_b.2).await;
        let approved = assigned_task(&h, &coord_b, &worker_b).await;
        let _ =
            call(&h, "GET", &format!("/api/relay/tasks?lane_id={worker_b}"), None, Some(&token_wb))
                .await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{approved}/report"),
            Some(report_body(&worker_b)),
            Some(&token_wb),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = claim_review(&h, &approved, &coord_b.0, &coord_b.1).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{approved}/review"),
            Some(lane_body(&coord_b.0, json!({"verdict": "approved", "summary": "Clean."}))),
            Some(&coord_b.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (to_lane_b, _) = register_contributor(&h, &coord_b.2).await;
        let (status, body) = handover(&h, &coord_b, &approved, &to_lane_b).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn handover_note_is_a_checked_summary() {
        // F1: the handoff note is person-facing (it lands in the brief and
        // the handed_over body), so it keeps the house summary bar: ≤ 600
        // chars, no secrets.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, _, _) = working_task(&h, &coord).await;
        let (to_lane, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/handover"),
            Some(lane_body(&coord.0, json!({"to_lane": to_lane, "note": "x".repeat(700)}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
    }

    #[tokio::test]
    async fn assign_merge_phase_is_422_q3() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": "T", "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let task_id = body["task_id"].as_str().unwrap().to_string();
        // No merge phase (Q3): merging is the merged action.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "merge",
                    "brief": {"summary": "x", "docs": []},
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
    }

    #[tokio::test]
    async fn assign_to_expired_lane_is_409() {
        // The F5 bar, end to end: assigning onto a dead lease 409s instead
        // of stranding the task where no sweep, cancel, or re-assign frees it.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = planned_task(&h, &coord).await;
        sqlx_core::query::query("update relay.leases set expires_at = $1 where lane_id = $2")
            .bind(chrono::Utc::now() - chrono::Duration::seconds(1))
            .bind(lane_id.parse::<uuid::Uuid>().unwrap())
            .execute(h.state.pg.pool())
            .await
            .unwrap();
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "tests",
                    "brief": {"summary": "Do it.", "docs": []},
                    "acceptance": ["green"], "owned_paths": ["app/**"],
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn unknown_task_is_404_with_code() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let missing = "01938931-0000-7000-8000-000000000099";
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{missing}/report"),
            Some(report_body(&coord.0)),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"], "relay_not_found");
    }

    #[tokio::test]
    async fn plan_requires_coordinator_403() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(&lane_id, json!({"title": "T", "phase": "tests"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "relay_forbidden");
    }

    // ── Fix gate F2: repo_key is required (null rejected) ──

    #[tokio::test]
    async fn plan_explicit_null_repo_key_rejected_422() {
        // F2: the plan body accepts explicit null syntactically, but a
        // task needs a repository — the rejection is 422 +
        // relay_invalid, documented on RelayPlanInput (never a silent
        // accept, never a 500).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(&coord.0, json!({"title": "T", "phase": "tests", "repo_key": null}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
        assert_eq!(body["message"], "a task needs a repository");
    }

    // ── decisions / messages ──

    #[tokio::test]
    async fn ask_then_person_answers_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": task_id, "question": "Where?",
                    "options": [{"id": "db", "label": "Db"}],
                    "blocks": [task_id], "asked_of": "person",
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let decision_id = body["decision_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            &h,
            "POST",
            "/relay/answers",
            Some(json!({"decision_id": decision_id, "option": "db", "note": null, "surface": "desktop"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["action_id"].is_string());
    }

    #[tokio::test]
    async fn lane_answers_lane_asked_decision() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": task_id, "question": "Which?",
                    "options": [{"id": "a", "label": "A"}],
                    "blocks": [], "asked_of": lane_id,
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let decision_id = body["decision_id"].as_str().unwrap().to_string();
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/decisions/{decision_id}/answer"),
            Some(lane_body(&lane_id, json!({"option": "a"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    async fn person_and_lane_messages_shape() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        // Person → lane (relay-api.ts shape)…
        let (status, body) = call(
            &h,
            "POST",
            "/relay/messages",
            Some(json!({
                "crew_id": coord.2,
                "to": {"kind": "lane", "lane_id": lane_id},
                "task_id": Value::Null, "text": "hi", "surface": "desktop",
            })),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["message_id"].is_string());
        // …lane → coordinator (lane shape)…
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/messages",
            Some(lane_body(&lane_id, json!({"to": {"kind": "coordinator"}, "text": "ack"}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["message_id"].is_string());
    }

    // ── Q2 part 2: crew → person messages in Needs-you ──

    async fn send_lane_message(
        h: &Harness,
        lane_id: &str,
        token: &str,
        task_id: Option<&str>,
        to: serde_json::Value,
        text: &str,
    ) {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/messages",
            Some(lane_body(lane_id, json!({"to": to, "task_id": task_id, "text": text}))),
            Some(token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    async fn a_lane_message_to_the_person_rides_the_stream_as_its_row() {
        // #418: Needs you gains the message from the stream, its row exactly
        // as /relay/now shows it. A message to a lane is not Needs you's.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (task_id, holder, token) = working_task(&h, &coord).await;
        let mut rx = h.state.event_tx.subscribe();
        send_lane_message(
            &h,
            &holder,
            &token,
            Some(&task_id),
            json!({"kind": "person"}),
            "Look here",
        )
        .await;
        let sent = serde_json::to_value(event_for_entity(&mut rx, "message").await).unwrap();
        let (_, now) = call(&h, "GET", "/relay/now", None, None).await;
        let mut row = now["person_messages"][0].clone();
        assert_eq!(row["text"], "Look here", "{now}");
        row["op"] = json!("sent");
        assert_eq!(sent, json!({"action": "add", "entity": "message", "data": row}));
        send_lane_message(&h, &holder, &token, None, json!({"kind": "coordinator"}), "ack").await;
        while let Ok(event) = rx.try_recv() {
            assert_ne!(event.entity, "message", "{:?}", event.data);
        }
    }

    #[tokio::test]
    async fn now_serves_person_messages_newest_first() {
        // Q2-2(a): to.person messages from a lane surface in the person
        // feed newest-first with lane code, text, and task link.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        send_lane_message(
            &h,
            &lane_id,
            &token,
            Some(&task_id),
            json!({"kind": "person"}),
            "first, needs your eyes",
        )
        .await;
        send_lane_message(
            &h,
            &lane_id,
            &token,
            Some(&task_id),
            json!({"kind": "person"}),
            "second, blocks the merge",
        )
        .await;
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let messages = body["person_messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2, "{body}");
        assert_eq!(messages[0]["text"], "second, blocks the merge");
        assert_eq!(messages[1]["text"], "first, needs your eyes");
        let lanes = body["lanes"].as_array().unwrap();
        let code = lanes.iter().find(|l| l["id"] == lane_id).unwrap()["code"].as_str().unwrap();
        assert_eq!(messages[0]["from_lane_code"], code);
        assert_eq!(messages[0]["task_id"], task_id);
        assert_eq!(messages[0]["crew_id"], coord.2);
        assert!(messages[0]["sent_at"].is_string());
        assert!(messages[0]["task_title"].is_string());
    }

    #[tokio::test]
    async fn now_person_message_clears_after_read() {
        // Q2-2(b): the envelope's receipt walk, mirrored for the person —
        // first read delivers, second read reads, third read is clear.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        send_lane_message(
            &h,
            &lane_id,
            &token,
            Some(&task_id),
            json!({"kind": "person"}),
            "read me twice",
        )
        .await;
        for expected in [1, 1, 0] {
            let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert_eq!(body["person_messages"].as_array().unwrap().len(), expected, "{body}");
        }
    }

    #[tokio::test]
    async fn now_person_messages_exclude_other_addresses() {
        // Q2-2(c): everyone/coordinator/lane mail stays out of the person
        // feed — only to.person arrives.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        send_lane_message(
            &h,
            &lane_id,
            &token,
            Some(&task_id),
            json!({"kind": "everyone"}),
            "to all",
        )
        .await;
        send_lane_message(
            &h,
            &lane_id,
            &token,
            Some(&task_id),
            json!({"kind": "coordinator"}),
            "to coord",
        )
        .await;
        let (status, body) = call(
            &h,
            "POST",
            "/relay/messages",
            Some(json!({
                "crew_id": coord.2,
                "to": {"kind": "lane", "lane_id": lane_id},
                "task_id": Value::Null, "text": "to lane", "surface": "desktop",
            })),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["person_messages"].as_array().unwrap().len(), 0, "{body}");
    }

    // ── Limit exhaustion: lanes report quota pauses ──

    async fn limit_pause(
        h: &Harness,
        lane_id: &str,
        token: &str,
        scope: &str,
        resumes_at: serde_json::Value,
        note: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        call(
            h,
            "POST",
            "/api/relay/limit_pause",
            Some(lane_body(
                lane_id,
                json!({"scope": scope, "resumes_at": resumes_at, "note": note}),
            )),
            Some(token),
        )
        .await
    }

    async fn now_lane(h: &Harness, lane_id: &str) -> serde_json::Value {
        let (status, body) = call(h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["lanes"].as_array().unwrap().iter().find(|l| l["id"] == lane_id).cloned().unwrap()
    }

    #[tokio::test]
    async fn limit_pause_records_scope_and_resume() {
        // The lane reports quota exhaustion; the person feed echoes scope
        // + resume time on the lane row (lease untouched — still live).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = limit_pause(
            &h,
            &lane_id,
            &token,
            "weekly",
            json!("2030-01-01T00:00:00Z"),
            Some("weekly cap hit during backfill"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let lane = now_lane(&h, &lane_id).await;
        assert_eq!(lane["limit_scope"], "weekly", "{lane}");
        // Timestamptz renders zone-dependent (lease_expires_at precedent),
        // so the resume instant — not the string — is the assertion.
        let resumes =
            chrono::DateTime::parse_from_rfc3339(lane["limit_resumes_at"].as_str().unwrap())
                .unwrap()
                .to_utc();
        assert_eq!(
            resumes,
            chrono::DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z").unwrap().to_utc()
        );
        assert_eq!(lane["lease_state"], "granted", "{lane}");
    }

    #[tokio::test]
    async fn limit_pause_unknown_scope_rejected() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) =
            limit_pause(&h, &lane_id, &token, "monthly", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }

    #[tokio::test]
    async fn sweep_paused_lane_task_interrupts_without_lease_expiry() {
        // A paused lane's held task suspends for re-offer elsewhere, but
        // the lane itself is not swept: no expiry, no failure.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, _) =
            limit_pause(&h, &lane_id, &token, "daily", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        h.state.pg.relay_sweep_leases(chrono::Utc::now()).await.unwrap();
        let lane = now_lane(&h, &lane_id).await;
        assert_eq!(lane["lease_state"], "granted", "{lane}");
        assert_eq!(lane["limit_scope"], "daily", "{lane}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "interrupted", "{view}");
    }

    #[tokio::test]
    async fn heartbeat_never_ends_a_pause() {
        // F03 (queue cutover): a beat renews the lease and nothing else —
        // before resumes_at, past it, and with no resume time at all. The
        // sweeper ends a pause at resumes_at; the lane ends it early with
        // limit_resume.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, _) =
            limit_pause(&h, &lane_id, &token, "daily", json!("2030-01-01T00:00:00Z"), None).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let lane = now_lane(&h, &lane_id).await;
        assert_eq!(lane["limit_scope"], "daily", "{lane}");
        h.state
            .pg
            .relay_heartbeat(
                lane_id.parse().unwrap(),
                chrono::DateTime::parse_from_rfc3339("2030-01-02T00:00:00Z").unwrap().to_utc(),
            )
            .await
            .unwrap();
        let lane = now_lane(&h, &lane_id).await;
        assert_eq!(lane["limit_scope"], "daily", "a beat past resumes_at: {lane}");
        let (status, _) =
            limit_pause(&h, &lane_id, &token, "other", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let lane = now_lane(&h, &lane_id).await;
        assert_eq!(lane["limit_scope"], "other", "an unknown resume: {lane}");
    }

    #[tokio::test]
    async fn reregister_clears_pause() {
        // Leaving and resuming returns the lane to live duty: no stale
        // pause survives the round trip.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, _) =
            limit_pause(&h, &lane_id, &token, "other", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        for route in ["/api/relay/unregister", "/api/relay/reregister"] {
            let (status, body) =
                call(&h, "POST", route, Some(lane_body(&lane_id, json!({}))), Some(&token)).await;
            assert_eq!(status, StatusCode::OK, "{body}");
        }
        let lane = now_lane(&h, &lane_id).await;
        assert!(lane["limit_scope"].is_null(), "{lane}");
    }

    // ── person reads ──

    #[tokio::test]
    async fn now_queue_read_and_filter() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let _ = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["crews"].as_array().unwrap().len(), 1);
        assert!(body["lanes"].is_array() && body["decisions"].is_array());
        let (status, body) =
            call(&h, "GET", "/relay/now?repo_key=github.com/nope", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["crews"], json!([]));
        let (status, body) = call(&h, "GET", "/relay/queue", None, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["triage"].is_array() && body["backlog"].is_array());
    }

    #[tokio::test]
    async fn task_view_cross_crew_is_404() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        // A second crew: its id with the first crew's task is not found.
        let (_, _, crew2) = register_coordinator(&h).await;
        let (status, body) =
            call(&h, "GET", &format!("/relay/tasks/{crew2}/{task_id}"), None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["error"], "relay_not_found");
    }

    #[tokio::test]
    async fn unregister_then_reregister_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Left: the lane reads gone…
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert_eq!(body["error"], "relay_lane_gone");
        // …until it reregisters with the same token.
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/reregister",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["lane_id"], lane_id);
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn backlog_add_item_and_ticket_is_501() {
        let h = harness().await;
        let _ = register_coordinator(&h).await;
        // A manual triage item, then the person's +.
        let (status, body) = call(
            &h,
            "POST",
            "/relay/triage",
            Some(json!({"title": "Gap", "kind": "gap", "tags": ["ux"]})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let item_id = body["item_id"].as_str().unwrap().to_string();
        let (status, body) =
            call(&h, "POST", "/relay/backlog", Some(json!({"item_id": item_id})), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["item_id"], item_id);
        // Slice 2 owns the tracker pull: a ticketed add is 501, never a
        // fabricated title (derived).
        let (status, body) = call(
            &h,
            "POST",
            "/relay/backlog",
            Some(json!({"ticket_provider": "github", "ticket_key": "SEN-52"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{body}");
        assert_eq!(body["error"], "relay_not_implemented");
    }

    // ── F3: triage / cancel / close ──

    /// Coordinator plans a tests-phase task without assigning; returns the
    /// task id. The task sits `assigned` with NULL lane: run-01's M2/M3.
    async fn planned_task(h: &Harness, coord: &(String, String, String)) -> String {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": "T", "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["task_id"].as_str().unwrap().to_string()
    }

    async fn unregister_lane(h: &Harness, lane_id: &str, token: &str) {
        let (status, body) = call(
            h,
            "POST",
            "/api/relay/unregister",
            Some(json!({"lane_id": lane_id})),
            Some(token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    async fn triage_surfaces_planned_task_with_full_identity() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let task_id = planned_task(&h, &coord).await;
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        let bucket = now["triage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["provider"] == "unassigned")
            .expect("unassigned bucket");
        assert_eq!(bucket["count"], 1);
        assert_eq!(bucket["sprint"], Value::Null);
        let tasks = bucket["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 1);
        let t = &tasks[0];
        assert_eq!(t["id"], task_id);
        assert_eq!(t["crew_id"], coord.2);
        assert_eq!(t["title"], "T");
        assert_eq!(t["phase"], "tests");
        // Queue cutover: a planned task waits in the pool as `ready`.
        assert_eq!(t["state"], "ready");
        assert_eq!(t["round"], 1);
        assert_eq!(t["repo_key"], "github.com/o/r");
    }

    #[tokio::test]
    async fn triage_excludes_lane_held_tasks() {
        // A held task rides its lane's census row, never the unassigned
        // bucket. (Pin: green before F3, guards the triage write.)
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        assigned_task(&h, &coord, &lane_id).await;
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        assert!(
            now["triage"].as_array().unwrap().iter().all(|e| e["provider"] != "unassigned"),
            "{now}"
        );
    }

    #[tokio::test]
    async fn cancel_unassigned_task_happy_path() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let task_id = planned_task(&h, &coord).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["task_id"], task_id);
        assert_eq!(body["state"], "cancelled");
        // Cancelled leaves triage: it is no longer unassigned work.
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "cancelled");
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        assert!(
            now["triage"].as_array().unwrap().iter().all(|e| e["provider"] != "unassigned"),
            "{now}"
        );
    }

    #[tokio::test]
    async fn cancel_requires_coordinator() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = planned_task(&h, &coord).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "relay_forbidden");
    }

    #[tokio::test]
    async fn cancel_terminal_task_is_409() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let task_id = planned_task(&h, &coord).await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // A cancelled task is terminal: the second cancel is a 409, never
        // a silent no-op.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn cancel_live_held_task_is_409() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        // The 409 changed nothing: the task is still assigned to its lane.
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "assigned");
    }

    #[tokio::test]
    async fn cancel_dead_holder_task_succeeds() {
        // Unregister suspends the holder's task to interrupted (lane id
        // stays); the coordinator cancels the orphan instead of reoffering.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        unregister_lane(&h, &lane_id, &token).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "cancelled");
    }

    #[tokio::test]
    async fn cancel_blocked_task_with_live_holder_is_409() {
        // Blocked does not bypass the holder rule: a live holder still
        // 409s, and the task (with its block memory) is untouched — any
        // partial write would 500 on the blocked-memory check. The clear
        // itself is pinned at the store level
        // (`cancel_blocked_task_clears_block_memory`).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": task_id, "question": "Which?",
                    "options": [{"id": "a", "label": "A"}],
                    "blocks": [task_id], "asked_of": "person",
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "blocked");
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "blocked");
    }

    // ── Follow-up: unregister clears block memory (F3 ruling-4) ──

    #[tokio::test]
    async fn unregister_blocked_task_holder_clears_block_memory() {
        // Unregister suspends the lane's task via LeaseLost; a BLOCKED
        // holder must land interrupted with its block memory cleared —
        // without the clear the suspend write 500s on the
        // tasks_blocked_memory check (cancel's precedent: the memory
        // clear rides the state write).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/decisions",
            Some(lane_body(
                &coord.0,
                json!({
                    "task_id": task_id, "question": "Which?",
                    "options": [{"id": "a", "label": "A"}],
                    "blocks": [task_id], "asked_of": "person",
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "blocked");
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, view) =
            call(&h, "GET", &format!("/relay/tasks/{}/{task_id}", coord.2), None, None).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["task"]["state"], "interrupted");
        let memory: Option<String> = sqlx_core::query_scalar::query_scalar(
            "select blocked_from_state::text from relay.tasks where id = $1",
        )
        .bind(task_id.parse::<uuid::Uuid>().unwrap())
        .fetch_one(h.state.pg.pool())
        .await
        .unwrap();
        assert_eq!(memory, None);
    }

    #[tokio::test]
    async fn cancelled_task_never_reoffers() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = planned_task(&h, &coord).await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/cancel"),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // A sweeper pass far past every lease and grace touches nothing.
        let far = chrono::Utc::now() + chrono::Duration::days(30);
        let report = h.state.pg.relay_sweep_leases(far).await.unwrap();
        assert_eq!(report.interrupted_tasks, 0);
        assert_eq!(report.reoffered_tasks, 0);
        // Still cancelled after the sweep: assign refuses it.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "tests",
                    "brief": {"summary": "Do it.", "docs": []},
                    "acceptance": ["green"], "owned_paths": ["app/**"],
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn close_happy_path() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        unregister_lane(&h, &lane_id, &token).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["crew_id"], coord.2);
        assert_eq!(body["status"], "closed");
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        assert_eq!(now["crews"][0]["status"], "closed");
        // The census crew row carries no completed_at (out of brief scope
        // to add one); the stamp itself is pinned at the store level
        // (`close_crew_stamps_completed_at`).
    }

    #[tokio::test]
    async fn close_with_live_lane_is_409() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (_lane_id, _token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        let (status, now) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        assert_eq!(now["crews"][0]["status"], "active");
    }

    #[tokio::test]
    async fn close_requires_coordinator() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "relay_forbidden");
    }

    #[tokio::test]
    async fn closed_crew_rejects_register_plan_assign() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        // Planned before the close so assign has a task to refuse.
        let task_id = planned_task(&h, &coord).await;
        unregister_lane(&h, &lane_id, &token).await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Join the closed crew: 409.
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "contributor", "family": "claude-code", "crew_id": coord.2})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        // Plan on the closed crew: 409.
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/tasks/plan",
            Some(lane_body(
                &coord.0,
                json!({"title": "Late", "phase": "tests", "repo_key": "github.com/o/r"}),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        // Assign on the closed crew: 409.
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/tasks/{task_id}/assign"),
            Some(lane_body(
                &coord.0,
                json!({
                    "to_lane": lane_id, "title": "T", "phase": "tests",
                    "brief": {"summary": "Do it.", "docs": []},
                    "acceptance": ["green"], "owned_paths": ["app/**"],
                }),
            )),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
    }

    #[tokio::test]
    async fn closed_crew_rejects_reregister() {
        // r2 (L0 ruling): a closed crew refuses resumes too — reregister is
        // register by another name, and a live lane on a closed crew strands
        // work no coordinator action can touch.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        unregister_lane(&h, &lane_id, &token).await;
        let (status, _) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/reregister",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "relay_conflict");
        // Still unregistered: the 409 wrote nothing (heartbeat stays 410).
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(json!({"lane_id": lane_id})),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::GONE, "{body}");
        assert_eq!(body["error"], "relay_lane_gone");
    }

    #[tokio::test]
    async fn close_is_idempotent() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(json!({"lane_id": coord.0})),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["status"], "closed");
    }

    // ── Crew-change + run-lifecycle SSE ──

    /// The first event matching `entity` + `op`, skipping others (a
    /// mutation can emit several: a lane event plus its tasks'). Fails
    /// naming everything seen when none arrives.
    async fn event_for(
        rx: &mut tokio::sync::broadcast::Receiver<crate::api::events::StateEvent>,
        entity: &str,
        op: &str,
    ) -> Value {
        let mut seen = Vec::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Ok(event)) => {
                    let json = serde_json::to_value(&event).unwrap();
                    if json["entity"] == entity && json["data"]["op"] == op {
                        return json;
                    }
                    seen.push(json);
                }
                _ => panic!("no {entity}/{op} event; saw {seen:?}"),
            }
        }
    }

    /// A lane event as #418 shapes it: the lane's row exactly as `/relay/now`
    /// shows it, plus the op, the lane id and (on suspend/resume) the reason.
    async fn lane_event(
        h: &Harness,
        action: &str,
        op: &str,
        lane_id: &str,
        crew_id: &str,
    ) -> Value {
        let (status, now) = call(h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::OK, "{now}");
        let mut data = now["lanes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == lane_id)
            .cloned()
            .unwrap_or_else(|| panic!("lane {lane_id} not on Now: {now}"));
        assert_eq!(data["crew_id"], crew_id);
        data["op"] = json!(op);
        data["lane_id"] = json!(lane_id);
        json!({"action": action, "entity": "lane", "data": data})
    }

    async fn lane_event_why(
        h: &Harness,
        action: &str,
        op: &str,
        lane_id: &str,
        crew_id: &str,
        reason: &str,
    ) -> Value {
        let mut event = lane_event(h, action, op, lane_id, crew_id).await;
        event["data"]["reason"] = json!(reason);
        event
    }

    #[tokio::test]
    async fn register_emits_lane_registered() {
        // The first live run's hole: a lane joined and the screen stayed
        // still. Join must fire, naming the lane and its crew.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let mut rx = h.state.event_tx.subscribe();
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let event = event_for(&mut rx, "lane", "registered").await;
        assert_eq!(event, lane_event(&h, "add", "registered", &lane_id, &coord.2).await);
    }

    #[tokio::test]
    async fn coordinator_register_emits_lane_registered() {
        // The crew-minting register is a join too.
        let h = harness().await;
        let mut rx = h.state.event_tx.subscribe();
        let coord = register_coordinator(&h).await;
        let event = event_for(&mut rx, "lane", "registered").await;
        assert_eq!(event, lane_event(&h, "add", "registered", &coord.0, &coord.2).await);
    }

    #[tokio::test]
    async fn unregister_emits_lane_unregistered() {
        // Leaving removes the row.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let event = event_for(&mut rx, "lane", "unregistered").await;
        // A removal names the row by its id: the screen drops it in place.
        assert_eq!(
            event,
            json!({"action": "remove", "entity": "lane",
                   "data": {"op": "unregistered", "id": lane_id, "lane_id": lane_id, "crew_id": coord.2}})
        );
    }

    #[tokio::test]
    async fn unregister_keeps_the_legacy_relay_op() {
        // The app's task stream re-reads on `relay`/`unregister` today;
        // the lane event rides beside it until the screen moves over.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let legacy = event_for(&mut rx, "relay", "unregister").await;
        assert_eq!(legacy["action"], "add");
    }

    #[tokio::test]
    async fn unregister_names_the_interrupted_task() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let event = event_for(&mut rx, "task", "interrupt").await;
        assert_eq!(event["action"], "update");
        assert_eq!(event["data"]["task_id"], task_id);
    }

    #[tokio::test]
    async fn reregister_emits_lane_reregistered() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/unregister",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/reregister",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let event = event_for(&mut rx, "lane", "reregistered").await;
        assert_eq!(event, lane_event(&h, "add", "reregistered", &lane_id, &coord.2).await);
    }

    #[tokio::test]
    async fn limit_pause_emits_lane_suspended() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) =
            limit_pause(&h, &lane_id, &token, "daily", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        let event = event_for(&mut rx, "lane", "suspended").await;
        assert_eq!(
            event,
            lane_event_why(&h, "update", "suspended", &lane_id, &coord.2, "limit_pause").await
        );
    }

    #[tokio::test]
    async fn heartbeat_on_a_paused_lane_emits_nothing() {
        // F03 (queue cutover): a beat ends no pause, so it announces no
        // resume — not even with an unknown resume time.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let (status, _) =
            limit_pause(&h, &lane_id, &token, "daily", serde_json::Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(rx.try_recv().is_err(), "a beat on a paused lane emitted an event");
    }

    #[tokio::test]
    async fn plain_heartbeat_emits_nothing() {
        // A routine renewal changes nothing the screen shows: no event
        // every 60 s per lane.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(rx.try_recv().is_err(), "a plain beat emitted an event");
    }

    #[tokio::test]
    async fn sweep_lease_loss_emits_lane_suspended_then_beat_resumes() {
        // Lease loss: the sweeper expires the lane (row → suspended), then
        // a late beat re-grants the lease (row → resumed).
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let mut rx = h.state.event_tx.subscribe();
        let past_lease = chrono::Utc::now() + chrono::Duration::seconds(301);
        crate::tasks::relay_lease_sweep::sweep_at(&h.state.pg, &h.state.event_tx, past_lease)
            .await
            .unwrap();
        // Every lane's lease lapsed: one suspend per lane, the
        // coordinator's and this one's.
        let mut suspended = vec![
            event_for(&mut rx, "lane", "suspended").await,
            event_for(&mut rx, "lane", "suspended").await,
        ];
        suspended.sort_by_key(|e| e["data"]["lane_id"].as_str().unwrap().to_string());
        let mut expected = vec![
            lane_event_why(&h, "update", "suspended", &coord.0, &coord.2, "lease_expired").await,
            lane_event_why(&h, "update", "suspended", &lane_id, &coord.2, "lease_expired").await,
        ];
        expected.sort_by_key(|e| e["data"]["lane_id"].as_str().unwrap().to_string());
        assert_eq!(suspended, expected);
        let mut rx = h.state.event_tx.subscribe();
        let (status, _) = call(
            &h,
            "POST",
            "/api/relay/heartbeat",
            Some(lane_body(&lane_id, json!({}))),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let event = event_for(&mut rx, "lane", "resumed").await;
        assert_eq!(
            event,
            lane_event_why(&h, "update", "resumed", &lane_id, &coord.2, "lease_regranted").await
        );
    }

    #[tokio::test]
    async fn sweep_names_the_interrupted_task() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, _) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        let mut rx = h.state.event_tx.subscribe();
        let past_lease = chrono::Utc::now() + chrono::Duration::seconds(301);
        crate::tasks::relay_lease_sweep::sweep_at(&h.state.pg, &h.state.event_tx, past_lease)
            .await
            .unwrap();
        let event = event_for(&mut rx, "task", "interrupt").await;
        assert_eq!(event["action"], "update");
        assert_eq!(event["data"]["task_id"], task_id);
    }

    #[tokio::test]
    async fn sweep_with_nothing_lapsed_emits_nothing() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let _ = register_contributor(&h, &coord.2).await;
        let mut rx = h.state.event_tx.subscribe();
        crate::tasks::relay_lease_sweep::sweep_at(
            &h.state.pg,
            &h.state.event_tx,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
        assert!(rx.try_recv().is_err(), "an idle sweep emitted an event");
    }

    #[tokio::test]
    async fn close_crew_emits_run_completed() {
        // CONTRACT §3k: the run (the crew's lifecycle) completes on close.
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let mut rx = h.state.event_tx.subscribe();
        let (status, body) = call(
            &h,
            "POST",
            &format!("/api/relay/crews/{}/close", coord.2),
            Some(lane_body(&coord.0, json!({}))),
            Some(&coord.1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let event = event_for(&mut rx, "run", "completed").await;
        assert_eq!(
            event,
            json!({
                "action": "update", "entity": "run",
                "data": {"op": "completed", "run_id": coord.2, "crew_id": coord.2},
            })
        );
    }

    // ── failure paths ──

    #[tokio::test]
    async fn read_failure_is_500_never_empty() {
        let h = harness().await;
        h.state.pg.pool().close().await;
        let (status, body) = call(&h, "GET", "/relay/now", None, None).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert_eq!(body["error"], "relay_internal");
        // A failed read never answers 200-with-empty either.
        assert_ne!(status, StatusCode::OK);
    }
}
