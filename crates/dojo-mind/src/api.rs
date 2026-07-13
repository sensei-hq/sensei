//! Axum router + handlers for the `/v1` federation API.

use crate::auth::{
    authenticate_dojo, require, role_satisfies, AuthCaller, DojoAccess, DojoAuthError, JwtConfig,
    Role,
};
use crate::collective::promote::{DecideOutcome, DecideStatus};
use crate::store::HiveStore;
use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use dojo_protocol::PublishedArtifact;
use hive_protocol::PublishedRule;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

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

// ── Tenant-scoped Dōjō artifact routes (dual-auth) ───────────────────────────

/// Map a dojo dual-auth failure onto a status code (401 / 403 / 500).
fn dojo_auth_status(e: DojoAuthError) -> (StatusCode, Json<serde_json::Value>) {
    match e {
        DojoAuthError::Unauthenticated => {
            err(StatusCode::UNAUTHORIZED, "authentication required")
        }
        DojoAuthError::Forbidden => err(StatusCode::FORBIDDEN, "not a member of this tenant"),
        DojoAuthError::Internal(m) => err(StatusCode::INTERNAL_SERVER_ERROR, &m),
    }
}

/// `POST /v1/t/{tenant_key}/artifacts` — publish (contribute) an artifact under
/// the path tenant. Dual-auth, contributor+. `tenant_key` is the url-encoded
/// discovery path (`<origin>/<org>[/<dojo>]`); unknown → 404.
async fn publish_artifact(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(artifact): Json<PublishedArtifact>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tenant_id = state
        .store
        .resolve_tenant(&tenant_key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such tenant"))?;
    let caller = authenticate_dojo(&state.store, &jwt, &headers, tenant_id)
        .await
        .map_err(dojo_auth_status)?;
    if caller.access < DojoAccess::Contributor {
        return Err(err(StatusCode::FORBIDDEN, "contributor role required"));
    }
    if !artifact.kind_matches_payload() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "artifact kind does not match payload",
        ));
    }
    // Attribution comes from the authenticated caller, not the body — a caller
    // can't spoof who contributed (mirrors the rules publish discipline).
    let contributed_by = Uuid::parse_str(&caller.subject).ok();
    let resp = state
        .store
        .publish_artifact(&tenant_id, &artifact, contributed_by)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    // Close the loop inline: triage this signature-cluster (auto-publish if it
    // clears the bar, else queue for a maintainer). Promotion failure must NOT
    // fail the already-committed contribution — surface it in the log instead
    // (a later publish or a maintainer promote sweep re-runs it idempotently).
    if let Err(e) = state.store.promote_cluster(&tenant_id, &artifact.signature).await {
        tracing::error!(
            tenant = %tenant_id,
            signature = %artifact.signature,
            error = %e,
            "collective promotion failed after publish"
        );
    }
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

/// `GET /v1/t/{tenant_key}/artifacts?since={seq}` — pull the tenant's artifacts
/// with `seq > since`, ordered by `seq`. Dual-auth, member+. Strictly scoped to
/// the path tenant — a tenant never sees another's artifacts.
async fn pull_artifacts(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    Query(q): Query<PullQuery>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let tenant_id = state
        .store
        .resolve_tenant(&tenant_key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such tenant"))?;
    // Any authenticated caller (member+) may pull — success here is sufficient.
    let _caller = authenticate_dojo(&state.store, &jwt, &headers, tenant_id)
        .await
        .map_err(dojo_auth_status)?;
    let page = state
        .store
        .pull_artifacts_since(&tenant_id, q.since.unwrap_or(0))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::to_value(page).unwrap()))
}

// ── Maintainer triage routes (service-side; serve the C12 console) ───────────

/// Resolve the path tenant then dual-authenticate a maintainer (`maintainer+`).
/// Reuses C3's tenant resolution + dual auth; a non-maintainer → 403.
async fn resolve_maintainer(
    state: &AppState,
    jwt: &JwtConfig,
    headers: &HeaderMap,
    tenant_key: &str,
) -> Result<(Uuid, crate::auth::DojoCaller), (StatusCode, Json<serde_json::Value>)> {
    let tenant_id = state
        .store
        .resolve_tenant(tenant_key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such tenant"))?;
    let caller = authenticate_dojo(&state.store, jwt, headers, tenant_id)
        .await
        .map_err(dojo_auth_status)?;
    if caller.access < DojoAccess::Maintainer {
        return Err(err(StatusCode::FORBIDDEN, "maintainer role required"));
    }
    Ok((tenant_id, caller))
}

/// `GET /v1/t/{tenant_key}/triage` — list the tenant's open triage rows
/// (queued / in_review) with cluster info. Maintainer+.
async fn list_triage(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tenant_id, _caller) = resolve_maintainer(&state, &jwt, &headers, &tenant_key).await?;
    let rows = state
        .store
        .list_triage(&tenant_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "queue": rows })))
}

/// `POST /v1/t/{tenant_key}/triage/promote` — run the tenant promotion sweep
/// (idempotent). Maintainer+. Lets a maintainer flush the queue on demand.
async fn promote_sweep(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tenant_id, _caller) = resolve_maintainer(&state, &jwt, &headers, &tenant_key).await?;
    let outcomes = state
        .store
        .promote_tenant(&tenant_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let promoted: Vec<serde_json::Value> = outcomes
        .into_iter()
        .map(|(signature, outcome)| serde_json::json!({ "signature": signature, "result": outcome }))
        .collect();
    Ok(Json(serde_json::json!({ "promoted": promoted })))
}

#[derive(Deserialize)]
struct DecideBody {
    status: String,
    #[serde(default)]
    distribution_scope: Option<serde_json::Value>,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /v1/t/{tenant_key}/triage/{signature}/decide` — record a maintainer
/// decision. Maintainer+. `approve` requires `distribution_scope`; `decline`
/// requires a non-empty `reason` (both rejected with 400 otherwise).
async fn decide_triage(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path((tenant_key, signature)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<DecideBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tenant_id, caller) = resolve_maintainer(&state, &jwt, &headers, &tenant_key).await?;
    let status = DecideStatus::parse(&body.status)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "status must be approve|revise|decline"))?;
    // Safe-default gates (maintainer-console done-gate): approve must name a
    // distribution scope; decline must give a reason.
    match status {
        DecideStatus::Approve if body.distribution_scope.is_none() => {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "approve requires distribution_scope",
            ));
        }
        DecideStatus::Decline
            if body
                .reason
                .as_deref()
                .map(|r| r.trim().is_empty())
                .unwrap_or(true) =>
        {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "decline requires a non-empty reason",
            ));
        }
        _ => {}
    }
    let maintainer_id = Uuid::parse_str(&caller.subject).ok();
    let outcome = state
        .store
        .decide_triage(
            &tenant_id,
            &signature,
            status,
            body.distribution_scope,
            body.reason,
            maintainer_id,
        )
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    match outcome {
        DecideOutcome::NotFound => Err(err(StatusCode::NOT_FOUND, "no such triage candidate")),
        DecideOutcome::Published { artifact_id, seq } => Ok(Json(serde_json::json!({
            "status": "approved", "artifact_id": artifact_id, "seq": seq,
        }))),
        DecideOutcome::Declined => Ok(Json(serde_json::json!({ "status": "declined" }))),
        DecideOutcome::Revised => Ok(Json(serde_json::json!({ "status": "revised" }))),
    }
}

/// Build the router with the default (Supabase local-dev) JWT config. Existing
/// callers keep this exact signature; the dojo routes get a default verifier.
pub fn build_router(state: AppState) -> Router {
    build_router_with_jwt(state, JwtConfig::default())
}

/// Build the router, injecting the Supabase-JWT verification config used by the
/// dual-auth dojo routes. The existing `/v1/rules` / `/v1/members` routes and
/// their API-key middleware are unchanged; the new `/v1/t/{tenant_key}/…` routes
/// carry `jwt` as a request extension.
pub fn build_router_with_jwt(state: AppState, jwt: JwtConfig) -> Router {
    let protected = Router::new()
        .route("/v1/rules", post(publish_rule).get(pull_rules))
        .route("/v1/rules/{id}", delete(retract_rule))
        .route("/v1/members", post(add_member))
        .route("/v1/members/{id}/keys", post(issue_key))
        .route("/v1/subscriptions", post(subscriptions_stub))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), require));
    let dojo = Router::new()
        .route(
            "/v1/t/{tenant_key}/artifacts",
            post(publish_artifact).get(pull_artifacts),
        )
        .route("/v1/t/{tenant_key}/triage", get(list_triage))
        .route("/v1/t/{tenant_key}/triage/promote", post(promote_sweep))
        .route(
            "/v1/t/{tenant_key}/triage/{signature}/decide",
            post(decide_triage),
        )
        .layer(Extension(Arc::new(jwt)));
    Router::new()
        .route("/v1/health", get(health))
        .merge(protected)
        .merge(dojo)
        .with_state(state)
}
