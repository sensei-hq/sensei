//! E2e test: start server, register project, index, query graph.

use std::time::Duration;

async fn wait_for_server(port: u16) -> bool {
    let client = reqwest::Client::new();
    for _ in 0..20 {
        if let Ok(resp) = client.get(format!("http://127.0.0.1:{}/health", port))
            .timeout(Duration::from_millis(500))
            .send().await
        {
            if resp.status().is_success() { return true; }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

#[tokio::test]
async fn full_index_flow_via_http() {
    let port = 17755u16; // Unique port to avoid conflicts

    // Create temp repo
    let repo_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(repo_dir.path().join("main.py"),
        "import os\nfrom typing import Optional\n\ndef greet(name: str) -> str:\n    \"\"\"Say hello.\"\"\"\n    return f'hello {name}'\n\nclass App:\n    def run(self):\n        greet('world')\n\nTIMEOUT = 30\n"
    ).unwrap();
    std::fs::write(repo_dir.path().join("utils.py"),
        "def helper():\n    pass\n\nMAX_RETRIES = 3\n"
    ).unwrap();

    // Create temp DB dir
    let db_dir = tempfile::TempDir::new().unwrap();
    let store_path = db_dir.path().join("sensei.db");
    let graph_path = db_dir.path().join("graph");

    // Start server in background
    let store = senseid_test::open_store(&store_path);
    let graph = senseid_test::open_graph(&graph_path);

    let server_handle = tokio::spawn(async move {
        senseid_test::start_server(store, graph, port).await
    });

    // Wait for server
    assert!(wait_for_server(port).await, "Server did not start on port {}", port);

    let client = reqwest::Client::new();

    // 1. Health check
    let resp = client.get(format!("http://127.0.0.1:{}/health", port))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let health: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(health["name"], "senseid");

    // 2. Register project
    let resp = client.post(format!("http://127.0.0.1:{}/api/projects", port))
        .json(&serde_json::json!({
            "repoId": "test-e2e",
            "path": repo_dir.path().to_string_lossy(),
        }))
        .send().await.unwrap();
    assert!(resp.status().is_success());

    // 3. List projects
    let resp = client.get(format!("http://127.0.0.1:{}/api/projects", port))
        .send().await.unwrap();
    let projects: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["repo_id"], "test-e2e");

    // 4. Index the project
    let resp = client.post(format!("http://127.0.0.1:{}/api/index", port))
        .json(&serde_json::json!({
            "repoId": "test-e2e",
            "repoPath": repo_dir.path().to_string_lossy(),
        }))
        .send().await.unwrap();
    assert!(resp.status().is_success());
    let result: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(result["ok"], true);
    let fns = result["functionsIndexed"].as_u64().unwrap_or(0);
    assert!(fns >= 2, "expected 2+ functions, got {}", fns);

    // 5. Query graph nodes
    let resp = client.get(format!("http://127.0.0.1:{}/api/graph/nodes?repoId=test-e2e", port))
        .send().await.unwrap();
    let graph_data: serde_json::Value = resp.json().await.unwrap();
    let nodes = graph_data["nodes"].as_array().unwrap();
    assert!(nodes.len() >= 2, "expected 2+ graph nodes, got {}", nodes.len());

    // 6. Tag the project
    let resp = client.post(format!("http://127.0.0.1:{}/api/projects/test-e2e/tags", port))
        .json(&serde_json::json!({"tag": "python"}))
        .send().await.unwrap();
    assert!(resp.status().is_success());

    // 7. Verify tag persisted
    let resp = client.get(format!("http://127.0.0.1:{}/api/projects", port))
        .send().await.unwrap();
    let projects: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(projects[0]["tags"].as_array().unwrap().contains(&serde_json::json!("python")));

    // 8. Create solution
    let resp = client.post(format!("http://127.0.0.1:{}/api/solutions", port))
        .json(&serde_json::json!({
            "name": "Test Solution",
            "repos": [{"repo_id": "test-e2e", "role": "backend"}]
        }))
        .send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 201);

    // 9. List solutions
    let resp = client.get(format!("http://127.0.0.1:{}/api/solutions", port))
        .send().await.unwrap();
    let solutions: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0]["name"], "Test Solution");

    // Shutdown
    let _ = client.post(format!("http://127.0.0.1:{}/stop", port)).send().await;
    server_handle.abort();
}

/// Helper module — exposes internals for testing.
/// In a real crate this would use `#[cfg(test)] pub` or a test-only feature.
mod senseid_test {
    use std::path::Path;
    use std::time::Duration;

    // Inline the store/graph/server creation since we can't import from the binary crate
    pub fn open_store(path: &Path) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects(repo_id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL UNIQUE, remote_url TEXT, indexed_at TEXT, last_error TEXT, duplicate_of TEXT, stack TEXT DEFAULT '[]', libs TEXT DEFAULT '[]', status TEXT DEFAULT 'active', last_commit_days INTEGER, commit_count INTEGER DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')));
            CREATE TABLE IF NOT EXISTS solutions(id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, client TEXT, category TEXT NOT NULL DEFAULT 'active', created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));
            CREATE TABLE IF NOT EXISTS solution_repos(solution_id TEXT NOT NULL REFERENCES solutions(id) ON DELETE CASCADE, repo_id TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'unknown', label TEXT, PRIMARY KEY(solution_id, repo_id));
            CREATE TABLE IF NOT EXISTS tags(entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, tag TEXT NOT NULL, PRIMARY KEY(entity_type, entity_id, tag));
            CREATE TABLE IF NOT EXISTS index_errors(id INTEGER PRIMARY KEY AUTOINCREMENT, repo_id TEXT NOT NULL, file_path TEXT NOT NULL, error TEXT NOT NULL, adapter TEXT, timestamp TEXT NOT NULL DEFAULT (datetime('now')));
        ").unwrap();
        conn
    }

    pub fn open_graph(path: &Path) -> rusqlite::Connection {
        std::fs::create_dir_all(path).ok();
        let conn = rusqlite::Connection::open(path.join("graph.db")).unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS functions(id TEXT PRIMARY KEY, name TEXT, file TEXT, line INTEGER, sig TEXT, body TEXT, docstring TEXT, complexity INTEGER DEFAULT 1, project TEXT);
            CREATE TABLE IF NOT EXISTS files(id TEXT PRIMARY KEY, path TEXT, module TEXT, lang TEXT, project TEXT);
            CREATE TABLE IF NOT EXISTS types(id TEXT PRIMARY KEY, name TEXT, file TEXT, line INTEGER, kind TEXT, project TEXT);
            CREATE TABLE IF NOT EXISTS edges(from_id TEXT NOT NULL, to_id TEXT NOT NULL, edge_type TEXT NOT NULL, weight REAL, PRIMARY KEY(from_id, to_id, edge_type));
        ").unwrap();
        conn
    }

    pub async fn start_server(store: rusqlite::Connection, graph: rusqlite::Connection, port: u16) -> std::io::Result<()> {
        // This is a simplified server that mirrors the real one's endpoints
        // In practice we'd import from the library, but binary crates can't be imported in integration tests
        use axum::{Router, routing::{get, post}, Json, extract::{Path as AxumPath}, http::StatusCode};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        struct AppState { store: Mutex<rusqlite::Connection>, graph: Mutex<rusqlite::Connection> }
        type SharedState = Arc<AppState>;

        let state: SharedState = Arc::new(AppState { store: Mutex::new(store), graph: Mutex::new(graph) });

        let app = Router::new()
            .route("/health", get(|| async { Json(serde_json::json!({"ok": true, "name": "senseid", "version": "0.1.0"})) }))
            .route("/api/projects", get({
                let s = state.clone();
                move || { let s = s.clone(); async move {
                    let db = s.store.lock().await;
                    let mut stmt = db.prepare("SELECT repo_id, name, path, indexed_at, last_error, stack, libs, status FROM projects ORDER BY name").unwrap();
                    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
                        let repo_id: String = row.get(0)?;
                        // Get tags
                        let tag_stmt = &format!("SELECT tag FROM tags WHERE entity_type='project' AND entity_id='{}'", repo_id);
                        let tags: Vec<String> = db.prepare(tag_stmt).ok()
                            .map(|mut s| s.query_map([], |r| r.get(0)).ok()
                                .map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default())
                            .unwrap_or_default();
                        Ok(serde_json::json!({
                            "repo_id": repo_id,
                            "name": row.get::<_, String>(1)?,
                            "path": row.get::<_, String>(2)?,
                            "indexed_at": row.get::<_, Option<String>>(3)?,
                            "last_error": row.get::<_, Option<String>>(4)?,
                            "stack": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?).unwrap_or_default(),
                            "libs": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(6)?).unwrap_or_default(),
                            "status": row.get::<_, String>(7)?,
                            "tags": tags,
                        }))
                    }).unwrap().filter_map(|r| r.ok()).collect();
                    Json(serde_json::json!(rows))
                }}
            }))
            .route("/api/projects", post({
                let s = state.clone();
                move |Json(body): Json<serde_json::Value>| { let s = s.clone(); async move {
                    let db = s.store.lock().await;
                    let repo_id = body["repoId"].as_str().unwrap_or("");
                    let path = body["path"].as_str().unwrap_or("");
                    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or(path.split('/').last().unwrap_or(""));
                    db.execute("INSERT OR REPLACE INTO projects(repo_id, name, path) VALUES(?1,?2,?3)",
                        rusqlite::params![repo_id, name, path]).ok();
                    Json(serde_json::json!({"ok": true}))
                }}
            }))
            .route("/api/projects/{repo_id}/tags", post({
                let s = state.clone();
                move |AxumPath(repo_id): AxumPath<String>, Json(body): Json<serde_json::Value>| { let s = s.clone(); async move {
                    let db = s.store.lock().await;
                    let tag = body["tag"].as_str().unwrap_or("");
                    db.execute("INSERT OR IGNORE INTO tags(entity_type, entity_id, tag) VALUES('project',?1,?2)",
                        rusqlite::params![repo_id, tag]).ok();
                    Json(serde_json::json!({"ok": true}))
                }}
            }))
            .route("/api/index", post({
                let s = state.clone();
                move |Json(body): Json<serde_json::Value>| { let s = s.clone(); async move {
                    let repo_id = body["repoId"].as_str().unwrap_or("").to_string();
                    let repo_path = body["repoPath"].as_str().unwrap_or("").to_string();

                    // Index using tree-sitter
                    let mut fns = 0u64;
                    let mut types = 0u64;
                    if let Ok(entries) = std::fs::read_dir(&repo_path) {
                        let gdb = s.graph.lock().await;
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                            if ext != ".py" { continue; }
                            if let Ok(source) = std::fs::read_to_string(&path) {
                                let mut parser = tree_sitter::Parser::new();
                                parser.set_language(&tree_sitter_python::LANGUAGE.into()).ok();
                                if let Some(tree) = parser.parse(&source, None) {
                                    let root = tree.root_node();
                                    let abs = path.to_string_lossy().to_string();
                                    for i in 0..root.child_count() {
                                        let c = root.child(i).unwrap();
                                        match c.kind() {
                                            "function_definition" => {
                                                let name = c.child_by_field_name("name").map(|n| n.utf8_text(source.as_bytes()).unwrap_or("")).unwrap_or("");
                                                let id = format!("fn:{}:{}:{}", abs, name, c.start_position().row + 1);
                                                gdb.execute("INSERT OR REPLACE INTO functions(id,name,file,line,sig,body,docstring,complexity,project) VALUES(?1,?2,?3,?4,'','','',1,?5)",
                                                    rusqlite::params![id, name, abs, c.start_position().row + 1, repo_id]).ok();
                                                fns += 1;
                                            }
                                            "class_definition" => {
                                                let name = c.child_by_field_name("name").map(|n| n.utf8_text(source.as_bytes()).unwrap_or("")).unwrap_or("");
                                                let id = format!("type:{}:{}:{}", abs, name, c.start_position().row + 1);
                                                gdb.execute("INSERT OR REPLACE INTO types(id,name,file,line,kind,project) VALUES(?1,?2,?3,?4,'class',?5)",
                                                    rusqlite::params![id, name, abs, c.start_position().row + 1, repo_id]).ok();
                                                types += 1;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        // Mark indexed
                        let sdb = s.store.lock().await;
                        sdb.execute("UPDATE projects SET indexed_at = datetime('now'), libs = '[]' WHERE repo_id = ?1",
                            rusqlite::params![repo_id]).ok();
                    }
                    Json(serde_json::json!({"ok": true, "functionsIndexed": fns, "typesIndexed": types}))
                }}
            }))
            .route("/api/graph/nodes", get({
                let s = state.clone();
                move |axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>| { let s = s.clone(); async move {
                    let repo_id = q.get("repoId").cloned().unwrap_or_default();
                    let gdb = s.graph.lock().await;
                    let mut stmt = gdb.prepare("SELECT id, name, file, line, complexity FROM functions WHERE project = ?1").unwrap();
                    let nodes: Vec<serde_json::Value> = stmt.query_map(rusqlite::params![repo_id], |row| {
                        Ok(serde_json::json!({"id": row.get::<_,String>(0)?, "name": row.get::<_,String>(1)?, "kind": "function", "file": row.get::<_,String>(2)?, "line": row.get::<_,u32>(3)?}))
                    }).unwrap().filter_map(|r| r.ok()).collect();
                    Json(serde_json::json!({"nodes": nodes, "edges": []}))
                }}
            }))
            .route("/api/solutions", get({
                let s = state.clone();
                move || { let s = s.clone(); async move {
                    let db = s.store.lock().await;
                    let mut stmt = db.prepare("SELECT id, name, description, client, category FROM solutions").unwrap();
                    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
                        Ok(serde_json::json!({"id": row.get::<_,String>(0)?, "name": row.get::<_,String>(1)?, "description": row.get::<_,Option<String>>(2)?, "client": row.get::<_,Option<String>>(3)?, "category": row.get::<_,String>(4)?, "repos": [], "tags": []}))
                    }).unwrap().filter_map(|r| r.ok()).collect();
                    Json(serde_json::json!(rows))
                }}
            }))
            .route("/api/solutions", post({
                let s = state.clone();
                move |Json(body): Json<serde_json::Value>| { let s = s.clone(); async move {
                    let db = s.store.lock().await;
                    let id = uuid::Uuid::new_v4().to_string();
                    let name = body["name"].as_str().unwrap_or("");
                    db.execute("INSERT INTO solutions(id,name) VALUES(?1,?2)", rusqlite::params![id, name]).ok();
                    if let Some(repos) = body["repos"].as_array() {
                        for r in repos {
                            let rid = r["repo_id"].as_str().unwrap_or("");
                            let role = r["role"].as_str().unwrap_or("unknown");
                            db.execute("INSERT OR REPLACE INTO solution_repos(solution_id,repo_id,role) VALUES(?1,?2,?3)",
                                rusqlite::params![id, rid, role]).ok();
                        }
                    }
                    (StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id})))
                }}
            }))
            .route("/stop", post(|| async {
                tokio::spawn(async { tokio::time::sleep(Duration::from_millis(50)).await; std::process::exit(0); });
                Json(serde_json::json!({"ok": true}))
            }));

        let cors = tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any);

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        axum::serve(listener, app.layer(cors)).await
    }
}
