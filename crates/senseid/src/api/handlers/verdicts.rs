//! `/api/instruments/verdicts*` — usage verdict classifier (#90).
//!
//! Three endpoints:
//! - `POST /api/instruments/verdicts/classify` — run the classifier against
//!   one session's `activity.assistant_events` and upsert results. Idempotent;
//!   the Replay tab (#84) triggers this once per session view.
//! - `GET  /api/instruments/verdicts?session_id=…` — the timeline join for
//!   Replay.
//! - `GET  /api/instruments/verdicts/summary?session_id=…` — used/partial/
//!   ignored counts for the session, StatBlock shape.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::api::handlers::err;
use crate::api::state::AppState;
use crate::tasks::verdict_classifier;

#[derive(Deserialize)]
pub(crate) struct SessionQuery {
    pub session_id: String,
}

#[derive(Deserialize)]
pub(crate) struct ClassifyBody {
    /// Session id (the `activity.assistant_events.session_id` string, not the
    /// `activity.sessions.id` UUID). Absent → classify every session (heavy;
    /// use sparingly).
    pub session_id: Option<String>,
}

/// POST /api/instruments/verdicts/classify
pub(crate) async fn classify(
    State(state): State<AppState>,
    Json(body): Json<ClassifyBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let Some(sid) = body.session_id else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "session_id required (fleet-wide backfill is a separate task, not this endpoint)",
        ));
    };
    let n = verdict_classifier::classify_session(&state.pg, &sid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({"session_id": sid, "verdicts_written": n})))
}

/// GET /api/instruments/verdicts?session_id=…
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(q): Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows = state
        .pg
        .get_verdicts_for_session(&q.session_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({"verdicts": rows})))
}

/// GET /api/instruments/verdicts/summary?session_id=…
pub(crate) async fn summary(
    State(state): State<AppState>,
    Query(q): Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sum = state
        .pg
        .get_verdict_summary_for_session(&q.session_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(sum))
}
