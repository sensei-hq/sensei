//! Claude Code transcript adapter (#73). Reads
//! `~/.claude/projects/<enc>/<session_id>.jsonl`; the filename stem is the
//! session id (== our `client_session_id`). Each line is a JSON record; a turn
//! spans one genuine human prompt to the next, and the assistant prose is the
//! `text` content blocks (tool_use / thinking excluded).

use super::{SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, UnitRef};
use std::path::{Path, PathBuf};

/// Cap stored assistant prose per turn (safety net for pathological turns).
const MAX_TURN_CHARS: usize = 50_000;

/// Skip transcript files larger than this (logged). A multi-hundred-MB file
/// would spike memory on read and block the executor on parse; rare outlier.
const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;

/// File mtime as epoch nanoseconds (the cursor change-stamp). `None` if the file
/// is gone / unreadable.
fn mtime_ns(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos() as i64)
}

/// Skip any single transcript line larger than this — a line this big is a
/// base64 attachment / blob, not prose, and parsing it stalls the executor.
const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

/// Leading markers that mark an injected (non-human) "user" message — harness
/// notifications, hook context, slash-command echoes. These are not turn
/// boundaries.
const INJECTED_MARKERS: &[&str] = &[
    "<task-notification",
    "<system-reminder",
    "<command-name",
    "<command-message",
    "<local-command",
    "Caveat:",
    "## Security Guidance",
];

pub struct ClaudeAdapter {
    root: PathBuf,
}

impl ClaudeAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl TranscriptAdapter for ClaudeAdapter {
    fn source(&self) -> &'static str {
        "claude_code"
    }
    fn family(&self) -> &'static str {
        "claude"
    }

    fn units(&self) -> Vec<UnitRef> {
        // layout: <root>/<project-dir>/<session_id>.jsonl — the main session
        // transcripts (direct children). INTENTIONALLY does not recurse into
        // <session_id>/subagents/agent-*.jsonl (subagent sidechains belong to a
        // parent session; ingesting/attributing them is a #73 follow-up).
        let mut units = Vec::new();
        let Ok(projects) = std::fs::read_dir(&self.root) else {
            return units;
        };
        for proj in projects.flatten() {
            let Ok(entries) = std::fs::read_dir(proj.path()) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("jsonl")
                    && let Some(stamp) = mtime_ns(&p)
                {
                    units.push(UnitRef { key: p.to_string_lossy().into_owned(), stamp });
                }
            }
        }
        units
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        mtime_ns(Path::new(key))
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        Path::new(key).file_stem().and_then(|s| s.to_str()).map(str::to_string)
    }

    fn load_content(&self, key: &str) -> Option<String> {
        let path = Path::new(key);
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_TRANSCRIPT_BYTES {
            tracing::warn!(file = key, size_mb = size / 1_048_576, "transcript ingest: skipping oversized transcript");
            return None;
        }
        std::fs::read_to_string(path)
            .map_err(|e| tracing::debug!(error = %e, file = key, "claude: read transcript failed"))
            .ok()
    }

    fn parse(&self, content: &str) -> Vec<TranscriptTurn> {
        parse_claude_transcript(content)
    }

    fn parse_session(&self, content: &str) -> Option<SynthSession> {
        parse_claude_session(content)
    }

    fn model_for(&self, content: &str) -> Option<(String, String)> {
        claude_model(content).map(|m| ("anthropic".to_string(), m))
    }
}

/// The model that ran a Claude transcript: the most frequent `message.model`
/// across assistant records (a session can switch models mid-run; the dominant
/// one wins). `None` if no assistant record carries a model. Pure.
pub fn claude_model(content: &str) -> Option<String> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, u32> = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) == Some("assistant")
            && let Some(model) = v.get("message").and_then(|m| m.get("model")).and_then(|m| m.as_str())
            && !model.is_empty()
            && model != "<synthetic>"
        {
            *counts.entry(model.to_string()).or_default() += 1;
        }
    }
    // Most frequent; ties broken by model name for determinism.
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(model, _)| model)
}

/// Reconstruct a session's event stream from a Claude transcript (#75): a
/// `UserPromptSubmit` per genuine human prompt, a `PostToolUse` per `tool_use`
/// block (name + file_path), and a synthetic terminal `Stop` (transcripts carry
/// no end marker). Also collects distinct `cwd`s for project resolution. Pure.
pub fn parse_claude_session(content: &str) -> Option<SynthSession> {
    let mut cwds: Vec<String> = Vec::new();
    let mut events: Vec<SynthEvent> = Vec::new();
    let mut max_ts: i64 = 0;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str())
            && !cwd.is_empty()
            && !cwds.iter().any(|c| c == cwd)
        {
            cwds.push(cwd.to_string());
        }
        let Some(ts) = parse_ts(&v).map(|d| d.timestamp_millis()) else {
            continue;
        };
        match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "user" => {
                if let Some(prompt) = human_prompt_text(&v) {
                    events.push(SynthEvent {
                        event_type: "UserPromptSubmit".into(),
                        tool_name: None,
                        file_path: None,
                        prompt: Some(prompt),
                        tool_input: None,
                        ts,
                    });
                    max_ts = max_ts.max(ts);
                }
            }
            "assistant" => {
                if let Some(serde_json::Value::Array(blocks)) =
                    v.get("message").and_then(|m| m.get("content"))
                {
                    for b in blocks {
                        if b.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            let tool_name = b.get("name").and_then(|n| n.as_str()).map(str::to_string);
                            // The full tool_use input — the enrich worker derives
                            // call_info/plugin/method from it (bash command, skill/agent
                            // params, …), same as a live-captured event.
                            let tool_input = b.get("input").cloned();
                            let file_path = tool_input
                                .as_ref()
                                .and_then(|i| i.get("file_path").or_else(|| i.get("path")))
                                .and_then(|p| p.as_str())
                                .map(str::to_string);
                            events.push(SynthEvent {
                                event_type: "PostToolUse".into(),
                                tool_name,
                                file_path,
                                prompt: None,
                                tool_input,
                                ts,
                            });
                            max_ts = max_ts.max(ts);
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
    // Transcripts have no explicit end marker — synthesize a terminal Stop so
    // the enricher derives outcome=completed.
    events.push(SynthEvent {
        event_type: "Stop".into(),
        tool_name: None,
        file_path: None,
        prompt: None,
        tool_input: None,
        ts: max_ts,
    });
    Some(SynthSession { cwds, events })
}

/// Parse a Claude transcript (JSONL) into user-prompt-bounded turns. A new turn
/// starts at each genuine human prompt; assistant `text` blocks until the next
/// prompt form that turn's response. Pure + deterministic.
pub fn parse_claude_transcript(content: &str) -> Vec<TranscriptTurn> {
    let mut turns: Vec<TranscriptTurn> = Vec::new();
    let mut cur: Option<TranscriptTurn> = None;
    let mut idx = 0i32;

    for line in content.lines() {
        let line = line.trim();
        // skip blank lines and oversized (blob/attachment) lines — parsing a
        // multi-MB JSON line would stall the executor.
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "user" => {
                if let Some(prompt) = human_prompt_text(&v) {
                    if let Some(t) = cur.take() {
                        turns.push(t);
                    }
                    idx += 1;
                    cur = Some(TranscriptTurn {
                        turn_index: idx,
                        user_text: Some(prompt),
                        assistant_text: String::new(),
                        started_at: parse_ts(&v),
                    });
                }
                // tool_result / meta / injected → not a boundary, ignore.
            }
            "assistant" => {
                if let Some(t) = cur.as_mut() {
                    let text = assistant_text_blocks(&v);
                    if !text.is_empty() {
                        if !t.assistant_text.is_empty() {
                            t.assistant_text.push_str("\n\n");
                        }
                        t.assistant_text.push_str(&text);
                    }
                }
                // assistant prose before any human prompt → ignore.
            }
            _ => {}
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

/// The human prompt text of a `user` record, or `None` if it's a tool result,
/// a meta/injected message, or empty.
fn human_prompt_text(v: &serde_json::Value) -> Option<String> {
    if v.get("isMeta").and_then(|m| m.as_bool()) == Some(true) {
        return None;
    }
    let content = v.get("message").and_then(|m| m.get("content"))?;
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => {
            // a tool_result message is not a human prompt
            if blocks
                .iter()
                .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result"))
            {
                return None;
            }
            let mut s = String::new();
            for b in blocks {
                if b.get("type").and_then(|t| t.as_str()) == Some("text")
                    && let Some(t) = b.get("text").and_then(|t| t.as_str())
                {
                    if !s.is_empty() {
                        s.push('\n');
                    }
                    s.push_str(t);
                }
            }
            s
        }
        _ => return None,
    };
    let trimmed = text.trim();
    if trimmed.is_empty() || is_injected_noise(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Concatenated `text` blocks of an `assistant` record (prose only — no
/// thinking, no tool_use).
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

fn is_injected_noise(text: &str) -> bool {
    INJECTED_MARKERS.iter().any(|m| text.starts_with(m))
}

fn parse_ts(v: &serde_json::Value) -> Option<chrono::DateTime<chrono::Utc>> {
    let ts = v.get("timestamp").and_then(|t| t.as_str())?;
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A compact transcript: prompt → (thinking+text+tool_use) → tool_result →
    // text, then a second prompt. Plus injected noise that must NOT split turns.
    const SAMPLE: &str = r#"
{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"add a login page"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:01.000Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"hmm"}]}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"On it — adding the page."}]}}
{"type":"assistant","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{}}]}}
{"type":"user","timestamp":"2026-06-22T10:00:04.000Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}
{"type":"user","timestamp":"2026-06-22T10:00:05.000Z","isMeta":true,"message":{"role":"user","content":[{"type":"text","text":"Base directory for this skill: ..."}]}}
{"type":"user","timestamp":"2026-06-22T10:00:06.000Z","message":{"role":"user","content":"<task-notification>ping</task-notification>"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:07.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Done."}]}}
{"type":"user","timestamp":"2026-06-22T10:01:00.000Z","message":{"role":"user","content":[{"type":"text","text":"now wire it up"}]}}
{"type":"assistant","timestamp":"2026-06-22T10:01:01.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Wired."}]}}
"#;

    #[test]
    fn splits_on_genuine_prompts_only() {
        let turns = parse_claude_transcript(SAMPLE);
        assert_eq!(turns.len(), 2, "two human prompts ⇒ two turns (tool_result/meta/notification ignored)");
        assert_eq!(turns[0].turn_index, 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("add a login page"));
        // assistant text accumulates across messages within the turn; tool_use
        // and thinking are excluded.
        assert_eq!(turns[0].assistant_text, "On it — adding the page.\n\nDone.");
        assert_eq!(turns[1].turn_index, 2);
        assert_eq!(turns[1].user_text.as_deref(), Some("now wire it up"));
        assert_eq!(turns[1].assistant_text, "Wired.");
    }

    #[test]
    fn parses_started_at_from_prompt_timestamp() {
        let turns = parse_claude_transcript(SAMPLE);
        assert_eq!(
            turns[0].started_at.unwrap().to_rfc3339(),
            "2026-06-22T10:00:00+00:00"
        );
    }

    #[test]
    fn human_prompt_text_rejects_tool_results_meta_and_noise() {
        let tool_result = serde_json::json!({"message":{"content":[{"type":"tool_result","content":"x"}]}});
        assert!(human_prompt_text(&tool_result).is_none());
        let meta = serde_json::json!({"isMeta":true,"message":{"content":[{"type":"text","text":"hi"}]}});
        assert!(human_prompt_text(&meta).is_none());
        let noise = serde_json::json!({"message":{"content":"<system-reminder>stuff</system-reminder>"}});
        assert!(human_prompt_text(&noise).is_none());
        let real = serde_json::json!({"message":{"content":"fix the bug"}});
        assert_eq!(human_prompt_text(&real).as_deref(), Some("fix the bug"));
    }

    #[test]
    fn ignores_malformed_lines_and_empty() {
        let turns = parse_claude_transcript("not json\n\n{\"type\":\"user\",\"message\":{\"content\":\"hello\"}}\n");
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("hello"));
    }

    const SESS: &str = r#"
{"type":"user","cwd":"/repo","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}
{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/x.rs"}}]}}
{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}
"#;

    #[test]
    fn parse_session_reconstructs_events() {
        let s = parse_claude_session(SESS).unwrap();
        assert_eq!(s.cwds, vec!["/repo".to_string()], "collects distinct cwd for project resolution");
        let kinds: Vec<&str> = s.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(kinds, vec!["UserPromptSubmit", "PostToolUse", "PostToolUse", "Stop"], "prompts + tool_uses + synthetic Stop");
        assert_eq!(s.events[0].prompt.as_deref(), Some("fix the parser"));
        let edit = s.events.iter().find(|e| e.tool_name.as_deref() == Some("Edit")).unwrap();
        assert_eq!(edit.file_path.as_deref(), Some("/repo/src/x.rs"), "Edit carries file_path (the churn signal)");
        let stop = s.events.last().unwrap();
        assert_eq!(stop.ts, s.events.iter().map(|e| e.ts).max().unwrap(), "Stop at the last timestamp");
    }

    #[test]
    fn parse_session_none_when_no_events() {
        assert!(parse_claude_session("").is_none());
        assert!(parse_claude_session("{\"type\":\"summary\"}\n").is_none());
    }

    #[test]
    fn claude_model_picks_dominant_model() {
        let t = r#"
{"type":"user","message":{"content":"hi"}}
{"type":"assistant","message":{"model":"claude-opus-4-8","content":[{"type":"text","text":"a"}]}}
{"type":"assistant","message":{"model":"claude-opus-4-8","content":[{"type":"text","text":"b"}]}}
{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"c"}]}}
"#;
        assert_eq!(claude_model(t).as_deref(), Some("claude-opus-4-8"), "most frequent model wins");
        assert_eq!(claude_model("{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}").as_deref(), None, "no model ⇒ None");
    }
}
