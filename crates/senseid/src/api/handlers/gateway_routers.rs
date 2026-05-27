//! `/api/gateway/routers/*` — read endpoints for the wizard's
//! Inference stage and any future router introspection UI.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;
use crate::gateway_keys;
use crate::gateway_routers::{REGISTRY, find};

/// GET /api/gateway/routers — every known router + `configured` flag
/// from Keychain.
pub(crate) async fn list_routers(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Keychain reads are sync and may prompt — run off-runtime.
    let entries = tokio::task::spawn_blocking(|| {
        REGISTRY.iter().map(|r| serde_json::json!({
            "id":           r.id,
            "name":         r.name,
            "providers":    r.providers,
            "capabilities": r.capabilities,
            "needs_key":    r.needs_key,
            "configured":   !r.needs_key || gateway_keys::has_key(r.id),
        })).collect::<Vec<_>>()
    }).await.map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": format!("router enumeration failed: {e}") }))
    ))?;
    Ok(Json(serde_json::json!({ "routers": entries })))
}

/// GET /api/gateway/routers/{id}/providers — providers fronted by
/// this router. Single-element today; future multi-provider routers
/// (Bedrock) return many.
pub(crate) async fn router_providers(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let router = find(&id).ok_or((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "unknown router" }))
    ))?;
    Ok(Json(serde_json::json!({ "providers": router.providers })))
}

/// GET /api/gateway/routers/{id}/models — models reachable through
/// this router. Walks the gateway's model registry filtered by
/// router id.
pub(crate) async fn router_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _router = find(&id).ok_or((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "unknown router" }))
    ))?;
    let models = state.gateway.list_models_for_router(&id).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))
        ))?;
    Ok(Json(serde_json::json!({ "models": models })))
}

/// GET /api/gateway/models — flat list of all models, each entry
/// router-qualified.
pub(crate) async fn list_all_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let models = state.gateway.list_models().await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))
        ))?;
    Ok(Json(serde_json::json!({ "models": models })))
}

#[derive(Deserialize)]
pub(crate) struct SetKeyBody {
    pub key: String,
}

/// POST /api/gateway/routers/{id}/key — store the router's API key
/// in the Keychain. The gateway picks it up on the next request via
/// gateway_init::router_config_with_keychain (Task 6 wires the refresh).
pub(crate) async fn set_router_key(
    Path(id): Path<String>,
    Json(body): Json<SetKeyBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let router = find(&id).ok_or((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "unknown router" }))
    ))?;
    if !router.needs_key {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{} does not need a key", id) }))
        ));
    }
    if body.key.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "key must not be empty" }))
        ));
    }
    let id_clone = id.clone();
    let key_owned = body.key;
    tokio::task::spawn_blocking(move || {
        gateway_keys::set_key(&id_clone, &key_owned)
    }).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain spawn failed: {e}") }))
        ))?
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain write failed: {e}") }))
        ))?;
    Ok(Json(serde_json::json!({ "ok": true, "configured": true })))
}

/// DELETE /api/gateway/routers/{id}/key — remove the router's API
/// key from the Keychain.
pub(crate) async fn clear_router_key(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _router = find(&id).ok_or((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "unknown router" }))
    ))?;
    let id_clone = id.clone();
    tokio::task::spawn_blocking(move || {
        gateway_keys::delete_key(&id_clone)
    }).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain spawn failed: {e}") }))
        ))?
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain delete failed: {e}") }))
        ))?;
    Ok(Json(serde_json::json!({ "ok": true, "configured": false })))
}
