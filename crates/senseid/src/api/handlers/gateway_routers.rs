//! `/api/gateway/routers/*` — read endpoints for the wizard's
//! Inference stage and any future router introspection UI.

use crate::api::state::AppState;
use crate::gateway_keys;
use crate::gateway_routers::{REGISTRY, find};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

/// `configured` and `key_error` for one router (#285): `configured` is `null`
/// when its key exists but could not be read, and `key_error` says why.
fn configured_fields(
    needs_key: bool,
    standing: &gateway_keys::KeyStanding,
) -> (serde_json::Value, serde_json::Value) {
    use gateway_keys::KeyStanding;
    use serde_json::Value;
    match (needs_key, standing) {
        (false, _) | (true, KeyStanding::Set(_)) => (Value::Bool(true), Value::Null),
        (true, KeyStanding::NotSet) => (Value::Bool(false), Value::Null),
        (true, KeyStanding::Unreadable(why)) => (Value::Null, Value::String(why.clone())),
    }
}

/// Every router's current key, read in one blocking task, for
/// `refresh_router_keys`. Shared by the set and clear handlers.
async fn router_key_snapshot()
-> Result<std::collections::HashMap<String, Option<String>>, (StatusCode, Json<serde_json::Value>)>
{
    tokio::task::spawn_blocking(|| {
        REGISTRY
            .iter()
            .map(|r| (r.id.to_string(), crate::api::gateway_init::keychain_api_key(r.id)))
            .collect()
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain refresh spawn failed: {e}") })),
        )
    })
}

/// GET /api/gateway/routers — every known router + `configured` flag
/// from Keychain.
pub(crate) async fn list_routers(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Keychain reads are blocking (quiet since #283) — run off-runtime.
    let entries = tokio::task::spawn_blocking(|| {
        REGISTRY
            .iter()
            .map(|r| {
                let standing = if r.needs_key {
                    gateway_keys::key_standing(r.id)
                } else {
                    gateway_keys::KeyStanding::NotSet
                };
                let (configured, key_error) = configured_fields(r.needs_key, &standing);
                serde_json::json!({
                    "id":           r.id,
                    "name":         r.name,
                    "providers":    r.providers,
                    "capabilities": r.capabilities,
                    "needs_key":    r.needs_key,
                    "configured":   configured,
                    "key_error":    key_error,
                })
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("router enumeration failed: {e}") })),
        )
    })?;
    Ok(Json(serde_json::json!({ "routers": entries })))
}

/// GET /api/gateway/routers/{id}/providers — providers fronted by
/// this router. Single-element today; future multi-provider routers
/// (Bedrock) return many.
pub(crate) async fn router_providers(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let router = find(&id)
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown router" }))))?;
    Ok(Json(serde_json::json!({ "providers": router.providers })))
}

/// GET /api/gateway/routers/{id}/models — models reachable through
/// this router. Walks the gateway's model registry filtered by
/// router id.
pub(crate) async fn router_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _router = find(&id)
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown router" }))))?;
    let models = state.gateway.list_models_for_router(&id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))
    })?;
    Ok(Json(serde_json::json!({ "models": models })))
}

/// GET /api/gateway/models — flat list of all models, each entry
/// router-qualified.
pub(crate) async fn list_all_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let models = state.gateway.list_models().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))
    })?;
    Ok(Json(serde_json::json!({ "models": models })))
}

#[derive(Deserialize)]
pub(crate) struct SetKeyBody {
    pub key: String,
}

/// POST /api/gateway/routers/{id}/key — store the router's API key
/// in the Keychain and refresh the gateway's in-memory RouterConfig so
/// the next request picks it up without a daemon restart.
pub(crate) async fn set_router_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SetKeyBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let router = find(&id)
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown router" }))))?;
    if !router.needs_key {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{} does not need a key", id) })),
        ));
    }
    if body.key.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "key must not be empty" })),
        ));
    }
    let id_clone = id.clone();
    let key_owned = body.key;
    // Saving a key is the user's own action, so it may prompt (#283).
    tokio::task::spawn_blocking(move || {
        gateway_keys::allow_prompt(|| gateway_keys::set_key(&id_clone, &key_owned))
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain spawn failed: {e}") })),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain write failed: {e}") })),
        )
    })?;
    let all_keys = router_key_snapshot().await?;
    state.gateway.refresh_router_keys(|id| all_keys.get(id).cloned().flatten()).await;
    Ok(Json(serde_json::json!({ "ok": true, "configured": true })))
}

/// DELETE /api/gateway/routers/{id}/key — remove the router's API
/// key from the Keychain and clear the in-memory RouterConfig entry so
/// subsequent requests do not use a stale key.
pub(crate) async fn clear_router_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let router = find(&id)
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown router" }))))?;
    if !router.needs_key {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{} does not need a key", id) })),
        ));
    }
    let id_clone = id.clone();
    // The user's own action, so it may prompt (#283).
    tokio::task::spawn_blocking(move || {
        gateway_keys::allow_prompt(|| gateway_keys::delete_key(&id_clone))
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain spawn failed: {e}") })),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("keychain delete failed: {e}") })),
        )
    })?;
    let all_keys = router_key_snapshot().await?;
    state.gateway.refresh_router_keys(|id| all_keys.get(id).cloned().flatten()).await;
    Ok(Json(serde_json::json!({ "ok": true, "configured": false })))
}

#[cfg(test)]
mod tests {
    use super::configured_fields;
    use crate::gateway_keys::KeyStanding;
    use serde_json::{Value, json};

    /// #285: the listing tells "not set" from "could not read".
    #[test]
    fn a_router_key_is_configured_unset_or_unreadable() {
        assert_eq!(configured_fields(false, &KeyStanding::NotSet), (json!(true), Value::Null));
        assert_eq!(
            configured_fields(true, &KeyStanding::Set("sk".into())),
            (json!(true), Value::Null)
        );
        assert_eq!(configured_fields(true, &KeyStanding::NotSet), (json!(false), Value::Null));
        assert_eq!(
            configured_fields(
                true,
                &KeyStanding::Unreadable("keychain access needs approval".into())
            ),
            (Value::Null, json!("keychain access needs approval"))
        );
    }
}
