//! `/api/share-review/*` — the personal upstream review surface (C6).
//!
//! Before an approved memory-share batch fires upstream, the user sees exactly
//! what is about to leave — the redacted (dereferenced) text per item and which
//! items are held for residual identifier risk — so they can hold/edit before
//! publish (`docs/spec/screen/observatory-share-review.md`).
//!
//! - `GET  /api/share-review/next-batch` — the next approved-but-unsent batch
//!   with its per-item dereference PREVIEW (no publish).
//! - `POST /api/share-review/{batch}/publish` — run the confidentiality-gated
//!   contribute path for that batch (client-work dereference is mandatory and
//!   cannot be overridden — the strip is enforced in [`crate::dojo::contribute`]).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};

use crate::api::state::AppState;
use crate::collective::anonymize::ContributorIdentity;
use crate::dojo::contribute;

/// GET /api/share-review/next-batch — the next approved-but-unsent batch preview,
/// or `{ "batch": null }` when nothing is pending.
pub(crate) async fn next_batch(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let next = state.pg.next_unsent_approved_batch().await.map_err(internal)?;
    let Some((batch_id, _project_id, _decided_at)) = next else {
        return Ok(Json(serde_json::json!({ "batch": null })));
    };
    let contributor = ContributorIdentity {
        user_key: state.pg.get_or_create_contributor_key().await.map_err(internal)?,
    };
    let preview =
        contribute::preview_batch(&state.pg, batch_id, &contributor).await.map_err(internal)?;
    Ok(Json(serde_json::json!({ "batch": preview })))
}

/// POST /api/share-review/{batch}/publish — contribute the approved batch to its
/// routed Dōjō(s). Returns the per-item outcome (published / held / queued /
/// error). A non-approved or missing batch is a 4xx.
pub(crate) async fn publish_batch(
    State(state): State<AppState>,
    Path(batch): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let batch_id = uuid::Uuid::parse_str(batch.trim())
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad batch id"))?;
    let outcome = contribute::contribute_batch(&state.pg, state.gateway.clone(), batch_id)
        .await
        .map_err(|e| {
            // Missing / not-approved batch → 4xx (caller error); everything else 500.
            if e.contains("not found") {
                err(StatusCode::NOT_FOUND, &e)
            } else if e.contains("not 'approved'") {
                err(StatusCode::CONFLICT, &e)
            } else {
                tracing::error!(error = %e, batch = %batch_id, "share-review: contribute_batch failed");
                err(StatusCode::INTERNAL_SERVER_ERROR, "contribute failed")
            }
        })?;
    Ok(Json(serde_json::to_value(outcome).unwrap_or(serde_json::Value::Null)))
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn internal(e: String) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!(error = %e, "share-review: query failed");
    err(StatusCode::INTERNAL_SERVER_ERROR, "share-review query failed")
}
