//! Axum router + handlers for the `/v1` federation API.

use crate::auth::{
    authenticate_dojo, require, role_satisfies, AuthCaller, DojoAccess, DojoAuthError, DojoCaller,
    JwtConfig, Role,
};
use crate::collective::promote::{DecideOutcome, DecideStatus};
use crate::store::DojoStore;
use axum::{
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use dojo_protocol::PublishedArtifact;
use dojo_protocol::PublishedRule;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// A member/dojo role literal accepted for a membership (`dojo.member_role`).
fn is_member_role(role: &str) -> bool {
    matches!(role, "contributor" | "maintainer" | "client_lead" | "admin")
}

pub struct SharedState {
    pub store: DojoStore,
}
pub type AppState = Arc<SharedState>;

fn err(code: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (code, Json(serde_json::json!({ "error": msg })))
}
/// Map a store error string onto a 500 (never silent — the message is surfaced).
fn ise(e: String) -> (StatusCode, Json<serde_json::Value>) {
    err(StatusCode::INTERNAL_SERVER_ERROR, &e)
}
/// The audit actor id for a dojo caller — the uuid `subject` (member id on the
/// API-key plane, Supabase `sub` on the JWT plane). `None` if it isn't a uuid.
fn actor_of(caller: &DojoCaller) -> Option<Uuid> {
    Uuid::parse_str(&caller.subject).ok()
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
    Json(serde_json::json!({ "status": "ok", "service": "sensei-dojo", "scope": "dojo" }))
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

/// The lowercase role name a `DojoAccess` floor corresponds to (for a 403 body).
fn access_label(a: DojoAccess) -> &'static str {
    match a {
        DojoAccess::Member => "member",
        DojoAccess::Contributor => "contributor",
        DojoAccess::Maintainer => "maintainer",
        DojoAccess::Admin => "admin",
    }
}

/// Resolve the path tenant then dual-authenticate a caller, enforcing a role
/// `floor`. Reuses tenant resolution + dual auth (`authenticate_dojo`); a caller
/// below the floor → 403. The shared primitive behind both the maintainer triage
/// routes and the admin console.
async fn resolve_tenant_access(
    state: &AppState,
    jwt: &JwtConfig,
    headers: &HeaderMap,
    tenant_key: &str,
    floor: DojoAccess,
) -> Result<(Uuid, DojoCaller), (StatusCode, Json<serde_json::Value>)> {
    let tenant_id = state
        .store
        .resolve_tenant(tenant_key)
        .await
        .map_err(ise)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such tenant"))?;
    let caller = authenticate_dojo(&state.store, jwt, headers, tenant_id)
        .await
        .map_err(dojo_auth_status)?;
    if caller.access < floor {
        return Err(err(
            StatusCode::FORBIDDEN,
            &format!("{} role required", access_label(floor)),
        ));
    }
    Ok((tenant_id, caller))
}

/// Resolve the path tenant then dual-authenticate a maintainer (`maintainer+`).
/// Reuses C3's tenant resolution + dual auth; a non-maintainer → 403.
async fn resolve_maintainer(
    state: &AppState,
    jwt: &JwtConfig,
    headers: &HeaderMap,
    tenant_key: &str,
) -> Result<(Uuid, DojoCaller), (StatusCode, Json<serde_json::Value>)> {
    resolve_tenant_access(state, jwt, headers, tenant_key, DojoAccess::Maintainer).await
}

/// Resolve the path tenant then dual-authenticate an admin (`admin`). The floor
/// for every admin-console route; a non-admin → 403.
async fn resolve_admin(
    state: &AppState,
    jwt: &JwtConfig,
    headers: &HeaderMap,
    tenant_key: &str,
) -> Result<(Uuid, DojoCaller), (StatusCode, Json<serde_json::Value>)> {
    resolve_tenant_access(state, jwt, headers, tenant_key, DojoAccess::Admin).await
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

// ── Admin console routes (dual-auth; ADMIN floor) ────────────────────────────
//
// Every handler resolves the path tenant + dual-authenticates behind
// `resolve_admin` (floor = `DojoAccess::Admin`), so a non-admin is 403 and the
// work is strictly tenant-scoped. Mutations stamp a `dojo.audit_events` row
// (non-repudiation) via `record_audit_event`; that write is surfaced (not
// swallowed) so a missing audit entry can never hide an admin action.

// ── members (provision + re-role) ────────────────────────────────────────────

/// `GET /v1/t/{tenant_key}/members` — the tenant's memberships (admin).
async fn list_members(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, _c) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let members = state.store.list_memberships(&tenant_id).await.map_err(ise)?;
    Ok(Json(json!({ "members": members })))
}

#[derive(Deserialize)]
struct NewMembership {
    user_id: String,
    kind: String,
    authenticated_via: String,
    #[serde(default)]
    git_provider_role: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

/// `POST /v1/t/{tenant_key}/members` — provision a membership (admin). The role
/// derives from the git-provider role by default (`dojo.roles` map), unless an
/// explicit `role` override is given; with neither, it falls back to the
/// least-privilege `contributor` (done-gate: a `write` git role lands as
/// `contributor`). Stamps a `member_added` audit event.
async fn add_membership(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<NewMembership>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let user_id =
        Uuid::parse_str(&body.user_id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad user_id"))?;
    // Explicit admin override wins; else derive from the git role; else default.
    let role = match &body.role {
        Some(r) => r.clone(),
        None => match &body.git_provider_role {
            Some(g) => state
                .store
                .derive_role_for_git_role(&tenant_id, g)
                .await
                .map_err(ise)?
                .unwrap_or_else(|| "contributor".to_string()),
            None => "contributor".to_string(),
        },
    };
    if !is_member_role(&role) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "role must be contributor|maintainer|client_lead|admin",
        ));
    }
    let id = state
        .store
        .create_membership(&tenant_id, &user_id, &body.kind, &body.authenticated_via, &role)
        .await
        .map_err(ise)?;
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "member_added",
            Some(&user_id.to_string()),
            json!({ "role": role, "git_provider_role": body.git_provider_role }),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "id": id.to_string(), "role": role })))
}

#[derive(Deserialize)]
struct SetRole {
    role: String,
}

/// `PATCH /v1/t/{tenant_key}/members/{user_id}/role` — admin role override.
/// Stamps a `role_changed` audit event.
async fn set_member_role(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path((tenant_key, user_id)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<SetRole>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let uid =
        Uuid::parse_str(&user_id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad user_id"))?;
    if !is_member_role(&body.role) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "role must be contributor|maintainer|client_lead|admin",
        ));
    }
    let changed = state
        .store
        .set_membership_role(&tenant_id, &uid, &body.role)
        .await
        .map_err(ise)?;
    if !changed {
        return Err(err(StatusCode::NOT_FOUND, "no such member"));
    }
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "role_changed",
            Some(&user_id),
            json!({ "role": body.role }),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "user_id": user_id, "role": body.role })))
}

// ── identities (SSO / GitHub / device-code config) ───────────────────────────

/// `GET /v1/t/{tenant_key}/identities` — the tenant's identity mappings (admin).
async fn list_identities(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, _c) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let identities = state.store.list_identities(&tenant_id).await.map_err(ise)?;
    Ok(Json(json!({ "identities": identities })))
}

#[derive(Deserialize)]
struct NewIdentity {
    user_id: String,
    provider: String,
    subject: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

fn is_auth_method(p: &str) -> bool {
    matches!(p, "sso" | "github_oauth" | "device_code")
}

/// `POST /v1/t/{tenant_key}/identities` — wire an identity provider (admin).
async fn create_identity(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<NewIdentity>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let uid =
        Uuid::parse_str(&body.user_id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad user_id"))?;
    if !is_auth_method(&body.provider) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "provider must be sso|github_oauth|device_code",
        ));
    }
    let id = state
        .store
        .create_identity(
            &tenant_id,
            &uid,
            &body.provider,
            &body.subject,
            body.email.as_deref(),
            body.display_name.as_deref(),
        )
        .await
        .map_err(ise)?;
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "identity_added",
            Some(&body.subject),
            json!({ "provider": body.provider }),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "id": id.to_string() })))
}

#[derive(Deserialize)]
struct UpdateIdentity {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

/// `PATCH /v1/t/{tenant_key}/identities/{id}` — update an identity's email /
/// display name (admin). A `null`/absent field is left unchanged.
async fn update_identity(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path((tenant_key, id)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<UpdateIdentity>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let iid = Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad identity id"))?;
    let changed = state
        .store
        .update_identity(&tenant_id, &iid, body.email.as_deref(), body.display_name.as_deref())
        .await
        .map_err(ise)?;
    if !changed {
        return Err(err(StatusCode::NOT_FOUND, "no such identity"));
    }
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "identity_updated",
            Some(&id),
            json!({}),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "id": id })))
}

/// `DELETE /v1/t/{tenant_key}/identities/{id}` — remove an identity (admin).
async fn delete_identity(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path((tenant_key, id)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let iid = Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad identity id"))?;
    let removed = state.store.delete_identity(&tenant_id, &iid).await.map_err(ise)?;
    if !removed {
        return Err(err(StatusCode::NOT_FOUND, "no such identity"));
    }
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "identity_deleted",
            Some(&id),
            json!({}),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "deleted": true })))
}

// ── policies (attribution / confidentiality / retention grid) ────────────────

/// `GET /v1/t/{tenant_key}/policies` — the tenant's policy grid (admin).
async fn list_policies(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, _c) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let policies = state.store.list_policies(&tenant_id).await.map_err(ise)?;
    Ok(Json(json!({ "policies": policies })))
}

fn is_attribution_mode(m: &str) -> bool {
    matches!(m, "named" | "anonymous" | "dereferenced")
}

#[derive(Deserialize)]
struct UpsertPolicy {
    scope_key: String,
    #[serde(default)]
    attribution_default: Option<String>,
    #[serde(default)]
    confidentiality: Option<Value>,
    #[serde(default)]
    retention_days: Option<i32>,
}

/// `POST /v1/t/{tenant_key}/policies` — create or edit-by-scope a policy
/// (admin). Upserts on `(tenant_id, scope_key)` so an edit takes effect on the
/// next batch. Stamps a `policy_edited` audit event.
async fn create_policy(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<UpsertPolicy>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let attribution = body.attribution_default.as_deref().unwrap_or("named");
    if !is_attribution_mode(attribution) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "attribution_default must be named|anonymous|dereferenced",
        ));
    }
    let confidentiality = body.confidentiality.unwrap_or_else(|| json!({}));
    let id = state
        .store
        .upsert_policy(
            &tenant_id,
            &body.scope_key,
            attribution,
            confidentiality,
            body.retention_days,
        )
        .await
        .map_err(ise)?;
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "policy_edited",
            Some(&body.scope_key),
            json!({ "attribution_default": attribution, "retention_days": body.retention_days }),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "id": id.to_string(), "scope_key": body.scope_key })))
}

#[derive(Deserialize)]
struct PatchPolicy {
    #[serde(default)]
    attribution_default: Option<String>,
    #[serde(default)]
    confidentiality: Option<Value>,
    #[serde(default)]
    retention_days: Option<i32>,
}

/// `PATCH /v1/t/{tenant_key}/policies/{id}` — update a policy by id (admin). An
/// absent field is left unchanged. Stamps a `policy_edited` audit event.
async fn update_policy(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path((tenant_key, id)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<PatchPolicy>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let pid = Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad policy id"))?;
    if let Some(a) = &body.attribution_default
        && !is_attribution_mode(a)
    {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "attribution_default must be named|anonymous|dereferenced",
        ));
    }
    let changed = state
        .store
        .update_policy(
            &tenant_id,
            &pid,
            body.attribution_default.as_deref(),
            body.confidentiality,
            body.retention_days,
        )
        .await
        .map_err(ise)?;
    if !changed {
        return Err(err(StatusCode::NOT_FOUND, "no such policy"));
    }
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "policy_edited",
            Some(&id),
            json!({}),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "id": id })))
}

/// `DELETE /v1/t/{tenant_key}/policies/{id}` — remove a policy (admin).
async fn delete_policy(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path((tenant_key, id)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, caller) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let pid = Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad policy id"))?;
    let removed = state.store.delete_policy(&tenant_id, &pid).await.map_err(ise)?;
    if !removed {
        return Err(err(StatusCode::NOT_FOUND, "no such policy"));
    }
    state
        .store
        .record_audit_event(
            &tenant_id,
            actor_of(&caller).as_ref(),
            "policy_deleted",
            Some(&id),
            json!({}),
        )
        .await
        .map_err(ise)?;
    Ok(Json(json!({ "deleted": true })))
}

// ── health (rollups from dojo.events) ────────────────────────────────────────

/// `GET /v1/t/{tenant_key}/health` — the admin health strip: connection count,
/// queue depth, publish rate (1h), error rate (1h). Read-only (no audit).
async fn admin_health(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, _c) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let (connections, queue_depth, publish_rate_1h, error_rate_1h) =
        state.store.health_rollup(&tenant_id).await.map_err(ise)?;
    Ok(Json(json!({
        "connections": connections,
        "queue_depth": queue_depth,
        "publish_rate_1h": publish_rate_1h,
        "error_rate_1h": error_rate_1h,
    })))
}

// ── audit (list dojo.audit_events) ───────────────────────────────────────────

#[derive(Deserialize)]
struct AuditQuery {
    #[serde(default)]
    limit: Option<i64>,
}

/// `GET /v1/t/{tenant_key}/audit` — the admin audit log (`dojo.audit_events`),
/// most recent first. Read-only (no audit).
async fn list_audit(
    State(state): State<AppState>,
    Extension(jwt): Extension<Arc<JwtConfig>>,
    axum::extract::Path(tenant_key): axum::extract::Path<String>,
    Query(q): Query<AuditQuery>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (tenant_id, _c) = resolve_admin(&state, &jwt, &headers, &tenant_key).await?;
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    let events = state.store.list_audit_events(&tenant_id, limit).await.map_err(ise)?;
    Ok(Json(json!({ "events": events })))
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
        // Admin console (ADMIN floor; every handler behind `resolve_admin`).
        .route(
            "/v1/t/{tenant_key}/members",
            get(list_members).post(add_membership),
        )
        .route(
            "/v1/t/{tenant_key}/members/{user_id}/role",
            patch(set_member_role),
        )
        .route(
            "/v1/t/{tenant_key}/identities",
            get(list_identities).post(create_identity),
        )
        .route(
            "/v1/t/{tenant_key}/identities/{id}",
            patch(update_identity).delete(delete_identity),
        )
        .route(
            "/v1/t/{tenant_key}/policies",
            get(list_policies).post(create_policy),
        )
        .route(
            "/v1/t/{tenant_key}/policies/{id}",
            patch(update_policy).delete(delete_policy),
        )
        .route("/v1/t/{tenant_key}/health", get(admin_health))
        .route("/v1/t/{tenant_key}/audit", get(list_audit))
        .layer(Extension(Arc::new(jwt)));
    Router::new()
        .route("/v1/health", get(health))
        .merge(protected)
        .merge(dojo)
        .with_state(state)
}
