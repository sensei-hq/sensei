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

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::state::AppState;
use crate::git_identity::read_git_user;

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
    let dir = q
        .under
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .unwrap_or_default();

    // Who: explicit override, else the git author at the folder (its email is the
    // user_key, the same identity used to sign in to the dōjō).
    let user_key = q
        .user
        .filter(|s| !s.is_empty())
        .or_else(|| read_git_user(std::path::Path::new(&dir)).email)
        .unwrap_or_default();

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
