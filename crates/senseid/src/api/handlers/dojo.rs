//! `/api/dojo/*` — the daemon's Dōjō connections (memberships) surface.
//!
//! Mirrors the federation `/api/knowledge/sources` handlers: register a
//! connection (URL + tenant_key + service membership id + device token →
//! Keychain, optional project binding) and list connections + bindings +
//! sync-status for the observatory Dōjō-connections pane.
//!
//! Live artifact publish/pull is out of scope here (C6/C7). The daemon
//! authenticates to the Dōjō service with a Keychain-backed Bearer token — never
//! Supabase (dual-plane auth: humans use Supabase in the web console only).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::api::state::AppState;
use crate::dojo::memberships::{self, NewConnection};

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

/// GET /api/dojo/memberships — list the daemon's Dōjō connections with each
/// one's bound projects and sync-status. Returns a top-level array (the shape
/// `docs/spec/pipeline/dojo-lifecycle.md` documents: `jq '.[] | ...'`).
pub(crate) async fn list_memberships(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let views = memberships::list(&state.pg)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::to_value(views).unwrap_or(serde_json::Value::Array(vec![]))))
}

/// Body for POST /api/dojo/memberships. `membership_id` is the service-assigned
/// `dojo.memberships.id` (from the console); `credential` is the per-membership
/// device token pasted into the app (stored in the Keychain, never in PG).
#[derive(Deserialize)]
pub(crate) struct NewMembershipBody {
    /// Service membership id (`dojo.memberships.id`) — becomes the local PK.
    pub membership_id: String,
    /// Registry base URL. Defaults to `sensei_config::dojo_registry_url()`
    /// (`SENSEI_DOJO_URL`, default `http://localhost:7755`) when omitted.
    pub registry_url: Option<String>,
    /// `<origin>/<org>[/<dojo>]` discovery path of the tenant Dōjō.
    pub tenant_key: String,
    /// employer | client | community | personal.
    pub kind: String,
    /// Git-remote owner slugs this membership covers (e.g. `["sensei-hq"]`) —
    /// the org-tagging that drives infer-at-detect auto-bind. Normalised
    /// (lowercased/deduped) server-side. Optional; defaults to none.
    pub org_slugs: Option<Vec<String>>,
    /// contributor | maintainer | lead | admin (default contributor).
    pub role: Option<String>,
    /// sso | github_oauth | device_code (default device_code).
    pub authenticated_via: Option<String>,
    /// named | anonymous | dereferenced (default named).
    pub attribution_default: Option<String>,
    /// The device token (Bearer). Stored in the OS Keychain.
    pub credential: String,
    /// Optional project to bind to this membership (`projects.dojo_id`).
    pub project_id: Option<String>,
}

/// POST /api/dojo/memberships — register a Dōjō connection.
pub(crate) async fn create_membership(
    State(state): State<AppState>,
    Json(b): Json<NewMembershipBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let membership_id = uuid::Uuid::parse_str(b.membership_id.trim()).map_err(|_| {
        err(StatusCode::BAD_REQUEST, "bad membership_id (expected the service membership uuid)")
    })?;

    let registry_url = b
        .registry_url
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(sensei_config::dojo_registry_url);
    // Never send the device token to a non-loopback cleartext endpoint.
    crate::api::util::require_secure_url(&registry_url, "registry url")
        .map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;

    let tenant_key = b.tenant_key.trim().to_string();
    let dojo_url = memberships::derive_dojo_url(&registry_url, &tenant_key);

    let project_id = match b.project_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(
            uuid::Uuid::parse_str(s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?,
        ),
        None => None,
    };

    let conn = NewConnection::validated(
        membership_id,
        registry_url,
        tenant_key,
        dojo_url,
        &b.kind,
        b.org_slugs.as_deref().unwrap_or(&[]),
        b.role.as_deref().unwrap_or("contributor"),
        b.authenticated_via.as_deref().unwrap_or("device_code"),
        // Fail closed on privacy: an omitted attribution defaults to the safest
        // conservative mode (source-dereferenced), NOT the least-private `named`.
        // Shared source of truth with collective preferences.
        b.attribution_default
            .as_deref()
            .unwrap_or(crate::collective::preferences::DEFAULT_ATTRIBUTION),
        // A fresh pairing is mid-authentication until the first heartbeat.
        "authenticating",
    )
    .map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;

    let id = memberships::register(&state.pg, conn, &b.credential)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    if let Some(pid) = project_id
        && !memberships::bind_project(&state.pg, &pid, &id)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        return Err(err(
            StatusCode::NOT_FOUND,
            "membership registered but project_id not found to bind",
        ));
    }

    Ok(Json(serde_json::json!({ "id": id.to_string() })))
}

/// Body for PUT /api/dojo/memberships/{id}/orgs — the org-tagging edit.
#[derive(Deserialize)]
pub(crate) struct SetOrgsBody {
    /// The git-remote owner slugs this membership covers. Replaces the existing
    /// set; normalised (lowercased/deduped) server-side.
    pub org_slugs: Vec<String>,
}

/// PUT /api/dojo/memberships/{id}/orgs — replace the org slugs a membership
/// covers (drives infer-at-detect auto-bind). Idempotent; 404 if unknown.
pub(crate) async fn set_membership_orgs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<SetOrgsBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let membership_id = uuid::Uuid::parse_str(id.trim())
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad membership id"))?;
    let updated = memberships::set_orgs(&state.pg, &membership_id, &b.org_slugs)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    if !updated {
        return Err(err(StatusCode::NOT_FOUND, "membership not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/projects/{id}/dojo-suggestion — the inferred (confirm-inferred)
/// project→Dōjō binding for the About-panel chip, or `{ "suggestion": null }`
/// when the project is already bound / has no matching membership. Read-only:
/// suggests, never binds.
pub(crate) async fn project_binding_suggestion(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Name-or-uuid (#100): resolve so a project name works, not only a uuid.
    let project_id = crate::api::util::resolve_project_uuid(&state, id.trim())
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "project lookup failed"))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "project not found"))?;
    let suggestion = memberships::suggest_binding(&state.pg, &project_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "suggestion": suggestion })))
}

/// Body for POST /api/projects/{id}/dojo-binding — confirm a binding.
#[derive(Deserialize)]
pub(crate) struct BindProjectBody {
    /// The membership to bind this project to (`projects.dojo_id`).
    pub membership_id: String,
}

/// POST /api/projects/{id}/dojo-binding — bind a project to a Dōjō membership
/// (the user confirming the inferred suggestion, or an explicit bind). Fails
/// closed if the membership is unknown — a project must never point at a
/// membership the daemon does not hold. 404 if the project is unknown.
pub(crate) async fn bind_project_to_membership(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<BindProjectBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Name-or-uuid (#100): resolve so a project name works, not only a uuid.
    let project_id = crate::api::util::resolve_project_uuid(&state, id.trim())
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "project lookup failed"))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "project not found"))?;
    let membership_id = uuid::Uuid::parse_str(b.membership_id.trim())
        .map_err(|_| err(StatusCode::BAD_REQUEST, "bad membership_id"))?;
    if state
        .pg
        .get_dojo_membership(&membership_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "unknown membership"));
    }
    if !memberships::bind_project(&state.pg, &project_id, &membership_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        return Err(err(StatusCode::NOT_FOUND, "project not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true, "dojo_id": membership_id.to_string() })))
}
