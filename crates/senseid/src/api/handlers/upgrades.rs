//! `/api/upgrades` — the daemon's DOWNSTREAM inbox (C7): approved Dōjō / Collective
//! artifacts pulled back for the user to review and Apply.
//!
//! Shape per `docs/spec/screen/observatory-upgrades.md`:
//! - `GET  /api/upgrades[?include_muted=1]` — the inbox across memberships
//!   (`{ artifacts: [...], unread_count: N }`); pinned float to the top, muted are
//!   hidden unless `include_muted`.
//! - `POST /api/upgrades/{id}/apply` — land a principle/pattern into
//!   `sensei.memories` (origin='dojo'); skill/agent/prompt/guard defer (no-op with
//!   a reason). Idempotent.
//! - `POST /api/upgrades/{id}/mute` | `/pin` — local overrides that never land.
//!
//! Wiring the Upgrades SCREEN to this endpoint is C11 (not this chunk).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::api::state::AppState;
use crate::collective::inbox::{self, ApplyOutcome};

type ApiResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Deserialize)]
pub(crate) struct UpgradesQuery {
    /// `?include_muted=1` (or `true`) surfaces muted artifacts too. Default: hide.
    include_muted: Option<String>,
}

/// GET /api/upgrades — the downstream inbox across all Dōjō memberships. Muted
/// items are hidden by default; pinned float to the top; `unread_count` is the
/// number of items still `pending`.
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(q): Query<UpgradesQuery>,
) -> ApiResult {
    let include_muted =
        q.include_muted.as_deref().is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let items = state.pg.list_dojo_inbox(include_muted).await.map_err(internal)?;
    let unread_count = inbox::unread_count(&items);
    Ok(Json(serde_json::json!({ "artifacts": items, "unread_count": unread_count })))
}

/// POST /api/upgrades/{id}/apply — apply one artifact (idempotent).
pub(crate) async fn apply(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult {
    let inbox_id = parse_id(&id)?;
    let outcome = inbox::apply(&state.pg, inbox_id).await.map_err(internal)?;
    if matches!(outcome, ApplyOutcome::NotFound) {
        return Err(err(StatusCode::NOT_FOUND, "upgrade not found"));
    }
    Ok(Json(serde_json::to_value(outcome).unwrap_or(serde_json::Value::Null)))
}

/// POST /api/upgrades/{id}/mute — suppress an artifact locally.
pub(crate) async fn mute(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult {
    let inbox_id = parse_id(&id)?;
    let found = inbox::mute(&state.pg, inbox_id).await.map_err(internal)?;
    state_response(inbox_id, "muted", found)
}

/// POST /api/upgrades/{id}/pin — float an artifact to the top / outrank ambiguous
/// local alternatives.
pub(crate) async fn pin(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult {
    let inbox_id = parse_id(&id)?;
    let found = inbox::pin(&state.pg, inbox_id).await.map_err(internal)?;
    state_response(inbox_id, "pinned", found)
}

fn state_response(inbox_id: uuid::Uuid, new_state: &str, found: bool) -> ApiResult {
    if !found {
        return Err(err(StatusCode::NOT_FOUND, "upgrade not found"));
    }
    Ok(Json(serde_json::json!({ "id": inbox_id.to_string(), "state": new_state })))
}

fn parse_id(id: &str) -> Result<uuid::Uuid, (StatusCode, Json<serde_json::Value>)> {
    uuid::Uuid::parse_str(id.trim()).map_err(|_| err(StatusCode::BAD_REQUEST, "bad upgrade id"))
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn internal(e: String) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!(error = %e, "upgrades: query failed");
    err(StatusCode::INTERNAL_SERVER_ERROR, "upgrades query failed")
}
