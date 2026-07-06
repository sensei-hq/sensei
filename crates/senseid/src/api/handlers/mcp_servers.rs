//! `/api/instruments/mcp-servers*` — #84 T2 Slice A. Discovered MCP servers
//! from ACP config files.
//!
//! - `GET  /api/instruments/mcp-servers?project_id=…?` — list; user-scope +
//!   the given project's project-scope rows (or user-scope only when the
//!   query is absent). Ordered by acp_family, then mcp_key.
//! - `PUT  /api/instruments/mcp-servers/{id}/enabled` — toggle. Body:
//!   `{ "enabled": <bool> }`.
//! - `POST /api/instruments/mcp-servers/refresh` — trigger a discovery pass
//!   against `$HOME` + every registered project's root. Returns the counts
//!   `{ discovered, pruned }`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::api::state::AppState;
use crate::tasks::mcp_discovery;

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    /// Optional project UUID (as string). Absent → user-scope only.
    pub project_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct EnabledBody {
    pub enabled: bool,
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({"error": msg})))
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = match q.project_id.as_deref() {
        Some(s) if !s.is_empty() => Some(uuid::Uuid::parse_str(s)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?),
        _ => None,
    };
    let rows = state.pg.list_mcp_servers(pid).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({"servers": rows})))
}

pub(crate) async fn set_enabled(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<EnabledBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let new_state = state.pg.set_mcp_server_enabled(&uuid, body.enabled).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "mcp server not found"))?;
    Ok(Json(serde_json::json!({"id": uuid, "enabled": new_state})))
}

/// Trigger a discovery pass. Reads `$HOME` (via `crate::paths::home()`) and
/// every registered project's absolute path; scans each for known ACP
/// configs; upserts + prunes stale.
pub(crate) async fn refresh(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home = crate::paths::home();

    // Registered projects — need (id, abs_path). Some projects are
    // multi-folder; use the first git-kind folder as the "project root"
    // for the ACP config scan, since that's where `.mcp.json`,
    // `.cursor/mcp.json`, and `.zed/settings.json` live.
    let projects = state.pg.list_projects().await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let mut project_roots: Vec<(uuid::Uuid, std::path::PathBuf)> = Vec::new();
    for p in projects {
        let Some(pid) = p["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) else { continue };
        let repos = state.pg.get_project_repos(&pid).await.unwrap_or_default();
        // First repo's path is the project root.
        if let Some(first) = repos.first()
            && let Some(path) = first["path"].as_str()
        {
            project_roots.push((pid, std::path::PathBuf::from(path)));
        }
    }

    let (discovered, pruned) = mcp_discovery::run_once(&state.pg, &home, &project_roots).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    Ok(Json(serde_json::json!({"discovered": discovered, "pruned": pruned})))
}
