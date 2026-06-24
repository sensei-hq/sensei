//! Corrections aggregation read API (#65 step 5).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
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

/// GET /api/projects/{id}/corrections — corrections touching a project.
pub(crate) async fn project_corrections(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let data = state
        .pg
        .list_corrections_for_project(&project_id)
        .await
        .map_err(|e| {
            tracing::error!("project_corrections: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(data))
}
