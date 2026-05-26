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
    let scope_project = q.solution_id.as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let min_repos: i64 = if q.shared.unwrap_or(false) { 2 } else { 1 };

    let libs = state.pg.list_libraries_with_usage(
        q.repo_id.as_deref(),
        scope_project.as_ref(),
        min_repos,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    // Fetch content (async)
    let content = crate::indexer::lib_indexer::fetch_lib_url(&body.url).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Upsert library into PgStore
    let lib_id = state.pg.upsert_library(
        &body.lib_name, "npm", body.version.as_deref(), Some(&content), Some("url"), Some(&body.url),
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "libName": body.lib_name,
        "libId": lib_id.to_string(),
        "docsIndexed": 1,
        "sourceType": "url",
        "version": body.version,
    })))
}

#[derive(Deserialize)]
pub(crate) struct LibDocsQuery {
    q: Option<String>,
}

pub(crate) async fn search_lib_docs(
    State(state): State<AppState>,
    Query(q): Query<LibDocsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let query = q.q.unwrap_or_default();
    if query.is_empty() {
        state.pg.list_libraries().await
    } else {
        state.pg.search_libraries(&query).await
    }.map(Json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn get_lib_docs(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    state.pg.get_library_by_name(&name).await
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
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folder = state.pg.get_repo_by_name(&q.repo_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let abs_path = folder["abs_path"].as_str().unwrap_or("");
    // Extract dependency versions from filesystem (no Store needed)
    let repo_path = std::path::Path::new(abs_path);
    if !repo_path.exists() {
        return Ok(Json(serde_json::json!([])));
    }

    // Read package.json / Cargo.toml for version info
    let mut deps = Vec::new();
    let pkg_json = repo_path.join("package.json");
    if pkg_json.exists()
        && let Ok(content) = std::fs::read_to_string(&pkg_json)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                for section in &["dependencies", "devDependencies"] {
                    if let Some(obj) = parsed.get(section).and_then(|v| v.as_object()) {
                        for (name, ver) in obj {
                            deps.push(serde_json::json!({"name": name, "version": ver, "source": section}));
                        }
                    }
                }
            }
    Ok(Json(serde_json::json!(deps)))
}
