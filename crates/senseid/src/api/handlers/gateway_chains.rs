//! `/api/gateway/chains/*` — read + role-assignment writes for the
//! Model Assignments wizard step (and the Settings/Inference tab).
//!
//! Reads project every active chain with its ordered models and (optional)
//! sensei role. Writes flip a single chain's role — the DDL's
//! unique-when-set index enforces one chain per role, so a conflicting
//! write comes back as a 409.

use crate::api::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

/// Supported sensei roles for chain assignment. `default_fallback` is
/// intentionally omitted from the UI-facing surface; the enum still
/// carries it for legacy compatibility but no chain should ever be
/// assigned to it via this handler.
const SUPPORTED_ROLES: [&str; 4] = ["inference", "consolidation", "embedding", "voice"];

/// GET /api/gateway/chains — every active chain, each row including its
/// optional role and its ordered model members.
pub(crate) async fn list_chains(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let chains = state.pg.list_chains_with_models().await.map_err(|e| {
        tracing::error!(error = %e, "list_chains_with_models failed");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))
    })?;
    Ok(Json(serde_json::json!({ "chains": chains })))
}

#[derive(Deserialize)]
pub(crate) struct SetRoleBody {
    /// New role for this chain, or `null` to clear.
    pub role: Option<String>,
}

/// PUT /api/gateway/chains/{id}/role — assign or clear the sensei
/// inference role a chain serves. 404 when the chain id is unknown,
/// 400 when the role isn't in the supported set, 409 when another
/// chain already owns that role (unique-index violation).
pub(crate) async fn set_chain_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SetRoleBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let chain_id = uuid::Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "chain id must be a uuid" })))
    })?;
    if let Some(ref r) = body.role
        && !SUPPORTED_ROLES.contains(&r.as_str())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("role must be one of {:?}", SUPPORTED_ROLES),
                "supported": SUPPORTED_ROLES,
            })),
        ));
    }

    state.pg.set_chain_role(&chain_id, body.role.as_deref()).await.map_err(|e| {
        // Map the DB-level uniqueness violation to 409 CONFLICT so
        // the wizard can render a "already assigned" banner instead
        // of a generic 500.
        if e.contains("duplicate") || e.contains("unique") {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "another chain already owns this role",
                    "detail": e,
                })),
            );
        }
        if e.contains("not found") {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e })));
        }
        tracing::error!(error = %e, chain = %chain_id, "set_chain_role failed");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))
    })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/gateway/chains/{id}/available-models — models with matching
/// capability, reachable via `models_in_router`, that aren't already
/// members of this chain. Powers the wizard's "Available models" picker.
pub(crate) async fn list_available_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let chain_id = uuid::Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "chain id must be a uuid" })))
    })?;
    let models = state.pg.list_available_models_for_chain(&chain_id).await.map_err(|e| {
        tracing::error!(error = %e, chain = %chain_id, "list_available_models_for_chain failed");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))
    })?;
    Ok(Json(serde_json::json!({ "models": models })))
}

#[derive(Deserialize)]
pub(crate) struct AddModelBody {
    #[serde(alias = "model_id")]
    pub model_id: uuid::Uuid,
    #[serde(alias = "router_id")]
    pub router_id: uuid::Uuid,
}

/// POST /api/gateway/chains/{id}/models — append a model to the end of
/// a chain's ordered list. Body: `{model_id, router_id}`. Returns the
/// new member id + assigned sequence_order so the caller can rehydrate
/// its optimistic UI. 400 on unknown chain / unreachable model pair,
/// 500 otherwise.
pub(crate) async fn add_chain_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AddModelBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let chain_id = uuid::Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "chain id must be a uuid" })))
    })?;
    let (member_id, sequence_order) = state
        .pg
        .add_chain_model(&chain_id, &body.model_id, &body.router_id)
        .await
        .map_err(|e| chain_member_err_to_status(e, chain_id))?;
    Ok(Json(serde_json::json!({
        "ok":            true,
        "memberId":      member_id,
        "sequenceOrder": sequence_order,
    })))
}

/// DELETE /api/gateway/chains/{id}/models/{member_id} — remove a
/// specific member and compact the remaining rows so sequence stays
/// contiguous. 404 on unknown member.
pub(crate) async fn remove_chain_model(
    State(state): State<AppState>,
    Path((id, member_id_str)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let chain_id = uuid::Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "chain id must be a uuid" })))
    })?;
    let member_id = uuid::Uuid::parse_str(&member_id_str).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "member id must be a uuid" })))
    })?;
    state
        .pg
        .remove_chain_model(&chain_id, &member_id)
        .await
        .map_err(|e| chain_member_err_to_status(e, chain_id))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(crate) struct MoveBody {
    /// -1 for up, +1 for down. Anything else → 400.
    pub direction: i32,
}

/// PUT /api/gateway/chains/{id}/models/{member_id}/move — swap this
/// member with its neighbour. Returns `moved: false` when at a
/// boundary (no-op, not an error) so the UI can dim the arrow.
pub(crate) async fn move_chain_model(
    State(state): State<AppState>,
    Path((id, member_id_str)): Path<(String, String)>,
    Json(body): Json<MoveBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let chain_id = uuid::Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "chain id must be a uuid" })))
    })?;
    let member_id = uuid::Uuid::parse_str(&member_id_str).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "member id must be a uuid" })))
    })?;
    let moved =
        state.pg.move_chain_model(&chain_id, &member_id, body.direction).await.map_err(|e| {
            if e.contains("direction") {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })));
            }
            chain_member_err_to_status(e, chain_id)
        })?;
    Ok(Json(serde_json::json!({ "ok": true, "moved": moved })))
}

/// Shared error-mapper for chain-member writes. Maps `not found` /
/// `not reachable` to 400/404, everything else to 500 with logging.
fn chain_member_err_to_status(
    e: String,
    chain_id: uuid::Uuid,
) -> (StatusCode, Json<serde_json::Value>) {
    if e.contains("not found") {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e })));
    }
    if e.contains("not reachable") {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })));
    }
    tracing::error!(error = %e, chain = %chain_id, "chain-member write failed");
    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))
}
