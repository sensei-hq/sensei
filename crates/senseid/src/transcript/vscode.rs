//! VS Code (Copilot Chat) transcript adapter. Reads from three storage layers:
//!
//! 1. **Chat session journals** (delta journal format):
//!    `~/Library/Application Support/Code/User/workspaceStorage/<hash>/chatSessions/<uuid>.jsonl`
//!    Delta journal: `kind:0` = root snapshot, `kind:1` = set value at path,
//!    `kind:2` = replace array slice. Reconstruct `requests[]` for turns.
//!
//! 2. **Newer transcript format** (event stream):
//!    `~/Library/Application Support/Code/User/workspaceStorage/<hash>/GitHub.copilot-chat/transcripts/<session>.jsonl`
//!    First line must have `type == "session.start"` and `data.producer == "copilot-agent"`.
//!
//! 3. **OTel span store** (richest — real token counts):
//!    `~/Library/Application Support/Code/User/globalStorage/github.copilot-chat/agent-traces.db`
//!    SQLite with `spans` table: `traceId`, `spanId`, `startTimeUnixNano`, attributes as JSON.
//!
//! Covers all VS Code variants: Code, Code - Insiders, VSCodium, Code - OSS.
//! Also covers `~/.vscode-server/data/User` for remote/SSH.

use super::{
    MAX_LINE_BYTES, MAX_TRANSCRIPT_BYTES, MAX_TURN_CHARS, ParsedTranscript, SessionTokens,
    SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, TurnFacts, UnitRef, merge_facts,
    parse_timestamp,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn mtime_ns(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos() as i64)
}

/// VS Code variant names to scan.
const VARIANTS: &[&str] = &["Code", "Code - Insiders", "VSCodium", "Code - OSS"];

/// Check if a first line looks like a delta journal entry (has `"kind":` field).
fn is_delta_journal(first_line: &str) -> bool {
    first_line.contains("\"kind\":")
}

/// Check if a first line is a transcript event stream start.
fn is_transcript_event_stream(first_line: &str) -> bool {
    first_line.contains("\"session.start\"") || first_line.contains("\"type\":\"session.start\"")
}

/// Path to the OTel agent-traces.db for a variant.
fn otel_db_path(variant: &str) -> Option<PathBuf> {
    let base = super::vscode_user_root(variant)?;
    let db = base.join("globalStorage/github.copilot-chat/agent-traces.db");
    if db.exists() { Some(db) } else { None }
}

/// Query distinct session IDs from the OTel SQLite DB.
fn query_otel_sessions(db_path: &Path) -> Vec<String> {
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let mut stmt =
        match conn.prepare("SELECT DISTINCT traceId FROM spans WHERE traceId IS NOT NULL") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

    stmt.query_map([], |row| row.get::<_, String>(0))
        .map(|r| r.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Glob for chat session journal files under a User root.
fn glob_chat_sessions(user_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    // workspaceStorage/<hash>/chatSessions/<uuid>.jsonl
    let ws = user_root.join("workspaceStorage");
    if let Ok(entries) = std::fs::read_dir(&ws) {
        for entry in entries.flatten() {
            let chat_dir = entry.path().join("chatSessions");
            if let Ok(files) = std::fs::read_dir(&chat_dir) {
                for f in files.flatten() {
                    let p = f.path();
                    if p.is_file() && p.extension().is_some_and(|e| e == "jsonl") {
                        paths.push(p);
                    }
                }
            }
        }
    }
    // globalStorage/emptyWindowChatSessions/<uuid>.jsonl
    let empty = user_root.join("globalStorage/emptyWindowChatSessions");
    if let Ok(files) = std::fs::read_dir(&empty) {
        for f in files.flatten() {
            let p = f.path();
            if p.is_file() && p.extension().is_some_and(|e| e == "jsonl") {
                paths.push(p);
            }
        }
    }
    paths
}

/// Glob for transcript event stream files.
fn glob_transcripts(user_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let ws = user_root.join("workspaceStorage");
    if let Ok(entries) = std::fs::read_dir(&ws) {
        for entry in entries.flatten() {
            let tx_dir = entry.path().join("GitHub.copilot-chat/transcripts");
            if let Ok(files) = std::fs::read_dir(&tx_dir) {
                for f in files.flatten() {
                    let p = f.path();
                    if p.is_file() && p.extension().is_some_and(|e| e == "jsonl") {
                        paths.push(p);
                    }
                }
            }
        }
    }
    paths
}

/// Extract session ID from a journal/transcript file path (filename stem).
fn session_id_from_path(path: &Path) -> Option<String> {
    path.file_stem()?.to_str().map(str::to_string)
}

/// Read workspace.json sibling to get the folder URI for a chat session.
fn workspace_folder(chat_session_path: &Path) -> Option<String> {
    let ws = chat_session_path.parent()?.join("workspace.json");
    let content = std::fs::read_to_string(&ws).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let uri = v.get("folder").and_then(|f| f.as_str())?;
    // Resolve file:// URIs
    if let Some(path) = uri.strip_prefix("file://") {
        return Some(path.to_string());
    }
    // Handle vscode-remote://wsl+<distro>/path
    if let Some(rest) = uri.strip_prefix("vscode-remote://")
        && let Some(slash) = rest.find('/')
    {
        return Some(rest[slash..].to_string());
    }
    Some(uri.to_string())
}

/// Reconstruct turns from a delta journal (kind:0/1/2 format).
/// The journal reconstructs a `requests[]` array; we extract user/assistant turns.
fn parse_journal_transcript(content: &str) -> Vec<TranscriptTurn> {
    // Reconstruct the full JSON state from delta operations
    let mut root: serde_json::Value = serde_json::json!(null);
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        let op: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let kind = op.get("kind").and_then(|k| k.as_i64()).unwrap_or(0);
        let path = op.get("path").and_then(|p| p.as_str()).unwrap_or("");
        match kind {
            0 => {
                // Root snapshot
                if let Some(value) = op.get("value") {
                    root = value.clone();
                }
            }
            1 => {
                // Set value at JSON path
                if let Some(value) = op.get("value") {
                    set_json_path(&mut root, path, value.clone());
                }
            }
            2 => {
                // Replace array slice
                if let Some(value) = op.get("value") {
                    set_json_path(&mut root, path, value.clone());
                }
            }
            _ => {}
        }
    }

    // Extract turns from reconstructed requests[]
    let requests = match root.get("requests") {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        _ => return Vec::new(),
    };

    let mut turns = Vec::new();
    let mut cur: Option<TranscriptTurn> = None;
    let mut cur_facts = TurnFacts::default();

    for req in &requests {
        let message = match req.get("message") {
            Some(m) => m,
            None => continue,
        };
        let text = message.get("text").and_then(|t| t.as_str()).unwrap_or("");

        // Check if this is a user message
        let role = req.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role == "user" && !text.is_empty() {
            if let Some(mut t) = cur.take() {
                t.facts = std::mem::take(&mut cur_facts);
                turns.push(t);
            }
            cur = Some(TranscriptTurn {
                turn_index: turns.len() as i32 + 1,
                user_text: Some(text.to_string()),
                assistant_text: String::new(),
                started_at: None,
                attrs: serde_json::json!({}),
                facts: TurnFacts::default(),
            });
        } else if role == "assistant" {
            // Extract text from response parts
            let mut assistant_text = String::new();
            if let Some(parts) = req.get("responseParts").and_then(|p| p.as_array()) {
                for part in parts {
                    if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                        let t = t.trim();
                        if !t.is_empty() {
                            if !assistant_text.is_empty() {
                                assistant_text.push_str("\n\n");
                            }
                            assistant_text.push_str(t);
                        }
                    }
                }
            }
            if !assistant_text.is_empty()
                && let Some(ref mut t) = cur
            {
                if !t.assistant_text.is_empty() {
                    t.assistant_text.push_str("\n\n");
                }
                t.assistant_text.push_str(&assistant_text);
            }
            // Promote metadata from response metadata
            if let Some(meta) = req.get("resultMetadata") {
                merge_facts(&mut cur_facts, meta);
            }
        }
    }
    if let Some(mut t) = cur.take() {
        t.facts = std::mem::take(&mut cur_facts);
        turns.push(t);
    }

    // Cap pathological turns
    for t in turns.iter_mut() {
        if t.assistant_text.chars().count() > MAX_TURN_CHARS {
            let mut s: String = t.assistant_text.chars().take(MAX_TURN_CHARS).collect();
            s.push('…');
            t.assistant_text = s;
        }
    }
    turns
}

/// Set a value at a JSON path (dot-separated keys, numeric indices for arrays).
fn set_json_path(root: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    if path.is_empty() {
        *root = value;
        return;
    }
    let parts: Vec<&str> = path.trim_start_matches('.').split('.').collect();
    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Set the value
            if let Ok(idx) = part.parse::<usize>() {
                if let serde_json::Value::Array(arr) = current
                    && idx < arr.len()
                {
                    arr[idx] = value;
                }
            } else if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), value);
            }
            return;
        }
        // Navigate deeper
        if let Ok(idx) = part.parse::<usize>() {
            if let serde_json::Value::Array(arr) = current {
                if idx < arr.len() {
                    current = &mut arr[idx];
                } else {
                    return;
                }
            } else {
                return;
            }
        } else if let Some(obj) = current.as_object_mut() {
            current = obj.entry(part.to_string()).or_insert(serde_json::json!({}));
        } else {
            return;
        }
    }
}

/// Reconstruct turns from a transcript event stream.
fn parse_transcript_content(content: &str) -> Vec<TranscriptTurn> {
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

/// Reconstruct turns from OTel spans (richest source — real token counts).
fn parse_otel_content(content: &str) -> Vec<TranscriptTurn> {
    let root: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let spans = match root.get("spans").and_then(|s| s.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    let mut turns = Vec::new();
    let mut cur: Option<TranscriptTurn> = None;
    let mut cur_facts = TurnFacts::default();

    for span in spans {
        let attrs = match span.get("attributes").and_then(|a| a.as_object()) {
            Some(a) => a,
            None => continue,
        };

        let role = attrs.get("gen_ai.system").and_then(|r| r.as_str()).unwrap_or("");
        let content_text = attrs.get("gen_ai.content").and_then(|c| c.as_str()).unwrap_or("");

        if role == "user" && !content_text.is_empty() {
            if let Some(mut t) = cur.take() {
                t.facts = std::mem::take(&mut cur_facts);
                turns.push(t);
            }
            cur = Some(TranscriptTurn {
                turn_index: turns.len() as i32 + 1,
                user_text: Some(content_text.to_string()),
                assistant_text: String::new(),
                started_at: None,
                attrs: serde_json::json!({ "otel_span_id": span.get("spanId") }),
                facts: TurnFacts::default(),
            });
        } else if role == "assistant" && !content_text.is_empty() {
            if let Some(ref mut t) = cur {
                if !t.assistant_text.is_empty() {
                    t.assistant_text.push_str("\n\n");
                }
                t.assistant_text.push_str(content_text);
            }
            // Extract real token counts from span attributes
            if let Some(usage) = attrs.get("gen_ai.usage") {
                cur_facts.tokens_in = usage.get("input_tokens").and_then(|v| v.as_i64());
                cur_facts.tokens_out = usage.get("output_tokens").and_then(|v| v.as_i64());
                cur_facts.cache_read =
                    usage.get("cache_read_input_tokens").and_then(|v| v.as_i64());
            }
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

/// Reconstruct session (cwds + events) from a delta journal.
fn parse_journal_session(content: &str, cwd: Option<String>) -> Option<SynthSession> {
    let turns = parse_journal_transcript(content);
    if turns.is_empty() {
        return None;
    }
    let mut events = Vec::new();
    let mut ts = 0i64;
    for t in &turns {
        if let Some(ref prompt) = t.user_text {
            events.push(SynthEvent {
                event_type: "UserPromptSubmit".to_string(),
                tool_name: None,
                file_path: None,
                prompt: Some(prompt.clone()),
                tool_input: None,
                ts,
            });
        }
        if !t.assistant_text.is_empty() {
            // No tool_use in journal format — just text
        }
        ts += 1000; // Spread events evenly (no per-message timestamps in journals)
    }
    events.push(SynthEvent {
        event_type: "Stop".to_string(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts,
    });
    let cwds = cwd.into_iter().collect();
    Some(SynthSession { cwds, events })
}

/// Reconstruct session from a transcript event stream.
fn parse_transcript_session(content: &str) -> Option<SynthSession> {
    let mut cwds = Vec::new();
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

        // Collect cwd from session.start
        if ev_type == "session.start"
            && let Some(ctx) = v.get("data").and_then(|d| d.get("context"))
            && let Some(cwd) = ctx.get("cwd").and_then(|c| c.as_str())
            && !cwd.is_empty()
            && !cwds.contains(&cwd.to_string())
        {
            cwds.push(cwd.to_string());
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
            "tool.execution_start" => {
                let name = v
                    .get("data")
                    .and_then(|d| d.get("toolName"))
                    .and_then(|n| n.as_str())
                    .map(str::to_string);
                events.push(SynthEvent {
                    event_type: "PostToolUse".to_string(),
                    tool_name: name,
                    file_path: None,
                    prompt: None,
                    tool_input: None,
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

/// Reconstruct session from OTel spans.
fn parse_otel_session(content: &str) -> Option<SynthSession> {
    let root: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let spans = root.get("spans").and_then(|s| s.as_array())?;

    let mut events = Vec::new();
    let mut max_ts = 0i64;

    for span in spans {
        let ts = span
            .get("startTimeUnixNano")
            .and_then(|t| t.as_i64())
            .map(|n| n / 1_000_000) // Convert nanos to millis
            .unwrap_or(0);
        if ts > max_ts {
            max_ts = ts;
        }

        let attrs = match span.get("attributes").and_then(|a| a.as_object()) {
            Some(a) => a,
            None => continue,
        };
        let role = attrs.get("gen_ai.system").and_then(|r| r.as_str()).unwrap_or("");

        match role {
            "user" => {
                let prompt =
                    attrs.get("gen_ai.content").and_then(|c| c.as_str()).map(str::to_string);
                events.push(SynthEvent {
                    event_type: "UserPromptSubmit".to_string(),
                    tool_name: None,
                    file_path: None,
                    prompt,
                    tool_input: None,
                    ts,
                });
            }
            "assistant" => {
                // Check for tool calls in the span
                if let Some(tools) = attrs.get("gen_ai.tool.calls").and_then(|t| t.as_array()) {
                    for tool in tools {
                        let name = tool.get("name").and_then(|n| n.as_str()).map(str::to_string);
                        let input = tool.get("input").cloned();
                        events.push(SynthEvent {
                            event_type: "PostToolUse".to_string(),
                            tool_name: name,
                            file_path: None,
                            prompt: None,
                            tool_input: input,
                            ts,
                        });
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
        event_type: "Stop".to_string(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts: max_ts,
    });
    Some(SynthSession { cwds: Vec::new(), events })
}

/// Extract model from journal content (selectedModel in root).
fn extract_journal_model(content: &str) -> Option<(String, String)> {
    // The journal is delta-compressed — try to find selectedModel in any line
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim())
            && let Some(model) = v.get("selectedModel").and_then(|m| m.as_str())
        {
            return Some(("copilot".to_string(), model.to_string()));
        }
    }
    None
}

/// Extract model from transcript event stream (session.start event).
fn extract_transcript_model(content: &str) -> Option<(String, String)> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
            && v.get("type").and_then(|t| t.as_str()) == Some("session.start")
        {
            let model = v.get("data").and_then(|d| d.get("model")).and_then(|m| m.as_str())?;
            return Some(("copilot".to_string(), model.to_string()));
        }
    }
    None
}

/// Extract model from OTel span attributes.
fn extract_otel_model(content: &str) -> Option<(String, String)> {
    let root: serde_json::Value = serde_json::from_str(content).ok()?;
    let spans = root.get("spans").and_then(|s| s.as_array())?;
    for span in spans {
        let attrs = span.get("attributes").and_then(|a| a.as_object())?;
        if let Some(model) = attrs.get("gen_ai.response.model").and_then(|m| m.as_str()) {
            return Some(("copilot".to_string(), model.to_string()));
        }
    }
    None
}

/// Extract tokens from OTel span attributes (richest source).
fn extract_otel_tokens(content: &str) -> Option<SessionTokens> {
    let root: serde_json::Value = serde_json::from_str(content).ok()?;
    let spans = root.get("spans").and_then(|s| s.as_array())?;
    let mut total_input = 0i64;
    let mut total_output = 0i64;
    let mut total_cache_read = 0i64;
    let mut found = false;

    for span in spans {
        let attrs = span.get("attributes").and_then(|a| a.as_object())?;
        if let Some(usage) = attrs.get("gen_ai.usage") {
            if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                total_input += input;
                found = true;
            }
            if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                total_output += output;
            }
            if let Some(cache) = usage.get("cache_read_input_tokens").and_then(|v| v.as_i64()) {
                total_cache_read += cache;
            }
        }
    }

    if found {
        Some(SessionTokens {
            input: total_input,
            output: total_output,
            cache_read: if total_cache_read > 0 { Some(total_cache_read) } else { None },
            cache_write: None,
            reasoning: None,
            cost: None,
        })
    } else {
        None
    }
}

pub struct VscodeAdapter {
    root: PathBuf,
}

impl VscodeAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl TranscriptAdapter for VscodeAdapter {
    fn source(&self) -> &'static str {
        "vscode"
    }

    fn family(&self) -> &'static str {
        "vscode"
    }

    fn units(&self) -> Vec<UnitRef> {
        let mut units = Vec::new();
        let mut otel_sessions: HashSet<String> = HashSet::new();

        // Pass 1: enumerate OTel sessions (per-session units)
        for variant in VARIANTS {
            if let Some(db_path) = otel_db_path(variant) {
                let sessions = query_otel_sessions(&db_path);
                for session_id in sessions {
                    otel_sessions.insert(session_id.clone());
                    if let Some(stamp) = mtime_ns(&db_path) {
                        units.push(UnitRef {
                            key: format!("{}#{}", db_path.display(), session_id),
                            stamp,
                        });
                    }
                }
            }
        }

        // Pass 2: journal + transcript units, but skip sessions OTel covers
        for variant in VARIANTS {
            let Some(root) = super::vscode_user_root(variant) else {
                continue;
            };
            // Journal units
            for path in glob_chat_sessions(&root) {
                if let Some(sid) = session_id_from_path(&path)
                    && otel_sessions.contains(&sid)
                {
                    continue;
                }
                if let Some(stamp) = mtime_ns(&path) {
                    units.push(UnitRef { key: path.to_string_lossy().to_string(), stamp });
                }
            }
            // Transcript units
            for path in glob_transcripts(&root) {
                if let Some(sid) = session_id_from_path(&path)
                    && otel_sessions.contains(&sid)
                {
                    continue;
                }
                if let Some(stamp) = mtime_ns(&path) {
                    units.push(UnitRef { key: path.to_string_lossy().to_string(), stamp });
                }
            }
        }

        units
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        // OTel keys are "<db_path>#<session_id>" — use the db file's mtime
        if let Some(db_path) = key.split('#').next() {
            return mtime_ns(Path::new(db_path));
        }
        mtime_ns(Path::new(key))
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        // OTel keys: "<db>#<session-id>" → use session-id after #
        if let Some(session_id) = key.split('#').nth(1) {
            return Some(format!("vscode-{}", session_id));
        }
        // Journal/transcript: use filename stem
        let path = Path::new(key);
        let stem = path.file_stem()?.to_str()?;
        Some(format!("vscode-{}", stem))
    }

    fn load_content(&self, key: &str) -> Option<String> {
        // OTel unit: key is "<db_path>#<session_id>"
        if let Some((db_path, session_id)) = key.split_once('#') {
            let conn = rusqlite::Connection::open_with_flags(
                Path::new(db_path),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .ok()?;
            let mut stmt = conn
                .prepare(
                    "SELECT spanId, startTimeUnixNano, attributes FROM spans WHERE traceId = ?1",
                )
                .ok()?;
            let spans: Vec<serde_json::Value> = stmt
                .query_map(rusqlite::params![session_id], |row| {
                    let span_id: String = row.get(0)?;
                    let start_ns: i64 = row.get(1)?;
                    let attrs_json: String = row.get(2)?;
                    Ok(serde_json::json!({
                        "spanId": span_id,
                        "startTimeUnixNano": start_ns,
                        "attributes": serde_json::from_str::<serde_json::Value>(&attrs_json)
                            .unwrap_or(serde_json::json!({})),
                    }))
                })
                .ok()?
                .filter_map(|r| r.ok())
                .collect();

            return serde_json::to_string(&serde_json::json!({
                "source": "otel_spans",
                "session_id": session_id,
                "spans": spans,
            }))
            .ok();
        }

        let path = Path::new(key);
        let meta = std::fs::metadata(path).ok()?;
        if meta.len() > MAX_TRANSCRIPT_BYTES {
            tracing::warn!(path = %key, size = meta.len(), "vscode: transcript exceeds MAX_TRANSCRIPT_BYTES, skipping");
            return None;
        }
        let raw = std::fs::read_to_string(path).ok()?;

        // Detect content type and wrap with source marker + cwd
        if let Some(first_line) = raw.lines().next() {
            let trimmed = first_line.trim();
            if is_delta_journal(trimmed) {
                let cwd = workspace_folder(path);
                return serde_json::to_string(&serde_json::json!({
                    "source": "journal",
                    "cwd": cwd,
                    "content": raw,
                }))
                .ok();
            }
            if is_transcript_event_stream(trimmed) {
                return serde_json::to_string(&serde_json::json!({
                    "source": "transcript",
                    "content": raw,
                }))
                .ok();
            }
        }
        // Fallback: treat as transcript
        serde_json::to_string(&serde_json::json!({
            "source": "transcript",
            "content": raw,
        }))
        .ok()
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        // Dispatch on the source marker in the content wrapper
        if let Ok(root) = serde_json::from_str::<serde_json::Value>(content) {
            let source = root.get("source").and_then(|s| s.as_str()).unwrap_or("");
            let inner = root.get("content").and_then(|c| c.as_str()).unwrap_or(content);
            let cwd = root.get("cwd").and_then(|c| c.as_str()).map(str::to_string);

            match source {
                "otel_spans" => {
                    return ParsedTranscript {
                        turns: parse_otel_content(content),
                        cwds: Vec::new(),
                        events: parse_otel_session(content).map_or_else(Vec::new, |s| s.events),
                        model: extract_otel_model(content),
                        tokens: extract_otel_tokens(content),
                    };
                }
                "journal" => {
                    return ParsedTranscript {
                        turns: parse_journal_transcript(inner),
                        cwds: cwd.clone().into_iter().collect(),
                        events: parse_journal_session(inner, cwd)
                            .map_or_else(Vec::new, |s| s.events),
                        model: extract_journal_model(inner),
                        tokens: None,
                    };
                }
                "transcript" => {
                    let session = parse_transcript_session(inner);
                    return ParsedTranscript {
                        turns: parse_transcript_content(inner),
                        cwds: session.as_ref().map_or_else(Vec::new, |s| s.cwds.clone()),
                        events: session.as_ref().map_or_else(Vec::new, |s| s.events.clone()),
                        model: extract_transcript_model(inner),
                        tokens: None,
                    };
                }
                _ => {}
            }
        }
        // Raw content (not wrapped) — try to detect type
        if let Some(first_line) = content.lines().next() {
            let trimmed = first_line.trim();
            if trimmed.contains("\"source\":\"otel_spans\"") {
                return ParsedTranscript {
                    turns: parse_otel_content(content),
                    cwds: Vec::new(),
                    events: parse_otel_session(content).map_or_else(Vec::new, |s| s.events),
                    model: extract_otel_model(content),
                    tokens: extract_otel_tokens(content),
                };
            }
            if is_delta_journal(trimmed) {
                return ParsedTranscript {
                    turns: parse_journal_transcript(content),
                    cwds: Vec::new(),
                    events: parse_journal_session(content, None)
                        .map_or_else(Vec::new, |s| s.events),
                    model: extract_journal_model(content),
                    tokens: None,
                };
            }
        }
        // Default: transcript event stream
        let session = parse_transcript_session(content);
        ParsedTranscript {
            turns: parse_transcript_content(content),
            cwds: session.as_ref().map_or_else(Vec::new, |s| s.cwds.clone()),
            events: session.as_ref().map_or_else(Vec::new, |s| s.events.clone()),
            model: extract_transcript_model(content),
            tokens: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSCRIPT_JSONL: &str = r#"{"type":"session.start","sessionId":"test-sid"}
{"type":"user.message","timestamp":"2026-06-22T10:00:00.000Z","data":{"content":"fix the parser"}}
{"type":"assistant.message","timestamp":"2026-06-22T10:00:02.000Z","data":{"content":"On it."}}
{"type":"user.message","timestamp":"2026-06-22T10:01:00.000Z","data":{"content":"now test it"}}
{"type":"assistant.message","timestamp":"2026-06-22T10:01:01.000Z","data":{"content":"Tests pass."}}
"#;

    #[test]
    fn is_delta_journal_detection() {
        assert!(is_delta_journal(r#"{"kind":0,"value":{}}"#));
        assert!(is_delta_journal(r#"{"kind":1,"path":"foo","value":42}"#));
        assert!(!is_delta_journal(r#"{"type":"user.message","data":{}}"#));
        assert!(!is_delta_journal(r#"not json at all"#));
    }

    #[test]
    fn is_transcript_event_stream_detection() {
        assert!(is_transcript_event_stream(r#"{"type":"session.start","sessionId":"x"}"#));
        assert!(is_transcript_event_stream(r#"{"session.start":{}}"#));
        assert!(!is_transcript_event_stream(r#"{"kind":0}"#));
    }

    #[test]
    fn parse_transcript_content_simple() {
        let turns = parse_transcript_content(TRANSCRIPT_JSONL);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].user_text.as_deref(), Some("fix the parser"));
        assert_eq!(turns[0].assistant_text, "On it.");
        assert_eq!(turns[1].user_text.as_deref(), Some("now test it"));
        assert_eq!(turns[1].assistant_text, "Tests pass.");
    }

    #[test]
    fn parse_transcript_content_turn_indices() {
        let turns = parse_transcript_content(TRANSCRIPT_JSONL);
        assert_eq!(turns[0].turn_index, 1);
        assert_eq!(turns[1].turn_index, 2);
    }

    #[test]
    fn parse_transcript_content_turn_count() {
        let turns = parse_transcript_content(TRANSCRIPT_JSONL);
        assert_eq!(turns.len(), 2);
    }

    #[test]
    fn parse_transcript_content_preserves_timestamps() {
        let turns = parse_transcript_content(TRANSCRIPT_JSONL);
        assert!(turns[0].started_at.is_some(), "user timestamp present");
        assert!(turns[1].started_at.is_some(), "second turn timestamp present");
    }

    #[test]
    fn parse_transcript_empty() {
        let turns = parse_transcript_content("");
        assert!(turns.is_empty());
    }

    #[test]
    fn wrapped_transcript_dispatch() {
        let inner = r#"{"type":"session.start","sessionId":"test-sid"}"#;
        let wrapper = serde_json::to_string(&serde_json::json!({
            "source": "transcript",
            "content": inner,
            "cwd": "/repo",
        }))
        .unwrap();
        let adapter = VscodeAdapter::new(PathBuf::from("/tmp"));
        let p = adapter.parse(&wrapper);
        assert_eq!(p.turns.len(), 0, "session.start alone has no user turn");
        // cwds come from session events, not the wrapper
        assert!(p.cwds.is_empty(), "no cwd in session.start");
    }

    #[test]
    fn wrapped_journal_dispatch() {
        // Delta journal with kind:0 root snapshot containing requests
        let journal = serde_json::json!({
            "kind": 0,
            "value": {
                "requests": [
                    {"role": "user", "message": {"text": "hello"}, "timestamp": "2026-06-22T10:00:00.000Z"},
                    {"role": "assistant", "message": {"text": "hi"}, "responseParts": [{"text": "hi there"}], "timestamp": "2026-06-22T10:00:02.000Z"}
                ]
            }
        });
        let journal_text = serde_json::to_string(&journal).unwrap();
        let wrapper = serde_json::to_string(&serde_json::json!({
            "source": "journal",
            "content": journal_text,
            "cwd": "/project",
        }))
        .unwrap();
        let adapter = VscodeAdapter::new(PathBuf::from("/tmp"));
        let p = adapter.parse(&wrapper);
        assert_eq!(p.turns.len(), 1);
        assert_eq!(p.turns[0].user_text.as_deref(), Some("hello"));
        assert_eq!(p.cwds, vec!["/project".to_string()]);
    }

    #[test]
    fn raw_delta_journal_dispatch() {
        let journal = serde_json::json!({
            "kind": 0,
            "value": {
                "requests": [
                    {"role": "user", "message": {"text": "go"}, "timestamp": "2026-06-22T10:00:00.000Z"},
                    {"role": "assistant", "message": {"text": "ok"}, "responseParts": [{"text": "going"}], "timestamp": "2026-06-22T10:00:02.000Z"}
                ]
            }
        });
        let adapter = VscodeAdapter::new(PathBuf::from("/tmp"));
        let p = adapter.parse(&serde_json::to_string(&journal).unwrap());
        assert_eq!(p.turns.len(), 1);
        assert_eq!(p.turns[0].user_text.as_deref(), Some("go"));
        assert_eq!(p.turns[0].assistant_text, "going");
    }

    #[test]
    fn parse_transcript_session_none_for_non_stream() {
        let adapter = VscodeAdapter::new(PathBuf::from("/tmp"));
        let p = adapter.parse(r#"{"kind":0,"value":{}}"#);
        assert!(p.turns.is_empty());
    }

    #[test]
    fn extract_transcript_model_from_content() {
        let content = r#"{"type":"session.start","data":{"model":"gpt-4o"},"sessionId":"s"}"#;
        let m = super::extract_transcript_model(content);
        assert!(m.is_some(), "should find model");
        let (provider, model) = m.unwrap();
        assert_eq!(provider, "copilot");
        assert_eq!(model, "gpt-4o");
    }

    #[test]
    fn session_id_from_path_stem() {
        let p = PathBuf::from("/repo/.vscode/copilot-chat-transcripts/abc-123.jsonl");
        assert_eq!(session_id_from_path(&p).as_deref(), Some("abc-123"));
    }

    #[test]
    fn session_id_none_for_empty_stem() {
        // session_id_from_path returns the file_stem — it cannot distinguish files
        // from dirs, but returns None for root paths with no stem.
        let p = PathBuf::from("/");
        assert!(session_id_from_path(&p).is_none());
    }

    #[test]
    fn parse_session_empty_for_garbage() {
        let s = parse_transcript_session("not json");
        assert!(s.is_none());
    }

    #[test]
    fn parse_journal_transcript_simple() {
        let journal = serde_json::json!({
            "kind": 0,
            "value": {
                "requests": [
                    {"role": "user", "message": {"text": "do it"}, "timestamp": "2026-06-22T10:00:00.000Z"},
                    {"role": "assistant", "message": {"text": "done"}, "responseParts": [{"text": "done now"}], "timestamp": "2026-06-22T10:00:02.000Z"}
                ]
            }
        });
        let turns = parse_journal_transcript(&serde_json::to_string(&journal).unwrap());
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("do it"));
        assert_eq!(turns[0].assistant_text, "done now");
    }

    #[test]
    fn parse_otel_content_turns() {
        let otel = serde_json::json!({
            "source": "otel_spans",
            "spans": [
                {"attributes": {"gen_ai.system": "user", "gen_ai.content": "fix the parser"}},
                {"attributes": {"gen_ai.system": "assistant", "gen_ai.content": "On it."}}
            ]
        });
        let turns = parse_otel_content(&serde_json::to_string(&otel).unwrap());
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("fix the parser"));
        assert_eq!(turns[0].assistant_text, "On it.");
    }

    #[test]
    fn parse_otel_tokens() {
        let otel = serde_json::json!({
            "source": "otel_spans",
            "spans": [{"attributes": {"gen_ai.usage": {"input_tokens": 100, "output_tokens": 50}}}],
            "model": "gpt-4"
        });
        let tokens = extract_otel_tokens(&serde_json::to_string(&otel).unwrap()).unwrap();
        assert_eq!(tokens.input, 100);
        assert_eq!(tokens.output, 50);
    }

    #[test]
    fn vs_code_parse_structure() {
        let transcript_with_model = r#"{"type":"session.start","data":{"model":"gpt-4o"},"sessionId":"test-sid"}
{"type":"user.message","timestamp":"2026-06-22T10:00:00.000Z","data":{"content":"fix the parser"}}
{"type":"assistant.message","timestamp":"2026-06-22T10:00:02.000Z","data":{"content":"On it."}}
"#;
        let adapter = VscodeAdapter::new(PathBuf::from("/tmp"));
        let wrapper = serde_json::to_string(&serde_json::json!({
            "source": "transcript",
            "content": transcript_with_model,
            "cwd": "/my/project",
        }))
        .unwrap();
        let p = adapter.parse(&wrapper);
        assert_eq!(p.turns.len(), 1);
        assert!(!p.events.is_empty());
        assert_eq!(p.model.as_ref().map(|(_, m)| m.as_str()), Some("gpt-4o"));
    }
}
