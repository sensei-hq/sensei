//! Claude Code transcript adapter (#73). Reads
//! `~/.claude/projects/<enc>/<session_id>.jsonl`; the filename stem is the
//! session id (== our `client_session_id`). Each line is a JSON record; a turn
//! spans one genuine human prompt to the next, and the assistant prose is the
//! `text` content blocks (tool_use / thinking excluded).

use super::{
    MAX_LINE_BYTES, MAX_TRANSCRIPT_BYTES, MAX_TURN_CHARS, ParsedTranscript, SessionTokens,
    SynthEvent, SynthSession, TranscriptAdapter, TranscriptTurn, TurnFacts, UnitRef,
    human_prompt_text, merge_facts, parse_timestamp, turn_attrs,
};
use std::path::{Path, PathBuf};

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
            tracing::warn!(
                file = key,
                size_mb = size / 1_048_576,
                "transcript ingest: skipping oversized transcript"
            );
            return None;
        }
        std::fs::read_to_string(path)
            .map_err(|e| tracing::debug!(error = %e, file = key, "claude: read transcript failed"))
            .ok()
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        let session = parse_claude_session(content);
        ParsedTranscript {
            turns: parse_claude_transcript(content),
            cwds: session.as_ref().map(|s| s.cwds.clone()).unwrap_or_default(),
            events: session.map(|s| s.events).unwrap_or_default(),
            model: claude_model(content).map(|m| ("anthropic".to_string(), m)),
            tokens: claude_tokens(content),
        }
    }
}

/// Session-total token usage across a Claude transcript's assistant records, kept
/// SPLIT (fresh input / cache write / cache read / output). `None` when no record
/// carries usage (honest-empty). Pure.
pub fn claude_tokens(content: &str) -> Option<SessionTokens> {
    let (mut fresh, mut cw, mut cr, mut tout, mut seen) = (0i64, 0i64, 0i64, 0i64, false);
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let Some(u) = v.get("message").and_then(|m| m.get("usage")) else {
            continue;
        };
        let g = |k: &str| u.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
        let line_in =
            g("input_tokens") + g("cache_creation_input_tokens") + g("cache_read_input_tokens");
        let line_out = g("output_tokens");
        if line_in > 0 || line_out > 0 {
            seen = true;
            fresh += g("input_tokens");
            cw += g("cache_creation_input_tokens");
            cr += g("cache_read_input_tokens");
            tout += line_out;
        }
    }
    seen.then_some(SessionTokens {
        input: fresh,
        output: tout,
        cache_read: Some(cr),
        cache_write: Some(cw),
        reasoning: None, // Claude does not separate reasoning tokens
        cost: None,      // no metered cost in the transcript
    })
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
            && let Some(model) =
                v.get("message").and_then(|m| m.get("model")).and_then(|m| m.as_str())
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
        let Some(ts) = parse_timestamp(&v).map(|d| d.timestamp_millis()) else {
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
                            let tool_name =
                                b.get("name").and_then(|n| n.as_str()).map(str::to_string);
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

/// The per-turn attributes we PROMOTE to columns, lifted from one transcript
/// record. Everything not named here still survives in `attrs` — see
/// [`turn_attrs`] — so adding a signal later is a query, not a re-ingest.
///
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
                    let mut facts = TurnFacts::default();
                    merge_facts(&mut facts, &v);
                    cur = Some(TranscriptTurn {
                        turn_index: idx,
                        user_text: Some(prompt),
                        assistant_text: String::new(),
                        started_at: parse_timestamp(&v),
                        attrs: turn_attrs(&v, &["message", "attachment", "toolUseResult"]),
                        facts,
                    });
                }
                // tool_result / meta / injected → not a boundary, ignore.
            }
            "assistant" => {
                if let Some(t) = cur.as_mut() {
                    // A turn spans several assistant records — tokens sum, and the
                    // LAST stop_reason is the one that ended it.
                    merge_facts(&mut t.facts, &v);
                    if let Some(sr) = v
                        .get("message")
                        .and_then(|m| m.get("stop_reason"))
                        .and_then(|x| x.as_str())
                        .filter(|x| !x.is_empty())
                    {
                        t.facts.stop_reason = Some(sr.to_string());
                    }
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

#[cfg(test)]
mod tests {

    #[test]
    fn turn_keeps_split_tokens_and_promoted_attributes() {
        // The whole point of the split: `tokens_in` here is FRESH input only.
        // Summing it with cache_read (as the session-grain total does) is what makes
        // cost read ~10x high — measured against real transcripts, ~98% of that sum
        // is cache reads, which bill far cheaper.
        let c = concat!(
            r#"{"type":"user","cwd":"/r","gitBranch":"develop","effort":"xhigh","isSidechain":false,"#,
            r#""attributionSkill":"superpowers:brainstorming","attributionPlugin":"superpowers","#,
            r#""timestamp":"2026-06-20T10:00:00.000Z","message":{"role":"user","content":"go"}}"#,
            "\n",
            r#"{"type":"assistant","cwd":"/r","message":{"role":"assistant","model":"claude-opus-4-8","stop_reason":"tool_use","#,
            r#""usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":900,"output_tokens":20,"service_tier":"standard"}},"#,
            r#""content":[{"type":"text","text":"ok"}]}"#,
            "\n",
            r#"{"type":"assistant","cwd":"/r","message":{"role":"assistant","model":"claude-opus-4-8","stop_reason":"end_turn","#,
            r#""usage":{"input_tokens":5,"cache_read_input_tokens":50,"output_tokens":8}},"content":[{"type":"text","text":"done"}]}"#,
            "\n",
        );
        let turns = parse_claude_transcript(c);
        assert_eq!(turns.len(), 1, "one user prompt = one turn");
        let f = &turns[0].facts;
        assert_eq!(
            f.tokens_in,
            Some(15),
            "fresh input only, summed over the turn's records (10+5)"
        );
        assert_eq!(f.cache_read, Some(950), "cache reads kept SEPARATE (900+50)");
        assert_eq!(f.cache_write, Some(100));
        assert_eq!(f.tokens_out, Some(28));
        // The LAST record's stop_reason is the one that ended the turn.
        assert_eq!(f.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(f.skill.as_deref(), Some("superpowers:brainstorming"));
        assert_eq!(f.plugin.as_deref(), Some("superpowers"));
        assert_eq!(f.git_branch.as_deref(), Some("develop"));
        assert_eq!(f.effort.as_deref(), Some("xhigh"));
        assert_eq!(f.service_tier.as_deref(), Some("standard"));
        assert_eq!(f.is_sidechain, Some(false));
        // Unpromoted attributes survive verbatim rather than being dropped…
        assert_eq!(turns[0].attrs["cwd"], "/r");
        // …but the bulky prose does not (it is already in assistant_text).
        assert!(
            turns[0].attrs.get("message").and_then(|m| m.get("content")).is_none(),
            "message.content is not duplicated into attrs"
        );
    }

    #[test]
    fn turn_facts_are_null_not_zero_when_the_transcript_lacks_them() {
        // Honest-empty: a transcript with no usage/attribution must not record
        // fabricated zeros that a consumer can't tell from a real reading.
        let c = concat!(
            r#"{"type":"user","message":{"role":"user","content":"hi"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"yo"}]}}"#,
            "\n",
        );
        let turns = parse_claude_transcript(c);
        assert_eq!(turns.len(), 1);
        let f = &turns[0].facts;
        assert_eq!(f.tokens_in, None, "absent usage is null, never 0");
        assert_eq!(f.cache_read, None);
        assert_eq!(f.skill, None);
        assert_eq!(f.is_sidechain, None);
    }

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
        assert_eq!(
            turns.len(),
            2,
            "two human prompts ⇒ two turns (tool_result/meta/notification ignored)"
        );
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
        assert_eq!(turns[0].started_at.unwrap().to_rfc3339(), "2026-06-22T10:00:00+00:00");
    }

    #[test]
    fn human_prompt_text_rejects_tool_results_meta_and_noise() {
        let tool_result =
            serde_json::json!({"message":{"content":[{"type":"tool_result","content":"x"}]}});
        assert!(super::super::human_prompt_text(&tool_result).is_none());
        let meta =
            serde_json::json!({"isMeta":true,"message":{"content":[{"type":"text","text":"hi"}]}});
        assert!(super::super::human_prompt_text(&meta).is_none());
        let noise =
            serde_json::json!({"message":{"content":"<system-reminder>stuff</system-reminder>"}});
        assert!(super::super::human_prompt_text(&noise).is_none());
        let real = serde_json::json!({"message":{"content":"fix the bug"}});
        assert_eq!(super::super::human_prompt_text(&real).as_deref(), Some("fix the bug"));
    }

    #[test]
    fn ignores_malformed_lines_and_empty() {
        let turns = parse_claude_transcript(
            "not json\n\n{\"type\":\"user\",\"message\":{\"content\":\"hello\"}}\n",
        );
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("hello"));
    }

    const SESS: &str = r#"
{"type":"user","cwd":"/repo","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}
{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/x.rs"}}]}}
{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}
"#;

    #[test]
    fn parse_produces_the_common_structure() {
        // The adapter's single `parse` maps raw content → the shared ParsedTranscript
        // (turns + cwds + events + model + tokens) that persistence consumes — verified
        // here without a DB, so a format change is caught at the seam.
        let content = concat!(
            r#"{"type":"user","cwd":"/repo","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}"#,
            "\n",
            r#"{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":10,"cache_read_input_tokens":90,"output_tokens":20},"content":[{"type":"text","text":"On it."},{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/x.rs"}}]}}"#,
            "\n",
        );
        let p = ClaudeAdapter::new(std::path::PathBuf::from("/tmp")).parse(content);
        assert_eq!(p.turns.len(), 1, "one user-bounded turn");
        assert_eq!(p.turns[0].user_text.as_deref(), Some("fix the parser"));
        assert_eq!(p.cwds, vec!["/repo".to_string()], "cwd collected for project resolution");
        let kinds: Vec<&str> = p.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(kinds, vec!["UserPromptSubmit", "PostToolUse", "Stop"]);
        assert_eq!(p.model, Some(("anthropic".to_string(), "claude-opus-4-8".to_string())));
        let t = p.tokens.expect("usage parsed");
        assert_eq!(t.input, 10, "FRESH input only — cache is no longer folded in");
        assert_eq!(t.cache_read, Some(90));
        assert_eq!(t.output, 20);
        assert_eq!(t.total_input(), 100, "the old tokens_in meaning is preserved");
    }

    #[test]
    fn parse_session_reconstructs_events() {
        let s = parse_claude_session(SESS).unwrap();
        assert_eq!(
            s.cwds,
            vec!["/repo".to_string()],
            "collects distinct cwd for project resolution"
        );
        let kinds: Vec<&str> = s.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(
            kinds,
            vec!["UserPromptSubmit", "PostToolUse", "PostToolUse", "Stop"],
            "prompts + tool_uses + synthetic Stop"
        );
        assert_eq!(s.events[0].prompt.as_deref(), Some("fix the parser"));
        let edit = s.events.iter().find(|e| e.tool_name.as_deref() == Some("Edit")).unwrap();
        assert_eq!(
            edit.file_path.as_deref(),
            Some("/repo/src/x.rs"),
            "Edit carries file_path (the churn signal)"
        );
        let stop = s.events.last().unwrap();
        assert_eq!(
            stop.ts,
            s.events.iter().map(|e| e.ts).max().unwrap(),
            "Stop at the last timestamp"
        );
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
        assert_eq!(
            claude_model("{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}").as_deref(),
            None,
            "no model ⇒ None"
        );
    }

    #[test]
    fn claude_tokens_sums_usage_including_cache() {
        let t = r#"
{"type":"user","message":{"content":"hi"}}
{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":50,"output_tokens":20}}}
{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"input_tokens":5,"output_tokens":8}}}
"#;
        // Split across the turn's records: fresh 10+5, write 100+0, read 50+0.
        let got = claude_tokens(t).expect("usage parsed");
        assert_eq!(got.input, 15, "fresh input only");
        assert_eq!(got.cache_write, Some(100));
        assert_eq!(got.cache_read, Some(50));
        assert_eq!(got.output, 28);
        // total_input() still reports what tokens_in always meant.
        assert_eq!(got.total_input(), 165);
        // no usage anywhere ⇒ honest-None, never a fabricated (0,0)
        assert_eq!(claude_tokens("{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}"), None);
    }
}
