use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct RecoQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct SessionsQuery {
    limit: Option<i64>,
}

pub(crate) async fn get_project_ftr(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_ftr(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_repos(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let repos = state.pg.get_project_repos(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "repos": repos })))
}

pub(crate) async fn get_project_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_drift(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_patterns(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_patterns(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "get_project_patterns failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(data))
}

pub(crate) async fn get_project_libraries(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let libs = state.pg.get_project_libraries(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "libraries": libs })))
}

pub(crate) async fn get_project_instruments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let tools = state.pg.get_project_extensions(&uuid, Some(&["skill", "command", "agent"])).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "tools": tools })))
}

pub(crate) async fn get_project_memories(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_memories(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<RecoQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let recs = state.pg.get_project_recommendations(&uuid, q.status.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(recs)))
}

/// GET /api/projects/{id}/impact — acted-on / consolidation recommendations
/// joined to their reasoning trace (before/after FTR + MOE reasoning). Powers
/// the Observatory Impact view (#70 read-path).
pub(crate) async fn get_project_impact(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state.pg.get_project_impact(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "get_project_impact failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!(data)))
}

/// GET /api/projects/{id}/library-version-conflicts — per-library version drift
/// across the project's folders (excluding local-protocol deps). Powers the
/// Track 3 Libraries screen "version conflicts" signal.
pub(crate) async fn get_project_library_version_conflicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let conflicts = state.pg.list_project_library_version_conflicts(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "list_project_library_version_conflicts failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({ "conflicts": conflicts })))
}

/// GET /api/projects/{id}/project-deps — outgoing project → project edges
/// detected from local-path protocols (npm link:/workspace:/file:,
/// Cargo path=). Powers the Track 3 Libraries screen "depends on other
/// project" section.
pub(crate) async fn get_project_project_deps(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let deps = state.pg.list_project_dependencies(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "list_project_dependencies failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({ "dependencies": deps })))
}

pub(crate) async fn get_project_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let limit = q.limit.unwrap_or(50);
    let sessions = state.pg.list_sessions_by_project(&uuid, limit).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

// ── Memory share batches ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct BatchListQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct BatchCreateBody {
    memory_ids: Vec<uuid::Uuid>,
    note: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct BatchDecisionBody {
    status: String,
    note: Option<String>,
}

/// GET /api/projects/{id}/memory-batches?status=proposed
pub(crate) async fn list_memory_share_batches(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<BatchListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let batches = state.pg.list_memory_share_batches(&uuid, q.status.as_deref()).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_memory_share_batches failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "batches": batches })))
}

/// POST /api/projects/{id}/memory-batches
pub(crate) async fn create_memory_share_batch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<BatchCreateBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if body.memory_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let batch_id = state.pg
        .create_memory_share_batch(&uuid, &body.memory_ids, body.note.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "create_memory_share_batch failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "id": batch_id })))
}

/// PUT /api/projects/{id}/memory-batches/{batch_id}
pub(crate) async fn decide_memory_share_batch(
    State(state): State<AppState>,
    Path((id, batch_id)): Path<(String, String)>,
    Json(body): Json<BatchDecisionBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _project_uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let batch_uuid = uuid::Uuid::parse_str(&batch_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg
        .set_memory_share_batch_status(&batch_uuid, &body.status, body.note.as_deref())
        .await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else if e.contains("invalid status") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, batch = %batch_uuid, "set_memory_share_batch_status failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
