//! `/api/instruments/mcp-servers*` — #84 T2 Slice A. Discovered MCP servers
//! from ACP config files.
//!
//! - `GET  /api/instruments/mcp-servers?workspace_id=…?` — list; user-scope +
//!   the given workspace's project-scope rows (or user-scope only when the
//!   query is absent). Ordered by acp_family, then mcp_key.
//! - `PUT  /api/instruments/mcp-servers/{id}/enabled` — toggle. Body:
//!   `{ "enabled": <bool> }`.
//! - `POST /api/instruments/mcp-servers/refresh` — trigger a discovery pass
//!   against `$HOME` + every registered workspace's root. Returns the counts
//!   `{ discovered, pruned }`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::api::handlers::err;
use crate::api::state::AppState;
use crate::tasks::mcp_discovery;

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    /// Optional workspace UUID (as string). Absent → user-scope only.
    pub workspace_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct EnabledBody {
    pub enabled: bool,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pid = match q.workspace_id.as_deref() {
        Some(s) if !s.is_empty() => Some(
            uuid::Uuid::parse_str(s)
                .map_err(|_| err(StatusCode::BAD_REQUEST, "bad workspace_id"))?,
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
/// every registered workspace's absolute path; scans each for known ACP
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
    let (command, args, env) = server_launch(&state, &uuid).await?;
    let outcome = crate::tasks::mcp_probe::probe_tools(&command, &args, &env, None).await;

    match outcome {
        crate::tasks::mcp_probe::ProbeOutcome::Ok(manifest) => {
            let tools = serde_json::Value::Array(manifest.tools.clone());
            state
                .pg
                .upsert_mcp_tool_manifest(&crate::db::pg_store::McpManifestRow {
                    server_id: &uuid,
                    tools: &tools,
                    tool_count: manifest.tools.len() as i32,
                    protocol_version: manifest.protocol_version.as_deref(),
                    server_name: manifest.server_name.as_deref(),
                    server_version: manifest.server_version.as_deref(),
                    error: None,
                })
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
        }
        crate::tasks::mcp_probe::ProbeOutcome::Error(msg) => {
            // Cache the error so callers see WHY the probe failed and don't
            // spam retries; TTL still applies so it retries eventually.
            let empty = serde_json::Value::Array(vec![]);
            state
                .pg
                .upsert_mcp_tool_manifest(&crate::db::pg_store::McpManifestRow::failed(
                    &uuid, &empty, &msg,
                ))
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

/// How to start a discovered server: its `command`, `args` and `env`, from its
/// config row. 404 when there is no such server.
async fn server_launch(
    state: &AppState,
    id: &uuid::Uuid,
) -> Result<(String, Vec<String>, HashMap<String, String>), (StatusCode, Json<serde_json::Value>)> {
    let server = state
        .pg
        .get_mcp_server_by_id(id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "mcp server not found"))?;
    let command = server.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let args: Vec<String> = server
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let env: HashMap<String, String> = server
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()
        })
        .unwrap_or_default();
    Ok((command, args, env))
}

#[derive(Deserialize)]
pub(crate) struct CallBody {
    pub tool: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// POST /api/instruments/mcp-servers/{id}/call — run one tool through the
/// server itself, over stdio, as an assistant would. Body:
/// `{ "tool": name, "arguments": {…} }`. Answers
/// `{ result, isError, duration_ms }` — a tool's own failure is its answer
/// (`isError: true`), while a server that cannot be reached is a 502 naming why.
pub(crate) async fn call(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CallBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    if body.tool.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "tool is required"));
    }
    let (command, args, env) = server_launch(&state, &uuid).await?;
    let arguments = if body.arguments.is_null() { serde_json::json!({}) } else { body.arguments };
    let started = std::time::Instant::now();
    let outcome =
        crate::tasks::mcp_probe::call_tool(&command, &args, &env, None, &body.tool, &arguments)
            .await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match outcome {
        crate::tasks::mcp_probe::CallOutcome::Ok { result, is_error } => Ok(Json(
            serde_json::json!({"result": result, "isError": is_error, "duration_ms": duration_ms}),
        )),
        crate::tasks::mcp_probe::CallOutcome::Error(e) => Err(err(StatusCode::BAD_GATEWAY, &e)),
    }
}

#[derive(Deserialize)]
pub(crate) struct RegisterBody {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// POST /api/instruments/mcp-servers — register a stdio server by hand
/// (#343). 409 when a server is already registered under the name.
pub(crate) async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = body.name.trim();
    let command = body.command.trim();
    if name.is_empty() || command.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "name and command are required"));
    }
    let id = state
        .pg
        .register_manual_mcp_server(
            name,
            command,
            &serde_json::json!(body.args),
            &serde_json::json!(body.env),
        )
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
        .ok_or_else(|| {
            err(StatusCode::CONFLICT, "a server is already registered under that name")
        })?;
    Ok(Json(serde_json::json!({"id": id})))
}

/// DELETE /api/instruments/mcp-servers/{id} — remove a hand-registered
/// server. A discovered one is refused (409): its config file owns it.
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| err(StatusCode::BAD_REQUEST, "bad id"))?;
    match state
        .pg
        .remove_manual_mcp_server(&uuid)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
    {
        None => Err(err(StatusCode::NOT_FOUND, "mcp server not found")),
        Some(false) => Err(err(
            StatusCode::CONFLICT,
            "a discovered server is removed from its assistant's config, not here",
        )),
        Some(true) => Ok(Json(serde_json::json!({"id": uuid, "removed": true}))),
    }
}

pub(crate) async fn refresh(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let home = crate::paths::home();

    // Registered workspaces — need (id, abs_path). Some workspaces are
    // multi-folder; use the first git-kind folder as the "workspace root"
    // for the ACP config scan, since that's where `.mcp.json`,
    // `.cursor/mcp.json`, and `.zed/settings.json` live.
    let workspaces =
        state.pg.list_workspaces().await.map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let mut workspace_roots: Vec<(uuid::Uuid, std::path::PathBuf)> = Vec::new();
    for p in workspaces {
        let Some(pid) = p["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) else {
            continue;
        };
        // Fail closed: a per-workspace repo read error must not silently drop that
        // workspace from the ACP config scan (masking a failure as "no repos").
        let repos = state
            .pg
            .get_workspace_repos(&pid)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
        // First repo's path is the workspace root.
        if let Some(first) = repos.first()
            && let Some(path) = first["path"].as_str()
        {
            workspace_roots.push((pid, std::path::PathBuf::from(path)));
        }
    }

    let (discovered, pruned) = mcp_discovery::run_once(&state.pg, &home, &workspace_roots)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    Ok(Json(serde_json::json!({"discovered": discovered, "pruned": pruned})))
}
