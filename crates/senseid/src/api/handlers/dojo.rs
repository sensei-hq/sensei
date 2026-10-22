//! `/api/dojo/*` — the daemon's Dōjō connections (memberships) surface.
//!
//! Lists the daemon's memberships with their bindings and sync status for the
//! observatory Dōjō-connections pane, and edits a membership's org tagging.
//! There is no POST: signing in is joining (#236 step 4e) — the sync creates
//! each membership from the signed-in persona's list, and its tenant calls
//! present that persona's session.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::api::handlers::err;
use crate::api::state::AppState;
use crate::dojo::memberships;

/// `GET /api/dojo/sync-state` — when each entity last agreed with the dōjō, and
/// what went wrong where it has not.
///
/// The first read of `sensei.sync_state`. Three writers existed and nothing
/// surfaced them, so "when did this last sync?" was answerable only by opening
/// the database — which is the gap #127 records for the credential too.
///
/// Rows arrive worst-first from the store (`error`, `pending`, `skipped`,
/// `synced`), so a caller rendering the head of the list sees what needs
/// attention without re-ranking. `counts` is a by-state tally for a summary line,
/// derived here rather than in a second query.
///
/// A read failure is a 500 — an empty list would read as "everything is fine",
/// which for a sync surface is the most expensive possible lie.
pub(crate) async fn sync_state(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rows = state.pg.sync_state_rows().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for r in &rows {
        *counts.entry(r.state.as_str()).or_default() += 1;
    }

    Ok(Json(serde_json::json!({
        "count": rows.len(),
        "counts": counts,
        "entities": rows,
    })))
}

/// GET /api/dojo/memberships — list the daemon's Dōjō connections with each
/// one's bound workspaces and sync-status. Returns a top-level array (the shape
/// `docs/spec/pipeline/dojo-lifecycle.md` documents: `jq '.[] | ...'`).
pub(crate) async fn list_memberships(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let views = memberships::list(&state.pg)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::to_value(views).unwrap_or(serde_json::Value::Array(vec![]))))
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

/// GET /api/workspaces/{id}/dojo-suggestion — the inferred (confirm-inferred)
/// workspace→Dōjō binding for the About-panel chip, or `{ "suggestion": null }`
/// when the workspace is already bound / has no matching membership. Read-only:
/// suggests, never binds.
pub(crate) async fn workspace_binding_suggestion(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Name-or-uuid (#100): resolve so a workspace name works, not only a uuid.
    let workspace_id = crate::api::util::resolve_workspace_uuid(&state, id.trim())
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "workspace lookup failed"))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "workspace not found"))?;
    let suggestion = memberships::suggest_binding(&state.pg, &workspace_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({ "suggestion": suggestion })))
}

/// Body for POST /api/workspaces/{id}/dojo-binding — confirm a binding.
#[derive(Deserialize)]
pub(crate) struct BindWorkspaceBody {
    /// The membership to bind this workspace to (`workspaces.dojo_id`).
    pub membership_id: String,
}

/// POST /api/workspaces/{id}/dojo-binding — bind a workspace to a Dōjō membership
/// (the user confirming the inferred suggestion, or an explicit bind). Fails
/// closed if the membership is unknown — a workspace must never point at a
/// membership the daemon does not hold. 404 if the workspace is unknown.
pub(crate) async fn bind_workspace_to_membership(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<BindWorkspaceBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Name-or-uuid (#100): resolve so a workspace name works, not only a uuid.
    let workspace_id = crate::api::util::resolve_workspace_uuid(&state, id.trim())
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "workspace lookup failed"))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "workspace not found"))?;
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
    if !memberships::bind_workspace(&state.pg, &workspace_id, &membership_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        return Err(err(StatusCode::NOT_FOUND, "workspace not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true, "dojo_id": membership_id.to_string() })))
}

/// Body for PATCH /api/dojo/metric-activation.
///
/// `persona` is REQUIRED and is the Keychain session slot, not the display
/// label: signing a call against the label silently addresses a different
/// credential (or none). Which tenant owns the repository is the dōjō's to
/// derive from `all_my_repositories`, so it is deliberately absent here — a body
/// that could name the tenant would let one dōjō's member write another's cost
/// decision.
#[derive(Deserialize)]
pub(crate) struct ActivationBody {
    pub persona: String,
    pub repo_key: String,
    pub metric: String,
    pub enabled: bool,
}

/// The three strings, trimmed, or the field that was blank.
///
/// Separated from the handler because this is the whole of the request contract
/// and the handler's remaining work needs a Keychain and a network.
fn validated(b: &ActivationBody) -> Result<(&str, &str, &str), &'static str> {
    let persona = b.persona.trim();
    if persona.is_empty() {
        return Err("persona is required (the Keychain session slot)");
    }
    let repo_key = b.repo_key.trim();
    if repo_key.is_empty() {
        return Err("repo_key is required");
    }
    let metric = b.metric.trim();
    if metric.is_empty() {
        return Err("metric is required");
    }
    Ok((persona, repo_key, metric))
}

/// Which status a credential failure earns.
///
/// `SignedOut`/`Rejected` are 401 — the user can fix them by signing in.
/// `Unreachable` is 503: the session is still good and a retry may work, so
/// answering 401 would send the app to a sign-in screen it does not need and
/// would discard a live credential over a network blip.
fn credential_status(e: &crate::api::handlers::auth::AuthError) -> StatusCode {
    // `needs_sign_in()` already owns this distinction — re-matching the variants
    // here would be a second copy to keep in step with the first.
    match e.needs_sign_in() {
        true => StatusCode::UNAUTHORIZED,
        false => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// PATCH /api/dojo/metric-activation — switch one metric off (or back on) for
/// one repository, via the dōjō that owns the ruling.
///
/// A proxy, not a local write: `dojo.metric_activations` is the tenant's record
/// and the daemon only ever READS the consequence back through the sync plan's
/// `disabled_metrics`. Writing a local copy would make the daemon a second
/// authority for a decision it does not own, and the two would drift.
pub(crate) async fn set_metric_activation(
    State(_state): State<AppState>,
    Json(b): Json<ActivationBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use crate::dojo_client::user_plane::UserPlane;

    let (persona, repo_key, metric) = validated(&b).map_err(|m| err(StatusCode::BAD_REQUEST, m))?;

    let token = crate::api::handlers::auth::live_access_token(persona).await.map_err(|e| {
        (credential_status(&e), Json(serde_json::json!({ "error": e.to_string() })))
    })?;

    let plane = crate::dojo_client::user_plane::HttpUserPlane {
        dojo_url: crate::dojo_client::settings::dojo_url(),
    };
    let outcome = plane
        .set_metric_activation(&token, repo_key, metric, b.enabled)
        .await
        // The dōjō's own refusal (403 not-configurable, 404 unknown metric) is
        // surfaced as a 502 with its text rather than swallowed into a generic
        // failure: the reason is the only thing that tells the user whether to
        // ask an admin or fix the metric key.
        .map_err(|e| err(StatusCode::BAD_GATEWAY, e))?;

    Ok(Json(serde_json::json!({
        "repoKey": outcome.repo_key,
        "metric": outcome.metric,
        "enabled": outcome.enabled,
        "tenant": outcome.tenant,
    })))
}

#[cfg(test)]
mod activation_contract {
    use super::*;
    use crate::api::handlers::auth::AuthError;

    fn body(persona: &str, repo: &str, metric: &str) -> ActivationBody {
        ActivationBody {
            persona: persona.into(),
            repo_key: repo.into(),
            metric: metric.into(),
            enabled: false,
        }
    }

    #[test]
    fn a_complete_body_is_trimmed_not_merely_accepted() {
        let ok = body("  work  ", " github.com/acme/api ", " ftr ");
        assert_eq!(validated(&ok), Ok(("work", "github.com/acme/api", "ftr")));
    }

    #[test]
    fn each_blank_field_is_named_in_its_own_refusal() {
        // A whitespace-only persona is the shape a form field produces, and it
        // would otherwise reach the Keychain as a slot that cannot exist —
        // answering 401 "signed out" for what is really a malformed request.
        for (b, want) in [
            (body("   ", "r", "m"), "persona"),
            (body("p", "  ", "m"), "repo_key"),
            (body("p", "r", "\t"), "metric"),
        ] {
            let e = validated(&b).expect_err("blank field must be refused");
            assert!(e.contains(want), "{e:?} should name {want}");
        }
    }

    #[test]
    fn enabled_false_survives_validation() {
        // `enabled` is not defaulted and not coerced: serde refuses a missing or
        // non-boolean value outright, so the "false is truthy" trap the dōjō
        // route guards against cannot arise here. Pinned so a later
        // `#[serde(default)]` — which would silently turn a metric ON — fails.
        let b = body("p", "r", "m");
        assert!(!b.enabled);
        assert!(validated(&b).is_ok());
    }

    #[test]
    fn a_dead_or_rejected_session_is_401_but_a_blip_is_503() {
        // The distinction the CLI got wrong once already: collapsing Unreachable
        // into 401 sends the app to a sign-in screen and discards a credential
        // that is still good.
        assert_eq!(credential_status(&AuthError::SignedOut), StatusCode::UNAUTHORIZED);
        assert_eq!(
            credential_status(&AuthError::Rejected("bad grant".into())),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            credential_status(&AuthError::Unreachable("connection reset".into())),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
