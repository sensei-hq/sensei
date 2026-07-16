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

// The pure projection API is consumed by the segment-publish path (P2 next chunk:
// read the latest TodoWrite for an active run → todos_to_segments → upsert_segments).
#![allow(dead_code)]

use dojo_protocol::relay::{RelaySegment, SegmentState};
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
            arr.iter()
                .filter_map(|v| serde_json::from_value::<Todo>(v.clone()).ok())
                .collect()
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
            seq: i as i32,
            title: t.content.clone(),
            summary: None,
            detail: None,
            state: todo_state(&t.status),
            is_gate: false,
            gate_severity: None,
            response_verdict: None,
            response_note: None,
        })
        .collect()
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
        let todos = parse_todos(&json!({"tool_input": {"todos": [{"content": "x", "status": "pending"}]}}));
        let segs = todos_to_segments(&todos);
        assert_eq!(segs[0].title, "x");
        assert!(segs[0].detail.is_none() && segs[0].summary.is_none());
    }
}
