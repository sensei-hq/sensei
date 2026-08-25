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
        Some(s) if !s.is_empty() => Some(
            uuid::Uuid::parse_str(s).map_err(|_| err(StatusCode::BAD_REQUEST, "bad project_id"))?,
        ),
        _ => None,
    };
    let rows = state
        .pg
        .list_mcp_servers(pid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(serde_json::json!({"servers": rows})))
}

pub(crate) async fn set_enabled(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<EnabledBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let new_state = state
        .pg
        .set_mcp_server_enabled(&uuid, body.enabled)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "mcp server not found"))?;
    Ok(Json(serde_json::json!({"id": uuid, "enabled": new_state})))
}

/// Trigger a discovery pass. Reads `$HOME` (via `crate::paths::home()`) and
/// every registered project's absolute path; scans each for known ACP
/// configs; upserts + prunes stale.
/// GET /api/instruments/mcp-servers/{id}/tools — Slice B. Return the cached
/// tool manifest. If `?refresh=true` OR the cache is older than
/// `ttl_seconds`, spawn a fresh probe first. Cache misses (no row yet) also
/// trigger a probe unless the caller passes `?probe=false`.
pub(crate) async fn get_tools(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    let force = q.get("refresh").is_some_and(|v| v == "true" || v == "1");
    let allow_probe = q.get("probe").map(|v| v != "false" && v != "0").unwrap_or(true);

    let cached = state
        .pg
        .get_mcp_tool_manifest(&uuid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    let stale = cached.as_ref().is_some_and(|c| {
        let age = c.get("age_seconds").and_then(|v| v.as_i64()).unwrap_or(0);
        let ttl = c.get("ttl_seconds").and_then(|v| v.as_i64()).unwrap_or(900);
        age > ttl
    });

    if let Some(mut c) = cached {
        if !force && !stale {
            return Ok(Json(c));
        }
        if !allow_probe {
            // Caller doesn't want us to spawn; return whatever we have with a
            // `stale: true` hint so the UI can badge it.
            if let Some(obj) = c.as_object_mut() {
                obj.insert("stale".into(), serde_json::Value::Bool(true));
            }
            return Ok(Json(c));
        }
    }

    // Probe path: fetch the server config and spawn the tool manifest probe.
    let server = state
        .pg
        .get_mcp_server_by_id(&uuid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "mcp server not found"))?;

    let command = server.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let args: Vec<String> = server
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let env: std::collections::HashMap<String, String> = server
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()
        })
        .unwrap_or_default();

    let outcome = crate::tasks::mcp_probe::probe_tools(&command, &args, &env, None).await;

    match outcome {
        crate::tasks::mcp_probe::ProbeOutcome::Ok(manifest) => {
            let tools = serde_json::Value::Array(manifest.tools.clone());
            state
                .pg
                .upsert_mcp_tool_manifest(
                    &uuid,
                    &tools,
                    manifest.tools.len() as i32,
                    manifest.protocol_version.as_deref(),
                    manifest.server_name.as_deref(),
                    manifest.server_version.as_deref(),
                    None,
                )
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
        }
        crate::tasks::mcp_probe::ProbeOutcome::Error(msg) => {
            // Cache the error so callers see WHY the probe failed and don't
            // spam retries; TTL still applies so it retries eventually.
            let empty = serde_json::Value::Array(vec![]);
            state
                .pg
                .upsert_mcp_tool_manifest(&uuid, &empty, 0, None, None, None, Some(&msg))
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
        }
    }

    // Return the freshly-updated row.
    let refreshed = state
        .pg
        .get_mcp_tool_manifest(&uuid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| {
            err(StatusCode::INTERNAL_SERVER_ERROR, "manifest upserted but not readable")
        })?;
    Ok(Json(refreshed))
}

pub(crate) async fn refresh(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home = crate::paths::home();

    // Registered projects — need (id, abs_path). Some projects are
    // multi-folder; use the first git-kind folder as the "project root"
    // for the ACP config scan, since that's where `.mcp.json`,
    // `.cursor/mcp.json`, and `.zed/settings.json` live.
    let projects =
        state.pg.list_projects().await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let mut project_roots: Vec<(uuid::Uuid, std::path::PathBuf)> = Vec::new();
    for p in projects {
        let Some(pid) = p["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) else {
            continue;
        };
        // Fail closed: a per-project repo read error must not silently drop that
        // project from the ACP config scan (masking a failure as "no repos").
        let repos = state
            .pg
            .get_project_repos(&pid)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
        // First repo's path is the project root.
        if let Some(first) = repos.first()
            && let Some(path) = first["path"].as_str()
        {
            project_roots.push((pid, std::path::PathBuf::from(path)));
        }
    }

    let (discovered, pruned) = mcp_discovery::run_once(&state.pg, &home, &project_roots)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    Ok(Json(serde_json::json!({"discovered": discovered, "pruned": pruned})))
}
