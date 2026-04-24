use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

// Re-use TagBody from workspace
use super::workspace::TagBody;

// ── Solutions CRUD ──────────────────────────────────────────────────────────

pub(crate) async fn list_solutions(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let projects = state.pg.list_projects().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO: Old Store enriched each project with its repos via list_repos() + project_id matching.
    // PgStore projects don't have a direct repo-membership view yet. Return projects without repo enrichment.
    // Add a list_folders_by_project() method to PgStore when needed.
    Ok(Json(serde_json::json!(projects)))
}

#[derive(Deserialize)]
pub(crate) struct CreateSolutionBody {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    client: Option<String>,
    // TODO: category and repos unused until PgStore supports maturity + repo membership
    #[serde(default = "default_category")]
    #[allow(dead_code)]
    category: String,
    #[serde(default)]
    #[allow(dead_code)]
    repos: Vec<CreateProjectRepo>,
}

#[derive(Deserialize)]
pub(crate) struct CreateProjectRepo {
    repo_id: String,
    #[serde(default = "default_role")]
    role: String,
    label: Option<String>,
}

fn default_category() -> String { "active".to_string() }
fn default_role() -> String { "unknown".to_string() }

pub(crate) async fn create_solution(
    State(state): State<AppState>,
    Json(body): Json<CreateSolutionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let id = state.pg.create_project(
        &body.name,
        body.description.as_deref(),
        body.client.as_deref(),
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO: PgStore has no set_repo_project — repo-to-project membership not yet modeled.
    // Old Store assigned repos to this project via set_repo_project. Skipping for now.
    // for r in &body.repos { ... }

    Ok((StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id}))))
}

pub(crate) async fn update_solution(
    State(_store): State<AppState>,
    Path(_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // TODO: implement update
    Json(serde_json::json!({"ok": true}))
}

pub(crate) async fn delete_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.delete_project(&project_id).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn add_solution_repo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateProjectRepo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no set_repo_project — repo-to-project membership not yet modeled.
    // Keep old store as bridge.
    let s = state.store.lock().await;
    s.set_repo_project(&body.repo_id, &id, &body.role, body.label.as_deref())
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_solution_repo(
    State(state): State<AppState>,
    Path((_id, repo_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no clear_repo_project. Keep old store as bridge.
    let s = state.store.lock().await;
    s.clear_repo_project(&repo_id)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn add_solution_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore tags are a controlled vocabulary, not per-entity. Keep old store as bridge.
    let s = state.store.lock().await;
    s.add_tag("solution", &id, &body.tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_solution_tag(
    State(state): State<AppState>,
    Path((id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore tags are a controlled vocabulary, not per-entity. Keep old store as bridge.
    let s = state.store.lock().await;
    s.remove_tag("solution", &id, &tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Solution Analysis ───────────────────────────────────────────────────────

pub(crate) async fn analyze_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::indexer::cross_repo::ProjectAnalysis>, StatusCode> {
    // TODO: cross_repo::analyze_project takes &Store + &GraphDb + &Project (old types).
    // Keep old store as bridge until analyze_project is migrated to PgStore.
    let store = state.store.lock().await;
    let projects = store.list_projects().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let project = projects.into_iter().find(|s| s.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let graph = state.graph.lock().await;
    crate::indexer::cross_repo::analyze_project(&store, &graph, &project)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Per-Repo Summary ────────────────────────────────────────────────────────

pub(crate) async fn project_summary(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no get_repo (repos are now folders). Keep old store as bridge
    // until project_summary is reworked for folder-based model.
    let store = state.store.lock().await;
    let repo = store.get_repo(&repo_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let graph = state.graph.lock().await;
    let (fn_count, type_count) = graph.count_symbols(&repo_id).unwrap_or((0, 0));
    let edge_count = graph.count_edges(&repo_id).unwrap_or(0);
    let pkg_count = graph.count_packages(&repo_id).unwrap_or(0);
    let mod_count = graph.count_modules(&repo_id).unwrap_or(0);

    // Find which project this repo belongs to
    let memberships: Vec<serde_json::Value> = if let Some(pid) = &repo.project_id {
        let projects = store.list_projects().unwrap_or_default();
        projects.iter()
            .filter(|p| p.id == *pid)
            .map(|p| serde_json::json!({"projectId": p.id, "projectName": p.name, "role": repo.role}))
            .collect()
    } else {
        vec![]
    };

    Ok(Json(serde_json::json!({
        "repoId": repo.repo_id,
        "name": repo.name,
        "path": repo.path,
        "stack": repo.stack,
        "libs": repo.libs,
        "tags": repo.tags,
        "status": repo.status,
        "indexedAt": repo.indexed_at,
        "functions": fn_count,
        "types": type_count,
        "packages": pkg_count,
        "modules": mod_count,
        "edges": edge_count,
        "solutions": memberships,
    })))
}

// ── Solution Graph & Roles ──────────────────────────────────────────────────

pub(crate) async fn solution_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Uses old store for list_projects (typed) + get_project_repos. Keep as bridge
    // until graph queries are migrated to PgStore nodes/edges.
    let store = state.store.lock().await;
    let projects = store.list_projects().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let project = projects.into_iter().find(|s| s.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let project_repos = store.get_project_repos(&project.id).unwrap_or_default();
    let graph = state.graph.lock().await;

    // Merge nodes and edges from all repos in the project.
    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();
    let mut seen_repo_ids = std::collections::HashSet::new();

    for repo in &project_repos {
        if !seen_repo_ids.insert(repo.repo_id.clone()) {
            continue;
        }

        let nodes = graph.get_nodes(&repo.repo_id).unwrap_or_default();
        let edges = graph.get_edges(&repo.repo_id).unwrap_or_default();
        for node in nodes {
            all_nodes.push(serde_json::json!({
                "id": node.id, "name": node.name, "kind": node.kind,
                "file": node.file, "line": node.line, "complexity": node.complexity,
                "doc_type": node.doc_type, "level": node.level, "parent_id": node.parent_id,
                "repoId": repo.repo_id, "role": repo.role,
            }));
        }
        for edge in edges {
            all_edges.push(serde_json::json!({
                "source": edge.source, "target": edge.target,
                "type": edge.edge_type, "repoId": repo.repo_id,
            }));
        }
    }

    // Inject project-level hierarchy: soln → repo nodes for each member
    let soln_node_id = format!("soln:{}", project.id);
    all_nodes.push(serde_json::json!({
        "id": &soln_node_id, "name": &project.name, "kind": "solution",
        "file": "", "line": 0, "complexity": null,
    }));
    for sr in &project_repos {
        let repo_node_id = format!("repo:{}", sr.repo_id);
        // Only add if not already present from graph data
        if !all_nodes.iter().any(|n| n.get("id").and_then(|v| v.as_str()) == Some(&repo_node_id)) {
            let label = sr.label.as_deref().unwrap_or(&sr.repo_id);
            let remote_url = sr.remote_url.as_deref();
            let local_path = Some(sr.path.as_str());
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
        "solutionId": project.id,
        "name": project.name,
        "nodes": all_nodes.len(),
        "edges": all_edges.len(),
        "repos": project_repos.len(),
        "graph": {"nodes": all_nodes, "edges": all_edges},
    })))
}

pub(crate) async fn solution_roles(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::indexer::cross_repo::InferredRole>>, StatusCode> {
    // TODO: Uses old store for list_projects (typed) + get_project_repos. Keep as bridge.
    let store = state.store.lock().await;
    let projects = store.list_projects().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _project = projects.into_iter().find(|s| s.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let project_repos = store.get_project_repos(&id).unwrap_or_default();
    let mut repos_map = std::collections::HashMap::new();
    for r in &project_repos {
        repos_map.insert(r.repo_id.clone(), r.clone());
    }

    Ok(Json(crate::indexer::cross_repo::infer_roles_pub(&repos_map)))
}

// ── Metrics ─────────────────────────────────────────────────────────────────

pub(crate) async fn get_metrics(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: PgStore has no compute_metrics. Keep old store as bridge.
    let store = state.store.lock().await;
    match store.compute_metrics(&project) {
        Ok(metrics) => Json(metrics),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
