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
    // Look up folder UUID and query patterns from PgStore
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(serde_json::json!({"ok": true, "patterns": patterns, "count": patterns.len()}));
        }
    }
    Json(serde_json::json!({"ok": false, "error": "project not found"}))
}

pub(crate) async fn list_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(serde_json::json!({"patterns": patterns, "count": patterns.len()}));
        }
    }
    Json(serde_json::json!({"patterns": [], "count": 0}))
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
    // Search patterns by folder using BM25 ranking
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            let ranked = state.pg.rank_bm25(&folder_id, &desc).await.unwrap_or_default();
            let matches: Vec<serde_json::Value> = ranked.into_iter()
                .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
                .collect();
            return Json(serde_json::json!({"matches": matches, "count": matches.len()}));
        }
    }
    Json(serde_json::json!({"matches": [], "count": 0}))
}

pub(crate) async fn pattern_for_symbol(
    State(state): State<AppState>,
    Path((project, symbol)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    // Search patterns by folder, then filter by symbol
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            // Find pattern whose members include this symbol
            for p in &patterns {
                if let Some(members) = p.get("members").and_then(|m| m.as_array()) {
                    if members.iter().any(|m| m.as_str() == Some(&symbol)) {
                        return Json(p.clone());
                    }
                }
            }
        }
    }
    Json(serde_json::json!({"pattern": null, "message": "symbol does not belong to any detected pattern"}))
}

pub(crate) async fn find_duplicates_handler(
    State(_state): State<AppState>,
    Path(_project): Path<String>,
) -> Json<serde_json::Value> {
    // Duplicate detection requires graph analysis; pending migration to PgStore nodes/edges.
    Json(serde_json::json!({"duplicates": [], "count": 0, "note": "pending migration to PgStore"}))
}

pub(crate) async fn project_conventions_handler(
    State(_state): State<AppState>,
    Path(_project): Path<String>,
) -> Json<serde_json::Value> {
    // Convention detection requires graph analysis; pending migration to PgStore nodes/edges.
    Json(serde_json::json!({"conventions": [], "count": 0, "note": "pending migration to PgStore"}))
}
