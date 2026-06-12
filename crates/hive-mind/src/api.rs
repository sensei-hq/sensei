//! Axum router + handlers for the `/v1` federation API.

use crate::auth::{require, role_satisfies, AuthCaller, Role};
use crate::store::HiveStore;
use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use hive_protocol::PublishedRule;
use serde::Deserialize;
use std::sync::Arc;

pub struct SharedState {
    pub store: HiveStore,
}
pub type AppState = Arc<SharedState>;

fn err(code: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (code, Json(serde_json::json!({ "error": msg })))
}
fn require_role(
    caller: &AuthCaller,
    floor: Role,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if role_satisfies(caller.role, floor) {
        Ok(())
    } else {
        Err(err(StatusCode::FORBIDDEN, "insufficient role"))
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "sensei-hive", "scope": "hive" }))
}

async fn publish_rule(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    Json(mut rule): Json<PublishedRule>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Publisher)?;
    // Attribution comes from the authenticated caller, not the request body —
    // a publisher can't spoof who published a rule. (published_at is stamped
    // server-side in the store.)
    rule.published_by = caller.name.clone();
    let resp = state
        .store
        .publish(&rule)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    // Audit-write failures are intentionally swallowed on the request path: the
    // publish already succeeded, and a failed audit insert shouldn't fail it.
    // (keygen.rs uses `?` instead — a failed audit there fails key creation,
    // acceptable for the one-off CLI bootstrap.)
    let _ = state
        .store
        .record_audit(
            Some(&caller.member_id),
            "publish",
            Some(&resp.id),
            serde_json::json!({ "version": resp.version, "seq": resp.seq }),
        )
        .await;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

#[derive(Deserialize)]
struct PullQuery {
    since: Option<i64>,
}
async fn pull_rules(
    State(state): State<AppState>,
    Extension(_c): Extension<AuthCaller>,
    Query(q): Query<PullQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let page = state
        .store
        .pull_since(q.since.unwrap_or(0))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::to_value(page).unwrap()))
}

async fn retract_rule(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Publisher)?;
    let ok = state
        .store
        .retract(&id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state
        .store
        .record_audit(
            Some(&caller.member_id),
            "retract",
            Some(&id),
            serde_json::json!({}),
        )
        .await;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(err(StatusCode::NOT_FOUND, "no such rule"))
    }
}

#[derive(Deserialize)]
struct NewMember {
    name: String,
    email: Option<String>,
    role: String,
}
async fn add_member(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    Json(m): Json<NewMember>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Admin)?;
    if Role::parse(&m.role).is_none() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "role must be member|publisher|admin",
        ));
    }
    let id = state
        .store
        .create_member(&m.name, m.email.as_deref(), &m.role)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state
        .store
        .record_audit(
            Some(&caller.member_id),
            "member.add",
            Some(&id.to_string()),
            serde_json::json!({ "role": m.role }),
        )
        .await;
    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

#[derive(Deserialize)]
struct NewKey {
    label: Option<String>,
}
async fn issue_key(
    State(state): State<AppState>,
    Extension(caller): Extension<AuthCaller>,
    axum::extract::Path(member_id): axum::extract::Path<String>,
    Json(k): Json<NewKey>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_role(&caller, Role::Admin)?;
    let mid = uuid::Uuid::parse_str(&member_id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad member id"))?;
    let issued = state
        .store
        .issue_key(&mid, k.label.as_deref())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let _ = state
        .store
        .record_audit(
            Some(&caller.member_id),
            "key.issue",
            Some(&issued.key_id),
            serde_json::json!({ "member_id": member_id }),
        )
        .await;
    Ok(Json(
        serde_json::json!({ "key_id": issued.key_id, "api_key": issued.plaintext }),
    ))
}

async fn subscriptions_stub(
    Extension(_c): Extension<AuthCaller>,
) -> (StatusCode, Json<serde_json::Value>) {
    err(
        StatusCode::NOT_IMPLEMENTED,
        "webhook subscriptions not yet implemented",
    )
}

pub fn build_router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/v1/rules", post(publish_rule).get(pull_rules))
        .route("/v1/rules/{id}", delete(retract_rule))
        .route("/v1/members", post(add_member))
        .route("/v1/members/{id}/keys", post(issue_key))
        .route("/v1/subscriptions", post(subscriptions_stub))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), require));
    Router::new()
        .route("/v1/health", get(health))
        .merge(protected)
        .with_state(state)
}
