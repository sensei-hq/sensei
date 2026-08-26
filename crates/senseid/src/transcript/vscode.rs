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
    SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, TurnFacts, UnitRef,
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

/// Every chat file under a bare `workspaceStorage/` root — both the delta
/// journals and the newer transcript event streams.
fn scan_workspace_storage(ws: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(entries) = std::fs::read_dir(ws) else {
        return paths;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        for sub in ["chatSessions", "GitHub.copilot-chat/transcripts"] {
            if let Ok(files) = std::fs::read_dir(dir.join(sub)) {
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
/// The project folder a chat session belongs to.
///
/// Delegates to the shared reader: `workspace.json` sits beside the
/// `chatSessions` DIRECTORY, and this resolved one level too shallow, so the
/// folder came back `None`, `parse()` yielded no cwds, and `synthesize_session`
/// skipped every VS Code session before creating it (#123 A2).
fn workspace_folder(chat_session_path: &Path) -> Option<String> {
    sensei_transcript_formats::paths::workspace_folder(chat_session_path)
}

/// Reconstruct turns from a delta journal (kind:0/1/2 format).
///
/// The replay itself lives in `sensei-transcript-formats` — the daemon and the
/// offline report tool used to carry separate copies that disagreed, and three
/// of the four critical defects in #123 were cases where one was right and the
/// other was not.
fn parse_journal_transcript(content: &str) -> Vec<TranscriptTurn> {
    let mut turns = Vec::new();
    for req in sensei_transcript_formats::journal::requests(content) {
        if req.user_text.is_empty() && req.assistant_text.is_empty() {
            continue;
        }
        let mut assistant_text = req.assistant_text;
        if assistant_text.chars().count() > MAX_TURN_CHARS {
            assistant_text = assistant_text.chars().take(MAX_TURN_CHARS).collect();
        }
        let mut facts = TurnFacts::default();
        if req.thinking_ms > 0 {
            facts.effort = Some(format!("{}ms reasoning", req.thinking_ms));
        }
        turns.push(TranscriptTurn {
            turn_index: req.index as i32 + 1,
            user_text: (!req.user_text.is_empty()).then_some(req.user_text),
            assistant_text,
            started_at: req.timestamp_ms.and_then(chrono::DateTime::from_timestamp_millis),
            attrs: serde_json::json!({
                "modelId": req.model,
                "toolInvocations": req.tool_calls.len(),
                "responseTimestamp": req.response_timestamp_ms,
            }),
            facts,
        });
    }
    turns
}

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
    // The journal DOES carry per-message timestamps — `timestamp` on each
    // request, read into TranscriptTurn::started_at. The previous code started
    // at 0 and added a second per turn, so every VS Code session landed on
    // 1970-01-01 and poisoned active-days, session duration and every
    // time-keyed metric downstream (#123 A4).
    //
    // A turn whose timestamp the journal genuinely omits contributes no event
    // rather than a made-up one: a fabricated instant is indistinguishable from
    // a real one once it is in the database.
    let mut events = Vec::new();
    let mut last_ts = None;
    for t in &turns {
        let Some(at) = t.started_at.map(|d| d.timestamp_millis()) else { continue };
        last_ts = Some(at);
        if let Some(ref prompt) = t.user_text {
            events.push(SynthEvent {
                event_type: "UserPromptSubmit".to_string(),
                tool_name: None,
                file_path: None,
                prompt: Some(prompt.clone()),
                tool_input: None,
                ts: at,
            });
        }
    }
    if events.is_empty() {
        return None;
    }
    // The session ends when the last reply came back, where the journal says so.
    let stop_ts =
        turns.iter().filter_map(|t| t.attrs["responseTimestamp"].as_i64()).max().or(last_ts)?;
    events.push(SynthEvent {
        event_type: "Stop".to_string(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts: stop_ts,
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

        // An explicit root OVERRIDES the installed-editor scan. Without this the
        // field was inert and the adapter could only ever read this machine's
        // own VS Code — so a shared transcript folder could not be ingested or
        // tested against. Accepts either a `User/` directory or a bare
        // `workspaceStorage/` one, which is the shape people actually send.
        if self.root.components().next().is_some() && self.root.exists() {
            let user_root = if self.root.join("workspaceStorage").is_dir() {
                self.root.clone()
            } else {
                // Treat the folder itself as workspaceStorage by borrowing its
                // parent as the notional User/ root.
                self.root.parent().map(Path::to_path_buf).unwrap_or_else(|| self.root.clone())
            };
            let ws_root = if self.root.join("workspaceStorage").is_dir() {
                user_root.clone()
            } else {
                self.root.clone()
            };
            for p in scan_workspace_storage(&ws_root) {
                if let Some(stamp) = mtime_ns(&p) {
                    units.push(UnitRef { key: p.display().to_string(), stamp });
                }
            }
            for p in glob_chat_sessions(&user_root) {
                if let Some(stamp) = mtime_ns(&p) {
                    let key = p.display().to_string();
                    if !units.iter().any(|u| u.key == key) {
                        units.push(UnitRef { key, stamp });
                    }
                }
            }
            return units;
        }

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
            "v": {
                "requests": [{
                    "modelId": "copilot/claude-opus-4.6",
                    "timestamp": 1785928643959i64,
                    "message": {"text": "hello"},
                    "response": [{"value": "hi there"}]
                }]
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
        // The shape VS Code actually writes: `v` not `value`, one request per
        // EXCHANGE (no `role`), reply text in untagged `response[]` parts.
        let journal = serde_json::json!({
            "kind": 0,
            "v": {
                "requests": [{
                    "modelId": "copilot/claude-opus-4.6",
                    "timestamp": 1785928643959i64,
                    "responseTimestamp": 1785928645000i64,
                    "message": {"text": "go"},
                    "response": [
                        {"kind": "thinking", "value": "hmm", "reasoningDurationMs": 2187},
                        {"kind": "toolInvocationSerialized", "toolId": "manage_todo_list"},
                        {"value": "going"}
                    ]
                }]
            }
        });
        let adapter = VscodeAdapter::new(PathBuf::from("/tmp"));
        let p = adapter.parse(&serde_json::to_string(&journal).unwrap());
        assert_eq!(p.turns.len(), 1);
        assert_eq!(p.turns[0].user_text.as_deref(), Some("go"));
        assert_eq!(p.turns[0].assistant_text, "going");
        // Thinking and tool parts are facts, never prose.
        assert!(!p.turns[0].assistant_text.contains("hmm"));
        assert_eq!(p.turns[0].attrs["toolInvocations"], 1);
        assert_eq!(p.turns[0].attrs["modelId"], "claude-opus-4.6");
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

    /// A4: the journal carries a real `timestamp` per request, so nothing may
    /// synthesise one. Events used to start at epoch 0 and step by a second,
    /// landing every VS Code session on 1970-01-01 and poisoning active-days,
    /// duration and every time-keyed metric.
    #[test]
    fn journal_events_carry_the_real_timestamp_not_a_synthesised_one() {
        let journal = concat!(
            r#"{"kind":0,"v":{"requests":[{"timestamp":1786294114560,"#,
            r#""responseTimestamp":1786294120000,"message":{"text":"go"},"#,
            r#""response":[{"value":"done"}]}]}}"#
        );
        let session = parse_journal_session(journal, Some("/repo".into()))
            .expect("a journal with one request is a session");

        let prompt = session
            .events
            .iter()
            .find(|e| e.event_type == "UserPromptSubmit")
            .expect("the prompt is an event");
        assert_eq!(prompt.ts, 1786294114560, "must be the journal's timestamp, not 0");

        let stop = session.events.iter().find(|e| e.event_type == "Stop").expect("a Stop event");
        assert_eq!(stop.ts, 1786294120000, "the session ends when the reply came back");
    }

    /// A turn the journal gave no timestamp for contributes nothing rather than
    /// a fabricated instant — once in the database the two are indistinguishable.
    #[test]
    fn a_journal_without_timestamps_yields_no_session() {
        let journal = r#"{"kind":0,"v":{"requests":[{"message":{"text":"go"}}]}}"#;
        assert!(parse_journal_session(journal, Some("/repo".into())).is_none());
    }

    /// A2's consequence: the folder has to resolve, or `parse()` yields no cwds
    /// and `synthesize_session` drops the session before creating it.
    #[test]
    fn a_chat_session_resolves_its_workspace_folder() {
        let root = std::env::temp_dir().join("senseid-vscode-a2/hash1");
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
        std::fs::create_dir_all(root.join("chatSessions")).unwrap();
        std::fs::write(root.join("workspace.json"), r#"{"folder":"file:///repo/app"}"#).unwrap();
        let journal = root.join("chatSessions").join("s.jsonl");
        std::fs::write(&journal, "").unwrap();

        assert_eq!(
            workspace_folder(&journal),
            Some("/repo/app".to_string()),
            "workspace.json is beside the chatSessions directory, not inside it"
        );
    }

    /// A1: the `k` path is untrusted — the daemon reads whatever journal is on
    /// disk. A huge index must be REFUSED, not allocated: without the ceiling
    /// this resizes the array to the requested length, and at 4e9 the daemon is
    /// OOM-killed by a two-line file.
    ///
    /// The index is deliberately only a little over the cap: large enough that
    /// the guard is what rejects it, small enough that a regression fails this
    /// test rather than taking the machine down with it.
    #[test]
    fn a_huge_journal_index_is_refused_not_allocated() {
        let journal = concat!(
            "{\"kind\":0,\"v\":{\"requests\":[]}}\n",
            "{\"kind\":1,\"k\":[\"requests\",2000000,\"message\"],\"v\":{\"text\":\"x\"}}"
        );
        assert!(
            parse_journal_transcript(journal).is_empty(),
            "an out-of-range index must be skipped, not backfilled with two million nulls"
        );
    }

    #[test]
    fn parse_journal_transcript_simple() {
        let journal = serde_json::json!({
            "kind": 0,
            "v": {
                "requests": [{
                    "modelId": "copilot/claude-opus-4.6",
                    "timestamp": 1785928643959i64,
                    "responseTimestamp": 1785928645000i64,
                    "message": {"text": "do it"},
                    "response": [
                        {"kind": "thinking", "value": "hmm", "reasoningDurationMs": 2187},
                        {"kind": "toolInvocationSerialized", "toolId": "manage_todo_list"},
                        {"value": "done now"}
                    ]
                }]
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

    #[test]
    fn windows_workspace_uris_decode_to_a_usable_path() {
        // Real shape from a shared transcript: VS Code writes Windows folders as
        // file:///c%3A/... Leaving the escape in place produced "/c%3A/Users/..."
        // which matches no directory, so every Windows session lost its project.
        assert_eq!(
            sensei_transcript_formats::paths::normalise_uri_path(
                "/c%3A/Users/dev.user/Documents/workspace/sample-portal"
            ),
            "c:/Users/dev.user/Documents/workspace/sample-portal"
        );
        // POSIX paths pass through untouched.
        assert_eq!(
            sensei_transcript_formats::paths::normalise_uri_path("/Users/jane/code/app"),
            "/Users/jane/code/app"
        );
        // A space, the other escape that shows up constantly.
        assert_eq!(
            sensei_transcript_formats::paths::normalise_uri_path("/Users/jane/My%20Code"),
            "/Users/jane/My Code"
        );
    }

    /// Ingestion proof against REAL VS Code data, when a sample is provided.
    ///
    /// Skips when unset — the samples are other people's transcripts and are not
    /// in the repo. Point SENSEI_VSCODE_SAMPLE at a `workspaceStorage`-shaped
    /// folder (one directory per workspace hash).
    #[test]
    fn discovers_units_in_a_real_workspace_storage_folder() {
        let Ok(dir) = std::env::var("SENSEI_VSCODE_SAMPLE") else {
            return;
        };
        let adapter = VscodeAdapter::new(PathBuf::from(&dir));
        let units = adapter.units();
        // Count what is on disk and require the adapter to find ALL of it. The
        // weaker "not empty" would pass while silently missing a whole layer —
        // and the two layers live in different subdirectories.
        let mut expected = 0usize;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                for sub in ["chatSessions", "GitHub.copilot-chat/transcripts"] {
                    if let Ok(files) = std::fs::read_dir(e.path().join(sub)) {
                        expected += files
                            .flatten()
                            .filter(|f| f.path().extension().is_some_and(|x| x == "jsonl"))
                            .count();
                    }
                }
            }
        }
        assert!(expected > 0, "the sample at {dir} has no chat files to find");
        assert_eq!(
            units.len(),
            expected,
            "adapter found {} of {expected} chat files under {dir}",
            units.len()
        );
        for u in units.iter().take(20) {
            assert!(
                adapter.session_id_for(&u.key).is_some(),
                "every unit must resolve a session id: {}",
                u.key
            );
        }
    }

    /// Ingestion proof over REAL journals, when a sample is provided.
    ///
    /// Skips when unset. Point SENSEI_VSCODE_SAMPLE at a `workspaceStorage`
    /// folder — the shape people actually send.
    #[test]
    fn parses_turns_from_real_chat_journals() {
        let Ok(dir) = std::env::var("SENSEI_VSCODE_SAMPLE") else {
            return;
        };
        let adapter = VscodeAdapter::new(PathBuf::from(&dir));
        let mut with_turns = 0usize;
        let mut turns = 0usize;
        let mut models = std::collections::HashSet::new();
        let mut checked = 0usize;
        for u in adapter.units() {
            if !u.key.contains("chatSessions") {
                continue;
            }
            checked += 1;
            let Some(content) = adapter.load_content(&u.key) else { continue };
            let p = adapter.parse(&content);
            if !p.turns.is_empty() {
                with_turns += 1;
                turns += p.turns.len();
                for t in &p.turns {
                    if let Some(m) = t.attrs["modelId"].as_str() {
                        models.insert(m.to_string());
                    }
                }
            }
        }
        assert!(checked > 0, "no chat journals under {dir}");
        // The bar the pre-fix parser failed: reading `path`/`value` and a `role`
        // field that does not exist reconstructed NOTHING from any real journal.
        assert!(
            with_turns > 0,
            "parsed 0 turns from {checked} real journals — the journal format drifted again"
        );
        assert!(turns >= with_turns, "every journal with turns yields at least one");
        assert!(!models.is_empty(), "no modelId recovered from any turn");
    }
}
