use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct LogBody {
    level: String,
    running_on: String,
    logged_at: String,
    message: Option<String>,
    context: Option<serde_json::Value>,
    data: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

/// POST /api/logs — ingest a structured log entry from CLI, MCP, or app.
pub(crate) async fn ingest_log(
    State(state): State<AppState>,
    Json(body): Json<LogBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.pg.insert_log(
        &body.level,
        &body.running_on,
        &body.logged_at,
        body.message.as_deref().unwrap_or(""),
        &body.context.unwrap_or(serde_json::json!({})),
        &body.data,
        &body.error,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"ok": true})))
}
