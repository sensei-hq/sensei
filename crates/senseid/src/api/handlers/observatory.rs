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
    let project_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Look up the folder by name (old string repo_id)
    let folder = state.pg.get_repo_by_name(&body.repo_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id = folder["id"].as_str()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    state.pg.set_folder_project(&folder_id, &project_id, &body.role, body.label.as_deref()).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_solution_repo(
    State(state): State<AppState>,
    Path((_id, repo_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clear folder-project association by setting props to remove project link
    let folder = state.pg.get_repo_by_name(&repo_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id = folder["id"].as_str()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clear the project association by setting project_id to null via props
    state.pg.set_folder_props(&folder_id, &serde_json::json!({"project_id": null})).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn add_solution_tag(
    State(state): State<AppState>,
    Path(_id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary. Register in vocabulary.
    state.pg.add_tag(&body.tag, Some("solution")).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_solution_tag(
    State(state): State<AppState>,
    Path((_id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary. Remove from vocabulary.
    state.pg.remove_tag(&tag).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Solution Analysis ───────────────────────────────────────────────────────

pub(crate) async fn analyze_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let project = state.pg.get_project(&project_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Return project data; full cross-repo analysis requires migration of analyze_project to PgStore
    Ok(Json(serde_json::json!({
        "project": project,
        "links": [],
        "shared_libs": [],
        "note": "Cross-repo analysis pending migration to PgStore"
    })))
}

// ── Per-Repo Summary ────────────────────────────────────────────────────────

pub(crate) async fn project_summary(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let folder = state.pg.get_repo_by_name(&repo_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let graph = state.graph.lock().await;
    let (fn_count, type_count) = graph.count_symbols(&repo_id).unwrap_or((0, 0));
    let edge_count = graph.count_edges(&repo_id).unwrap_or(0);
    let pkg_count = graph.count_packages(&repo_id).unwrap_or(0);
    let mod_count = graph.count_modules(&repo_id).unwrap_or(0);

    Ok(Json(serde_json::json!({
        "repoId": folder["name"],
        "name": folder["name"],
        "path": folder["abs_path"],
        "stack": folder.get("stack").unwrap_or(&serde_json::json!([])),
        "libs": folder.get("libs").unwrap_or(&serde_json::json!([])),
        "tags": folder.get("tags").unwrap_or(&serde_json::json!([])),
        "status": folder.get("status").unwrap_or(&serde_json::json!("active")),
        "indexedAt": folder.get("indexed_at"),
        "functions": fn_count,
        "types": type_count,
        "packages": pkg_count,
        "modules": mod_count,
        "edges": edge_count,
        "solutions": [],
    })))
}

// ── Solution Graph & Roles ──────────────────────────────────────────────────

pub(crate) async fn solution_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let project = state.pg.get_project(&project_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let project_name = project["name"].as_str().unwrap_or("unknown");

    // Get all repos (folders) and filter those belonging to this project
    let all_repos = state.pg.list_repositories().await.unwrap_or_default();
    let project_repos: Vec<&serde_json::Value> = all_repos.iter()
        .filter(|r| r["project_id"].as_str() == Some(&id))
        .collect();

    let graph = state.graph.lock().await;

    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();
    let mut seen_repo_ids = std::collections::HashSet::new();

    for repo in &project_repos {
        let repo_name = repo["name"].as_str().unwrap_or("");
        if !seen_repo_ids.insert(repo_name.to_string()) {
            continue;
        }

        let nodes = graph.get_nodes(repo_name).unwrap_or_default();
        let edges = graph.get_edges(repo_name).unwrap_or_default();
        let role = repo["role"].as_str().unwrap_or("unknown");
        for node in nodes {
            all_nodes.push(serde_json::json!({
                "id": node.id, "name": node.name, "kind": node.kind,
                "file": node.file, "line": node.line, "complexity": node.complexity,
                "doc_type": node.doc_type, "level": node.level, "parent_id": node.parent_id,
                "repoId": repo_name, "role": role,
            }));
        }
        for edge in edges {
            all_edges.push(serde_json::json!({
                "source": edge.source, "target": edge.target,
                "type": edge.edge_type, "repoId": repo_name,
            }));
        }
    }

    // Inject project-level hierarchy: soln -> repo nodes for each member
    let soln_node_id = format!("soln:{}", id);
    all_nodes.push(serde_json::json!({
        "id": &soln_node_id, "name": project_name, "kind": "solution",
        "file": "", "line": 0, "complexity": null,
    }));
    for repo in &project_repos {
        let repo_name = repo["name"].as_str().unwrap_or("");
        let repo_node_id = format!("repo:{}", repo_name);
        if !all_nodes.iter().any(|n| n.get("id").and_then(|v| v.as_str()) == Some(&repo_node_id)) {
            let label = repo["label"].as_str().unwrap_or(repo_name);
            let abs_path = repo["abs_path"].as_str().unwrap_or("");
            let role = repo["role"].as_str().unwrap_or("unknown");
            all_nodes.push(serde_json::json!({
                "id": &repo_node_id, "name": label, "kind": "repo",
                "file": abs_path, "line": 0, "complexity": null,
                "role": role,
            }));
        }
        all_edges.push(serde_json::json!({
            "source": &soln_node_id, "target": &repo_node_id, "type": "CONTAINS_REPO",
        }));
    }

    Ok(Json(serde_json::json!({
        "solutionId": id,
        "name": project_name,
        "nodes": all_nodes.len(),
        "edges": all_edges.len(),
        "repos": project_repos.len(),
        "graph": {"nodes": all_nodes, "edges": all_edges},
    })))
}

pub(crate) async fn solution_roles(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    // Verify project exists
    state.pg.get_project(&project_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get repos belonging to this project
    let all_repos = state.pg.list_repositories().await.unwrap_or_default();
    let project_repos: Vec<serde_json::Value> = all_repos.into_iter()
        .filter(|r| r["project_id"].as_str() == Some(&id))
        .collect();

    // Build simple role list from folder data
    let roles: Vec<serde_json::Value> = project_repos.iter().map(|r| {
        serde_json::json!({
            "repoId": r["name"],
            "role": r.get("role").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "label": r.get("label").and_then(|v| v.as_str()),
        })
    }).collect();

    Ok(Json(serde_json::json!(roles)))
}

// ── Metrics ─────────────────────────────────────────────────────────────────

pub(crate) async fn get_metrics(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    // Metrics are computed from session data in PgStore.
    // Look up folder to get its UUID, then query sessions.
    let folder = state.pg.get_repo_by_name(&project).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            let sessions = state.pg.list_sessions_by_folder(&folder_id, 100).await.unwrap_or_default();
            let session_count = sessions.len();
            let completed = sessions.iter().filter(|s| s["outcome"].as_str() == Some("completed")).count();
            return Json(serde_json::json!({
                "project": project,
                "sessions": session_count,
                "completed": completed,
                "ftr": if session_count > 0 { completed as f64 / session_count as f64 } else { 0.0 },
            }));
        }
    }
    Json(serde_json::json!({"error": "project not found"}))
}
