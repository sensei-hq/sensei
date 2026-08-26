//! The retrospective itself.
//!
//! Shape: what happened → what's working → where it snagged → what to try. Every
//! claim carries a reference (session id + timestamp, sometimes an event id) so
//! it can be checked in the source before it is shown to anyone.
//!
//! Findings are only emitted when the evidence clears a threshold. A section
//! with nothing to say prints nothing — padding a retrospective with
//! "no issues found" trains people to skim it.

use crate::metrics::{Analysis, percentile};
use crate::model::Session;
use chrono::{TimeZone, Utc};
use std::fmt::Write;

fn day(ms: i64) -> String {
    Utc.timestamp_millis_opt(ms).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}
fn stamp(ms: i64) -> String {
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_default()
}
fn dur(ms: i64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}h {}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m {}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}
fn n(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

pub fn report(label: &str, sessions: &[Session], a: &Analysis) -> String {
    let mut o = String::new();

    let _ = writeln!(o, "# Working session retrospective — {label}\n");
    let _ = writeln!(
        o,
        "Drawn from {} GitHub Copilot CLI session{} between {} and {}, covering {} \
         event{}. Everything below comes from the transcripts themselves; nothing is \
         estimated or inferred from outside them.\n",
        a.sessions,
        if a.sessions == 1 { "" } else { "s" },
        day(a.first_ms),
        day(a.last_ms),
        n(a.events as i64),
        if a.events == 1 { "" } else { "s" }
    );

    coverage(&mut o, a);
    at_a_glance(&mut o, a);
    working_well(&mut o, a);
    friction(&mut o, a, sessions);
    try_next(&mut o, a);
    per_session(&mut o, sessions);
    method(&mut o, a);
    o
}

/// State the shape of the evidence before any conclusion drawn from it.
fn coverage(o: &mut String, a: &Analysis) {
    let mut caveats: Vec<String> = Vec::new();
    if a.unclosed > 0 {
        caveats.push(format!(
            "{} session{} ended without a shutdown record (still holding a lock file), so \
             {} no token or code-change totals",
            a.unclosed,
            if a.unclosed == 1 { "" } else { "s" },
            if a.unclosed == 1 { "it has" } else { "they have" }
        ));
    }
    if a.sessions_with_totals < a.sessions {
        caveats.push(format!(
            "cost and code-change figures cover {} of {} sessions",
            a.sessions_with_totals, a.sessions
        ));
    }
    if a.skipped_lines > 0 {
        caveats.push(format!("{} unreadable line(s) were skipped", a.skipped_lines));
    }
    if a.tool_unreported > 0 {
        caveats.push(format!(
            "{} tool call{} never reported a result and {} counted as neither success nor failure",
            a.tool_unreported,
            if a.tool_unreported == 1 { "" } else { "s" },
            if a.tool_unreported == 1 { "is" } else { "are" }
        ));
    }
    if caveats.is_empty() {
        return;
    }
    let _ = writeln!(o, "> **What this covers.** {}.\n", caveats.join("; "));
}

fn at_a_glance(o: &mut String, a: &Analysis) {
    let _ = writeln!(o, "## At a glance\n");
    let _ = writeln!(o, "| | |");
    let _ = writeln!(o, "|---|---|");
    let _ = writeln!(o, "| Sessions | {} across {} active day(s) |", a.sessions, a.active_days);
    let _ = writeln!(
        o,
        "| Time actively working | {} (idle gaps over 10 min excluded) |",
        dur(a.active_ms)
    );
    let _ = writeln!(o, "| Prompts written | {} |", a.prompts);
    if let Some(t) = a.turns_per_prompt() {
        let _ = writeln!(o, "| Assistant turns | {} ({t:.1} per prompt) |", a.turns);
    }
    if let Some(t) = a.tools_per_prompt() {
        let _ = writeln!(o, "| Tool calls | {} ({t:.0} per prompt) |", n(a.tool_calls as i64));
    }
    if a.sessions_with_totals > 0 {
        let _ = writeln!(
            o,
            "| Code changed | +{} / −{} lines across {} file touches |",
            n(a.lines_added),
            n(a.lines_removed),
            n(a.files_modified as i64)
        );
        let _ = writeln!(o, "| Premium requests | {} |", n(a.premium_requests));
    }
    if let Some(p) = percentile(&a.turn_ms_sorted, 50.0) {
        let p90 = percentile(&a.turn_ms_sorted, 90.0).unwrap_or(p);
        let _ = writeln!(o, "| Turn length | {} typical, {} at the 90th percentile |", dur(p), dur(p90));
    }
    let _ = writeln!(o);
}

fn working_well(o: &mut String, a: &Analysis) {
    let mut items: Vec<String> = Vec::new();

    if let Some(c) = a.cache_reuse_pct()
        && c >= 60.0
    {
        items.push(format!(
            "**Context is being reused, not resent.** {c:.0}% of input tokens were served \
             from cache ({} cached vs {} fresh). That is what long, continuous sessions \
             look like when they go well — the model keeps its place instead of \
             re-reading the codebase on every turn.",
            n(a.cache_read_tokens),
            n(a.input_tokens)
        ));
    }

    if let Some(f) = a.tool_failure_pct()
        && f < 5.0
        && a.tool_calls > 100
    {
        items.push(format!(
            "**Tool use is landing.** {:.1}% of {} tool calls failed. Most of the loop is \
             productive rather than spent recovering.",
            f,
            n(a.tool_calls as i64)
        ));
    }

    if let Some(t) = a.tools_per_prompt()
        && t >= 10.0
    {
        items.push(format!(
            "**Prompts are carrying real weight.** Each prompt drove {t:.0} tool calls on \
             average — these are substantial pieces of work handed over whole, not \
             step-by-step instruction."
        ));
    }

    if a.lines_added > 0 && a.sessions_with_totals > 0 {
        let ratio = a.lines_added as f64 / (a.lines_removed.max(1)) as f64;
        if ratio > 3.0 {
            items.push(format!(
                "**Mostly additive work.** +{} against −{} lines — new capability rather \
                 than churn over the same code.",
                n(a.lines_added),
                n(a.lines_removed)
            ));
        }
    }

    if items.is_empty() {
        return;
    }
    let _ = writeln!(o, "## What's working\n");
    for i in items {
        let _ = writeln!(o, "- {i}\n");
    }
}

fn friction(o: &mut String, a: &Analysis, sessions: &[Session]) {
    let mut items: Vec<String> = Vec::new();

    // Repeated failure of one tool, back to back — the shape of a stuck loop.
    for run in a.failure_runs.iter().take(3) {
        let s = sessions.iter().find(|s| s.id == run.session);
        let short = s.map(|s| s.short_id().to_string()).unwrap_or_default();
        items.push(format!(
            "**`{}` failed {} times in a row.** A single failure is noise; a run this long \
             means the agent kept trying the same thing. Worth a look at what it was \
             reaching for — a path that does not exist, or a command the environment \
             does not have, will not fix itself on retry.\n\n  \
             *Reference: session `{}`, {} (event `{}`)*",
            run.tool,
            run.length,
            short,
            stamp(run.at_ms),
            run.event_id.get(..8).unwrap_or(&run.event_id)
        ));
    }

    // Tools that fail disproportionately.
    let mut worst: Vec<_> = a
        .tools
        .iter()
        .filter(|t| t.calls >= 20 && t.failures > 0)
        .filter(|t| t.failure_pct().unwrap_or(0.0) >= 10.0)
        .collect();
    worst.sort_by(|x, y| y.failure_pct().partial_cmp(&x.failure_pct()).unwrap());
    for t in worst.iter().take(3) {
        items.push(format!(
            "**`{}` fails {:.0}% of the time** ({} of {} calls). Against an overall rate of \
             {:.1}%, this one tool is doing most of the recovering.",
            t.name,
            t.failure_pct().unwrap_or(0.0),
            t.failures,
            t.calls,
            a.tool_failure_pct().unwrap_or(0.0)
        ));
    }

    // Long tail on turn duration — where the waiting actually happens.
    if let (Some(p50), Some(p95)) =
        (percentile(&a.turn_ms_sorted, 50.0), percentile(&a.turn_ms_sorted, 95.0))
        && p50 > 0
        && p95 > p50 * 6
    {
        items.push(format!(
            "**The slow turns are much slower than typical.** Half of turns finish inside \
             {}, but the slowest 5% take over {} — a {}× spread. The average hides this; \
             the long ones are where the waiting is.",
            dur(p50),
            dur(p95),
            p95 / p50.max(1)
        ));
    }

    // Approval interruptions.
    if a.permission_events >= 10 {
        let per = a.permission_events as f64 / a.sessions as f64;
        items.push(format!(
            "**{} permission prompts** ({per:.0} per session). Each one stops the agent \
             until you answer. If the same kinds of command keep asking, granting them \
             once at the session level removes a recurring pause.",
            a.permission_events
        ));
    }

    if items.is_empty() {
        return;
    }
    let _ = writeln!(o, "## Where it snagged\n");
    for i in items {
        let _ = writeln!(o, "- {i}\n");
    }
}

fn try_next(o: &mut String, a: &Analysis) {
    let mut items: Vec<String> = Vec::new();

    if !a.failure_runs.is_empty() {
        items.push(
            "**When a tool fails twice on the same thing, step in.** The transcripts show \
             runs of the same call failing repeatedly. Retrying rarely resolves a wrong \
             path or a missing binary — a one-line correction from you is faster than \
             several more attempts."
                .to_string(),
        );
    }

    if let Some(share) = a.api_share_pct()
        && share < 40.0
        && a.api_duration_ms > 0
    {
        items.push(format!(
            "**Most of the session is not model time.** {share:.0}% of active time was \
             spent waiting on the model ({} of {}). The rest is reading, deciding and \
             replying — which is where a sharper first prompt pays back more than a \
             faster model.",
            dur(a.api_duration_ms),
            dur(a.active_ms)
        ));
    }

    if let Some(t) = a.tools_per_prompt()
        && t >= 25.0
    {
        items.push(format!(
            "**Consider checkpointing inside long runs.** At {t:.0} tool calls per prompt, \
             a lot happens between your inputs. A brief 'show me the plan before you \
             edit' costs one turn and makes a wrong direction cheap to correct."
        ));
    }

    if a.unclosed > 0 {
        items.push(format!(
            "**{} session{} never closed cleanly.** Those lose their token and \
             code-change totals, so any cost review will under-count. Exiting the CLI \
             rather than closing the terminal keeps the record complete.",
            a.unclosed,
            if a.unclosed == 1 { "" } else { "s" }
        ));
    }

    if items.is_empty() {
        return;
    }
    let _ = writeln!(o, "## Worth trying\n");
    for i in items {
        let _ = writeln!(o, "- {i}\n");
    }
}

fn per_session(o: &mut String, sessions: &[Session]) {
    let _ = writeln!(o, "## Session by session\n");
    let _ = writeln!(o, "| Session | Date | Active | Prompts | Tools | Failed | +/− lines |");
    let _ = writeln!(o, "|---|---|---:|---:|---:|---:|---|");
    for s in sessions {
        let failed = s.tools.iter().filter(|t| t.failed()).count();
        let lines = match (s.totals.lines_added, s.totals.lines_removed) {
            (Some(a), Some(r)) => format!("+{} / −{}", n(a), n(r)),
            _ => "—".into(),
        };
        let _ = writeln!(
            o,
            "| `{}`{} | {} | {} | {} | {} | {} | {} |",
            s.short_id(),
            if s.unclosed { " ⚠" } else { "" },
            day(s.first_ms),
            dur(s.active_ms()),
            s.prompts.len(),
            s.tools.len(),
            failed,
            lines
        );
    }
    let _ = writeln!(o, "\n`⚠` = ended without a shutdown record, so its totals are missing.\n");
}

fn method(o: &mut String, a: &Analysis) {
    let _ = writeln!(o, "## How to check this\n");
    let _ = writeln!(
        o,
        "Every figure comes from `events.jsonl` in the session folders. The signals used:\n"
    );
    let _ = writeln!(o, "| Reported as | Read from |");
    let _ = writeln!(o, "|---|---|");
    let _ = writeln!(o, "| Prompts | `user.message` events |");
    let _ = writeln!(o, "| Assistant turns, turn length | `assistant.turn_start` → `assistant.turn_end`, paired by turn id |");
    let _ = writeln!(o, "| Tool calls, failures | `tool.execution_start` → `tool.execution_complete`, paired by tool-call id; failure is `success: false` |");
    let _ = writeln!(o, "| Code changed, tokens, premium requests | `session.shutdown` totals |");
    let _ = writeln!(o, "| Permission prompts | `session.permissions_changed` events |");
    let _ = writeln!(
        o,
        "\nPercentiles are nearest-rank over observed values, so every figure shown was \
         actually measured. Sessions without a shutdown record contribute to counts and \
         timings but not to totals — {} of {} here.\n",
        a.sessions_with_totals, a.sessions
    );
}
