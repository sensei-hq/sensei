//! `/api/gateway/chains/*` — read + role-assignment writes for the
//! Model Assignments wizard step (and the Settings/Inference tab).
//!
//! Reads project every active chain with its ordered models and (optional)
//! sensei role. Writes flip a single chain's role — the DDL's
//! unique-when-set index enforces one chain per role, so a conflicting
//! write comes back as a 409.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

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
    let chains = state.pg.list_chains_with_models().await
        .map_err(|e| {
            tracing::error!(error = %e, "list_chains_with_models failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e }))
            )
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
    let chain_id = uuid::Uuid::parse_str(&id).map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "chain id must be a uuid" }))
    ))?;
    if let Some(ref r) = body.role
        && !SUPPORTED_ROLES.contains(&r.as_str())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("role must be one of {:?}", SUPPORTED_ROLES),
                "supported": SUPPORTED_ROLES,
            }))
        ));
    }

    state.pg.set_chain_role(&chain_id, body.role.as_deref()).await
        .map_err(|e| {
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
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": e })),
                );
            }
            tracing::error!(error = %e, chain = %chain_id, "set_chain_role failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
