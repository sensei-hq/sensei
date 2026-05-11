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

/// Resolve a repo_id string to a folder UUID, returning None if not found.
pub(crate) async fn resolve_folder_id(state: &AppState, repo_id: &str) -> Option<uuid::Uuid> {
    if repo_id.is_empty() { return None; }
    state.pg.get_repo_by_name(repo_id).await.ok().flatten()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]))
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
    let results = if let Some(fid) = resolve_folder_id(state, repo_id).await {
        state.pg.search_functions(&fid, &term).await.unwrap_or_default()
    } else {
        vec![]
    };
    serde_json::json!({
        "type": "functions",
        "query": q,
        "results": results,
    })
}

pub(crate) async fn query_types(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let results = if let Some(fid) = resolve_folder_id(state, repo_id).await {
        state.pg.search_types(&fid, &term).await.unwrap_or_default()
    } else {
        vec![]
    };
    serde_json::json!({
        "type": "types",
        "query": q,
        "results": results,
    })
}

pub(crate) async fn query_callers(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    // TODO: implement — need node_uuid from name lookup
    let _ = (state, repo_id);
    serde_json::json!({
        "type": "callers",
        "query": q,
        "function": term,
        "results": [],
    })
}

pub(crate) async fn query_callees(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    // TODO: implement — need node_uuid from name lookup
    let _ = (state, repo_id);
    serde_json::json!({
        "type": "callees",
        "query": q,
        "function": term,
        "results": [],
    })
}

pub(crate) async fn query_files(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    // TODO: implement files_by_tag in PG
    let _ = (state, repo_id);
    serde_json::json!({ "type": "files", "query": q, "results": [] })
}

pub(crate) async fn query_patterns(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    // Extract pattern keyword and search functions by tag
    let tag = if q.contains("hook") { "hook" }
        else if q.contains("middleware") { "middleware" }
        else if q.contains("route") { "route" }
        else if q.contains("handler") { "handler" }
        else if q.contains("component") { "component" }
        else { &extract_search_term(q) };

    // TODO: implement files_by_tag in PG
    let _ = (state, repo_id);
    serde_json::json!({ "type": "patterns", "query": q, "pattern": tag, "results": [] })
}

pub(crate) async fn query_docs(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    // TODO: implement doc drift in PG
    let _ = (state, repo_id);
    serde_json::json!({
        "type": "docs",
        "query": q,
        "driftedDocs": [],
    })
}

pub(crate) async fn query_communities(state: &AppState, repo_id: &str) -> serde_json::Value {
    let communities = if let Some(fid) = resolve_folder_id(state, repo_id).await {
        state.pg.list_communities(&fid).await.unwrap_or_default()
    } else {
        vec![]
    };
    serde_json::json!({
        "type": "communities",
        "results": communities,
    })
}

pub(crate) async fn query_general(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let (functions, types) = if let Some(fid) = resolve_folder_id(state, repo_id).await {
        let fns = state.pg.search_functions(&fid, &term).await.unwrap_or_default();
        let tys = state.pg.search_types(&fid, &term).await.unwrap_or_default();
        (fns, tys)
    } else {
        (vec![], vec![])
    };

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
