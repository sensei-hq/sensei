//! Session enrichment — analyzer layer L0 (#66).
//!
//! `activity.events` is empty and sessions are created metric-less by the #31
//! hook derivation, so the captured `activity.hook_events` stream is the only
//! signal. This stage turns a session's events (UserPromptSubmit / PreToolUse /
//! PostToolUse / Stop …) into `turns`, `corrections`, `outcome`, `ftr`,
//! `duration_ms`, `module`, and a per-tool usage breakdown, then writes them
//! back onto `activity.sessions`. It is what makes FTR/outcomes non-null across
//! the product. Pure derivation (`derive_session_metrics`) is decoupled from
//! the DB so it is unit-testable over an in-memory slice; the orchestrators
//! (`enrich_session`, `analyze_project`) handle the I/O.

use super::super::executor::TaskContext;
use super::super::Task;

/// One hook event projected to just the fields the heuristics read — decoupled
/// from the DB row so derivation is a pure function.
#[derive(Debug, Clone)]
pub struct HookEvent {
    pub event_type: String,
    pub tool_name: Option<String>,
    pub ts: i64,
    pub prompt: Option<String>,
    pub file_path: Option<String>,
    pub tool_failed: bool,
}

/// Derived per-session metrics written to `activity.sessions`.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionMetrics {
    pub turns: i32,
    pub corrections: i32,
    pub outcome: &'static str, // a `sensei.session_outcome` label
    pub ftr: bool,
    pub duration_ms: i64,
    pub module: Option<String>,
    pub tool_usage: serde_json::Value, // { "<tool>": { "pre": n, "post": n, "failed": n } }
}

/// A user prompt that signals the previous turn needed correcting — the FTR
/// detractor. Maps onto the schema's `triage_signal` vocabulary. Text-only,
/// lowercased, and deliberately PRECISION-favoring: a false correction wrongly
/// tanks FTR, so only unambiguous phrasings count (e.g. plain instructions like
/// "don't forget the test" must NOT match). Tunable as we see real data.
pub fn correction_signal(prompt: &str) -> Option<&'static str> {
    let p = prompt.trim().to_lowercase();
    const REVERT: &[&str] = &["revert", "roll back", "undo that", "undo the", "undo your"];
    const WRONG: &[&str] = &[
        "that's wrong", "thats wrong", "that's not right", "thats not right",
        "that's not what", "thats not what", "not what i asked", "you missed",
        "doesn't work", "does not work", "didn't work", "did not work",
        "still broken", "still failing", "still fails", "you broke", "that broke it",
    ];
    const ACTUALLY: &[&str] = &["actually,", "actually ", "wait,", "wait ", "no, that"];
    if REVERT.iter().any(|s| p.contains(s)) {
        return Some("revert");
    }
    if WRONG.iter().any(|s| p.contains(s)) {
        return Some("correction");
    }
    if ACTUALLY.iter().any(|s| p.starts_with(s)) {
        return Some("actually");
    }
    None
}

fn count_turns(events: &[HookEvent]) -> i32 {
    events.iter().filter(|e| e.event_type == "UserPromptSubmit").count() as i32
}

fn count_corrections(events: &[HookEvent]) -> i32 {
    events
        .iter()
        .filter(|e| e.event_type == "UserPromptSubmit")
        .filter(|e| e.prompt.as_deref().and_then(correction_signal).is_some())
        .count() as i32
}

/// Failed `PostToolUse` events among the last few events — an error cluster at
/// the tail of a session with no clean end suggests it was blocked, not merely
/// abandoned.
fn trailing_failures(events: &[HookEvent]) -> usize {
    let window = events.len().min(5);
    events[events.len() - window..]
        .iter()
        .filter(|e| e.event_type == "PostToolUse" && e.tool_failed)
        .count()
}

/// `session_outcome` label. A clean end (Stop/SessionEnd) → `corrected` if the
/// user had to correct, else `completed`. No end → `blocked` on a tail error
/// cluster, else `abandoned`.
fn derive_outcome(events: &[HookEvent], corrections: i32) -> &'static str {
    let has_end = events
        .iter()
        .any(|e| e.event_type == "Stop" || e.event_type == "SessionEnd");
    if has_end {
        if corrections > 0 { "corrected" } else { "completed" }
    } else if trailing_failures(events) >= 2 {
        "blocked"
    } else {
        "abandoned"
    }
}

fn parent_dir(path: &str) -> Option<String> {
    path.rsplit_once('/').map(|(dir, _)| dir.to_string()).filter(|d| !d.is_empty())
}

/// Most-touched directory across the session's file operations.
fn dominant_module(events: &[HookEvent]) -> Option<String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in events {
        if let Some(fp) = e.file_path.as_deref()
            && let Some(dir) = parent_dir(fp)
        {
            *counts.entry(dir).or_default() += 1;
        }
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(dir, _)| dir)
}

/// Per-tool { pre, post, failed } counts, as a stable (sorted) JSON object.
fn tally_tool_usage(events: &[HookEvent]) -> serde_json::Value {
    let mut map: std::collections::BTreeMap<String, (i64, i64, i64)> =
        std::collections::BTreeMap::new();
    for e in events {
        let Some(tool) = e.tool_name.as_deref() else { continue };
        let entry = map.entry(tool.to_string()).or_default();
        match e.event_type.as_str() {
            "PreToolUse" => entry.0 += 1,
            "PostToolUse" => {
                entry.1 += 1;
                if e.tool_failed {
                    entry.2 += 1;
                }
            }
            _ => {}
        }
    }
    let obj: serde_json::Map<String, serde_json::Value> = map
        .into_iter()
        .map(|(tool, (pre, post, failed))| {
            (tool, serde_json::json!({ "pre": pre, "post": post, "failed": failed }))
        })
        .collect();
    serde_json::Value::Object(obj)
}

/// Derive metrics for one session from its hook events. Returns `None` for an
/// empty stream (don't fabricate an outcome for a session we saw nothing of).
pub fn derive_session_metrics(events: &[HookEvent]) -> Option<SessionMetrics> {
    if events.is_empty() {
        return None;
    }
    let turns = count_turns(events);
    let corrections = count_corrections(events);
    let outcome = derive_outcome(events, corrections);
    let (min_ts, max_ts) = events
        .iter()
        .fold((i64::MAX, i64::MIN), |(lo, hi), e| (lo.min(e.ts), hi.max(e.ts)));
    Some(SessionMetrics {
        turns,
        corrections,
        outcome,
        ftr: corrections == 0,
        duration_ms: (max_ts - min_ts).max(0),
        module: dominant_module(events),
        tool_usage: tally_tool_usage(events),
    })
}

/// Map a `{event_type, tool_name, ts, payload}` hook_events row to a HookEvent,
/// extracting the prompt / file path / tool-failure signal from the payload.
/// `success` column is unreliable for PostToolUse (NULL at ingest), so failure
/// is read from `tool_response.is_error` / an `error` key instead.
fn hook_event_from_row(row: &serde_json::Value) -> HookEvent {
    let payload = row.get("payload").cloned().unwrap_or(serde_json::Value::Null);
    let prompt = payload.get("prompt").and_then(|v| v.as_str()).map(str::to_string);
    let file_path = payload
        .get("file_path")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("tool_input").and_then(|t| t.get("file_path")).and_then(|v| v.as_str()))
        .map(str::to_string);
    let tool_failed = match payload.get("tool_response") {
        Some(serde_json::Value::Object(o)) => {
            o.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false) || o.contains_key("error")
        }
        _ => false,
    };
    HookEvent {
        event_type: row.get("event_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        tool_name: row.get("tool_name").and_then(|v| v.as_str()).map(str::to_string),
        ts: row.get("ts").and_then(|v| v.as_i64()).unwrap_or(0),
        prompt,
        file_path,
        tool_failed,
    }
}

/// Enrich one session in place. Returns `true` if metrics were written, `false`
/// if the session had no hook events (left untouched). Idempotent — recompute
/// overwrites, never duplicates.
pub async fn enrich_session(
    ctx: &TaskContext,
    session_id: &uuid::Uuid,
    client_session_id: &str,
) -> Result<bool, String> {
    let rows = ctx.pg().get_hook_events_for_session(client_session_id).await?;
    let events: Vec<HookEvent> = rows.iter().map(hook_event_from_row).collect();
    match derive_session_metrics(&events) {
        Some(m) => {
            ctx.pg()
                .update_session_metrics(
                    session_id, m.turns, m.corrections, m.outcome, m.ftr,
                    m.duration_ms, m.module.as_deref(), &m.tool_usage,
                )
                .await?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Handler for `TaskKind::AnalyzeProject`: enrich every attributed session of a
/// project. `task.path` carries the project id (UUID string).
pub async fn analyze_project(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(&task.path)
        .map_err(|_| format!("AnalyzeProject: invalid project id '{}'", task.path))?;
    let sessions = ctx.pg().get_project_session_ids(&project_id).await?;
    let mut enriched = 0u32;
    for (id, client_session_id) in sessions {
        match enrich_session(ctx, &id, &client_session_id).await {
            Ok(true) => enriched += 1,
            Ok(false) => {}
            Err(e) => tracing::warn!(error = %e, session = %id, "analyze_project: enrich_session failed"),
        }
    }
    tracing::info!("analyze_project: {} — enriched {} sessions", project_id, enriched);
    Ok(enriched)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(event_type: &str, ts: i64) -> HookEvent {
        HookEvent {
            event_type: event_type.into(),
            tool_name: None,
            ts,
            prompt: None,
            file_path: None,
            tool_failed: false,
        }
    }
    fn prompt_ev(text: &str, ts: i64) -> HookEvent {
        HookEvent { prompt: Some(text.into()), ..ev("UserPromptSubmit", ts) }
    }
    fn tool_ev(event_type: &str, tool: &str, ts: i64, failed: bool) -> HookEvent {
        HookEvent {
            tool_name: Some(tool.into()),
            tool_failed: failed,
            ..ev(event_type, ts)
        }
    }

    #[test]
    fn empty_stream_yields_no_metrics() {
        assert!(derive_session_metrics(&[]).is_none());
    }

    #[test]
    fn clean_session_is_completed_first_try() {
        let events = vec![
            prompt_ev("add a test for the parser", 1000),
            prompt_ev("now wire it into the build", 2000),
            prompt_ev("ship it", 3000),
            ev("Stop", 4000),
        ];
        let m = derive_session_metrics(&events).unwrap();
        assert_eq!(m.turns, 3);
        assert_eq!(m.corrections, 0);
        assert!(m.ftr);
        assert_eq!(m.outcome, "completed");
        assert_eq!(m.duration_ms, 3000);
    }

    #[test]
    fn correction_prompt_drops_ftr_and_marks_corrected() {
        let events = vec![
            prompt_ev("implement the cache", 1000),
            prompt_ev("actually, revert that — wrong approach", 2000),
            ev("Stop", 3000),
        ];
        let m = derive_session_metrics(&events).unwrap();
        assert_eq!(m.corrections, 1, "the revert/actually prompt counts as a correction");
        assert!(!m.ftr);
        assert_eq!(m.outcome, "corrected");
    }

    #[test]
    fn benign_imperative_prompts_are_not_corrections() {
        // Precision guard: ordinary instructions must not look like corrections.
        for p in ["don't forget the test", "no rush on this", "add error handling"] {
            assert!(correction_signal(p).is_none(), "false positive on: {p}");
        }
        assert_eq!(correction_signal("Actually, that's wrong"), Some("correction"));
        assert_eq!(correction_signal("revert the last change"), Some("revert"));
    }

    #[test]
    fn no_end_event_is_abandoned() {
        let events = vec![prompt_ev("start something", 1000), tool_ev("PostToolUse", "Edit", 2000, false)];
        assert_eq!(derive_session_metrics(&events).unwrap().outcome, "abandoned");
    }

    #[test]
    fn tail_error_cluster_without_end_is_blocked() {
        let events = vec![
            prompt_ev("fix the build", 1000),
            tool_ev("PostToolUse", "Bash", 2000, true),
            tool_ev("PostToolUse", "Bash", 3000, true),
        ];
        assert_eq!(derive_session_metrics(&events).unwrap().outcome, "blocked");
    }

    #[test]
    fn tool_usage_tallies_pre_post_and_failures() {
        let events = vec![
            tool_ev("PreToolUse", "Edit", 1000, false),
            tool_ev("PostToolUse", "Edit", 1100, false),
            tool_ev("PreToolUse", "Bash", 2000, false),
            tool_ev("PostToolUse", "Bash", 2100, true),
            ev("Stop", 3000),
        ];
        let m = derive_session_metrics(&events).unwrap();
        assert_eq!(m.tool_usage["Edit"], serde_json::json!({ "pre": 1, "post": 1, "failed": 0 }));
        assert_eq!(m.tool_usage["Bash"], serde_json::json!({ "pre": 1, "post": 1, "failed": 1 }));
    }

    #[test]
    fn dominant_module_is_most_touched_dir() {
        let events = vec![
            HookEvent { file_path: Some("src/api/handlers/x.rs".into()), ..tool_ev("PostToolUse", "Edit", 1000, false) },
            HookEvent { file_path: Some("src/api/handlers/y.rs".into()), ..tool_ev("PostToolUse", "Edit", 1100, false) },
            HookEvent { file_path: Some("README.md".into()), ..tool_ev("PostToolUse", "Edit", 1200, false) },
            ev("Stop", 2000),
        ];
        assert_eq!(derive_session_metrics(&events).unwrap().module.as_deref(), Some("src/api/handlers"));
    }

    #[test]
    fn hook_event_from_row_extracts_payload_fields() {
        let row = serde_json::json!({
            "event_type": "PostToolUse", "tool_name": "Edit", "ts": 42,
            "payload": { "tool_input": { "file_path": "src/x.rs" },
                         "tool_response": { "is_error": true } }
        });
        let e = hook_event_from_row(&row);
        assert_eq!(e.event_type, "PostToolUse");
        assert_eq!(e.file_path.as_deref(), Some("src/x.rs"));
        assert!(e.tool_failed);
    }

    // ── DB-backed orchestrator test ──────────────────────────────────────
    use std::sync::Arc;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::TaskKind;
    use crate::api::state::SharedState;

    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });
        Arc::new(TaskContext { queue, app_state, _graph_path: None, logger: sensei_logger::Logger::noop() })
    }

    #[tokio::test]
    async fn analyze_project_enriches_sessions_from_hooks_idempotently() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let pid = pg.create_project(&format!("_test:analyze-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/ana-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let fid = pg.upsert_repo(&root, "ana-repo", &format!("/_test/ana-{}", uuid::Uuid::new_v4())).await.unwrap();
        let csid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let sid = pg.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

        // hook stream: 2 prompts (one a correction) + an edit + a Stop.
        let prompt = |t: &str| serde_json::json!({ "prompt": t });
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1000, None, &prompt("build the thing")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 2000, None, &prompt("actually, revert that")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "PostToolUse", Some("Edit"), None, 2500, None, &serde_json::json!({})).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "Stop", None, None, 3000, None, &serde_json::json!({})).await.unwrap();

        // Task::new(kind, folder_path, path) — the handler reads the project id
        // from `task.path` (the last arg).
        let task = Task::new(TaskKind::AnalyzeProject, "", &pid.to_string());
        assert_eq!(analyze_project(&ctx, &task).await.unwrap(), 1, "one session enriched");

        let metrics = || async {
            let row: (i32, i32, Option<bool>, Option<String>, Option<i32>) = sqlx_core::query_as::query_as(
                "SELECT turns, corrections, ftr, outcome::text, duration_ms FROM activity.sessions WHERE id = $1"
            ).bind(sid).fetch_one(pg.pool()).await.unwrap();
            row
        };
        let (turns, corrections, ftr, outcome, duration) = metrics().await;
        assert_eq!(turns, 2);
        assert_eq!(corrections, 1, "the 'revert' prompt is a correction");
        assert_eq!(ftr, Some(false));
        assert_eq!(outcome.as_deref(), Some("corrected"), "has Stop + a correction");
        assert_eq!(duration, Some(2000));

        // Idempotent: a second run leaves identical values.
        analyze_project(&ctx, &task).await.unwrap();
        let (turns2, corrections2, ..) = metrics().await;
        assert_eq!((turns2, corrections2), (2, 1));

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.hook_events WHERE session_id = $1").bind(&csid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1").bind(sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(pool).await.ok();
    }
}
