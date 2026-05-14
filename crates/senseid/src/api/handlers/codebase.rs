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
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let nodes = state.pg.get_nodes_by_folder(&folder_id).await.unwrap_or_default();
            let edges = state.pg.get_edges_by_kind(&folder_id, "calls").await.unwrap_or_default();
            return Ok(Json(serde_json::json!({"nodes": nodes, "edges": edges})));
        }
    Ok(Json(serde_json::json!({"nodes": [], "edges": []})))
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
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let folder = state.pg.get_repo_by_name(&q.repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let results = state.pg.search_functions(&folder_id, &q.query).await.unwrap_or_default();
            return Ok(Json(results));
        }
    Ok(Json(vec![]))
}

pub(crate) async fn search_types(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let folder = state.pg.get_repo_by_name(&q.repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let results = state.pg.search_types(&folder_id, &q.query).await.unwrap_or_default();
            return Ok(Json(results));
        }
    Ok(Json(vec![]))
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
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let results = state.pg.get_callers_by_name(&q.repo_id, &q.name).await.unwrap_or_default();
    Ok(Json(results))
}

pub(crate) async fn fn_callees(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let results = state.pg.get_callees_by_name(&q.repo_id, &q.name).await.unwrap_or_default();
    Ok(Json(results))
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
    let results = state.pg.get_files_by_tag(&q.repo_id, &q.tag).await.unwrap_or_default();
    Ok(Json(serde_json::json!(results)))
}

pub(crate) async fn doc_drift(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let results = state.pg.get_doc_drift(&repo_id).await.unwrap_or_default();
    Ok(Json(serde_json::json!(results)))
}

pub(crate) async fn call_flow(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let edges = state.pg.get_edges_by_kind(&folder_id, "calls").await.unwrap_or_default();
            let nodes = state.pg.get_nodes_by_folder(&folder_id).await.unwrap_or_default();
            let modules: Vec<serde_json::Value> = nodes.iter()
                .filter(|n| n["kind"].as_str() == Some("file"))
                .map(|n| serde_json::json!({
                    "path": n["file_path"],
                    "exports": nodes.iter()
                        .filter(|c| c["file_path"] == n["file_path"] && c["is_exported"].as_bool() == Some(true))
                        .filter_map(|c| c["name"].as_str())
                        .collect::<Vec<_>>(),
                }))
                .collect();
            return Ok(Json(serde_json::json!({
                "modules": modules,
                "calls": edges,
                "moduleCount": modules.len(),
                "exportCount": modules.iter().map(|m| m["exports"].as_array().map_or(0, |a| a.len())).sum::<usize>(),
                "callCount": edges.len(),
            })));
        }
    Ok(Json(serde_json::json!({"modules": [], "calls": [], "moduleCount": 0, "exportCount": 0, "callCount": 0})))
}

pub(crate) async fn detect_communities(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = body["repoId"].as_str().unwrap_or_default().to_string();
    if repo_id.is_empty() {
        return Ok(Json(serde_json::json!({"error": "repoId required"})));
    }
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let communities = state.pg.list_communities(&folder_id).await.unwrap_or_default();
            let num = communities.len();
            return Ok(Json(serde_json::json!({
                "ok": true,
                "communities": num,
                "assignments": num,
            })));
        }
    Ok(Json(serde_json::json!({"ok": false, "error": "project not found"})))
}

pub(crate) async fn community_info(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let folder = state.pg.get_repo_by_name(&repo_id).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let communities = state.pg.list_communities(&folder_id).await.unwrap_or_default();
            return Ok(Json(serde_json::json!(communities)));
        }
    Ok(Json(serde_json::json!([])))
}

// ── Patterns ────────────────────────────────────────────────────────────────

pub(crate) async fn detect_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    // Look up folder UUID and query patterns from PgStore
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(serde_json::json!({"ok": true, "patterns": patterns, "count": patterns.len()}));
        }
    Json(serde_json::json!({"ok": false, "error": "project not found"}))
}

pub(crate) async fn list_patterns(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            return Json(serde_json::json!({"patterns": patterns, "count": patterns.len()}));
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
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let ranked = state.pg.rank_bm25(&folder_id, &desc).await.unwrap_or_default();
            let matches: Vec<serde_json::Value> = ranked.into_iter()
                .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
                .collect();
            return Json(serde_json::json!({"matches": matches, "count": matches.len()}));
        }
    Json(serde_json::json!({"matches": [], "count": 0}))
}

pub(crate) async fn pattern_for_symbol(
    State(state): State<AppState>,
    Path((project, symbol)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    // Search patterns by folder, then filter by symbol
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            let patterns = state.pg.list_patterns_by_folder(&folder_id).await.unwrap_or_default();
            // Find pattern whose members include this symbol
            for p in &patterns {
                if let Some(members) = p.get("members").and_then(|m| m.as_array())
                    && members.iter().any(|m| m.as_str() == Some(&symbol)) {
                        return Json(p.clone());
                    }
            }
        }
    Json(serde_json::json!({"pattern": null, "message": "symbol does not belong to any detected pattern"}))
}

pub(crate) async fn find_duplicates_handler(
    State(_state): State<AppState>,
    Path(_project): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: implement duplicate detection via graph analysis
    Json(serde_json::json!({"duplicates": [], "count": 0}))
}

pub(crate) async fn project_conventions_handler(
    State(_state): State<AppState>,
    Path(_project): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: implement convention detection via graph analysis
    Json(serde_json::json!({"conventions": [], "count": 0}))
}
