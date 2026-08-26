//! One LLM-derived facet record per session.
//!
//! The mechanical half of a retrospective — how many tool calls, which
//! languages, how often you commit — comes out of parsing. It cannot answer
//! *what were you trying to do*, *what got in the way*, or *did it work*, and
//! those are the questions a retrospective is for.
//!
//! So each session gets one call to a model, producing a fixed-shape record.
//! The report's qualitative sections are then group-bys over those records
//! rather than free generation, which is what keeps them checkable.
//!
//! # Where the text goes
//!
//! These are other people's transcripts. The parsers deliberately never retain
//! prompt text, so this module re-reads ONE session at a time and drops the text
//! as soon as the record is produced. The model is a LOCAL ollama instance by
//! default — nothing leaves the machine unless an endpoint is passed explicitly.

use crate::model::Session;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How much of a session to show the model.
///
/// Prompts carry the intent, so they are worth more per byte than assistant
/// output. A long session is sampled from both ends rather than truncated: the
/// goal is usually stated at the start and the outcome at the end, and cutting
/// the tail loses whether it worked.
const MAX_PROMPTS: usize = 40;
const MAX_PROMPT_CHARS: usize = 600;

/// The facet vocabulary. Closed sets, so the report can group by them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facet {
    /// Filled in by [`derive`] after parsing — the model is never asked for it,
    /// so it must not be required of the model's reply.
    #[serde(default)]
    pub session_id: String,
    /// One sentence: what the person was actually trying to achieve.
    #[serde(default)]
    pub underlying_goal: String,
    /// Which kinds of work this session involved. Multi-valued: real sessions
    /// mix feature work with testing and schema changes.
    #[serde(default)]
    pub goal_categories: Vec<String>,
    /// How it ended. Mirrors sensei's `session_outcome` enum.
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub friction: Vec<String>,
    #[serde(default)]
    pub friction_detail: String,
    #[serde(default)]
    pub primary_success: String,
    #[serde(default)]
    pub brief_summary: String,
    /// A verbatim line from the transcript backing the goal. Without it the
    /// record is dropped: an observation nobody can check is not worth showing.
    #[serde(default)]
    pub evidence: String,
}

pub const GOAL_CATEGORIES: &[&str] = &[
    "feature_implementation",
    "bug_fixing",
    "refactoring",
    "testing",
    "database_schema_work",
    "code_review",
    "documentation",
    "research_and_exploration",
    "build_and_tooling",
    "deployment_and_ops",
    "ui_work",
];

pub const FRICTION_KINDS: &[&str] = &[
    "repeated_tool_failures",
    "wrong_direction_taken",
    "misunderstood_requirement",
    "lost_context",
    "environment_or_setup",
    "slow_feedback_loop",
    "rework_after_correction",
    "none",
];

pub const OUTCOMES: &[&str] =
    &["completed", "mostly_achieved", "partial", "blocked", "abandoned", "unclear"];

/// The human prompts of one session, bounded.
///
/// Each ACP stores them differently, so this switches on the format rather than
/// asking the caller to know.
pub fn session_text(file: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(file).ok()?;
    let mut prompts: Vec<String> = Vec::new();

    // A VS Code delta journal is not a stream of readable records: a prompt is
    // assembled from `kind:1`/`2` operations at a path. It has to be replayed
    // before any text can be read out, so reuse the parser's replay rather than
    // scanning the raw operations and finding nothing.
    if is_journal(&raw) {
        let root = crate::vscode::replay(&raw);
        if let Some(requests) = root["requests"].as_array() {
            for r in requests {
                if let Some(t) = r["message"]["text"].as_str() {
                    prompts.push(t.to_string());
                }
            }
        }
    } else if raw.starts_with('{') || raw.starts_with('[') {
        for line in raw.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            // Claude Code
            if v["type"] == "user"
                && v["isMeta"].as_bool() != Some(true)
                && let Some(t) = user_text(&v["message"]["content"])
            {
                prompts.push(t);
            }
            // Copilot CLI / VS Code event stream
            if v["type"] == "user.message" {
                for k in ["content", "text", "message"] {
                    if let Some(t) = v["data"][k].as_str() {
                        prompts.push(t.to_string());
                        break;
                    }
                }
            }

        }
    }

    prompts.retain(|p| !p.trim().is_empty() && !is_injected(p));
    if prompts.is_empty() {
        return None;
    }
    Some(sample(&prompts))
}

fn user_text(content: &serde_json::Value) -> Option<String> {
    match content {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(a) => {
            // A `user` record is also how tool RESULTS arrive; those are not
            // the person speaking.
            if a.iter().any(|b| b["type"] == "tool_result") {
                return None;
            }
            let t = a.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n");
            (!t.is_empty()).then_some(t)
        }
        _ => None,
    }
}

/// A delta journal, as opposed to a stream of self-contained records.
///
/// Both are JSONL starting with `{`; the journal is distinguished by its
/// operations carrying a value under `v`.
fn is_journal(raw: &str) -> bool {
    raw.lines()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .is_some_and(|v| v.get("v").is_some() && v.get("kind").is_some())
}

/// Text the ACP injects on the human's behalf — not someone typing.
fn is_injected(t: &str) -> bool {
    let t = t.trim_start();
    [
        "<task-notification",
        "<system-reminder",
        "<command-name",
        "<command-message",
        "<local-command",
        "Caveat:",
        "## Security Guidance",
    ]
    .iter()
    .any(|m| t.starts_with(m))
}

/// Take from both ends: the goal is stated at the start, the outcome at the end.
fn sample(prompts: &[String]) -> String {
    let clip = |s: &String| -> String {
        let t = s.trim();
        match t.char_indices().nth(MAX_PROMPT_CHARS) {
            Some((i, _)) => format!("{}…", &t[..i]),
            None => t.to_string(),
        }
    };
    if prompts.len() <= MAX_PROMPTS {
        return prompts.iter().map(clip).collect::<Vec<_>>().join("\n---\n");
    }
    let half = MAX_PROMPTS / 2;
    let head = prompts[..half].iter().map(clip).collect::<Vec<_>>().join("\n---\n");
    let tail = prompts[prompts.len() - half..].iter().map(clip).collect::<Vec<_>>().join("\n---\n");
    format!("{head}\n---\n[… {} prompts omitted …]\n---\n{tail}", prompts.len() - MAX_PROMPTS)
}

fn prompt_for(session: &Session, text: &str) -> String {
    format!(
        "You are analysing one AI coding session to produce a retrospective record.\n\
         Below are the HUMAN's prompts, in order, separated by ---.\n\n\
         Mechanical facts already measured (do not contradict them):\n\
         - {} prompts, {} tool calls, {} of them reported as failed\n\
         - languages touched: {}\n\
         - {} commits, {} pushes\n\n\
         TRANSCRIPT:\n{}\n\n\
         Reply with ONLY a JSON object, no prose, with these keys:\n\
         - underlying_goal: one sentence, what the person was trying to achieve\n\
         - goal_categories: array from {:?}\n\
         - outcome: one of {:?}\n\
         - friction: array from {:?} (use [\"none\"] if it went smoothly)\n\
         - friction_detail: one sentence, or \"\" if no friction\n\
         - primary_success: short phrase for what went best\n\
         - brief_summary: two sentences\n\
         - evidence: ONE verbatim sentence copied exactly from the transcript above \
           that supports underlying_goal\n",
        session.prompts,
        session.tools.len(),
        session.tools.iter().filter(|t| t.success == Some(false)).count(),
        {
            let mut l: Vec<&String> = session.languages.keys().collect();
            l.sort();
            if l.is_empty() {
                "none recorded".to_string()
            } else {
                l.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            }
        },
        session.git_commits,
        session.git_pushes,
        text,
        GOAL_CATEGORIES,
        OUTCOMES,
        FRICTION_KINDS,
    )
}

/// Ask the model, and keep the record only if it is well-formed AND grounded.
///
/// A facet whose evidence does not appear in the transcript is DROPPED rather
/// than stored — the same rule sensei's process analyzer applies. An unverifiable
/// observation in a report about someone's work is worse than a missing one.
pub fn derive(session: &Session, endpoint: &str, model: &str) -> Result<Facet, String> {
    let file = session.file.as_ref().ok_or("session has no source file")?;
    let text = session_text(file).ok_or("no human prompts in transcript")?;

    let body = serde_json::json!({
        "model": model,
        "prompt": prompt_for(session, &text),
        "stream": false,
        "format": "json",
        // num_predict is explicit: the default cuts the reply mid-object on a
        // long transcript, which reads as "the model cannot follow the schema"
        // when it is really a truncated string.
        "options": {"temperature": 0.0, "num_ctx": 16384, "num_predict": 1500},
    });

    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "300",
            "-X",
            "POST",
            endpoint,
            "-H",
            "Content-Type: application/json",
            "-d",
            "@-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut().unwrap().write_all(body.to_string().as_bytes())?;
            c.wait_with_output()
        })
        .map_err(|e| format!("calling {endpoint}: {e}"))?;

    let reply: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("bad response: {e}"))?;
    let payload = reply["response"].as_str().ok_or_else(|| {
        format!(
            "no response field: {}",
            String::from_utf8_lossy(&out.stdout).chars().take(200).collect::<String>()
        )
    })?;

    let object = json_object(payload)
        .ok_or_else(|| format!("no JSON object in reply: {}", snippet(payload)))?;
    let mut facet: Facet = serde_json::from_str(object)
        .map_err(|e| format!("model returned non-conforming JSON ({e}): {}", snippet(object)))?;
    facet.session_id = session.id.clone();

    // Grounding check — the quote must really be in the transcript.
    let quote = facet.evidence.trim();
    if quote.len() < 12 || !collapse(&text).contains(&collapse(quote)) {
        return Err("evidence not found verbatim in transcript".into());
    }
    // Vocabulary check — a value outside the closed set would break the group-by.
    facet.goal_categories.retain(|g| GOAL_CATEGORIES.contains(&g.as_str()));
    facet.friction.retain(|f| FRICTION_KINDS.contains(&f.as_str()));
    if !OUTCOMES.contains(&facet.outcome.as_str()) {
        facet.outcome = "unclear".into();
    }
    Ok(facet)
}

/// The outermost `{...}` in a reply.
///
/// A thinking model narrates before it answers, and some wrap the object in a
/// fenced block. Feeding the whole reply to the parser reports a schema failure
/// for what is really a preamble.
fn json_object(reply: &str) -> Option<&str> {
    let start = reply.find('{')?;
    let bytes = reply.as_bytes();
    let (mut depth, mut in_str, mut escaped) = (0i32, false, false);
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            match c {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&reply[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Collapse whitespace runs so a quote copied correctly but re-wrapped still
/// matches. This forgives formatting, not content: every other character must
/// still line up.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn snippet(s: &str) -> String {
    s.chars().take(180).collect::<String>().replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thinking model narrates before it answers; the object still has to be
    /// found, or a preamble is misreported as a schema failure.
    #[test]
    fn the_object_is_found_past_a_preamble() {
        let reply = "Let me think about this.\n```json\n{\"outcome\": \"completed\"}\n```";
        assert_eq!(json_object(reply), Some("{\"outcome\": \"completed\"}"));
    }

    /// Braces inside a string must not close the object early.
    #[test]
    fn braces_inside_strings_do_not_end_the_object() {
        let reply = r#"{"detail": "used a map {k: v} here", "outcome": "partial"}"#;
        assert_eq!(json_object(reply), Some(reply));
        // An ESCAPED quote inside the string: the scanner must not treat it as
        // the closing quote, or the `}` after it ends the object early.
        let escaped = r#"{"detail": "a quote \" then }", "x": 1}"#;
        assert_eq!(json_object(escaped), Some(escaped));
    }

    /// A reply cut off mid-object yields nothing rather than a wrong parse.
    #[test]
    fn a_truncated_reply_is_not_an_object() {
        assert_eq!(json_object("{\"underlying_goal\": \"do the thi"), None);
    }

    /// Re-wrapping a correctly copied quote must not fail grounding, but
    /// changing a word must.
    #[test]
    fn grounding_forgives_reflow_but_not_rewording() {
        let text = "please fix\n  the failing   auth test";
        assert!(collapse(text).contains(&collapse("fix the failing auth test")));
        assert!(!collapse(text).contains(&collapse("fix the broken auth test")));
    }

    #[test]
    fn a_delta_journal_is_recognised() {
        assert!(is_journal(r#"{"kind":0,"v":{"requests":[]}}"#));
        assert!(!is_journal(r#"{"type":"user.message","data":{"content":"hi"}}"#));
    }

    #[test]
    fn injected_text_is_not_a_prompt() {
        assert!(is_injected("<system-reminder>do a thing</system-reminder>"));
        assert!(is_injected("Caveat: the messages below"));
        assert!(!is_injected("fix the failing test"));
    }

    /// A tool RESULT arrives as a user record too. Counting it as a prompt would
    /// feed the model tool output and call it the person's intent.
    #[test]
    fn tool_results_are_not_prompts() {
        let content = serde_json::json!([{"type": "tool_result", "content": "ok"}]);
        assert_eq!(user_text(&content), None);
        let real = serde_json::json!([{"type": "text", "text": "add a test"}]);
        assert_eq!(user_text(&real), Some("add a test".into()));
    }

    /// A long session is sampled from BOTH ends: truncating the tail loses
    /// whether the work actually landed.
    #[test]
    fn long_sessions_keep_their_ending() {
        let prompts: Vec<String> = (0..100).map(|i| format!("prompt number {i}")).collect();
        let s = sample(&prompts);
        assert!(s.contains("prompt number 0"), "keeps the opening");
        assert!(s.contains("prompt number 99"), "keeps the ending");
        assert!(s.contains("omitted"), "says what it dropped");
    }

    #[test]
    fn each_prompt_is_clipped_but_not_lost() {
        let long = "x".repeat(MAX_PROMPT_CHARS * 3);
        let s = sample(&[long, "short one".into()]);
        assert!(s.contains('…'), "clipped");
        assert!(s.contains("short one"), "later prompts survive clipping of an earlier one");
    }
}
