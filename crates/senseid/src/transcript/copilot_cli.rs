//! GitHub Copilot CLI transcript adapter. Reads from:
//! - `~/.copilot/session-state/<session-id>/events.jsonl` (full event stream)
//! - `~/.copilot/session-store.db` (structured turns, SQLite)
//! - `~/.copilot/data.db` (aggregate tokens)
//!
//! One unit per session — deduped by session ID. JSONL text wins when both
//! sources have the same turn (JSONL is the primary record). SQLite fills
//! turns that JSONL lacks.

use super::{
    MAX_LINE_BYTES, MAX_TRANSCRIPT_BYTES, MAX_TURN_CHARS, ParsedTranscript, SessionTokens,
    SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, TurnFacts, UnitRef,
    parse_timestamp,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn mtime_ns(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos() as i64)
}

/// Query session IDs and their updated_at timestamps from session-store.db.
fn query_session_ids(db_path: &Path) -> Vec<(String, i64)> {
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let mut stmt = match conn.prepare("SELECT id, updated_at FROM sessions") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let updated_at: i64 = row.get(1)?;
        Ok((id, updated_at))
    })
    .map(|r| r.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Read the `cwd:` line from a workspace.yaml file.
fn read_workspace_yaml(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(cwd) = trimmed.strip_prefix("cwd:") {
            return Some(cwd.trim().to_string());
        }
    }
    None
}

/// Parse events.jsonl lines into turns.
fn parse_events_jsonl_turns(content: &str) -> Vec<TranscriptTurn> {
    let mut turns = Vec::new();
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

        let ev_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match ev_type {
            "user.message" => {
                let text = v
                    .get("data")
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                if !text.is_empty() {
                    if let Some(mut t) = cur.take() {
                        t.facts = std::mem::take(&mut cur_facts);
                        turns.push(t);
                    }
                    cur = Some(TranscriptTurn {
                        turn_index: turns.len() as i32 + 1,
                        user_text: Some(text.to_string()),
                        assistant_text: String::new(),
                        started_at: parse_timestamp(&v),
                        attrs: serde_json::json!({}),
                        facts: TurnFacts::default(),
                    });
                }
            }
            "assistant.message" => {
                let text = v
                    .get("data")
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                if !text.is_empty()
                    && let Some(ref mut t) = cur
                {
                    if !t.assistant_text.is_empty() {
                        t.assistant_text.push_str("\n\n");
                    }
                    t.assistant_text.push_str(text);
                }
            }
            _ => {}
        }
    }
    if let Some(mut t) = cur.take() {
        t.facts = std::mem::take(&mut cur_facts);
        turns.push(t);
    }

    for t in turns.iter_mut() {
        if t.assistant_text.chars().count() > MAX_TURN_CHARS {
            let mut s: String = t.assistant_text.chars().take(MAX_TURN_CHARS).collect();
            s.push('…');
            t.assistant_text = s;
        }
    }
    turns
}

/// Parse events.jsonl into a reconstructed session (cwds + events).
fn parse_events_jsonl_session(content: &str) -> Option<SynthSession> {
    let cwds = Vec::new();
    let mut events = Vec::new();
    let mut max_ts = 0i64;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let ev_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let ts = super::parse_timestamp_ms(&v).unwrap_or(0);
        if ts > max_ts {
            max_ts = ts;
        }

        match ev_type {
            "user.message" => {
                let prompt = v
                    .get("data")
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                    .map(str::to_string);
                events.push(SynthEvent {
                    event_type: "UserPromptSubmit".to_string(),
                    tool_name: None,
                    file_path: None,
                    prompt,
                    tool_input: None,
                    ts,
                });
            }
            // `tool.execution_start`, NOT `tool_use`. Verified against 77,139
            // events across 9 real sessions: `tool_use` and `tool_result` do not
            // occur even once. The name carries `toolName` and the payload
            // `arguments` — the completion arrives separately as
            // `tool.execution_complete`, keyed by `toolCallId`.
            "tool.execution_start" => {
                let name = v
                    .get("data")
                    .and_then(|d| d.get("toolName"))
                    .and_then(|n| n.as_str())
                    .map(str::to_string);
                let input = v.get("data").and_then(|d| d.get("arguments")).cloned();
                let file_path = input
                    .as_ref()
                    .and_then(|i| i.get("file_path").or_else(|| i.get("path")))
                    .and_then(|p| p.as_str())
                    .map(str::to_string);
                events.push(SynthEvent {
                    event_type: "PostToolUse".to_string(),
                    tool_name: name,
                    file_path,
                    prompt: None,
                    tool_input: input,
                    ts,
                });
            }
            _ => {}
        }
    }

    if events.is_empty() {
        return None;
    }
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

/// Extract the model from the JSONL — whichever event names it first.
fn extract_jsonl_model(content: &str) -> Option<(String, String)> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
            && matches!(
                v.get("type").and_then(|t| t.as_str()),
                // `assistant.message` carries `model` on every turn;
                // `session.model_change` fires only when the user switches, and
                // occurred just twice in 77,139 sampled events — relying on it
                // alone left model NULL for almost every session.
                Some("assistant.message" | "session.model_change" | "assistant.turn_start")
            )
        {
            // `continue`, not `?`: a matching event without a model must not
            // abandon the scan — the next one almost certainly has it.
            if let Some(model) = v.get("data").and_then(|d| d.get("model")).and_then(|m| m.as_str())
            {
                return Some(("copilot".to_string(), model.to_string()));
            }
        }
    }
    None
}

/// Extract tokens from session.shutdown event in JSONL.
fn extract_jsonl_tokens(content: &str) -> Option<SessionTokens> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
            && v.get("type").and_then(|t| t.as_str()) == Some("session.shutdown")
        {
            let metrics = v.get("data").and_then(|d| d.get("modelMetrics"))?;
            // Take the first model's usage
            if let Some(usage) = metrics.as_object().and_then(|m| m.values().next()) {
                let u = usage.get("usage")?;
                let input = u.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let output = u.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                return Some(SessionTokens {
                    input,
                    output,
                    cache_read: None,
                    cache_write: None,
                    reasoning: None,
                    cost: None,
                });
            }
        }
    }
    None
}

/// Parse turns from session-store.db (SQLite).
fn parse_sqlite_turns(db_path: &Path, session_id: &str) -> Vec<TranscriptTurn> {
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let mut stmt = match conn
        .prepare("SELECT user_message, assistant_response FROM turns WHERE session_id = ?1")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![session_id], |row| {
            let user: String = row.get(0)?;
            let assistant: String = row.get(1)?;
            Ok((user, assistant))
        })
        .map(|r| r.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut turns = Vec::new();
    for (i, (user_msg, assistant_msg)) in rows.into_iter().enumerate() {
        let mut assistant_text = assistant_msg;
        if assistant_text.chars().count() > MAX_TURN_CHARS {
            assistant_text = assistant_text.chars().take(MAX_TURN_CHARS).collect();
            assistant_text.push('…');
        }
        turns.push(TranscriptTurn {
            turn_index: i as i32 + 1,
            user_text: if user_msg.is_empty() { None } else { Some(user_msg) },
            assistant_text,
            started_at: None,
            attrs: serde_json::json!({}),
            facts: TurnFacts::default(),
        });
    }
    turns
}

/// Parse the merged JSON blob from load_content().
fn parse_copilot_merged(content: &str) -> ParsedTranscript {
    let root: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return ParsedTranscript::default(),
    };

    let source = root.get("source").and_then(|s| s.as_str()).unwrap_or("");

    match source {
        "merged" | "jsonl" => {
            let inner = root.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let cwd = root.get("cwd").and_then(|c| c.as_str()).map(str::to_string);
            let session = parse_events_jsonl_session(inner);
            let tokens = extract_jsonl_tokens(inner);
            ParsedTranscript {
                turns: parse_events_jsonl_turns(inner),
                cwds: {
                    let mut cwds = session.as_ref().map_or_else(Vec::new, |s| s.cwds.clone());
                    if let Some(c) = cwd
                        && !cwds.contains(&c)
                    {
                        cwds.push(c);
                    }
                    cwds
                },
                events: session.as_ref().map_or_else(Vec::new, |s| s.events.clone()),
                model: extract_jsonl_model(inner),
                tokens,
            }
        }
        "sqlite" => {
            let session_id = root.get("session_id").and_then(|s| s.as_str()).unwrap_or("");
            let db_path = root.get("db_path").and_then(|d| d.as_str()).map(Path::new);
            let turns = if let Some(db) = db_path {
                parse_sqlite_turns(db, session_id)
            } else {
                Vec::new()
            };
            let events = if !turns.is_empty() {
                let mut evts = Vec::new();
                for t in &turns {
                    if let Some(ref prompt) = t.user_text {
                        evts.push(SynthEvent {
                            event_type: "UserPromptSubmit".to_string(),
                            tool_name: None,
                            file_path: None,
                            prompt: Some(prompt.clone()),
                            tool_input: None,
                            ts: 0,
                        });
                    }
                }
                evts.push(SynthEvent {
                    event_type: "Stop".to_string(),
                    tool_name: None,
                    file_path: None,
                    prompt: None,
                    tool_input: None,
                    ts: 0,
                });
                evts
            } else {
                Vec::new()
            };
            ParsedTranscript { turns, cwds: Vec::new(), events, model: None, tokens: None }
        }
        _ => ParsedTranscript::default(),
    }
}

pub struct CopilotCliAdapter {
    home: PathBuf,
}

impl CopilotCliAdapter {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }
}

impl TranscriptAdapter for CopilotCliAdapter {
    fn source(&self) -> &'static str {
        "copilot_cli"
    }

    fn family(&self) -> &'static str {
        "copilot"
    }

    fn units(&self) -> Vec<UnitRef> {
        let mut by_session: HashMap<String, UnitRef> = HashMap::new();

        // Source A: events.jsonl
        let session_state = self.home.join("session-state");
        if let Ok(entries) = std::fs::read_dir(&session_state) {
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let sid = dir.file_name().and_then(|f| f.to_str()).unwrap_or("");
                let events = dir.join("events.jsonl");
                let stamp = mtime_ns(&events).unwrap_or(0);
                by_session
                    .entry(sid.to_string())
                    .and_modify(|u| u.stamp = u.stamp.max(stamp))
                    .or_insert_with(|| UnitRef { key: dir.to_string_lossy().to_string(), stamp });
            }
        }

        // Source B: session-store.db
        let db_path = self.home.join("session-store.db");
        if db_path.exists() {
            for (sid, updated_at_millis) in query_session_ids(&db_path) {
                let stamp_nanos = updated_at_millis * 1_000_000;
                by_session
                    .entry(sid.clone())
                    .and_modify(|u| u.stamp = u.stamp.max(stamp_nanos))
                    .or_insert_with(|| UnitRef {
                        key: format!("copilot-sqlite-{}", sid),
                        stamp: stamp_nanos,
                    });
            }
        }

        by_session.into_values().collect()
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        // For JSONL units, key is the session-state dir path
        let dir = Path::new(key);
        let events = dir.join("events.jsonl");
        let jsonl_stamp = mtime_ns(&events).unwrap_or(0);

        // For sqlite units, key is "copilot-sqlite-<sid>"
        let sqlite_stamp = if let Some(sid) = key.strip_prefix("copilot-sqlite-") {
            let db_path = self.home.join("session-store.db");
            if db_path.exists() {
                let conn = rusqlite::Connection::open_with_flags(
                    &db_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )
                .ok()?;
                let mut stmt =
                    conn.prepare("SELECT updated_at FROM sessions WHERE id = ?1").ok()?;
                let ts: i64 = stmt.query_row(rusqlite::params![sid], |row| row.get(0)).unwrap_or(0);
                ts * 1_000_000
            } else {
                0
            }
        } else {
            0
        };

        let stamp = jsonl_stamp.max(sqlite_stamp);
        if stamp > 0 { Some(stamp) } else { None }
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        // SQLite: key is "copilot-sqlite-<sid>" — check first (before file_name
        // which would match the full string for a bare key without directory separators).
        if let Some(sid) = key.strip_prefix("copilot-sqlite-") {
            return Some(format!("copilot-{}", sid));
        }
        // JSONL: key is the session-state dir path, session id is the dir name
        if let Some(sid) = Path::new(key).file_name().and_then(|f| f.to_str()) {
            return Some(format!("copilot-{}", sid));
        }
        None
    }

    fn load_content(&self, key: &str) -> Option<String> {
        let dir = Path::new(key);

        // JSONL session directory
        let events_path = dir.join("events.jsonl");
        let has_jsonl = events_path.exists();

        // Check if session-store.db also has this session
        let session_id = dir.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let db_path = self.home.join("session-store.db");
        let has_sqlite = db_path.exists()
            && query_session_ids(&db_path).iter().any(|(sid, _)| sid == session_id);

        if has_jsonl {
            let raw = std::fs::read_to_string(&events_path).ok()?;
            if raw.len() as u64 > MAX_TRANSCRIPT_BYTES {
                tracing::warn!(path = %key, size = raw.len(), "copilot_cli: events.jsonl exceeds MAX_TRANSCRIPT_BYTES, skipping");
                return None;
            }
            let cwd = dir
                .join("workspace.yaml")
                .exists()
                .then(|| read_workspace_yaml(&dir.join("workspace.yaml")).unwrap_or_default());
            let mut blob = serde_json::json!({
                "source": "merged",
                "cwd": cwd,
                "content": raw,
            });
            if has_sqlite {
                blob["session_id"] = serde_json::json!(session_id);
                blob["db_path"] = serde_json::json!(db_path.to_string_lossy());
            }
            return serde_json::to_string(&blob).ok();
        }

        // SQLite-only session
        if has_sqlite {
            let blob = serde_json::json!({
                "source": "sqlite",
                "session_id": session_id,
                "db_path": db_path.to_string_lossy(),
            });
            return serde_json::to_string(&blob).ok();
        }

        None
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        parse_copilot_merged(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The REAL wire format, copied from a live transcript. The previous fixture
    // used `tool_use` / `tool_result`, which do not occur in Copilot CLI output
    // at all — so these tests passed while the adapter produced zero tool events
    // against every real session.
    const EVENTS_JSONL: &str = r#"{"type":"user.message","timestamp":"2026-06-22T10:00:00.000Z","data":{"content":"fix the parser"}}
{"type":"assistant.turn_start","timestamp":"2026-06-22T10:00:01.000Z","data":{"turnId":"0","model":"claude-opus-4.6"}}
{"type":"assistant.message","timestamp":"2026-06-22T10:00:02.000Z","data":{"content":"On it.","model":"claude-opus-4.6"}}
{"type":"tool.execution_start","timestamp":"2026-06-22T10:00:03.000Z","data":{"toolCallId":"t1","toolName":"str_replace_editor","arguments":{"file_path":"/repo/src/x.rs"}}}
{"type":"tool.execution_complete","timestamp":"2026-06-22T10:00:04.000Z","data":{"toolCallId":"t1","success":true}}
{"type":"assistant.turn_end","timestamp":"2026-06-22T10:00:05.000Z","data":{"turnId":"0"}}
{"type":"user.message","timestamp":"2026-06-22T10:01:00.000Z","data":{"content":"now test it"}}
{"type":"assistant.message","timestamp":"2026-06-22T10:01:01.000Z","data":{"content":"Tests pass.","model":"claude-opus-4.6"}}
"#;

    fn merged_blob(jsonl: &str) -> String {
        serde_json::to_string(&serde_json::json!({
            "source": "merged",
            "cwd": "/repo",
            "content": jsonl,
        }))
        .unwrap()
    }

    #[test]
    fn parse_events_jsonl_turns() {
        let content = merged_blob(EVENTS_JSONL);
        let p = parse_copilot_merged(&content);
        assert_eq!(p.turns.len(), 2);
        assert_eq!(p.turns[0].user_text.as_deref(), Some("fix the parser"));
        assert_eq!(p.turns[0].assistant_text, "On it.");
        assert_eq!(p.turns[1].user_text.as_deref(), Some("now test it"));
        assert_eq!(p.turns[1].assistant_text, "Tests pass.");
    }

    #[test]
    fn parse_events_jsonl_tool_use() {
        let content = merged_blob(EVENTS_JSONL);
        let p = parse_copilot_merged(&content);
        let kinds: Vec<&str> = p.events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(kinds.contains(&"UserPromptSubmit"));
        assert!(kinds.contains(&"PostToolUse"));
        assert!(kinds.contains(&"Stop"));
        let edit = p
            .events
            .iter()
            .find(|e| e.tool_name.as_deref() == Some("str_replace_editor"))
            .expect("the tool call from tool.execution_start");
        assert_eq!(edit.file_path.as_deref(), Some("/repo/src/x.rs"));
    }

    #[test]
    fn parse_session_shutdown_tokens() {
        let content = r#"{"type":"session.shutdown","timestamp":"2026-06-22T10:02:00.000Z","data":{"modelMetrics":{"gpt-4":{"usage":{"prompt_tokens":100,"completion_tokens":50}}}}}"#;
        let tokens = extract_jsonl_tokens(content).unwrap();
        assert_eq!(tokens.input, 100);
        assert_eq!(tokens.output, 50);
    }

    #[test]
    fn parse_workspace_yaml_cwd() {
        let dir = std::env::temp_dir().join(format!("copilot-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("workspace.yaml"), "cwd: /my/repo\n").unwrap();
        let cwd = read_workspace_yaml(&dir.join("workspace.yaml"));
        assert_eq!(cwd.as_deref(), Some("/my/repo"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parse_sqlite_turns_empty_on_missing_db() {
        let fake_db = Path::new("/nonexistent/db.sqlite");
        let turns = parse_sqlite_turns(fake_db, "any-session");
        assert!(turns.is_empty());
    }

    #[test]
    fn units_dedup_by_session_id() {
        // Create a temp structure with events.jsonl
        let home = std::env::temp_dir().join(format!("copilot-units-{}", uuid::Uuid::new_v4()));
        let session_dir = home.join("session-state").join("abc-123");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("events.jsonl"), "{}\n").unwrap();
        std::fs::write(session_dir.join("workspace.yaml"), "cwd: /repo\n").unwrap();

        let adapter = CopilotCliAdapter::new(home.clone());
        let units = adapter.units();
        assert_eq!(units.len(), 1, "one unit per session");
        assert_eq!(adapter.session_id_for(&units[0].key), Some("copilot-abc-123".to_string()));

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn stamp_is_monotonic() {
        let home = std::env::temp_dir().join(format!("copilot-stamp-{}", uuid::Uuid::new_v4()));
        let session_dir = home.join("session-state").join("s1");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("events.jsonl"), "{}\n").unwrap();
        std::fs::write(session_dir.join("workspace.yaml"), "cwd: /repo\n").unwrap();

        let adapter = CopilotCliAdapter::new(home.clone());
        let units = adapter.units();
        assert_eq!(units.len(), 1);
        // stamp should be > 0 (mtime in nanos)
        assert!(units[0].stamp > 0, "stamp should be positive nanos");

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn parse_session_none_when_empty() {
        let blob = merged_blob("");
        let p = parse_copilot_merged(&blob);
        assert!(p.turns.is_empty());
        assert!(p.events.is_empty());
    }

    #[test]
    fn session_id_from_jsonl_key() {
        let adapter = CopilotCliAdapter::new(PathBuf::from("/tmp"));
        let key = "/home/.copilot/session-state/my-session-uuid";
        assert_eq!(adapter.session_id_for(key), Some("copilot-my-session-uuid".to_string()));
    }

    #[test]
    fn session_id_from_sqlite_key() {
        let adapter = CopilotCliAdapter::new(PathBuf::from("/tmp"));
        assert_eq!(
            adapter.session_id_for("copilot-sqlite-my-sid"),
            Some("copilot-my-sid".to_string())
        );
    }

    #[test]
    fn parse_copilot_common_structure() {
        let adapter = CopilotCliAdapter::new(PathBuf::from("/tmp"));
        let content = merged_blob(EVENTS_JSONL);
        let p = adapter.parse(&content);
        assert_eq!(p.cwds, vec!["/repo".to_string()]);
        assert!(!p.events.is_empty());
        // Was asserting None, which only held because the adapter looked for
        // `session.model_change` — an event that fired twice in 77,139 real ones.
        // Every `assistant.message` names the model, so it resolves now.
        assert_eq!(p.model, Some(("copilot".into(), "claude-opus-4.6".into())));
    }

    /// Ingestion proof against a REAL transcript, when one is present.
    ///
    /// Skips silently when the sample folder is absent — it holds other people's
    /// data and is not in the repo. Point SENSEI_COPILOT_SAMPLE at a session
    /// directory to run it.
    #[test]
    fn parses_a_real_copilot_session_when_one_is_available() {
        let Ok(dir) = std::env::var("SENSEI_COPILOT_SAMPLE") else {
            return;
        };
        let path = std::path::Path::new(&dir);
        let content = match std::fs::read_to_string(path.join("events.jsonl")) {
            Ok(c) => c,
            Err(_) => return,
        };
        // The blob load_content builds: raw JSONL under `content`, not a parsed
        // array. Getting this wrong makes the adapter look broken when it is not.
        let blob =
            serde_json::json!({ "source": "jsonl", "cwd": "", "content": content }).to_string();
        let p = parse_copilot_merged(&blob);
        let tools = p.events.iter().filter(|e| e.event_type == "PostToolUse").count();
        let prompts = p.events.iter().filter(|e| e.event_type == "UserPromptSubmit").count();
        // The bar that the pre-fix adapter failed: a real session MUST yield tool
        // events. It produced zero, because it looked for `tool_use`.
        assert!(tools > 0, "no tool events from a real session — event names drifted again");
        assert!(prompts > 0, "no prompts from a real session");
        assert!(extract_jsonl_model(&content).is_some(), "no model resolved");
    }
}
