//! Project a run's `TodoWrite` list into the relay outline (`relay_segments`).
//!
//! The daemon captures every assistant event into `activity.assistant_events`; a
//! `TodoWrite` event carries the agent's own task list in `payload.tool_input.todos`.
//! This module holds the PURE, testable core — parse the todos out of a captured
//! payload, and map them to the `dojo_protocol::relay::RelaySegment` wire type
//! (the phone outline). The impure part (read the latest TodoWrite for an active
//! run from Postgres, then publish via [`crate::dojo::client::DojoClient::upsert_segments`])
//! wraps these.
//!
//! Zero-knowledge (D10): only the todo `content` (a short human phrase) becomes a
//! segment title — never tool args, file contents, or diffs. Todos are already
//! one-liners, so no summarization is applied here; the gemma4 rollup is for the
//! richer tool-activity→phase projection (a later enrichment), not per-todo.

use dojo_protocol::relay::{RelayRunStatus, RelaySegment, RelaySessionUpdate, SegmentState};
use serde::Deserialize;

/// One `TodoWrite` entry (Claude Code): `content` + `status`
/// (`pending` | `in_progress` | `completed`). Extra fields (activeForm, id) are
/// ignored.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Todo {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub status: String,
}

/// Extract the todo list from an `activity.assistant_events` `payload` jsonb of a
/// `TodoWrite` tool event: `payload.tool_input.todos`. Empty if absent/malformed.
pub fn parse_todos(payload: &serde_json::Value) -> Vec<Todo> {
    payload
        .get("tool_input")
        .and_then(|ti| ti.get("todos"))
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter().filter_map(|v| serde_json::from_value::<Todo>(v.clone()).ok()).collect()
        })
        .unwrap_or_default()
}

/// Map a todo status string to a [`SegmentState`]. Unknown → `Pending`.
fn todo_state(status: &str) -> SegmentState {
    match status {
        "completed" => SegmentState::Done,
        "in_progress" => SegmentState::Active,
        _ => SegmentState::Pending,
    }
}

/// Roll a run's `TodoWrite` list into relay segments — the phone outline. Each
/// todo → a top-level segment: `seq` = index, `title` = the todo content, `state`
/// from the todo status. Server ids are `None` (the Worker upserts by session+seq).
pub fn todos_to_segments(todos: &[Todo]) -> Vec<RelaySegment> {
    todos
        .iter()
        .enumerate()
        .map(|(i, t)| RelaySegment {
            id: None,
            parent_id: None,
            parent_seq: None,
            seq: i as i32,
            title: t.content.clone(),
            summary: None,
            detail: None,
            state: todo_state(&t.status),
            is_gate: false,
            gate_severity: None,
            response_verdict: None,
            response_note: None,
            agent: None,
            model: None,
            spec_ref: None,
        })
        .collect()
}

/// Derive a run title from the assistant's working directory: the folder's
/// basename (trailing '/' trimmed, last path segment). Falls back to
/// `"Coding session"` when the cwd is absent/empty — the phone always shows a
/// human label, never a raw path or a blank.
pub fn title_from_cwd(cwd: Option<&str>) -> String {
    cwd.map(|c| c.trim_end_matches('/'))
        .filter(|c| !c.is_empty())
        .and_then(|c| c.rsplit('/').next())
        .filter(|seg| !seg.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "Coding session".to_string())
}

/// Build the filtered status snapshot for a run from its outline segments —
/// the run-detail header (P2 uses the assistant `session_id` as `run_id`; there
/// is no `activity.runs` table yet). Status is always `Running` (the segment
/// feed only fires while the run is live); progress is derived from segment
/// state (`total` = count, `done` = count of `Done`); `current_feature` is the
/// title of the first `Active` segment, if any. All other optional fields are
/// `None` — the todo outline carries no phase/goal/timing.
pub fn session_update(run_id: &str, title: &str, segments: &[RelaySegment]) -> RelaySessionUpdate {
    let progress_total = segments.len() as i32;
    let progress_done = segments.iter().filter(|s| s.state == SegmentState::Done).count() as i32;
    let current_feature =
        segments.iter().find(|s| s.state == SegmentState::Active).map(|s| s.title.clone());
    RelaySessionUpdate {
        run_id: run_id.to_string(),
        status: RelayRunStatus::Running,
        title: title.to_string(),
        goal: None,
        progress_done,
        progress_total,
        current_phase: None,
        current_feature,
        last_event_at: None,
        paused_until: None,
        pause_reason: None,
        // The TodoWrite/session-keyed path carries no run heartbeat — that is
        // the run-bridge's job (relay_run_project::run_to_session_update).
        heartbeat_at: None,
        // Seat attribution is driven from the run-bridge path (publish_run), not
        // this session-keyed TodoWrite projection.
        project_slug: None,
        project: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_todos_from_todowrite_payload() {
        let payload = json!({
            "tool_name": "TodoWrite",
            "event_type": "PreToolUse",
            "tool_input": { "todos": [
                {"content": "add publish_gate", "status": "completed", "activeForm": "adding publish_gate"},
                {"content": "poll_inbox", "status": "in_progress"},
                {"content": "wire /v1/relay", "status": "pending"}
            ]}
        });
        let todos = parse_todos(&payload);
        assert_eq!(todos.len(), 3);
        assert_eq!(todos[0].content, "add publish_gate");
        assert_eq!(todos[1].status, "in_progress");
    }

    #[test]
    fn parse_todos_empty_on_missing_or_malformed() {
        assert!(parse_todos(&json!({"tool_input": {}})).is_empty());
        assert!(parse_todos(&json!({})).is_empty());
        assert!(parse_todos(&json!({"tool_input": {"todos": "nope"}})).is_empty());
    }

    #[test]
    fn maps_todos_to_segments_with_state() {
        let todos = vec![
            Todo { content: "done thing".into(), status: "completed".into() },
            Todo { content: "doing thing".into(), status: "in_progress".into() },
            Todo { content: "todo thing".into(), status: "pending".into() },
            Todo { content: "weird".into(), status: "??".into() },
        ];
        let segs = todos_to_segments(&todos);
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].seq, 0);
        assert_eq!(segs[0].title, "done thing");
        assert_eq!(segs[0].state, SegmentState::Done);
        assert_eq!(segs[1].state, SegmentState::Active);
        assert_eq!(segs[2].state, SegmentState::Pending);
        assert_eq!(segs[3].state, SegmentState::Pending); // unknown → pending
        assert!(segs.iter().all(|s| !s.is_gate && s.id.is_none() && s.parent_id.is_none()));
    }

    #[test]
    fn projection_carries_no_code_or_diffs() {
        // Only the todo `content` is copied into title — never tool args/diffs.
        let todos =
            parse_todos(&json!({"tool_input": {"todos": [{"content": "x", "status": "pending"}]}}));
        let segs = todos_to_segments(&todos);
        assert_eq!(segs[0].title, "x");
        assert!(segs[0].detail.is_none() && segs[0].summary.is_none());
    }

    #[test]
    fn title_from_cwd_takes_basename() {
        assert_eq!(title_from_cwd(Some("/Users/jerry/Developer/sensei-hq/sensei")), "sensei");
        assert_eq!(title_from_cwd(Some("/Users/jerry/Developer/sensei-hq/sensei/")), "sensei");
        assert_eq!(title_from_cwd(Some("bare")), "bare");
    }

    #[test]
    fn title_from_cwd_falls_back_when_missing_or_empty() {
        assert_eq!(title_from_cwd(None), "Coding session");
        assert_eq!(title_from_cwd(Some("")), "Coding session");
        assert_eq!(title_from_cwd(Some("/")), "Coding session");
    }

    #[test]
    fn session_update_counts_progress_and_picks_active_feature() {
        let segs = todos_to_segments(&[
            Todo { content: "done a".into(), status: "completed".into() },
            Todo { content: "done b".into(), status: "completed".into() },
            Todo { content: "doing it".into(), status: "in_progress".into() },
            Todo { content: "todo".into(), status: "pending".into() },
        ]);
        let u = session_update("sess-1", "sensei", &segs);
        assert_eq!(u.run_id, "sess-1");
        assert_eq!(u.title, "sensei");
        assert_eq!(u.status, RelayRunStatus::Running);
        assert_eq!(u.progress_total, 4);
        assert_eq!(u.progress_done, 2);
        assert_eq!(u.current_feature.as_deref(), Some("doing it"));
        // The todo outline carries no phase/goal/timing.
        assert!(u.goal.is_none() && u.current_phase.is_none());
        assert!(u.last_event_at.is_none() && u.paused_until.is_none() && u.pause_reason.is_none());
    }

    #[test]
    fn session_update_empty_segments_is_zero_progress_no_feature() {
        let u = session_update("sess-2", "proj", &[]);
        assert_eq!(u.progress_total, 0);
        assert_eq!(u.progress_done, 0);
        assert!(u.current_feature.is_none());
    }

    #[test]
    fn session_update_no_active_segment_yields_no_feature() {
        let segs = todos_to_segments(&[
            Todo { content: "a".into(), status: "completed".into() },
            Todo { content: "b".into(), status: "pending".into() },
        ]);
        let u = session_update("sess-3", "proj", &segs);
        assert_eq!(u.progress_total, 2);
        assert_eq!(u.progress_done, 1);
        assert!(u.current_feature.is_none(), "no in_progress todo ⇒ no current_feature");
    }
}
