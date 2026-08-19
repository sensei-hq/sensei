//! Session enrichment — analyzer layer L0 (#66) + signal derivation L1 (#68).
//!
//! Sessions are created metric-less by the #31 hook derivation, so the captured
//! `activity.assistant_events` stream is the only signal. This stage turns a
//! session's events into per-turn rows
//! (`activity.turns`) + session aggregates (`activity.sessions`):
//!
//! * a **turn** spans one `UserPromptSubmit` to the next; it carries a
//!   `segment` marker that increments after an idle gap > [`IDLE_GAP_MS`], so a
//!   multi-day *resumed* session splits into work segments (sub-sessions) while
//!   staying one session row;
//! * the session's **duration** is gap-aware *active* time (idle/away gaps
//!   excluded), so a session resumed across days reports real work, not its
//!   calendar span; `started_at`..`completed_at` still hold the full span.
//!
//! Pure derivation is decoupled from the DB so it is unit-testable over an
//! in-memory slice; the orchestrators (`enrich_session`, `analyze_project`)
//! handle the I/O.

use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::prompt_classify::{classify_batch, PromptClass};
use crate::transcript::TranscriptTurn;

/// Idle gap (ms) that separates "still working" from "came back later" — turns
/// further apart than this start a new segment, and the gap is excluded from
/// active duration. 30 minutes.
const IDLE_GAP_MS: i64 = 30 * 60 * 1000;

/// A file re-edited at least this many times within a SINGLE session is a
/// rework/churn anti-pattern — the agent kept returning to it (#68). Tool
/// failures aren't captured by the hooks, so re-edit churn is the derivable
/// "the result needed follow-ups" signal (~16% of (session,file) pairs in the
/// live corpus hit 5+).
const CHURN_MIN_EDITS: i64 = 5;

/// A folder needs at least this many corrective prompts to be flagged
/// "correction-prone".
const CORRECTION_MIN: usize = 2;

/// Max chars of a prompt stored in a pattern instance (keep rows small).
const PROMPT_SNIPPET_MAX: usize = 200;

/// One hook event projected to just the fields the heuristics read — decoupled
/// from the DB row so derivation is a pure function.
#[derive(Debug, Clone)]
pub struct HookEvent {
    pub event_type: String,
    pub tool_name: Option<String>,
    pub ts: i64,
    pub prompt: Option<String>,
    pub file_path: Option<String>,
    pub tool_failed: bool,
    /// `command_invoked.payload.action` (e.g. `"resume"`) — the in-session marker
    /// that a session was reopened/continued (Phase B). `None` for other events.
    pub action: Option<String>,
}

/// One turn: a `UserPromptSubmit` and the work until the next prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    pub turn_number: i32,
    pub segment: i32,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub duration_ms: i64,
    pub is_correction: bool,
    pub triage_signal: Option<&'static str>,
    pub tool_calls: i32,
}

/// Derived per-session metrics written to `activity.sessions` (+ the per-turn
/// detail written to `activity.turns`).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionMetrics {
    /// Per-turn detail rows (hook-derived — timings + tool calls), written to
    /// `activity.turns`. May be SHORTER than `turn_count` when the transcript is
    /// richer than the captured hook stream (the drill-down uses `turn_count`).
    pub turns: Vec<Turn>,
    /// The session-level turn COUNT — transcript turn count (ground truth) when a
    /// transcript exists, else the hook turn count. This is what the metric reads
    /// (e.g. `792d7ce4` = 94, not the sparse hooks' 1).
    pub turn_count: i32,
    pub corrections: i32,
    pub outcome: &'static str, // a `sensei.session_outcome` label
    pub ftr: bool,
    /// The session carried an in-session `command_invoked{action:resume}` marker —
    /// it was reopened/continued (Phase B). Written to `sessions.props.resumed`.
    pub resumed: bool,
    pub duration_ms: i64, // session-level gap-aware active time
    pub module: Option<String>,
    pub tool_usage: serde_json::Value, // { "<tool>": { "pre", "post", "failed" } }
}

/// A user prompt that signals the previous turn needed correcting — the FTR
/// detractor. Maps onto the schema's `triage_signal` vocabulary. PRECISION-
/// favoring: a false correction wrongly tanks FTR, so only unambiguous
/// phrasings count (plain instructions like "don't forget the test" must not
/// match). Tunable as we see real data.
pub fn correction_signal(prompt: &str) -> Option<&'static str> {
    let p = prompt.trim().to_lowercase();
    const REVERT: &[&str] = &["revert", "roll back", "undo that", "undo the", "undo your"];
    const WRONG: &[&str] = &[
        "that's wrong", "thats wrong", "that's not right", "thats not right",
        "that's not what", "thats not what", "not what i asked", "you missed",
        "doesn't work", "does not work", "didn't work", "did not work",
        "still broken", "still failing", "still fails", "you broke", "that broke it",
        "is incorrect", "is wrong",
    ];
    const WHY: &[&str] = &["why did you", "why are you", "why'd you", "why would you"];
    const ACTUALLY: &[&str] = &["actually,", "actually ", "wait,", "wait ", "no, that"];
    if REVERT.iter().any(|s| p.contains(s)) {
        return Some("revert");
    }
    if WHY.iter().any(|s| p.contains(s)) {
        return Some("why");
    }
    if WRONG.iter().any(|s| p.contains(s)) {
        return Some("correction");
    }
    if ACTUALLY.iter().any(|s| p.starts_with(s)) {
        return Some("actually");
    }
    None
}

/// Detect an imperative *principle/rule* in a user prompt — the user stating a
/// durable "do X / never Y" expectation (a teaching/rule candidate, distinct
/// from a one-off correction). Keyword match; an LLM classifier (L2) refines
/// precision later. Returns the matched cue.
pub fn principle_signal(prompt: &str) -> Option<&'static str> {
    let p = prompt.trim().to_lowercase();
    const CUES: &[&str] = &[
        "you should always", "you should never", "you must always", "you must never",
        "always make sure", "make sure to", "make sure you", "make sure we",
        "from now on", "going forward", "as a rule", "don't ever", "never forget",
        "you should", "you must", "please always", "please never",
    ];
    CUES.iter().copied().find(|s| p.contains(*s))
}

/// A POSITIVE, explicit "this session is being given up / paused" cue — the user
/// abandoning, or Claude advising a stop / resume-later / fresh session. This is
/// the ONLY thing that marks a session `abandoned`: a missing end event is an
/// absence, never abandonment (see `derive_outcome`). PRECISION-favoring — only
/// unambiguous phrasings, so ordinary work is never mislabeled. Reads both the
/// user turn and Claude's reply (Phase D mines these hints further).
pub fn abandonment_signal(user_text: &str, assistant_text: &str) -> bool {
    let u = user_text.trim().to_lowercase();
    let a = assistant_text.trim().to_lowercase();
    const USER: &[&str] = &[
        "i give up", "let's abandon", "lets abandon", "abandon this", "give up on this",
        "forget it", "forget this", "let's stop here", "lets stop here", "stop working on this",
        "i'll come back to this later", "come back to this later", "let's scrap", "lets scrap",
    ];
    const CLAUDE: &[&str] = &[
        "start a new session", "start a fresh session", "running low on context",
        "low on context", "resume this later", "let's resume later", "pick this up later",
        "continue in a new session", "out of context",
    ];
    USER.iter().any(|s| u.contains(s)) || CLAUDE.iter().any(|s| a.contains(s))
}

/// Sum of consecutive-event gaps below the idle threshold — active time, with
/// idle/away gaps (segment boundaries, multi-day resumes) excluded. Events are
/// assumed oldest-first (the DB returns them ordered by ts).
fn active_duration_ms(events: &[HookEvent], idle_ms: i64) -> i64 {
    events
        .windows(2)
        .map(|w| {
            let gap = w[1].ts - w[0].ts;
            if gap > 0 && gap <= idle_ms { gap } else { 0 }
        })
        .sum()
}

/// Split a session's events into turns at `UserPromptSubmit` boundaries and
/// assign segment numbers (a new segment after an idle gap > `idle_ms`). Events
/// before the first prompt (SessionStart, etc.) are not a turn.
fn split_into_turns(events: &[HookEvent], idle_ms: i64) -> Vec<Turn> {
    let starts: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.event_type == "UserPromptSubmit")
        .map(|(i, _)| i)
        .collect();

    let mut turns: Vec<Turn> = starts
        .iter()
        .enumerate()
        .map(|(k, &si)| {
            let end = starts.get(k + 1).copied().unwrap_or(events.len()); // exclusive
            let slice = &events[si..end];
            let triage_signal = events[si].prompt.as_deref().and_then(correction_signal);
            Turn {
                turn_number: (k + 1) as i32,
                segment: 1,
                started_ms: events[si].ts,
                ended_ms: slice.last().map(|e| e.ts).unwrap_or(events[si].ts),
                duration_ms: active_duration_ms(slice, idle_ms),
                is_correction: triage_signal.is_some(),
                triage_signal,
                tool_calls: slice.iter().filter(|e| e.event_type == "PostToolUse").count() as i32,
            }
        })
        .collect();

    for i in 1..turns.len() {
        let gap = turns[i].started_ms - turns[i - 1].ended_ms;
        turns[i].segment = if gap > idle_ms {
            turns[i - 1].segment + 1
        } else {
            turns[i - 1].segment
        };
    }
    turns
}

/// Failed `PostToolUse` events among the last few events — an error cluster at
/// the tail of a session with no clean end suggests it was blocked.
fn trailing_failures(events: &[HookEvent]) -> usize {
    let window = events.len().min(5);
    events[events.len() - window..]
        .iter()
        .filter(|e| e.event_type == "PostToolUse" && e.tool_failed)
        .count()
}

/// `session_outcome` label under the transcript-ground-truth taxonomy. `real_turns`
/// is the max of transcript and hook turn counts (0 ⇒ nothing was attempted).
///
/// - **empty** — 0 turns: not a measured outcome (the read path excludes it from
///   throughput/ftr). Never a signal card.
/// - **abandoned** — ONLY on a POSITIVE transcript abandonment signal (user gave
///   up, or Claude advised stop/resume-later). Phase B additionally spares any
///   session later resumed. NEVER inferred from a missing end event.
/// - **completed / corrected** — a clean end (Stop/SessionEnd); `corrected` if the
///   user had to correct.
/// - **blocked** — no clean end but a tail error cluster.
/// - **incomplete** — real work, no clean end, no abandonment signal: a neutral
///   crash/close, NOT a failure and NOT abandoned (this is what ~42% of sessions
///   with a missing `SessionEnd` actually are).
fn derive_outcome(
    events: &[HookEvent],
    transcript: &[TranscriptTurn],
    real_turns: usize,
    corrections: i32,
    resumed: bool,
) -> &'static str {
    if real_turns == 0 {
        return "empty";
    }
    // `abandoned` requires a positive signal AND no resume-link: a session that was
    // reopened/continued is never abandoned (Phase B), even if a turn said "stop".
    let abandoned = !resumed
        && transcript.iter().any(|t| {
            abandonment_signal(t.user_text.as_deref().unwrap_or(""), &t.assistant_text)
        });
    if abandoned {
        return "abandoned";
    }
    let has_end = events
        .iter()
        .any(|e| e.event_type == "Stop" || e.event_type == "SessionEnd");
    if has_end {
        if corrections > 0 { "corrected" } else { "completed" }
    } else if trailing_failures(events) >= 2 {
        "blocked"
    } else {
        "incomplete"
    }
}

/// Parent directory of a file path — the "module" locus. `pub(super)` so the
/// sibling `session_retro` facts-gatherer derives "module" the same way.
pub(super) fn parent_dir(path: &str) -> Option<String> {
    path.rsplit_once('/').map(|(dir, _)| dir.to_string()).filter(|d| !d.is_empty())
}

/// Most-touched directory across the session's file operations.
fn dominant_module(events: &[HookEvent]) -> Option<String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in events {
        if let Some(fp) = e.file_path.as_deref()
            && let Some(dir) = parent_dir(fp)
        {
            *counts.entry(dir).or_default() += 1;
        }
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(dir, _)| dir)
}

/// Per-tool { pre, post, failed } counts, as a stable (sorted) JSON object.
fn tally_tool_usage(events: &[HookEvent]) -> serde_json::Value {
    let mut map: std::collections::BTreeMap<String, (i64, i64, i64)> =
        std::collections::BTreeMap::new();
    for e in events {
        let Some(tool) = e.tool_name.as_deref() else { continue };
        let entry = map.entry(tool.to_string()).or_default();
        match e.event_type.as_str() {
            "PreToolUse" => entry.0 += 1,
            "PostToolUse" => {
                entry.1 += 1;
                if e.tool_failed {
                    entry.2 += 1;
                }
            }
            _ => {}
        }
    }
    let obj: serde_json::Map<String, serde_json::Value> = map
        .into_iter()
        .map(|(tool, (pre, post, failed))| {
            (tool, serde_json::json!({ "pre": pre, "post": post, "failed": failed }))
        })
        .collect();
    serde_json::Value::Object(obj)
}

/// Derive metrics for one session. The **transcript is ground truth** for the
/// turn count and corrections (hooks are frequently sparse/incomplete — a session
/// can miss `SessionEnd` or undercount turns); the hook stream corroborates and
/// supplies per-turn timing + tool usage. Falls back to hooks only when no
/// transcript was captured. `None` only when BOTH streams are empty (don't
/// fabricate an outcome for a session we saw nothing of).
pub fn derive_session_metrics(
    events: &[HookEvent],
    transcript: &[TranscriptTurn],
) -> Option<SessionMetrics> {
    if events.is_empty() && transcript.is_empty() {
        return None;
    }
    // Hook-derived per-turn detail (timings, tool calls) — supplementary rows.
    let turns = split_into_turns(events, IDLE_GAP_MS);
    // Transcript-first turn count + corrections; hooks are the fallback only when
    // no transcript exists. Corrections reuse `correction_signal` over the real
    // `user_text`, so `792d7ce4` reads its 94 turns rather than the hooks' 1.
    let (turn_count, corrections) = if transcript.is_empty() {
        (turns.len() as i32, turns.iter().filter(|t| t.is_correction).count() as i32)
    } else {
        let c = transcript
            .iter()
            .filter(|t| t.user_text.as_deref().is_some_and(|u| correction_signal(u).is_some()))
            .count() as i32;
        (transcript.len() as i32, c)
    };
    let real_turns = (turn_count as usize).max(turns.len());
    let resumed = was_resumed(events);
    Some(SessionMetrics {
        outcome: derive_outcome(events, transcript, real_turns, corrections, resumed),
        ftr: corrections == 0,
        duration_ms: active_duration_ms(events, IDLE_GAP_MS),
        module: dominant_module(events),
        tool_usage: tally_tool_usage(events),
        corrections,
        turn_count,
        turns,
        resumed,
    })
}

/// Serialize turns to the JSON array shape `replace_session_turns` expands into
/// rows (ms epochs / ms durations → timestamptz / interval in SQL).
fn turns_to_json(turns: &[Turn]) -> serde_json::Value {
    serde_json::Value::Array(
        turns
            .iter()
            .map(|t| {
                serde_json::json!({
                    "turn_number": t.turn_number,
                    "segment": t.segment,
                    "started_ms": t.started_ms,
                    "ended_ms": t.ended_ms,
                    "duration_ms": t.duration_ms,
                    "is_correction": t.is_correction,
                    "triage_signal": t.triage_signal,
                    "tool_calls": t.tool_calls,
                })
            })
            .collect(),
    )
}

/// Map a `{event_type, tool_name, ts, payload}` hook_events row to a HookEvent.
/// `success` column is unreliable for PostToolUse (NULL at ingest), so failure
/// is read from `tool_response.is_error` / an `error` key instead.
fn hook_event_from_row(row: &serde_json::Value) -> HookEvent {
    let payload = row.get("payload").cloned().unwrap_or(serde_json::Value::Null);
    let prompt = payload.get("prompt").and_then(|v| v.as_str()).map(str::to_string);
    let action = payload.get("action").and_then(|v| v.as_str()).map(str::to_string);
    let file_path = payload
        .get("file_path")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("tool_input").and_then(|t| t.get("file_path")).and_then(|v| v.as_str()))
        .map(str::to_string);
    let tool_failed = match payload.get("tool_response") {
        Some(serde_json::Value::Object(o)) => {
            o.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false) || o.contains_key("error")
        }
        _ => false,
    };
    HookEvent {
        event_type: row.get("event_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        tool_name: row.get("tool_name").and_then(|v| v.as_str()).map(str::to_string),
        ts: row.get("ts").and_then(|v| v.as_i64()).unwrap_or(0),
        prompt,
        file_path,
        tool_failed,
        action,
    }
}

/// Whether the session was reopened/continued: it carries an in-session
/// `command_invoked{action:resume…}` marker (the resume event is logged under the
/// resumed session's own id). Such a session is NEVER abandoned — it was
/// continued (Phase B).
fn was_resumed(events: &[HookEvent]) -> bool {
    events.iter().any(|e| {
        e.event_type == "command_invoked"
            && e.action.as_deref().is_some_and(|a| a.trim().to_lowercase().starts_with("resume"))
    })
}

/// Tidy a raw transcript turn for a one-line quote: drop fenced code blocks
/// (noise in a quote), strip inline markdown emphasis/heading/bullet markers, and
/// collapse all whitespace to single spaces. Deterministic — no summarisation
/// model, so the quote stays the user's/assistant's real words, just neater.
fn clean_prose(s: &str) -> String {
    let mut kept = String::new();
    let mut in_fence = false;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        // Strip a leading markdown marker (heading #, quote >, bullet -/*/+).
        let stripped = trimmed
            .trim_start_matches(['#', '>', '-', '*', '+', ' '])
            .to_string();
        kept.push_str(&stripped);
        kept.push(' ');
    }
    let no_emphasis = kept.replace("**", "").replace("__", "").replace('`', "");
    no_emphasis.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cap a cleaned quote at ~`max` chars, ending on a sentence boundary (`.!?`) when
/// one is near, else a word boundary — so the "important action" reads as a whole
/// thought, never a mid-word truncation. Appends `…` only when it actually cut.
fn clip_sentence(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    // Prefer the last sentence end past the halfway mark, else the last space.
    let cut = head
        .rmatch_indices(['.', '!', '?'])
        .map(|(i, m)| i + m.len())
        .find(|&i| i > max / 2)
        .or_else(|| head.rfind(' '))
        .unwrap_or(head.len());
    format!("{}…", head[..cut].trim())
}

/// Phase C: DETERMINISTIC drill-down evidence — the real transcript turns that
/// ground a session's signals, cleaned (markdown/code stripped, sentence-capped)
/// but never summarised by a model, with NO invented causality. Selects the
/// opening ask, the first correction (the FTR detractor), and the closing result.
/// `assistant_label` is the ACP the transcript came from (e.g. `claude`) — the
/// assistant `who`, NEVER "sensei". A session with no transcript yields `null`.
/// Shape: `{ "source": "transcript", "moments": [{ "turn", "who", "text", "kind"? }] }`.
pub fn build_session_evidence(transcript: &[TranscriptTurn], assistant_label: &str) -> serde_json::Value {
    const MAX: usize = 220;
    let tidy = |s: &str| clip_sentence(&clean_prose(s), MAX);
    let mut moments: Vec<serde_json::Value> = Vec::new();
    let mut first_turn: Option<i32> = None;

    // Opening ask — the first non-empty user turn (what was asked of this session).
    if let Some(first) = transcript
        .iter()
        .find(|t| t.user_text.as_deref().is_some_and(|u| !u.trim().is_empty()))
    {
        first_turn = Some(first.turn_index);
        moments.push(serde_json::json!({
            "turn": first.turn_index, "who": "you",
            "text": tidy(first.user_text.as_deref().unwrap_or("")),
        }));
    }
    // First correction — the FTR detractor, quoted from the user turn that flags it
    // (skip when it IS the opening turn, to avoid a duplicate moment).
    if let Some(corr) = transcript
        .iter()
        .find(|t| t.user_text.as_deref().is_some_and(|u| correction_signal(u).is_some()))
        && Some(corr.turn_index) != first_turn
    {
        moments.push(serde_json::json!({
            "turn": corr.turn_index, "who": "you", "kind": "correction",
            "text": tidy(corr.user_text.as_deref().unwrap_or("")),
        }));
    }
    // Closing result — the last non-empty assistant turn, labelled with the ACP.
    if let Some(last) = transcript.iter().rev().find(|t| !t.assistant_text.trim().is_empty())
        && Some(last.turn_index) != first_turn
    {
        moments.push(serde_json::json!({
            "turn": last.turn_index, "who": assistant_label, "text": tidy(&last.assistant_text),
        }));
    }

    if moments.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::json!({ "source": "transcript", "moments": moments })
}

/// A "something's going wrong" hint in Claude's reply — it flagged context
/// pressure, advised a restart, or reported being stuck. Precision-favoring, and
/// DISTINCT from `abandonment_signal` (which decides the Phase-A outcome): a
/// session can complete yet still show trouble. Returns the category. (Phase D)
pub fn trouble_hint(assistant_text: &str) -> Option<&'static str> {
    let a = assistant_text.trim().to_lowercase();
    if a.contains("running low on context") || a.contains("low on context")
        || a.contains("out of context") || a.contains("running out of context")
        || a.contains("context window is") || a.contains("hitting the context")
    {
        return Some("context-pressure");
    }
    if a.contains("start a new session") || a.contains("start a fresh session")
        || a.contains("continue in a new session") || a.contains("resume this later")
        || a.contains("pick this up later")
    {
        return Some("suggested-restart");
    }
    if a.contains("i'm stuck") || a.contains("i am stuck") || a.contains("going in circles")
        || a.contains("unable to resolve") || a.contains("keep hitting the same")
    {
        return Some("stuck");
    }
    None
}

/// Phase D: a trouble CASE for a session — the first trouble hint in Claude's
/// replies, correlated with context-pressure signals (`PreCompact` count, turns,
/// active duration). `null` when no hint fired. A new signal *family* (a case
/// list surfaced on the drill-down), never a metric value. Shape:
/// `{ "hint", "precompact", "turns", "duration_ms" }`.
fn detect_session_trouble(
    events: &[HookEvent], transcript: &[TranscriptTurn], turn_count: i32, duration_ms: i64,
) -> serde_json::Value {
    let Some(hint) = transcript.iter().find_map(|t| trouble_hint(&t.assistant_text)) else {
        return serde_json::Value::Null;
    };
    let precompact = events.iter().filter(|e| e.event_type == "PreCompact").count() as i64;
    serde_json::json!({
        "hint": hint,
        "precompact": precompact,
        "turns": turn_count,
        "duration_ms": duration_ms,
    })
}

/// Enrich one session in place: write the session aggregates and replace its
/// turn rows. Returns `true` if metrics were written, `false` if the session
/// had no hook events (left untouched). Idempotent — recompute overwrites.
pub async fn enrich_session(
    ctx: &TaskContext,
    session_id: &uuid::Uuid,
    client_session_id: &str,
) -> Result<bool, String> {
    let rows = ctx.pg().get_hook_events_for_session(client_session_id).await?;
    let events: Vec<HookEvent> = rows.iter().map(hook_event_from_row).collect();
    // Transcript is ground truth (turns/corrections/outcome); hooks corroborate.
    // `family` is the ACP the transcript came from (e.g. claude) — the evidence's
    // assistant label, never "sensei".
    let (transcript, family) = ctx.pg().get_transcript_turns_for_session(client_session_id).await?;
    match derive_session_metrics(&events, &transcript) {
        Some(m) => {
            ctx.pg()
                .update_session_metrics(
                    session_id, m.turn_count, m.corrections, m.outcome, m.ftr,
                    m.duration_ms, m.module.as_deref(), &m.tool_usage,
                )
                .await?;
            ctx.pg().replace_session_turns(session_id, &turns_to_json(&m.turns)).await?;
            // Phase B: record the in-session resume marker so the read path shows
            // "resumed" and never treats it as abandoned. Non-fatal; a no-op when
            // unchanged (guarded), so a steady-state re-enrich writes nothing.
            if let Err(e) = ctx.pg().set_session_resumed(session_id, m.resumed).await {
                tracing::warn!(error = %e, session = %session_id, "enrich_session: set_session_resumed failed");
            }
            // Phase C: deterministic transcript-sourced evidence for the drill-down
            // (real quoted moments, no invented causality). Non-fatal; guarded.
            let evidence = build_session_evidence(&transcript, family.as_deref().unwrap_or("assistant"));
            if let Err(e) = ctx.pg().set_session_evidence(session_id, &evidence).await {
                tracing::warn!(error = %e, session = %session_id, "enrich_session: set_session_evidence failed");
            }
            // Phase D: a trouble case (Claude struggle/context-pressure hint) with
            // its PreCompact/turns/duration correlation, or cleared when none.
            let trouble = detect_session_trouble(&events, &transcript, m.turn_count, m.duration_ms);
            if let Err(e) = ctx.pg().set_session_trouble(session_id, &trouble).await {
                tracing::warn!(error = %e, session = %session_id, "enrich_session: set_session_trouble failed");
            }

            // Retrospective narrative → activity.sessions.summary. Deterministic
            // facts stay code-owned; only the prose routes through insight-copy,
            // which degrades to a deterministic fallback on a gateway miss. Written
            // with a refresh-if-changed guard so a re-derivation (the backfill)
            // corrects a now-stale line (e.g. an outcome that flipped
            // abandoned → completed) — non-fatal, logged not propagated.
            let facts = super::session_retro::gather_session_facts(&events, &m);
            let summary = super::session_retro::generate_session_summary(
                ctx.pg(), &ctx.app_state.gateway, &facts,
            ).await;
            if let Err(e) = ctx.pg().set_session_summary(session_id, &summary).await {
                tracing::warn!(error = %e, session = %session_id, "enrich_session: set_session_summary failed");
            }
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Handler for `TaskKind::AnalyzeProject`: enrich every attributed session of a
/// project. `task.path` carries the project id (UUID string).
pub async fn analyze_project(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(&task.path)
        .map_err(|_| format!("AnalyzeProject: invalid project id '{}'", task.path))?;
    let sessions = ctx.pg().get_project_sessions_needing_enrichment(&project_id).await?;
    let mut enriched = 0u32;
    // Folders touched by sessions enriched THIS pass — derivation is scoped to
    // them so we don't recompute every folder's patterns on each new session.
    let mut affected: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();
    for (id, client_session_id, folder_id) in sessions {
        match enrich_session(ctx, &id, &client_session_id).await {
            Ok(true) => {
                enriched += 1;
                if let Some(fid) = folder_id {
                    affected.insert(fid);
                }
            }
            Ok(false) => {}
            Err(e) => tracing::warn!(error = %e, session = %id, "analyze_project: enrich_session failed"),
        }
    }
    tracing::info!("analyze_project: {} — enriched {} sessions", project_id, enriched);
    // L1: derive signals (detected_patterns) for the folders that changed — needs
    // new activity to recompute, and only the affected folders' aggregates moved.
    if enriched > 0 {
        let folders: Vec<uuid::Uuid> = affected.into_iter().collect();
        if let Err(e) = derive_signals(ctx, &project_id, Some(&folders)).await {
            tracing::warn!(error = %e, project = %project_id, "analyze_project: derive_signals failed");
        }
    }
    // L2: generate recommendations + learned memories from whatever patterns
    // exist (#69). Runs every analysis pass, not just when sessions were
    // enriched — it's idempotent (skips patterns that already produced an
    // artifact), so existing patterns still surface after a fresh deploy.
    // Degrades independently — a failure here is logged, not fatal.
    if let Err(e) = super::generate::generate_for_project(ctx, &project_id).await {
        tracing::warn!(error = %e, project = %project_id, "analyze_project: generate failed");
    }
    // L2 consolidation (#70): batch the strongest findings through the
    // `reasoning` chain for one synthesized recommendation + a reasoning trace.
    // Idempotent (signature-guarded) + degrades to no-op without a model.
    if let Err(e) = super::consolidate::consolidate_for_project(ctx, &project_id).await {
        tracing::warn!(error = %e, project = %project_id, "analyze_project: consolidate failed");
    }
    // Model-effectiveness rec (#65): if one model clearly out-performs others on
    // this project's FTR, recommend preferring it. Idempotent; degrades to no-op
    // without enough model-tagged sessions.
    if let Err(e) = super::model_insight::model_insight_for_project(ctx, &project_id).await {
        tracing::warn!(error = %e, project = %project_id, "analyze_project: model_insight failed");
    }
    // Ranking pass (#65 tail): now that generate + consolidate + model_insight
    // have written all recs, score every pending one and mark the focal "do
    // first" pick. Runs last so the whole pending set is ranked together; idempotent.
    if let Err(e) = super::rank::rank_for_project(ctx, &project_id).await {
        tracing::warn!(error = %e, project = %project_id, "analyze_project: rank failed");
    }
    // Analysis completion makes this project's (re)synthesized sessions
    // MEASURABLE — a `session_outcomes` day is only countable once the analyzer has
    // set each session's `outcome` — so analysis drives the per-day metric plan
    // (synthesizer → analyzer → ComputeProjectMetrics), with the daily metrics scheduler
    // as the self-heal backstop. Project id rides in `folder_path` (empty `path`),
    // matching the metrics scheduler's enqueue shape. `enqueue_unique` guards
    // against a plan storm: analyze runs often, so a ComputeProjectMetrics already
    // pending/blocked/running for this project coalesces — the `None` return is
    // intentionally ignored (the in-flight plan already covers this project) and
    // never fails the analyze result.
    let _ = ctx
        .queue
        .enqueue_unique(Task::new(TaskKind::ComputeProjectMetrics, &project_id.to_string(), ""))
        .await;
    Ok(enriched)
}

/// Rework/churn anti-pattern name for a file.
fn churn_pattern_name(file: &str) -> String {
    format!("rework: {file}")
}

/// Churn confidence from the max re-edits in a single session (caps at 10).
fn churn_confidence(max_session_edits: i64) -> f64 {
    (max_session_edits as f64 / 10.0).clamp(0.0, 1.0)
}

/// Truncate a prompt for storage in a pattern instance.
fn prompt_snippet(prompt: &str) -> String {
    let t = prompt.trim();
    if t.chars().count() > PROMPT_SNIPPET_MAX {
        let mut s: String = t.chars().take(PROMPT_SNIPPET_MAX).collect();
        s.push('…');
        s
    } else {
        t.to_string()
    }
}

/// SignalDeriver (L1, #68): derive detected patterns from a project's enriched
/// events. Tool failures aren't captured by the hooks, so the signals are
/// behavioral:
///   - **re-edit churn** — a file re-edited `>= CHURN_MIN_EDITS` times in one
///     session (anti-pattern, file-scoped) — the agent kept returning to it;
///   - **correction-prone** — folders with `>= CORRECTION_MIN` corrective
///     prompts (anti-pattern); and
///   - **rule-candidates** — imperative-principle prompts ("you should always",
///     "make sure", …) that may promote to rules (pattern, non-anti).
///
/// These become F4 (#69) recommendations/teachings. Idempotent (upsert by
/// folder+name). Returns the number of pattern rows written.
///
/// `affected = Some(folders)` re-derives only those folders (the incremental
/// path — patterns are folder aggregates, so untouched folders keep theirs);
/// `None` re-derives the whole project (full / on-demand).
pub async fn derive_signals(ctx: &TaskContext, project_id: &uuid::Uuid, affected: Option<&[uuid::Uuid]>) -> Result<u32, String> {
    let mut count = 0u32;

    // 1. Re-edit churn anti-patterns (file-scoped). Patterns are project-scoped
    //    now (#82): the ON CONFLICT key is (project_id, name, is_anti_pattern),
    //    so if the same churned file surfaces from two folders in this project
    //    the two rows collapse into one. `folder_id` stays as the locus pointer.
    for (folder_id, file, max_edits, total_edits) in
        ctx.pg().get_file_churn_stats(project_id, CHURN_MIN_EDITS, affected).await?
    {
        let instances = serde_json::json!([{
            "file": file, "max_session_edits": max_edits, "total_edits": total_edits
        }]);
        match ctx
            .pg()
            .upsert_pattern(project_id, Some(&folder_id), &churn_pattern_name(&file), true, Some(churn_confidence(max_edits)), &instances)
            .await
        {
            Ok(_) => count += 1,
            Err(e) => tracing::warn!(error = %e, file = %file, "derive_signals: churn upsert failed"),
        }
    }

    // 2. Prompt-derived signals: corrections (anti) + rule candidates (pattern).
    // Cheap regex RECALL → L2 LLM PRECISION (#70): the regex flags candidates,
    // then one model pass refines them (drops false positives, fixes
    // correction↔principle mislabels). Falls back to the regex class when no
    // chat model is configured, so this never blocks the analyzer.
    //
    // Aggregation is at PROJECT scope (#82): one `correction-prone` and one
    // `rule-candidates` pattern per project — the folder locus survives inside
    // each instance blob for later drill-down, but the row itself rolls up.
    let mut candidates: Vec<(uuid::Uuid, String, String, PromptClass)> = Vec::new();
    for (folder_id, session_id, prompt) in ctx.pg().get_project_prompts(project_id, affected).await? {
        let regex_class = if correction_signal(&prompt).is_some() {
            PromptClass::Correction
        } else if principle_signal(&prompt).is_some() {
            PromptClass::Principle
        } else {
            continue;
        };
        candidates.push((folder_id, session_id, prompt, regex_class));
    }
    let texts: Vec<&str> = candidates.iter().map(|(_, _, p, _)| p.as_str()).collect();
    let refined = classify_batch(&ctx.app_state.gateway, &texts).await;

    let mut corrections: Vec<serde_json::Value> = Vec::new();
    let mut principles: Vec<serde_json::Value> = Vec::new();
    for (i, (folder_id, session_id, prompt, regex_class)) in candidates.iter().enumerate() {
        // LLM-refined class when available for this prompt, else the regex class.
        let class = refined.get(i).copied().flatten().unwrap_or(*regex_class);
        let instance = serde_json::json!({
            "folder_id": folder_id,
            "session": session_id,
            "prompt": prompt_snippet(prompt),
        });
        match class {
            PromptClass::Correction => corrections.push(instance),
            PromptClass::Principle => principles.push(instance),
            PromptClass::Neither => {} // LLM rejected the regex candidate — a false positive
        }
    }
    if corrections.len() >= CORRECTION_MIN {
        let instances = serde_json::Value::Array(corrections);
        match ctx.pg().upsert_pattern(project_id, None, "correction-prone", true, None, &instances).await {
            Ok(_) => count += 1,
            Err(e) => tracing::warn!(error = %e, project = %project_id, "derive_signals: correction upsert failed"),
        }
    }
    if !principles.is_empty() {
        let instances = serde_json::Value::Array(principles);
        match ctx.pg().upsert_pattern(project_id, None, "rule-candidates", false, None, &instances).await {
            Ok(_) => count += 1,
            Err(e) => tracing::warn!(error = %e, project = %project_id, "derive_signals: rule-candidate upsert failed"),
        }
    }

    if count > 0 {
        tracing::info!("derive_signals: {} — {} pattern rows", project_id, count);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(event_type: &str, ts: i64) -> HookEvent {
        HookEvent {
            event_type: event_type.into(),
            tool_name: None,
            ts,
            prompt: None,
            file_path: None,
            tool_failed: false,
            action: None,
        }
    }
    /// A `command_invoked` event carrying an action (e.g. a resume marker).
    fn cmd_ev(action: &str, ts: i64) -> HookEvent {
        HookEvent { action: Some(action.into()), ..ev("command_invoked", ts) }
    }
    fn prompt_ev(text: &str, ts: i64) -> HookEvent {
        HookEvent { prompt: Some(text.into()), ..ev("UserPromptSubmit", ts) }
    }
    fn tool_ev(event_type: &str, tool: &str, ts: i64, failed: bool) -> HookEvent {
        HookEvent { tool_name: Some(tool.into()), tool_failed: failed, ..ev(event_type, ts) }
    }
    const MIN: i64 = 60 * 1000;

    #[test]
    fn empty_stream_yields_no_metrics() {
        assert!(derive_session_metrics(&[], &[]).is_none());
    }

    #[test]
    fn clean_session_is_completed_first_try() {
        let events = vec![
            prompt_ev("add a test for the parser", 1000),
            prompt_ev("now wire it into the build", 2000),
            prompt_ev("ship it", 3000),
            ev("Stop", 4000),
        ];
        let m = derive_session_metrics(&events, &[]).unwrap();
        assert_eq!(m.turns.len(), 3);
        assert_eq!(m.corrections, 0);
        assert!(m.ftr);
        assert_eq!(m.outcome, "completed");
        assert_eq!(m.duration_ms, 3000, "all gaps under idle ⇒ full span is active");
    }

    #[test]
    fn correction_prompt_drops_ftr_and_marks_corrected() {
        let events = vec![
            prompt_ev("implement the cache", 1000),
            prompt_ev("actually, revert that — wrong approach", 2000),
            ev("Stop", 3000),
        ];
        let m = derive_session_metrics(&events, &[]).unwrap();
        assert_eq!(m.turns.len(), 2);
        assert_eq!(m.corrections, 1);
        assert!(!m.ftr);
        assert_eq!(m.outcome, "corrected");
        assert!(m.turns[1].is_correction && m.turns[1].triage_signal == Some("revert"));
    }

    #[test]
    fn benign_imperative_prompts_are_not_corrections() {
        for p in ["don't forget the test", "no rush on this", "add error handling"] {
            assert!(correction_signal(p).is_none(), "false positive on: {p}");
        }
        assert_eq!(correction_signal("Actually, that's wrong"), Some("correction"));
        assert_eq!(correction_signal("revert the last change"), Some("revert"));
    }

    #[test]
    fn no_end_no_signal_is_incomplete_not_abandoned() {
        // Regression (transcript-ground-truth): a missing end event is an ABSENCE,
        // not abandonment. With no transcript abandonment signal, real work without
        // a clean end is neutral `incomplete` (crash/window-close) — never
        // `abandoned` (the old classifier's bug that mislabeled ~42% of sessions).
        let events = vec![prompt_ev("start something", 1000), tool_ev("PostToolUse", "Edit", 2000, false)];
        assert_eq!(derive_session_metrics(&events, &[]).unwrap().outcome, "incomplete");
    }

    #[test]
    fn tail_error_cluster_without_end_is_blocked() {
        let events = vec![
            prompt_ev("fix the build", 1000),
            tool_ev("PostToolUse", "Bash", 2000, true),
            tool_ev("PostToolUse", "Bash", 3000, true),
        ];
        assert_eq!(derive_session_metrics(&events, &[]).unwrap().outcome, "blocked");
    }

    /// A minimal transcript turn for the transcript-first derivation tests.
    fn tt(idx: i32, user: &str, assistant: &str) -> TranscriptTurn {
        TranscriptTurn { turn_index: idx, user_text: Some(user.into()), assistant_text: assistant.into(), started_at: None }
    }

    #[test]
    fn zero_turn_session_is_empty() {
        // Only a SessionStart, no prompts, no transcript → nothing was attempted:
        // `empty` (excluded from throughput/ftr by the read path), never `abandoned`.
        let events = vec![ev("SessionStart", 1000)];
        assert_eq!(derive_session_metrics(&events, &[]).unwrap().outcome, "empty");
    }

    #[test]
    fn transcript_turn_count_overrides_sparse_hooks() {
        // The `792d7ce4` case: hooks captured 1 prompt, the transcript has 94 real
        // turns. turn_count comes from the transcript, and a missing end with no
        // abandonment signal is `incomplete`, not `abandoned`.
        let events = vec![prompt_ev("scan is a read operation", 1000)];
        let transcript: Vec<TranscriptTurn> = (0..94).map(|i| tt(i, "keep going", "done")).collect();
        let m = derive_session_metrics(&events, &transcript).unwrap();
        assert_eq!(m.turn_count, 94, "turn count is the transcript's, not the sparse hooks'");
        assert_eq!(m.outcome, "incomplete");
    }

    #[test]
    fn abandoned_only_on_positive_transcript_signal() {
        let events = vec![prompt_ev("try the migration", 1000)];
        let quit = vec![tt(0, "let's abandon this approach", "ok")];
        assert_eq!(derive_session_metrics(&events, &quit).unwrap().outcome, "abandoned", "explicit user give-up");
        let claude = vec![tt(0, "continue", "we're running low on context, start a new session")];
        assert_eq!(derive_session_metrics(&events, &claude).unwrap().outcome, "abandoned", "Claude advised a fresh session");
        let ordinary = vec![tt(0, "add the endpoint", "added")];
        assert_eq!(derive_session_metrics(&events, &ordinary).unwrap().outcome, "incomplete", "no cue ⇒ not abandoned");
    }

    #[test]
    fn resumed_session_is_never_abandoned() {
        // Phase B: a session with an in-session `command_invoked{action:resume}`
        // marker was reopened/continued — even with an explicit abandonment cue it
        // is NOT abandoned, and `resumed` is flagged for the read path.
        let events = vec![prompt_ev("keep going", 1000), cmd_ev("resume", 1500)];
        let quit = vec![tt(0, "let's abandon this approach", "ok")];
        let m = derive_session_metrics(&events, &quit).unwrap();
        assert!(m.resumed, "the resume marker sets resumed");
        assert_ne!(m.outcome, "abandoned", "a resumed session is continued, not abandoned");
    }

    #[test]
    fn evidence_quotes_real_transcript_turns_no_confabulation() {
        // Phase C: evidence is the REAL transcript turns — opening ask, the
        // correction (FTR detractor), and the closing result — never an invented
        // causal "why". Empty transcript → null (honest-empty).
        assert!(build_session_evidence(&[], "claude").is_null(), "no transcript ⇒ no evidence");
        let transcript = vec![
            tt(0, "implement the cache layer", "on it"),
            tt(1, "no, that's wrong — revert it", "reverted"),
            tt(2, "thanks", "**Done** — cache added.\n```rust\nlet x = 1;\n```\nShipped it."),
        ];
        let ev = build_session_evidence(&transcript, "claude");
        let moments = ev["moments"].as_array().expect("moments present");
        assert_eq!(ev["source"], "transcript");
        assert_eq!(moments[0]["who"], "you");
        assert_eq!(moments[0]["turn"], 0, "opening ask is the first user turn");
        let corr = moments.iter().find(|m| m["kind"] == "correction").expect("correction moment present");
        assert_eq!(corr["turn"], 1, "the correction quotes the real detractor turn");
        assert!(corr["text"].as_str().unwrap().contains("revert"), "quotes the actual user text");
        // The assistant side is labelled with the ACP (claude), never "sensei",
        // and its text is cleaned (code fences + markdown emphasis stripped).
        let closing = moments.iter().find(|m| m["who"] == "claude")
            .expect("closing result is labelled with the ACP, not 'sensei'");
        let text = closing["text"].as_str().unwrap();
        assert!(text.contains("Done") && text.contains("Shipped"), "keeps the real words");
        assert!(!text.contains("```") && !text.contains("**"), "strips code fences + markdown");
    }

    #[test]
    fn trouble_case_correlates_context_pressure() {
        // Phase D: a Claude trouble hint is collected WITH its PreCompact/turns/
        // duration correlation; a clean session yields null (no case).
        let events = vec![prompt_ev("do the migration", 0), ev("PreCompact", 1000), ev("PreCompact", 2000)];
        let transcript = vec![tt(0, "do the migration", "we're running low on context, let's start a new session")];
        let trouble = detect_session_trouble(&events, &transcript, 12, 5000);
        assert_eq!(trouble["hint"], "context-pressure", "the earliest matching category wins");
        assert_eq!(trouble["precompact"], 2, "correlates the PreCompact count");
        assert_eq!(trouble["turns"], 12);
        assert_eq!(trouble["duration_ms"], 5000);
        let clean = detect_session_trouble(&[], &[tt(0, "add a test", "done")], 3, 100);
        assert!(clean.is_null(), "a clean session is not a trouble case");
    }

    #[test]
    fn corrections_come_from_transcript_user_text() {
        let events = vec![prompt_ev("noise", 1000)]; // the hook prompt is not a correction
        let transcript = vec![
            tt(0, "implement the parser", "done"),
            tt(1, "actually, that's wrong — revert it", "reverted"),
        ];
        let m = derive_session_metrics(&events, &transcript).unwrap();
        assert_eq!(m.corrections, 1, "correction is detected in transcript user_text, not the hooks");
        assert!(!m.ftr);
    }

    #[test]
    fn idle_gap_splits_segments_and_excludes_idle_from_duration() {
        // Two prompts close together, then a 2-hour break, then a third prompt.
        let events = vec![
            prompt_ev("a", 0),
            tool_ev("PostToolUse", "Edit", MIN, false),
            prompt_ev("b", 2 * MIN),
            tool_ev("PostToolUse", "Edit", 3 * MIN, false),
            // ── 2-hour idle gap (came back later) ──
            prompt_ev("c", 123 * MIN),
            ev("Stop", 124 * MIN),
        ];
        let m = derive_session_metrics(&events, &[]).unwrap();
        assert_eq!(m.turns.len(), 3);
        assert_eq!(
            m.turns.iter().map(|t| t.segment).collect::<Vec<_>>(),
            vec![1, 1, 2],
            "the 2h gap before turn 3 starts a new segment"
        );
        // active = 1m + 1m + 1m (within/between the first two turns) + 1m (turn 3) = 4 min;
        // the ~2h idle gap is excluded.
        assert_eq!(m.duration_ms, 4 * MIN, "idle gap excluded from active duration");
    }

    #[test]
    fn tool_usage_tallies_pre_post_and_failures() {
        let events = vec![
            tool_ev("PreToolUse", "Edit", 1000, false),
            tool_ev("PostToolUse", "Edit", 1100, false),
            tool_ev("PreToolUse", "Bash", 2000, false),
            tool_ev("PostToolUse", "Bash", 2100, true),
            ev("Stop", 3000),
        ];
        let m = derive_session_metrics(&events, &[]).unwrap();
        assert_eq!(m.tool_usage["Edit"], serde_json::json!({ "pre": 1, "post": 1, "failed": 0 }));
        assert_eq!(m.tool_usage["Bash"], serde_json::json!({ "pre": 1, "post": 1, "failed": 1 }));
    }

    #[test]
    fn dominant_module_is_most_touched_dir() {
        let events = vec![
            HookEvent { file_path: Some("src/api/handlers/x.rs".into()), ..tool_ev("PostToolUse", "Edit", 1000, false) },
            HookEvent { file_path: Some("src/api/handlers/y.rs".into()), ..tool_ev("PostToolUse", "Edit", 1100, false) },
            HookEvent { file_path: Some("README.md".into()), ..tool_ev("PostToolUse", "Edit", 1200, false) },
            ev("Stop", 2000),
        ];
        assert_eq!(derive_session_metrics(&events, &[]).unwrap().module.as_deref(), Some("src/api/handlers"));
    }

    #[test]
    fn hook_event_from_row_extracts_payload_fields() {
        let row = serde_json::json!({
            "event_type": "PostToolUse", "tool_name": "Edit", "ts": 42,
            "payload": { "tool_input": { "file_path": "src/x.rs" },
                         "tool_response": { "is_error": true } }
        });
        let e = hook_event_from_row(&row);
        assert_eq!(e.event_type, "PostToolUse");
        assert_eq!(e.file_path.as_deref(), Some("src/x.rs"));
        assert!(e.tool_failed);
    }

    #[test]
    fn churn_pattern_name_prefixes_file() {
        assert_eq!(churn_pattern_name("src/x.rs"), "rework: src/x.rs");
    }

    #[test]
    fn churn_confidence_scales_with_edits() {
        assert_eq!(churn_confidence(5), 0.5);
        assert_eq!(churn_confidence(10), 1.0);
        assert_eq!(churn_confidence(25), 1.0, "clamped to 1.0");
        assert_eq!(churn_confidence(0), 0.0);
    }

    #[test]
    fn principle_signal_flags_imperative_rules_only() {
        assert_eq!(principle_signal("you should always run the tests first"), Some("you should always"));
        assert_eq!(principle_signal("make sure to use vite snapshots"), Some("make sure to"));
        assert_eq!(principle_signal("from now on, branch off develop"), Some("from now on"));
        // normal feature requests are not principles
        assert!(principle_signal("add a login page").is_none());
        assert!(principle_signal("the colors don't match").is_none());
    }

    #[test]
    fn prompt_snippet_truncates_long_prompts() {
        let long = "x".repeat(PROMPT_SNIPPET_MAX + 50);
        let s = prompt_snippet(&long);
        assert_eq!(s.chars().count(), PROMPT_SNIPPET_MAX + 1, "PROMPT_SNIPPET_MAX chars + ellipsis");
        assert!(s.ends_with('…'));
        assert_eq!(prompt_snippet("  short  "), "short", "trims, no ellipsis when under cap");
    }

    // ── DB-backed orchestrator test ──────────────────────────────────────
    
    
    use crate::tasks::TaskKind;
    

    use crate::tasks::test_support::make_ctx;

    #[tokio::test]
    async fn analyze_project_enriches_sessions_and_writes_turns() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let pid = pg.create_project(&format!("_test:analyze-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/ana-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let fid = pg.upsert_repo(&root, "ana-repo", &format!("/_test/ana-{}", uuid::Uuid::new_v4())).await.unwrap();
        let csid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let sid = pg.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

        // 2 prompts (one a correction) + an edit + a Stop.
        let prompt = |t: &str| serde_json::json!({ "prompt": t });
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1000, None, &prompt("build the thing")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 2000, None, &prompt("actually, revert that")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "PostToolUse", Some("Edit"), None, 2500, None, &serde_json::json!({})).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "Stop", None, None, 3000, None, &serde_json::json!({})).await.unwrap();

        let task = Task::new(TaskKind::AnalyzeProject, "", &pid.to_string());
        assert_eq!(analyze_project(&ctx, &task).await.unwrap(), 1, "one session enriched");

        // session aggregates — duration is now an interval; read back as ms.
        let row: (i32, i32, Option<bool>, Option<String>, Option<f64>) = sqlx_core::query_as::query_as(
            "SELECT turns, corrections, ftr, outcome::text,
                    (extract(epoch from duration)*1000)::float8 AS duration_ms
             FROM activity.sessions WHERE id = $1"
        ).bind(sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(row.0, 2, "two turns");
        assert_eq!(row.1, 1, "the 'revert' prompt is a correction");
        assert_eq!(row.2, Some(false));
        assert_eq!(row.3.as_deref(), Some("corrected"));
        assert_eq!(row.4, Some(2000.0), "gap-aware active duration");

        // retrospective summary persisted by enrichment. The test gateway has no
        // insight-copy chain, so `generate_and_cache` misses → the deterministic
        // fallback is written (non-empty, names the outcome).
        let summary: (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT summary FROM activity.sessions WHERE id = $1"
        ).bind(sid).fetch_one(pg.pool()).await.unwrap();
        let summary = summary.0.expect("summary written");
        assert!(!summary.trim().is_empty(), "session summary is non-empty: {summary:?}");
        assert!(summary.contains("outcome corrected"), "fallback names the outcome: {summary:?}");

        // turn rows written, ordered, segmented.
        let turns: Vec<(i32, i32, bool)> = sqlx_core::query_as::query_as(
            "SELECT turn_number, segment, is_correction FROM activity.turns WHERE session_id = $1 ORDER BY turn_number"
        ).bind(sid).fetch_all(pg.pool()).await.unwrap();
        assert_eq!(turns, vec![(1, 1, false), (2, 1, true)]);

        // incremental — no assistant_events newer than analyzed_at ⇒ the
        // session is skipped on the next run (cost scales with new activity).
        assert_eq!(analyze_project(&ctx, &task).await.unwrap(), 0, "unchanged session is skipped");
        let n: (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM activity.turns WHERE session_id = $1")
            .bind(sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(n.0, 2, "turns from the first enrichment remain (no dupes)");

        // cleanup (turns cascade on session delete)
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = $1").bind(&csid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1").bind(sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(pool).await.ok();
    }

    #[tokio::test]
    async fn derive_signals_flags_churn_corrections_and_principles() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let pid = pg.create_project(&format!("_test:sig-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/sig-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let fid = pg.upsert_repo(&root, "sig-repo", &format!("/_test/sig-{}", uuid::Uuid::new_v4())).await.unwrap();
        let csid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let sid = pg.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

        let edit = |fp: &str| serde_json::json!({ "tool_input": {"file_path": fp} });
        let prompt = |t: &str| serde_json::json!({ "prompt": t });
        // hot.rs: 5 edits in one session ⇒ churn anti-pattern (5 >= CHURN_MIN_EDITS).
        for ts in [1100, 1200, 1300, 1400, 1500] {
            pg.insert_hook_event(&csid, "claude", "PostToolUse", Some("Edit"), None, ts, None, &edit("src/hot.rs")).await.unwrap();
        }
        // cold.rs: only 2 edits ⇒ below threshold, not flagged.
        pg.insert_hook_event(&csid, "claude", "PostToolUse", Some("Edit"), None, 1600, None, &edit("src/cold.rs")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "PostToolUse", Some("Edit"), None, 1700, None, &edit("src/cold.rs")).await.unwrap();
        // prompts: 2 corrections (⇒ correction-prone) + 1 principle (⇒ rule-candidates) + 1 neutral.
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1000, None, &prompt("fix hot.rs")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1050, None, &prompt("revert that change")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1075, None, &prompt("that's not right, try again")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1090, None, &prompt("you should always run the tests first")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "Stop", None, None, 2000, None, &serde_json::json!({})).await.unwrap();

        // full wiring: analyze_project enriches (enriched>0) then derives signals.
        let task = Task::new(TaskKind::AnalyzeProject, "", &pid.to_string());
        assert_eq!(analyze_project(&ctx, &task).await.unwrap(), 1, "one session enriched");

        let pats: Vec<(String, bool, Option<f64>, i32)> = sqlx_core::query_as::query_as(
            "SELECT name, is_anti_pattern, confidence::float8, instance_count FROM inference.detected_patterns WHERE project_id = $1 ORDER BY name"
        ).bind(pid).fetch_all(pg.pool()).await.unwrap();
        assert_eq!(pats.len(), 3, "churn + correction-prone + rule-candidates (cold.rs below churn threshold)");
        assert_eq!(pats[0].0, "correction-prone");
        assert!(pats[0].1, "correction-prone is an anti-pattern");
        assert_eq!(pats[0].3, 2, "two corrective prompts");
        assert_eq!(pats[1].0, "rework: src/hot.rs");
        assert!(pats[1].1, "churn is an anti-pattern");
        assert_eq!(pats[1].2, Some(0.5), "5 edits / 10");
        assert_eq!(pats[2].0, "rule-candidates");
        assert!(!pats[2].1, "rule candidate is a pattern, not anti");
        assert_eq!(pats[2].3, 1, "one principle prompt");

        // idempotent: deriving again (whole-project path) upserts the same 3 rows.
        assert_eq!(derive_signals(&ctx, &pid, None).await.unwrap(), 3);
        let n: (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM inference.detected_patterns WHERE project_id = $1")
            .bind(pid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(n.0, 3, "upsert, not insert");

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE project_id = $1").bind(pid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = $1").bind(&csid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1").bind(sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(pool).await.ok();
    }

    #[tokio::test]
    async fn analyze_project_enqueues_one_plan_metric_days_and_guards_storms() {
        // Analysis completion is what makes a project's (re)synthesized sessions
        // MEASURABLE — `session_outcomes` days become countable only once the
        // analyzer sets each session's `outcome` — so a successful analyze pass
        // drives the per-day metric plan (synthesizer → analyzer → ComputeProjectMetrics).
        // Because analyze runs often, the enqueue is `enqueue_unique`-guarded: a
        // second pass while the first ComputeProjectMetrics is still in flight must NOT
        // stack a duplicate (the plan-storm guard).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let pid = pg.create_project(&format!("_test:analyze-plan-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/ana-plan-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let fid = pg.upsert_repo(&root, "ana-plan-repo", &format!("/_test/ana-plan-{}", uuid::Uuid::new_v4())).await.unwrap();
        let csid = format!("_test-sid-{}", uuid::Uuid::new_v4());
        let sid = pg.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

        // A minimal enrichable session (a prompt + a Stop) so analyze_project
        // reaches its success path.
        let prompt = |t: &str| serde_json::json!({ "prompt": t });
        pg.insert_hook_event(&csid, "claude", "UserPromptSubmit", None, None, 1000, None, &prompt("build the thing")).await.unwrap();
        pg.insert_hook_event(&csid, "claude", "Stop", None, None, 2000, None, &serde_json::json!({})).await.unwrap();

        // Count ComputeProjectMetrics enqueued for THIS project (id in folder_path, empty path).
        let owner = pid.to_string();
        let count_plans = |snap: &[(TaskKind, String, String)]| -> usize {
            snap.iter()
                .filter(|(k, f, p)| *k == TaskKind::ComputeProjectMetrics && *f == owner && p.is_empty())
                .count()
        };

        let task = Task::new(TaskKind::AnalyzeProject, "", &pid.to_string());
        analyze_project(&ctx, &task).await.unwrap();
        assert_eq!(
            count_plans(&ctx.queue.snapshot().await), 1,
            "analysis completion enqueues exactly one ComputeProjectMetrics for the project",
        );

        // A second pass while the first ComputeProjectMetrics is still pending coalesces —
        // enqueue_unique dedups on (kind, folder_path, path), so no second stacks.
        analyze_project(&ctx, &task).await.unwrap();
        assert_eq!(
            count_plans(&ctx.queue.snapshot().await), 1,
            "a re-run while the plan is still in flight does not stack a duplicate (enqueue_unique)",
        );

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = $1").bind(&csid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1").bind(sid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(pool).await.ok();
    }
}
