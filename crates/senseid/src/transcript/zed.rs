//! Zed transcript adapter — ingests Zed's Agent-panel threads so the analyzer
//! gets a **multi-model** corpus (Copilot/OpenAI/Anthropic/Google/Grok/Ollama),
//! not just Claude Code.
//!
//! Storage: `~/Library/Application Support/Zed/threads/threads.db` (SQLite). Each
//! `threads` row is one thread; the messages live in a **zstd-compressed JSON**
//! `data` BLOB. Everything the parser needs is inside that JSON:
//! - `version` — schema tag. Two message shapes seen in the wild: **flat**
//!   (`0.2.0`, `{id, role:"user"|"assistant", segments:[{type,text}], tool_uses}`)
//!   and **tagged** (`0.3.0`, `{"User"|"Agent": {content:[{"Text"|"Mention"|"ToolUse":..}]}}`).
//!   The parser detects the shape per-message (robust to version drift) rather
//!   than branching on `version`.
//! - `model` — `{provider, model}` (the multi-model signal, captured per turn).
//! - `initial_project_snapshot.worktree_snapshots[].worktree_path` — the cwd(s)
//!   for project resolution (present for 225/226 threads; `folder_paths` column
//!   is mostly empty and not needed).
//! - `initial_project_snapshot.timestamp` (start) + top-level `updated_at` (end)
//!   — there are no per-message timestamps, so session events are spread evenly
//!   across this window (correct duration; monotonic order).
//!
//! IO (SQLite read + zstd decompress) lives in the [`ZedAdapter`] impl; parsing
//! is pure + deterministic (these free functions) so it's unit-testable without
//! a database.

use super::{ParsedTranscript, SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, UnitRef};
use std::io::Read;
use std::path::PathBuf;

/// Cap stored assistant prose per turn (matches the Claude adapter).
const MAX_TURN_CHARS: usize = 50_000;
/// Skip a thread whose decompressed JSON exceeds this (pathological).
const MAX_THREAD_BYTES: usize = 16 * 1024 * 1024;

/// A normalized message — the common shape both schema versions collapse to.
#[derive(Debug, Clone, PartialEq)]
struct NormMsg {
    is_user: bool,
    text: String,
    tools: Vec<NormTool>,
}

#[derive(Debug, Clone, PartialEq)]
struct NormTool {
    name: String,
    file_path: Option<String>,
}

pub struct ZedAdapter {
    db_path: PathBuf,
}

impl ZedAdapter {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Open the threads DB read-only (it's a live DB Zed may be writing).
    fn open(&self) -> Option<rusqlite::Connection> {
        rusqlite::Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| tracing::debug!(error = %e, db = %self.db_path.display(), "zed: open threads.db failed"))
        .ok()
    }
}

impl TranscriptAdapter for ZedAdapter {
    fn source(&self) -> &'static str {
        "zed"
    }
    fn family(&self) -> &'static str {
        // `family` is the harness/agent (the `sensei.assistant_family` enum:
        // claude, cursor, zed, …) — here, Zed. The precise *model* a thread used
        // (provider + model, which varies per thread) is captured separately on
        // each turn via `model_for` → `transcript_turns.provider/model`.
        "zed"
    }

    fn units(&self) -> Vec<UnitRef> {
        let Some(conn) = self.open() else {
            return Vec::new();
        };
        let mut stmt = match conn.prepare("SELECT id, updated_at FROM threads") {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "zed: prepare units query failed");
                return Vec::new();
            }
        };
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        });
        let Ok(rows) = rows else { return Vec::new() };
        rows.flatten()
            .filter_map(|(id, updated_at)| {
                parse_iso_ms(&updated_at).map(|stamp| UnitRef { key: id, stamp })
            })
            .collect()
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        let conn = self.open()?;
        let updated_at: String = conn
            .query_row("SELECT updated_at FROM threads WHERE id = ?1", [key], |r| r.get(0))
            .ok()?;
        parse_iso_ms(&updated_at)
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        // Namespace under the source so a Zed thread uuid can never collide with
        // a Claude session uuid in `client_session_id`/`transcript_turns`.
        Some(format!("zed-{key}"))
    }

    fn load_content(&self, key: &str) -> Option<String> {
        let conn = self.open()?;
        let blob: Vec<u8> = conn
            .query_row("SELECT data FROM threads WHERE id = ?1", [key], |r| r.get(0))
            .map_err(|e| tracing::debug!(error = %e, thread = key, "zed: load data failed"))
            .ok()?;
        let json = decompress_zstd(&blob)?;
        if json.len() > MAX_THREAD_BYTES {
            tracing::warn!(thread = key, size_mb = json.len() / 1_048_576, "zed: skipping oversized thread");
            return None;
        }
        Some(json)
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        let session = parse_zed_session(content);
        ParsedTranscript {
            turns: parse_zed_thread(content),
            cwds: session.as_ref().map(|s| s.cwds.clone()).unwrap_or_default(),
            events: session.map(|s| s.events).unwrap_or_default(),
            model: extract_model(content),
            tokens: extract_tokens(content),
        }
    }
}

/// zstd-decompress a thread BLOB to its JSON text. `None` on a malformed stream
/// or non-UTF-8 output (logged, skipped — never panics on bad data).
fn decompress_zstd(blob: &[u8]) -> Option<String> {
    let mut decoder = ruzstd::StreamingDecoder::new(blob)
        .map_err(|e| tracing::warn!(error = %e, "zed: zstd init failed"))
        .ok()?;
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| tracing::warn!(error = %e, "zed: zstd decompress failed"))
        .ok()?;
    String::from_utf8(out)
        .map_err(|_| tracing::warn!("zed: thread JSON not valid UTF-8"))
        .ok()
}

/// Total `(tokens_in, tokens_out)` for a Zed thread. Zed records usage per model
/// request in `request_token_usage` (a `{request_id → {input_tokens, output_tokens,
/// [cache_*]}}` map); each request's input is the full context sent that turn, so
/// summing across requests is the total tokens *processed* — the same "consumption"
/// grain as the Claude adapter (which sums per-message usage). Prefers the
/// `cumulative_token_usage` summary when Zed has populated it. `None` when no usage
/// is recorded (honest-empty — never a fabricated 0). Pure.
pub fn extract_tokens(content: &str) -> Option<(i64, i64)> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    // A TokenUsage object → (input + cache tokens, output). Cache keys are absent in
    // current threads but summed defensively so a newer Zed schema is covered.
    let usage = |u: &serde_json::Value| -> (i64, i64) {
        let g = |k: &str| u.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
        (
            g("input_tokens") + g("cache_creation_input_tokens") + g("cache_read_input_tokens"),
            g("output_tokens"),
        )
    };
    // Prefer Zed's own running total when present.
    if let Some(obj) = v.get("cumulative_token_usage").and_then(|c| c.as_object())
        && !obj.is_empty()
    {
        let (tin, tout) = usage(v.get("cumulative_token_usage").unwrap());
        if tin > 0 || tout > 0 {
            return Some((tin, tout));
        }
    }
    // Else sum the per-request usage map.
    if let Some(reqs) = v.get("request_token_usage").and_then(|r| r.as_object()) {
        let (mut tin, mut tout) = (0i64, 0i64);
        for u in reqs.values() {
            let (i, o) = usage(u);
            tin += i;
            tout += o;
        }
        if tin > 0 || tout > 0 {
            return Some((tin, tout));
        }
    }
    None
}

/// `{provider, model}` of the thread, or `None` if absent.
pub fn extract_model(content: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    let m = v.get("model")?;
    let provider = m.get("provider").and_then(|p| p.as_str())?.to_string();
    let model = m.get("model").and_then(|p| p.as_str())?.to_string();
    Some((provider, model))
}

/// Parse a Zed thread JSON into user-prompt-bounded turns (mirrors the Claude
/// adapter's grain). Pure + deterministic.
pub fn parse_zed_thread(content: &str) -> Vec<TranscriptTurn> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let msgs = normalize_messages(&v);
    let mut turns: Vec<TranscriptTurn> = Vec::new();
    let mut cur: Option<TranscriptTurn> = None;
    let mut idx = 0i32;
    for m in &msgs {
        if m.is_user {
            if m.text.trim().is_empty() {
                continue; // a user message with only mentions/no prose isn't a turn boundary
            }
            if let Some(t) = cur.take() {
                turns.push(t);
            }
            idx += 1;
            cur = Some(TranscriptTurn {
                turn_index: idx,
                user_text: Some(m.text.trim().to_string()),
                assistant_text: String::new(),
                started_at: None, // Zed has no per-message timestamps,
                        ..Default::default()
                    });
        } else if let Some(t) = cur.as_mut() {
            let text = m.text.trim();
            if !text.is_empty() {
                if !t.assistant_text.is_empty() {
                    t.assistant_text.push_str("\n\n");
                }
                t.assistant_text.push_str(text);
            }
        }
    }
    if let Some(t) = cur.take() {
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

/// Reconstruct a session's event stream from a Zed thread (#75): a
/// `UserPromptSubmit` per user prompt, a `PostToolUse` per tool use, a terminal
/// `Stop`. cwds come from the worktree snapshot. Since Zed records no
/// per-message timestamps, events are spread evenly across
/// `[snapshot.timestamp, updated_at]` — monotonic order + correct duration. Pure.
pub fn parse_zed_session(content: &str) -> Option<SynthSession> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    let cwds = worktree_paths(&v);
    let msgs = normalize_messages(&v);

    // Build the (untimed) event sequence first, then stamp it.
    enum Ev {
        Prompt(String),
        Tool(NormTool),
    }
    let mut seq: Vec<Ev> = Vec::new();
    for m in &msgs {
        if m.is_user {
            if !m.text.trim().is_empty() {
                seq.push(Ev::Prompt(m.text.trim().to_string()));
            }
        } else {
            for t in &m.tools {
                seq.push(Ev::Tool(t.clone()));
            }
        }
    }
    if seq.is_empty() {
        return None;
    }

    let start = snapshot_start_ms(&v).unwrap_or(0);
    let end = v
        .get("updated_at")
        .and_then(|u| u.as_str())
        .and_then(parse_iso_ms)
        .unwrap_or(start)
        .max(start);
    let n = seq.len() as i64;
    let ts_at = |i: i64| -> i64 {
        if n <= 1 {
            start
        } else {
            start + (end - start) * i / (n - 1)
        }
    };

    let mut events: Vec<SynthEvent> = Vec::with_capacity(seq.len() + 1);
    for (i, ev) in seq.into_iter().enumerate() {
        let ts = ts_at(i as i64);
        match ev {
            Ev::Prompt(p) => events.push(SynthEvent {
                event_type: "UserPromptSubmit".into(),
                tool_name: None,
                file_path: None,
                prompt: Some(p),
                tool_input: None,
                ts,
            }),
            Ev::Tool(t) => events.push(SynthEvent {
                event_type: "PostToolUse".into(),
                tool_name: Some(t.name),
                file_path: t.file_path,
                prompt: None,
                tool_input: None, // the Zed thread parser carries only file_path (no full input yet)
                ts,
            }),
        }
    }
    events.push(SynthEvent {
        event_type: "Stop".into(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts: end,
    });
    Some(SynthSession { cwds, events })
}

/// Distinct worktree paths from `initial_project_snapshot.worktree_snapshots`.
fn worktree_paths(v: &serde_json::Value) -> Vec<String> {
    let mut cwds = Vec::new();
    if let Some(snaps) = v
        .get("initial_project_snapshot")
        .and_then(|s| s.get("worktree_snapshots"))
        .and_then(|w| w.as_array())
    {
        for s in snaps {
            if let Some(p) = s.get("worktree_path").and_then(|p| p.as_str())
                && !p.is_empty()
                && !cwds.iter().any(|c| c == p)
            {
                cwds.push(p.to_string());
            }
        }
    }
    cwds
}

fn snapshot_start_ms(v: &serde_json::Value) -> Option<i64> {
    v.get("initial_project_snapshot")
        .and_then(|s| s.get("timestamp"))
        .and_then(|t| t.as_str())
        .and_then(parse_iso_ms)
}

/// Normalize both schema shapes (tagged `User`/`Agent` vs flat `role`) into a
/// uniform message list. Detection is per-message, so mixed/evolving schemas
/// still parse.
fn normalize_messages(v: &serde_json::Value) -> Vec<NormMsg> {
    let Some(arr) = v.get("messages").and_then(|m| m.as_array()) else {
        return Vec::new();
    };
    arr.iter().filter_map(normalize_one).collect()
}

fn normalize_one(m: &serde_json::Value) -> Option<NormMsg> {
    let obj = m.as_object()?;
    // Tagged shape (v0.3.0): a single key "User" | "Agent" (and "Assistant"/"System" defensively).
    for (tag, is_user) in [("User", true), ("Agent", false), ("Assistant", false)] {
        if let Some(body) = obj.get(tag) {
            let content = body.get("content").and_then(|c| c.as_array());
            let (text, tools) = collect_tagged_content(content);
            return Some(NormMsg { is_user, text, tools });
        }
    }
    // Flat shape (v0.2.0): { role: "user"|"assistant", segments: [...], tool_uses: [...] }
    if let Some(role) = obj.get("role").and_then(|r| r.as_str()) {
        let is_user = role.eq_ignore_ascii_case("user");
        let text = collect_flat_text(obj.get("segments"));
        let tools = collect_flat_tools(obj.get("tool_uses"));
        return Some(NormMsg { is_user, text, tools });
    }
    None
}

/// v0.3.0 content: `[{"Text": str}, {"Mention": {..}}, {"ToolUse": {name, raw_input}}, {"Thinking": ..}]`.
/// Prose = `Text` segments; tools = `ToolUse` (with file_path mined from `raw_input` JSON).
fn collect_tagged_content(content: Option<&Vec<serde_json::Value>>) -> (String, Vec<NormTool>) {
    let mut text = String::new();
    let mut tools = Vec::new();
    let Some(segs) = content else {
        return (text, tools);
    };
    for seg in segs {
        let Some((tag, body)) = seg.as_object().and_then(|o| o.iter().next()) else {
            continue;
        };
        match tag.as_str() {
            "Text" => {
                if let Some(s) = body.as_str() {
                    push_prose(&mut text, s);
                }
            }
            "ToolUse" => {
                let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                let file_path = body
                    .get("raw_input")
                    .and_then(|r| r.as_str())
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    .as_ref()
                    .and_then(file_path_in_input);
                if !name.is_empty() {
                    tools.push(NormTool { name, file_path });
                }
            }
            _ => {} // Mention / Thinking / etc. — not prose, not a tool action
        }
    }
    (text, tools)
}

/// v0.2.0 segments: `[{type:"text", text:str}]`.
fn collect_flat_text(segments: Option<&serde_json::Value>) -> String {
    let mut text = String::new();
    if let Some(arr) = segments.and_then(|s| s.as_array()) {
        for seg in arr {
            if seg.get("type").and_then(|t| t.as_str()) == Some("text")
                && let Some(s) = seg.get("text").and_then(|t| t.as_str())
            {
                push_prose(&mut text, s);
            }
        }
    }
    text
}

/// v0.2.0 tool_uses: `[{id, name, input:{...}}]`.
fn collect_flat_tools(tool_uses: Option<&serde_json::Value>) -> Vec<NormTool> {
    let mut tools = Vec::new();
    if let Some(arr) = tool_uses.and_then(|t| t.as_array()) {
        for t in arr {
            let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let file_path = t.get("input").and_then(file_path_in_input);
            if !name.is_empty() {
                tools.push(NormTool { name, file_path });
            }
        }
    }
    tools
}

/// Mine a file path from a tool input object (the churn signal). Checks the
/// common key names Zed tools use.
fn file_path_in_input(input: &serde_json::Value) -> Option<String> {
    for k in ["file_path", "path", "abs_path", "target_file"] {
        if let Some(p) = input.get(k).and_then(|p| p.as_str())
            && !p.is_empty()
        {
            return Some(p.to_string());
        }
    }
    None
}

fn push_prose(buf: &mut String, s: &str) {
    let s = s.trim();
    if s.is_empty() {
        return;
    }
    if !buf.is_empty() {
        buf.push_str("\n\n");
    }
    buf.push_str(s);
}

/// Parse an ISO-8601/RFC-3339 timestamp to epoch milliseconds.
fn parse_iso_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    // v0.3.0 (tagged) thread: User(Text+Mention) → Agent(Text+ToolUse) → User → Agent.
    const V3: &str = r#"{
      "version": "0.3.0",
      "updated_at": "2026-05-15T10:30:00.000Z",
      "model": {"provider": "copilot_chat", "model": "GPT-5"},
      "initial_project_snapshot": {
        "timestamp": "2026-05-15T10:00:00.000Z",
        "worktree_snapshots": [{"worktree_path": "/Users/Jerry/Developer/rokkit"}]
      },
      "messages": [
        {"User": {"content": [{"Text": "fix the toc builder"}, {"Mention": {"uri": {"File": {"abs_path": "/x/y.md"}}}}]}},
        {"Agent": {"content": [{"Thinking": "hmm"}, {"Text": "On it."}, {"ToolUse": {"name": "edit_file", "raw_input": "{\"file_path\": \"/Users/Jerry/Developer/rokkit/src/toc.ts\"}"}}]}},
        {"User": {"content": [{"Text": "now add a test"}]}},
        {"Agent": {"content": [{"Text": "Added."}]}}
      ]
    }"#;

    // v0.2.0 (flat) thread.
    const V2: &str = r#"{
      "version": "0.2.0",
      "updated_at": "2026-03-01T12:05:00.000Z",
      "model": {"provider": "ollama", "model": "gemma4:latest"},
      "initial_project_snapshot": {
        "timestamp": "2026-03-01T12:00:00.000Z",
        "worktree_snapshots": [{"worktree_path": "/Users/Jerry/Developer/dbd"}]
      },
      "messages": [
        {"id": "1", "role": "user", "segments": [{"type": "text", "text": "check the front-matter toc"}], "tool_uses": [], "tool_results": []},
        {"id": "2", "role": "assistant", "segments": [{"type": "text", "text": "Checking."}], "tool_uses": [{"id": "t1", "name": "find_path", "input": {"path": "/Users/Jerry/Developer/dbd/src/toc.rs"}}], "tool_results": []}
      ]
    }"#;

    #[test]
    fn extract_model_reads_provider_and_model() {
        assert_eq!(extract_model(V3), Some(("copilot_chat".into(), "GPT-5".into())));
        assert_eq!(extract_model(V2), Some(("ollama".into(), "gemma4:latest".into())));
        assert_eq!(extract_model("{}"), None);
    }

    #[test]
    fn extract_tokens_sums_request_usage_and_prefers_cumulative() {
        // Per-request map (the shape seen in real v0.3.0 threads): sum across requests.
        let per_req = r#"{"request_token_usage":{
            "r1":{"input_tokens":12427,"output_tokens":406},
            "r2":{"input_tokens":13448,"output_tokens":1}
        }}"#;
        assert_eq!(extract_tokens(per_req), Some((25875, 407)), "input + output summed across requests");
        // Populated cumulative summary wins over the request map, and folds cache into input.
        let cumulative = r#"{"cumulative_token_usage":{"input_tokens":100,"cache_read_input_tokens":900,"output_tokens":50},
                             "request_token_usage":{"r1":{"input_tokens":1,"output_tokens":1}}}"#;
        assert_eq!(extract_tokens(cumulative), Some((1000, 50)), "cumulative preferred; cache_read folded into input");
        // No usage recorded ⇒ honest-None, never a fabricated (0,0).
        assert_eq!(extract_tokens(r#"{"messages":[]}"#), None);
        assert_eq!(extract_tokens("not json"), None);
    }

    #[test]
    fn parse_v3_tagged_pairs_turns_excluding_thinking_and_tooluse() {
        let turns = parse_zed_thread(V3);
        assert_eq!(turns.len(), 2, "two user prompts ⇒ two turns");
        assert_eq!(turns[0].turn_index, 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("fix the toc builder"));
        assert_eq!(turns[0].assistant_text, "On it.", "Thinking + ToolUse excluded from prose");
        assert_eq!(turns[1].user_text.as_deref(), Some("now add a test"));
        assert_eq!(turns[1].assistant_text, "Added.");
    }

    #[test]
    fn parse_v2_flat_pairs_turns() {
        let turns = parse_zed_thread(V2);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("check the front-matter toc"));
        assert_eq!(turns[0].assistant_text, "Checking.");
    }

    #[test]
    fn parse_session_v3_reconstructs_events_and_cwd() {
        let s = parse_zed_session(V3).unwrap();
        assert_eq!(s.cwds, vec!["/Users/Jerry/Developer/rokkit".to_string()]);
        let kinds: Vec<&str> = s.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(kinds, vec!["UserPromptSubmit", "PostToolUse", "UserPromptSubmit", "Stop"]);
        assert_eq!(s.events[0].prompt.as_deref(), Some("fix the toc builder"));
        let tool = s.events.iter().find(|e| e.event_type == "PostToolUse").unwrap();
        assert_eq!(tool.tool_name.as_deref(), Some("edit_file"));
        assert_eq!(tool.file_path.as_deref(), Some("/Users/Jerry/Developer/rokkit/src/toc.ts"), "file_path mined from raw_input");
    }

    #[test]
    fn parse_session_timestamps_span_snapshot_to_updated_at() {
        let s = parse_zed_session(V3).unwrap();
        let start = parse_iso_ms("2026-05-15T10:00:00.000Z").unwrap();
        let end = parse_iso_ms("2026-05-15T10:30:00.000Z").unwrap();
        assert_eq!(s.events.first().unwrap().ts, start, "first event at snapshot start");
        assert_eq!(s.events.last().unwrap().ts, end, "Stop at updated_at (end)");
        // monotonic non-decreasing
        assert!(s.events.windows(2).all(|w| w[0].ts <= w[1].ts));
    }

    #[test]
    fn parse_session_v2_uses_tool_input_path() {
        let s = parse_zed_session(V2).unwrap();
        assert_eq!(s.cwds, vec!["/Users/Jerry/Developer/dbd".to_string()]);
        let tool = s.events.iter().find(|e| e.event_type == "PostToolUse").unwrap();
        assert_eq!(tool.tool_name.as_deref(), Some("find_path"));
        assert_eq!(tool.file_path.as_deref(), Some("/Users/Jerry/Developer/dbd/src/toc.rs"));
    }

    #[test]
    fn parse_session_none_without_events() {
        assert!(parse_zed_session(r#"{"messages":[]}"#).is_none());
        assert!(parse_zed_session("not json").is_none());
    }

    #[test]
    fn malformed_content_yields_no_turns() {
        assert!(parse_zed_thread("not json").is_empty());
        assert!(parse_zed_thread(r#"{"messages":"oops"}"#).is_empty());
    }
}
