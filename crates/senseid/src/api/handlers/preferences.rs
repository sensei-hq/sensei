//! `/api/preferences/collective` — the Preferences → Sharing screen's backend.
//!
//! GET returns the current collective sharing preferences (or conservative
//! defaults when nothing is stored yet, so the screen always has a shape); PUT
//! validates + upserts them. Domain types, enum validation, and defaults live in
//! [`crate::collective::preferences`]; the single-row storage is in
//! `sensei.collective_preferences`.

use axum::{extract::State, http::StatusCode, response::Json};

use crate::api::state::AppState;
use crate::collective::preferences::{self, CollectivePreferences};

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

/// GET /api/preferences/collective — current preferences, or defaults when unset.
pub(crate) async fn get_collective(
    State(state): State<AppState>,
) -> Result<Json<CollectivePreferences>, (StatusCode, Json<serde_json::Value>)> {
    let prefs = preferences::get(&state.pg)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(prefs))
}

/// PUT /api/preferences/collective — validate + upsert the preferences, returning
/// the saved shape. Rejects an unknown destination / cadence / attribution /
/// category with 400 (no silent bad writes).
pub(crate) async fn put_collective(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<CollectivePreferences>, (StatusCode, Json<serde_json::Value>)> {
    let prefs =
        CollectivePreferences::from_request(&body).map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;
    let saved = preferences::set(&state.pg, prefs)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(saved))
}
