//! `GET /api/user` — the effective git author identity for a folder plus the
//! sensei workspace that owns it. Backs the MCP `get_user_for_workspace` tool.
//!
//! It answers *who* is doing the work (git `user.name`/`user.email`, resolved
//! with git's own local→global precedence) and *where* (the workspace under the
//! folder, via the same folder-scoped resolver `find_projects` uses). A run or
//! plan can then be registered under the same identity a contributor uses to
//! sign in to the Dōjō, so relay attribution needs no hand-paired token.
//!
//! Never 500s: a missing git binary or a non-repo dir yields a `user` with null
//! fields rather than an error — the caller degrades gracefully.

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::api::state::AppState;
use crate::git_identity::read_git_user;
use crate::resolution::Resolution;

#[derive(Debug, Deserialize)]
pub(crate) struct UserQuery {
    /// Absolute folder to resolve the identity + owning workspace for. Defaults to
    /// the daemon's cwd when omitted (the MCP proxy always sends the caller's
    /// cwd, matching `find_projects`).
    under: Option<String>,
}

/// Resolve `{ dir, user, workspace }` for a folder.
pub(crate) async fn get_user(
    State(state): State<AppState>,
    Query(q): Query<UserQuery>,
) -> Json<serde_json::Value> {
    let dir = q
        .under
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_default();

    let user = read_git_user(std::path::Path::new(&dir));

    // The owning workspace = the workspace whose folders live under `dir` — the same
    // folder-scoped view `find_projects` returns. FAIL CLOSED (issue #109): name
    // a workspace only when EXACTLY one resolves. When `dir` is a container of
    // several workspaces, don't silently pick the first (that misattributes the
    // run/plan to the wrong workspace); surface `workspace_ambiguous` so the caller
    // scopes down instead. A DB error degrades to "no workspace", logged not 500'd.
    let workspaces = match state.pg.list_workspaces_under(Some(&dir)).await {
        Ok(ps) => ps,
        Err(e) => {
            tracing::warn!(error = %e, dir, "get_user: list_workspaces_under failed");
            Vec::new()
        }
    };
    let (workspace, workspace_ambiguous) = match Resolution::from_unique(workspaces) {
        Resolution::Resolved(p) => {
            (Some(serde_json::json!({ "id": p["id"], "name": p["name"] })), false)
        }
        Resolution::Ambiguous { .. } => (None, true),
        Resolution::Unresolved => (None, false),
    };

    Json(serde_json::json!({
        "dir": dir,
        "user": user,
        "workspace": workspace,
        "workspace_ambiguous": workspace_ambiguous,
    }))
}
