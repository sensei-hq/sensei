//! #90 Per-tool-call usage verdict classifier.
//!
//! Reads `activity.assistant_events` rows in session order, pairs each
//! `PostToolUse` with the next event on the same session, and derives a
//! verdict: **used** / **partial** / **ignored**. Writes verdicts into
//! `sensei.tool_call_verdicts` (see the DDL for schema notes). Idempotent —
//! reruns with a new heuristic just refresh the table (`ON CONFLICT (event_id)
//! DO UPDATE`).
//!
//! Heuristic today is deliberately simple and text-based; the ticket
//! (`#90`) explicitly accepts iteration. An LLM classifier is the natural
//! follow-up.
//!
//! Terminology:
//! - **Pair** — one `PostToolUse` (the "answer") + the next event on the same
//!   session (the "reaction").
//! - **Fragments** — meaningful strings extracted from the answer's
//!   `tool_response`: file-path-shaped tokens, quoted strings, distinctive
//!   identifiers. If any fragment appears in the reaction's `tool_input`
//!   / prompt / next-command, we say the answer was **used**.
//! - **Same target** — the reaction touches the same primary path/URL/symbol
//!   as the answer, but doesn't literally cite the response text (a fuzzy
//!   "you looked at the same thing but didn't reference the reply").

use serde::{Deserialize, Serialize};

/// A pair-derived verdict for one `PostToolUse` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Reaction cites fragments of the answer's response text — the
    /// assistant clearly consumed it.
    Used,
    /// Reaction addresses the same target but without citing the response
    /// — the assistant knew where to look, response text wasn't the reason.
    Partial,
    /// Reaction is unrelated, or there is no reaction (SessionEnd /
    /// UserPromptSubmit / Stop with no in-between tool call).
    Ignored,
}

impl Verdict {
    /// Serialised name matching the `verdict` check constraint on
    /// `sensei.tool_call_verdicts` and the Replay tab's TS enum. Never take
    /// the derived Serde name directly — the storage layer needs a
    /// `&'static str` for the sqlx bind.
    #[allow(dead_code)] // kept for callers that need string form outside the classifier driver
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Used => "used",
            Self::Partial => "partial",
            Self::Ignored => "ignored",
        }
    }
}

/// One (answer, reaction) pair.
#[derive(Debug, Clone)]
pub struct Pair<'a> {
    /// Text of the answer's `payload.tool_response` (concatenated across
    /// shape variants — sometimes a JSON string, sometimes an object with
    /// `content` field, sometimes an array).
    pub response_text: &'a str,
    /// Type of the next event on the same session.
    pub reaction_event_type: &'a str,
    /// Serialised `payload.tool_input` from the next PreToolUse (empty when
    /// the reaction isn't a tool call).
    pub reaction_tool_input: &'a str,
    /// The primary target the answer acted on (file path, URL, symbol) —
    /// derived from the answer's payload.tool_input by [`primary_target`].
    pub answer_target: &'a str,
    /// The primary target of the reaction (same derivation).
    pub reaction_target: &'a str,
}

/// The classifier decision plus supporting evidence for debugging + future
/// heuristic iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub verdict: Verdict,
    pub confidence: f32,
    pub reason: String,
}

/// Extract meaningful string fragments from a response for citation matching.
/// Focuses on tokens that are distinctive enough that finding them in a
/// downstream tool_input is unlikely to be coincidence:
/// - Path-shaped tokens (`crates/foo/bar.rs`, `src/lib.rs`) — length ≥ 5.
/// - Line-number-anchored refs (`file.rs:42`).
/// - Backtick-quoted identifiers ` `fn_name` ` from markdown-style responses.
///
/// Deliberately does NOT extract every whitespace-separated word — common
/// English words would false-positive on almost any reaction.
pub fn extract_fragments(response: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = response.as_bytes();

    // Path-shaped tokens: sequences of [a-zA-Z0-9_/\.\-] length ≥ 5 that
    // contain at least one `/` or `.` (otherwise they're just words).
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && is_path_char(bytes[i]) {
            i += 1;
        }
        if i - start >= 5 {
            let tok = &response[start..i];
            if tok.contains('/') || tok.contains('.') || tok.contains(':') {
                out.push(tok.to_string());
                // Also add a path-only variant with any `:<digits>` line
                // suffix stripped. A response saying "…foo.rs:42" often
                // reappears in the next call as `"file_path":"foo.rs"`
                // with no line number; both should count as a citation.
                if let Some(colon_idx) = tok.rfind(':')
                    && tok[colon_idx + 1..].bytes().all(|b| b.is_ascii_digit())
                {
                    let path_only = &tok[..colon_idx];
                    if path_only.len() >= 5 && (path_only.contains('/') || path_only.contains('.'))
                    {
                        out.push(path_only.to_string());
                    }
                }
            }
        }
        if i == start {
            i += 1;
        }
    }

    // Backtick-quoted identifiers.
    let mut in_tick = false;
    let mut tick_start = 0;
    for (idx, ch) in response.char_indices() {
        if ch == '`' {
            if in_tick {
                let s = &response[tick_start..idx];
                if s.len() >= 3 {
                    out.push(s.to_string());
                }
                in_tick = false;
            } else {
                in_tick = true;
                tick_start = idx + 1;
            }
        }
    }

    out.sort();
    out.dedup();
    out
}

fn is_path_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-' | b':')
}

/// Try to identify the primary target of a tool call from its serialised
/// `tool_input` — the file path / URL / symbol the tool is acting on.
/// Best-effort: peeks at common field names (`path`, `file_path`, `url`,
/// `pattern`, `command`); returns the empty string when none are present.
pub fn primary_target(tool_input_json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(tool_input_json) else {
        return String::new();
    };
    for key in ["file_path", "path", "url", "pattern", "command"] {
        if let Some(s) = v.get(key).and_then(|x| x.as_str())
            && !s.is_empty()
        {
            return s.to_string();
        }
    }
    String::new()
}

/// Classify a (answer, reaction) pair — the pure decision function.
pub fn classify(pair: &Pair<'_>) -> Classification {
    // No reaction at all → ignored.
    if pair.reaction_event_type.is_empty() {
        return Classification {
            verdict: Verdict::Ignored,
            confidence: 1.0,
            reason: "no reaction event on the same session".into(),
        };
    }

    // Reaction is a hard end — user hit Stop, session ended, or the
    // assistant took a fresh prompt without another tool call in between.
    if matches!(pair.reaction_event_type, "Stop" | "SessionEnd" | "UserPromptSubmit") {
        return Classification {
            verdict: Verdict::Ignored,
            confidence: 0.9,
            reason: format!("reaction is {} — no downstream tool call", pair.reaction_event_type),
        };
    }

    // Reaction is a tool call. Look for citation of the answer text.
    let frags = extract_fragments(pair.response_text);
    if frags.is_empty() {
        // Nothing distinctive to cite — fall back to target-equality.
        return classify_by_target(pair, "no citable fragments in response");
    }

    let mut hits: Vec<&String> =
        frags.iter().filter(|f| pair.reaction_tool_input.contains(f.as_str())).collect();
    hits.sort();
    hits.dedup();

    if !hits.is_empty() {
        let reason = format!(
            "reaction cites {} fragment(s): {}",
            hits.len(),
            hits.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
        );
        // More hits = higher confidence, capped at 0.99.
        let conf = (0.6 + 0.1 * hits.len() as f32).min(0.99);
        return Classification { verdict: Verdict::Used, confidence: conf, reason };
    }

    classify_by_target(pair, "no fragment overlap")
}

fn classify_by_target(pair: &Pair<'_>, no_cite_reason: &str) -> Classification {
    if !pair.answer_target.is_empty()
        && !pair.reaction_target.is_empty()
        && pair.answer_target == pair.reaction_target
    {
        return Classification {
            verdict: Verdict::Partial,
            confidence: 0.7,
            reason: format!("{no_cite_reason}; same target {}", pair.answer_target),
        };
    }
    Classification {
        verdict: Verdict::Ignored,
        confidence: 0.6,
        reason: format!("{no_cite_reason}; different target"),
    }
}

/// Classify every `PostToolUse` event on one session and upsert verdicts.
/// Returns the number of verdicts written. Idempotent — reruns with a new
/// heuristic just refresh the rows.
///
/// Best-effort: sessions with no `PostToolUse` events return 0. Events that
/// end the session (last event of the log) have no reaction and produce no
/// verdict (the assistant might still consume the response later — we don't
/// want to prematurely classify).
pub async fn classify_session(
    pg: &crate::db::pg_store::PgStore,
    client_session_id: &str,
) -> Result<usize, String> {
    let events = pg.get_hook_events_for_session_with_id(client_session_id).await?;
    if events.is_empty() {
        return Ok(0);
    }

    // Walk the event list in ts order. For every PostToolUse, look at the
    // next event on the same session (i+1) and derive a verdict.
    // Row = (event_id, ts, tool_name, verdict, confidence, reason).
    type VerdictRow = (String, i64, Option<String>, &'static str, f32, String);
    let mut rows: Vec<VerdictRow> = Vec::new();
    for i in 0..events.len() {
        let (event_id, event_type, tool_name, _ts, payload) = &events[i];
        if event_type != "PostToolUse" {
            continue;
        }
        // Get the next event; if there isn't one, skip — we don't classify a
        // dangling last call.
        let Some(next) = events.get(i + 1) else { continue };
        let (_next_id, next_type, _next_tool, _next_ts, next_payload) = next;

        let response_text = extract_response_text(payload);
        let answer_target = primary_target(
            &payload
                .get("tool_input")
                .and_then(|v| serde_json::to_string(v).ok())
                .unwrap_or_default(),
        );
        let next_input_json = next_payload
            .get("tool_input")
            .and_then(|v| serde_json::to_string(v).ok())
            .unwrap_or_default();
        let reaction_target = primary_target(&next_input_json);

        let pair = Pair {
            response_text: &response_text,
            reaction_event_type: next_type,
            reaction_tool_input: &next_input_json,
            answer_target: &answer_target,
            reaction_target: &reaction_target,
        };
        let c = classify(&pair);

        // `verdict` needs a &'static str — map from the enum's as_str.
        let v: &'static str = match c.verdict {
            Verdict::Used => "used",
            Verdict::Partial => "partial",
            Verdict::Ignored => "ignored",
        };
        rows.push((
            client_session_id.to_string(),
            *event_id,
            tool_name.clone(),
            v,
            c.confidence,
            c.reason,
        ));
    }

    pg.upsert_verdicts_batch(&rows).await
}

/// Peel `payload.tool_response` down to the underlying text — the daemon's
/// hook writer stores it in one of three shapes depending on the tool:
/// - Plain string (Bash: stdout).
/// - Object `{ "content": "..." }` (Edit / Write).
/// - Array `[ { text: "..." }, ... ]` (MCP results).
pub fn extract_response_text(payload: &serde_json::Value) -> String {
    let Some(resp) = payload.get("tool_response") else { return String::new() };
    if let Some(s) = resp.as_str() {
        return s.to_string();
    }
    if let Some(o) = resp.as_object() {
        for k in ["content", "text", "output", "stdout"] {
            if let Some(s) = o.get(k).and_then(|v| v.as_str()) {
                return s.to_string();
            }
        }
        // Whole object as JSON fallback.
        return serde_json::to_string(o).unwrap_or_default();
    }
    if let Some(a) = resp.as_array() {
        let mut out = String::new();
        for entry in a {
            if let Some(s) = entry.as_str() {
                out.push_str(s);
                out.push('\n');
            } else if let Some(s) = entry.get("text").and_then(|v| v.as_str()) {
                out.push_str(s);
                out.push('\n');
            }
        }
        return out;
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_fragments_pulls_path_like_tokens() {
        let resp = "See crates/senseid/src/api/routes.rs:42 for the wiring.";
        let frags = extract_fragments(resp);
        assert!(
            frags.iter().any(|f| f == "crates/senseid/src/api/routes.rs:42"),
            "path+line ref extracted, got: {frags:?}"
        );
    }

    #[test]
    fn extract_fragments_skips_short_common_words() {
        // "See" and "for" and "the" would false-positive on any English
        // reaction. Only path/dot/colon-bearing tokens survive.
        let resp = "See it now, right there at the top.";
        let frags = extract_fragments(resp);
        assert!(frags.is_empty(), "short common words shouldn't survive, got: {frags:?}");
    }

    #[test]
    fn extract_fragments_grabs_backtick_identifiers() {
        let resp = "Call `resolve_rules_raw` and `structure_ruleset`.";
        let frags = extract_fragments(resp);
        assert!(frags.iter().any(|f| f == "resolve_rules_raw"));
        assert!(frags.iter().any(|f| f == "structure_ruleset"));
    }

    #[test]
    fn primary_target_reads_common_fields() {
        assert_eq!(
            primary_target(r#"{"file_path":"src/lib.rs","pattern":"fn"}"#),
            "src/lib.rs",
            "file_path preferred over pattern",
        );
        assert_eq!(primary_target(r#"{"url":"https://example.com/x"}"#), "https://example.com/x",);
        assert_eq!(primary_target(r#"{"unrelated":"yes"}"#), "");
        assert_eq!(primary_target("not json"), "");
    }

    #[test]
    fn classify_used_when_reaction_cites_a_fragment() {
        let pair = Pair {
            response_text: "The bug is at crates/senseid/src/db/pg_store.rs:3864",
            reaction_event_type: "PreToolUse",
            reaction_tool_input: r#"{"file_path":"crates/senseid/src/db/pg_store.rs","new_string":"…"}"#,
            answer_target: "crates/senseid/src/db/pg_store.rs",
            reaction_target: "crates/senseid/src/db/pg_store.rs",
        };
        let c = classify(&pair);
        assert_eq!(c.verdict, Verdict::Used);
        assert!(c.confidence >= 0.7);
        assert!(c.reason.contains("cites"), "reason: {}", c.reason);
    }

    #[test]
    fn classify_partial_when_same_target_but_no_citation() {
        let pair = Pair {
            response_text: "Nothing distinctive here in prose form.",
            reaction_event_type: "PreToolUse",
            reaction_tool_input: r#"{"file_path":"a/b/c.rs"}"#,
            answer_target: "a/b/c.rs",
            reaction_target: "a/b/c.rs",
        };
        let c = classify(&pair);
        assert_eq!(c.verdict, Verdict::Partial);
        assert!(c.reason.contains("same target"));
    }

    #[test]
    fn classify_ignored_when_next_event_is_user_prompt() {
        let pair = Pair {
            response_text: "Any response.",
            reaction_event_type: "UserPromptSubmit",
            reaction_tool_input: "",
            answer_target: "a/b/c.rs",
            reaction_target: "",
        };
        let c = classify(&pair);
        assert_eq!(c.verdict, Verdict::Ignored);
        assert!(c.reason.contains("UserPromptSubmit"));
    }

    #[test]
    fn classify_ignored_when_targets_differ_and_no_citation() {
        let pair = Pair {
            response_text: "Nothing citable.",
            reaction_event_type: "PreToolUse",
            reaction_tool_input: r#"{"file_path":"unrelated.md"}"#,
            answer_target: "a/b/c.rs",
            reaction_target: "unrelated.md",
        };
        let c = classify(&pair);
        assert_eq!(c.verdict, Verdict::Ignored);
    }

    #[test]
    fn classify_ignored_when_no_reaction() {
        let pair = Pair {
            response_text: "Anything.",
            reaction_event_type: "",
            reaction_tool_input: "",
            answer_target: "",
            reaction_target: "",
        };
        let c = classify(&pair).verdict;
        assert_eq!(c, Verdict::Ignored);
    }

    #[test]
    fn extract_response_text_handles_string_variant() {
        let p = serde_json::json!({"tool_response": "hello"});
        assert_eq!(extract_response_text(&p), "hello");
    }

    #[test]
    fn extract_response_text_handles_object_content() {
        let p = serde_json::json!({"tool_response": {"content": "body text", "extra": 1}});
        assert_eq!(extract_response_text(&p), "body text");
    }

    #[test]
    fn extract_response_text_handles_array_of_text_parts() {
        let p = serde_json::json!({"tool_response": [
            {"text": "part 1"},
            "part 2",
        ]});
        let s = extract_response_text(&p);
        assert!(s.contains("part 1"));
        assert!(s.contains("part 2"));
    }

    #[test]
    fn extract_response_text_none_when_missing() {
        let p = serde_json::json!({"other": "field"});
        assert_eq!(extract_response_text(&p), "");
    }

    #[test]
    fn used_confidence_climbs_with_more_hits() {
        let one_hit = classify(&Pair {
            response_text: "single/frag.rs",
            reaction_event_type: "PreToolUse",
            reaction_tool_input: r#"{"file_path":"single/frag.rs"}"#,
            answer_target: "single/frag.rs",
            reaction_target: "single/frag.rs",
        });
        let three_hits = classify(&Pair {
            response_text: "path/a.rs and path/b.rs and path/c.rs",
            reaction_event_type: "PreToolUse",
            reaction_tool_input: r#"{"file_path":"path/a.rs","other":"path/b.rs; path/c.rs"}"#,
            answer_target: "path/a.rs",
            reaction_target: "path/a.rs",
        });
        assert_eq!(one_hit.verdict, Verdict::Used);
        assert_eq!(three_hits.verdict, Verdict::Used);
        assert!(
            three_hits.confidence > one_hit.confidence,
            "3 hits ({}) should beat 1 hit ({})",
            three_hits.confidence,
            one_hit.confidence
        );
    }
}
