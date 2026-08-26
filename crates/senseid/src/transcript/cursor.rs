//! Cursor transcript adapter. Reads JSONL files from
//! `~/.cursor/projects/<project-hash>/agent-transcripts/`.
//!
//! Storage layout:
//!   Nested: `~/.cursor/projects/<hash>/agent-transcripts/<session-id>/<session-id>.jsonl`
//!   Flat:   `~/.cursor/projects/<hash>/agent-transcripts/<session-id>.jsonl`
//!
//! JSONL line types:
//!   `user`      → user prompt turn boundary
//!   `assistant` → assistant text (text content blocks only)
//!   `tool_use`  → SynthEvent (tool name + input, no tool_result available)
//!   `system`    → skip (injected)
//!
//! Known limitation: Cursor JSONL does NOT include `tool_result` content.
//! `SynthEvent` reconstruction is limited to `tool_use` entries.

use super::{
    MAX_LINE_BYTES, MAX_TRANSCRIPT_BYTES, MAX_TURN_CHARS, ParsedTranscript, SynthEvent,
    SynthSession, TranscriptAdapter, TranscriptTurn, TurnFacts, UnitRef, human_prompt_text,
    merge_facts, turn_attrs,
};
use std::path::{Path, PathBuf};

fn mtime_ns(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos() as i64)
}

/// Heuristic: does this string look like a UUID? (8-4-4-4-12 hex)
fn looks_like_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Extract a file path from a tool_use `input` object.
fn tool_file_path(input: &serde_json::Value) -> Option<String> {
    input
        .get("file_path")
        .or_else(|| input.get("path"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Concatenated `text` content blocks from an assistant record.
fn assistant_text_blocks(v: &serde_json::Value) -> String {
    let Some(serde_json::Value::Array(blocks)) = v.get("message").and_then(|m| m.get("content"))
    else {
        return String::new();
    };
    let mut s = String::new();
    for b in blocks {
        if b.get("type").and_then(|t| t.as_str()) == Some("text")
            && let Some(t) = b.get("text").and_then(|t| t.as_str())
        {
            let t = t.trim();
            if !t.is_empty() {
                if !s.is_empty() {
                    s.push_str("\n\n");
                }
                s.push_str(t);
            }
        }
    }
    s
}

/// Parse Cursor JSONL transcript lines into turns.
pub fn parse_cursor_transcript(content: &str) -> Vec<TranscriptTurn> {
    let mut turns: Vec<TranscriptTurn> = Vec::new();
    let mut cur: Option<TranscriptTurn> = None;
    let mut cur_facts = TurnFacts::default();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let rec_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match rec_type {
            "user" => {
                if let Some(prompt) = human_prompt_text(&v) {
                    if let Some(mut t) = cur.take() {
                        t.facts = std::mem::take(&mut cur_facts);
                        turns.push(t);
                    }
                    let attrs = turn_attrs(&v, &["message"]);
                    cur = Some(TranscriptTurn {
                        turn_index: turns.len() as i32 + 1,
                        user_text: Some(prompt),
                        assistant_text: String::new(),
                        started_at: super::parse_timestamp(&v),
                        attrs,
                        facts: TurnFacts::default(),
                    });
                }
            }
            "assistant" => {
                let text = assistant_text_blocks(&v);
                if !text.is_empty()
                    && let Some(ref mut t) = cur
                {
                    if !t.assistant_text.is_empty() {
                        t.assistant_text.push_str("\n\n");
                    }
                    t.assistant_text.push_str(&text);
                }
                merge_facts(&mut cur_facts, &v);
            }
            _ => {}
        }
    }
    if let Some(mut t) = cur.take() {
        t.facts = std::mem::take(&mut cur_facts);
        turns.push(t);
    }

    // Cap pathological turns.
    for t in turns.iter_mut() {
        if t.assistant_text.chars().count() > MAX_TURN_CHARS {
            let mut s: String = t.assistant_text.chars().take(MAX_TURN_CHARS).collect();
            s.push('…');
            t.assistant_text = s;
        }
    }
    turns
}

/// Parse Cursor JSONL into a reconstructed session (cwds + events).
pub fn parse_cursor_session(content: &str) -> Option<SynthSession> {
    let mut cwds: Vec<String> = Vec::new();
    let mut events: Vec<SynthEvent> = Vec::new();
    let mut max_ts: i64 = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Collect cwds
        if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str())
            && !cwd.is_empty()
            && !cwds.contains(&cwd.to_string())
        {
            cwds.push(cwd.to_string());
        }

        let ts = super::parse_timestamp_ms(&v).unwrap_or(0);
        if ts > max_ts {
            max_ts = ts;
        }

        let rec_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match rec_type {
            "user" => {
                if human_prompt_text(&v).is_some() {
                    events.push(SynthEvent {
                        event_type: "UserPromptSubmit".to_string(),
                        tool_name: None,
                        file_path: None,
                        prompt: human_prompt_text(&v),
                        tool_input: None,
                        ts,
                    });
                }
            }
            "assistant" => {
                // Emit PostToolUse for each tool_use block
                if let Some(serde_json::Value::Array(blocks)) =
                    v.get("message").and_then(|m| m.get("content"))
                {
                    for b in blocks {
                        if b.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            let name =
                                b.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
                            let input = b.get("input").cloned().unwrap_or(serde_json::json!({}));
                            let file_path = tool_file_path(&input);
                            events.push(SynthEvent {
                                event_type: "PostToolUse".to_string(),
                                tool_name: name,
                                file_path,
                                prompt: None,
                                tool_input: Some(input),
                                ts,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if events.is_empty() {
        return None;
    }

    // Append synthetic Stop at max_ts
    events.push(SynthEvent {
        event_type: "Stop".to_string(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts: max_ts,
    });

    Some(SynthSession { cwds, events })
}

pub struct CursorAdapter {
    root: PathBuf,
}

impl CursorAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl TranscriptAdapter for CursorAdapter {
    fn source(&self) -> &'static str {
        "cursor"
    }

    fn family(&self) -> &'static str {
        "cursor"
    }

    fn units(&self) -> Vec<UnitRef> {
        let mut units = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return units;
        };
        for entry in entries.flatten() {
            let project_dir = entry.path();
            if !project_dir.is_dir() {
                continue;
            }
            let agent_dir = project_dir.join("agent-transcripts");
            if !agent_dir.is_dir() {
                continue;
            }
            // Flat layout: agent-transcripts/<session-id>.jsonl
            if let Ok(dir_entries) = std::fs::read_dir(&agent_dir) {
                for f in dir_entries.flatten() {
                    let path = f.path();
                    if path.is_file()
                        && path.extension().is_some_and(|e| e == "jsonl")
                        && let Some(stamp) = mtime_ns(&path)
                    {
                        units.push(UnitRef { key: path.to_string_lossy().to_string(), stamp });
                    }
                }
            }
            // Nested layout: agent-transcripts/<session-id>/<session-id>.jsonl
            if let Ok(dir_entries) = std::fs::read_dir(&agent_dir) {
                for d in dir_entries.flatten() {
                    let session_dir = d.path();
                    if !session_dir.is_dir() {
                        continue;
                    }
                    if let Ok(session_entries) = std::fs::read_dir(&session_dir) {
                        for f in session_entries.flatten() {
                            let path = f.path();
                            if path.is_file()
                                && path.extension().is_some_and(|e| e == "jsonl")
                                && let Some(stamp) = mtime_ns(&path)
                            {
                                units.push(UnitRef {
                                    key: path.to_string_lossy().to_string(),
                                    stamp,
                                });
                            }
                        }
                    }
                }
            }
        }
        units
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        mtime_ns(Path::new(key))
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        let path = Path::new(key);
        let stem = path.file_stem()?.to_str()?;
        let parent = path.parent()?.file_name()?.to_str()?;
        // If parent dir looks like a UUID, it's the session id (nested layout)
        // Otherwise the file stem is the session id (flat layout)
        let id = if looks_like_uuid(parent) { parent } else { stem };
        Some(format!("cursor-{}", id))
    }

    fn load_content(&self, key: &str) -> Option<String> {
        let path = Path::new(key);
        let meta = std::fs::metadata(path).ok()?;
        if meta.len() > MAX_TRANSCRIPT_BYTES {
            tracing::warn!(path = %key, size = meta.len(), "cursor: transcript exceeds MAX_TRANSCRIPT_BYTES, skipping");
            return None;
        }
        std::fs::read_to_string(path).ok()
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        let session = parse_cursor_session(content);
        ParsedTranscript {
            turns: parse_cursor_transcript(content),
            cwds: session.as_ref().map_or_else(Vec::new, |s| s.cwds.clone()),
            events: session.as_ref().map_or_else(Vec::new, |s| s.events.clone()),
            model: None, // Cursor JSONL does not carry model info
            tokens: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = r#"{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"add a login page"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"On it."}]}}
{"type":"user","timestamp":"2026-06-22T10:01:00.000Z","message":{"role":"user","content":"now wire it up"}}
{"type":"assistant","timestamp":"2026-06-22T10:01:01.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Wired."}]}}
"#;

    #[test]
    fn parse_simple_turn() {
        let turns = parse_cursor_transcript(SIMPLE);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].user_text.as_deref(), Some("add a login page"));
        assert_eq!(turns[0].assistant_text, "On it.");
        assert_eq!(turns[1].user_text.as_deref(), Some("now wire it up"));
        assert_eq!(turns[1].assistant_text, "Wired.");
    }

    #[test]
    fn parse_multiple_turns() {
        let turns = parse_cursor_transcript(SIMPLE);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].turn_index, 1);
        assert_eq!(turns[1].turn_index, 2);
    }

    #[test]
    fn parse_tool_use_events() {
        let content = r#"{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/x.rs"}}]}}
{"type":"assistant","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}
"#;
        let session = parse_cursor_session(content).unwrap();
        let kinds: Vec<&str> = session.events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(
            kinds.contains(&"UserPromptSubmit")
                && kinds.contains(&"PostToolUse")
                && kinds.contains(&"Stop"),
            "got {kinds:?}"
        );
        let edit = session.events.iter().find(|e| e.tool_name.as_deref() == Some("Edit")).unwrap();
        assert_eq!(edit.file_path.as_deref(), Some("/repo/src/x.rs"));
    }

    #[test]
    fn parse_injected_messages() {
        let content = r#"{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}
{"type":"user","timestamp":"2026-06-22T10:00:01.000Z","message":{"role":"user","content":"<system-reminder>hook context</system-reminder>"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"On it."}]}}
"#;
        let turns = parse_cursor_transcript(content);
        assert_eq!(turns.len(), 1, "injected message skipped");
        assert_eq!(turns[0].user_text.as_deref(), Some("fix the parser"));
    }

    #[test]
    fn turn_facts_populated() {
        let content = r#"{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"go"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","gitBranch":"develop","message":{"role":"assistant","stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":20}},"content":[{"type":"text","text":"ok"}]}
"#;
        let turns = parse_cursor_transcript(content);
        assert_eq!(turns[0].facts.git_branch.as_deref(), Some("develop"));
        assert_eq!(turns[0].facts.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(turns[0].facts.tokens_in, Some(10));
        assert_eq!(turns[0].facts.tokens_out, Some(20));
    }

    #[test]
    fn turn_facts_null_when_absent() {
        let content = r#"{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"hi"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"yo"}]}}
"#;
        let turns = parse_cursor_transcript(content);
        assert_eq!(turns[0].facts.git_branch, None);
        assert_eq!(turns[0].facts.stop_reason, None);
        assert_eq!(turns[0].facts.tokens_in, None);
    }

    #[test]
    fn parse_session_reconstructs_events() {
        let session = parse_cursor_session(SIMPLE).unwrap();
        let kinds: Vec<&str> = session.events.iter().map(|e| e.event_type.as_str()).collect();
        // Two user messages → two UserPromptSubmit events; a single synthetic Stop at the end.
        assert_eq!(kinds, vec!["UserPromptSubmit", "UserPromptSubmit", "Stop"]);
        assert_eq!(session.events[0].prompt.as_deref(), Some("add a login page"));
    }

    #[test]
    fn parse_session_none_when_empty() {
        assert!(parse_cursor_session("").is_none());
        assert!(parse_cursor_session("not json\n").is_none());
    }

    #[test]
    fn session_id_nested_layout() {
        let adapter = CursorAdapter::new(PathBuf::from("/tmp"));
        let key = "/Users/.cursor/projects/abc/agent-transcripts/550e8400-e29b-41d4-a716-446655440000/550e8400-e29b-41d4-a716-446655440000.jsonl";
        assert_eq!(
            adapter.session_id_for(key),
            Some("cursor-550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn session_id_flat_layout() {
        let adapter = CursorAdapter::new(PathBuf::from("/tmp"));
        let key = "/Users/.cursor/projects/abc/agent-transcripts/my-session.jsonl";
        assert_eq!(adapter.session_id_for(key), Some("cursor-my-session".to_string()));
    }

    #[test]
    fn looks_like_uuid_cases() {
        assert!(looks_like_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!looks_like_uuid("not-a-uuid"));
        assert!(!looks_like_uuid("abc"));
        assert!(!looks_like_uuid("550e8400-e29b-41d4-a716-44665544000")); // too short
    }

    #[test]
    fn parse_produces_common_structure() {
        let adapter = CursorAdapter::new(PathBuf::from("/tmp"));
        let p = adapter.parse(SIMPLE);
        assert_eq!(p.turns.len(), 2);
        assert_eq!(p.model, None);
        assert_eq!(p.tokens, None);
        let kinds: Vec<&str> = p.events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(kinds.contains(&"UserPromptSubmit") && kinds.contains(&"Stop"));
    }

    #[test]
    fn assistant_text_concatenation() {
        let content = r#"{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"go"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"first"}]}}
{"type":"assistant","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"text","text":"second"}]}}
"#;
        let turns = parse_cursor_transcript(content);
        assert_eq!(turns[0].assistant_text, "first\n\nsecond");
    }
}
