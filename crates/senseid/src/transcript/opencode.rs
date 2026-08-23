//! OpenCode transcript adapter — ingests OpenCode's SQLite-backed sessions so
//! the analyzer covers a **multi-model** Go agent (OpenCode v1.18.x).
//!
//! Storage: `~/.local/share/opencode/opencode.db` (single global SQLite DB,
//! Drizzle ORM schema). Key tables:
//! - `session` — `id TEXT PK, project_id TEXT FK, parent_id TEXT, directory TEXT,
//!   title TEXT, model TEXT, agent TEXT, time_created INT, time_updated INT,
//!   tokens_input INT, tokens_output INT, cost REAL, ...`
//! - `message` — `id TEXT PK, session_id TEXT FK, time_created INT, time_updated INT,
//!   data TEXT (JSON: {role, time, modelID, providerID, path, tokens, ...})`
//! - `part` — `id TEXT PK, message_id TEXT FK, session_id TEXT FK, time_created INT,
//!   time_updated INT, data TEXT (JSON: {type, text|tool|reasoning, ...})`
//!
//! Part types: `text` (assistant prose), `tool` (tool calls with state),
//! `reasoning` (thinking), `step-start`/`step-finish`, `file`, `compaction`, `patch`.
//! Tool parts: `{type:"tool", tool:"Read", callID:"...", state:{status:"completed",
//! input:{...}, output:"..."}}`.
//!
//! `parent_id` links compacted sessions (auto-compact). We only ingest top-level
//! sessions (parent_id IS NULL) to avoid double-counting the summarized portion.
//!
//! Timestamps are unix milliseconds.

use super::{ParsedTranscript, SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, UnitRef};
use std::path::PathBuf;

/// Cap stored assistant prose per turn (matches Claude/Zed adapters).
const MAX_TURN_CHARS: usize = 50_000;

pub struct OpenCodeAdapter {
    db_path: PathBuf,
}

impl OpenCodeAdapter {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn open(&self) -> Option<rusqlite::Connection> {
        rusqlite::Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| tracing::debug!(error = %e, db = %self.db_path.display(), "opencode: open opencode.db failed"))
        .ok()
    }
}

impl TranscriptAdapter for OpenCodeAdapter {
    fn source(&self) -> &'static str {
        "opencode"
    }
    fn family(&self) -> &'static str {
        "opencode"
    }

    fn units(&self) -> Vec<UnitRef> {
        let Some(conn) = self.open() else {
            return Vec::new();
        };
        // Only top-level sessions (no parent) — compacted child sessions are
        // summaries of the parent and would double-count.
        let mut stmt = match conn.prepare(
            "SELECT id, time_updated FROM session WHERE parent_id IS NULL ORDER BY time_created DESC",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "opencode: prepare units query failed");
                return Vec::new();
            }
        };
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        });
        let Ok(rows) = rows else { return Vec::new() };
        rows.flatten()
            .filter_map(|(id, time_updated)| {
                if time_updated > 0 { Some(UnitRef { key: id, stamp: time_updated }) } else { None }
            })
            .collect()
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT time_updated FROM session WHERE id = ?1",
            [key],
            |r| r.get(0),
        )
        .ok()
        .filter(|&v: &i64| v > 0)
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        Some(format!("opencode-{key}"))
    }

    fn load_content(&self, key: &str) -> Option<String> {
        let conn = self.open()?;
        // Fetch session metadata + all messages with their parts, ordered by time.
        // We need session info (directory, model) + message data + part data.
        let mut stmt = conn
            .prepare(
                "SELECT s.directory, s.model, s.agent, s.project_id, \
                        m.id, m.data, m.time_created, \
                        p.data, p.time_created, \
                        s.tokens_input, s.tokens_output, \
                        s.tokens_reasoning, s.tokens_cache_read, s.tokens_cache_write \
                 FROM session s \
                 JOIN message m ON m.session_id = s.id \
                 LEFT JOIN part p ON p.message_id = m.id \
                 WHERE s.id = ?1 \
                 ORDER BY m.time_created ASC, p.time_created ASC",
            )
            .ok()?;
        let rows = stmt
            .query_map([key], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,   // directory
                    r.get::<_, Option<String>>(1)?,   // model
                    r.get::<_, Option<String>>(2)?,   // agent
                    r.get::<_, Option<String>>(3)?,   // project_id
                    r.get::<_, String>(4)?,            // message.id
                    r.get::<_, String>(5)?,            // message.data
                    r.get::<_, i64>(6)?,               // message.time_created
                    r.get::<_, Option<String>>(7)?,   // part.data
                    r.get::<_, Option<i64>>(8)?,      // part.time_created
                    r.get::<_, Option<i64>>(9)?,      // session.tokens_input
                    r.get::<_, Option<i64>>(10)?,     // session.tokens_output
                    r.get::<_, Option<i64>>(11)?,     // session.tokens_reasoning
                    r.get::<_, Option<i64>>(12)?,     // session.tokens_cache_read
                    r.get::<_, Option<i64>>(13)?,     // session.tokens_cache_write
                ))
            })
            .ok()?;
        // Build a JSON object with session metadata + messages + parts.
        let mut msgs: Vec<serde_json::Value> = Vec::new();
        let mut cur_msg_id: Option<String> = None;
        let mut cur_msg: Option<serde_json::Value> = None;
        let mut directory: Option<String> = None;
        let mut session_model: Option<String> = None;
        let mut session_agent: Option<String> = None;
        let mut project_id: Option<String> = None;
        let mut tokens_input: Option<i64> = None;
        let mut tokens_output: Option<i64> = None;
        let mut tokens_reasoning: Option<i64> = None;
        let mut tokens_cache_read: Option<i64> = None;
        let mut tokens_cache_write: Option<i64> = None;
        let mut meta_seen = false;

        for row in rows.flatten() {
            if !meta_seen {
                meta_seen = true;
                directory = row.0;
                session_model = row.1;
                session_agent = row.2;
                project_id = row.3;
                tokens_input = row.9;
                tokens_output = row.10;
                tokens_reasoning = row.11;
                tokens_cache_read = row.12;
                tokens_cache_write = row.13;
            }
            let msg_id = row.4;
            let msg_data: serde_json::Value = serde_json::from_str(&row.5).unwrap_or(serde_json::json!({}));
            let msg_time = row.6;
            let part_data: Option<serde_json::Value> = row.7.map(|d| serde_json::from_str(&d).unwrap_or(serde_json::json!({})));
            let part_time = row.8;

            if cur_msg_id.as_deref() != Some(&msg_id) {
                if let Some(m) = cur_msg.take() {
                    msgs.push(m);
                }
                cur_msg_id = Some(msg_id.clone());
                cur_msg = Some(serde_json::json!({
                    "id": msg_id,
                    "data": msg_data,
                    "time_created": msg_time,
                    "parts": Vec::<serde_json::Value>::new(),
                }));
            }
            if let (Some(ref mut m), Some(pd)) = (cur_msg.as_mut(), part_data)
                && let Some(parts) = m.get_mut("parts").and_then(|p| p.as_array_mut())
            {
                parts.push(serde_json::json!({
                    "data": pd,
                    "time_created": part_time,
                }));
            }
        }
        if let Some(m) = cur_msg.take() {
            msgs.push(m);
        }

        if msgs.is_empty() {
            return None;
        }
        serde_json::to_string(&serde_json::json!({
            "directory": directory,
            "model": session_model,
            "agent": session_agent,
            "project_id": project_id,
            "tokens_input": tokens_input,
            "tokens_output": tokens_output,
            "tokens_reasoning": tokens_reasoning,
            "tokens_cache_read": tokens_cache_read,
            "tokens_cache_write": tokens_cache_write,
            "messages": msgs,
        }))
        .ok()
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        let session = parse_opencode_session(content);
        ParsedTranscript {
            turns: parse_opencode_messages(content),
            cwds: session.as_ref().map(|s| s.cwds.clone()).unwrap_or_default(),
            events: session.map(|s| s.events).unwrap_or_default(),
            model: extract_dominant_model(content),
            tokens: extract_tokens(content),
        }
    }
}

// ── Parsing ──────────────────────────────────────────────────────────────────

/// Parse the full content blob (session metadata + messages + parts) into
/// user-prompt-bounded turns.
pub fn parse_opencode_messages(content: &str) -> Vec<TranscriptTurn> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let msgs = match root.get("messages").and_then(|m| m.as_array()) {
        Some(m) => m,
        None => return Vec::new(),
    };
    let mut turns: Vec<TranscriptTurn> = Vec::new();
    let mut cur: Option<TranscriptTurn> = None;
    let mut idx = 0i32;

    for msg in msgs {
        let msg_data = msg.get("data").and_then(|d| d.as_object());
        let role = msg_data.and_then(|d| d.get("role")).and_then(|r| r.as_str()).unwrap_or("");
        let time_created = msg.get("time_created").and_then(|t| t.as_i64());

        match role {
            "user" => {
                // Check if this user message has only summary/diff parts (no real text).
                // A user message with only a summary and no text content is a
                // compaction boundary — not a real prompt.
                let has_text = msg.get("parts")
                    .and_then(|p| p.as_array())
                    .map(|parts| {
                        parts.iter().any(|part| {
                            let pd = part.get("data");
                            let typ = pd.and_then(|d| d.get("type")).and_then(|t| t.as_str()).unwrap_or("");
                            typ == "text"
                        })
                    })
                    .unwrap_or(false);
                if !has_text {
                    continue;
                }
                let text = extract_text_from_parts(msg);
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(t) = cur.take() {
                    turns.push(t);
                }
                idx += 1;
                cur = Some(TranscriptTurn {
                    turn_index: idx,
                    user_text: Some(trimmed.to_string()),
                    assistant_text: String::new(),
                    started_at: time_created.and_then(millis_to_datetime),
                        ..Default::default()
                    });
            }
            "assistant" => {
                if let Some(t) = cur.as_mut() {
                    let text = extract_text_from_parts(msg);
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        if !t.assistant_text.is_empty() {
                            t.assistant_text.push_str("\n\n");
                        }
                        t.assistant_text.push_str(trimmed);
                    }
                }
            }
            _ => {} // system, tool — not turn boundaries
        }
    }
    if let Some(t) = cur.take() {
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

/// Reconstruct a session's event stream from the content blob: a
/// `UserPromptSubmit` per user text message, a `PostToolUse` per `tool` part
/// in assistant messages, and a terminal `Stop`. Timestamps come from
/// `time_created` (unix ms).
pub fn parse_opencode_session(content: &str) -> Option<SynthSession> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(content) else {
        return None;
    };
    let directory = root.get("directory").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let msgs = root.get("messages").and_then(|m| m.as_array())?;
    let mut events: Vec<SynthEvent> = Vec::new();
    let mut max_ts: i64 = 0;

    for msg in msgs {
        let msg_data = msg.get("data").and_then(|d| d.as_object());
        let role = msg_data.and_then(|d| d.get("role")).and_then(|r| r.as_str()).unwrap_or("");
        let time_created = msg.get("time_created").and_then(|t| t.as_i64()).unwrap_or(0);

        match role {
            "user" => {
                let has_text = msg.get("parts")
                    .and_then(|p| p.as_array())
                    .map(|parts| {
                        parts.iter().any(|part| {
                            let pd = part.get("data");
                            let typ = pd.and_then(|d| d.get("type")).and_then(|t| t.as_str()).unwrap_or("");
                            typ == "text"
                        })
                    })
                    .unwrap_or(false);
                if !has_text {
                    continue;
                }
                let text = extract_text_from_parts(msg);
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                max_ts = max_ts.max(time_created);
                events.push(SynthEvent {
                    event_type: "UserPromptSubmit".into(),
                    tool_name: None,
                    file_path: None,
                    prompt: Some(trimmed.to_string()),
                    tool_input: None,
                    ts: time_created,
                });
            }
            "assistant" => {
                max_ts = max_ts.max(time_created);
                if let Some(parts) = msg.get("parts").and_then(|p| p.as_array()) {
                    for part in parts {
                        let pd = part.get("data");
                        let typ = pd.and_then(|d| d.get("type")).and_then(|t| t.as_str()).unwrap_or("");
                        if typ == "tool" {
                            let tool_name = pd.and_then(|d| d.get("tool")).and_then(|t| t.as_str()).unwrap_or("").to_string();
                            let state = pd.and_then(|d| d.get("state"));
                            let tool_input = state.and_then(|s| s.get("input")).cloned();
                            let file_path = tool_input
                                .as_ref()
                                .and_then(|i| {
                                    for k in ["file_path", "path", "command", "filePath"] {
                                        if let Some(p) = i.get(k).and_then(|p| p.as_str())
                                            && !p.is_empty()
                                        {
                                            return Some(p.to_string());
                                        }
                                    }
                                    None
                                });
                            let part_time = part.get("time_created").and_then(|t| t.as_i64()).unwrap_or(time_created);
                            if !tool_name.is_empty() {
                                events.push(SynthEvent {
                                    event_type: "PostToolUse".into(),
                                    tool_name: Some(tool_name),
                                    file_path,
                                    prompt: None,
                                    tool_input,
                                    ts: part_time,
                                });
                                max_ts = max_ts.max(part_time);
                            }
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
    events.push(SynthEvent {
        event_type: "Stop".into(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts: max_ts,
    });
    Some(SynthSession { cwds: if directory.is_empty() { Vec::new() } else { vec![directory] }, events })
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract concatenated text from `text`-type parts in a message.
fn extract_text_from_parts(msg: &serde_json::Value) -> String {
    let Some(parts) = msg.get("parts").and_then(|p| p.as_array()) else {
        return String::new();
    };
    let mut s = String::new();
    for part in parts {
        let pd = part.get("data");
        let typ = pd.and_then(|d| d.get("type")).and_then(|t| t.as_str()).unwrap_or("");
        if typ == "text"
            && let Some(text) = pd.and_then(|d| d.get("text")).and_then(|t| t.as_str())
        {
            let text = text.trim();
            if !text.is_empty() {
                if !s.is_empty() {
                    s.push('\n');
                }
                s.push_str(text);
            }
        }
    }
    s
}

/// The dominant model across assistant messages (most frequent).
pub fn extract_dominant_model(content: &str) -> Option<(String, String)> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(content) else {
        return None;
    };
    // Check session-level model first.
    if let Some(model) = root.get("model").and_then(|m| m.as_str())
        && !model.is_empty()
    {
        // OpenCode stores session.model either as a JSON object
        // {"id": "<model>", "providerID": "<provider>", ...} or, older, as a plain
        // "provider/model" string. Prefer the structured id/providerID.
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(model)
            && let Some(id) = obj.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        {
            let provider = obj.get("providerID").and_then(|v| v.as_str()).unwrap_or("opencode");
            return Some((provider.to_string(), id.to_string()));
        }
        let parts: Vec<&str> = model.splitn(2, '/').collect();
        return if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            Some(("opencode".to_string(), model.to_string()))
        };
    }
    // Fall back to message-level model.
    let msgs = root.get("messages").and_then(|m| m.as_array())?;
    use std::collections::HashMap;
    let mut counts: HashMap<String, u32> = HashMap::new();
    for msg in msgs {
        let msg_data = msg.get("data").and_then(|d| d.as_object());
        let role = msg_data.and_then(|d| d.get("role")).and_then(|r| r.as_str()).unwrap_or("");
        if role != "assistant" {
            continue;
        }
        // Try modelID + providerID from message data.
        let model_id = msg_data.and_then(|d| d.get("modelID")).and_then(|m| m.as_str());
        let provider_id = msg_data.and_then(|d| d.get("providerID")).and_then(|p| p.as_str());
        if let (Some(mid), Some(pid)) = (model_id, provider_id)
            && !mid.is_empty()
        {
            let key = format!("{}/{}", pid, mid);
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(model, _)| {
            let parts: Vec<&str> = model.splitn(2, '/').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                ("opencode".to_string(), model)
            }
        })
}

/// Session-level `(tokens_in, tokens_out)` from the content blob (OpenCode stores
/// per-session cumulative token totals on `session`). To match the cross-adapter
/// definition ([`crate::transcript::claude::claude_tokens`]): `tokens_in` = all
/// input the model processed = `tokens_input` + `tokens_cache_read` +
/// `tokens_cache_write` (cache reads/writes are real input tokens, just cheaper);
/// `tokens_out` = all generated = `tokens_output` + `tokens_reasoning` (thinking is
/// output-side). `None` when the session carries no usage (honest-empty — never a
/// fabricated 0).
pub fn extract_tokens(content: &str) -> Option<(i64, i64)> {
    let root: serde_json::Value = serde_json::from_str(content).ok()?;
    let g = |k: &str| root.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    let tokens_in = g("tokens_input") + g("tokens_cache_read") + g("tokens_cache_write");
    let tokens_out = g("tokens_output") + g("tokens_reasoning");
    (tokens_in > 0 || tokens_out > 0).then_some((tokens_in, tokens_out))
}

fn millis_to_datetime(ms: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::from_timestamp_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal content blob matching the real load_content output.
    fn sample_session() -> String {
        serde_json::json!({
            "directory": "/Users/jerry/project",
            "model": "anthropic/claude-sonnet-4-20250514",
            "agent": "build",
            "project_id": "abc123",
            "messages": [
                {
                    "id": "msg1",
                    "data": {"role": "user", "time": {"created": 1719060000000_i64}},
                    "time_created": 1719060000000_i64,
                    "parts": [
                        {"data": {"type": "text", "text": "fix the parser"}, "time_created": 1719060000000_i64}
                    ]
                },
                {
                    "id": "msg2",
                    "data": {"role": "assistant", "modelID": "claude-sonnet-4-20250514", "providerID": "anthropic", "time": {"created": 1719060001000_i64}},
                    "time_created": 1719060001000_i64,
                    "parts": [
                        {"data": {"type": "tool", "tool": "Edit", "callID": "tc1", "state": {"status": "completed", "input": {"file_path": "/repo/src/x.rs"}, "output": "ok"}}, "time_created": 1719060001000_i64},
                        {"data": {"type": "text", "text": "Fixed the parser."}, "time_created": 1719060002000_i64}
                    ]
                },
                {
                    "id": "msg3",
                    "data": {"role": "user", "time": {"created": 1719060002000_i64}},
                    "time_created": 1719060002000_i64,
                    "parts": [
                        {"data": {"type": "text", "text": "now add docs"}, "time_created": 1719060002000_i64}
                    ]
                },
                {
                    "id": "msg4",
                    "data": {"role": "assistant", "modelID": "claude-sonnet-4-20250514", "providerID": "anthropic", "time": {"created": 1719060003000_i64}},
                    "time_created": 1719060003000_i64,
                    "parts": [
                        {"data": {"type": "text", "text": "Added docs."}, "time_created": 1719060003000_i64}
                    ]
                }
            ]
        })
        .to_string()
    }

    #[test]
    fn parse_pairs_turns_from_user_prompts() {
        let turns = parse_opencode_messages(&sample_session());
        assert_eq!(turns.len(), 2, "two user prompts ⇒ two turns");
        assert_eq!(turns[0].turn_index, 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("fix the parser"));
        assert_eq!(turns[0].assistant_text, "Fixed the parser.");
        assert_eq!(turns[1].user_text.as_deref(), Some("now add docs"));
        assert_eq!(turns[1].assistant_text, "Added docs.");
    }

    #[test]
    fn parse_starts_at_from_created_at() {
        let turns = parse_opencode_messages(&sample_session());
        let dt = turns[0].started_at.unwrap();
        assert_eq!(dt.timestamp_millis(), 1719060000000);
    }

    #[test]
    fn tool_parts_are_not_turn_boundaries() {
        let content = serde_json::json!({
            "directory": "/repo",
            "messages": [
                {"id":"m1","data":{"role":"user"},"time_created":1000_i64,"parts":[{"data":{"type":"text","text":"run tests"},"time_created":1000_i64}]},
                {"id":"m2","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":1100_i64,"parts":[
                    {"data":{"type":"tool","tool":"Bash","callID":"tc1","state":{"status":"completed","input":{"command":"cargo test"},"output":"2 passed"}},"time_created":1100_i64},
                    {"data":{"type":"text","text":"All tests pass."},"time_created":1200_i64}
                ]},
                {"id":"m3","data":{"role":"user"},"time_created":1300_i64,"parts":[{"data":{"type":"text","text":"now add docs"},"time_created":1300_i64}]},
                {"id":"m4","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":1400_i64,"parts":[{"data":{"type":"text","text":"Added."},"time_created":1400_i64}]}
            ]
        })
        .to_string();
        let turns = parse_opencode_messages(&content);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].user_text.as_deref(), Some("run tests"));
        assert_eq!(turns[0].assistant_text, "All tests pass.");
        assert_eq!(turns[1].user_text.as_deref(), Some("now add docs"));
        assert_eq!(turns[1].assistant_text, "Added.");
    }

    #[test]
    fn parse_session_reconstructs_events() {
        let s = parse_opencode_session(&sample_session()).unwrap();
        let kinds: Vec<&str> = s.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(
            kinds,
            vec!["UserPromptSubmit", "PostToolUse", "UserPromptSubmit", "Stop"],
        );
        assert_eq!(s.events[0].prompt.as_deref(), Some("fix the parser"));
        let tool = s.events.iter().find(|e| e.event_type == "PostToolUse").unwrap();
        assert_eq!(tool.tool_name.as_deref(), Some("Edit"));
        assert_eq!(tool.file_path.as_deref(), Some("/repo/src/x.rs"));
        assert_eq!(s.cwds, vec!["/Users/jerry/project"]);
        let stop = s.events.last().unwrap();
        assert_eq!(stop.ts, 1719060003000);
    }

    #[test]
    fn parse_session_none_when_empty() {
        assert!(parse_opencode_session("{}").is_none());
        assert!(parse_opencode_session("not json").is_none());
        let empty = serde_json::json!({"directory":"/repo","messages":[]}).to_string();
        assert!(parse_opencode_session(&empty).is_none());
    }

    #[test]
    fn model_extraction_from_session_level() {
        let content = serde_json::json!({
            "model": "anthropic/claude-sonnet-4-20250514",
            "messages": []
        })
        .to_string();
        let m = extract_dominant_model(&content).unwrap();
        assert_eq!(m, ("anthropic".to_string(), "claude-sonnet-4-20250514".to_string()));
    }

    #[test]
    fn model_extraction_from_session_level_json_object() {
        // OpenCode's current schema stores session.model as a serialized JSON object;
        // the load_content blob carries it verbatim as a string. Parse out id/providerID.
        let model_json = serde_json::json!({"id": "some-model", "providerID": "opencode", "variant": "default"}).to_string();
        let content = serde_json::json!({ "model": model_json, "messages": [] }).to_string();
        let m = extract_dominant_model(&content).unwrap();
        assert_eq!(m, ("opencode".to_string(), "some-model".to_string()), "structured id/providerID wins over the raw JSON blob");
    }

    #[test]
    fn model_extraction_from_message_level() {
        let content = serde_json::json!({
            "messages": [
                {"id":"m1","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":1000_i64,"parts":[]},
                {"id":"m2","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":1100_i64,"parts":[]},
                {"id":"m3","data":{"role":"assistant","modelID":"gpt-4o","providerID":"openai"},"time_created":1200_i64,"parts":[]}
            ]
        })
        .to_string();
        let m = extract_dominant_model(&content).unwrap();
        assert_eq!(m, ("anthropic".to_string(), "sonnet".to_string()));
    }

    #[test]
    fn model_extraction_returns_none_for_no_models() {
        let content = serde_json::json!({
            "messages": [
                {"id":"m1","data":{"role":"user"},"time_created":1000_i64,"parts":[{"data":{"type":"text","text":"hi"},"time_created":1000_i64}]}
            ]
        })
        .to_string();
        assert!(extract_dominant_model(&content).is_none());
    }

    #[test]
    fn empty_turns_for_no_text_messages() {
        let content = serde_json::json!({
            "directory": "/repo",
            "messages": [
                {"id":"m1","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":1000_i64,"parts":[
                    {"data":{"type":"tool","tool":"Bash","callID":"tc1","state":{"status":"completed","input":{},"output":""}},"time_created":1000_i64}
                ]}
            ]
        })
        .to_string();
        let turns = parse_opencode_messages(&content);
        assert!(turns.is_empty(), "no user prompts ⇒ no turns");
    }

    #[test]
    fn reasoning_parts_excluded_from_text() {
        let content = serde_json::json!({
            "directory": "/repo",
            "messages": [
                {"id":"m1","data":{"role":"user"},"time_created":1000_i64,"parts":[{"data":{"type":"text","text":"think about this"},"time_created":1000_i64}]},
                {"id":"m2","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":1100_i64,"parts":[
                    {"data":{"type":"reasoning","thinking":"hmm..."},"time_created":1100_i64},
                    {"data":{"type":"text","text":"Here's my answer."},"time_created":1100_i64}
                ]}
            ]
        })
        .to_string();
        let turns = parse_opencode_messages(&content);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].assistant_text, "Here's my answer.", "reasoning excluded");
    }

    #[test]
    fn summary_user_messages_skipped() {
        // User messages with only summary (no text parts) are compaction boundaries.
        let content = serde_json::json!({
            "directory": "/repo",
            "messages": [
                {"id":"m1","data":{"role":"user","summary":{"title":"Init","diffs":[]}},"time_created":1000_i64,"parts":[]},
                {"id":"m2","data":{"role":"user"},"time_created":2000_i64,"parts":[{"data":{"type":"text","text":"real prompt"},"time_created":2000_i64}]},
                {"id":"m3","data":{"role":"assistant","modelID":"sonnet","providerID":"anthropic"},"time_created":2100_i64,"parts":[{"data":{"type":"text","text":"Response."},"time_created":2100_i64}]}
            ]
        })
        .to_string();
        let turns = parse_opencode_messages(&content);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("real prompt"));
    }

    #[test]
    fn extract_tokens_sums_cache_and_reasoning_else_none() {
        // Real OpenCode session shape: cache reads/writes fold into input; reasoning into output.
        let with = serde_json::json!({
            "tokens_input": 3474319, "tokens_output": 334525, "tokens_reasoning": 607047,
            "tokens_cache_read": 128858240, "tokens_cache_write": 0, "messages": []
        }).to_string();
        // in = 3474319 + 128858240 + 0 = 132332559; out = 334525 + 607047 = 941572
        assert_eq!(extract_tokens(&with), Some((132_332_559, 941_572)));
        // plain input/output with no cache/reasoning keys still works
        let plain = serde_json::json!({"tokens_input": 100, "tokens_output": 40, "messages": []}).to_string();
        assert_eq!(extract_tokens(&plain), Some((100, 40)));
        // absent or all-zero → None (honest-empty, never a fabricated 0)
        assert_eq!(extract_tokens(&serde_json::json!({"messages": []}).to_string()), None);
        assert_eq!(extract_tokens(&serde_json::json!({"tokens_input": 0, "tokens_output": 0}).to_string()), None);
    }
}
