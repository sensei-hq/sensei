use axum::response::Json;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    ok: bool,
    name: &'static str,
    version: &'static str,
}

pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        name: "senseid",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Component health — checks if CLI, MCP bridge, and daemon binaries are available.
pub(crate) async fn health_components() -> Json<serde_json::Value> {
    let version = env!("CARGO_PKG_VERSION");

    let cli_status = check_binary("sensei");
    let mcp_status = check_binary("sensei-mcp");

    Json(serde_json::json!({
        "components": [
            { "id": "cli",    "name": "sensei-cli",    "version": cli_status.1, "status": cli_status.0, "icon": "令" },
            { "id": "mcp",    "name": "MCP bridge",    "version": mcp_status.1, "status": mcp_status.0, "icon": "橋" },
            { "id": "daemon", "name": "sensei-daemon", "version": version,      "status": "ready",       "icon": "守" },
        ]
    }))
}

/// Watcher status — shows watched roots, exclusions, and watcher state.
pub(crate) async fn watcher_status() -> Json<serde_json::Value> {
    let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);

    if let Ok(w) = watcher.lock() {
        let status = format!("{:?}", w.status());
        let roots: Vec<serde_json::Value> = w.roots().iter().map(|(path, root)| {
            serde_json::json!({
                "path": path.to_string_lossy(),
                "excluded": root.excluded,
            })
        }).collect();
        Json(serde_json::json!({ "status": status, "roots": roots }))
    } else {
        Json(serde_json::json!({ "status": "error", "message": "lock poisoned" }))
    }
}

/// Unregister a root from the watcher.
pub(crate) async fn watcher_unregister(
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() {
        return Json(serde_json::json!({ "error": "path required" }));
    }

    let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);

    if let Ok(mut w) = watcher.lock() {
        w.unregister(&std::path::PathBuf::from(path));
        Json(serde_json::json!({ "ok": true, "unregistered": path }))
    } else {
        Json(serde_json::json!({ "error": "lock poisoned" }))
    }
}

pub(crate) fn check_binary(name: &str) -> (&'static str, Option<String>) {
    // Check if binary exists on PATH or in known locations
    let paths = [
        std::path::PathBuf::from(format!("/opt/homebrew/bin/{}", name)),
        std::path::PathBuf::from(format!("/usr/local/bin/{}", name)),
    ];

    // Check PATH first
    if std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
    {
        // Try to get version
        let version = std::process::Command::new(name)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());
        return ("ready", version);
    }

    // Check known paths
    for path in &paths {
        if path.is_file() {
            return ("ready", None);
        }
    }

    ("missing", None)
}
