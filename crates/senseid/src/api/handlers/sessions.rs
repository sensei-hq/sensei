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
    use crate::dojo::gate::{decision_from_reply, gated_tools_from_env, should_gate};

    let session_id = payload["session_id"].as_str().unwrap_or("");
    let tool_name = payload["tool_name"].as_str().unwrap_or("");
    let cwd = payload["cwd"].as_str();

    // Gating is OFF by default: only gate a tool named in SENSEI_RELAY_GATE_TOOLS.
    let gate_env = std::env::var("SENSEI_RELAY_GATE_TOOLS").ok();
    let gated = gated_tools_from_env(gate_env.as_deref());
    if !should_gate(tool_name, &gated) {
        return gate_decision("allow", "not gated");
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

    // Raise the gate. Zero-knowledge: the payload names the tool only — never
    // its args/code/diffs.
    let item = dojo_protocol::relay::RelayInboxItem {
        id: None,
        run_id: session_id.to_string(),
        segment_id: None,
        kind: dojo_protocol::relay::RelayInboxKind::Approval,
        direction: dojo_protocol::relay::RelayMessageDirection::AgentToHuman,
        status: dojo_protocol::relay::RelayInboxStatus::Pending,
        payload: serde_json::json!({
            "prompt": format!("Approve {tool_name}?"),
            "tool": tool_name,
        }),
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
}
