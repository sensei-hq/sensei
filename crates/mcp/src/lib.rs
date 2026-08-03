//! sensei-mcp library surface.
//!
//! The binary (`main.rs`) is a thin stdio JSON-RPC loop; everything that is
//! pure — the tool catalog and, crucially, the **daemon-request shaping** for
//! each tool — lives here so it can be unit-tested and, more importantly,
//! contract-tested against the real daemon router in one process (see
//! `crates/senseid/tests/mcp_contract.rs`).
//!
//! The proxy's job is to turn one MCP `tools/call` into one daemon HTTP call.
//! When the *shape* of that call (path, query params, body) drifts from what
//! the daemon accepts, tools silently 400/404. [`daemon_request_for`] is the
//! single place that decides the shape, so a drift is catchable by a test that
//! crosses the seam.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// HTTP method for a shaped daemon request. Mirrors the reqwest verbs the
/// binary drives; kept dependency-free so the library needs no HTTP client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// A fully-shaped daemon HTTP request: everything the proxy would send for one
/// tool, minus the base URL (the binary prepends `daemon_url()`; a test prepends
/// nothing and drives the in-process router by `path`).
///
/// `query` is an ordered list of key/value pairs (some daemon handlers read
/// several, e.g. `?project=…&limit=…&tags=…`); `body` is the JSON payload for
/// `POST`/`PUT` tools (`None` for plain `GET`s).
#[derive(Debug, Clone, PartialEq)]
pub struct DaemonRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: Option<Value>,
}

impl DaemonRequest {
    fn get(path: impl Into<String>) -> Self {
        Self { method: HttpMethod::Get, path: path.into(), query: Vec::new(), body: None }
    }
    fn post_json(path: impl Into<String>, body: Value) -> Self {
        Self { method: HttpMethod::Post, path: path.into(), query: Vec::new(), body: Some(body) }
    }
    fn put_json(path: impl Into<String>, body: Value) -> Self {
        Self { method: HttpMethod::Put, path: path.into(), query: Vec::new(), body: Some(body) }
    }
    fn with_query(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.query.push((key.into(), val.into()));
        self
    }
}

/// Shape the daemon HTTP request the proxy sends for `tool`.
///
/// `resolved_project` is the project **name** the binary already resolved
/// (via `resolve_project` / `resolve_project_from_cwd`); `None` when nothing
/// resolved (equivalent to the binary's empty `repo_id`). `cwd` is the MCP
/// process working directory — some tools (rules fallback, governance folder,
/// workflow `project_path`) forward it.
///
/// Returns:
///   * `Some(req)` — a straightforward daemon call whose response the binary
///     pipes through `daemon_result`. This covers every knowledge / project /
///     patterns / workflow tool plus the `/api/mcp/call` proxy default.
///   * `None` — a tool the binary must handle inline because it needs a custom
///     HTTP client (longer timeout), bespoke response parsing, or makes no
///     daemon call at all: `infer`, `embed`, `gateway_status`, `consensus`,
///     `generate_image`, `log_event`. `get_layered_context` also returns `None`
///     in the one degenerate case where no project can be resolved (the binary
///     then returns a friendly "no project" error instead of a daemon 400).
///
/// This is the anti-drift seam: the contract test builds requests here and
/// issues them against the real daemon router, so a renamed path, a changed
/// query key, or a dropped tool fails the suite.
pub fn daemon_request_for(
    tool: &str,
    args: &Value,
    cwd: &str,
    resolved_project: Option<&str>,
) -> Option<DaemonRequest> {
    // The binary's `repo_id`: the resolved project NAME, or "" when unresolved.
    let repo_id = resolved_project.unwrap_or("");

    match tool {
        // ── Handled inline by the binary (custom client / response / no-op) ──
        // `use_project` is here too: it needs the daemon project list, a
        // {id,name} resolve, and a pin-file WRITE — no single daemon call.
        "infer" | "embed" | "gateway_status" | "consensus" | "generate_image" | "log_event"
        | "use_project" => None,

        // ── Folder-scoped project discovery ─────────────────────────────────
        // `find_projects` is the folder-scoped view of `list_projects`: shape
        // `GET /api/projects?under=<path>`, defaulting the scope to the MCP
        // call's cwd when the caller gives no `under`.
        "find_projects" => {
            let under = args["under"].as_str().filter(|s| !s.is_empty()).unwrap_or(cwd);
            Some(DaemonRequest::get("/api/projects").with_query("under", under))
        }

        // ── Who is doing the work: the folder's effective git author ─────────
        // `get_user_for_project` resolves the git `user.name`/`user.email` (with
        // git's own local→global precedence) + the owning project, defaulting to
        // the MCP call's cwd — same folder-scoping as `find_projects`. The
        // resolved identity matches the commit author and the Dōjō sign-in, so
        // a run/plan can be registered to the right person.
        "get_user_for_project" => {
            let under = args["under"].as_str().filter(|s| !s.is_empty()).unwrap_or(cwd);
            Some(DaemonRequest::get("/api/user").with_query("under", under))
        }

        // ── Set the user's behavioural stance (autonomy · sharing · review) ─────
        // `set_stance` POSTs `/api/stance`; `under` defaults to the MCP cwd (same
        // folder-scoping + git-identity resolution as `get_user_for_project`).
        // `scope` picks the rung (omitted → the user's default); each omitted dial
        // takes its stored default. The daemon validates the dial values.
        "set_stance" => {
            let under = args["under"].as_str().filter(|s| !s.is_empty()).unwrap_or(cwd);
            let mut body = json!({ "under": under });
            for k in ["user", "scope", "autonomy", "sharing", "review"] {
                if let Some(v) = args[k].as_str().filter(|s| !s.is_empty()) {
                    body[k] = json!(v);
                }
            }
            Some(DaemonRequest::post_json("/api/stance", body))
        }

        // ── Workflow state ──────────────────────────────────────────────────
        // `project` pins the workflow state to a specific project instead of the
        // cwd-resolved one — the escape hatch when the cwd mis-resolves (issue #109:
        // MCP default cwd = the `sensei-hq` container → wrong project), which
        // otherwise makes two concurrent sessions clobber one shared state row.
        // Mirrors start_run/register_plan's `project` handling.
        "update_phase" => {
            let project = args["project"].as_str().filter(|s| !s.is_empty()).unwrap_or(repo_id);
            let body = json!({
                "active_phase":   args["phase"].as_str(),
                "active_task":    args["task"].as_str(),
                "active_issue":   args["issue"].as_str().and_then(|s| s.parse::<i64>().ok()),
                "active_plan":    args["plan"].as_str(),
                "last_checkpoint": args["checkpoint"].as_str(),
                "project_path":   cwd,
            });
            Some(DaemonRequest::put_json(format!("/api/state/{project}"), body))
        }
        "get_workflow_state" => {
            let project = args["project"].as_str().filter(|s| !s.is_empty()).unwrap_or(repo_id);
            Some(DaemonRequest::get(format!("/api/state/{project}")))
        }

        // ── Pattern engine (direct /api/patterns endpoints) ─────────────────
        "match_pattern" => {
            let desc = args["description"].as_str().unwrap_or("");
            Some(DaemonRequest::get(format!("/api/patterns/{repo_id}/match")).with_query("description", desc))
        }
        "get_pattern_for" => {
            let symbol = args["symbol"].as_str().unwrap_or("");
            Some(DaemonRequest::get(format!("/api/patterns/{repo_id}/for/{symbol}")))
        }
        "get_duplicates" => Some(DaemonRequest::get(format!("/api/patterns/{repo_id}/duplicates"))),
        "get_project_conventions" => Some(DaemonRequest::get(format!("/api/patterns/{repo_id}/conventions"))),

        // ── Review-depth gate (E1) ──────────────────────────────────────────
        "resolve_risk_class" => {
            let paths = args.get("paths").cloned().unwrap_or_else(|| serde_json::json!([]));
            let mut body = serde_json::json!({ "paths": paths });
            if let Some(t) = args.get("task").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                body["task"] = serde_json::json!(t);
            }
            Some(DaemonRequest::post_json("/api/review/risk-class", body))
        }

        // ── Library-provided capabilities (workstream D) ────────────────────
        "list_library_skills" => {
            let name = args["name"].as_str().unwrap_or("");
            Some(DaemonRequest::get(format!("/api/libs/{name}/skills")))
        }
        "get_library_skill" => {
            let name = args["name"].as_str().unwrap_or("");
            let focus = args["focus"].as_str().unwrap_or("");
            Some(DaemonRequest::get(format!("/api/libs/{name}/skills/{focus}")))
        }
        "list_library_agents" => {
            let name = args["name"].as_str().unwrap_or("");
            Some(DaemonRequest::get(format!("/api/libs/{name}/agents")))
        }

        // ── Governance rules ────────────────────────────────────────────────
        "get_rules" => {
            let (key, val) = rules_query_param(repo_id, cwd);
            Some(DaemonRequest::get("/api/knowledge/rules").with_query(key, val))
        }

        // ── Commands surface (#83) ──────────────────────────────────────────
        "get_commands" => {
            let mut req = DaemonRequest::get(format!("/api/projects/{repo_id}/commands"));
            let category = args["category"].as_str().unwrap_or("");
            if !category.is_empty() {
                req = req.with_query("category", category);
            }
            Some(req)
        }

        // ── Layered memory context ──────────────────────────────────────────
        "get_layered_context" => {
            let explicit_pid = args["project_id"].as_str().unwrap_or("");
            // `None` → no project resolvable; the binary returns a guidance error.
            let (key, val) = context_project_param(explicit_pid, repo_id)?;
            let mut req = DaemonRequest::get("/api/knowledge/context").with_query(key, val);
            if let Some(l) = args["limit"].as_str().filter(|s| !s.is_empty()) {
                req = req.with_query("limit", l);
            }
            if let Some(t) = args["tags"].as_str().filter(|s| !s.is_empty()) {
                req = req.with_query("tags", t);
            }
            if let Some(s) = args["slot"].as_str().filter(|s| !s.is_empty()) {
                req = req.with_query("slot", s);
            }
            if let Some(f) = args["feature"].as_str().filter(|s| !s.is_empty()) {
                req = req.with_query("feature", f);
            }
            Some(req)
        }

        // ── Knowledge writes ────────────────────────────────────────────────
        "propose_memory" | "save_memory" => {
            let body = build_memory_body(args, cwd, repo_id);
            let path = if tool == "propose_memory" {
                "/api/knowledge/proposals"
            } else {
                "/api/knowledge/memories"
            };
            Some(DaemonRequest::post_json(path, body))
        }
        "promote_memory" => {
            let id = args["id"].as_str().unwrap_or("");
            Some(DaemonRequest::post_json(
                format!("/api/knowledge/memories/{id}/promote"),
                build_governance_body(args, cwd),
            ))
        }
        "accept_proposal" => {
            let id = args["id"].as_str().unwrap_or("");
            Some(DaemonRequest::post_json(format!("/api/knowledge/proposals/{id}/accept"), json!({})))
        }
        "reject_proposal" => {
            let id = args["id"].as_str().unwrap_or("");
            let body = match args["reason"].as_str().filter(|s| !s.is_empty()) {
                Some(r) => json!({ "reason": r }),
                None => json!({}),
            };
            Some(DaemonRequest::post_json(format!("/api/knowledge/proposals/{id}/reject"), body))
        }
        "record_outcome" => {
            let outcomes: Value = serde_json::from_str(args["outcomes"].as_str().unwrap_or("[]"))
                .unwrap_or_else(|_| json!([]));
            Some(DaemonRequest::post_json("/api/knowledge/outcomes", json!({ "outcomes": outcomes })))
        }

        // `plan` (D-PLANNER) decomposes a goal/spec/issue into a structured plan
        // (phases → features → acceptance criteria) + rendered docs/plan markdown.
        "plan" => {
            let mut body = json!({ "goal": args["goal"].as_str().unwrap_or("") });
            if let Some(ctx) = args["context"].as_str().filter(|s| !s.is_empty()) {
                body["context"] = json!(ctx);
            }
            Some(DaemonRequest::post_json("/api/planner/generate", body))
        }

        // `run_checkers` (D-CHECKER) runs the repo's adopted checker-backed rules
        // (their `checker_ref` command) and returns pass/fail verdicts.
        "run_checkers" => {
            // #109: only fall back to the cwd `folder` when NO project is given.
            // Sending folder=cwd alongside a project let the daemon prefer the
            // (container-mis-resolved) folder and 404 even with project:'sensei'.
            let mut body = json!({});
            if let Some(p) = args["project"].as_str().filter(|s| !s.is_empty()) {
                body["project"] = json!(p);
            } else {
                let folder = args["folder"].as_str().filter(|s| !s.is_empty()).unwrap_or(cwd);
                body["folder"] = json!(folder);
            }
            Some(DaemonRequest::post_json("/api/checkers/run", body))
        }

        // ── Relay run-control (P3.8) → the daemon's /api/runs endpoints ─────
        // `start_run` POSTs a new run; `project` defaults to the resolved repo
        // (name), matching every other tool's cwd→project convention. The daemon
        // resolves the project name/uuid and creates the run.
        "start_run" => {
            let goal = args["goal"].as_str().unwrap_or("");
            // Caller-supplied project wins; else the cwd-resolved repo name.
            let project = args["project"].as_str().filter(|s| !s.is_empty()).unwrap_or(repo_id);
            let mut body = json!({ "goal": goal });
            if !project.is_empty() {
                body["project"] = json!(project);
            }
            if let Some(plan) = args["plan_ref"].as_str().filter(|s| !s.is_empty()) {
                body["plan_ref"] = json!(plan);
            }
            Some(DaemonRequest::post_json("/api/runs", body))
        }
        // `run_status` lists active runs, or one run + its events with a run_id.
        "run_status" => match args["run_id"].as_str().filter(|s| !s.is_empty()) {
            Some(id) => Some(DaemonRequest::get(format!("/api/runs/{id}"))),
            None => Some(DaemonRequest::get("/api/runs")),
        },
        // `pause_run` marks a run paused-until-reset (a limit-wait, auto-resumes) —
        // POSTs `/api/runs/pause`; `project` defaults to the cwd-resolved repo, so
        // the active run for the current project is paused when no run_id is given.
        "pause_run" => {
            let mut body = json!({ "until": args["until"].as_str().unwrap_or("") });
            let project = args["project"].as_str().filter(|s| !s.is_empty()).unwrap_or(repo_id);
            if !project.is_empty() {
                body["project"] = json!(project);
            }
            for k in ["reason", "run_id"] {
                if let Some(v) = args[k].as_str().filter(|s| !s.is_empty()) {
                    body[k] = json!(v);
                }
            }
            Some(DaemonRequest::post_json("/api/runs/pause", body))
        }

        // ── Automated-run coordinator contract (AR-3) → daemon /api/runs/* ───
        // `register_plan` seeds an authored plan GRAPH as a run (AR-2): the daemon
        // validates it (DAG), stores it, and authors the Dōjō outline from it.
        // `plan` is the graph as a JSON string (parsed here, like record_outcome).
        "register_plan" => {
            let goal = args["goal"].as_str().unwrap_or("");
            let plan: Value =
                serde_json::from_str(args["plan"].as_str().unwrap_or("")).unwrap_or(Value::Null);
            let project = args["project"].as_str().filter(|s| !s.is_empty()).unwrap_or(repo_id);
            let mut body = json!({ "goal": goal, "plan": plan });
            if !project.is_empty() {
                body["project"] = json!(project);
            }
            if let Some(p) = args["plan_ref"].as_str().filter(|s| !s.is_empty()) {
                body["plan_ref"] = json!(p);
            }
            if let Some(mc) = args["max_concurrency"].as_str().and_then(|s| s.parse::<i64>().ok()) {
                body["max_concurrency"] = json!(mc);
            }
            Some(DaemonRequest::post_json("/api/runs/plan", body))
        }
        // `update_task_status` flips one plan task's state as the executor works
        // the graph — re-projects the authored outline + feeds the progress clock.
        "update_task_status" => {
            let run_id = args["run_id"].as_str().unwrap_or("");
            let task_id = args["task_id"].as_str().unwrap_or("");
            let mut body = json!({ "state": args["state"].as_str().unwrap_or("") });
            if let Some(n) = args["note"].as_str().filter(|s| !s.is_empty()) {
                body["note"] = json!(n);
            }
            Some(DaemonRequest::post_json(format!("/api/runs/{run_id}/tasks/{task_id}"), body))
        }
        // `report_run_outcome` marks a run terminal (done|failed) — the one terminal
        // transition an external coordinator may set.
        "report_run_outcome" => {
            let run_id = args["run_id"].as_str().unwrap_or("");
            let mut body = json!({ "outcome": args["outcome"].as_str().unwrap_or("") });
            if let Some(s) = args["summary"].as_str().filter(|s| !s.is_empty()) {
                body["summary"] = json!(s);
            }
            Some(DaemonRequest::post_json(format!("/api/runs/{run_id}/outcome"), body))
        }
        // `get_pending_nudges` pulls the human→agent steer for a run (the "daemon
        // initiates a check" pull side). Read-only; fail-soft to an empty list.
        "get_pending_nudges" => {
            let run_id = args["run_id"].as_str().unwrap_or("");
            Some(DaemonRequest::get(format!("/api/runs/{run_id}/nudges")))
        }

        // ── Front-door intake ────────────────────────────────────────────────
        "get_intake_guide" => Some(DaemonRequest::get("/api/playbook/guide")),
        "recommend_playbook" => {
            let mut body = serde_json::json!({
                "lifecycle": args["lifecycle"], "intent": args["intent"], "risk": args["risk"],
            });
            for k in ["session_id", "feature", "confirm"] {
                if let Some(v) = args[k].as_str().filter(|s| !s.is_empty()) {
                    body[k] = serde_json::json!(v);
                }
            }
            // Forward the project (explicit arg, else the cwd-resolved one) so the
            // daemon can suggest the skills/agents provided by libraries it depends on.
            let project = args["project"].as_str().filter(|s| !s.is_empty()).unwrap_or(repo_id);
            if !project.is_empty() {
                body["project"] = serde_json::json!(project);
            }
            Some(DaemonRequest::post_json("/api/playbook/recommend", body))
        }

        // ── §9 learning loop: accept path ────────────────────────────────────
        "list_playbook_rule_proposals" => Some(DaemonRequest::get("/api/playbook/rule-proposals")),
        "accept_playbook_rule" => {
            let id = args["id"].as_str().unwrap_or("");
            Some(DaemonRequest::post_json(format!("/api/playbook/rule/{id}/accept"), json!({})))
        }

        // ── Everything else → the daemon mcp proxy ──────────────────────────
        _ => {
            let params = build_daemon_params(args, repo_id);
            Some(DaemonRequest::post_json(
                "/api/mcp/call",
                json!({ "tool": map_daemon_tool(tool), "params": params }),
            ))
        }
    }
}

/// Pure: build the JSON body for `propose_memory` / `save_memory`. Both tools
/// send the identical body; only the endpoint differs. `project_id` precedence:
/// explicit arg wins, else the resolved project name (when scope=project).
/// Governance fields (`gov_scope` + `folder`=cwd, `enforcement`) are attached
/// only when the caller set them. Spine anchoring (`spine_slot`, `feature`) is
/// likewise attached only when the caller set them — the daemon's
/// `insert_with_status` validates the (slot, feature) scope rule and persists
/// both columns; absent here means unchanged behavior for every existing call.
fn build_memory_body(args: &Value, cwd: &str, repo_id: &str) -> Value {
    let scope = args["scope"].as_str().unwrap_or("");
    let tags: Vec<String> = args["tags"].as_str().unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut body = json!({
        "scope":   scope,
        "type":    args["type"].as_str().unwrap_or(""),
        "title":   args["title"].as_str().unwrap_or(""),
        "content": args["content"].as_str().unwrap_or(""),
        "tags":    tags,
    });
    for (arg_key, body_key) in [
        ("scope_filter", "scope_filter"), ("impact", "impact"), ("triage_signal", "triage_signal"),
        ("spine_slot", "spine_slot"), ("feature", "feature"), ("evidence", "evidence"),
    ] {
        if let Some(v) = args[arg_key].as_str().filter(|s| !s.is_empty()) {
            body[body_key] = json!(v);
        }
    }
    // project_id: explicit param wins, else resolved project name (scope=project).
    if let Some(pid) = args["project_id"].as_str().filter(|s| !s.is_empty()) {
        body["project_id"] = json!(pid);
    } else if scope == "project" && !repo_id.is_empty() {
        body["project_id"] = json!(repo_id);
    }
    attach_governance(&mut body, args, cwd);
    body
}

/// Pure: build the `promote_memory` body — just the governance overlay.
fn build_governance_body(args: &Value, cwd: &str) -> Value {
    let mut body = json!({});
    attach_governance(&mut body, args, cwd);
    body
}

/// Attach `gov_scope` (+ `folder`=cwd) and `enforcement` onto a body when set.
/// The daemon resolves `gov_scope` against the repo at `folder`, so the cwd is
/// forwarded as the governing folder.
fn attach_governance(body: &mut Value, args: &Value, cwd: &str) {
    if let Some(gs) = args["gov_scope"].as_str().filter(|s| !s.is_empty()) {
        body["gov_scope"] = json!(gs);
        // #109: prefer an explicit project (the daemon resolves the gov_scope's
        // repo from it server-side); only forward the cwd `folder` when no project
        // is given. Sending folder=cwd unconditionally shadowed a passed project
        // and mis-resolved to the container → a 400 "folder is not an indexed repo".
        if let Some(p) = args["project"].as_str().filter(|s| !s.is_empty()) {
            body["project"] = json!(p);
        } else if !cwd.is_empty() {
            body["folder"] = json!(cwd);
        }
    }
    if let Some(enf) = args["enforcement"].as_str().filter(|s| !s.is_empty()) {
        body["enforcement"] = json!(enf);
    }
}

/// Build the `/hook/event` ingest body for `log_event` — the assistant-agnostic
/// capture path into `activity.assistant_events`. The `data` JSON (if an object)
/// is the base payload; the routing keys `hook_event_fields` reads
/// (`hook_event_name`/`assistant_family`/`session_id`/`tool_name`/`cwd`/`exit_code`)
/// are overlaid. `cwd` falls back to the MCP working dir so the event attributes
/// to a project. Pure so it's unit-testable without a daemon.
pub fn build_log_event_body(args: &Value, cwd: &str) -> Value {
    let mut body = match args["data"].as_str() {
        Some(s) => match serde_json::from_str::<Value>(s) {
            Ok(Value::Object(m)) => Value::Object(m),
            Ok(v) => json!({ "data": v }),
            Err(_) => json!({ "data": s }),
        },
        None => json!({}),
    };
    body["hook_event_name"] = json!(args["type"].as_str().unwrap_or("unknown"));
    body["assistant_family"] = json!(args["family"].as_str().filter(|s| !s.is_empty()).unwrap_or("claude"));
    if let Some(sid) = args["session_id"].as_str().filter(|s| !s.is_empty()) {
        body["session_id"] = json!(sid);
    }
    if let Some(t) = args["tool_name"].as_str().filter(|s| !s.is_empty()) {
        body["tool_name"] = json!(t);
    }
    let event_cwd = args["cwd"].as_str().filter(|s| !s.is_empty()).unwrap_or(cwd);
    if !event_cwd.is_empty() {
        body["cwd"] = json!(event_cwd);
    }
    match args["success"].as_str() {
        Some("true") => { body["exit_code"] = json!(0); }
        Some("false") => { body["exit_code"] = json!(1); }
        _ => {}
    }
    body
}

pub fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "sensei", "version": env!("CARGO_PKG_VERSION") }
    })
}

pub fn handle_list_tools() -> Value {
    json!({
        "tools": [
            tool("search", "Search functions, types, and symbols in the current project or a named library/project. Use this when you need to find where something is defined.", &[
                ("query", "string", "What to search for (function name, type name, etc)"),
            ], &[
                ("project", "string", "Project or library name to search in (e.g. 'rokkit', 'kavach'). Defaults to current project."),
            ]),
            tool("context_pack", "Assemble a ready-to-use context bundle for a task or concept: the most relevant symbols (ranked by keyword AND meaning) with their actual code snippets, in one call. Prefer this over `search` when you want the code, not just where it lives.", &[
                ("query", "string", "The task or concept to gather context for (natural language works — it's matched semantically)"),
            ], &[
                ("project", "string", "Project name. Defaults to current project."),
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
            tool("list_library_skills", "List the focused skills a library provides (declared in its sensei.library.json manifest, or generated). Call when working with a library to see what curated how-to skills exist for it.", &[
                ("name", "string", "Library name (e.g. 'rokkit')"),
            ], &[]),
            tool("get_library_skill", "Get one library-provided skill by its topic focus (e.g. 'styling'). Returns the skill body to load. Use after list_library_skills.", &[
                ("name", "string", "Library name (e.g. 'rokkit')"),
                ("focus", "string", "Topic focus of the skill (e.g. 'styling')"),
            ], &[]),
            tool("list_library_agents", "List the review agents a library provides (declared in its sensei.library.json manifest) — e.g. a config/pattern reviewer for that library. Use during /sensei:review when the project depends on the library.", &[
                ("name", "string", "Library name (e.g. 'rokkit')"),
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
            tool("find_projects", "List the projects that live under a folder — the folder-scoped view of list_projects (which returns every project on the machine). Use this to discover which sensei project owns the directory you're working in. Defaults to the current working directory when 'under' is omitted.", &[], &[
                ("under", "string", "Absolute folder path to scope the search to. Defaults to the current working directory."),
            ]),
            tool("get_user_for_project", "Resolve the git author identity — user.name and user.email, using git's own local-over-global precedence (a repo .git/config override wins; otherwise ~/.gitconfig) — for the folder you're working in, plus the sensei project that owns it. This is the same identity as the commit author and the Dōjō sign-in, so it's how a run or plan gets registered to the right person for relay attribution. Defaults to the current working directory when 'under' is omitted.", &[], &[
                ("under", "string", "Absolute folder path to resolve the identity + owning project for. Defaults to the current working directory."),
            ]),
            tool("set_stance", "Set the user's behavioural stance — the three dials that govern HOW a run behaves (autonomy: how far it goes before asking · sharing: what surfaces to the dōjō · review: who signs off a rule) — at a scope. Complements the governance rules (WHAT a run may do). User-scoped and daemon-local. Omitting 'scope' sets the user's default across scopes; a scope key (e.g. 'project', 'organization') sets it for that rung and overrides the default there. Each omitted dial keeps its stored default. Defaults 'under' to the current working directory to resolve the git identity + scope namespace.", &[], &[
                ("autonomy", "string", "How far a run goes before asking: ask_always | ask_on_guarded (default) | ask_on_risky | run_freely."),
                ("sharing",  "string", "What surfaces to the dōjō: private | patterns (default) | patterns_prompts | derived."),
                ("review",   "string", "Approvers before a rule adopts: me_alone | one_maintainer (default) | two_maintainers | quorum."),
                ("scope",    "string", "Scope key to set the stance at (e.g. 'project', 'organization'). Omit for the user's default across scopes."),
                ("under",    "string", "Absolute folder path — resolves the git identity + the scope's namespace. Defaults to the current working directory."),
                ("user",     "string", "Explicit user key (git email) to set the stance for. Defaults to the git identity at 'under'."),
            ]),
            tool("use_project", "Pin the active project so every subsequent tool call resolves to it regardless of the server's working directory. Call this when the user tells you which project they're working on (e.g. use_project 'sensei'). The pin persists until you switch it by calling use_project again; an explicit project= argument on any other tool still overrides it for that one call.", &[
                ("project", "string", "Project name or UUID to pin as active (e.g. 'sensei')."),
            ], &[]),
            tool("create_session", "Start tracking a new coding session. Call at the beginning of a task.", &[
                ("task", "string", "Description of what you're working on"),
            ], &[]),
            tool("update_session", "Update a session with outcome and summary. Call when task is complete or blocked.", &[
                ("sessionId", "string", "Session ID returned by create_session"),
                ("outcome", "string", "completed, partial, or blocked"),
            ], &[
                ("summary", "string", "What was accomplished"),
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
                ("project", "string", "Project name or UUID to pin the state to. Defaults to the current (cwd-resolved) project; pass it when concurrent sessions must not share one project's workflow state."),
            ]),
            tool("get_workflow_state", "Get current workflow state — active phase, task, issue, checkpoint. Call when you need orientation or feel lost.", &[], &[
                ("project", "string", "Project name or UUID. Defaults to the current (cwd-resolved) project."),
            ]),
            // Pattern matching
            tool("match_pattern", "Find applicable patterns for a task. Returns detected patterns from the codebase that match the description. Call during the locate step before writing code. MANDATORY in /sensei:build.", &[
                ("description", "string", "What you're about to build (e.g. 'add SQL parsing', 'new API endpoint')"),
            ], &[]),
            tool("get_pattern_for", "Check if a specific symbol belongs to a detected pattern. Use during /sensei:review to check pattern conformance.", &[
                ("symbol", "string", "Symbol name to check (e.g. 'SqlAdapter', 'TaskWorker')"),
            ], &[]),
            tool("get_duplicates", "Find duplicate or very similar functions across different files. Use during /sensei:review to catch code duplication.", &[], &[]),
            tool("get_project_conventions", "Analyze project conventions — naming patterns, directory structure, design patterns. Use to understand how this project is structured.", &[], &[]),
            tool("resolve_risk_class", "Classify how hard a change must be reviewed (auto | review | approve) from its changed file paths. Call FIRST in /sensei:review to set the depth: 'approve' (identity/auth/money/secrets/schema/governance) demands the full adversarial review + human sign-off; 'review' is a standard review; 'auto' can skip the heavy pass. Returns {class, reasons}.", &[
                ("paths", "array", "Changed file paths (repo-relative or absolute)."),
            ], &[
                ("task", "string", "Optional task description; a sensitive/destructive task escalates an otherwise-low change."),
            ]),
            tool("get_rules", "Get the governance rules that apply to this repository, resolved across its scopes (organization / project / technology / …) and ranked by enforcement. Rules flagged mandatory are non-negotiable and cannot be overridden. Call at the start of a task to learn the constraints you must obey.", &[], &[]),
            tool("get_commands", "List the discoverable commands for this project — the actual `test` / `build` / `lint` / `e2e` invocations, derived from each folder's manifest (package.json scripts, etc.). Call when you need to know how to run tests, build, or lint here without guessing. Optionally filter by canonical category verb.", &[], &[
                ("project",  "string", "Project name. Defaults to current project."),
                ("category", "string", "Canonical verb filter — test | build | lint | e2e | run | format | typecheck | bench | docs | dev | start."),
            ]),
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
            tool("generate_image", "Generate an image from a text prompt using the configured image provider (OpenAI by default). Saves to the given output_path (relative paths are resolved against CWD) or to a sensei-managed cache when omitted. Returns the absolute file path. Use this when the user asks for visual assets — logos, illustrations, diagrams, character art, mockup imagery — that belong in the project.", &[
                ("prompt", "string", "Description of the image to generate"),
            ], &[
                ("output_path", "string", "Where to save the image. Relative to the current working directory, or absolute. If omitted, saves to ~/.sensei/generated/<hash>.png"),
                ("model", "string", "Model id, bare or router-qualified (e.g. 'dall-e-3', 'openai/dall-e-3')"),
                ("router", "string", "Which router to use (openai, stability, fal, replicate). Defaults to gateway selection."),
                ("size", "string", "Image size — provider-specific (e.g. 1024x1024 for OpenAI)"),
                ("quality", "string", "Image quality — provider-specific (standard|hd for OpenAI)"),
                ("style", "string", "Image style — provider-specific (vivid|natural for OpenAI)"),
                ("n", "string", "Number of images to generate (default 1)"),
            ]),
            // Event logging
            tool("log_event", "Record an activity/workflow event into the capture stream (activity.assistant_events) — the same sink Claude Code hooks write to and the analyzer reads for tool-usage + working-pattern signals. This is the assistant-AGNOSTIC capture path: assistants without a hook system (Cursor/Codex/etc.) call this to emit their tool-calls and workflow milestones so their activity is analyzed too. For tool-call capture set type=PreToolUse|PostToolUse + tool_name; for workflow milestones use phase_transition|command_invoked|review_finding|rework|checkpoint.", &[
                ("type", "string", "Event type — a hook name (PreToolUse, PostToolUse, Stop, SessionStart) for raw capture, or a workflow milestone (phase_transition, command_invoked, review_finding, rework, checkpoint, files_modified)"),
            ], &[
                ("data", "string", "JSON string with event-specific detail (stored as the event payload)"),
                ("session_id", "string", "Assistant session id (groups events into a session)"),
                ("family", "string", "Assistant family emitting the event (claude, cursor, codex, copilot, zed). Defaults to claude."),
                ("tool_name", "string", "The tool, for PreToolUse/PostToolUse events — required for tool-usage analysis"),
                ("cwd", "string", "Working directory (used to attribute the event to a project). Defaults to the MCP working directory."),
                ("success", "string", "'true'/'false' for a tool/turn outcome"),
            ]),
            // ── Knowledge plane ───────────────────────────────────────
            tool("propose_memory",
                "Capture an AI-detected learning into the triage queue. Use when a heuristic fires \
                 (revert / correction / 'actually...' / repeat_pattern / override / test_failure). \
                 User reviews these in the Learnings UI before they enter active memory.",
                &[
                    ("scope",         "string", "global | project | stack"),
                    ("type",          "string", "memory_type enum value (e.g. convention, pattern, decision)"),
                    ("title",         "string", "Short heading"),
                    ("content",       "string", "Rule body — what the agent should know"),
                    ("triage_signal", "string", "Which capture heuristic fired"),
                ],
                &[
                    ("project_id",   "string", "Project UUID (required when scope=project)"),
                    ("scope_filter", "string", "Required when scope=stack (e.g. 'rust')"),
                    ("impact",       "string", "What breaks if ignored"),
                    ("tags",         "string", "Comma-separated tag list (e.g. 'security,performance')"),
                    ("gov_scope",    "string", "Governance scope this rule governs: general|user|organization|client|technology|team|project|repository (resolved against the current repo)"),
                    ("enforcement",  "string", "Authority: advisory|recommended|required|mandatory (default recommended; mandatory = non-overridable)"),
                    ("spine_slot",   "string", "spine slot to anchor to: vision|personas|journeys|roadmap|design|mockups|decisions|brief|plan|tests"),
                    ("feature",      "string", "feature name — required for feature-scope slots (brief/plan/tests)"),
                    ("evidence",     "string", "Optional source evidence — a file:line, test name, or run id — recorded as the memory's provenance"),
                ]),
            tool("save_memory",
                "Explicit memory save — used when the user runs /save. Goes straight into active state. \
                 Never call this on heuristic detection — use propose_memory for that.",
                &[
                    ("scope",   "string", "global | project | stack"),
                    ("type",    "string", "memory_type enum value"),
                    ("title",   "string", "Short heading"),
                    ("content", "string", "Rule body"),
                ],
                &[
                    ("project_id",   "string", "Project UUID"),
                    ("scope_filter", "string", "Required when scope=stack"),
                    ("impact",       "string", "What breaks if ignored"),
                    ("tags",         "string", "Comma-separated tags"),
                    ("gov_scope",    "string", "Governance scope this rule governs: general|user|organization|client|technology|team|project|repository (resolved against the current repo)"),
                    ("enforcement",  "string", "Authority: advisory|recommended|required|mandatory (default recommended; mandatory = non-overridable)"),
                    ("spine_slot",   "string", "spine slot to anchor to: vision|personas|journeys|roadmap|design|mockups|decisions|brief|plan|tests"),
                    ("feature",      "string", "feature name — required for feature-scope slots (brief/plan/tests)"),
                    ("evidence",     "string", "Optional source evidence — a file:line, test name, or run id — recorded as the memory's provenance"),
                ]),
            tool("promote_memory",
                "Promote a proven (battle-tested) rule to a broader governance scope, e.g. project → organization. \
                 Creates a proposal at the new scope for the user to accept; it never auto-applies.",
                &[("id", "string", "Memory id (UUID) to promote")],
                &[
                    ("gov_scope",   "string", "Target scope: organization|client|technology|team|project (resolved against the project's repo)"),
                    ("project",     "string", "Project the gov_scope resolves against (e.g. 'sensei'). Prefer this over the working directory."),
                    ("enforcement", "string", "Authority at the new scope: advisory|recommended|required|mandatory"),
                ]),
            tool("accept_proposal",
                "Accept a proposed memory — moves it from triage to active.",
                &[("id", "string", "Proposal memory id (UUID)")],
                &[]),
            tool("reject_proposal",
                "Reject a proposed memory — moves it to rejected state.",
                &[("id", "string", "Proposal memory id (UUID)")],
                &[("reason", "string", "Why it was rejected (logged)")]),
            tool("record_outcome",
                "Record one or more memory outcomes (applied / consulted / violated / ignored). \
                 Batched — call once per turn with all outcomes the session generated.",
                &[("outcomes", "string", "JSON array string of {memory_id, outcome[, session_id, context]}")],
                &[]),
            tool("get_layered_context",
                "Fetch the blended memory context for the current project — global + project + \
                 stack-matched memories, ordered by strength. Call at session start and on /recall.",
                &[],
                &[
                    ("project",    "string", "Project name. Defaults to current project."),
                    ("project_id", "string", "Project UUID — overrides project name lookup."),
                    ("limit",      "string", "Max memories to return (default 200, cap 500)"),
                    ("tags",       "string", "Comma-separated tag filter"),
                    ("slot",       "string", "optional: lead the context with memories anchored to this spine slot (vision|personas|journeys|roadmap|design|mockups|decisions|brief|plan|tests)"),
                    ("feature",    "string", "optional feature name for a feature-scope slot"),
                ]),
            // ── Planning (D-PLANNER) ─────────────────────────────────────────
            tool("plan",
                "Decompose a goal, spec, or issue into a structured plan — ordered phases, each with \
                 features that carry observable acceptance criteria, scope, and dependencies (shaped to \
                 clear the plan-depth-reviewer bar). Returns the structured plan plus rendered \
                 docs/plan markdown; review it, save it, then hand the path to `start_run` as plan_ref.",
                &[("goal", "string", "The goal / spec / issue to decompose into a plan")],
                &[("context", "string", "Optional grounding — a spec, an issue body, or conventions to plan against")]),
            tool("run_checkers",
                "Run this repo's checker-backed governance rules (D-CHECKER) — each rule whose \
                 verification is a `checker` runs its resolved command (e.g. the repo's lint/test) and \
                 yields a pass/fail verdict. Returns one result per rule; a rule with no matching command \
                 for this repo is 'skipped'. Makes adopted rules enforceable, not advisory-only.",
                &[],
                &[
                    ("folder",  "string", "Absolute repo path to check. Defaults to the current project's folder."),
                    ("project", "string", "Project name or UUID instead of a path."),
                ]),
            // ── Relay run-control (P3.8) ─────────────────────────────────────
            tool("start_run",
                "Start a daemon-owned autonomous run against a goal (the relay engine). The daemon \
                 tracks it durably (survives restarts), pauses/auto-resumes on provider limits, and \
                 recovers from stalls — watch it with `run_status`. Note: whether it actually drives an \
                 agent depends on the daemon's OFF-by-default drive switch; creating the run is always safe. \
                 The response includes `track_url` (when a Dōjō is connected) — the auth-gated link to \
                 watch this run in the Dōjō; surface it to the user as the handoff.",
                &[("goal", "string", "What the run should accomplish — the objective it's anchored to")],
                &[
                    ("project",  "string", "Project name or UUID the run works in. Defaults to the current project."),
                    ("plan_ref", "string", "Path/ref of the committed plan doc the run executes (e.g. docs/plan/x.md)"),
                ]),
            tool("run_status",
                "Show daemon-owned autonomous runs. Without a run_id: lists the active runs \
                 (running / paused / stalled / blocked). With a run_id: returns that run plus its recent \
                 cadence events (the filtered feed — phase/feature/gate/commit markers, no code).",
                &[],
                &[("run_id", "string", "A specific run's UUID. Omit to list all active runs.")]),
            tool("pause_run",
                "Pause a run until a usage/rate limit resets — marks it 'paused' (not stalled) and the \
                 daemon auto-resumes it at the reset. Call this when you hit (or foresee) a limit so the \
                 watcher sees a resumable wait, not a stall. Defaults to the active run for the current project.",
                &[("until", "string", "RFC-3339 timestamp when the limit resets (e.g. 2026-07-25T11:30:00-05:00).")],
                &[
                    ("reason", "string", "Human-readable cause (e.g. 'usage limit', 'weekly cap')."),
                    ("project", "string", "Project name or UUID; defaults to the current project."),
                    ("run_id", "string", "A specific run's UUID; defaults to the active run for the project."),
                ]),
            // ── Automated-run coordinator contract (AR-3) ────────────────────
            tool("register_plan",
                "Register an authored plan GRAPH as a daemon-owned run and mirror it to Dōjō (phases → \
                 tasks, each carrying its assigned agent + model + a spec ref). The daemon validates the \
                 graph (unique task ids, deps resolve, no cycles), stores it, and authors the phone \
                 outline from it so the whole plan is watchable before execution. Pass `plan` as a JSON \
                 string: {\"goal\"?, \"phases\":[{\"title\", \"tasks\":[{\"id\", \"title\", \"agent\"?, \
                 \"model\"?, \"spec_ref\"?, \"deps\"?:[id], \"summary\"?}]}]}. Returns the run plus \
                 `track_url` (when a Dōjō is connected) — the auth-gated link to watch the plan/run in the \
                 Dōjō; surface it to the user as the handoff.",
                &[
                    ("goal", "string", "The run's objective — a short label; the plan graph carries the detail"),
                    ("plan", "string", "The plan graph as a JSON string (phases → tasks with agent/model/spec_ref/deps)"),
                ],
                &[
                    ("project", "string", "Project name or UUID the run works in. Defaults to the current project."),
                    ("plan_ref", "string", "Path to the committed human plan doc (e.g. docs/plan/<id>/plan.md)."),
                    ("max_concurrency", "string", "Max parallel tasks (integer as a string). Defaults to 1."),
                ]),
            tool("update_task_status",
                "Flip one plan task's state as the executor works the graph. Re-projects the task's Dōjō \
                 segment and feeds the run's progress clock — call it when a task goes active / done / \
                 failed / blocked. Never touches the liveness heartbeat (that stays the daemon's).",
                &[
                    ("run_id", "string", "The run's UUID (from register_plan)."),
                    ("task_id", "string", "The task's id within the plan graph."),
                    ("state", "string", "New state: pending | active | done | skipped | failed | blocked | needs_review."),
                ],
                &[("note", "string", "Optional one-line note (no code) recorded on the cadence event.")]),
            tool("report_run_outcome",
                "Mark a run terminal — done or failed — when the plan is complete (or unrecoverable). The \
                 one terminal transition an external coordinator may set; the daemon watchdog keeps its \
                 own independent stall/crash authority.",
                &[
                    ("run_id", "string", "The run's UUID."),
                    ("outcome", "string", "'done' or 'failed'."),
                ],
                &[("summary", "string", "Optional one-line outcome summary (no code).")]),
            tool("get_pending_nudges",
                "Pull the pending human→agent steer for a run from Dōjō — the 'check in' pull side of the \
                 contract. Poll it each executor loop and act on any nudge/chat the human sent. Read-only \
                 and fail-soft (empty list if there's no dojo or the poll fails). Steer, not drive.",
                &[("run_id", "string", "The run's UUID.")],
                &[]),
            // ── Front-door intake ────────────────────────────────────────────
            tool("get_intake_guide",
                "Load the intake guide (grounding frame + per-axis elicitation prompts + the playbook \
                 catalog) to run /sensei:intake. Call at the start of an intake before asking the user anything.",
                &[],
                &[]),
            tool("recommend_playbook",
                "Recommend a playbook for the current work chunk from its lifecycle/intent/risk. \
                 Call after the intake dialogue has classified the chunk. Returns playbook + rationale.",
                &[
                    ("lifecycle", "string", "greenfield | stable"),
                    ("intent",    "string", "explore | ux | feature | enhancement | bug"),
                    ("risk",      "string", "low | high (blast-radius)"),
                ],
                &[
                    ("session_id", "string", "session UUID to attribute the run to"),
                    ("feature",    "string", "feature slug when the chunk maps to a dossier"),
                    ("confirm",    "string", "true to record the run as confirmed"),
                    ("project",    "string", "project name or UUID — enables suggested_skills/agents from the libraries it uses (defaults to the cwd-resolved project)"),
                ]),
            // ── §9 learning loop: accept path ─────────────────────────────────
            tool("list_playbook_rule_proposals",
                "List pending §9 learned-rule proposals (source='learned', not yet enabled) — new \
                 playbook rules the learning loop proposed from observed FTR outcomes. Review before accepting.",
                &[],
                &[]),
            tool("accept_playbook_rule",
                "Accept a §9 learned-rule proposal, flipping it enabled so the recommender starts using it.",
                &[("id", "string", "The proposal's rule UUID (from list_playbook_rule_proposals)")],
                &[]),
        ]
    })
}

pub fn tool(name: &str, description: &str, required: &[(&str, &str, &str)], optional: &[(&str, &str, &str)]) -> Value {
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

/// Pure: build the params object forwarded to the daemon's `/api/mcp/call`.
/// Forwards every caller arg, injects the resolved `repoId`, derives the search
/// term (`query` → else `name` → else `pattern`) and mirrors it to both `query`
/// and `q` (different daemon handlers read different keys), and renames
/// `pattern` → `tag` (the daemon's tag-filter key).
pub fn build_daemon_params(args: &Value, repo_id: &str) -> Value {
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
    daemon_params
}

/// Pure: pick the query param the daemon's `GET /api/knowledge/context`
/// accepts for the resolved project. An explicit `project_id` (a UUID the
/// caller passed) is forwarded as `project_id`; otherwise the resolved project
/// **name** is sent as `project` (the daemon resolves names → UUIDs, mirroring
/// `get_commands`). Returns `None` when neither is known.
///
/// The original bug lived here: the name was sent as `project_id`, which the
/// daemon's uuid-only handler rejected with HTTP 400.
pub fn context_project_param<'a>(explicit_pid: &'a str, repo_id: &'a str) -> Option<(&'static str, &'a str)> {
    if !explicit_pid.is_empty() {
        Some(("project_id", explicit_pid))
    } else if !repo_id.is_empty() {
        Some(("project", repo_id))
    } else {
        None
    }
}

/// Pure: pick the query param the daemon's `GET /api/knowledge/rules` accepts.
/// Prefer the resolved project **name** (`project`) so the daemon resolves the
/// governing folder itself; fall back to `folder=<cwd>` only when no project
/// resolved. The original bug always sent the MCP process's own `cwd`, which is
/// not the repo path.
pub fn rules_query_param<'a>(repo_id: &'a str, cwd: &'a str) -> (&'static str, &'a str) {
    if !repo_id.is_empty() {
        ("project", repo_id)
    } else {
        ("folder", cwd)
    }
}

/// Pure: map an MCP tool name to the daemon's internal tool name. Most pass
/// through unchanged; `get_patterns` is the daemon's `get_file_tags`.
pub fn map_daemon_tool(tool_name: &str) -> &str {
    match tool_name {
        "get_patterns" => "get_file_tags",
        other => other,
    }
}

// ── Active-project pin (in-memory, session-scoped) ───────────────────────────
//
// The MCP server process's cwd is fixed at launch and is usually NOT the repo,
// so cwd-based resolution can say "no project resolved". `use_project` sets an
// IN-MEMORY pin (`ACTIVE_PIN` in the binary) so a chosen project survives across
// every subsequent tool call regardless of cwd; the default resolver
// (`resolve_default_project`) consults it after an explicit `project=` arg and
// cwd. The pin is NOT persisted to a file — it lives only for this session (the
// MCP is one stdio process per session), so it can never leak across
// sessions/repos: the #109 stale-pin misroute is unrepresentable by design.

/// The pinned active project (`{id, name}`), held in memory for the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveProject {
    pub id: String,
    pub name: String,
}

/// The default-project decision for a project-scoped tool (no explicit `project=`).
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectResolution {
    /// A project was chosen. `source` is how it was picked (`"explicit"` | `"cwd"` | `"pin"`);
    /// `note` carries a stale-pin warning when a resolved cwd overrode a *conflicting* pin —
    /// surfaced to the caller so a mis-resolution is never silent.
    Resolved { name: String, source: &'static str, note: Option<String> },
    /// No explicit arg, no cwd match, no pin — the caller must name the project.
    Unresolved,
}

/// Pure: choose the default project for a project-scoped tool. Precedence:
///   1. **explicit** — a non-empty `project=` arg (strongest; a per-call override).
///   2. **cwd** — the project the WORKING DIR resolves to. This **beats the session `pin`** when
///      they disagree: being *in* a repo is the strongest current signal. (The pin is now an
///      in-memory *session* value, so it can no longer be *stale across sessions*; respecting cwd
///      on a conflict still keeps a mid-session `use_project` from masking the repo you're in.)
///      The override is carried in `note`, never applied silently.
///   3. **pin** — the fallback when the cwd resolves nothing (e.g. launched from a container dir
///      that is not itself a project); this is what `use_project` exists for.
///   4. otherwise **Unresolved** — the caller must pass `project=` or run `use_project`.
pub fn resolve_default_project(
    explicit: Option<&str>,
    pin: Option<&ActiveProject>,
    cwd_name: &str,
) -> ProjectResolution {
    if let Some(e) = explicit.filter(|s| !s.is_empty()) {
        return ProjectResolution::Resolved { name: e.to_string(), source: "explicit", note: None };
    }
    let cwd = (!cwd_name.trim().is_empty()).then_some(cwd_name);
    let pin_name = pin.map(|p| p.name.as_str()).filter(|n| !n.is_empty());
    match (cwd, pin_name) {
        (Some(c), Some(p)) if c != p => ProjectResolution::Resolved {
            name: c.to_string(),
            source: "cwd",
            note: Some(format!(
                "working dir resolves to '{c}' but the pinned project is '{p}' — using '{c}'. \
                 Run use_project to update the pin, or pass project= to override."
            )),
        },
        (Some(c), _) => ProjectResolution::Resolved { name: c.to_string(), source: "cwd", note: None },
        (None, Some(p)) => ProjectResolution::Resolved { name: p.to_string(), source: "pin", note: None },
        (None, None) => ProjectResolution::Unresolved,
    }
}

/// Pure: find the project row matching `hint` among `/api/projects` rows.
/// Matching order:
///   1. Exact `id` (UUID) match
///   2. Exact `name` match (case-insensitive)
///   3. Partial `name` match (`contains`, case-insensitive)
///
/// Shared by the name-only [`resolve_project_in`] and the `{id,name}`
/// [`resolve_active_project_in`] so the match order is defined exactly once.
fn find_project<'a>(projects: &'a [Value], hint: &str) -> Option<&'a Value> {
    let hint_lower = hint.to_lowercase();

    // 1. Exact id match
    if let Some(p) = projects.iter().find(|p| p["id"].as_str() == Some(hint)) {
        return Some(p);
    }
    // 2. Exact name match (case-insensitive)
    if let Some(p) = projects.iter()
        .find(|p| p["name"].as_str().map(|n| n.to_lowercase()) == Some(hint_lower.clone()))
    {
        return Some(p);
    }
    // 3. Partial name match
    projects.iter()
        .find(|p| p["name"].as_str().map(|n| n.to_lowercase().contains(&hint_lower)) == Some(true))
}

/// Pure function: resolve a project hint to a project **name** from a slice of
/// `/api/projects` rows (see [`find_project`] for the match order). Returns
/// `None` when nothing matches.
pub fn resolve_project_in(projects: &[Value], hint: &str) -> Option<String> {
    find_project(projects, hint).and_then(|p| p["name"].as_str().map(str::to_string))
}

/// Pure function: resolve a hint to the matching project's `{id, name}` — what
/// `use_project` pins. Same match order as [`resolve_project_in`]. Returns
/// `None` when nothing matches or the row has no `name`.
pub fn resolve_active_project_in(projects: &[Value], hint: &str) -> Option<ActiveProject> {
    let p = find_project(projects, hint)?;
    let name = p["name"].as_str()?.to_string();
    let id = p["id"].as_str().unwrap_or_default().to_string();
    Some(ActiveProject { id, name })
}

/// Pure function: resolve a project name from `cwd` by matching against each
/// project's `folders[].abs_path`.  Picks the project whose folder has the
/// longest `abs_path` that is a prefix of `cwd`.  Returns "" if no match.
pub fn resolve_from_cwd_in(projects: &[Value], cwd: &str) -> String {
    let mut best_name = String::new();
    let mut best_len = 0usize;

    for p in projects {
        if let Some(folders) = p["folders"].as_array() {
            for folder in folders {
                if let Some(abs_path) = folder["abs_path"].as_str()
                    && cwd.starts_with(abs_path) && abs_path.len() > best_len {
                    best_len = abs_path.len();
                    best_name = p["name"].as_str().unwrap_or("").to_string();
                }
            }
        }
    }

    best_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Vec<serde_json::Value> {
        vec![json!({
            "id": "11111111-1111-1111-1111-111111111111",
            "name": "sensei",
            "folders": [
                {"abs_path": "/Users/x/dev/sensei", "name": "sensei"},
                {"abs_path": "/Users/x/dev/sensei/crates/senseid", "name": "senseid"}
            ]
        })]
    }

    #[test]
    fn resolve_project_matches_by_name_not_empty() {
        assert_eq!(resolve_project_in(&sample(), "sensei"), Some("sensei".into()));
        assert_eq!(resolve_project_in(&sample(), "SENSEI"), Some("sensei".into()));
        assert_eq!(resolve_project_in(&sample(), "sens"), Some("sensei".into())); // partial
        assert_eq!(
            resolve_project_in(&sample(), "11111111-1111-1111-1111-111111111111"),
            Some("sensei".into())
        ); // by id
        assert_eq!(resolve_project_in(&sample(), "nope"), None);
    }

    #[test]
    fn resolve_from_cwd_matches_longest_folder_prefix() {
        // deeper path → senseid folder wins, but both belong to same project "sensei"
        assert_eq!(
            resolve_from_cwd_in(&sample(), "/Users/x/dev/sensei/crates/senseid/src"),
            "sensei".to_string()
        );
        assert_eq!(resolve_from_cwd_in(&sample(), "/tmp/other"), "".to_string());
    }

    #[test]
    fn resolve_project_exact_name_wins_over_partial() {
        let projects = vec![
            json!({ "id": "a", "name": "sensei-web", "folders": [] }),
            json!({ "id": "b", "name": "sensei",     "folders": [] }),
        ];
        // Exact "sensei" must return "sensei", not the partial-matching "sensei-web".
        assert_eq!(resolve_project_in(&projects, "sensei"), Some("sensei".into()));
    }

    #[test]
    fn resolve_from_cwd_no_folders_is_empty() {
        let projects = vec![json!({ "id": "a", "name": "x" })]; // no folders key
        assert_eq!(resolve_from_cwd_in(&projects, "/anywhere"), "".to_string());
    }

    #[test]
    fn resolve_from_cwd_with_root_folders_only() {
        // The daemon now sends only repo-root folders (kind git|standalone) on
        // the `?under=` find_projects path — the nested `kind:'folder'`
        // descendants (e.g. `.../crates/senseid`) are dropped to stay under the
        // MCP client's token cap. cwd→project resolution must still work off the
        // repo root alone: a deep cwd `starts_with` the root, so longest-prefix
        // still lands the project.
        let projects = vec![json!({
            "id": "11111111-1111-1111-1111-111111111111",
            "name": "sensei",
            // Only the repo root — NO descendant folder rows.
            "folders": [
                {"abs_path": "/Users/x/dev/sensei", "kind": "git", "name": "sensei"}
            ]
        })];
        assert_eq!(
            resolve_from_cwd_in(&projects, "/Users/x/dev/sensei/crates/senseid/src/api"),
            "sensei".to_string(),
            "a deep cwd must still resolve to the project via its repo-root folder alone",
        );
        assert_eq!(
            resolve_from_cwd_in(&projects, "/Users/x/dev/sensei"),
            "sensei".to_string(),
            "the repo root itself must resolve",
        );
        assert_eq!(
            resolve_from_cwd_in(&projects, "/Users/x/dev/other"),
            "".to_string(),
            "a cwd outside the root must not resolve",
        );
    }

    // ── MCP↔daemon seam: the knowledge params the proxy sends ─────────────
    // These pin the request SHAPE that the (fixed) daemon accepts. They go RED
    // on the original bug, which sent the project NAME as `project_id` (context)
    // and the process `cwd` as `folder` (rules).

    #[test]
    fn context_param_sends_project_name_not_project_id() {
        // Resolved-from-cwd/hint: `repo_id` is a NAME → send it as `project`
        // (the daemon resolves names). Sending it as `project_id` was the bug.
        assert_eq!(context_project_param("", "sensei"), Some(("project", "sensei")));
        // An explicit uuid passes straight through as `project_id`.
        assert_eq!(
            context_project_param("11111111-1111-1111-1111-111111111111", "sensei"),
            Some(("project_id", "11111111-1111-1111-1111-111111111111")),
        );
        // Nothing resolved → caller returns a "no project" error.
        assert_eq!(context_project_param("", ""), None);
    }

    #[test]
    fn rules_param_prefers_project_name_over_cwd() {
        // Fixed: send the resolved project NAME so the daemon finds the repo.
        assert_eq!(rules_query_param("sensei", "/mcp/proc/cwd"), ("project", "sensei"));
        // Fall back to folder=cwd ONLY when no project resolved.
        assert_eq!(rules_query_param("", "/mcp/proc/cwd"), ("folder", "/mcp/proc/cwd"));
    }

    // ── Tool catalog contract ────────────────────────────────────────────

    /// Every tool the daemon's mcp_call_tool / direct endpoints dispatch on.
    /// Keep in sync with handle_list_tools — a missing entry means the tool is
    /// advertised but untested, an extra entry means it's tested but unadvertised.
    const EXPECTED_TOOLS: &[&str] = &[
        "search", "context_pack", "get_callers", "get_callees", "get_project_summary",
        "get_lib_docs", "search_lib_docs", "list_library_skills", "get_library_skill", "list_library_agents", "get_communities", "get_patterns",
        "list_projects", "find_projects", "get_user_for_project", "set_stance", "use_project", "create_session", "update_session", "add_library",
        "update_phase", "get_workflow_state", "match_pattern", "get_pattern_for",
        "get_duplicates", "get_project_conventions", "resolve_risk_class", "get_rules", "get_commands", "infer", "embed",
        "gateway_status", "consensus", "generate_image", "log_event",
        "propose_memory", "save_memory", "promote_memory", "accept_proposal",
        "reject_proposal", "record_outcome", "get_layered_context",
        "plan", "run_checkers", "start_run", "run_status", "pause_run", "recommend_playbook", "get_intake_guide",
        "register_plan", "update_task_status", "report_run_outcome", "get_pending_nudges",
        "list_playbook_rule_proposals", "accept_playbook_rule",
    ];

    fn tools() -> Vec<Value> {
        handle_list_tools()["tools"].as_array().unwrap().clone()
    }
    fn tool_named<'a>(ts: &'a [Value], name: &str) -> &'a Value {
        ts.iter().find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("tool '{name}' not in catalog"))
    }

    #[test]
    fn catalog_exposes_exactly_the_expected_tools() {
        let ts = tools();
        let mut names: Vec<&str> = ts.iter().filter_map(|t| t["name"].as_str()).collect();
        for expected in EXPECTED_TOOLS {
            assert!(names.contains(expected), "catalog missing tool: {expected}");
        }
        // No duplicates, and nothing advertised that isn't in the expected set.
        names.sort();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(deduped.len(), names.len(), "duplicate tool names in catalog");
        for name in &names {
            assert!(EXPECTED_TOOLS.contains(name), "undeclared tool advertised: {name}");
        }
    }

    #[test]
    fn every_tool_has_a_well_formed_schema() {
        for t in tools() {
            let name = t["name"].as_str().unwrap_or("");
            assert!(!name.is_empty(), "a tool is missing its name");
            assert!(
                t["description"].as_str().map(|d| d.len() > 10).unwrap_or(false),
                "{name}: description missing or too short"
            );
            assert_eq!(t["inputSchema"]["type"], "object", "{name}: schema not an object");
            assert!(t["inputSchema"]["properties"].is_object(), "{name}: no properties map");
            let required = t["inputSchema"]["required"].as_array()
                .unwrap_or_else(|| panic!("{name}: required is not an array"));
            // Every required param must also be declared in properties.
            for r in required {
                let rn = r.as_str().unwrap();
                assert!(
                    t["inputSchema"]["properties"][rn].is_object(),
                    "{name}: required param '{rn}' absent from properties"
                );
            }
        }
    }

    #[test]
    fn lib_docs_tools_declare_their_documented_params() {
        let ts = tools();
        let gld = tool_named(&ts, "get_lib_docs");
        assert_eq!(gld["inputSchema"]["required"], json!(["name"]), "get_lib_docs requires name");
        assert!(
            gld["inputSchema"]["properties"]["component"].is_object(),
            "get_lib_docs must offer an optional 'component'"
        );
        let sld = tool_named(&ts, "search_lib_docs");
        assert_eq!(sld["inputSchema"]["required"], json!(["query"]), "search_lib_docs requires query");
    }

    #[test]
    fn update_session_requires_session_id_and_outcome() {
        let ts = tools();
        let req = tool_named(&ts, "update_session")["inputSchema"]["required"].clone();
        assert_eq!(req, json!(["sessionId", "outcome"]));
    }

    #[test]
    fn tool_helper_lists_only_required_in_required_array() {
        let t = tool("demo", "a demo tool description", &[("a", "string", "the a")], &[("b", "string", "the b")]);
        assert_eq!(t["name"], "demo");
        assert_eq!(t["inputSchema"]["properties"]["a"]["type"], "string");
        assert_eq!(t["inputSchema"]["properties"]["b"]["type"], "string");
        assert_eq!(t["inputSchema"]["required"], json!(["a"]), "optional must not be required");
    }

    #[test]
    fn initialize_reports_protocol_and_server_info() {
        let init = handle_initialize();
        assert_eq!(init["protocolVersion"], "2024-11-05");
        assert_eq!(init["serverInfo"]["name"], "sensei");
        assert!(init["capabilities"]["tools"].is_object());
    }

    // ── Daemon param building ────────────────────────────────────────────

    #[test]
    fn build_daemon_params_injects_repo_id_and_mirrors_query() {
        let p = build_daemon_params(&json!({ "query": "foo" }), "my-repo");
        assert_eq!(p["repoId"], "my-repo");
        assert_eq!(p["query"], "foo");
        assert_eq!(p["q"], "foo", "daemon search handlers read 'q'");
    }

    #[test]
    fn build_daemon_params_derives_query_from_name_then_pattern() {
        // name fills query when query is absent
        assert_eq!(build_daemon_params(&json!({ "name": "Foo" }), "r")["query"], "Foo");
        // explicit query wins over name
        assert_eq!(build_daemon_params(&json!({ "query": "q", "name": "n" }), "r")["query"], "q");
        // pattern is the last fallback
        assert_eq!(build_daemon_params(&json!({ "pattern": "hook" }), "r")["query"], "hook");
    }

    #[test]
    fn build_daemon_params_renames_pattern_to_tag() {
        let p = build_daemon_params(&json!({ "pattern": "hook" }), "r");
        assert_eq!(p["tag"], "hook", "pattern becomes the daemon 'tag' key");
        assert!(p.get("pattern").is_none(), "raw 'pattern' must be removed");
    }

    #[test]
    fn build_daemon_params_preserves_other_args_and_defaults_query_empty() {
        let p = build_daemon_params(&json!({ "task": "build X", "outcome": "completed" }), "r");
        assert_eq!(p["task"], "build X");
        assert_eq!(p["outcome"], "completed");
        assert_eq!(p["query"], "", "no query/name/pattern → empty search term");
        assert_eq!(p["q"], "");
    }

    #[test]
    fn map_daemon_tool_aliases_get_patterns_and_passes_through() {
        assert_eq!(map_daemon_tool("get_patterns"), "get_file_tags");
        assert_eq!(map_daemon_tool("search"), "search");
        assert_eq!(map_daemon_tool("get_lib_docs"), "get_lib_docs");
    }

    // ── daemon_request_for: per-tool request SHAPE ───────────────────────
    // These pin what the proxy sends. A renamed path, a changed query key, or a
    // dropped tool trips these; the in-process contract test (crates/senseid/
    // tests/mcp_contract.rs) then proves the daemon actually accepts the shape.

    /// Convenience: find a query value by key.
    fn q<'a>(req: &'a DaemonRequest, key: &str) -> Option<&'a str> {
        req.query.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    #[test]
    fn gateway_and_noop_tools_are_handled_inline() {
        for t in ["infer", "embed", "gateway_status", "consensus", "generate_image", "log_event"] {
            assert_eq!(
                daemon_request_for(t, &json!({}), "/cwd", Some("sensei")), None,
                "{t} must be binary-handled (None), not shaped as a plain daemon request"
            );
        }
    }

    #[test]
    fn context_request_sends_project_name_as_project_query() {
        // The Chunk-A bug shape: proxy must send ?project=<name>, NOT ?project_id=<name>.
        let req = daemon_request_for("get_layered_context", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "/api/knowledge/context");
        assert_eq!(q(&req, "project"), Some("sensei"), "name must ride on ?project=");
        assert_eq!(q(&req, "project_id"), None, "name must NOT be sent as project_id (the bug)");
    }

    #[test]
    fn context_request_forwards_explicit_uuid_limit_and_tags() {
        let req = daemon_request_for(
            "get_layered_context",
            &json!({ "project_id": "11111111-1111-1111-1111-111111111111", "limit": "50", "tags": "rust,api" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(q(&req, "project_id"), Some("11111111-1111-1111-1111-111111111111"),
            "explicit uuid overrides the resolved name");
        assert_eq!(q(&req, "project"), None);
        assert_eq!(q(&req, "limit"), Some("50"));
        assert_eq!(q(&req, "tags"), Some("rust,api"));
    }

    #[test]
    fn context_request_forwards_slot_and_feature_when_present() {
        // slot hint threads onto the daemon request as query params; absent →
        // absent (zero behavior change for every existing get_layered_context call).
        let req = daemon_request_for(
            "get_layered_context",
            &json!({ "slot": "brief", "feature": "auth" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(q(&req, "slot"), Some("brief"));
        assert_eq!(q(&req, "feature"), Some("auth"));

        let no_slot = daemon_request_for("get_layered_context", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(q(&no_slot, "slot"), None, "no slot arg → no slot query param");
        assert_eq!(q(&no_slot, "feature"), None, "no feature arg → no feature query param");
    }

    #[test]
    fn context_request_none_when_no_project_resolvable() {
        // No explicit project_id, nothing resolved → binary returns a guidance error.
        assert_eq!(daemon_request_for("get_layered_context", &json!({}), "/cwd", None), None);
    }

    #[test]
    fn rules_request_sends_project_name_not_folder() {
        let req = daemon_request_for("get_rules", &json!({}), "/mcp/cwd", Some("sensei")).unwrap();
        assert_eq!(req.path, "/api/knowledge/rules");
        assert_eq!(q(&req, "project"), Some("sensei"), "resolved name rides on ?project=");
        assert_eq!(q(&req, "folder"), None);
        // Fallback: no project resolved → ?folder=<cwd> (the historical contract).
        let fb = daemon_request_for("get_rules", &json!({}), "/mcp/cwd", None).unwrap();
        assert_eq!(q(&fb, "folder"), Some("/mcp/cwd"));
        assert_eq!(q(&fb, "project"), None);
    }

    #[test]
    fn log_event_body_maps_capture_fields_for_any_assistant() {
        // Tool-call capture from a NON-Claude assistant → the hook-event shape the
        // ingest reads, tagged with its family, so the analyzer treats it like a hook.
        let b = build_log_event_body(&json!({
            "type": "PostToolUse", "family": "cursor", "tool_name": "Edit",
            "session_id": "s1", "success": "true", "data": "{\"file\":\"x.rs\"}"
        }), "/mcp/cwd");
        assert_eq!(b["hook_event_name"], "PostToolUse");
        assert_eq!(b["assistant_family"], "cursor", "family carries through (was always 'claude')");
        assert_eq!(b["tool_name"], "Edit", "tool_name present → tally_tool_usage can count it");
        assert_eq!(b["session_id"], "s1");
        assert_eq!(b["exit_code"], 0, "success:true → exit_code 0 (hook_event_fields → success)");
        assert_eq!(b["file"], "x.rs", "data object is merged into the stored payload");
        assert_eq!(b["cwd"], "/mcp/cwd", "cwd falls back to the MCP working dir for attribution");

        // Workflow milestone with defaults: family→claude, explicit cwd, failure.
        let d = build_log_event_body(&json!({ "type": "phase_transition", "cwd": "/repo", "success": "false" }), "/mcp/cwd");
        assert_eq!(d["assistant_family"], "claude");
        assert_eq!(d["hook_event_name"], "phase_transition");
        assert_eq!(d["cwd"], "/repo");
        assert_eq!(d["exit_code"], 1);
        assert!(d.get("session_id").is_none(), "no session_id key when absent");
        assert!(d.get("tool_name").is_none(), "no tool_name key when absent");
    }

    #[test]
    fn run_checkers_omits_cwd_folder_when_project_is_given() {
        // #109: an explicit project must NOT also carry folder=cwd — the daemon
        // prefers the folder, and the MCP cwd mis-resolves to the container.
        let req = daemon_request_for("run_checkers", &json!({ "project": "sensei" }), "/mcp/cwd", Some("sensei")).unwrap();
        assert_eq!(req.path, "/api/checkers/run");
        let body = req.body.clone().unwrap();
        assert_eq!(body["project"], json!("sensei"));
        assert!(body.get("folder").is_none(), "no folder shadow when project is explicit");
        // No project → fall back to the cwd folder (historical contract).
        let fb = daemon_request_for("run_checkers", &json!({}), "/mcp/cwd", Some("sensei")).unwrap();
        let fbody = fb.body.clone().unwrap();
        assert_eq!(fbody["folder"], json!("/mcp/cwd"));
        assert!(fbody.get("project").is_none());
    }

    #[test]
    fn commands_request_targets_project_path_with_optional_category() {
        let req = daemon_request_for("get_commands", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "/api/projects/sensei/commands");
        assert!(req.query.is_empty(), "no category → no query");
        let filtered = daemon_request_for("get_commands", &json!({ "category": "test" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(q(&filtered, "category"), Some("test"));
    }

    #[test]
    fn start_run_posts_goal_with_resolved_project_and_optional_plan() {
        // project defaults to the resolved repo name (cwd→project convention).
        let req = daemon_request_for(
            "start_run",
            &json!({ "goal": "ship the relay engine" }),
            "/cwd",
            Some("sensei"),
        ).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/runs");
        let body = req.body.clone().unwrap();
        assert_eq!(body["goal"], "ship the relay engine");
        assert_eq!(body["project"], "sensei", "unset project defaults to the resolved repo");
        assert!(body.get("plan_ref").is_none(), "no plan_ref key when omitted");

        // An explicit project + plan_ref override / are forwarded.
        let full = daemon_request_for(
            "start_run",
            &json!({ "goal": "g", "project": "other", "plan_ref": "docs/plan/x.md" }),
            "/cwd",
            Some("sensei"),
        ).unwrap();
        let fb = full.body.unwrap();
        assert_eq!(fb["project"], "other", "explicit project wins over the resolved repo");
        assert_eq!(fb["plan_ref"], "docs/plan/x.md");

        // No project resolvable and none given → no project key (daemon makes a
        // project-less run rather than a 400).
        let noproj = daemon_request_for("start_run", &json!({ "goal": "g" }), "/cwd", None).unwrap();
        assert!(noproj.body.unwrap().get("project").is_none(), "no project anywhere → omit the key");
    }

    #[test]
    fn run_status_lists_all_or_one_by_id() {
        // No run_id → list active runs.
        let list = daemon_request_for("run_status", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(list.path, "/api/runs");
        // A run_id → that run + its events.
        let one = daemon_request_for(
            "run_status",
            &json!({ "run_id": "44444444-0000-0000-0000-000000000004" }),
            "/cwd",
            Some("sensei"),
        ).unwrap();
        assert_eq!(one.path, "/api/runs/44444444-0000-0000-0000-000000000004");
        assert!(one.body.is_none(), "a status read is a plain GET");
    }

    #[test]
    fn register_plan_forwards_parsed_graph_with_resolved_project() {
        let plan = r#"{"phases":[{"title":"P","tasks":[{"id":"t1","title":"x","agent":"general-purpose","model":"sonnet"}]}]}"#;
        let req = daemon_request_for(
            "register_plan",
            &json!({ "goal": "ship it", "plan": plan, "plan_ref": "docs/plan/x/plan.md" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/runs/plan");
        let body = req.body.unwrap();
        assert_eq!(body["goal"], "ship it");
        assert_eq!(body["project"], "sensei", "unset project defaults to the resolved repo");
        assert_eq!(body["plan_ref"], "docs/plan/x/plan.md");
        // The JSON-string plan is parsed into an object the daemon can validate.
        assert_eq!(body["plan"]["phases"][0]["tasks"][0]["id"], "t1");
        assert_eq!(body["plan"]["phases"][0]["tasks"][0]["model"], "sonnet");
    }

    #[test]
    fn task_status_outcome_and_nudges_target_run_subpaths() {
        // update_task_status → POST /api/runs/{id}/tasks/{task_id}
        let u = daemon_request_for(
            "update_task_status",
            &json!({ "run_id": "r1", "task_id": "t9", "state": "done", "note": "green" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(u.method, HttpMethod::Post);
        assert_eq!(u.path, "/api/runs/r1/tasks/t9");
        let ub = u.body.unwrap();
        assert_eq!(ub["state"], "done");
        assert_eq!(ub["note"], "green");

        // report_run_outcome → POST /api/runs/{id}/outcome
        let o = daemon_request_for(
            "report_run_outcome",
            &json!({ "run_id": "r1", "outcome": "failed", "summary": "gate red" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(o.path, "/api/runs/r1/outcome");
        assert_eq!(o.body.unwrap()["outcome"], "failed");

        // get_pending_nudges → GET /api/runs/{id}/nudges (read-only)
        let n = daemon_request_for("get_pending_nudges", &json!({ "run_id": "r1" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(n.method, HttpMethod::Get);
        assert_eq!(n.path, "/api/runs/r1/nudges");
        assert!(n.body.is_none());
    }

    #[test]
    fn duplicates_and_conventions_target_pattern_endpoints() {
        let d = daemon_request_for("get_duplicates", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(d.path, "/api/patterns/sensei/duplicates");
        assert_eq!(d.method, HttpMethod::Get);
        let c = daemon_request_for("get_project_conventions", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(c.path, "/api/patterns/sensei/conventions");
    }

    #[test]
    fn match_pattern_and_pattern_for_carry_their_args() {
        let m = daemon_request_for("match_pattern", &json!({ "description": "add SQL parsing" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(m.path, "/api/patterns/sensei/match");
        assert_eq!(q(&m, "description"), Some("add SQL parsing"));
        let pf = daemon_request_for("get_pattern_for", &json!({ "symbol": "SqlAdapter" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(pf.path, "/api/patterns/sensei/for/SqlAdapter");
    }

    #[test]
    fn workflow_state_get_and_put_target_state_endpoint() {
        let g = daemon_request_for("get_workflow_state", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(g.method, HttpMethod::Get);
        assert_eq!(g.path, "/api/state/sensei");
        let p = daemon_request_for(
            "update_phase",
            &json!({ "phase": "build", "task": "x", "issue": "42", "plan": "p", "checkpoint": "c" }),
            "/proj/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(p.method, HttpMethod::Put);
        assert_eq!(p.path, "/api/state/sensei");
        let body = p.body.unwrap();
        assert_eq!(body["active_phase"], "build");
        assert_eq!(body["active_issue"], 42, "issue string parses to an integer");
        assert_eq!(body["project_path"], "/proj/cwd", "cwd rides along as project_path");

        // An explicit `project` pins the state to that project (not the cwd-resolved
        // one) — the escape hatch against cross-session clobber when the cwd
        // mis-resolves (#109). Both verbs honour it.
        let pinned_get =
            daemon_request_for("get_workflow_state", &json!({ "project": "torii" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(pinned_get.path, "/api/state/torii", "explicit project overrides the cwd repo");
        let pinned_put = daemon_request_for(
            "update_phase",
            &json!({ "phase": "build", "project": "torii" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(pinned_put.path, "/api/state/torii", "update_phase honours explicit project");
    }

    #[test]
    fn search_falls_through_to_the_mcp_proxy() {
        let req = daemon_request_for("search", &json!({ "query": "foo" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/mcp/call");
        let body = req.body.unwrap();
        assert_eq!(body["tool"], "search");
        assert_eq!(body["params"]["repoId"], "sensei", "resolved name injected as repoId");
        assert_eq!(body["params"]["q"], "foo");
    }

    #[test]
    fn get_patterns_proxy_aliases_to_get_file_tags_and_renames_pattern_to_tag() {
        let req = daemon_request_for("get_patterns", &json!({ "pattern": "hook" }), "/cwd", Some("sensei")).unwrap();
        assert_eq!(req.path, "/api/mcp/call");
        let body = req.body.unwrap();
        assert_eq!(body["tool"], "get_file_tags", "daemon-side tool name");
        assert_eq!(body["params"]["tag"], "hook", "pattern renamed to tag");
        assert_eq!(body["params"]["repoId"], "sensei");
    }

    #[test]
    fn get_project_summary_proxies_with_repo_id() {
        let req = daemon_request_for("get_project_summary", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(req.path, "/api/mcp/call");
        let body = req.body.unwrap();
        assert_eq!(body["tool"], "get_project_summary");
        assert_eq!(body["params"]["repoId"], "sensei");
    }

    #[test]
    fn memory_writes_target_the_right_endpoints_and_bodies() {
        // propose → proposals; save → memories. project scope + resolved name → project_id.
        let prop = daemon_request_for(
            "propose_memory",
            &json!({ "scope": "project", "type": "convention", "title": "t", "content": "c",
                     "triage_signal": "revert", "tags": "rust, api" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(prop.method, HttpMethod::Post);
        assert_eq!(prop.path, "/api/knowledge/proposals");
        let pb = prop.body.unwrap();
        assert_eq!(pb["scope"], "project");
        assert_eq!(pb["project_id"], "sensei", "scope=project + resolved name → project_id=name");
        assert_eq!(pb["triage_signal"], "revert");
        assert_eq!(pb["tags"], json!(["rust", "api"]), "csv tags split + trimmed");

        let save = daemon_request_for(
            "save_memory",
            &json!({ "scope": "global", "type": "decision", "title": "t", "content": "c" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(save.path, "/api/knowledge/memories");
        assert!(save.body.unwrap().get("project_id").is_none(), "global scope → no project_id");

        // explicit project_id wins over resolved name.
        let explicit = daemon_request_for(
            "save_memory",
            &json!({ "scope": "project", "type": "decision", "title": "t", "content": "c",
                     "project_id": "22222222-2222-2222-2222-222222222222" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(explicit.body.unwrap()["project_id"], "22222222-2222-2222-2222-222222222222");
    }

    #[test]
    fn memory_write_attaches_governance_when_present() {
        let req = daemon_request_for(
            "save_memory",
            &json!({ "scope": "global", "type": "decision", "title": "t", "content": "c",
                     "gov_scope": "organization", "enforcement": "mandatory" }),
            "/proj/cwd", Some("sensei"),
        ).unwrap();
        let body = req.body.unwrap();
        assert_eq!(body["gov_scope"], "organization");
        assert_eq!(body["folder"], "/proj/cwd", "cwd forwarded as governing folder");
        assert_eq!(body["enforcement"], "mandatory");
    }

    #[test]
    fn memory_write_carries_spine_slot_and_feature_when_present() {
        // save_memory/propose_memory both route through build_memory_body; a
        // feature-scope slot (e.g. "brief") plus its feature must ride onto the
        // body unchanged so the daemon's insert_with_status can validate + persist
        // them (spine_slot/feature columns on sensei.memories).
        let req = daemon_request_for(
            "save_memory",
            &json!({ "scope": "project", "type": "decision", "title": "t", "content": "c",
                     "project_id": "sensei", "spine_slot": "brief", "feature": "auth" }),
            "/proj/cwd", Some("sensei"),
        ).unwrap();
        let body = req.body.unwrap();
        assert_eq!(body["spine_slot"], "brief");
        assert_eq!(body["feature"], "auth");

        // propose_memory carries the same fields.
        let prop = daemon_request_for(
            "propose_memory",
            &json!({ "scope": "global", "type": "decision", "title": "t", "content": "c",
                     "triage_signal": "revert", "spine_slot": "design" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        let pbody = prop.body.unwrap();
        assert_eq!(pbody["spine_slot"], "design");
        assert!(pbody.get("feature").is_none(), "no feature arg → no feature key");

        // Neither arg present → neither key present (zero behavior change).
        let plain = daemon_request_for(
            "save_memory",
            &json!({ "scope": "global", "type": "decision", "title": "t", "content": "c" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        let plain_body = plain.body.unwrap();
        assert!(plain_body.get("spine_slot").is_none(), "no spine_slot arg → no spine_slot key");
        assert!(plain_body.get("feature").is_none(), "no feature arg → no feature key");
    }

    #[test]
    fn proposal_lifecycle_requests_hit_id_scoped_endpoints() {
        let acc = daemon_request_for("accept_proposal", &json!({ "id": "abc" }), "/cwd", None).unwrap();
        assert_eq!(acc.path, "/api/knowledge/proposals/abc/accept");
        assert_eq!(acc.body, Some(json!({})));
        let rej = daemon_request_for("reject_proposal", &json!({ "id": "abc", "reason": "dup" }), "/cwd", None).unwrap();
        assert_eq!(rej.path, "/api/knowledge/proposals/abc/reject");
        assert_eq!(rej.body.unwrap()["reason"], "dup");
        let prom = daemon_request_for("promote_memory", &json!({ "id": "xyz", "gov_scope": "team" }), "/cwd", None).unwrap();
        assert_eq!(prom.path, "/api/knowledge/memories/xyz/promote");
        assert_eq!(prom.body.unwrap()["gov_scope"], "team");
    }

    #[test]
    fn promote_memory_prefers_project_over_cwd_folder() {
        // #109: an explicit project carries {project} into the gov overlay and NO
        // folder=cwd (which mis-resolved to the container → 400 "not an indexed repo").
        let req = daemon_request_for(
            "promote_memory",
            &json!({ "id": "m1", "gov_scope": "team", "project": "sensei" }),
            "/mcp/cwd", None).unwrap();
        assert_eq!(req.path, "/api/knowledge/memories/m1/promote");
        let body = req.body.clone().unwrap();
        assert_eq!(body["project"], json!("sensei"));
        assert_eq!(body["gov_scope"], json!("team"));
        assert!(body.get("folder").is_none(), "no cwd folder shadow when project is explicit");
        // No project → fall back to the cwd folder (historical contract).
        let fb = daemon_request_for(
            "promote_memory",
            &json!({ "id": "m1", "gov_scope": "team" }),
            "/mcp/cwd", None).unwrap();
        let fbody = fb.body.clone().unwrap();
        assert_eq!(fbody["folder"], json!("/mcp/cwd"));
        assert!(fbody.get("project").is_none());
    }

    #[test]
    fn record_outcome_wraps_the_outcomes_array() {
        let req = daemon_request_for(
            "record_outcome",
            &json!({ "outcomes": "[{\"memory_id\":\"m1\",\"outcome\":\"applied\"}]" }),
            "/cwd", None,
        ).unwrap();
        assert_eq!(req.path, "/api/knowledge/outcomes");
        let body = req.body.unwrap();
        assert_eq!(body["outcomes"][0]["memory_id"], "m1");
        assert_eq!(body["outcomes"][0]["outcome"], "applied");
    }

    #[test]
    fn unknown_tool_defaults_to_the_mcp_proxy() {
        // Behavior-preserving: an unrecognized tool still routes to /api/mcp/call
        // (the daemon returns its own "Unknown tool" error), never None.
        let req = daemon_request_for("totally_made_up", &json!({}), "/cwd", Some("sensei")).unwrap();
        assert_eq!(req.path, "/api/mcp/call");
        assert_eq!(req.body.unwrap()["tool"], "totally_made_up");
    }

    // ── Folder→project workflow: find_projects / use_project + the pin ───────

    #[test]
    fn find_projects_defaults_under_to_cwd() {
        // No `under` arg → scope to the MCP call's cwd (the whole point: the
        // assistant just wants "projects under here").
        let req = daemon_request_for("find_projects", &json!({}), "/my/cwd", None).unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "/api/projects");
        assert_eq!(q(&req, "under"), Some("/my/cwd"), "no `under` arg → default to call cwd");
    }

    #[test]
    fn find_projects_uses_explicit_under_over_cwd() {
        let req = daemon_request_for(
            "find_projects", &json!({ "under": "/some/dir" }), "/my/cwd", None,
        ).unwrap();
        assert_eq!(req.path, "/api/projects");
        assert_eq!(q(&req, "under"), Some("/some/dir"), "explicit `under` overrides cwd");
    }

    #[test]
    fn pause_run_posts_until_and_defaults_project_to_cwd() {
        let req = daemon_request_for(
            "pause_run", &json!({ "until": "2026-07-25T11:30:00Z", "reason": "usage limit" }),
            "/cwd", Some("sensei"),
        ).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/runs/pause");
        let body = req.body.as_ref().expect("pause_run has a JSON body");
        assert_eq!(body["until"], "2026-07-25T11:30:00Z");
        assert_eq!(body["project"], "sensei", "project defaults to the pinned/cwd repo");
        assert_eq!(body["reason"], "usage limit");
    }

    #[test]
    fn get_user_for_project_defaults_under_to_cwd() {
        // No `under` → resolve identity for the MCP call's cwd (same
        // folder-scoping as find_projects), hitting the daemon's /api/user.
        let req = daemon_request_for("get_user_for_project", &json!({}), "/my/cwd", None).unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "/api/user");
        assert_eq!(q(&req, "under"), Some("/my/cwd"), "no `under` arg → default to call cwd");
    }

    #[test]
    fn get_user_for_project_uses_explicit_under_over_cwd() {
        let req = daemon_request_for(
            "get_user_for_project", &json!({ "under": "/some/repo" }), "/my/cwd", None,
        ).unwrap();
        assert_eq!(req.path, "/api/user");
        assert_eq!(q(&req, "under"), Some("/some/repo"), "explicit `under` overrides cwd");
    }

    #[test]
    fn set_stance_posts_dials_and_defaults_under_to_cwd() {
        // Only-provided dials are sent; `under` defaults to cwd; no scope → default row.
        let req = daemon_request_for(
            "set_stance", &json!({ "autonomy": "run_freely" }), "/my/cwd", None,
        ).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/stance");
        let body = req.body.unwrap();
        assert_eq!(body["under"], json!("/my/cwd"), "under defaults to cwd");
        assert_eq!(body["autonomy"], json!("run_freely"));
        assert!(body.get("sharing").is_none(), "omitted dials are not sent (daemon keeps stored default)");
        assert!(body.get("scope").is_none(), "no scope → default row");
    }

    #[test]
    fn set_stance_forwards_scope_and_explicit_under() {
        let req = daemon_request_for(
            "set_stance",
            &json!({ "under": "/repo", "scope": "project", "review": "quorum" }),
            "/my/cwd", None,
        ).unwrap();
        let body = req.body.unwrap();
        assert_eq!(body["under"], json!("/repo"), "explicit under overrides cwd");
        assert_eq!(body["scope"], json!("project"));
        assert_eq!(body["review"], json!("quorum"));
    }

    #[test]
    fn use_project_is_handled_inline() {
        // The binary owns use_project (resolve + pin write); the lib shapes no
        // single daemon request, so daemon_request_for must return None.
        assert_eq!(
            daemon_request_for("use_project", &json!({ "project": "sensei" }), "/cwd", None),
            None
        );
    }

    #[test]
    fn resolve_active_project_returns_id_and_name() {
        let ap = resolve_active_project_in(&sample(), "sensei").unwrap();
        assert_eq!(ap.name, "sensei");
        assert_eq!(ap.id, "11111111-1111-1111-1111-111111111111");
        // by uuid resolves the same row
        assert_eq!(
            resolve_active_project_in(&sample(), "11111111-1111-1111-1111-111111111111").unwrap().name,
            "sensei"
        );
        assert!(resolve_active_project_in(&sample(), "nope").is_none());
    }

    #[test]
    fn default_project_explicit_then_cwd_beats_pin_then_pin_fallback_then_unresolved() {
        use ProjectResolution::*;
        let pin = ActiveProject { id: "id".into(), name: "pinned".into() };
        let resolved = |r: ProjectResolution| match r {
            Resolved { name, source, note } => (name, source, note),
            Unresolved => ("<unresolved>".into(), "none", None),
        };

        // explicit wins over pin AND cwd.
        assert_eq!(resolved(resolve_default_project(Some("explicit"), Some(&pin), "cwdname")).0, "explicit");

        // THE FIX: a resolved cwd beats a CONFLICTING pin (the stale cross-session pin that
        // returned sensei while working in torii) — and the override is surfaced in `note`,
        // never silent. (Old behaviour returned the pin here — that test codified the bug.)
        let (name, source, note) = resolved(resolve_default_project(None, Some(&pin), "torii"));
        assert_eq!((name.as_str(), source), ("torii", "cwd"));
        assert!(note.as_deref().unwrap().contains("pinned project is 'pinned'"), "stale-pin override is surfaced");

        // cwd + pin AGREE → cwd, no note (nothing to warn about).
        let agree = ActiveProject { id: "id".into(), name: "sensei".into() };
        assert_eq!(resolved(resolve_default_project(None, Some(&agree), "sensei")), ("sensei".into(), "cwd", None));

        // pin is the fallback ONLY when the cwd resolves nothing (e.g. a container dir).
        assert_eq!(resolved(resolve_default_project(None, Some(&pin), "")), ("pinned".into(), "pin", None));

        // cwd used when there's no pin at all.
        assert_eq!(resolved(resolve_default_project(None, None, "cwdname")), ("cwdname".into(), "cwd", None));

        // nothing resolves → Unresolved (caller must pass project= / use_project).
        assert_eq!(resolve_default_project(None, None, "  "), ProjectResolution::Unresolved);

        // empty explicit + empty-name pin are treated as absent.
        let empty = ActiveProject { id: "id".into(), name: String::new() };
        assert_eq!(resolved(resolve_default_project(Some(""), Some(&empty), "cwdname")), ("cwdname".into(), "cwd", None));
    }

}
