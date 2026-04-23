use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct LibsQuery {
    /// Scope to a single repo
    #[serde(rename = "repoId")]
    repo_id: Option<String>,
    /// Scope to repos in this solution
    #[serde(rename = "solutionId")]
    solution_id: Option<String>,
    /// Only return libs used by 2+ repos
    #[serde(default)]
    shared: Option<bool>,
}

/// GET /api/libs — query detected libraries.
///   ?repoId=X      — libs for a single repo (monorepo use case)
///   ?solutionId=X  — scope to repos in a solution
///   ?shared=true   — only libs used by 2+ repos
pub(crate) async fn list_libs(
    State(state): State<AppState>,
    Query(q): Query<LibsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    let all_repos = store.list_repos().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Scope: repoId > solutionId > all
    let repos: Vec<_> = if let Some(rid) = &q.repo_id {
        all_repos.into_iter().filter(|p| p.repo_id == *rid).collect()
    } else if let Some(pid) = &q.solution_id {
        all_repos.into_iter().filter(|r| r.project_id.as_deref() == Some(pid.as_str())).collect()
    } else {
        all_repos
    };

    // lib_name → [repo_ids]
    let mut lib_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for p in &repos {
        for lib in &p.libs {
            lib_map.entry(lib.clone()).or_default().push(p.repo_id.clone());
        }
    }

    // Filter shared if requested
    let min_repos = if q.shared.unwrap_or(false) { 2 } else { 1 };

    let mut libs: Vec<serde_json::Value> = lib_map.into_iter()
        .filter(|(_, repos)| repos.len() >= min_repos)
        .map(|(name, repos)| serde_json::json!({
            "name": name,
            "repos": repos,
            "repoCount": repos.len(),
        }))
        .collect();
    libs.sort_by(|a, b| b["repoCount"].as_u64().cmp(&a["repoCount"].as_u64()));

    Ok(Json(serde_json::json!({
        "total": libs.len(),
        "libs": libs,
    })))
}

#[derive(Deserialize)]
pub(crate) struct IndexLibBody {
    #[serde(rename = "libName")]
    lib_name: String,
    url: String,
    version: Option<String>,
}

pub(crate) async fn index_lib(
    State(state): State<AppState>,
    Json(body): Json<IndexLibBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Fetch content first (async), then store (sync with lock)
    let content = crate::indexer::lib_indexer::fetch_lib_url(&body.url).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let store = state.store.lock().await;
    match crate::indexer::lib_indexer::index_lib_content(
        &store, &body.lib_name, &body.url, &content, body.version.as_deref()
    ) {
        Ok(result) => Ok(Json(serde_json::json!({
            "ok": true,
            "libName": result.lib_name,
            "docsIndexed": result.docs_indexed,
            "sourceType": result.source_type,
            "version": result.version,
        }))),
        Err(e) => Ok(Json(serde_json::json!({"ok": false, "error": e}))),
    }
}

#[derive(Deserialize)]
pub(crate) struct LibDocsQuery {
    q: Option<String>,
}

pub(crate) async fn search_lib_docs(
    State(state): State<AppState>,
    Query(q): Query<LibDocsQuery>,
) -> Result<Json<Vec<crate::indexer::lib_indexer::LibDoc>>, StatusCode> {
    let query = q.q.unwrap_or_default();
    let store = state.store.lock().await;
    store.search_lib_docs(&query)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn get_lib_docs(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Vec<crate::indexer::lib_indexer::LibDoc>>, StatusCode> {
    let store = state.store.lock().await;
    store.get_lib_docs(&name)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
pub(crate) struct DepVersionsQuery {
    #[serde(rename = "repoId")]
    repo_id: String,
}

pub(crate) async fn get_dep_versions(
    State(state): State<AppState>,
    Query(q): Query<DepVersionsQuery>,
) -> Result<Json<Vec<crate::indexer::lib_indexer::DepVersion>>, StatusCode> {
    let store = state.store.lock().await;
    let repo = store.get_repo(&q.repo_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    crate::indexer::lib_indexer::extract_dep_versions(&store, &q.repo_id, &repo.path)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
