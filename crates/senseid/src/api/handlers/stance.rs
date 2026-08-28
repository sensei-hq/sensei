//! `GET /api/stance` — the effective behavioural stance a user runs under for a
//! folder: the three dials (autonomy · sharing · review) resolved on the
//! `sensei.scopes` ladder. Complements `/api/knowledge/rules` (WHAT a run may do)
//! with HOW a run behaves.
//!
//! Who = the git identity at the folder (same resolution as `/api/user`), or an
//! explicit `user` override. Where = the indexed repo at `under` (soft-resolved;
//! an un-indexed path just yields the user's default stance). User-scoped +
//! daemon-local (D-STANCE-SCOPE).
//!
//! Never 500s on a missing identity or un-indexed folder — it degrades to the
//! user's default / the enum fallback stance.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use crate::api::handlers::err;
use crate::api::state::AppState;
use crate::git_identity::read_git_user;
use crate::stance::StanceInput;

/// Resolve `dir` (explicit `under`, else the daemon cwd) and `user_key`
/// (explicit `user`, else the git author at `dir`) from a request field-getter —
/// the shared who/where resolution behind both GET and POST `/api/stance`.
fn resolve_who_where(under: Option<&str>, user: Option<&str>) -> (String, String) {
    let dir = under
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let user_key = user
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| read_git_user(std::path::Path::new(&dir)).email)
        .unwrap_or_default();
    (dir, user_key)
}

#[derive(Debug, Deserialize)]
pub(crate) struct StanceQuery {
    /// Absolute folder to resolve the stance for. Defaults to the daemon's cwd
    /// when omitted (the MCP proxy sends the caller's cwd, matching `/api/user`).
    under: Option<String>,
    /// Explicit user key (git email) to resolve the stance for. Omitted → derived
    /// from the git identity at `under` (local→global, like `/api/user`).
    user: Option<String>,
}

/// Resolve `{ dir, user_key, stance }` for a folder.
pub(crate) async fn get_stance(
    State(state): State<AppState>,
    Query(q): Query<StanceQuery>,
) -> Json<serde_json::Value> {
    let (dir, user_key) = resolve_who_where(q.under.as_deref(), q.user.as_deref());

    // Where: soft-resolve the indexed repo at `dir`. An un-indexed path (or a
    // subdir) yields None → only the user's default stance is a candidate.
    let folder_id = state
        .pg
        .get_repo_by_path(&dir)
        .await
        .ok()
        .flatten()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));

    let stance = state
        .pg
        .resolve_stance(&user_key, folder_id.as_ref())
        .await
        .unwrap_or_else(|_| crate::stance::ResolvedStance::fallback());

    Json(serde_json::json!({
        "dir": dir,
        "user_key": user_key,
        "stance": stance,
    }))
}

/// POST /api/stance — set a user's stance. `scope` (a `sensei.scopes` key like
/// `project` / `organization`) picks the rung; omitted / `user` / `general` sets
/// the user's default row. The three dials (`autonomy` · `sharing` · `review`)
/// are validated (400 on a bad value); an absent dial takes its DDL default
/// (full-replace, mirroring the collective-preferences PUT). Setting a non-default
/// scope for a folder that has no namespace at that scope is a 400 — a stance is
/// never silently written to the default when a real scope was asked for.
pub(crate) async fn set_stance(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let under = body.get("under").and_then(serde_json::Value::as_str);
    let user = body.get("user").and_then(serde_json::Value::as_str);
    let (dir, user_key) = resolve_who_where(under, user);
    if user_key.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "no user: pass `user` or run where a git identity resolves",
        ));
    }

    let input = StanceInput::from_request(&body).map_err(|e| err(StatusCode::BAD_REQUEST, &e))?;

    // Where: an omitted / always-on scope → the default row (namespace_id NULL);
    // a real scope must resolve to a namespace for this folder, else 400 (never a
    // silent default write).
    let scope = body.get("scope").and_then(serde_json::Value::as_str).filter(|s| !s.is_empty());
    let namespace_id = match scope {
        None | Some("user") | Some("general") => None,
        Some(scope_key) => {
            let folder_id = state
                .pg
                .get_repo_by_path(&dir)
                .await
                .ok()
                .flatten()
                .and_then(|f| crate::api::util::json_uuid(&f["id"]))
                .ok_or_else(|| {
                    err(
                        StatusCode::BAD_REQUEST,
                        "scope given but the folder is not an indexed repo",
                    )
                })?;
            Some(
                state
                    .pg
                    .namespace_for_folder_scope(&folder_id, scope_key)
                    .await
                    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
                    .ok_or_else(|| {
                        err(
                            StatusCode::BAD_REQUEST,
                            format!("this folder has no namespace at scope {scope_key:?}"),
                        )
                    })?,
            )
        }
    };

    let updated_at = state
        .pg
        .upsert_stance(
            &user_key,
            namespace_id.as_ref(),
            &input.autonomy,
            &input.sharing,
            &input.review,
        )
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    Ok(Json(serde_json::json!({
        "dir": dir,
        "user_key": user_key,
        "scope": scope,
        "stance": {
            "autonomy": input.autonomy,
            "sharing": input.sharing,
            "review": input.review,
            "source": if namespace_id.is_some() { "scoped" } else { "default" },
        },
        "updated_at": updated_at,
    })))
}
