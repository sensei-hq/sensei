use serde_json::{json, Value};
use std::io::{BufRead, Write};

const DAEMON_URL: &str = "http://127.0.0.1:7744";

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to create HTTP client");

    // Detect current project from CWD
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() { continue; }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = request.get("id").cloned();
        let method = request["method"].as_str().unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => handle_initialize(),
            "notifications/initialized" => continue,
            "tools/list" => handle_list_tools(),
            "tools/call" => handle_call_tool(&params, &client, &cwd),
            "resources/list" => json!({"resources": []}),
            "prompts/list" => json!({"prompts": []}),
            _ => {
                let response = json!({
                    "jsonrpc": "2.0", "id": id,
                    "error": {"code": -32601, "message": format!("Method not found: {}", method)}
                });
                writeln!(out, "{}", serde_json::to_string(&response).unwrap()).ok();
                out.flush().ok();
                continue;
            }
        };

        let response = json!({"jsonrpc": "2.0", "id": id, "result": result});
        writeln!(out, "{}", serde_json::to_string(&response).unwrap()).ok();
        out.flush().ok();
    }
}

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "sensei", "version": "0.1.0" }
    })
}

fn handle_list_tools() -> Value {
    json!({
        "tools": [
            tool("search", "Search functions, types, and symbols in the current project or a named library/project. Use this when you need to find where something is defined.", &[
                ("query", "string", "What to search for (function name, type name, etc)"),
            ], &[
                ("project", "string", "Project or library name to search in (e.g. 'rokkit', 'kavach'). Defaults to current project."),
            ]),
            tool("get_callers", "Find all functions that call a given function. Use this to understand who depends on a function.", &[
                ("name", "string", "Function name to find callers of"),
            ], &[
                ("project", "string", "Project name. Defaults to current project."),
            ]),
            tool("get_callees", "Find all functions called by a given function. Use this to understand what a function depends on.", &[
                ("name", "string", "Function name to find callees of"),
            ], &[
                ("project", "string", "Project name. Defaults to current project."),
            ]),
            tool("get_project_summary", "Get overview of a project — function count, types, libraries used, tech stack.", &[], &[
                ("project", "string", "Project name. Defaults to current project."),
            ]),
            tool("get_lib_docs", "Get indexed documentation for a library. Without component param returns the index/overview. With component returns that specific component's docs (e.g. 'list', 'select', 'button').", &[
                ("name", "string", "Library name (e.g. 'bits-ui', 'rokkit', 'hono')"),
            ], &[
                ("component", "string", "Specific component name to get docs for (e.g. 'list', 'select', 'button'). Omit for the library index."),
            ]),
            tool("search_lib_docs", "Search across all indexed library documentation. Use when looking for how to use a feature.", &[
                ("query", "string", "What to search for in library docs"),
            ], &[]),
            tool("get_communities", "Get code architecture — clusters of related functions detected by community analysis.", &[], &[
                ("project", "string", "Project name. Defaults to current project."),
            ]),
            tool("get_patterns", "Get files tagged with a framework pattern (e.g. 'hook', 'middleware', 'route', 'component').", &[
                ("pattern", "string", "Pattern to search for"),
            ], &[
                ("project", "string", "Project name. Defaults to current project."),
            ]),
            tool("list_projects", "List all known projects and their index status.", &[], &[]),
            tool("create_session", "Start tracking a new coding session. Call at the beginning of a task.", &[
                ("task", "string", "Description of what you're working on"),
            ], &[]),
            tool("update_session", "Update a session with outcome and summary. Call when task is complete or blocked.", &[
                ("sessionId", "string", "Session ID returned by create_session"),
                ("outcome", "string", "completed, partial, or blocked"),
            ], &[
                ("summary", "string", "What was accomplished"),
                ("cost", "string", "Cost in USD"),
                ("tokensIn", "string", "Input tokens used"),
                ("tokensOut", "string", "Output tokens used"),
            ]),
            tool("add_library", "Index an external library's documentation. Tries to auto-discover llms.txt from common URLs. Provide url only if auto-discovery fails.", &[
                ("name", "string", "Library name (e.g. 'bits-ui', 'hono', 'drizzle-orm')"),
            ], &[
                ("url", "string", "Explicit URL if auto-discovery fails"),
                ("version", "string", "Library version"),
            ]),
            // Workflow state
            tool("update_phase", "Update the workflow phase, task, or active issue. Call this at the start of every phase command. MANDATORY — do not skip.", &[
                ("phase", "string", "Phase name: ideate, analyze, blueprint, experiment, plan, build, validate, brainstorm"),
            ], &[
                ("task", "string", "Active task description"),
                ("issue", "string", "GitHub issue number"),
                ("plan", "string", "Path to active plan doc"),
                ("checkpoint", "string", "Checkpoint description"),
            ]),
            tool("get_workflow_state", "Get current workflow state — active phase, task, issue, checkpoint. Call when you need orientation or feel lost.", &[], &[]),
        ]
    })
}

fn handle_call_tool(params: &Value, client: &reqwest::blocking::Client, cwd: &str) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // Resolve project: explicit "project" param → or detect from CWD
    let project_hint = args["project"].as_str().unwrap_or("");
    let repo_id = if !project_hint.is_empty() {
        resolve_project(project_hint, client)
    } else {
        resolve_project_from_cwd(cwd, client)
    };

    // Build daemon call params — forward all args + add resolved repoId
    let query = args["query"].as_str()
        .or(args["name"].as_str())
        .or(args["pattern"].as_str())
        .unwrap_or("");

    let mut daemon_params = args.clone();
    if let Some(obj) = daemon_params.as_object_mut() {
        obj.insert("repoId".into(), json!(repo_id));
        obj.insert("query".into(), json!(query));
        if let Some(pattern) = obj.remove("pattern") {
            obj.insert("tag".into(), pattern);
        }
        obj.insert("q".into(), json!(query));
    }

    // ── Workflow state tools (direct endpoints, not mcp proxy) ─────────
    if tool_name == "update_phase" {
        let body = json!({
            "active_phase": args["phase"].as_str(),
            "active_task": args["task"].as_str(),
            "active_issue": args["issue"].as_str().and_then(|s| s.parse::<i64>().ok()),
            "active_plan": args["plan"].as_str(),
            "last_checkpoint": args["checkpoint"].as_str(),
            "project_path": cwd,
        });
        let result = client.put(format!("{}/api/state/{}", DAEMON_URL, repo_id))
            .json(&body)
            .send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({"ok": true}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }
    if tool_name == "get_workflow_state" {
        let result = client.get(format!("{}/api/state/{}", DAEMON_URL, repo_id)).send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    // ── Standard tools (via daemon mcp proxy) ───────────────────────────
    // Map tool names
    let daemon_tool = match tool_name {
        "get_patterns" => "get_file_tags",
        other => other,
    };

    let result = client.post(format!("{}/api/mcp/call", DAEMON_URL))
        .json(&json!({"tool": daemon_tool, "params": daemon_params}))
        .send();

    match result {
        Ok(resp) if resp.status().is_success() => {
            let data: Value = resp.json().unwrap_or(json!({"error": "invalid response"}));
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                }]
            })
        }
        Ok(resp) => {
            json!({
                "content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}],
                "isError": true
            })
        }
        Err(e) => {
            json!({
                "content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}],
                "isError": true
            })
        }
    }
}

/// Resolve a project name/library name to a repoId by querying the daemon.
fn resolve_project(hint: &str, client: &reqwest::blocking::Client) -> String {
    // Try exact match first
    let projects = get_projects(client);

    // Exact repo_id match
    if let Some(p) = projects.iter().find(|p| p["repo_id"].as_str() == Some(hint)) {
        return p["repo_id"].as_str().unwrap().to_string();
    }

    // Exact name match (case insensitive)
    let hint_lower = hint.to_lowercase();
    if let Some(p) = projects.iter().find(|p| {
        p["name"].as_str().map(|n| n.to_lowercase()) == Some(hint_lower.clone())
    }) {
        return p["repo_id"].as_str().unwrap().to_string();
    }

    // Partial name match
    if let Some(p) = projects.iter().find(|p| {
        p["name"].as_str().map(|n| n.to_lowercase().contains(&hint_lower)) == Some(true)
    }) {
        return p["repo_id"].as_str().unwrap().to_string();
    }

    // Check if it's a library name used by any project
    if let Some(p) = projects.iter().find(|p| {
        p["libs"].as_array().map(|libs| {
            libs.iter().any(|l| l.as_str().map(|s| s.to_lowercase()) == Some(hint_lower.clone()))
        }).unwrap_or(false)
    }) {
        // Return the project that uses this lib — but the lib itself might be a project too
        // Check if there's a project with this lib's name
        if let Some(lib_project) = projects.iter().find(|p2| {
            p2["name"].as_str().map(|n| n.to_lowercase()) == Some(hint_lower.clone())
        }) {
            return lib_project["repo_id"].as_str().unwrap().to_string();
        }
    }

    hint.to_string() // fallback: use hint as-is
}

/// Resolve current project from CWD by matching against registered project paths.
fn resolve_project_from_cwd(cwd: &str, client: &reqwest::blocking::Client) -> String {
    let projects = get_projects(client);

    // Find project whose path is a prefix of (or equal to) CWD
    let mut best_match = String::new();
    let mut best_len = 0;

    for p in &projects {
        if let Some(path) = p["path"].as_str() {
            if cwd.starts_with(path) && path.len() > best_len {
                best_match = p["repo_id"].as_str().unwrap_or("").to_string();
                best_len = path.len();
            }
        }
    }

    best_match
}

fn get_projects(client: &reqwest::blocking::Client) -> Vec<Value> {
    client.get(format!("{}/api/projects", DAEMON_URL))
        .send()
        .ok()
        .and_then(|r| r.json::<Vec<Value>>().ok())
        .unwrap_or_default()
}

fn tool(name: &str, description: &str, required: &[(&str, &str, &str)], optional: &[(&str, &str, &str)]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut req_names = Vec::new();

    for (pname, ptype, pdesc) in required {
        properties.insert(pname.to_string(), json!({"type": ptype, "description": pdesc}));
        req_names.push(pname.to_string());
    }
    for (pname, ptype, pdesc) in optional {
        properties.insert(pname.to_string(), json!({"type": ptype, "description": pdesc}));
    }

    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": req_names,
        }
    })
}
