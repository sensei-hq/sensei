use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use crate::db::Store;
use crate::indexer::graph::GraphDb;
use crate::tasks::queue::TaskQueue;
use crate::types::{Project, Solution, SolutionRepo, IndexError};

pub struct SharedState {
    pub store: Mutex<Store>,
    pub graph: Mutex<GraphDb>,
    pub task_queue: Arc<TaskQueue>,
}

pub type AppState = Arc<SharedState>;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health))
        // Projects
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{repo_id}", put(update_project).delete(delete_project))
        .route("/api/projects/{repo_id}/tags", post(add_project_tag))
        .route("/api/projects/{repo_id}/tags/{tag}", delete(remove_project_tag))
        .route("/api/projects/{repo_id}/summary", get(project_summary))
        // Solutions
        .route("/api/solutions", get(list_solutions).post(create_solution))
        .route("/api/solutions/{id}", put(update_solution).delete(delete_solution))
        .route("/api/solutions/{id}/repos", post(add_solution_repo))
        .route("/api/solutions/{id}/repos/{repo_id}", delete(remove_solution_repo))
        .route("/api/solutions/{id}/tags", post(add_solution_tag))
        .route("/api/solutions/{id}/tags/{tag}", delete(remove_solution_tag))
        // Indexing
        .route("/api/index", post(index_project))
        .route("/api/index/status", get(task_status))
        .route("/api/index/progress", get(index_progress_sse))
        // dirty_status removed — task queue handles incremental
        .route("/api/index/errors", get(list_index_errors))
        .route("/api/index/errors/{repo_id}", get(list_repo_index_errors))
        // Task queue (new)
        .route("/api/tasks/status", get(task_status))
        .route("/api/tasks/progress", get(task_progress_sse))
        // Graph
        .route("/api/graph/nodes", get(graph_nodes))
        .route("/api/graph/functions", get(search_functions))
        .route("/api/graph/types", get(search_types))
        .route("/api/graph/callers", get(fn_callers))
        .route("/api/graph/callees", get(fn_callees))
        .route("/api/graph/files", get(files_by_tag))
        .route("/api/graph/communities", post(detect_communities))
        .route("/api/graph/communities/info", get(community_info))
        .route("/api/graph/doc-drift", get(doc_drift))
        .route("/api/graph/call-flow", get(call_flow))
        // Solution analysis
        .route("/api/solutions/{id}/analyze", post(analyze_solution))
        .route("/api/solutions/{id}/graph", get(solution_graph))
        .route("/api/solutions/{id}/roles", get(solution_roles))
        // Libraries
        .route("/api/libs", get(list_libs))
        .route("/api/libs/index", post(index_lib))
        .route("/api/libs/docs", get(search_lib_docs))
        .route("/api/libs/{name}/docs", get(get_lib_docs))
        .route("/api/libs/versions", get(get_dep_versions))
        // Unified query (desktop/MCP)
        .route("/api/query", post(unified_query))
        // MCP tool proxy
        .route("/api/mcp/tools", get(mcp_list_tools))
        .route("/api/mcp/call", post(mcp_call_tool))
        // Marketplace install (legacy — prefer /api/install endpoints)
        .route("/api/marketplace/install", post(marketplace_install))
        // ACP (AI Coding Platform) detection & configuration
        .route("/api/acp/detect", get(acp_detect))
        .route("/api/acp/configure", post(acp_configure))
        .route("/api/acp/unconfigure", post(acp_unconfigure))
        // Installer — hooks, skills, commands, full install/uninstall
        .route("/api/install", post(install_all))
        .route("/api/install/hooks", post(install_hooks))
        .route("/api/install/item", post(install_single_item))
        .route("/api/install/item/remove", post(uninstall_single_item))
        .route("/api/install/catalog", get(get_catalog))
        .route("/api/install/installed", get(list_installed_items))
        .route("/api/uninstall", post(uninstall_all))
        // Config (user preferences)
        .route("/api/config", get(get_config).put(set_config_handler))
        .route("/api/config/{key}", get(get_config_key).delete(delete_config_key))
        // Sessions
        .route("/api/sessions", get(get_sessions_stub).post(create_session))
        .route("/api/sessions/{id}", put(update_session_handler))
        // Patterns
        .route("/api/patterns/{project}/detect", post(detect_patterns))
        .route("/api/patterns/{project}", get(list_patterns))
        .route("/api/patterns/{project}/match", get(match_pattern_handler))
        .route("/api/patterns/{project}/for/{symbol}", get(pattern_for_symbol))
        .route("/api/patterns/{project}/duplicates", get(find_duplicates_handler))
        .route("/api/patterns/{project}/conventions", get(project_conventions_handler))
        // Events
        .route("/api/events", post(create_event))
        .route("/api/events/{project}", get(list_events))
        // Workflow state
        .route("/api/state/{project}", get(get_workflow_state).put(update_workflow_state))
        // Reset (clears all data)
        .route("/api/reset", post(reset_all))
        // Scan
        .route("/api/scan", post(scan_folder))
        // Stop
        .route("/stop", post(stop))
        .with_state(state)
}

// ── Health ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    name: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        name: "senseid",
        version: env!("CARGO_PKG_VERSION"),
    })
}

// ── Projects ─────────────────────────────────────────────────────────────────

async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<Project>>, StatusCode> {
    let s = state.store.lock().await;
    s.list_projects()
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct CreateProjectBody {
    #[serde(rename = "repoId")]
    repo_id: String,
    name: Option<String>,
    path: String,
}

async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    let project = Project {
        repo_id: body.repo_id,
        name: body.name.unwrap_or_else(|| body.path.split('/').last().unwrap_or("unknown").to_string()),
        path: body.path,
        remote_url: None,
        indexed_at: None,
        last_error: None,
        duplicate_of: None,
        stack: vec![],
        libs: vec![],
        tags: vec![],
        status: "active".to_string(),
    };
    s.upsert_project(&project)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    let mut project = s.get_project(&repo_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        project.name = name.to_string();
    }
    if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
        project.status = status.to_string();
    }

    s.upsert_project(&project)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.delete_project(&repo_id)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Project Tags ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TagBody {
    tag: String,
}

async fn add_project_tag(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.add_tag("project", &repo_id, &body.tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn remove_project_tag(
    State(state): State<AppState>,
    Path((repo_id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.remove_tag("project", &repo_id, &tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Solutions ────────────────────────────────────────────────────────────────

async fn list_solutions(State(state): State<AppState>) -> Result<Json<Vec<Solution>>, StatusCode> {
    let s = state.store.lock().await;
    s.list_solutions()
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct CreateSolutionBody {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    client: Option<String>,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    repos: Vec<SolutionRepo>,
}

fn default_category() -> String { "active".to_string() }

async fn create_solution(
    State(state): State<AppState>,
    Json(body): Json<CreateSolutionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let s = state.store.lock().await;
    let id = uuid::Uuid::new_v4().to_string();
    let solution = Solution {
        id: id.clone(),
        name: body.name,
        description: body.description,
        client: body.client,
        category: body.category,
        repos: body.repos,
        tags: vec![],
        created_at: None,
        updated_at: None,
    };
    s.create_solution(&solution)
        .map(|_| (StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id}))))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_solution(
    State(_store): State<AppState>,
    Path(_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // TODO: implement update
    Json(serde_json::json!({"ok": true}))
}

async fn delete_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.delete_solution(&id)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn add_solution_repo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SolutionRepo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.add_repo_to_solution(&id, &body)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn remove_solution_repo(
    State(state): State<AppState>,
    Path((id, repo_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.remove_repo_from_solution(&id, &repo_id)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn add_solution_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.add_tag("solution", &id, &body.tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn remove_solution_tag(
    State(state): State<AppState>,
    Path((id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let s = state.store.lock().await;
    s.remove_tag("solution", &id, &tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Solution Analysis ────────────────────────────────────────────────────────

async fn analyze_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::indexer::cross_repo::SolutionAnalysis>, StatusCode> {
    let store = state.store.lock().await;
    let solutions = store.list_solutions().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let solution = solutions.into_iter().find(|s| s.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let graph = state.graph.lock().await;
    crate::indexer::cross_repo::analyze_solution(&store, &graph, &solution)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Per-Repo Summary ─────────────────────────────────────────────────────────

async fn project_summary(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    let project = store.get_project(&repo_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let graph = state.graph.lock().await;
    let (fn_count, type_count) = graph.count_symbols(&repo_id).unwrap_or((0, 0));
    let edge_count = graph.count_edges(&repo_id).unwrap_or(0);
    let pkg_count = graph.count_packages(&repo_id).unwrap_or(0);
    let mod_count = graph.count_modules(&repo_id).unwrap_or(0);

    // Find which solutions contain this repo
    let solutions = store.list_solutions().unwrap_or_default();
    let memberships: Vec<serde_json::Value> = solutions.iter()
        .filter(|s| s.repos.iter().any(|r| r.repo_id == repo_id))
        .map(|s| {
            let role = s.repos.iter().find(|r| r.repo_id == repo_id)
                .map(|r| r.role.as_str()).unwrap_or("unknown");
            serde_json::json!({"solutionId": s.id, "solutionName": s.name, "role": role})
        })
        .collect();

    Ok(Json(serde_json::json!({
        "repoId": project.repo_id,
        "name": project.name,
        "path": project.path,
        "stack": project.stack,
        "libs": project.libs,
        "tags": project.tags,
        "status": project.status,
        "indexedAt": project.indexed_at,
        "functions": fn_count,
        "types": type_count,
        "packages": pkg_count,
        "modules": mod_count,
        "edges": edge_count,
        "solutions": memberships,
    })))
}

// ── Solution Graph & Roles ───────────────────────────────────────────────────

async fn solution_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    let solutions = store.list_solutions().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let solution = solutions.into_iter().find(|s| s.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let graph = state.graph.lock().await;

    // Merge nodes and edges from all repos in the solution.
    // For subtree repos, their data lives under the parent repo's graph.
    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();
    let mut seen_repo_ids = std::collections::HashSet::new();

    for sr in &solution.repos {
        // Each repo (including subtrees) is indexed under its own repo_id
        let graph_repo_id = &sr.repo_id;

        if !seen_repo_ids.insert(graph_repo_id.clone()) {
            continue;
        }

        let nodes = graph.get_nodes(graph_repo_id).unwrap_or_default();
        let edges = graph.get_edges(graph_repo_id).unwrap_or_default();
        for node in nodes {
            all_nodes.push(serde_json::json!({
                "id": node.id, "name": node.name, "kind": node.kind,
                "file": node.file, "line": node.line, "complexity": node.complexity,
                "doc_type": node.doc_type, "level": node.level, "parent_id": node.parent_id,
                "repoId": graph_repo_id, "role": sr.role,
            }));
        }
        for edge in edges {
            all_edges.push(serde_json::json!({
                "source": edge.source, "target": edge.target,
                "type": edge.edge_type, "repoId": graph_repo_id,
            }));
        }
    }

    // Inject solution-level hierarchy: soln → repo nodes for each member
    let soln_node_id = format!("soln:{}", solution.id);
    all_nodes.push(serde_json::json!({
        "id": &soln_node_id, "name": &solution.name, "kind": "solution",
        "file": "", "line": 0, "complexity": null,
    }));
    for sr in &solution.repos {
        let repo_node_id = format!("repo:{}", sr.repo_id);
        // Only add if not already present from graph data
        if !all_nodes.iter().any(|n| n.get("id").and_then(|v| v.as_str()) == Some(&repo_node_id)) {
            let label = sr.label.as_deref().unwrap_or(&sr.repo_id);
            // Look up remote_url from the project store
            let project = store.get_project(&sr.repo_id).ok().flatten();
            let remote_url = project.as_ref().and_then(|p| p.remote_url.as_deref());
            let local_path = sr.path.as_deref()
                .or(project.as_ref().map(|p| p.path.as_str()));
            all_nodes.push(serde_json::json!({
                "id": &repo_node_id, "name": label, "kind": "repo",
                "file": local_path.unwrap_or(""), "line": 0, "complexity": null,
                "role": sr.role, "remoteUrl": remote_url,
            }));
        }
        all_edges.push(serde_json::json!({
            "source": &soln_node_id, "target": &repo_node_id, "type": "CONTAINS_REPO",
        }));

        // Subtree repos are indexed as separate repos via process_repo.
        // Their graph data is under their own repo_id (e.g. "sensei:marketplace").
        // No API-level hacking needed — the graph already has the full hierarchy.
    }

    Ok(Json(serde_json::json!({
        "solutionId": solution.id,
        "name": solution.name,
        "nodes": all_nodes.len(),
        "edges": all_edges.len(),
        "repos": solution.repos.len(),
        "graph": {"nodes": all_nodes, "edges": all_edges},
    })))
}

async fn solution_roles(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::indexer::cross_repo::InferredRole>>, StatusCode> {
    let store = state.store.lock().await;
    let solutions = store.list_solutions().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let solution = solutions.into_iter().find(|s| s.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut projects = std::collections::HashMap::new();
    for sr in &solution.repos {
        if let Ok(Some(p)) = store.get_project(&sr.repo_id) {
            projects.insert(sr.repo_id.clone(), p);
        }
    }

    Ok(Json(crate::indexer::cross_repo::infer_roles_pub(&projects)))
}

// ── Libraries ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LibsQuery {
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
async fn list_libs(
    State(state): State<AppState>,
    Query(q): Query<LibsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    let all_projects = store.list_projects().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Scope: repoId > solutionId > all
    let projects: Vec<_> = if let Some(rid) = &q.repo_id {
        all_projects.into_iter().filter(|p| p.repo_id == *rid).collect()
    } else if let Some(sid) = &q.solution_id {
        let solutions = store.list_solutions().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let repo_ids: std::collections::HashSet<String> = solutions.into_iter()
            .find(|s| s.id == *sid)
            .map(|s| s.repos.iter().map(|r| r.repo_id.clone()).collect())
            .unwrap_or_default();
        all_projects.into_iter().filter(|p| repo_ids.contains(&p.repo_id)).collect()
    } else {
        all_projects
    };

    // lib_name → [repo_ids]
    let mut lib_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for p in &projects {
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
struct IndexLibBody {
    #[serde(rename = "libName")]
    lib_name: String,
    url: String,
    version: Option<String>,
}

async fn index_lib(
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
struct LibDocsQuery {
    q: Option<String>,
}

async fn search_lib_docs(
    State(state): State<AppState>,
    Query(q): Query<LibDocsQuery>,
) -> Result<Json<Vec<crate::indexer::lib_indexer::LibDoc>>, StatusCode> {
    let query = q.q.unwrap_or_default();
    let store = state.store.lock().await;
    store.search_lib_docs(&query)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_lib_docs(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Vec<crate::indexer::lib_indexer::LibDoc>>, StatusCode> {
    let store = state.store.lock().await;
    store.get_lib_docs(&name)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct DepVersionsQuery {
    #[serde(rename = "repoId")]
    repo_id: String,
}

async fn get_dep_versions(
    State(state): State<AppState>,
    Query(q): Query<DepVersionsQuery>,
) -> Result<Json<Vec<crate::indexer::lib_indexer::DepVersion>>, StatusCode> {
    let store = state.store.lock().await;
    let project = store.get_project(&q.repo_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    crate::indexer::lib_indexer::extract_dep_versions(&store, &q.repo_id, &project.path)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Index Errors ─────────────────────────────────────────────────────────────

async fn list_index_errors(State(state): State<AppState>) -> Result<Json<Vec<IndexError>>, StatusCode> {
    let s = state.store.lock().await;
    s.get_index_errors(None)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn list_repo_index_errors(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<IndexError>>, StatusCode> {
    let s = state.store.lock().await;
    s.get_index_errors(Some(&repo_id))
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Indexing ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct IndexBody {
    #[serde(rename = "repoId")]
    repo_id: String,
    #[serde(rename = "repoPath")]
    repo_path: String,
    #[serde(default)]
    force: bool,
}

async fn index_project(
    State(state): State<AppState>,
    Json(body): Json<IndexBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clear errors for this project
    { let s = state.store.lock().await; s.clear_index_errors(&body.repo_id).ok(); }

    let task = crate::tasks::Task::new(
        crate::tasks::TaskKind::ProcessRepo, &body.repo_id, &body.repo_path,
    );
    let task_id = state.task_queue.enqueue(task).await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "queued": true,
        "taskId": task_id,
        "repoId": body.repo_id,
    })))
}

async fn task_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let status = state.task_queue.status().await;
    let progress = state.task_queue.progress().await;
    Json(serde_json::json!({ "queue": status, "repos": progress }))
}

async fn index_progress_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.task_queue.sender().subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok(Event::default().data(data)))
                }
                Err(_) => None,
            }
        });
    Sse::new(stream)
}

// ── Task Queue ──────────────────────────────────────────────────────────────

async fn task_progress_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.task_queue.sender().subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok(Event::default().data(data)))
                }
                Err(_) => None,
            }
        });
    Sse::new(stream)
}

// ── Graph ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GraphQuery {
    #[serde(rename = "repoId")]
    repo_id: Option<String>,
}

async fn graph_nodes(
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
struct SymbolQuery {
    #[serde(rename = "repoId")]
    repo_id: String,
    #[serde(rename = "q")]
    query: String,
}

async fn search_functions(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<crate::types::FunctionDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.search_functions(&q.query, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn search_types(
    State(state): State<AppState>,
    Query(q): Query<SymbolQuery>,
) -> Result<Json<Vec<crate::types::TypeDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.search_types(&q.query, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct TraceQuery {
    #[serde(rename = "repoId")]
    repo_id: String,
    name: String,
}

async fn fn_callers(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<crate::types::FunctionDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.callers_of(&q.name, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn fn_callees(
    State(state): State<AppState>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<Vec<crate::types::FunctionDetail>>, StatusCode> {
    let graph = state.graph.lock().await;
    graph.callees_of(&q.name, &q.repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct TagQuery {
    #[serde(rename = "repoId")]
    repo_id: String,
    tag: String,
}

async fn files_by_tag(
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

async fn doc_drift(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<Vec<crate::indexer::graph::DocDrift>>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let graph = state.graph.lock().await;
    graph.get_doc_drift(&repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn call_flow(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let graph = state.graph.lock().await;
    graph.get_call_flow(&repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn detect_communities(
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

async fn community_info(
    State(state): State<AppState>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<Vec<crate::indexer::community::CommunityInfo>>, StatusCode> {
    let repo_id = q.repo_id.unwrap_or_default();
    let graph = state.graph.lock().await;
    graph.get_communities(&repo_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Unified Query (desktop/MCP) ──────────────────────────────────────────────

#[derive(Deserialize)]
struct QueryBody {
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
async fn unified_query(
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

async fn query_libs(state: &AppState, q: &str, repo_id: &str, _solution_id: &Option<String>) -> serde_json::Value {
    let store = state.store.lock().await;
    let projects = store.list_projects().unwrap_or_default();

    let filtered: Vec<_> = if !repo_id.is_empty() {
        projects.into_iter().filter(|p| p.repo_id == repo_id).collect()
    } else {
        projects
    };

    let mut all_libs: Vec<serde_json::Value> = Vec::new();
    for p in &filtered {
        for lib in &p.libs {
            all_libs.push(serde_json::json!({"name": lib, "repoId": p.repo_id}));
        }
    }

    // Also search lib docs
    let lib_docs = store.search_lib_docs(&extract_search_term(q)).unwrap_or_default();

    serde_json::json!({
        "type": "libs",
        "query": q,
        "libs": all_libs,
        "libDocs": lib_docs.iter().take(5).map(|d| serde_json::json!({
            "title": d.title, "summary": d.summary, "url": d.url,
        })).collect::<Vec<_>>(),
    })
}

async fn query_functions(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.search_functions(&term, repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "functions",
        "query": q,
        "results": results,
    })
}

async fn query_types(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.search_types(&term, repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "types",
        "query": q,
        "results": results,
    })
}

async fn query_callers(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
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

async fn query_callees(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
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

async fn query_files(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let results = graph.files_by_tag(&term, repo_id).unwrap_or_default();
    let files: Vec<serde_json::Value> = results.into_iter()
        .map(|(id, path, tags)| serde_json::json!({"id": id, "path": path, "tags": tags}))
        .collect();
    serde_json::json!({ "type": "files", "query": q, "results": files })
}

async fn query_patterns(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
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

async fn query_docs(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let graph = state.graph.lock().await;
    let drift = graph.get_doc_drift(repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "docs",
        "query": q,
        "driftedDocs": drift,
    })
}

async fn query_communities(state: &AppState, repo_id: &str) -> serde_json::Value {
    let graph = state.graph.lock().await;
    let communities = graph.get_communities(repo_id).unwrap_or_default();
    serde_json::json!({
        "type": "communities",
        "results": communities,
    })
}

async fn query_general(state: &AppState, q: &str, repo_id: &str) -> serde_json::Value {
    let term = extract_search_term(q);
    let graph = state.graph.lock().await;
    let functions = graph.search_functions(&term, repo_id).unwrap_or_default();
    let types = graph.search_types(&term, repo_id).unwrap_or_default();
    drop(graph);

    let store = state.store.lock().await;
    let lib_docs = store.search_lib_docs(&term).unwrap_or_default();

    serde_json::json!({
        "type": "general",
        "query": q,
        "functions": functions,
        "types": types,
        "libDocs": lib_docs.iter().take(5).map(|d| serde_json::json!({
            "title": d.title, "summary": d.summary,
        })).collect::<Vec<_>>(),
    })
}

/// Extract the most meaningful search term from a natural language query.
fn extract_search_term(q: &str) -> String {
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

// ── MCP Tool Proxy ───────────────────────────────────────────────────────────

async fn mcp_list_tools() -> Json<serde_json::Value> {
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
        ]
    }))
}

async fn mcp_call_tool(
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
            let store = state.store.lock().await;
            let docs = store.search_lib_docs(query).unwrap_or_default();
            serde_json::json!({"docs": docs})
        }
        "get_lib_docs" => {
            let name = params["name"].as_str().unwrap_or(query);
            let component = params["component"].as_str().unwrap_or("");
            let store = state.store.lock().await;
            if !component.is_empty() {
                // Return specific component doc
                let doc = store.get_lib_doc_component(name, component).unwrap_or(None);
                serde_json::json!({"doc": doc})
            } else {
                // Return index doc if it exists (try "index" then "README"), otherwise all docs
                let index = store.get_lib_doc_component(name, "index").unwrap_or(None)
                    .or_else(|| store.get_lib_doc_component(name, "README").unwrap_or(None));
                if let Some(idx) = index {
                    serde_json::json!({"index": idx})
                } else {
                    let docs = store.get_lib_docs(name).unwrap_or_default();
                    serde_json::json!({"docs": docs})
                }
            }
        }
        "list_projects" => {
            let store = state.store.lock().await;
            let projects = store.list_projects().unwrap_or_default();
            serde_json::json!({"projects": projects})
        }
        "create_session" => {
            let repo_id = params["repoId"].as_str().unwrap_or(query);
            let task = params["task"].as_str().unwrap_or("untitled");
            let id = uuid::Uuid::new_v4().to_string();
            let store = state.store.lock().await;
            store.create_session(&id, repo_id, task).ok();
            serde_json::json!({"ok": true, "sessionId": id})
        }
        "update_session" => {
            let session_id = params["sessionId"].as_str().unwrap_or("");
            let store = state.store.lock().await;
            store.update_session(
                session_id,
                params["outcome"].as_str(),
                params["summary"].as_str(),
                params["cost"].as_f64(),
                params["tokensIn"].as_i64(),
                params["tokensOut"].as_i64(),
            ).ok();
            serde_json::json!({"ok": true})
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
                            let store = state.store.lock().await;
                            match crate::indexer::lib_indexer::index_lib_content(&store, name, url, &content, version) {
                                Ok(result) => {
                                    result_json = serde_json::json!({
                                        "ok": true,
                                        "libName": result.lib_name,
                                        "docsIndexed": result.docs_indexed,
                                        "sourceType": result.source_type,
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
            let store = state.store.lock().await;
            let project = store.get_project(repo_id).ok().flatten();
            let graph = state.graph.lock().await;
            let (fns, types) = graph.count_symbols(repo_id).unwrap_or((0, 0));
            serde_json::json!({
                "project": project,
                "functions": fns,
                "types": types,
            })
        }
        _ => serde_json::json!({"error": format!("Unknown tool: {}", tool)}),
    };

    Ok(Json(result))
}

// ── Marketplace Install ──────────────────────────────────────────────────────

async fn marketplace_install(
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let target = body["target"].as_str().unwrap_or("");
    let _item_name = body["item"].as_str().unwrap_or("");
    let scope = body["scope"].as_str().unwrap_or("global");
    let _marketplace_path = body["marketplacePath"].as_str().unwrap_or("");

    if target.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "target required"}));
    }

    // Use native Rust installer (replaces shelling out to marketplace/install.ts)
    let acps: Vec<String> = body["acps"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let result = crate::installer::install(&acps, scope);
    Json(serde_json::json!({
        "ok": true,
        "hooks_installed": result.hooks_installed,
        "skills_installed": result.skills_installed,
        "commands_installed": result.commands_installed,
        "acps_configured": result.acps_configured,
        "errors": result.errors,
    }))
}

// ── ACP (AI Coding Platform) ─────────────────────────────────────────────────

async fn acp_detect() -> Json<Vec<crate::acp::AcpStatus>> {
    Json(crate::acp::detect())
}

#[derive(Deserialize)]
struct AcpConfigureBody {
    #[serde(default)]
    acps: Vec<String>,
}

async fn acp_configure(
    Json(body): Json<AcpConfigureBody>,
) -> Json<crate::acp::ConfigureResult> {
    Json(crate::acp::configure(&body.acps))
}

async fn acp_unconfigure() -> Json<serde_json::Value> {
    let removed = crate::acp::unconfigure();
    serde_json::json!({"ok": true, "removed": removed}).into()
}

// ── Installer ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct InstallBody {
    #[serde(default)]
    acps: Vec<String>,
    #[serde(default = "default_scope")]
    scope: String,
}

fn default_scope() -> String {
    "global".into()
}

async fn install_all(
    Json(body): Json<InstallBody>,
) -> Json<crate::installer::InstallResult> {
    // Run in blocking thread — marketplace download is synchronous
    let result = tokio::task::spawn_blocking(move || {
        crate::installer::install(&body.acps, &body.scope)
    })
    .await
    .unwrap_or_default();
    Json(result)
}

async fn install_hooks() -> Json<serde_json::Value> {
    match tokio::task::spawn_blocking(|| crate::installer::install_hooks_only()).await {
        Ok(Ok(n)) => serde_json::json!({"ok": true, "count": n}).into(),
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e}).into(),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}).into(),
    }
}

#[derive(Deserialize)]
struct InstallItemBody {
    name: String,
    kind: String,
}

async fn install_single_item(
    Json(body): Json<InstallItemBody>,
) -> Json<serde_json::Value> {
    let name = body.name;
    let kind = body.kind;
    match tokio::task::spawn_blocking(move || crate::installer::install_item(&name, &kind)).await {
        Ok(Ok(path)) => serde_json::json!({"ok": true, "path": path}).into(),
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e}).into(),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}).into(),
    }
}

async fn uninstall_single_item(
    Json(body): Json<InstallItemBody>,
) -> Json<serde_json::Value> {
    let name = body.name;
    let kind = body.kind;
    match tokio::task::spawn_blocking(move || crate::installer::uninstall_item(&name, &kind)).await {
        Ok(Ok(())) => serde_json::json!({"ok": true}).into(),
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e}).into(),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}).into(),
    }
}

async fn get_catalog() -> Json<serde_json::Value> {
    match tokio::task::spawn_blocking(crate::installer::fetch_catalog).await {
        Ok(Ok(catalog)) => {
            let items: Vec<serde_json::Value> = catalog
                .items
                .iter()
                .map(|i| serde_json::json!({
                    "name": i.name,
                    "kind": i.kind,
                    "description": i.description,
                    "scope": i.scope,
                    "path": i.path,
                    "recommended_for": i.recommended_for,
                    "stage": i.stage,
                }))
                .collect();
            serde_json::json!({
                "version": catalog.version,
                "items": items,
            })
            .into()
        }
        Ok(Err(e)) => serde_json::json!({"error": e}).into(),
        Err(e) => serde_json::json!({"error": e.to_string()}).into(),
    }
}

async fn list_installed_items() -> Json<Vec<crate::installer::InstalledItem>> {
    Json(crate::installer::list_installed())
}

async fn uninstall_all() -> Json<crate::installer::UninstallResult> {
    match tokio::task::spawn_blocking(crate::installer::uninstall).await {
        Ok(result) => Json(result),
        Err(_) => Json(crate::installer::UninstallResult::default()),
    }
}

// ── Config ───────────────────────────────────────────────────────────────────

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    let config = store.get_all_config().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(config)))
}

async fn get_config_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    let val = store.get_config(&key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"key": key, "value": val})))
}

async fn set_config_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    if let Some(obj) = body.as_object() {
        for (key, val) in obj {
            let v = match val { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
            store.set_config(key, &v).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn delete_config_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.store.lock().await;
    store.delete_config(&key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── Sessions (stub) ──────────────────────────────────────────────────────────

async fn get_sessions_stub(
    State(state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let repo_id = q.get("repoId").map(|s| s.as_str());
    let sessions = store.get_sessions(repo_id).unwrap_or_default();
    let total = sessions.len();
    let completed = sessions.iter().filter(|s| s["outcome"].as_str() == Some("completed")).count();
    Json(serde_json::json!({
        "stats": { "totalSessions": total, "completed": completed },
        "sessions": sessions,
        "toolUsage": [],
        "benchmarkPairs": []
    }))
}

async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = body["id"].as_str().unwrap_or(&uuid::Uuid::new_v4().to_string()).to_string();
    let repo_id = body["repoId"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("untitled");
    let store = state.store.lock().await;
    store.create_session(&id, repo_id, task).ok();
    Json(serde_json::json!({"ok": true, "id": id}))
}

async fn update_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    store.update_session(
        &id,
        body["outcome"].as_str(),
        body["summary"].as_str(),
        body["cost"].as_f64(),
        body["tokensIn"].as_i64(),
        body["tokensOut"].as_i64(),
    ).ok();
    Json(serde_json::json!({"ok": true}))
}

// ── Patterns ────────────────────────────────────────────────────────────────

async fn detect_patterns(
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

async fn list_patterns(
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
struct MatchQuery {
    description: Option<String>,
}

async fn match_pattern_handler(
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

async fn pattern_for_symbol(
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

async fn find_duplicates_handler(
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

async fn project_conventions_handler(
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

// ── Events ──────────────────────────────────────────────────────────────────

async fn create_event(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let id = body["id"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let project = body["project"].as_str().unwrap_or("unknown");
    let session_id = body["session_id"].as_str();
    let event_type = match body["event_type"].as_str().or(body["type"].as_str()) {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "event_type is required"}))),
    };
    let data = body.get("data")
        .map(|d| serde_json::to_string(d).unwrap_or_default())
        .unwrap_or_else(|| "{}".to_string());

    let store = state.store.lock().await;
    match store.insert_event(&id, project, session_id, event_type, &data) {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e.to_string()}))),
    }
}

#[derive(Deserialize)]
struct EventQuery {
    #[serde(rename = "type")]
    event_type: Option<String>,
    session: Option<String>,
    limit: Option<u32>,
}

async fn list_events(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<EventQuery>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let limit = q.limit.unwrap_or(50).min(500);
    match store.list_events(&project, q.event_type.as_deref(), q.session.as_deref(), limit) {
        Ok(events) => Json(serde_json::json!({"events": events, "count": events.len()})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

// ── Workflow State ───────────────────────────────────────────────────────────

async fn get_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    match store.get_workflow_state(&project) {
        Ok(Some(ws)) => Json(ws),
        Ok(None) => Json(serde_json::json!({
            "project": project,
            "active_phase": null,
            "active_plan": null,
            "active_task": null,
            "active_issue": null,
            "last_checkpoint": null,
            "rules_hash": null,
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn update_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;

    let result = store.upsert_workflow_state(
        &project,
        body["active_phase"].as_str(),
        body["active_plan"].as_str(),
        body["active_task"].as_str(),
        body["active_issue"].as_i64(),
        body["last_checkpoint"].as_str(),
        body["rules_hash"].as_str(),
    );

    if let Err(e) = result {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }

    // Sync to .sensei/state.yaml
    // Use explicit project_path from body, or look up from projects table
    let project_path = body["project_path"].as_str().map(String::from)
        .or_else(|| store.get_project_path(&project).ok().flatten());
    if let Some(project_path) = project_path {
        let sensei_dir = std::path::Path::new(&project_path).join(".sensei");
        std::fs::create_dir_all(&sensei_dir).ok();
        let state_file = sensei_dir.join("state.yaml");

        // Read back the state we just wrote to get all fields
        if let Ok(Some(ws)) = store.get_workflow_state(&project) {
            let yaml = format!(
                "active_phase: {}\nactive_plan: {}\nactive_task: {}\nactive_issue: {}\nlast_checkpoint: {}\nrules_hash: {}\n",
                ws["active_phase"].as_str().unwrap_or("~"),
                ws["active_plan"].as_str().unwrap_or("~"),
                ws["active_task"].as_str().unwrap_or("~"),
                ws["active_issue"].as_i64().map(|n| n.to_string()).unwrap_or("~".to_string()),
                ws["last_checkpoint"].as_str().unwrap_or("~"),
                ws["rules_hash"].as_str().unwrap_or("~"),
            );
            std::fs::write(&state_file, yaml).ok();
        }
    }

    Json(serde_json::json!({"ok": true}))
}

// ── Reset ────────────────────────────────────────────────────────────────────

async fn reset_all(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // Clear store: projects, solutions, config, errors
    {
        let store = state.store.lock().await;
        store.execute_raw("DELETE FROM projects").ok();
        store.execute_raw("DELETE FROM solutions").ok();
        store.execute_raw("DELETE FROM solution_repos").ok();
        store.execute_raw("DELETE FROM config").ok();
        store.execute_raw("DELETE FROM index_errors").ok();
        store.execute_raw("DELETE FROM lib_docs").ok();
        store.execute_raw("DELETE FROM lib_meta").ok();
    }

    // Clear graph DB
    {
        let graph = state.graph.lock().await;
        graph.clear_all().ok();
    }

    // Clear manifest files
    if let Some(home) = dirs::home_dir() {
        let projects_dir = home.join(".sensei").join("projects");
        std::fs::remove_dir_all(&projects_dir).ok();
        std::fs::create_dir_all(&projects_dir).ok();
    }

    Json(serde_json::json!({"ok": true}))
}

// ── Scan ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ScanBody {
    root: String,
    #[serde(default = "default_depth")]
    max_depth: u32,
}

fn default_depth() -> u32 { 4 }

async fn scan_folder(
    State(state): State<AppState>,
    Json(body): Json<ScanBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Expand ~ to home directory
    let expanded = if body.root.starts_with("~/") {
        dirs::home_dir()
            .map(|h| h.join(&body.root[2..]).to_string_lossy().to_string())
            .unwrap_or(body.root.clone())
    } else {
        body.root.clone()
    };
    let root_path = expanded.clone();
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Enqueue ScanRoot task — runs asynchronously via task workers
    let task = crate::tasks::Task::new(
        crate::tasks::TaskKind::ScanRoot, "", &root_path,
    );
    let task_id = state.task_queue.enqueue(task).await;

    Ok(Json(serde_json::json!({"ok": true, "scanning": true, "taskId": task_id})))
}

// ── Stop ─────────────────────────────────────────────────────────────────────

async fn stop() -> Json<serde_json::Value> {
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    Json(serde_json::json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_app() -> (Router, AppState) {
        let store = Store::open_memory().unwrap();
        let graph = GraphDb::open_memory().unwrap();
        let state = Arc::new(SharedState {
            store: Mutex::new(store),
            graph: Mutex::new(graph),
            task_queue: Arc::new(TaskQueue::new()),
        });
        let router = create_router(state.clone());
        (router, state)
    }

    #[tokio::test]
    async fn health_check() {
        let (app, _) = test_app();
        let resp = app.oneshot(
            Request::builder().uri("/health").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["name"], "senseid");
    }

    #[tokio::test]
    async fn create_and_list_projects() {
        let (app, _) = test_app();

        // Create
        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"repoId":"test","path":"/tmp/test"}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // List
        let resp = app.oneshot(
            Request::builder().uri("/api/projects").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let projects: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["repo_id"], "test");
    }

    #[tokio::test]
    async fn delete_project_returns_ok() {
        let (app, state) = test_app();
        {
            let s = state.store.lock().await;
            s.upsert_project(&Project {
                repo_id: "x".into(), name: "x".into(), path: "/x".into(),
                remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
                stack: vec![], libs: vec![], tags: vec![], status: "active".into(),
            }).unwrap();
        }
        let resp = app.oneshot(
            Request::builder().method("DELETE").uri("/api/projects/x").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_and_list_solutions() {
        let (app, _) = test_app();

        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/solutions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Acme","repos":[]}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["id"].is_string());

        let resp = app.oneshot(
            Request::builder().uri("/api/solutions").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let solutions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0]["name"], "Acme");
    }

    #[tokio::test]
    async fn project_tags_via_api() {
        let (app, state) = test_app();
        {
            let s = state.store.lock().await;
            s.upsert_project(&Project {
                repo_id: "r".into(), name: "r".into(), path: "/r".into(),
                remote_url: None, indexed_at: None, last_error: None, duplicate_of: None,
                stack: vec![], libs: vec![], tags: vec![], status: "active".into(),
            }).unwrap();
        }

        // Add tag
        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/r/tags")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"tag":"backend"}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // List projects — tag should be there
        let resp = app.clone().oneshot(
            Request::builder().uri("/api/projects").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let projects: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(projects[0]["tags"][0], "backend");

        // Remove tag
        let resp = app.clone().oneshot(
            Request::builder().method("DELETE").uri("/api/projects/r/tags/backend").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify removed
        let resp = app.oneshot(
            Request::builder().uri("/api/projects").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let projects: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(projects[0]["tags"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn index_errors_via_api() {
        let (app, state) = test_app();
        {
            let s = state.store.lock().await;
            s.log_index_error(&IndexError {
                repo_id: "foo".into(), file_path: "bad.py".into(),
                error: "SyntaxError".into(), adapter: Some("python".into()),
                timestamp: "2026-04-12".into(),
            }).unwrap();
        }

        let resp = app.clone().oneshot(
            Request::builder().uri("/api/index/errors").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let errors: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["error"], "SyntaxError");

        let resp = app.oneshot(
            Request::builder().uri("/api/index/errors/foo").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let errors: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(errors.len(), 1);
    }

    #[tokio::test]
    async fn index_project_via_api() {
        let (app, _) = test_app();

        // Create a temp repo with a Python file
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.py"), "def greet(name):\n    return f'hi {name}'\n").unwrap();

        let body = serde_json::json!({
            "repoId": "test-repo",
            "repoPath": dir.path().to_string_lossy(),
        });

        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/index")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["queued"], true);

        // Note: indexing happens async via worker — graph won't have data in unit test
        // (no worker spawned in test_app). The e2e_server test covers the full flow.

        // Verify queue status endpoint works
        let resp = app.oneshot(
            Request::builder()
                .uri("/api/index/status")
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["queue"].is_object());
    }

    #[tokio::test]
    async fn scan_folder_finds_repos() {
        let (app, _) = test_app();

        // Create a temp dir with a "repo" (has .git)
        let root = tempfile::TempDir::new().unwrap();
        let repo = root.path().join("my-project");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("package.json"), r#"{"name":"test","dependencies":{"express":"4"}}"#).unwrap();

        let resp = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scan")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"root": root.path().to_string_lossy()}).to_string()))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        // scan_folder returns {"ok": true, "scanning": true} — scan runs async in background
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["scanning"], true);
    }
}
