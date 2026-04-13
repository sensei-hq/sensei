use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::db::Store;
use crate::indexer::graph::GraphDb;
use crate::types::{Project, Solution, SolutionRepo, IndexError, IndexResult};

pub struct SharedState {
    pub store: Mutex<Store>,
    pub graph: Mutex<GraphDb>,
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
        // Solutions
        .route("/api/solutions", get(list_solutions).post(create_solution))
        .route("/api/solutions/{id}", put(update_solution).delete(delete_solution))
        .route("/api/solutions/{id}/repos", post(add_solution_repo))
        .route("/api/solutions/{id}/repos/{repo_id}", delete(remove_solution_repo))
        .route("/api/solutions/{id}/tags", post(add_solution_tag))
        .route("/api/solutions/{id}/tags/{tag}", delete(remove_solution_tag))
        // Indexing
        .route("/api/index", post(index_project))
        .route("/api/index/errors", get(list_index_errors))
        .route("/api/index/errors/{repo_id}", get(list_repo_index_errors))
        // Graph
        .route("/api/graph/nodes", get(graph_nodes))
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

    let graph = state.graph.lock().await;
    match crate::indexer::index_repo(&graph, &body.repo_path, &body.repo_id) {
        Ok(result) => {
            // Update project as indexed
            let s = state.store.lock().await;
            s.mark_indexed(&body.repo_id, &result.libs).ok();
            Ok(Json(serde_json::json!({
                "ok": true,
                "filesIndexed": result.files_indexed,
                "functionsIndexed": result.functions_indexed,
                "typesIndexed": result.types_indexed,
                "edgesCreated": result.edges_created,
                "durationMs": result.duration_ms,
                "libs": result.libs,
            })))
        }
        Err(e) => {
            let s = state.store.lock().await;
            s.mark_index_failed(&body.repo_id, &e).ok();
            Ok(Json(serde_json::json!({"ok": false, "error": e})))
        }
    }
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

// ── Scan ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ScanBody {
    root: String,
    #[serde(default = "default_depth")]
    max_depth: u32,
}

fn default_depth() -> u32 { 4 }

#[derive(Serialize)]
struct ScannedRepo {
    name: String,
    path: String,
    stack: Vec<String>,
    has_git: bool,
}

async fn scan_folder(
    Json(body): Json<ScanBody>,
) -> Result<Json<Vec<ScannedRepo>>, StatusCode> {
    let root = std::path::Path::new(&body.root);
    if !root.exists() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut repos = Vec::new();
    scan_dir(root, 0, body.max_depth, &mut repos);
    Ok(Json(repos))
}

fn scan_dir(dir: &std::path::Path, depth: u32, max_depth: u32, repos: &mut Vec<ScannedRepo>) {
    if depth > max_depth { return; }

    if dir.join(".git").exists() {
        let name = dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let stack = crate::config::detect_stack(dir);
        repos.push(ScannedRepo {
            name,
            path: dir.to_string_lossy().to_string(),
            stack,
            has_git: true,
        });
        return; // Don't recurse into git repos
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "node_modules" || name == "target" || name == "dist" {
                    continue;
                }
                scan_dir(&path, depth + 1, max_depth, repos);
            }
        }
    }
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
        assert!(json["functionsIndexed"].as_u64().unwrap() >= 1);

        // Graph should have data now
        let resp = app.oneshot(
            Request::builder()
                .uri("/api/graph/nodes?repoId=test-repo")
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["nodes"].as_array().unwrap().len() >= 1);
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
        let repos: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0]["name"], "my-project");
        assert_eq!(repos[0]["has_git"], true);
        assert!(repos[0]["stack"].as_array().unwrap().len() >= 1);
    }
}
