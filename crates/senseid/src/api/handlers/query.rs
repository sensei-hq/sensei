use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

// ── Unified Query ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct QueryBody {
    /// The query string, e.g. "find auth functions in kavach"
    #[serde(rename = "q")]
    query: String,
    /// Optional repo scope
    #[serde(rename = "repoId")]
    repo_id: Option<String>,
    /// Optional solution scope
    #[serde(rename = "solutionId")]
    solution_id: Option<String>,
}

/// POST /api/query — unified query endpoint for desktop and MCP.
/// Routes queries to appropriate backends based on keywords.
pub(crate) async fn unified_query(
    State(state): State<AppState>,
    Json(body): Json<QueryBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let q = body.query.to_lowercase();
    let repo_id = body.repo_id.clone().unwrap_or_default();

    // Determine query type from keywords
    let result = if q.contains("lib") || q.contains("dependenc") || q.contains("package") {
        // Library query
        query_libs(&state, &q, &repo_id, &body.solution_id).await
    } else if q.contains("function") || q.contains("method") || q.contains("fn ") || q.contains("def ") {
        // Function search
        query_functions(&state, &q, &repo_id).await
    } else if q.contains("type") || q.contains("interface") || q.contains("class") || q.contains("struct") {
        // Type search
        query_types(&state, &q, &repo_id).await
    } else if q.contains("who calls") || q.contains("callers") || q.contains("called by") {
        // Caller traceability
        query_callers(&state, &q, &repo_id).await
    } else if q.contains("calls") || q.contains("callees") || q.contains("depends on") {
        // Callee traceability
        query_callees(&state, &q, &repo_id).await
    } else if q.contains("file") || q.contains("component") || q.contains("tagged") || q.contains("framework") {
        // File/tag search
        query_files(&state, &q, &repo_id).await
    } else if q.contains("pattern") || q.contains("hook") || q.contains("middleware") || q.contains("route") {
        // Pattern search (via tags)
        query_patterns(&state, &q, &repo_id).await
    } else if q.contains("doc") || q.contains("readme") || q.contains("drift") {
        // Doc query
        query_docs(&state, &q, &repo_id).await
    } else if q.contains("communit") || q.contains("cluster") || q.contains("module") {
        // Community/architecture query
        query_communities(&state, &repo_id).await
    } else {
        // Default: search functions then types then lib docs
        query_general(&state, &q, &repo_id).await
    };

    Ok(Json(result))
}

pub(crate) async fn query_libs(state: &AppState, q: &str, repo_id: &str, _solution_id: &Option<String>) -> serde_json::Value {
    let repos = state.pg.list_repositories().await.unwrap_or_default();

    let filtered: Vec<&serde_json::Value> = if !repo_id.is_empty() {
        repos.iter().filter(|p| p["name"].as_str() == Some(repo_id)).collect()
    } else {
        repos.iter().collect()
    };

    let mut all_libs: Vec<serde_json::Value> = Vec::new();
    for p in &filtered {
        let repo_name = p["name"].as_str().unwrap_or("");
        if let Some(libs_arr) = p["libs"].as_array() {
            for lib in libs_arr {
                if let Some(lib_str) = lib.as_str() {
                    all_libs.push(serde_json::json!({"name": lib_str, "repoId": repo_name}));
                }
            }
        }
    }

    // Also search libraries from PgStore
    let lib_docs = state.pg.list_libraries().await.unwrap_or_default();
    let _term = extract_search_term(q);

    serde_json::json!({
        "type": "libs",
        "query": q,
        "libs": all_libs,
        "libDocs": lib_docs.iter().take(5).map(|d| serde_json::json!({
            "title": d["name"], "summary": d.get("description").unwrap_or(&serde_json::json!(null)), "url": d.get("url"),
        })).collect::<Vec<_>>(),
    })
}

pub(crate) async fn query_functions(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.search_functions(&term, repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "functions",
        "query": q,
        "results": results,
    })
}

pub(crate) async fn query_types(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.search_types(&term, repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "types",
        "query": q,
        "results": results,
    })
}

pub(crate) async fn query_callers(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.callers_of(&term, repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "callers",
        "query": q,
        "function": term,
        "results": results,
    })
}

pub(crate) async fn query_callees(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.callees_of(&term, repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "callees",
        "query": q,
        "function": term,
        "results": results,
    })
}

pub(crate) async fn query_files(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.files_by_tag(&term, repo_id).unwrap_or_default();
    let files: Vec<serde_json::Value> = results.into_iter()
        .map(|(id, path, tags)| serde_json::json!({"id": id, "path": path, "tags": tags}))
        .collect();
    serde_json::json!({ "type": "files", "query": q, "results": files })
}

pub(crate) async fn query_patterns(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    // Extract pattern keyword and search functions by tag
    let tag = if q.contains("hook") { "hook" }
        else if q.contains("middleware") { "middleware" }
        else if q.contains("route") { "route" }
        else if q.contains("handler") { "handler" }
        else if q.contains("component") { "component" }
        else { &extract_search_term(q) };

    let graph = state.graph.lock().await;
    let files = graph.files_by_tag(tag, repo_id).unwrap_or_default();
    let file_results: Vec<serde_json::Value> = files.into_iter()
        .map(|(_id, path, tags)| serde_json::json!({"path": path, "tags": tags}))
        .collect();
    serde_json::json!({ "type": "patterns", "query": q, "pattern": tag, "results": file_results })
}

pub(crate) async fn query_docs(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let graph = state.graph.lock().await;
    let drift = graph.get_doc_drift(repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "docs",
        "query": q,
        "driftedDocs": drift,
    })
}

pub(crate) async fn query_communities(state: &AppState, repo_id: &str) -> serde_json::Value {
    let graph = state.graph.lock().await;
    let communities = graph.get_communities(repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "communities",
        "results": communities,
    })
}

pub(crate) async fn query_general(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let functions = graph.search_functions(&term, repo_id).unwrap_or_default();
    let types = graph.search_types(&term, repo_id).unwrap_or_default();
    drop(graph);

    let lib_docs = state.pg.list_libraries().await.unwrap_or_default();

    serde_json::json!({
        "type": "general",
        "query": q,
        "functions": functions,
        "types": types,
        "libDocs": lib_docs.iter().take(5).map(|d| serde_json::json!({
            "title": d["name"], "summary": d.get("description").unwrap_or(&serde_json::json!(null)),
        })).collect::<Vec<_>>(),
    })
}

/// Extract the most meaningful search term from a natural language query.
pub(crate) fn extract_search_term(q: &str) -> String {
    let stop_words = ["find", "search", "show", "get", "list", "what", "which", "where",
        "how", "the", "in", "for", "from", "all", "me", "that", "are", "is",
        "function", "functions", "method", "methods", "type", "types", "class",
        "interface", "lib", "libs", "library", "libraries", "file", "files",
        "who", "calls", "called", "by", "does", "do", "callers", "callees",
        "pattern", "patterns", "doc", "docs", "a", "an", "of", "with"];

    let words: Vec<&str> = q.split_whitespace()
        .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
        .collect();

    // Return the longest non-stop word (likely the most specific)
    words.into_iter()
        .max_by_key(|w| w.len())
        .unwrap_or("")
        .to_string()
}

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
            let graph = state.graph.lock().await;
            let fns = graph.search_functions(query, repo_id).unwrap_or_default();
            let types = graph.search_types(query, repo_id).unwrap_or_default();
            serde_json::json!({"functions": fns, "types": types})
        }
        "get_symbol" => {
            let graph = state.graph.lock().await;
            let fns = graph.search_functions(query, repo_id).unwrap_or_default();
            serde_json::json!({"results": fns})
        }
        "get_callers" => {
            let graph = state.graph.lock().await;
            let results = graph.callers_of(query, repo_id).unwrap_or_default();
            serde_json::json!({"callers": results})
        }
        "get_callees" => {
            let graph = state.graph.lock().await;
            let results = graph.callees_of(query, repo_id).unwrap_or_default();
            serde_json::json!({"callees": results})
        }
        "get_file_tags" => {
            let tag = params["tag"].as_str().unwrap_or(query);
            let graph = state.graph.lock().await;
            let files = graph.files_by_tag(tag, repo_id).unwrap_or_default();
            let results: Vec<serde_json::Value> = files.into_iter()
                .map(|(_, path, tags)| serde_json::json!({"path": path, "tags": tags}))
                .collect();
            serde_json::json!({"files": results})
        }
        "get_communities" => {
            let graph = state.graph.lock().await;
            let communities = graph.get_communities(repo_id).unwrap_or_default();
            serde_json::json!({"communities": communities})
        }
        "get_doc_drift" => {
            let graph = state.graph.lock().await;
            let drift = graph.get_doc_drift(repo_id).unwrap_or_default();
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
                if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
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
            let graph = state.graph.lock().await;
            let (fns, types) = graph.count_symbols(repo_id).unwrap_or((0, 0));
            serde_json::json!({
                "project": folder,
                "functions": fns,
                "types": types,
            })
        }
        "get_metrics" => {
            let folder = state.pg.get_repo_by_name(repo_id).await.ok().flatten();
            if let Some(folder) = folder {
                if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
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
