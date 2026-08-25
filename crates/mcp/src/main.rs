use serde_json::{Value, json};
use std::io::{BufRead, Write};

use sensei_mcp::{
    ActiveProject, DaemonRequest, HttpMethod, ProjectResolution, build_log_event_body,
    daemon_request_for, handle_initialize, handle_list_tools, resolve_active_project_in,
    resolve_default_project, resolve_from_cwd_in, resolve_project_in,
};

/// The active-project pin for THIS MCP session, held in memory — NOT a file.
/// The MCP is one stdio process per Claude session with a single-threaded
/// request loop, so a process-global is exactly session-scoped: `use_project`
/// sets it, later tool calls read it, and it vanishes when the session ends.
/// There is no `~/.sensei/active-project`, so a pin can never leak across
/// sessions or repos — the #109 stale-pin misroute is unrepresentable.
static ACTIVE_PIN: std::sync::Mutex<Option<ActiveProject>> = std::sync::Mutex::new(None);

/// Prepend a note (as its own text block) to a tool result's `content` — used to surface a
/// stale-pin project override so a mis-resolution is visible, not silent.
fn prepend_note(result: &mut Value, note: &str) {
    if let Some(arr) = result.get_mut("content").and_then(Value::as_array_mut) {
        arr.insert(0, json!({ "type": "text", "text": note }));
    }
}

fn daemon_url() -> String {
    sensei_bootstrap::daemon_url()
}

/// Convert a daemon HTTP result to an MCP content response.
fn daemon_result(result: reqwest::Result<reqwest::blocking::Response>) -> Value {
    match result {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>() {
            Ok(data) => {
                json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
            }
            // A 2xx with an unreadable body is a real failure — surface it, don't
            // hand the LLM a fabricated empty `{}` that reads as a genuine result.
            Err(e) => {
                json!({"content": [{"type": "text", "text": format!("Daemon returned an unreadable success response: {e}")}], "isError": true})
            }
        },
        // Non-2xx: surface the daemon's error MESSAGE, not just the status. The daemon
        // returns `{"error": "..."}` on a 400 (e.g. a bad register_plan graph — "duplicate
        // task id t1", "task t4 depends on unknown task tX"), so the caller sees WHAT to fix.
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            let detail = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
                .unwrap_or(body);
            let text = if detail.trim().is_empty() {
                format!("Daemon error: HTTP {status}")
            } else {
                format!("Daemon error: HTTP {status} — {detail}")
            };
            json!({"content": [{"type": "text", "text": text}], "isError": true})
        }
        Err(e) => {
            json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true})
        }
    }
}

/// Send a lib-shaped [`DaemonRequest`] with the given client. The library
/// decides the method / path / query / body; here we only prepend the base URL
/// and drive reqwest. Keeping the shaping in the library is what lets the
/// contract test cross the MCP↔daemon seam in-process.
fn send_daemon_request(
    client: &reqwest::blocking::Client,
    req: &DaemonRequest,
) -> reqwest::Result<reqwest::blocking::Response> {
    let url = format!("{}{}", daemon_url(), req.path);
    // Build a fresh RequestBuilder each attempt (send() consumes it).
    let build = || {
        let mut rb = match req.method {
            HttpMethod::Get => client.get(&url),
            HttpMethod::Post => client.post(&url),
            HttpMethod::Put => client.put(&url),
            HttpMethod::Delete => client.delete(&url),
        };
        if !req.query.is_empty() {
            rb = rb.query(&req.query);
        }
        if let Some(body) = &req.body {
            rb = rb.json(body);
        }
        rb
    };
    // The daemon bounces during an upgrade (stop → overlay → restart). A request
    // in that window fails to connect — but the proxy keeps running, so retry once
    // after a short pause and a daemon restart never surfaces as a failed tool call.
    // Only retry on a CONNECT error: the request never reached the daemon, so
    // there's no double-apply risk (unlike a timeout mid-request).
    match build().send() {
        Err(e) if e.is_connect() => {
            std::thread::sleep(std::time::Duration::from_millis(300));
            build().send()
        }
        other => other,
    }
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
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

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

fn handle_call_tool(params: &Value, client: &reqwest::blocking::Client, cwd: &str) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // `use_project` pins the named project; its `project` arg is the pin TARGET,
    // not a scoping hint, so handle it before the generic resolution below.
    if tool_name == "use_project" {
        return handle_use_project(&args, client);
    }

    // Resolve the project for scoping. `repo_id` is the resolved project NAME
    // (or "" when nothing resolved). Precedence: explicit `project=` arg →
    // cwd resolution → in-memory session pin (set by `use_project`). An
    // explicit-but-unresolvable arg errors (it never silently falls through to
    // the pin/cwd).
    let project_hint = args["project"].as_str().unwrap_or("");
    let explicit = if project_hint.is_empty() {
        None
    } else {
        match resolve_project(project_hint, client) {
            Ok(Some(name)) => Some(name),
            Ok(None) => {
                return json!({
                    "content": [{"type": "text", "text": format!(
                        "Project '{}' not found. Use the list_projects tool to see available projects.",
                        project_hint
                    )}],
                    "isError": true
                });
            }
            // Daemon unreachable/unreadable — do NOT report the project as missing.
            Err(e) => {
                return json!({
                    "content": [{"type": "text", "text": format!(
                        "Could not resolve project '{}': {}. The daemon may be starting or unreachable — retry shortly (this does NOT mean the project is absent).",
                        project_hint, e
                    )}],
                    "isError": true
                });
            }
        }
    };
    // Only read the pin + resolve cwd when there's no explicit arg (explicit wins).
    let (pin, cwd_name) = if explicit.is_some() {
        (None, String::new())
    } else {
        (ACTIVE_PIN.lock().unwrap().clone(), resolve_project_from_cwd(cwd, client))
    };
    // A resolved cwd beats a conflicting (stale) pin; the override is surfaced, not silent.
    let (repo_id, resolution_note) =
        match resolve_default_project(explicit.as_deref(), pin.as_ref(), &cwd_name) {
            ProjectResolution::Resolved { name, note, .. } => (name, note),
            ProjectResolution::Unresolved => (String::new(), None),
        };
    let resolved = (!repo_id.is_empty()).then_some(repo_id.as_str());

    // The library shapes every straightforward daemon call (knowledge / project
    // / patterns / workflow / mcp-proxy). Send it here and pipe through the
    // standard response conversion — prepending a stale-pin note so a mis-resolution
    // is visible to the caller instead of silently returning the wrong project's state.
    if let Some(req) = daemon_request_for(tool_name, &args, cwd, resolved) {
        let mut result = daemon_result(send_daemon_request(client, &req));
        if let Some(note) = resolution_note {
            prepend_note(&mut result, &format!("⟦project: {repo_id} — {note}⟧"));
        }
        return result;
    }

    // `None` → tools the binary owns because they need a custom HTTP client
    // (longer timeout), bespoke response parsing, or make no daemon call. Plus
    // `get_layered_context`'s one no-project guard.
    match tool_name {
        // ── Inference tools (direct endpoints, not mcp proxy) ──────────────
        "infer" => {
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

            let result =
                client.post(format!("{}/api/gateway/infer", daemon_url())).json(&body).send();

            match result {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>() {
                    // Unreadable 2xx → surface it, never pretty-print a fabricated `{}`.
                    Err(e) => {
                        json!({"content": [{"type": "text", "text": format!("Daemon returned an unreadable inference response: {e}")}], "isError": true})
                    }
                    Ok(data) => {
                        if let Some(content) = data["content"].as_str() {
                            json!({"content": [{"type": "text", "text": content}]})
                        } else if let Some(error) = data["error"].as_str() {
                            json!({"content": [{"type": "text", "text": format!("Inference error: {}", error)}], "isError": true})
                        } else {
                            json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_default()}]})
                        }
                    }
                },
                Ok(resp) => {
                    json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true})
                }
                Err(e) => {
                    json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true})
                }
            }
        }

        "embed" => {
            let texts_str = args["texts"].as_str().unwrap_or("");
            let texts: Vec<String> = texts_str.split(',').map(|s| s.trim().to_string()).collect();
            let model = args["model"].as_str();

            let body = json!({
                "texts": texts,
                "model": model,
            });

            let result =
                client.post(format!("{}/api/gateway/embed", daemon_url())).json(&body).send();

            daemon_result(result)
        }

        "gateway_status" => {
            let result = client.get(format!("{}/api/gateway/status", daemon_url())).send();
            daemon_result(result)
        }

        "consensus" => {
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

            match result {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>() {
                    // Unreadable 2xx → surface it, never a fabricated `{}`.
                    Err(e) => {
                        json!({"content": [{"type": "text", "text": format!("Daemon returned an unreadable consensus response: {e}")}], "isError": true})
                    }
                    Ok(data) => {
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
                },
                Ok(resp) => {
                    json!({"content": [{"type": "text", "text": format!("Daemon error: HTTP {}", resp.status())}], "isError": true})
                }
                Err(e) => {
                    json!({"content": [{"type": "text", "text": format!("Cannot reach senseid daemon: {}", e)}], "isError": true})
                }
            }
        }

        "generate_image" => {
            let prompt = match args["prompt"].as_str() {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => {
                    return json!({"content": [{"type": "text", "text": "prompt is required"}], "isError": true});
                }
            };

            // Resolve relative output_path against CWD so the daemon only sees absolute paths.
            let output_path = match args.get("output_path").and_then(|v| v.as_str()) {
                Some(p) => {
                    let path = std::path::PathBuf::from(p);
                    if path.is_absolute() {
                        Some(p.to_string())
                    } else if cwd.is_empty() {
                        return json!({
                            "content": [{
                                "type": "text",
                                "text": "cannot resolve relative output_path: CWD is unavailable"
                            }],
                            "isError": true,
                        });
                    } else {
                        Some(std::path::PathBuf::from(cwd).join(p).display().to_string())
                    }
                }
                None => None,
            };

            let mut body = json!({ "prompt": prompt });
            if let Some(p) = output_path {
                body["output_path"] = json!(p);
            }
            for (key, name) in [
                (args.get("model"), "model"),
                (args.get("router"), "router"),
                (args.get("size"), "size"),
                (args.get("quality"), "quality"),
                (args.get("style"), "style"),
            ] {
                if let Some(v) = key.and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
                    body[name] = json!(v);
                }
            }
            if let Some(n_str) = args["n"].as_str()
                && let Ok(n) = n_str.parse::<u8>()
            {
                body["n"] = json!(n);
            }

            // Use longer timeout for image generation (can take 15-30+ seconds with DALL-E 3 / high-res)
            let image_client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| client.clone());

            let result = image_client
                .post(format!("{}/api/gateway/image/generate", daemon_url()))
                .json(&body)
                .send();
            daemon_result(result)
        }

        "log_event" => {
            // Route the event into the hook-event sink (activity.assistant_events)
            // via the EXISTING /hook/event ingest — the same table Claude hooks
            // write to and the analyzer reads. This is the assistant-agnostic
            // capture path: any MCP-connected assistant emits here, family-tagged,
            // so its tool-calls + workflow milestones feed the same analyzer.
            // The `data` object (if any) is the base payload; the routing fields
            // are overlaid by build_log_event_body (the pure, tested mapper).
            let body = build_log_event_body(&args, cwd);
            let result = client.post(format!("{}/hook/event", daemon_url())).json(&body).send();
            daemon_result(result)
        }

        "get_layered_context" => {
            // Reached only when the library returned `None` — i.e. neither an
            // explicit project_id nor a resolved project name is available.
            json!({"content":[{"type":"text","text":"No project resolved. Pass project_id or run from a project directory."}], "isError": true})
        }

        // Unreachable: `daemon_request_for` returns `Some` for every other tool
        // (unknown names route to the mcp proxy). Kept as an honest fallback
        // rather than a silent success.
        other => json!({
            "content": [{"type": "text", "text": format!("tool '{}' has no handler", other)}],
            "isError": true
        }),
    }
}

/// `use_project`: resolve the named project (name or uuid) against the daemon,
/// then set the `{id,name}` pin IN MEMORY for this session so every later tool
/// call resolves to it regardless of cwd. No file is written — the pin is
/// process-global (this MCP process == one session), so it never outlives the
/// session or leaks to another repo. The resolve logic lives in the lib
/// (unit-tested); this only drives the daemon fetch and the in-memory set.
fn handle_use_project(args: &Value, client: &reqwest::blocking::Client) -> Value {
    let hint = args["project"].as_str().unwrap_or("");
    if hint.is_empty() {
        return json!({
            "content": [{"type": "text", "text": "use_project requires a `project` name or UUID."}],
            "isError": true
        });
    }
    let projects = match get_projects(client) {
        Ok(p) => p,
        Err(e) => {
            return json!({
                "content": [{"type": "text", "text": format!(
                    "Could not reach the daemon to resolve '{hint}': {e}. Retry shortly (not a 'project missing')."
                )}],
                "isError": true
            });
        }
    };
    let Some(pin) = resolve_active_project_in(&projects, hint) else {
        return json!({
            "content": [{"type": "text", "text": format!(
                "Project '{hint}' not found. Use find_projects or list_projects to see available projects."
            )}],
            "isError": true
        });
    };
    let msg = format!(
        "Pinned active project to '{}' ({}) for this session. Every tool will now resolve to it regardless of the working directory. Pass project=<other> on a tool to override for one call, or run use_project again to switch. (Session-scoped — the pin resets when this session ends.)",
        pin.name, pin.id
    );
    *ACTIVE_PIN.lock().unwrap() = Some(pin);
    json!({ "content": [{"type": "text", "text": msg}] })
}

/// Resolve a project name/library name to a project **name** by querying the daemon.
///
/// Returns `None` when no project matches the hint so callers can return a
/// clear error to the LLM rather than silently forwarding an arbitrary string.
/// `Ok(Some(name))` = resolved, `Ok(None)` = genuinely not found, `Err` = the
/// daemon couldn't be reached/read (so the caller must NOT say "not found").
fn resolve_project(
    hint: &str,
    client: &reqwest::blocking::Client,
) -> Result<Option<String>, String> {
    Ok(resolve_project_in(&get_projects(client)?, hint))
}

/// Resolve current project name from CWD by matching against registered project
/// folder paths. Best-effort: a daemon error yields "" (→ Unresolved, handled by
/// the caller), logged to stderr — never a fabricated match.
fn resolve_project_from_cwd(cwd: &str, client: &reqwest::blocking::Client) -> String {
    match get_projects(client) {
        Ok(projects) => resolve_from_cwd_in(&projects, cwd),
        Err(e) => {
            eprintln!("sensei-mcp: cwd project resolution skipped — {e}");
            String::new()
        }
    }
}

/// Fetch the project list, distinguishing a genuinely-empty list from a daemon
/// failure. NEVER collapse an error to `[]` — that made every named project look
/// nonexistent during a daemon bounce (a fabricated "project not found"). Mirrors
/// `send_daemon_request`'s connect-retry so an upgrade restart is survived.
fn get_projects(client: &reqwest::blocking::Client) -> Result<Vec<Value>, String> {
    let url = format!("{}/api/projects", daemon_url());
    let resp = match client.get(&url).send() {
        Err(e) if e.is_connect() => {
            std::thread::sleep(std::time::Duration::from_millis(300));
            client.get(&url).send()
        }
        other => other,
    }
    .map_err(|e| format!("cannot reach senseid daemon: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("daemon returned HTTP {} for /api/projects", resp.status()));
    }
    resp.json::<Vec<Value>>().map_err(|e| format!("unreadable /api/projects response: {e}"))
}
