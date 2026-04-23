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
