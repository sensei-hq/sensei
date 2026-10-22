//! Corrections aggregation read API (#65 step 5).

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::api::state::AppState;

/// GET /api/corrections — global recurring-corrections list.
pub(crate) async fn list_corrections(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.pg.list_corrections().await.map_err(|e| {
        tracing::error!("list_corrections: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(data))
}

/// GET /api/workspaces/{id}/corrections — corrections touching a workspace.
pub(crate) async fn workspace_corrections(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Name-or-uuid: AI callers pass a workspace name; raw Uuid::parse_str 400s on
    // it (silent empty). Resolve → 404 when no such workspace (#100).
    let workspace_id = crate::api::util::resolve_workspace_uuid(&state, &id)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.list_corrections_for_workspace(&workspace_id).await.map_err(|e| {
        tracing::error!("workspace_corrections: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(data))
}
