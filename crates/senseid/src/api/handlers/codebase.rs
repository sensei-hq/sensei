use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

// ── Graph Queries ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct GraphQuery {
    #[serde(rename = "repoId")]
    pub repo_id: Option<String>,
}

pub(crate) async fn graph_nodes(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    if repo_id.is_empty() {
        return Ok(Json(serde_json::json!({"nodes": [], "edges": []})));
    }
    let graph = state.graph.lock().await;
    let nodes = graph.get_nodes(&repo_id).unwrap_or_default();
    let edges = graph.get_edges(&repo_id).unwrap_or_default();
    Ok(Json(serde_json::json!({"nodes": nodes, "edges": edges})))
}

#[derive(Deserialize)]
pub(crate) struct SymbolQuery {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    #[serde(rename = "q")]
    pub query: String,
}

pub(crate) async fn search_functions(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<crate::types::FunctionDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.search_functions(&q.query, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn search_types(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<crate::types::TypeDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.search_types(&q.query, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
pub(crate) struct TraceQuery {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    pub name: String,
}

pub(crate) async fn fn_callers(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<crate::types::FunctionDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.callers_of(&q.name, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn fn_callees(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<crate::types::FunctionDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.callees_of(&q.name, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
pub(crate) struct TagQuery {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    pub tag: String,
}

pub(crate) async fn files_by_tag(
    State(state): State<AppState>,
    Query(q): Query<TagQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let graph = state.graph.lock().await;
    match graph.files_by_tag(&q.tag, &q.repo_id) {
        Ok(files) => {
            let result: Vec<serde_json::Value> = files.into_iter().map(|(id, path, tags)| {
                serde_json::json!({"id": id, "path": path, "tags": tags})
            }).collect();
            Ok(Json(serde_json::json!(result)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub(crate) async fn doc_drift(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<Vec<crate::indexer::graph::DocDrift>>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let graph = state.graph.lock().await;
    graph.get_doc_drift(&repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn call_flow(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let graph = state.graph.lock().await;
    graph.get_call_flow(&repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn detect_communities(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = body["repoId"].as_str().unwrap_or_default().to_string();
    if repo_id.is_empty() {
        return Ok(Json(serde_json::json!({"error": "repoId required"})));
    }
    let graph = state.graph.lock().await;
    match crate::indexer::community::detect_communities(&graph, &repo_id) {
        Ok(communities) => {
            let num = communities.values().collect::<std::collections::HashSet<_>>().len();
            Ok(Json(serde_json::json!({
                "ok": true,
                "communities": num,
                "assignments": communities.len(),
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({"ok": false, "error": e}))),
    }
}

pub(crate) async fn community_info(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<Vec<crate::indexer::community::CommunityInfo>>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let graph = state.graph.lock().await;
    graph.get_communities(&repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Patterns ────────────────────────────────────────────────────────────────

pub(crate) async fn detect_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let graph = state.graph.lock().await;
    match store.detect_patterns_from_graph(graph.conn_ref(), &project) {
        Ok(patterns) => Json(serde_json::json!({"ok": true, "patterns": patterns, "count": patterns.len()})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

pub(crate) async fn list_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    match store.list_detected_patterns(&project) {
        Ok(patterns) => Json(serde_json::json!({"patterns": patterns, "count": patterns.len()})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub(crate) struct MatchQuery {
    pub description: Option<String>,
}

pub(crate) async fn match_pattern_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<MatchQuery>,
) -> Json<serde_json::Value> {
    let desc = q.description.unwrap_or_default();
    let store = state.store.lock().await;
    match store.match_pattern(&project, &desc) {
        Ok(matches) => Json(serde_json::json!({"matches": matches, "count": matches.len()})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub(crate) async fn pattern_for_symbol(
    State(state): State<AppState>,
    Path((project, symbol)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    match store.get_pattern_for(&project, &symbol) {
        Ok(Some(pattern)) => Json(pattern),
        Ok(None) => Json(serde_json::json!({"pattern": null, "message": "symbol does not belong to any detected pattern"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub(crate) async fn find_duplicates_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let graph = state.graph.lock().await;
    match store.find_duplicates(graph.conn_ref(), &project) {
        Ok(dups) => Json(serde_json::json!({"duplicates": dups, "count": dups.len()})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub(crate) async fn project_conventions_handler(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let graph = state.graph.lock().await;
    match store.get_project_conventions(graph.conn_ref(), &project) {
        Ok(conventions) => Json(serde_json::json!({"conventions": conventions, "count": conventions.len()})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
