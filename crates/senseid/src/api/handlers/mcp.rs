use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use crate::api::state::AppState;
use crate::api::util::json_uuid;
use super::query::resolve_folder_id;

// ── MCP Tool Proxy ──────────────────────────────────────────────────────────

pub(crate) async fn mcp_list_tools() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "tools": [
            {"name": "search", "description": "Search functions and types by name", "params": ["query", "repoId"]},
            {"name": "get_symbol", "description": "Get function details by name", "params": ["name", "repoId"]},
            {"name": "get_callers", "description": "Get callers of a function", "params": ["name", "repoId"]},
            {"name": "get_callees", "description": "Get callees of a function", "params": ["name", "repoId"]},
            {"name": "get_file_tags", "description": "Get files by framework tag", "params": ["tag", "repoId"]},
            {"name": "get_communities", "description": "Get code communities for a project", "params": ["repoId"]},
            {"name": "get_doc_drift", "description": "Get docs that may be stale", "params": ["repoId"]},
            {"name": "search_lib_docs", "description": "Search indexed library documentation", "params": ["query"]},
            {"name": "query", "description": "Natural language query across graph", "params": ["q", "repoId"]},
            {"name": "get_project_summary", "description": "Get project stats and metadata", "params": ["repoId"]},
            {"name": "get_metrics", "description": "Get project quality metrics: FTR, turn count, rework rate, tool adherence", "params": ["repoId"]},
        ]
    }))
}

pub(crate) async fn mcp_call_tool(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tool = body["tool"].as_str().unwrap_or("");
    let params = &body["params"];
    let repo_id = params["repoId"].as_str().unwrap_or("");
    let query = params["query"].as_str().or(params["q"].as_str()).or(params["name"].as_str()).unwrap_or("");

    let result = match tool {
        "search" => {
            let (fns, types) = if let Some(fid) = resolve_folder_id(&state, repo_id).await {
                let f = state.pg.search_functions(&fid, query).await.unwrap_or_default();
                let t = state.pg.search_types(&fid, query).await.unwrap_or_default();
                (f, t)
            } else {
                (vec![], vec![])
            };
            serde_json::json!({"functions": fns, "types": types})
        }
        "get_symbol" => {
            let fns = if let Some(fid) = resolve_folder_id(&state, repo_id).await {
                state.pg.search_functions(&fid, query).await.unwrap_or_default()
            } else {
                vec![]
            };
            serde_json::json!({"results": fns})
        }
        "get_callers" => {
            let name = params["name"].as_str().unwrap_or(query);
            let callers = state.pg.get_callers_by_name(repo_id, name).await.unwrap_or_default();
            serde_json::json!({"callers": callers})
        }
        "get_callees" => {
            let name = params["name"].as_str().unwrap_or(query);
            let callees = state.pg.get_callees_by_name(repo_id, name).await.unwrap_or_default();
            serde_json::json!({"callees": callees})
        }
        "get_file_tags" => {
            let tag = params["tag"].as_str().unwrap_or(query);
            let files = state.pg.get_files_by_tag(repo_id, tag).await.unwrap_or_default();
            serde_json::json!({"files": files})
        }
        "get_communities" => {
            let communities = if let Some(fid) = resolve_folder_id(&state, repo_id).await {
                state.pg.list_communities(&fid).await.unwrap_or_default()
            } else {
                vec![]
            };
            serde_json::json!({"communities": communities})
        }
        "get_doc_drift" => {
            let drift = state.pg.get_doc_drift(repo_id).await.unwrap_or_default();
            serde_json::json!({"drift": drift})
        }
        "search_lib_docs" => {
            let docs = state.pg.list_libraries().await.unwrap_or_default();
            serde_json::json!({"docs": docs})
        }
        "get_lib_docs" => {
            let _name = params["name"].as_str().unwrap_or(query);
            let docs = state.pg.list_libraries().await.unwrap_or_default();
            serde_json::json!({"docs": docs})
        }
        "list_projects" => {
            let repos = state.pg.list_repositories().await.unwrap_or_default();
            serde_json::json!({"projects": repos})
        }
        "create_session" => {
            let repo_id_str = params["repoId"].as_str().unwrap_or(query);
            let task = params["task"].as_str().unwrap_or("untitled");
            // Look up folder UUID from repo name
            let folder = state.pg.get_repo_by_name(repo_id_str).await.ok().flatten();
            if let Some(folder) = folder {
                if let Some(folder_id) = json_uuid(&folder["id"]) {
                    match state.pg.create_session(&folder_id, task, None).await {
                        Ok(session_id) => serde_json::json!({"ok": true, "sessionId": session_id.to_string()}),
                        Err(e) => serde_json::json!({"error": e}),
                    }
                } else {
                    serde_json::json!({"error": "invalid folder id"})
                }
            } else {
                serde_json::json!({"error": "repo not found"})
            }
        }
        "update_session" => {
            let session_id_str = params["sessionId"].as_str().unwrap_or("");
            if let Ok(session_id) = uuid::Uuid::parse_str(session_id_str) {
                let outcome = params["outcome"].as_str().unwrap_or("completed");
                let ftr = outcome == "completed";
                let turns = params["turns"].as_i64().unwrap_or(0) as i32;
                let corrections = params["corrections"].as_i64().unwrap_or(0) as i32;
                state.pg.complete_session(
                    &session_id,
                    outcome,
                    ftr,
                    turns,
                    corrections,
                ).await.ok();
                serde_json::json!({"ok": true})
            } else {
                serde_json::json!({"error": "invalid sessionId"})
            }
        }
        "add_library" => {
            let name = params["name"].as_str().unwrap_or("");
            let explicit_url = params["url"].as_str().unwrap_or("");
            let version = params["version"].as_str();
            if name.is_empty() {
                serde_json::json!({"error": "name required"})
            } else {
                // Try explicit URL first, then auto-discover
                let urls_to_try: Vec<String> = if !explicit_url.is_empty() {
                    vec![explicit_url.to_string()]
                } else {
                    // Common llms.txt URL patterns
                    let clean = name.trim_start_matches('@').replace('/', "-");
                    vec![
                        format!("https://{}.com/llms.txt", clean),
                        format!("https://{}.dev/llms.txt", clean),
                        format!("https://{}.com/llms-full.txt", clean),
                        format!("https://{}.io/llms.txt", clean),
                        format!("https://www.{}.com/llms.txt", clean),
                        format!("https://raw.githubusercontent.com/{name}/main/llms.txt"),
                        format!("https://raw.githubusercontent.com/{name}/master/README.md"),
                    ]
                };

                let mut result_json = serde_json::json!({"error": "Could not find docs. Provide a url parameter."});
                let mut tried = Vec::new();

                for url in &urls_to_try {
                    tried.push(url.clone());
                    // Short timeout for auto-discovery probing
                    let timeout = if explicit_url.is_empty() { 5 } else { 15 };
                    match crate::indexer::lib_indexer::fetch_lib_url_with_timeout(url, timeout).await {
                        Ok(content) if content.len() > 50 => {
                            match state.pg.upsert_library(name, "npm", version, Some(&content), Some("url"), Some(url)).await {
                                Ok(lib_id) => {
                                    result_json = serde_json::json!({
                                        "ok": true,
                                        "libName": name,
                                        "libId": lib_id.to_string(),
                                        "docsIndexed": 1,
                                        "sourceType": "url",
                                        "url": url,
                                    });
                                    break;
                                }
                                Err(_) => continue,
                            }
                        }
                        _ => continue,
                    }
                }

                if !result_json["ok"].as_bool().unwrap_or(false) {
                    result_json = serde_json::json!({
                        "error": format!("Could not find docs for '{}'. Tried: {}. Provide an explicit url.", name, tried.join(", ")),
                    });
                }

                result_json
            }
        }
        "query" => {
            // Reuse unified query logic — delegate to POST /api/query handler
            serde_json::json!({"hint": "Use POST /api/query directly"})
        }
        "get_project_summary" => {
            let folder = state.pg.get_repo_by_name(repo_id).await.ok().flatten();
            let (fns, types) = if let Some(fid) = resolve_folder_id(&state, repo_id).await {
                let counts = state.pg.count_nodes_by_kind(&fid).await.unwrap_or_default();
                let f = counts.get("function").copied().unwrap_or(0)
                    + counts.get("method").copied().unwrap_or(0);
                let t = counts.get("class").copied().unwrap_or(0)
                    + counts.get("struct").copied().unwrap_or(0)
                    + counts.get("interface").copied().unwrap_or(0);
                (f, t)
            } else {
                (0, 0)
            };
            serde_json::json!({
                "project": folder,
                "functions": fns,
                "types": types,
            })
        }
        "get_metrics" => {
            let folder = state.pg.get_repo_by_name(repo_id).await.ok().flatten();
            if let Some(folder) = folder {
                if let Some(folder_id) = json_uuid(&folder["id"]) {
                    let sessions = state.pg.list_sessions_by_folder(&folder_id, 100).await.unwrap_or_default();
                    let session_count = sessions.len();
                    let completed = sessions.iter().filter(|s| s["outcome"].as_str() == Some("completed")).count();
                    serde_json::json!({
                        "project": repo_id,
                        "sessions": session_count,
                        "completed": completed,
                        "ftr": if session_count > 0 { completed as f64 / session_count as f64 } else { 0.0 },
                    })
                } else {
                    serde_json::json!({"error": "invalid folder id"})
                }
            } else {
                serde_json::json!({"error": "project not found"})
            }
        }
        _ => serde_json::json!({"error": format!("Unknown tool: {}", tool)}),
    };

    Ok(Json(result))
}
