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
//! | lane | `POST /api/relay/crews/{id}/close` | close the crew (coordinator) |
//! | person | `GET /relay/now`, `GET /relay/queue`, `GET /relay/tasks/{crew}/{task}` | reads |
//! | person | `POST /relay/messages`, `POST /relay/answers`, `POST /relay/backlog` | writes |
//! | person | `POST /relay/triage` | add a manual triage item (derived: nothing
//! | | else creates one in slice 1 — the tracker pull is slice 2) |
//!
//! Lane calls authenticate over the `Authorization: Bearer <lane_token>`
//! header and name their `lane_id` in the query (GET) or body (POST); the
//! token never appears anywhere else. Person calls ride the daemon's
//! localhost trust; the daemon user comes from `relay_current_user` (Q19).
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
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::events::StateEvent;
use crate::api::state::AppState;
use crate::db::pg_store::relay::{
    RelayAnswerInput, RelayAskInput, RelayAskedOf, RelayAssignInput, RelayCaller,
    RelayDecisionOption, RelayFileInput, RelayFindingInput, RelayGateInput, RelayHeartbeatOutput,
    RelayLaneSession, RelayMergeInput, RelayMessageInput, RelayMessageTo, RelayPlanInput,
    RelayReadFilter, RelayRegisterInput, RelayReportInput, RelayResourceRequest, RelayReviewInput,
    RelayTicketInput, RelayTriageInput,
};
use crate::relay::envelope::{LaneEnvelope, empty_envelope};
use crate::relay::{
    RelayError, RelayFindingSeverity, RelayItemKind, RelayRole, RelaySurface, RelayTaskPhase,
    RelayTicketProvider, ReviewVerdict, clamp_wait_timeout,
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
        | RelayError::NotImplemented(m) => m.clone(),
        RelayError::SignInRequired => "this machine never signed into a dōjō".to_string(),
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

/// The person behind a person route (Q19): fail-closed 401 when the
/// machine never signed in.
async fn person_user(state: &AppState) -> Result<Uuid, RelayError> {
    state.pg.relay_current_user().await
}

fn emit(state: &AppState, op: &str) {
    let _ = state.event_tx.send(StateEvent::relay(op));
}

// ─── Wire bodies ───

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterBody {
    role: String,
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
pub(crate) struct ResourceBody {
    lane_id: String,
    resource: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TasksQuery {
    lane_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WaitQuery {
    lane_id: String,
    timeout_ms: Option<u64>,
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
    reviewer_lane: Option<String>,
    lane_title: Option<String>,
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
                role: parse_word::<RelayRole>(&body.role, "role").map_err(err)?,
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
            },
            chrono::Utc::now(),
        )
        .await
        .map_err(err)?;
    ok(serde_json::json!({
        "lane_token": out.lane_token,
        "lane_id": out.lane_id.to_string(),
        "lane_title": out.lane_title,
        "crew_id": out.crew_id.to_string(),
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
    let RelayHeartbeatOutput { expires_at } = state
        .pg
        .relay_heartbeat(parse_uuid(&body.lane_id, "lane_id").map_err(err)?, chrono::Utc::now())
        .await
        .map_err(err)?;
    let _ = session;
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
    let _session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let lane_id = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    state.pg.relay_unregister(lane_id).await.map_err(err)?;
    emit(&state, "unregister");
    ok(serde_json::json!({ "lane_id": body.lane_id }))
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
    let hold = clamp_wait_timeout(query.timeout_ms);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(hold);
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
            return Ok(Json(envelope));
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Ok(Json(empty_envelope()));
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
                reviewer_lane: body
                    .reviewer_lane
                    .as_deref()
                    .map(|r| parse_uuid(r, "reviewer_lane"))
                    .transpose()
                    .map_err(err)?,
                lane_title: body.lane_title,
            },
        )
        .await
        .map_err(err)?;
    emit(&state, "assign");
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
    emit(&state, "report");
    ok(serde_json::json!({
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
        "round": out.round,
        "reviewer_lane_id": out.reviewer_lane_id.map(|id| id.to_string()),
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
    emit(&state, "review");
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
    let out = state
        .pg
        .relay_ask_decision(
            session.user_id,
            caller,
            RelayAskInput {
                task_id: body
                    .task_id
                    .as_deref()
                    .map(|t| parse_uuid(t, "task_id"))
                    .transpose()
                    .map_err(err)?,
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
    emit(&state, "ask");
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
    let action_id = state
        .pg
        .relay_answer_decision(
            session.user_id,
            RelayCaller::Lane { lane_id: caller },
            decision_id,
            RelayAnswerInput { option: body.option, note: body.note },
        )
        .await
        .map_err(err)?;
    emit(&state, "answer");
    ok(serde_json::json!({ "action_id": action_id.to_string() }))
}

pub(crate) async fn lane_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LaneMessageBody>,
) -> RelayResult {
    let session = auth_lane(&state, &body.lane_id, &headers).await.map_err(err)?;
    let caller = parse_uuid(&body.lane_id, "lane_id").map_err(err)?;
    let out = state
        .pg
        .relay_send_message(
            session.user_id,
            RelayCaller::Lane { lane_id: caller },
            RelayMessageInput {
                to: message_to(&body.to).map_err(err)?,
                task_id: body
                    .task_id
                    .as_deref()
                    .map(|t| parse_uuid(t, "task_id"))
                    .transpose()
                    .map_err(err)?,
                text: body.text,
            },
        )
        .await
        .map_err(err)?;
    emit(&state, "message");
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
    emit(&state, "merge");
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
    emit(&state, "cancel");
    ok(serde_json::json!({
        "task_id": out.task_id.to_string(),
        "state": serde_json::to_value(out.state).unwrap_or(serde_json::Value::Null),
    }))
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
    emit(&state, "close");
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
    let out = state
        .pg
        .relay_send_message(
            user_id,
            RelayCaller::Person { surface },
            RelayMessageInput {
                to: message_to(&body.to).map_err(err)?,
                task_id: body
                    .task_id
                    .as_deref()
                    .map(|t| parse_uuid(t, "task_id"))
                    .transpose()
                    .map_err(err)?,
                text: body.text,
            },
        )
        .await
        .map_err(err)?;
    emit(&state, "message");
    ok(serde_json::json!({ "message_id": out.message_id.to_string() }))
}

pub(crate) async fn person_answer(
    State(state): State<AppState>,
    Json(body): Json<PersonAnswerBody>,
) -> RelayResult {
    let user_id = person_user(&state).await.map_err(err)?;
    let surface = parse_word::<RelaySurface>(&body.surface, "surface").map_err(err)?;
    let action_id = state
        .pg
        .relay_answer_decision(
            user_id,
            RelayCaller::Person { surface },
            parse_uuid(&body.decision_id, "decision_id").map_err(err)?,
            RelayAnswerInput { option: body.option, note: body.note },
        )
        .await
        .map_err(err)?;
    emit(&state, "answer");
    ok(serde_json::json!({ "action_id": action_id.to_string() }))
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
            let id = state
                .pg
                .relay_backlog_add(
                    user_id,
                    RelayCaller::Person { surface },
                    parse_uuid(&item_id, "item_id").map_err(err)?,
                )
                .await
                .map_err(err)?;
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
        // The daemon under test runs on a signed-in machine (Q19): without
        // a default persona, register and the person routes fail closed.
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
    async fn register_contributor_without_crew_is_422() {
        let h = harness().await;
        let (status, body) = call(
            &h,
            "POST",
            "/api/relay/register",
            Some(json!({"role": "contributor", "family": "claude-code"})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"], "relay_invalid");
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

    fn report_body(lane_id: &str) -> Value {
        lane_body(
            lane_id,
            json!({
                "phase": "tests", "round": 1,
                "summary": {"changed": "Wired it.", "look_at": "app/", "flags": null},
                "files": [{"path": "app/a.ts", "added": 1, "removed": 0}],
            }),
        )
    }

    #[tokio::test]
    async fn report_then_review_then_merge_flow() {
        let h = harness().await;
        let coord = register_coordinator(&h).await;
        let (lane_id, token) = register_contributor(&h, &coord.2).await;
        let task_id = assigned_task(&h, &coord, &lane_id).await;
        // Delivery marks started; the report moves to in_review…
        let _ = call(&h, "GET", &format!("/api/relay/tasks?lane_id={lane_id}"), None, Some(&token))
            .await;
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
        // …the coordinator (picked reviewer on a two-lane crew) approves…
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
        assert_eq!(t["state"], "assigned");
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
