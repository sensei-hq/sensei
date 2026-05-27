//! `/api/knowledge/*` — memory CRUD, proposals, outcomes, context assembly.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;
use crate::db::pg_store::{InsertMemory, OutcomeRow};

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

// ============================================================================
// GET /api/knowledge/memories?status=&scope=&project_id=&limit=
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    pub status:     Option<String>,
    pub scope:      Option<String>,
    pub project_id: Option<String>,
    pub limit:      Option<i64>,
}

pub(crate) async fn list_memories(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = match q.project_id {
        Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        None => None,
    };
    let rows = state.pg.list_memories(pid, q.status.as_deref(), q.scope.as_deref(), q.limit.unwrap_or(200))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "memories": rows })))
}

// ============================================================================
// GET /api/knowledge/memories/:id
// ============================================================================

pub(crate) async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let detail = state.pg.get_memory_detail(mid).await
        .map_err(|e| {
            if e.contains("not found") { err(StatusCode::NOT_FOUND, "memory not found") }
            else { err(StatusCode::INTERNAL_SERVER_ERROR, &e) }
        })?;
    Ok(Json(detail))
}

// ============================================================================
// GET /api/knowledge/context?project_id=&limit=&tags=csv
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct ContextQuery {
    pub project_id: String,
    pub limit:      Option<i64>,
    pub tags:       Option<String>,
}

pub(crate) async fn get_context(
    State(state): State<AppState>,
    Query(q): Query<ContextQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = uuid::Uuid::parse_str(&q.project_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?;
    let tags: Option<Vec<String>> = q.tags.map(|s|
        s.split(',').filter(|t| !t.trim().is_empty()).map(|t| t.trim().to_string()).collect()
    );
    let stack_ids = state.pg.get_project_stack_ids(&pid).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let blob = state.pg.assemble_context(pid, &stack_ids, tags.as_deref(), q.limit.unwrap_or(200))
        .await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(blob))
}

// ============================================================================
// POST /api/knowledge/proposals  — propose_memory
// POST /api/knowledge/memories   — save_memory (explicit)
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct MemoryBody {
    pub project_id:    Option<String>,
    pub scope:         String,
    pub scope_filter:  Option<String>,
    #[serde(rename = "type")]
    pub mtype:         String,
    pub title:         String,
    pub content:       String,
    pub impact:        Option<String>,
    #[serde(default)]
    pub tags:          Vec<String>,
    pub triage_signal: Option<String>,
}

async fn insert_with_status(
    state: AppState,
    body:  MemoryBody,
    status: &str,
    require_triage_signal: bool,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if body.title.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "title must not be empty"));
    }
    if body.content.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "content must not be empty"));
    }
    if body.scope == "stack" && body.scope_filter.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "scope_filter required for scope='stack'"));
    }
    if require_triage_signal && body.triage_signal.as_deref().unwrap_or("").is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "triage_signal required for proposals"));
    }
    let pid = match body.project_id {
        Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        None => None,
    };
    let id = state.pg.insert_memory(&InsertMemory {
        project_id:    pid,
        scope:         body.scope,
        scope_filter:  body.scope_filter,
        mtype:         body.mtype,
        title:         body.title,
        content:       body.content,
        impact:        body.impact,
        tags:          body.tags,
        triage_signal: body.triage_signal,
        status:        status.into(),
    }).await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

pub(crate) async fn propose_memory(
    State(state): State<AppState>,
    Json(body):   Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "proposed", true).await
}

pub(crate) async fn save_memory(
    State(state): State<AppState>,
    Json(body):   Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    insert_with_status(state, body, "active", false).await
}

// ============================================================================
// POST /api/knowledge/proposals/:id/accept
// POST /api/knowledge/proposals/:id/reject
// ============================================================================

pub(crate) async fn accept_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let new_status = state.pg.set_memory_status(mid, "active", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
}

#[derive(Deserialize)]
pub(crate) struct RejectBody {
    pub reason: Option<String>,
}

pub(crate) async fn reject_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    if let Some(reason) = body.reason.as_deref().filter(|s| !s.trim().is_empty()) {
        tracing::info!(memory_id = %mid, reason, "proposal rejected");
    }
    let new_status = state.pg.set_memory_status(mid, "rejected", &["proposed"]).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match new_status {
        Some(s) => Ok(Json(serde_json::json!({ "id": mid, "status": s }))),
        None => Err(err(StatusCode::CONFLICT, "proposal not in 'proposed' state")),
    }
}

// ============================================================================
// POST /api/knowledge/outcomes
// ============================================================================

#[derive(Deserialize)]
pub(crate) struct OutcomeBody {
    pub memory_id:  String,
    pub outcome:    String,
    pub session_id: Option<String>,
    pub context:    Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct OutcomesBatch {
    pub outcomes: Vec<OutcomeBody>,
}

pub(crate) async fn record_outcomes(
    State(state): State<AppState>,
    Json(body):   Json<OutcomesBatch>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let valid_outcomes = ["applied", "consulted", "violated", "ignored"];
    let mut rows: Vec<OutcomeRow> = Vec::with_capacity(body.outcomes.len());
    for o in body.outcomes {
        if !valid_outcomes.contains(&o.outcome.as_str()) {
            return Err(err(StatusCode::BAD_REQUEST, &format!("invalid outcome: {}", o.outcome)));
        }
        let mid = uuid::Uuid::parse_str(&o.memory_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "bad memory_id"))?;
        let sess = match o.session_id {
            Some(s) => Some(uuid::Uuid::parse_str(&s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad session_id"))?),
            None => None,
        };
        rows.push(OutcomeRow {
            memory_id: mid, session_id: sess, outcome: o.outcome, context: o.context,
        });
    }
    let total = rows.len();
    let skipped = state.pg.record_outcomes_batch(&rows).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({
        "recorded": total - skipped.len(),
        "skipped":  skipped,
    })))
}
