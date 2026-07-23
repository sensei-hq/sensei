use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use crate::api::state::AppState;

// ── Sessions ────────────────────────────────────────────────────────────────

/// Map a range chip to a day cutoff for the Observatory · Sessions digest.
/// `"7d"`/`"30d"`/`"90d"` → the day count; anything else (or absent) → `None`
/// (no time filter). Pure so it is unit-tested without a DB.
pub(crate) fn range_to_days(range: Option<&str>) -> Option<i64> {
    match range {
        Some("7d") => Some(7),
        Some("30d") => Some(30),
        Some("90d") => Some(90),
        _ => None,
    }
}

pub(crate) async fn get_sessions_stub(
    State(state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let range_days = range_to_days(q.get("range").map(String::as_str));
    // `?project=<name-or-uuid>` scopes the digest to one project (honours the
    // name-or-UUID contract). An unresolvable name yields None → no scope.
    let project = match q.get("project") {
        Some(p) => crate::api::util::resolve_project_uuid(&state, p).await,
        None => None,
    };
    // PgStore uses list_sessions_by_folder(&Uuid, limit) instead of get_sessions(repo_id)
    let sessions = if let Some(folder_str) = q.get("repoId") {
        if let Ok(folder_id) = uuid::Uuid::parse_str(folder_str) {
            state.pg.list_sessions_by_folder(&folder_id, 50).await.unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        // 500 comfortably covers the real corpus within any range window; range +
        // project narrow it. The digest aggregates these client-side per day.
        state.pg.list_all_sessions(500, range_days, project.as_ref()).await.unwrap_or_default()
    };
    let total = sessions.len();
    let completed = sessions.iter().filter(|s| s["outcome"].as_str() == Some("completed")).count();
    Json(serde_json::json!({
        "stats": { "totalSessions": total, "completed": completed },
        "sessions": sessions,
        "toolUsage": [],
        "benchmarkPairs": []
    }))
}

pub(crate) async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let folder_str = body["repoId"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("untitled");
    let acp_id = body["acpId"].as_str();

    let folder_id = match uuid::Uuid::parse_str(folder_str) {
        Ok(id) => id,
        Err(_) => return Json(serde_json::json!({"ok": false, "error": "invalid repoId (expected UUID)"})),
    };

    match state.pg.create_session(&folder_id, task, acp_id).await {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// GET /api/sessions/{id} — one session row by UUID. Powers hero.source and
/// recent-session link resolution on the Observatory · Today screen. Honors the
/// session-id contract: a well-formed UUID with no row yields 404, not a 500.
pub(crate) async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    match state.pg.get_session(&uuid).await {
        Ok(Some(row)) => Ok(Json(row)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!(error = %e, session = %id, "get_session failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sessions/{id}/tool-timeline — paired PreToolUse ↔ PostToolUse
/// timeline for one assistant session. Query param `limit` (default 200)
/// caps rows. `id` is the assistant string session id.
pub(crate) async fn get_session_tool_timeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if id.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let limit: i32 = q
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(200)
        .clamp(1, 1000);

    let calls = state
        .pg
        .get_session_tool_calls(&id, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, session = %id, "get_session_tool_calls failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "sessionId": id,
        "calls":     calls,
        "count":     calls.len(),
    })))
}

/// GET /api/sessions/{id}/replay — #84 T2 Slice C. Same paired timeline
/// as `tool-timeline`, but each row also carries the usage verdict from
/// `sensei.tool_call_verdicts` (#90). Rows with no verdict yet ship
/// `verdict: null` — the Replay tab decides whether to render a "—"
/// placeholder or trigger a classify pass.
///
/// If `?classify=true` is set, runs the verdict classifier for this
/// session before returning, so the caller doesn't need two round-trips
/// on first open.
pub(crate) async fn get_session_replay(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if id.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let limit: i32 = q
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(200)
        .clamp(1, 1000);
    let classify_first = q.get("classify").is_some_and(|v| v == "true" || v == "1");

    // Resolve the observatory session UUID → assistant-events client
    // session id. Callers (the app's Replay tab) pass `activity.sessions.id`,
    // but `activity.assistant_events.session_id` is the assistant's *own*
    // session identifier (the string the hook writer sends). Without this
    // lookup, `get_session_replay_timeline` reads 0 rows even for sessions
    // with hundreds of PostToolUse events (root cause of the "no tool calls
    // in this session" bug).
    //
    // If parsing `id` as UUID fails, fall back to treating it as an
    // already-resolved client_session_id — some callers already pass that
    // shape.
    let client_sid: String = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => {
            match state.pg.get_session_client_id(&uuid).await {
                Ok(Some(csid)) => csid,
                Ok(None) => {
                    tracing::warn!(session = %id, "replay: no session row for UUID; treating id as client_session_id");
                    id.clone()
                }
                Err(e) => {
                    tracing::error!(error = %e, session = %id, "replay: get_session_client_id failed");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(_) => id.clone(),
    };

    // Optionally classify first so verdicts are populated before the read.
    // Idempotent — refresh is safe on every open.
    let classified = if classify_first {
        crate::tasks::verdict_classifier::classify_session(&state.pg, &client_sid).await
            .map_err(|e| {
                tracing::error!(error = %e, session = %client_sid, "replay: classify_session failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        0
    };

    let calls = state
        .pg
        .get_session_replay_timeline(&client_sid, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, session = %client_sid, "get_session_replay_timeline failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let summary = state
        .pg
        .get_verdict_summary_for_session(&client_sid)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, session = %client_sid, "get_verdict_summary_for_session failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "sessionId":  id,
        "calls":      calls,
        "count":      calls.len(),
        "summary":    summary,
        "classified": classified,
    })))
}

pub(crate) async fn update_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let session_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => return Json(serde_json::json!({"ok": false, "error": "invalid session id (expected UUID)"})),
    };

    // PgStore complete_session expects: outcome, ftr (bool), turns (i32), corrections (i32)
    let outcome = body["outcome"].as_str().unwrap_or("completed");
    let ftr = body["ftr"].as_bool().unwrap_or(false);
    let turns = body["turns"].as_i64().unwrap_or(0) as i32;
    let corrections = body["corrections"].as_i64().unwrap_or(0) as i32;

    match state.pg.complete_session(&session_id, outcome, ftr, turns, corrections).await {
        Ok(_) => {
            // Fire-and-forget: enqueue verdict measurement after session ends
            let task = crate::tasks::Task::new(
                crate::tasks::TaskKind::MeasureVerdicts, "", "",
            );
            state.task_queue.enqueue(task).await;
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

// ── Hook event ingestion ────────────────────────────────────────────────────

/// Accepts a hook payload from any assistant and stores it in activity.assistant_events.
/// Called by hook scripts (sensei-hook.ts or equivalent) for every event type.
/// Returns 200 OK always — hook scripts must not block on errors.
pub(crate) async fn ingest_hook_event(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    let event_type       = payload["hook_event_name"].as_str().unwrap_or("unknown");
    let session_id       = payload["session_id"].as_str().unwrap_or("");
    let assistant_family = payload["assistant_family"].as_str().unwrap_or("claude");
    let tool_name        = payload["tool_name"].as_str();
    let cwd              = payload["cwd"].as_str();
    let ts               = chrono::Utc::now().timestamp_millis();
    let success          = payload.get("exit_code")
        .and_then(|v| v.as_i64())
        .map(|c| c == 0);

    // Always return 200 so a DB hiccup never blocks the hook — but DON'T
    // swallow the error silently: a failing capture insert is exactly how
    // capture dies invisibly (the bug the capture watchdog exists to catch).
    // Log it so it's inspectable in the daemon log / public.logs.
    if let Err(e) = state.pg.insert_hook_event(
        session_id, assistant_family, event_type, tool_name, cwd, ts, success, &payload,
    ).await {
        tracing::warn!(error = %e, event_type, assistant_family, "ingest_hook_event: insert failed");
    }

    // Relay segment-publish (A2): a TodoWrite carries the run's todo outline —
    // project it into the relay and push to enrolled dojos. Fire-and-forget so
    // the publish (a DB read + bounded HTTP posts) never blocks the hook.
    if tool_name == Some("TodoWrite") && !session_id.is_empty() {
        let task = crate::tasks::Task::new(
            crate::tasks::TaskKind::PublishRelaySegments, "", session_id,
        );
        state.task_queue.enqueue(task).await;
    }

    // Derive/maintain the activity.sessions row from the hook stream (#31).
    // A session is one assistant session_id, attributed to the indexed folder
    // its cwd resolves to; Stop/SessionEnd marks it completed. Best-effort —
    // events whose cwd is under no indexed folder simply aren't attributed.
    if !session_id.is_empty()
        && let Some(cwd) = cwd {
            match state.pg.find_folder_for_path(cwd).await {
                Ok(Some((folder_id, project_id))) => {
                    let is_end = matches!(event_type, "Stop" | "SessionEnd");
                    if let Err(e) = state.pg.record_session_event(
                        session_id, &folder_id, project_id.as_ref(), assistant_family, is_end,
                    ).await {
                        tracing::warn!(error = %e, event_type, "ingest_hook_event: record_session_event failed");
                    }
                }
                Ok(None) => {} // cwd not under any indexed folder — nothing to attribute
                Err(e) => tracing::warn!(error = %e, "ingest_hook_event: find_folder_for_path failed"),
            }
        }

    StatusCode::OK
}

// ── Hook gate (relay-engine feature B) ───────────────────────────────────────

/// Build a Claude Code `PreToolUse` hook decision body. `decision` is `"allow"`
/// or `"deny"`; `reason` is the human-facing explanation the hook surfaces.
fn gate_decision(decision: &str, reason: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": decision,
            "permissionDecisionReason": reason,
        }
    }))
}

/// `POST /hook/gate` — the daemon↔agent control leg (relay-engine feature B).
///
/// A Claude `PreToolUse` hook POSTs the tool-call payload here; the daemon
/// decides whether the tool may proceed. When the tool is in the
/// `SENSEI_RELAY_GATE_TOOLS` allow-list AND a Dōjō is enrolled, it raises a
/// blocking relay gate to the phone, waits (bounded ~50s) for the human's
/// answer, and returns allow/deny. Everything else — gating off, non-uuid
/// session, no dojo, any raise/poll error, timeout — is **fail-open → allow**.
///
/// ALWAYS returns 200 with an allow/deny body; NEVER a 5xx. Fail-open error
/// paths `tracing::warn!` (never swallowed). Only an explicit human `deny`
/// reply blocks. The payload raised to the phone carries only the tool NAME —
/// no tool args, code, or diffs (zero-knowledge, D10).
pub(crate) async fn hook_gate(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use crate::dojo::gate::{
        classify_hard_block, decision_from_reply, gated_tools_from_env, needs_semantic_review,
        parse_semantic_verdict, semantic_system_prompt, semantic_user_message, should_gate,
    };

    let session_id = payload["session_id"].as_str().unwrap_or("");
    let tool_name = payload["tool_name"].as_str().unwrap_or("");
    let cwd = payload["cwd"].as_str();
    // The PreToolUse event carries the tool's input alongside its name. Parse
    // defensively — a missing/malformed `tool_input` defaults to `{}`, which the
    // classifier treats as progress (no danger inferable ⇒ don't over-block).
    let tool_input = payload.get("tool_input").cloned().unwrap_or_else(|| serde_json::json!({}));

    // Two ways a call gates (relay-engine.md §5, P3.5):
    //  1. static allow-list — a tool named in SENSEI_RELAY_GATE_TOOLS, OR
    //  2. hard-block — a genuinely dangerous action (deploy / main-branch /
    //     destructive / credentials / money), even with NO allow-list set.
    // Everything else PROGRESSES (progress-over-asking). The hard-block only
    // RAISES a gate for a human to approve/deny — it never auto-denies.
    let gate_env = std::env::var("SENSEI_RELAY_GATE_TOOLS").ok();
    let gated = gated_tools_from_env(gate_env.as_deref());
    let mut hard_block = classify_hard_block(tool_name, &tool_input);

    // gemma4 semantic backstop (pre-drive hardening). The deterministic classifier
    // above is a literal-matching FLOOR; a determined obfuscation can evade it.
    // ORDER MATTERS: only when the deterministic verdict is `None` AND the command
    // carries an obfuscation/indirection marker do we ask gemma4 for a second
    // opinion — so the benign 99% keeps the byte-for-byte fast path (no model call).
    // FAIL-OPEN: timeout, gateway error, !success, or an unparseable/`dangerous:false`
    // answer all keep the deterministic verdict (progress). Only a parsed
    // `dangerous:true` becomes an effective hard-block. A gemma4 outage must never
    // start blocking everything and never stop the run.
    if hard_block.is_none()
        && let Some(cmd) = needs_semantic_review(tool_name, &tool_input)
    {
        use gateway::types::capability::Capability;
        use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};
        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: None,
            router: None,
            chain: Some("reasoning".into()),
            payload: Payload::Chat {
                messages: vec![Message::text(MessageRole::User, semantic_user_message(&cmd))],
                system: Some(semantic_system_prompt().to_string()),
                max_tokens: Some(200),
                temperature: None,
                tools: Vec::new(),
            },
            budget: None,
            auth: None,
            panel: None,
            consensus: None,
        };
        // Bound the call so a cold / wedged embedded inference can't hang the
        // PreToolUse gate. Timeout → fail-open (keep the deterministic verdict).
        let fut = state.gateway.execute(&request);
        match tokio::time::timeout(std::time::Duration::from_secs(8), fut).await {
            Ok(Ok(resp)) if resp.success => {
                match resp.content.as_deref().and_then(parse_semantic_verdict) {
                    Some(hb) => {
                        // A semantic backstop raised a gate. Zero-knowledge: log the
                        // tool + category only — NEVER the command text.
                        tracing::info!(
                            session_id,
                            tool_name,
                            category = hb.category,
                            "hook_gate: semantic backstop raised a gate"
                        );
                        hard_block = Some(hb);
                    }
                    None => {
                        // dangerous:false / unparseable → defer to progress.
                        tracing::warn!(
                            session_id,
                            tool_name,
                            "hook_gate: semantic backstop returned no block — progressing (fail-open)"
                        );
                    }
                }
            }
            Ok(Ok(_)) => tracing::warn!(
                session_id,
                tool_name,
                "hook_gate: semantic backstop response not successful — progressing (fail-open)"
            ),
            Ok(Err(e)) => tracing::warn!(
                session_id,
                tool_name,
                error = %e,
                "hook_gate: semantic backstop gateway error — progressing (fail-open)"
            ),
            Err(_) => tracing::warn!(
                session_id,
                tool_name,
                "hook_gate: semantic backstop timed out — progressing (fail-open)"
            ),
        }
    }

    if !should_gate(tool_name, &gated) && hard_block.is_none() {
        return gate_decision("allow", "not gated");
    }
    if let Some(hb) = &hard_block {
        tracing::info!(session_id, tool_name, category = hb.category, "hook_gate: hard-block classified — raising gate");
    }

    // Fail-open: a non-uuid session (non-Claude harness) would 500 the Worker
    // (relay run_id is uuid NOT NULL), so never gate it.
    if uuid::Uuid::parse_str(session_id).is_err() {
        tracing::warn!(session_id, tool_name, "hook_gate: non-uuid session — allowing (fail-open)");
        return gate_decision("allow", "gate unavailable — allowed");
    }

    // Fail-open: no enrolled Dōjō ⇒ nowhere to raise the gate ⇒ allow.
    let memberships = match state.pg.list_dojo_memberships().await {
        Ok(ms) => ms.into_iter().filter(|m| m.enabled).collect::<Vec<_>>(),
        Err(e) => {
            tracing::warn!(error = %e, "hook_gate: list_dojo_memberships failed — allowing (fail-open)");
            return gate_decision("allow", "gate unavailable — allowed");
        }
    };
    let Some(membership) = memberships.into_iter().next() else {
        return gate_decision("allow", "no dojo enrolled");
    };
    // FIRST enabled membership only (personal beta = one). Gating across
    // multiple memberships (which phone answers, quorum, races) is a tracked
    // follow-up — see relay-engine.md feature B.
    let client = crate::dojo::client::DojoClient::for_membership(&membership);

    // Ensure the cloud session exists so the phone can render the gate in
    // context (Running status; no segments — the gate carries its own prompt).
    let update = crate::dojo::relay_project::session_update(
        session_id,
        &crate::dojo::relay_project::title_from_cwd(cwd),
        &[],
    );
    if let Err(e) = client.publish_session_update(&update).await {
        tracing::warn!(session_id, error = %e, "hook_gate: session update failed — allowing (fail-open)");
        return gate_decision("allow", "gate unavailable — allowed");
    }

    // Raise the gate. Zero-knowledge: the payload names the tool and — for a
    // hard-block — the matched danger CATEGORY + REASON only. Never its
    // args/code/diffs (the reason is a fixed daemon-owned phrase, not command text).
    let gate_payload = match &hard_block {
        Some(hb) => serde_json::json!({
            "prompt": format!("Hard-block: {}. Approve {tool_name}?", hb.reason),
            "tool": tool_name,
            "category": hb.category,
            "reason": hb.reason,
        }),
        None => serde_json::json!({
            "prompt": format!("Approve {tool_name}?"),
            "tool": tool_name,
        }),
    };
    let item = dojo_protocol::relay::RelayInboxItem {
        id: None,
        run_id: session_id.to_string(),
        segment_id: None,
        kind: dojo_protocol::relay::RelayInboxKind::Approval,
        direction: dojo_protocol::relay::RelayMessageDirection::AgentToHuman,
        status: dojo_protocol::relay::RelayInboxStatus::Pending,
        payload: gate_payload,
        reply: None,
        created_at: None,
        answered_at: None,
    };
    let ack = match client.raise_inbox_item(&item).await {
        Ok(ack) => ack,
        Err(e) => {
            tracing::warn!(session_id, tool_name, error = %e, "hook_gate: raise failed — allowing (fail-open)");
            return gate_decision("allow", "gate unavailable — allowed");
        }
    };

    // Block (bounded < Claude's 60s hook cap) for the human's answer.
    match client
        .await_reply(&ack.id, ack.seq, std::time::Duration::from_secs(50))
        .await
    {
        Ok(reply) => {
            let decision = decision_from_reply(reply.as_ref());
            let reason = match (decision, reply.is_some()) {
                ("deny", _) => "declined from your phone",
                (_, true) => "approved from your phone",
                (_, false) => "gate timed out — allowed",
            };
            gate_decision(decision, reason)
        }
        Err(e) => {
            tracing::warn!(session_id, tool_name, error = %e, "hook_gate: await_reply failed — allowing (fail-open)");
            gate_decision("allow", "gate unavailable — allowed")
        }
    }
}

/// Sessions already nudged this process lifetime. Once-per-session guard so
/// an un-confirmed session gets suggested `/sensei:intake` a single time
/// instead of on every PreToolUse call (it would otherwise fire on every
/// tool call for the rest of the session — spammy, not a nudge). Mirrors the
/// `inflight()` idiom in `analysis::insight_copy` (`OnceLock` + `Mutex`,
/// poison recovered rather than panicking the daemon). In-process only —
/// resets on daemon restart, which is acceptable for an advisory nudge.
fn nudged_sessions() -> &'static std::sync::Mutex<std::collections::HashSet<uuid::Uuid>> {
    static NUDGED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<uuid::Uuid>>> =
        std::sync::OnceLock::new();
    NUDGED.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

/// POST /hook/nudge  { session_id }  ->  { nudge: bool, message?: string }
///
/// Non-blocking, informational only (unlike `hook_gate`, which can block a
/// tool call): suggests `/sensei:intake` when a session has started work
/// without a confirmed playbook run. Nudges **once per session** — a second
/// call for the same un-confirmed session returns `{nudge:false}` rather
/// than repeating (see [`nudged_sessions`]). **Fail-open** — mirrors
/// `hook_gate`'s posture: a missing/unparseable `session_id` or any DB error
/// yields `{nudge:false}` and never blocks. Registered in the sensei plugin's
/// `PreToolUse` hooks (activated 2026-07-19); `hooks/nudge` reshapes this
/// response into Claude Code's `additionalContext` so the suggestion surfaces.
pub(crate) async fn hook_nudge(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let Some(session_id) = payload["session_id"].as_str().and_then(|s| s.parse::<uuid::Uuid>().ok())
    else {
        return Json(serde_json::json!({ "nudge": false }));
    };

    match state.pg.session_has_confirmed_run(&session_id).await {
        Ok(true) => Json(serde_json::json!({ "nudge": false })),
        Ok(false) => {
            let mut nudged = nudged_sessions().lock().unwrap_or_else(|e| e.into_inner());
            if !nudged.insert(session_id) {
                // Already nudged this session — stay quiet on repeat calls.
                return Json(serde_json::json!({ "nudge": false }));
            }
            Json(serde_json::json!({
                "nudge": true,
                "message": "No playbook chosen for this chunk yet — consider /sensei:intake to pick one."
            }))
        }
        Err(e) => {
            tracing::warn!(error = %e, "hook_nudge: db error — fail-open (no nudge)");
            Json(serde_json::json!({ "nudge": false }))
        }
    }
}

// ── Workflow State ──────────────────────────────────────────────────────────

pub(crate) async fn get_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    match state.pg.get_workflow_state(&project).await {
        Ok(Some(ws)) => Json(ws),
        Ok(None) => Json(serde_json::json!({
            "project": project,
            "active_phase": null,
            "active_plan": null,
            "active_task": null,
            "active_issue": null,
            "last_checkpoint": null,
            "rules_hash": null,
        })),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

pub(crate) async fn update_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let result = state.pg.upsert_workflow_state(
        &project,
        body["active_phase"].as_str(),
        body["active_plan"].as_str(),
        body["active_task"].as_str(),
        body["active_issue"].as_i64(),
        body["last_checkpoint"].as_str(),
        body["rules_hash"].as_str(),
    ).await;

    if let Err(e) = result {
        return Json(serde_json::json!({"ok": false, "error": e}));
    }

    // Sync to .sensei/state.yaml
    // TODO: Add a lookup for folder abs_path by project name if needed.
    let project_path = body["project_path"].as_str().map(String::from);
    if let Some(project_path) = project_path {
        let sensei_dir = std::path::Path::new(&project_path).join(".sensei");
        std::fs::create_dir_all(&sensei_dir).ok();
        let state_file = sensei_dir.join("state.yaml");

        // Read back the state we just wrote to get all fields
        if let Ok(Some(ws)) = state.pg.get_workflow_state(&project).await {
            let yaml = format!(
                "active_phase: {}\nactive_plan: {}\nactive_task: {}\nactive_issue: {}\nlast_checkpoint: {}\nrules_hash: {}\n",
                ws["active_phase"].as_str().unwrap_or("~"),
                ws["active_plan"].as_str().unwrap_or("~"),
                ws["active_task"].as_str().unwrap_or("~"),
                ws["active_issue"].as_i64().map(|n| n.to_string()).unwrap_or("~".to_string()),
                ws["last_checkpoint"].as_str().unwrap_or("~"),
                ws["rules_hash"].as_str().unwrap_or("~"),
            );
            std::fs::write(&state_file, yaml).ok();
        }
    }

    Json(serde_json::json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::range_to_days;

    #[test]
    fn range_to_days_maps_known_chips() {
        assert_eq!(range_to_days(Some("7d")), Some(7));
        assert_eq!(range_to_days(Some("30d")), Some(30));
        assert_eq!(range_to_days(Some("90d")), Some(90));
    }

    #[test]
    fn range_to_days_none_for_unknown_or_absent() {
        assert_eq!(range_to_days(Some("1y")), None);
        assert_eq!(range_to_days(Some("")), None);
        assert_eq!(range_to_days(None), None);
    }

    // ── hook_nudge once-per-session guard ────────────────────────────────

    use super::hook_nudge;
    use crate::api::state::SharedState;
    use axum::extract::State;
    use axum::response::Json;
    use std::sync::Arc;

    async fn make_state() -> Option<super::AppState> {
        let queue = Arc::new(crate::tasks::queue::TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        Some(Arc::new(SharedState {
            task_queue: queue,
            pg,
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        }))
    }

    #[tokio::test]
    async fn hook_nudge_fires_once_per_unconfirmed_session() {
        let Some(state) = make_state().await else { return; };
        // Fresh session with no playbook_run row at all → session_has_confirmed_run
        // is Ok(false), so the first call should nudge.
        let session_id = uuid::Uuid::new_v4();
        let payload = serde_json::json!({ "session_id": session_id.to_string() });

        let Json(first) = hook_nudge(State(state.clone()), Json(payload.clone())).await;
        assert_eq!(first["nudge"], serde_json::json!(true));
        assert!(first["message"].as_str().is_some());

        // Second call for the SAME session must stay quiet even though the
        // session is still unconfirmed — the once-per-session guard, not the
        // DB state, suppresses it.
        let Json(second) = hook_nudge(State(state.clone()), Json(payload)).await;
        assert_eq!(second["nudge"], serde_json::json!(false));
    }

    #[tokio::test]
    async fn hook_nudge_missing_session_id_is_fail_open() {
        let Some(state) = make_state().await else { return; };
        let Json(body) = hook_nudge(State(state), Json(serde_json::json!({}))).await;
        assert_eq!(body["nudge"], serde_json::json!(false));
    }
}
