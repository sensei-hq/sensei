use serde_json::{Value, json};
use std::io::{BufRead, Write};

use sensei_mcp::{
    ActiveWorkspace, DaemonRequest, HttpMethod, NEEDS_WORKSPACE, WORKSPACES_FOR_RESOLUTION,
    WorkspaceResolution, build_log_event_body, build_relay_wait_request, daemon_request_for_cwd,
    detect_host, handle_initialize, handle_list_tools, record_outcome_body,
    resolve_active_workspace_in, resolve_cwd_match_in, resolve_default_workspace,
    resolve_workspace_in,
};

/// The active-workspace pin for THIS MCP session, held in memory — NOT a file.
/// The MCP is one stdio process per Claude session with a single-threaded
/// request loop, so a process-global is exactly session-scoped: `use_workspace`
/// sets it, later tool calls read it, and it vanishes when the session ends.
/// There is no `~/.sensei/active-workspace`, so a pin can never leak across
/// sessions or repos — the #109 stale-pin misroute is unrepresentable.
static ACTIVE_PIN: std::sync::Mutex<Option<ActiveWorkspace>> = std::sync::Mutex::new(None);

/// Prepend a note (as its own text block) to a tool result's `content` — used to surface a
/// stale-pin workspace override so a mis-resolution is visible, not silent.
fn prepend_note(result: &mut Value, note: &str) {
    if let Some(arr) = result.get_mut("content").and_then(Value::as_array_mut) {
        arr.insert(0, json!({ "type": "text", "text": note }));
    }
}

/// Scan argv for `--daemon-url <value>` / `--daemon-url=<value>` (F1: the
/// run-01 broker was unreachable because no override existed). A manual scan:
/// one flag does not justify a CLI parser dep. Unknown args are ignored (the
/// MCP speaks stdio; historical launchers pass none). A dangling trailing
/// `--daemon-url` — or one followed by another `--*`-shaped flag (F1-L1:
/// `--daemon-url --other` must not eat `--other` as the value) — yields
/// `None` and `main` warns; it falls through to `SENSEI_DAEMON_URL`/default
/// rather than failing the session. The `=` form assigns literally, even a
/// flag-shaped suffix (explicit assignment wins).
fn parse_daemon_url_flag(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--daemon-url" {
            match iter.next() {
                Some(next) if !next.starts_with("--") => return Some(next.clone()),
                _ => return None,
            }
        }
        if let Some(value) = arg.strip_prefix("--daemon-url=") {
            return Some(value.to_string());
        }
    }
    None
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
    base: &str,
) -> reqwest::Result<reqwest::blocking::Response> {
    let url = format!("{base}{}", req.path);
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
        // Inside the per-attempt closure: the connect retry rebuilds the
        // whole request, and relay lane calls authenticate over these
        // headers (dropped here, every authed call 401s).
        for (name, value) in &req.headers {
            rb = rb.header(name, value);
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

    // Detect current workspace from CWD
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    // Daemon base, resolved once: --daemon-url flag > SENSEI_DAEMON_URL env >
    // default. Threaded explicitly (not a global): the call graph below is
    // small enough that the data flow stays visible.
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let flag = parse_daemon_url_flag(&argv);
    if argv.iter().any(|a| a == "--daemon-url") && flag.is_none() {
        eprintln!(
            "sensei-mcp: --daemon-url without a value — falling through to SENSEI_DAEMON_URL/default"
        );
    }
    let base = sensei_bootstrap::daemon_url_with_flag(flag.as_deref());

    // #455: the editor this session runs in. The environment is the client's (it
    // started this process) and is read once, here; the client's name arrives with
    // `initialize`. Until then, detection has only the environment.
    let env: std::collections::HashMap<String, String> = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();
    let mut detected_host = detect_host(None, &env);

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
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

        let result = match method {
            "initialize" => {
                detected_host = detect_host(params["clientInfo"]["name"].as_str(), &env);
                handle_initialize()
            }
            "notifications/initialized" => continue,
            "tools/list" => handle_list_tools(),
            "tools/call" => handle_call_tool(&params, &client, &cwd, &base, detected_host),
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

fn handle_call_tool(
    params: &Value,
    client: &reqwest::blocking::Client,
    cwd: &str,
    base: &str,
    detected_host: Option<&str>,
) -> Value {
    let tool_name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    // #455: register carries the editor this binary detected, never the caller's word for it.
    let args = if tool_name == "relay_register" {
        sensei_mcp::with_detected_host(&args, detected_host)
    } else {
        args
    };

    // A required argument that is missing is the caller's mistake, said as one —
    // sent on as an empty value it came back as an empty, plausible answer.
    let missing = sensei_mcp::missing_required(tool_name, &args);
    if !missing.is_empty() {
        return json!({
            "content": [{"type": "text", "text": format!(
                "{tool_name}: missing required argument(s): {}", missing.join(", ")
            )}],
            "isError": true
        });
    }

    // `use_workspace` pins the named workspace; its `workspace` arg is the pin TARGET,
    // not a scoping hint, so handle it before the generic resolution below.
    if tool_name == "use_workspace" {
        return handle_use_workspace(&args, client, base);
    }

    // Resolve the workspace for scoping. `repo_id` is the resolved workspace NAME
    // (or "" when nothing resolved). Precedence: explicit `workspace=` arg →
    // cwd resolution → in-memory session pin (set by `use_workspace`). An
    // explicit-but-unresolvable arg errors (it never silently falls through to
    // the pin/cwd).
    let workspace_hint = args["workspace"].as_str().unwrap_or("");
    let explicit = if workspace_hint.is_empty() {
        None
    } else {
        match resolve_workspace(workspace_hint, client, base) {
            Ok(Some(name)) => Some(name),
            Ok(None) => {
                return json!({
                    "content": [{"type": "text", "text": format!(
                        "Workspace '{}' not found. Use the list_workspaces tool to see available workspaces.",
                        workspace_hint
                    )}],
                    "isError": true
                });
            }
            // Daemon unreachable/unreadable — do NOT report the workspace as missing.
            Err(e) => {
                return json!({
                    "content": [{"type": "text", "text": format!(
                        "Could not resolve workspace '{}': {}. The daemon may be starting or unreachable — retry shortly (this does NOT mean the workspace is absent).",
                        workspace_hint, e
                    )}],
                    "isError": true
                });
            }
        }
    };
    // Only read the pin + resolve cwd when there's no explicit arg (explicit wins).
    // The cwd match carries BOTH the workspace name and the matched folder's
    // abs_path (#541) from a SINGLE workspace-list fetch.
    let (pin, cwd_name, cwd_folder) = if explicit.is_some() {
        (None, String::default(), String::default())
    } else {
        let pin = ACTIVE_PIN.lock().unwrap().clone();
        let (name, folder) = resolve_cwd_from_daemon(cwd, client, base);
        (pin, name, folder)
    };
    // A resolved cwd beats a conflicting (stale) pin; the override is surfaced, not silent.
    let (repo_id, resolution_note) =
        match resolve_default_workspace(explicit.as_deref(), pin.as_ref(), &cwd_name) {
            WorkspaceResolution::Resolved { name, note, .. } => (name, note),
            WorkspaceResolution::Unresolved => (String::default(), None),
        };
    let resolved = (!repo_id.is_empty()).then_some(repo_id.as_str());

    // The library shapes every straightforward daemon call (knowledge / workspace
    // / patterns / workflow / mcp-proxy). Send it here and pipe through the
    // standard response conversion — prepending a stale-pin note so a mis-resolution
    // is visible to the caller instead of silently returning the wrong workspace's state.
    if let Some(req) = daemon_request_for_cwd(tool_name, &args, cwd, resolved, &cwd_folder) {
        let mut result = daemon_result(send_daemon_request(client, &req, base));
        if let Some(note) = resolution_note {
            prepend_note(&mut result, &format!("⟦workspace: {repo_id} — {note}⟧"));
        }
        return result;
    }

    // `None` → tools the binary owns because they need a custom HTTP client
    // (longer timeout), bespoke response parsing, or make no daemon call. Plus
    // `get_layered_context`'s one no-workspace guard.
    match tool_name {
        // Outcomes the library could not read: the reason, as an error (never `[]`).
        "record_outcome" => {
            let reason = record_outcome_body(&args)
                .err()
                .unwrap_or_else(|| "record_outcome: no daemon call was made".into());
            json!({"content": [{"type": "text", "text": reason}], "isError": true})
        }
        // ── Inference tools (direct endpoints, not mcp proxy) ──────────────
        "infer" => infer_tool(&args, client, base),

        "embed" => embed_tool(&args, client, base),

        "gateway_status" => gateway_status_tool(client, base),

        "consensus" => consensus_tool(&args, client, base),

        "generate_image" => generate_image_tool(&args, client, cwd, base),

        "log_event" => log_event_tool(&args, client, cwd, base),

        // ── Relay: the long hold (≤50 s outlives the default 30 s client),
        // so it rides its own client — but through the standard send path,
        // which is what puts the Authorization header on the wire.
        "relay_wait_for_task" => relay_wait_tool(&args, client, base),

        // Reached only when the library returned `None` — no workspace given
        // and none resolved.
        t if t == "get_layered_context" || NEEDS_WORKSPACE.contains(&t) => {
            json!({"content":[{"type":"text","text":"No workspace resolved. Pass workspace_id or run from a workspace directory."}], "isError": true})
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

/// `infer`: answered by the binary (see `handle_call_tool`).
fn infer_tool(args: &Value, client: &reqwest::blocking::Client, base: &str) -> Value {
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

    let result = client.post(format!("{}/api/gateway/infer", base)).json(&body).send();

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

/// `embed`: answered by the binary (see `handle_call_tool`).
fn embed_tool(args: &Value, client: &reqwest::blocking::Client, base: &str) -> Value {
    let texts_str = args["texts"].as_str().unwrap_or("");
    let texts: Vec<String> = texts_str.split(',').map(|s| s.trim().to_string()).collect();
    let model = args["model"].as_str();

    let body = json!({
        "texts": texts,
        "model": model,
    });

    let result = client.post(format!("{}/api/gateway/embed", base)).json(&body).send();

    daemon_result(result)
}

/// `gateway_status`: answered by the binary (see `handle_call_tool`).
fn gateway_status_tool(client: &reqwest::blocking::Client, base: &str) -> Value {
    let result = client.get(format!("{}/api/gateway/status", base)).send();
    daemon_result(result)
}

/// `consensus`: answered by the binary (see `handle_call_tool`).
fn consensus_tool(args: &Value, client: &reqwest::blocking::Client, base: &str) -> Value {
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

    let result =
        consensus_client.post(format!("{}/api/gateway/consensus", base)).json(&body).send();

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

/// `generate_image`: answered by the binary (see `handle_call_tool`).
fn generate_image_tool(
    args: &Value,
    client: &reqwest::blocking::Client,
    cwd: &str,
    base: &str,
) -> Value {
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

    let result =
        image_client.post(format!("{}/api/gateway/image/generate", base)).json(&body).send();
    daemon_result(result)
}

/// `log_event`: answered by the binary (see `handle_call_tool`).
fn log_event_tool(
    args: &Value,
    client: &reqwest::blocking::Client,
    cwd: &str,
    base: &str,
) -> Value {
    // Route the event into the hook-event sink (activity.assistant_events)
    // via the EXISTING /hook/event ingest — the same table Claude hooks
    // write to and the analyzer reads. This is the assistant-agnostic
    // capture path: any MCP-connected assistant emits here, family-tagged,
    // so its tool-calls + workflow milestones feed the same analyzer.
    // The `data` object (if any) is the base payload; the routing fields
    // are overlaid by build_log_event_body (the pure, tested mapper).
    let body = build_log_event_body(args, cwd);
    let result = client.post(format!("{}/hook/event", base)).json(&body).send();
    daemon_result(result)
}

/// `relay_wait_for_task`: answered by the binary (see `handle_call_tool`).
fn relay_wait_tool(args: &Value, client: &reqwest::blocking::Client, base: &str) -> Value {
    let req = build_relay_wait_request(
        args["lane_id"].as_str().unwrap_or(""),
        args["lane_token"].as_str().unwrap_or(""),
        args["timeout_ms"].as_u64(),
        args["timeout_seconds"].as_u64(),
    );
    let wait_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| client.clone());
    daemon_result(send_daemon_request(&wait_client, &req, base))
}

/// `use_workspace`: resolve the named workspace (name or uuid) against the daemon,
/// then set the `{id,name}` pin IN MEMORY for this session so every later tool
/// call resolves to it regardless of cwd. No file is written — the pin is
/// process-global (this MCP process == one session), so it never outlives the
/// session or leaks to another repo. The resolve logic lives in the lib
/// (unit-tested); this only drives the daemon fetch and the in-memory set.
fn handle_use_workspace(args: &Value, client: &reqwest::blocking::Client, base: &str) -> Value {
    let hint = args["workspace"].as_str().unwrap_or("");
    if hint.is_empty() {
        return json!({
            "content": [{"type": "text", "text": "use_workspace requires a `workspace` name or UUID."}],
            "isError": true
        });
    }
    let workspaces = match get_workspaces(client, base) {
        Ok(p) => p,
        Err(e) => {
            return json!({
                "content": [{"type": "text", "text": format!(
                    "Could not reach the daemon to resolve '{hint}': {e}. Retry shortly (not a 'workspace missing')."
                )}],
                "isError": true
            });
        }
    };
    let Some(pin) = resolve_active_workspace_in(&workspaces, hint) else {
        return json!({
            "content": [{"type": "text", "text": format!(
                "Workspace '{hint}' not found. Use find_workspaces or list_workspaces to see available workspaces."
            )}],
            "isError": true
        });
    };
    let msg = format!(
        "Pinned active workspace to '{}' ({}) for this session. Every tool will now resolve to it regardless of the working directory. Pass workspace=<other> on a tool to override for one call, or run use_workspace again to switch. (Session-scoped — the pin resets when this session ends.)",
        pin.name, pin.id
    );
    *ACTIVE_PIN.lock().unwrap() = Some(pin);
    json!({ "content": [{"type": "text", "text": msg}] })
}

/// Resolve a workspace name/library name to a workspace **name** by querying the daemon.
///
/// Returns `None` when no workspace matches the hint so callers can return a
/// clear error to the LLM rather than silently forwarding an arbitrary string.
/// `Ok(Some(name))` = resolved, `Ok(None)` = genuinely not found, `Err` = the
/// daemon couldn't be reached/read (so the caller must NOT say "not found").
fn resolve_workspace(
    hint: &str,
    client: &reqwest::blocking::Client,
    base: &str,
) -> Result<Option<String>, String> {
    Ok(resolve_workspace_in(&get_workspaces(client, base)?, hint))
}

/// Resolve current workspace (name, folder) from CWD by matching against
/// registered workspace folder paths. Best-effort: a daemon error yields
/// ("", "") (→ Unresolved, handled by the caller), logged to stderr — never a
/// fabricated match. One fetch serves both halves (#541 needs the folder's
/// abs_path as well as the name).
fn resolve_cwd_from_daemon(
    cwd: &str,
    client: &reqwest::blocking::Client,
    base: &str,
) -> (String, String) {
    match get_workspaces(client, base) {
        Ok(workspaces) => resolve_cwd_match_in(&workspaces, cwd),
        Err(e) => {
            eprintln!("sensei-mcp: cwd workspace resolution skipped — {e}");
            (String::default(), String::default())
        }
    }
}

/// Fetch the workspace list, distinguishing a genuinely-empty list from a daemon
/// failure. NEVER collapse an error to `[]` — that made every named workspace look
/// nonexistent during a daemon bounce (a fabricated "workspace not found"). Mirrors
/// `send_daemon_request`'s connect-retry so an upgrade restart is survived.
fn get_workspaces(client: &reqwest::blocking::Client, base: &str) -> Result<Vec<Value>, String> {
    let url = format!("{base}{WORKSPACES_FOR_RESOLUTION}");
    let resp = match client.get(&url).send() {
        Err(e) if e.is_connect() => {
            std::thread::sleep(std::time::Duration::from_millis(300));
            client.get(&url).send()
        }
        other => other,
    }
    .map_err(|e| format!("cannot reach senseid daemon: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("daemon returned HTTP {} for /api/workspaces", resp.status()));
    }
    resp.json::<Vec<Value>>().map_err(|e| format!("unreadable /api/workspaces response: {e}"))
}

#[cfg(test)]
mod daemon_url_flag_tests {
    use super::parse_daemon_url_flag;

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn space_form_takes_next_arg() {
        assert_eq!(
            parse_daemon_url_flag(&argv(&["--daemon-url", "127.0.0.1:7844"])),
            Some("127.0.0.1:7844".to_string())
        );
    }

    #[test]
    fn equals_form_reads_suffix() {
        assert_eq!(
            parse_daemon_url_flag(&argv(&["--daemon-url=http://h:1"])),
            Some("http://h:1".to_string())
        );
    }

    #[test]
    fn absent_and_dangling_yield_none() {
        assert_eq!(parse_daemon_url_flag(&argv(&[])), None);
        assert_eq!(parse_daemon_url_flag(&argv(&["--other"])), None);
        assert_eq!(parse_daemon_url_flag(&argv(&["--daemon-url"])), None);
    }

    #[test]
    fn flag_shaped_next_arg_counts_as_dangling() {
        // F1-L1: `--daemon-url --other` must not consume `--other` as the
        // value — a `--*`-shaped next arg is dangling (None + warn + fall
        // through), same as a trailing `--daemon-url`.
        assert_eq!(parse_daemon_url_flag(&argv(&["--daemon-url", "--other"])), None);
    }
}

/// The tools the binary answers itself, against a fake daemon: a local listener that
/// records each request (method, path, body) and answers with a canned status and body.
#[cfg(test)]
mod binary_tool_tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

    /// A daemon on a free local port answering each request in turn with `replies`.
    fn fake_daemon(replies: Vec<(u16, &'static str)>) -> (String, Seen) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let seen: Seen = Arc::default();
        let log = seen.clone();
        std::thread::spawn(move || {
            for (status, body) in replies {
                let (mut conn, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(conn.try_clone().unwrap());
                let mut line = String::default();
                reader.read_line(&mut line).unwrap();
                let mut parts = line.split_whitespace();
                let (method, path) = (
                    parts.next().unwrap_or("").to_string(),
                    parts.next().unwrap_or("").to_string(),
                );
                let mut len = 0;
                loop {
                    let mut header = String::default();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                    if let Some(v) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut buf = vec![0; len];
                reader.read_exact(&mut buf).unwrap();
                log.lock().unwrap().push((method, path, String::from_utf8(buf).unwrap()));
                write!(
                    conn,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (base, seen)
    }

    fn client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::new()
    }
    fn text(result: &Value) -> &str {
        result["content"][0]["text"].as_str().unwrap_or("")
    }
    fn is_error(result: &Value) -> bool {
        result["isError"].as_bool() == Some(true)
    }
    fn body_of(seen: &Seen, i: usize) -> Value {
        serde_json::from_str(&seen.lock().unwrap()[i].2).unwrap()
    }
    /// A port nothing listens on.
    fn closed_base() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        drop(l);
        base
    }

    #[test]
    fn infer_sends_the_prompt_and_reads_content_error_status_and_unreachable() {
        let (base, seen) = fake_daemon(vec![
            (200, r#"{"content":"hello"}"#),
            (200, r#"{"error":"no model"}"#),
            (200, r#"{"other":1}"#),
            (503, "{}"),
        ]);
        let args = json!({ "prompt": "hi", "max_tokens": "64" });
        let ok = infer_tool(&args, &client(), &base);
        assert_eq!((text(&ok), is_error(&ok)), ("hello", false));
        let (method, path, _) = seen.lock().unwrap()[0].clone();
        assert_eq!((method.as_str(), path.as_str()), ("POST", "/api/gateway/infer"));
        assert_eq!(body_of(&seen, 0)["max_tokens"], 64);
        assert_eq!(body_of(&seen, 0)["capability"], "text_chat");
        let err = infer_tool(&args, &client(), &base);
        assert!(is_error(&err) && text(&err).contains("no model"));
        assert!(text(&infer_tool(&args, &client(), &base)).contains("\"other\""));
        let status = infer_tool(&args, &client(), &base);
        assert!(is_error(&status) && text(&status).contains("503"));
        let down = infer_tool(&args, &client(), &closed_base());
        assert!(is_error(&down) && text(&down).contains("Cannot reach senseid daemon"));
    }

    #[test]
    fn embed_status_log_event_and_relay_wait_reach_their_routes() {
        let (base, seen) =
            fake_daemon(vec![(200, "{}"), (200, r#"{"ok":true}"#), (200, "{}"), (200, "{}")]);
        embed_tool(&json!({ "texts": "a, b" }), &client(), &base);
        gateway_status_tool(&client(), &base);
        log_event_tool(&json!({ "type": "checkpoint" }), &client(), "/repo", &base);
        relay_wait_tool(
            &json!({ "lane_id": "l1", "lane_token": "t", "timeout_ms": 5 }),
            &client(),
            &base,
        );
        let seen_now = seen.lock().unwrap().clone();
        let routes: Vec<(&str, &str)> =
            seen_now.iter().map(|(m, p, _)| (m.as_str(), p.split('?').next().unwrap())).collect();
        assert_eq!(
            routes,
            [
                ("POST", "/api/gateway/embed"),
                ("GET", "/api/gateway/status"),
                ("POST", "/hook/event"),
                ("GET", "/api/relay/tasks/wait"),
            ]
        );
        assert_eq!(body_of(&seen, 0)["texts"], json!(["a", "b"]));
        assert_eq!(body_of(&seen, 2)["hook_event_name"], "checkpoint");
        assert!(seen_now[3].1.contains("lane_id=l1"));
    }

    #[test]
    fn consensus_formats_its_conclusion_and_surfaces_errors() {
        let (base, seen) = fake_daemon(vec![
            (
                200,
                r#"{"conclusion":"ship it","confidence":"high","total_duration_ms":12,"models_used":["a"]}"#,
            ),
            (200, r#"{"error":"no models"}"#),
            (500, "{}"),
        ]);
        let args = json!({ "signal": "s" });
        let ok = consensus_tool(&args, &client(), &base);
        assert!(
            text(&ok).contains("confidence: high")
                && text(&ok).contains("ship it")
                && text(&ok).contains("12ms")
        );
        assert_eq!(seen.lock().unwrap()[0].1, "/api/gateway/consensus");
        assert!(
            text(&consensus_tool(&args, &client(), &base)).contains("Consensus error: no models")
        );
        let failed = consensus_tool(&args, &client(), &base);
        assert!(is_error(&failed) && text(&failed).contains("500"));
    }

    #[test]
    fn generate_image_needs_a_prompt_and_resolves_a_relative_path_against_the_cwd() {
        let none = generate_image_tool(&json!({}), &client(), "/repo", &closed_base());
        assert!(is_error(&none) && text(&none).contains("prompt is required"));
        let no_cwd = generate_image_tool(
            &json!({ "prompt": "p", "output_path": "out.png" }),
            &client(),
            "",
            &closed_base(),
        );
        assert!(is_error(&no_cwd) && text(&no_cwd).contains("CWD is unavailable"));

        let (base, seen) = fake_daemon(vec![(200, r#"{"paths":["/repo/out.png"]}"#)]);
        let args = json!({ "prompt": "p", "output_path": "out.png", "size": "1024x1024", "n": "2", "style": "" });
        let ok = generate_image_tool(&args, &client(), "/repo", &base);
        assert!(!is_error(&ok));
        assert_eq!(seen.lock().unwrap()[0].1, "/api/gateway/image/generate");
        // Joined with the platform separator (`/repo\out.png` on Windows).
        let joined = std::path::Path::new("/repo").join("out.png");
        assert_eq!(
            body_of(&seen, 0),
            json!({ "prompt": "p", "output_path": joined.to_string_lossy(), "size": "1024x1024", "n": 2 })
        );
    }

    /// #455 5.3: what the binary detected rides `relay_register`; a caller's own
    /// `host_detected` never does.
    #[test]
    fn register_carries_the_host_this_binary_detected() {
        let (base, seen) = fake_daemon(vec![
            (200, "[]"),
            (200, r#"{"lane_id":"l"}"#),
            (200, "[]"),
            (200, r#"{"lane_id":"l"}"#),
        ]);
        let params = json!({ "name": "relay_register", "arguments": {
            "family": "claude-code", "host": "cursor", "host_detected": "kiro"
        }});
        let register = |seen: &Seen| {
            let calls = seen.lock().unwrap();
            let i =
                calls.iter().rposition(|c| c.1 == "/api/relay/register").expect("register sent");
            serde_json::from_str::<Value>(&calls[i].2).unwrap()
        };
        handle_call_tool(&params, &client(), "/repo", &base, Some("zed"));
        let body = register(&seen);
        assert_eq!(body["host_detected"], "zed");
        assert_eq!(body["host"], "cursor");
        handle_call_tool(&params, &client(), "/repo", &base, None);
        let body = register(&seen);
        assert!(body.get("host_detected").is_none(), "{body}");
        assert_eq!(body["host"], "cursor");
    }

    #[test]
    fn record_outcome_with_unreadable_outcomes_is_an_error_without_a_daemon_call() {
        let params = json!({ "name": "record_outcome", "arguments": { "outcomes": "[{oops" } });
        let result = handle_call_tool(&params, &client(), "/repo", &closed_base(), None);
        assert!(is_error(&result));
        assert!(text(&result).contains("not valid JSON"), "{result}");
    }
}
