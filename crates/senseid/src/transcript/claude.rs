//! Claude Code transcript adapter (#73). Reads
//! `~/.claude/projects/<enc>/<session_id>.jsonl`; the filename stem is the
//! session id (== our `client_session_id`). Each line is a JSON record; a turn
//! spans one genuine human prompt to the next, and the assistant prose is the
//! `text` content blocks (tool_use / thinking excluded).

use super::{TranscriptAdapter, TranscriptTurn};
use std::path::{Path, PathBuf};

/// Cap stored assistant prose per turn (safety net for pathological turns).
const MAX_TURN_CHARS: usize = 50_000;

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

    fn transcript_files(&self) -> Vec<PathBuf> {
        // layout: <root>/<project-dir>/<session_id>.jsonl
        let mut files = Vec::new();
        let Ok(projects) = std::fs::read_dir(&self.root) else {
            return files;
        };
        for proj in projects.flatten() {
            let Ok(entries) = std::fs::read_dir(proj.path()) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
                    files.push(p);
                }
            }
        }
        files
    }

    fn session_id_for(&self, path: &Path) -> Option<String> {
        path.file_stem().and_then(|s| s.to_str()).map(str::to_string)
    }

    fn parse(&self, content: &str) -> Vec<TranscriptTurn> {
        parse_claude_transcript(content)
    }
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
}
