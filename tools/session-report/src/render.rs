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
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
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
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

pub fn report(
    label: &str,
    tool: Option<crate::Tool>,
    sessions: &[Session],
    a: &Analysis,
) -> String {
    let mut o = String::new();

    let _ = writeln!(o, "# Working session retrospective — {label}\n");
    let _ = writeln!(
        o,
        "Drawn from {} {} session{} between {} and {}, covering {} event{}. Everything \
         below comes from the transcripts themselves; nothing is estimated or inferred \
         from outside them.\n",
        a.sessions,
        tool.map(crate::Tool::label).unwrap_or("coding assistant"),
        if a.sessions == 1 { "" } else { "s" },
        day(a.first_ms),
        day(a.last_ms),
        n(a.events as i64),
        if a.events == 1 { "" } else { "s" }
    );

    coverage(&mut o, a);
    at_a_glance(&mut o, a);
    cost(&mut o, a);
    working_well(&mut o, a);
    friction(&mut o, a, sessions);
    try_next(&mut o, a);
    delegation(&mut o, a);
    per_session(&mut o, sessions);
    method(&mut o, tool, sessions, a);
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
    if a.projects > 0 {
        let _ = writeln!(o, "| Projects worked in | {} |", a.projects);
    }
    if a.tool_calls > 0 && a.tool_outcomes_known == 0 {
        let _ = writeln!(o, "| Tool outcomes | not recorded by this transcript format |");
    }
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
    // Only when the transcript actually times turns. VS Code writes
    // `responseTimestamp` equal to `timestamp` on these records, so every turn
    // measures zero — printing "0s typical" would read as instant replies rather
    // than as a field the format does not populate.
    if let Some(p) = percentile(&a.turn_ms_sorted, 50.0) {
        let p90 = percentile(&a.turn_ms_sorted, 90.0).unwrap_or(p);
        if p90 > 0 {
            let _ = writeln!(
                o,
                "| Turn length | {} typical, {} at the 90th percentile |",
                dur(p),
                dur(p90)
            );
        }
    }
    let _ = writeln!(o);
}

/// What the sessions drew against the Copilot plan, and which models did it.
///
/// A "premium request" is GitHub Copilot's billing unit: plans carry a monthly
/// allowance, and only premium models draw from it — most models report zero.
/// So the interesting number is not how many requests were made but how few of
/// them were chargeable.
fn cost(o: &mut String, a: &Analysis) {
    if a.by_model.is_empty() {
        return;
    }
    let _ = writeln!(o, "## Cost and model mix\n");

    let total_req: i64 = a.by_model.values().map(|m| m.requests).sum();
    let _ = writeln!(
        o,
        "{} model request(s) across {} model(s), of which **{} were premium** — GitHub Copilot's billable unit, drawn from the monthly plan allowance. Everything else was included at no premium cost.\n",
        n(total_req),
        a.by_model.len(),
        n(a.premium_requests)
    );

    let _ = writeln!(o, "| Model | Requests | Premium | Share of premium | Output tokens |");
    let _ = writeln!(o, "|---|---:|---:|---:|---:|");
    let mut rows: Vec<_> = a.by_model.iter().collect();
    rows.sort_by_key(|(_, m)| std::cmp::Reverse(m.requests));
    for (name, m) in rows {
        let share = if a.premium_requests > 0 {
            format!("{:.0}%", 100.0 * m.premium as f64 / a.premium_requests as f64)
        } else {
            "—".into()
        };
        let _ = writeln!(
            o,
            "| `{}` | {} | {} | {} | {} |",
            name,
            n(m.requests),
            n(m.premium),
            share,
            n(m.output_tokens)
        );
    }

    if let Some(c) = a.cache_reuse_pct() {
        let _ = writeln!(
            o,
            "\nToken flow: {} input, {} output, {} read back from cache ({c:.0}% reuse), {} reasoning.",
            n(a.input_tokens),
            n(a.output_tokens),
            n(a.cache_read_tokens),
            n(a.reasoning_tokens)
        );
    }

    // Name the premium concentration when one model dominates — that is the
    // lever, and it is usually invisible.
    if let Some((name, m)) = a.by_model.iter().max_by_key(|(_, m)| m.premium)
        && m.premium > 0
        && a.premium_requests > 0
    {
        let share = 100.0 * m.premium as f64 / a.premium_requests as f64;
        if share >= 60.0 {
            let _ = writeln!(
                o,
                "\n**`{}` accounts for {share:.0}% of premium usage.** It is {:.0}% of requests overall, so the plan cost is concentrated in a minority of the work. Worth knowing which tasks genuinely need it — the other models here cost nothing against the allowance.",
                name,
                100.0 * m.requests as f64 / total_req.max(1) as f64
            );
        }
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
        && a.tool_outcomes_known > 100
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
    //
    // Stated ONCE, with the runs listed under it. Repeating the explanation on
    // every bullet was three paragraphs saying the same thing, which is how a
    // reader learns to skip the section.
    if !a.failure_runs.is_empty() {
        let mut lines = String::new();
        for run in a.failure_runs.iter().take(3) {
            let short = sessions
                .iter()
                .find(|s| s.id == run.session)
                .map(|s| s.short_id().to_string())
                .unwrap_or_default();
            let _ = write!(
                lines,
                "\n  - `{}` × {} — session `{}`, {} (event `{}`)",
                run.tool,
                run.length,
                short,
                stamp(run.at_ms),
                run.event_id.get(..8).unwrap_or(&run.event_id)
            );
        }
        items.push(format!(
            "**The same tool failed several times in a row.** Retrying rarely resolves a \
             wrong path or a missing binary, so these runs are usually the agent stuck \
             rather than the work being hard:{lines}"
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

/// Sub-agent activity, and what it runs on.
///
/// Split from the main thread deliberately: the useful question is whether
/// delegated work goes to a cheaper model than the thread doing the deciding.
/// Merged into one mix, that is unanswerable.
fn delegation(o: &mut String, a: &Analysis) {
    if a.delegated == 0 {
        return;
    }
    let _ = writeln!(o, "## Delegated work\n");
    let _ = writeln!(
        o,
        "{} sub-agent transcript(s), folded into the sessions that spawned them. A delegated agent runs inside its parent session but writes its own file, so the parent records only the hand-off — counting sessions alone would miss most of the activity here.\n",
        a.delegated
    );

    if a.delegated_models.is_empty() {
        return;
    }
    let sub_total: usize = a.delegated_models.values().sum();
    let mut main: Vec<(String, usize)> = a
        .models
        .iter()
        .map(|(m, c)| (m.clone(), c.saturating_sub(*a.delegated_models.get(m).unwrap_or(&0))))
        .filter(|(_, c)| *c > 0)
        .collect();
    main.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    let main_total: usize = main.iter().map(|(_, c)| c).sum();

    let _ = writeln!(o, "| Model | Main thread | Delegated | Share of delegated |");
    let _ = writeln!(o, "|---|---:|---:|---:|");
    let mut names: Vec<&String> = a.models.keys().collect();
    names.sort_by_key(|m| std::cmp::Reverse(a.models.get(*m).copied().unwrap_or(0)));
    for m in names {
        let sub = *a.delegated_models.get(m).unwrap_or(&0);
        let mt = main.iter().find(|(x, _)| x == m).map(|(_, c)| *c).unwrap_or(0);
        let share = if sub_total > 0 {
            format!("{:.0}%", 100.0 * sub as f64 / sub_total as f64)
        } else {
            "—".into()
        };
        let _ = writeln!(o, "| `{m}` | {} | {} | {} |", n(mt as i64), n(sub as i64), share);
    }

    let top_sub = a.delegated_models.iter().max_by_key(|(_, c)| **c);
    if let (Some((sm, sc)), Some((mm, _))) = (top_sub, main.first()) {
        let share = 100.0 * *sc as f64 / sub_total.max(1) as f64;
        if sm != mm && share >= 60.0 {
            let _ = writeln!(
                o,
                "\nDelegated work runs mostly on `{sm}` ({share:.0}% of sub-agent messages) while the main thread is mostly `{mm}`. That is the shape you want — the expensive model decides, a cheaper one carries out."
            );
        } else if sm == mm {
            let _ = writeln!(
                o,
                "\nSub-agents run on the same model as the main thread (`{sm}`, {} delegated messages against {} on the main thread). Delegation is buying parallelism here, not cost — worth knowing if the plan allowance is tight.",
                n(sub_total as i64),
                n(main_total as i64)
            );
        }
    }
    let _ = writeln!(o);
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
            s.prompts,
            s.tools.len(),
            failed,
            lines
        );
    }
    let _ = writeln!(o, "\n`⚠` = ended without a shutdown record, so its totals are missing.\n");
}

fn method(o: &mut String, tool: Option<crate::Tool>, sessions: &[Session], a: &Analysis) {
    let _ = writeln!(o, "## How to check this\n");
    let _ = writeln!(o, "| Reported as | Read from |");
    let _ = writeln!(o, "|---|---|");
    match tool {
        Some(crate::Tool::VsCode) => {
            // VS Code writes two transcripts and they do not record the same
            // things. Stating one caveat for both would either understate the
            // sessions that have outcomes or overstate the ones that do not.
            let ev = sessions.iter().filter(|s| s.source == Some("events")).count();
            let jn = sessions.len() - ev;
            let _ = writeln!(
                o,
                "| Prompts | `user.message` events, or `message.text` in the journal's `requests[]` |"
            );
            let _ = writeln!(
                o,
                "| Turns, turn length | `assistant.turn_start` → `turn_end`, or `timestamp` → `responseTimestamp` |"
            );
            let _ = writeln!(
                o,
                "| Tool calls | `tool.execution_start`/`_complete`, or `toolInvocationSerialized` parts |"
            );
            let _ = writeln!(o, "| Model | `modelId`, with its `copilot/` prefix stripped |");
            let _ = writeln!(
                o,
                "| Project | the `workspace.json` beside the chat folder, percent-decoded |"
            );
            let _ = writeln!(
                o,
                "\nVS Code keeps **two** transcripts, and they do not record the same \
                 things. `GitHub.copilot-chat/transcripts/` is a full event stream — the \
                 same format Copilot CLI writes — with tool outcomes and real turn \
                 timing. `chatSessions/` is a delta journal with neither. {ev} of these \
                 {} session(s) came from the event stream and {jn} from the journal \
                 alone; the journal-only ones contribute no failure counts and stamp a \
                 request and its response identically. Neither transcript records \
                 tokens, so cost is absent throughout.\n",
                sessions.len()
            );
        }
        Some(crate::Tool::ClaudeCode) => {
            let _ = writeln!(
                o,
                "| Prompts | `user` records — excluding tool results, `isMeta`, and text Claude injects on your behalf (`<system-reminder>`, `<command-name>`, …) |"
            );
            let _ = writeln!(
                o,
                "| Turns, turn length | a prompt to the assistant's last message before the next prompt |"
            );
            let _ = writeln!(
                o,
                "| Tool calls, failures | `tool_use` blocks, closed by the matching `tool_result`; failure is `is_error: true` |"
            );
            let _ = writeln!(o, "| Tokens | `message.usage` on each assistant record, summed |");
            let _ = writeln!(
                o,
                "| Delegated work | sub-agent transcripts under `<session-id>/`, folded into their parent session |"
            );
        }
        _ => {
            let _ = writeln!(o, "| Prompts | `user.message` events |");
            let _ = writeln!(
                o,
                "| Assistant turns, turn length | `assistant.turn_start` → `assistant.turn_end`, paired by turn id |"
            );
            let _ = writeln!(
                o,
                "| Tool calls, failures | `tool.execution_start` → `tool.execution_complete`, paired by tool-call id; failure is `success: false` |"
            );
            let _ = writeln!(
                o,
                "| Code changed, tokens, premium requests | the LAST `session.shutdown` — its totals are cumulative for the session's whole life |"
            );
            let _ = writeln!(o, "| Permission prompts | `session.permissions_changed` events |");
        }
    }
    let _ = writeln!(
        o,
        "\nPercentiles are nearest-rank over observed values, so every figure shown was \
         actually measured. Sessions without a shutdown record contribute to counts and \
         timings but not to totals — {} of {} here.\n",
        a.sessions_with_totals, a.sessions
    );
}
