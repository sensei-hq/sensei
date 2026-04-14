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
            "notifications/initialized" => continue, // no response needed
            "tools/list" => handle_list_tools(),
            "tools/call" => handle_call_tool(&params, &client),
            "resources/list" => json!({"resources": []}),
            "prompts/list" => json!({"prompts": []}),
            _ => {
                // Unknown method — return error
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
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
            tool("search", "Search functions and types by name across the codebase", &[
                ("query", "string", "Search term"),
                ("repoId", "string", "Project/repo ID to search in"),
            ]),
            tool("get_symbol", "Get detailed info about a function — file, line, signature, complexity", &[
                ("name", "string", "Function name"),
                ("repoId", "string", "Project/repo ID"),
            ]),
            tool("get_callers", "Find all functions that call the given function", &[
                ("name", "string", "Function name"),
                ("repoId", "string", "Project/repo ID"),
            ]),
            tool("get_callees", "Find all functions called by the given function", &[
                ("name", "string", "Function name"),
                ("repoId", "string", "Project/repo ID"),
            ]),
            tool("get_file_tags", "Get files tagged with a framework or pattern (svelte, react, hook, route, etc)", &[
                ("tag", "string", "Tag to search for"),
                ("repoId", "string", "Project/repo ID"),
            ]),
            tool("get_communities", "Get Leiden-detected code communities (clusters of related symbols)", &[
                ("repoId", "string", "Project/repo ID"),
            ]),
            tool("get_project_summary", "Get project stats — functions, types, libs, stack, solutions", &[
                ("repoId", "string", "Project/repo ID"),
            ]),
            tool("search_lib_docs", "Search indexed library documentation (llms.txt, README, etc)", &[
                ("query", "string", "Search term"),
            ]),
            tool("get_lib_docs", "Get all indexed docs for a library (e.g. bits-ui, rokkit)", &[
                ("name", "string", "Library name"),
            ]),
            tool("list_projects", "List all registered projects with their index status", &[]),
            tool("query", "Natural language query across the code graph", &[
                ("q", "string", "Query in natural language"),
                ("repoId", "string", "Optional project/repo ID to scope the query"),
            ]),
        ]
    })
}

fn handle_call_tool(params: &Value, client: &reqwest::blocking::Client) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // Route to the daemon's /api/mcp/call endpoint
    let result = client.post(format!("{}/api/mcp/call", DAEMON_URL))
        .json(&json!({"tool": tool_name, "params": args}))
        .send();

    match result {
        Ok(resp) => {
            if resp.status().is_success() {
                let data: Value = resp.json().unwrap_or(json!({"error": "invalid response"}));
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                    }]
                })
            } else {
                json!({
                    "content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}],
                    "isError": true
                })
            }
        }
        Err(e) => {
            json!({
                "content": [{"type": "text", "text": format!("Cannot reach senseid daemon at {}: {}", DAEMON_URL, e)}],
                "isError": true
            })
        }
    }
}

fn tool(name: &str, description: &str, params: &[(&str, &str, &str)]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for (pname, ptype, pdesc) in params {
        properties.insert(pname.to_string(), json!({"type": ptype, "description": pdesc}));
        required.push(pname.to_string());
    }
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}
