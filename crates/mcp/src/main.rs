use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::OnceLock;

static DAEMON_URL_CACHE: OnceLock<String> = OnceLock::new();

/// Return the daemon base URL, detecting dev mode from the binary name.
///
/// Dev binaries (name ending in `-dev`) connect to port 7745; release binaries
/// connect to port 7744. The result is computed once and cached for the
/// lifetime of the process.
fn daemon_url() -> &'static str {
    DAEMON_URL_CACHE.get_or_init(|| {
        let is_dev = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .map(|name| name.ends_with("-dev"))
            .unwrap_or(false);
        let port = if is_dev { 7745u16 } else { 7744u16 };
        format!("http://127.0.0.1:{port}")
    })
}

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
        "serverInfo": { "name": "sensei", "version": env!("CARGO_PKG_VERSION") }
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
            // Pattern matching
            tool("match_pattern", "Find applicable patterns for a task. Returns detected patterns from the codebase that match the description. Call during the locate step before writing code. MANDATORY in /sensei:build.", &[
                ("description", "string", "What you're about to build (e.g. 'add SQL parsing', 'new API endpoint')"),
            ], &[]),
            tool("get_pattern_for", "Check if a specific symbol belongs to a detected pattern. Use during /sensei:review to check pattern conformance.", &[
                ("symbol", "string", "Symbol name to check (e.g. 'SqlAdapter', 'TaskWorker')"),
            ], &[]),
            tool("get_duplicates", "Find duplicate or very similar functions across different files. Use during /sensei:review to catch code duplication.", &[], &[]),
            tool("get_project_conventions", "Analyze project conventions — naming patterns, directory structure, design patterns. Use to understand how this project is structured.", &[], &[]),
            // Inference
            tool("infer", "Run inference using the gateway — chat, classify, summarize, or reason about text. Routes to the best available model automatically.", &[
                ("prompt", "string", "The text prompt or question"),
            ], &[
                ("system", "string", "System prompt to guide the response"),
                ("model", "string", "Specific model to use (e.g. 'gemma3:27b', 'claude-haiku')"),
                ("max_tokens", "string", "Maximum tokens in response"),
                ("capability", "string", "Capability: text_chat (default), text_complete"),
            ]),
            tool("embed", "Generate vector embeddings for text. Used for semantic search.", &[
                ("texts", "string", "Comma-separated texts to embed, or a single text"),
            ], &[
                ("model", "string", "Embedding model to use"),
            ]),
            tool("gateway_status", "Show inference gateway status — available adapters, models, health.", &[], &[]),
            tool("consensus", "Run a multi-model consensus analysis (MOE). Three models debate: proposer analyzes, challenger reviews, synthesizer produces consensus. Use for root cause analysis, architecture decisions, or any analysis that benefits from multiple perspectives.", &[
                ("signal", "string", "The signal, question, or topic to analyze"),
            ], &[
                ("context", "string", "Additional context (metrics, history, code snippets)"),
                ("proposer_model", "string", "Model for the proposer (default: best available)"),
                ("challenger_model", "string", "Model for the challenger (default: balanced)"),
                ("synthesizer_model", "string", "Model for the synthesizer (default: best available)"),
            ]),
            // Event logging
            tool("log_event", "Log a workflow event. Call this to record phase transitions, locate steps, issue lifecycle, review findings. MANDATORY in commands — do not skip.", &[
                ("type", "string", "Event type: phase_transition, command_invoked, locate, issue_started, issue_completed, review_finding, rework, checkpoint, context_loaded, files_modified"),
            ], &[
                ("data", "string", "JSON string with event-specific data"),
                ("session_id", "string", "Session ID if known"),
            ]),
        ]
    })
}

fn handle_call_tool(params: &Value, client: &reqwest::blocking::Client, cwd: &str) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // Resolve project: explicit "project" param → or detect from CWD
    let project_hint = args["project"].as_str().unwrap_or("");
    let repo_id = if !project_hint.is_empty() {
        match resolve_project(project_hint, client) {
            Some(id) => id,
            None => return json!({
                "content": [{"type": "text", "text": format!(
                    "Project '{}' not found. Use the list_projects tool to see available projects.",
                    project_hint
                )}],
                "isError": true
            }),
        }
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

    // ── Inference tools (direct endpoints, not mcp proxy) ──────────────
    if tool_name == "infer" {
        let prompt = args["prompt"].as_str().unwrap_or("");
        let system = args["system"].as_str();
        let model = args["model"].as_str();
        let max_tokens = args["max_tokens"].as_str().and_then(|s| s.parse::<u32>().ok());
        let capability = args["capability"].as_str().unwrap_or("text_chat");

        let body = json!({
            "capability": capability,
            "prompt": prompt,
            "system": system,
            "model": model,
            "max_tokens": max_tokens,
        });

        let result = client.post(format!("{}/api/gateway/infer", daemon_url()))
            .json(&body)
            .send();

        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({}));
                if let Some(content) = data["content"].as_str() {
                    json!({"content": [{"type": "text", "text": content}]})
                } else if let Some(error) = data["error"].as_str() {
                    json!({"content": [{"type": "text", "text": format!("Inference error: {}", error)}], "isError": true})
                } else {
                    json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
                }
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    if tool_name == "embed" {
        let texts_str = args["texts"].as_str().unwrap_or("");
        let texts: Vec<String> = texts_str.split(',').map(|s| s.trim().to_string()).collect();
        let model = args["model"].as_str();

        let body = json!({
            "texts": texts,
            "model": model,
        });

        let result = client.post(format!("{}/api/gateway/embed", daemon_url()))
            .json(&body)
            .send();

        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    if tool_name == "gateway_status" {
        let result = client.get(format!("{}/api/gateway/status", daemon_url())).send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    if tool_name == "consensus" {
        let signal = args["signal"].as_str().unwrap_or("");
        let context = args["context"].as_str();
        let proposer = args["proposer_model"].as_str();
        let challenger = args["challenger_model"].as_str();
        let synthesizer = args["synthesizer_model"].as_str();

        let body = json!({
            "signal": signal,
            "context": context,
            "proposer_model": proposer,
            "challenger_model": challenger,
            "synthesizer_model": synthesizer,
        });

        // Use longer timeout for consensus (3 model calls)
        let consensus_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| client.clone());

        let result = consensus_client
            .post(format!("{}/api/gateway/consensus", daemon_url()))
            .json(&body)
            .send();

        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({}));
                let text = if let Some(conclusion) = data["conclusion"].as_str() {
                    let confidence = data["confidence"].as_str().unwrap_or("unknown");
                    let duration = data["total_duration_ms"].as_u64().unwrap_or(0);
                    format!(
                        "## Consensus (confidence: {})\n\n{}\n\n---\n\nDuration: {}ms | Models: {}",
                        confidence,
                        conclusion,
                        duration,
                        serde_json::to_string(&data["models_used"]).unwrap_or_default()
                    )
                } else if let Some(error) = data["error"].as_str() {
                    format!("Consensus error: {}", error)
                } else {
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                };
                json!({"content": [{"type": "text", "text": text}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
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
        let result = client.put(format!("{}/api/state/{}", daemon_url(), repo_id))
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
        let result = client.get(format!("{}/api/state/{}", daemon_url(), repo_id)).send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    if tool_name == "match_pattern" {
        let desc = args["description"].as_str().unwrap_or("");
        let result = client.get(format!("{}/api/patterns/{}/match", daemon_url(), repo_id))
            .query(&[("description", desc)])
            .send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: serde_json::Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    if tool_name == "get_pattern_for" {
        let symbol = args["symbol"].as_str().unwrap_or("");
        let result = client.get(format!("{}/api/patterns/{}/for/{}", daemon_url(), repo_id, symbol)).send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: serde_json::Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }
    if tool_name == "get_duplicates" {
        let result = client.get(format!("{}/api/patterns/{}/duplicates", daemon_url(), repo_id)).send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: serde_json::Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }
    if tool_name == "get_project_conventions" {
        let result = client.get(format!("{}/api/patterns/{}/conventions", daemon_url(), repo_id)).send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: serde_json::Value = resp.json().unwrap_or(json!({}));
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            Ok(resp) => json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true}),
        };
    }

    if tool_name == "log_event" {
        let event_type = args["type"].as_str().unwrap_or("unknown");
        let data_str = args["data"].as_str().unwrap_or("{}");
        let session_id = args["session_id"].as_str();
        let body = json!({
            "project": repo_id,
            "event_type": event_type,
            "session_id": session_id,
            "data": serde_json::from_str::<serde_json::Value>(data_str).unwrap_or(json!(data_str)),
        });
        let result = client.post(format!("{}/api/events", daemon_url()))
            .json(&body)
            .send();
        return match result {
            Ok(resp) if resp.status().is_success() => {
                let data: serde_json::Value = resp.json().unwrap_or(json!({"ok": true}));
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

    let result = client.post(format!("{}/api/mcp/call", daemon_url()))
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

/// Resolve a project name/library name to a repo_id by querying the daemon.
///
/// Returns `None` when no project matches the hint so callers can return a
/// clear error to the LLM rather than silently forwarding an arbitrary string.
fn resolve_project(hint: &str, client: &reqwest::blocking::Client) -> Option<String> {
    let projects = get_projects(client);
    let hint_lower = hint.to_lowercase();

    // Exact repo_id match
    if let Some(p) = projects.iter().find(|p| p["repo_id"].as_str() == Some(hint)) {
        return Some(p["repo_id"].as_str().unwrap_or("").to_string());
    }

    // Exact name match (case insensitive)
    if let Some(p) = projects.iter().find(|p| {
        p["name"].as_str().map(|n| n.to_lowercase()) == Some(hint_lower.clone())
    }) {
        return Some(p["repo_id"].as_str().unwrap_or("").to_string());
    }

    // Partial name match
    if let Some(p) = projects.iter().find(|p| {
        p["name"].as_str().map(|n| n.to_lowercase().contains(&hint_lower)) == Some(true)
    }) {
        return Some(p["repo_id"].as_str().unwrap_or("").to_string());
    }

    // Library-name match: find the project that owns the lib
    if projects.iter().any(|p| {
        p["libs"].as_array().map(|libs| {
            libs.iter().any(|l| l.as_str().map(|s| s.to_lowercase()) == Some(hint_lower.clone()))
        }).unwrap_or(false)
    }) {
        if let Some(lib_project) = projects.iter().find(|p2| {
            p2["name"].as_str().map(|n| n.to_lowercase()) == Some(hint_lower.clone())
        }) {
            return Some(lib_project["repo_id"].as_str().unwrap_or("").to_string());
        }
    }

    None
}

/// Resolve current project from CWD by matching against registered project paths.
fn resolve_project_from_cwd(cwd: &str, client: &reqwest::blocking::Client) -> String {
    let projects = get_projects(client);

    // Find project whose path is a prefix of (or equal to) CWD
    let mut best_match = String::new();
    let mut best_len = 0;

    for p in &projects {
        if let Some(path) = p["path"].as_str()
            && cwd.starts_with(path) && path.len() > best_len
        {
            best_match = p["repo_id"].as_str().unwrap_or("").to_string();
            best_len = path.len();
        }
    }

    best_match
}

fn get_projects(client: &reqwest::blocking::Client) -> Vec<Value> {
    client.get(format!("{}/api/projects", daemon_url()))
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
