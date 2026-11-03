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
pub fn dur(ms: i64) -> String {
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
        "Drawn from {} {} session{} between {} and {}. Everything below comes from the \
         transcripts themselves; nothing is estimated or inferred from outside them.\n",
        a.sessions,
        tool.map(crate::Tool::label).unwrap_or("coding assistant"),
        if a.sessions == 1 { "" } else { "s" },
        day(a.first_ms),
        day(a.last_ms)
    );

    coverage(&mut o, a);
    at_a_glance(&mut o, a);
    cost(&mut o, a);
    working_well(&mut o, a);
    friction(&mut o, a, sessions);
    delegation(&mut o, a);
    o
}

/// The reference half: the session table and how every figure was derived.
///
/// Kept apart so it can be placed AFTER the synthesised sections. It was
/// previously emitted before them, which put a 40-row table and a methodology
/// appendix between the reader and the only part of the report that says
/// anything — the report read as unchanged because the changed part was 200
/// lines down.
pub fn reference(tool: Option<crate::Tool>, sessions: &[Session], a: &Analysis) -> String {
    let mut o = String::new();
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
    // The gaps between stamps: n/a where the format does not time its turns,
    // as the comparison report says for the same figure.
    if a.timing_is_measurable() {
        let _ = writeln!(
            o,
            "| Time actively working | {} (idle gaps over 10 min excluded) |",
            dur(a.active_ms)
        );
    } else {
        let _ = writeln!(
            o,
            "| Time actively working | n/a (this transcript format does not time its turns) |"
        );
    }
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
    // Only when the transcript actually times turns (the median, as everywhere
    // else: #123). VS Code writes
    // `responseTimestamp` equal to `timestamp` on these records, so every turn
    // measures zero — printing "0s typical" would read as instant replies rather
    // than as a field the format does not populate.
    if a.timing_is_measurable()
        && let Some(p) = percentile(&a.turn_ms_sorted, 50.0)
    {
        let p90 = percentile(&a.turn_ms_sorted, 90.0).unwrap_or(p);
        let _ = writeln!(
            o,
            "| Turn length | {} typical, {} at the 90th percentile |",
            dur(p),
            dur(p90)
        );
    }
    // How long the PERSON took to reply, as distinct from how long the assistant
    // took to work. A short reply time means the output was read and acted on;
    // a long one means it was studied, or the person had left.
    if let Some(p) = percentile(&a.response_ms_sorted, 50.0) {
        let p90 = percentile(&a.response_ms_sorted, 90.0).unwrap_or(p);
        let _ = writeln!(
            o,
            "| Your reply time | {} typical, {} at the 90th percentile |",
            dur(p),
            dur(p90)
        );
    }
    // Where the time actually goes. Stated as a fact rather than as advice —
    // it was previously only reachable through a suggestion, which meant the
    // measurement disappeared along with the template.
    if let Some(share) = a.api_share_pct()
        && a.api_duration_ms > 0
    {
        let _ = writeln!(
            o,
            "| Waiting on the model | {} of {} active ({share:.0}%) |",
            dur(a.api_duration_ms),
            dur(a.active_ms)
        );
    }
    if a.git_commits > 0 || a.git_pushes > 0 {
        let _ = writeln!(
            o,
            "| Shipped | {} commit(s), {} push(es) |",
            n(a.git_commits as i64),
            n(a.git_pushes as i64)
        );
    }
    let _ = writeln!(o);
    languages(o, a);
    tool_mix(o, a);
}

/// What the person actually reaches for.
///
/// The shape of the tool mix says more about how someone works than any single
/// rate does — a run dominated by reads is exploration, one dominated by edits
/// is execution, and a long tail of one MCP server is an integration.
fn tool_mix(o: &mut String, a: &Analysis) {
    if a.tools.len() < 3 {
        return;
    }
    let total: usize = a.tools.iter().map(|t| t.calls).sum();
    if total == 0 {
        return;
    }
    let _ = writeln!(o, "## What you reach for\n");
    let _ = writeln!(o, "| Tool | Calls | Share | Failed |");
    let _ = writeln!(o, "|---|---:|---:|---:|");
    for t in a.tools.iter().take(10) {
        let _ = writeln!(
            o,
            "| `{}` | {} | {:.0}% | {} |",
            t.name,
            n(t.calls as i64),
            100.0 * t.calls as f64 / total as f64,
            // Absent, not zero, when the transcript records no outcome at all.
            t.failure_pct().map(|p| format!("{p:.1}%")).unwrap_or_else(|| "n/a".into())
        );
    }
    let shown: usize = a.tools.iter().take(10).map(|t| t.calls).sum();
    if a.tools.len() > 10 {
        let _ = writeln!(
            o,
            "\n{} further tool(s) account for the remaining {:.0}%.\n",
            a.tools.len() - 10,
            100.0 * (total - shown) as f64 / total as f64
        );
    } else {
        let _ = writeln!(o);
    }
}

/// What the work was actually written in.
///
/// Counted per file-addressing tool call, so it reflects where the effort went
/// rather than what the repo happens to contain. Search tools are excluded —
/// a grep at a repo root is not work in every language beneath it.
fn languages(o: &mut String, a: &Analysis) {
    if a.languages.is_empty() {
        return;
    }
    let mut rows: Vec<(&String, &usize)> = a.languages.iter().collect();
    rows.sort_by(|x, y| y.1.cmp(x.1).then(x.0.cmp(y.0)));
    let total: usize = a.languages.values().sum();
    let _ = writeln!(o, "## What you work in\n");
    let _ = writeln!(o, "| Language | File touches | Share |");
    let _ = writeln!(o, "|---|---:|---:|");
    for (lang, count) in rows.iter().take(10) {
        let _ = writeln!(
            o,
            "| {lang} | {} | {:.0}% |",
            n(**count as i64),
            100.0 * **count as f64 / total as f64
        );
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
            "**Tool use is landing.** {:.1}% of the {} tool calls that reported an outcome \
             failed. Most of the loop is productive rather than spent recovering.",
            f,
            n(a.tool_outcomes_known as i64)
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
        // The lead-in carries THIS person's numbers. A fixed sentence read
        // identically in four reports, which is the tell that a section is
        // describing a category rather than reporting a measurement.
        let longest = a.failure_runs.iter().map(|r| r.length).max().unwrap_or(0);
        let shown = a.failure_runs.len().min(3);
        items.push(format!(
            "**{} run{} where the same call failed back-to-back**, the longest {longest} deep. \
             Retrying rarely resolves a wrong path or a missing binary, so these are usually \
             the agent stuck rather than the work being hard{}:{lines}",
            a.failure_runs.len(),
            if a.failure_runs.len() == 1 { "" } else { "s" },
            if a.failure_runs.len() > shown {
                format!(" — the {shown} longest")
            } else {
                String::new()
            }
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
            "**`{}` fails {:.0}% of the time** ({} of the {} calls that reported an outcome). \
             Against an overall rate of {:.1}%, this one tool is doing most of the recovering.",
            t.name,
            t.failure_pct().unwrap_or(0.0),
            t.failures,
            t.known,
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

// `try_next` lived here: a fixed list of suggestions gated on thresholds, so
// "Consider checkpointing inside long runs" appeared in three reports with only
// the number changing. It is superseded by the synthesised recommendations in
// `retro::recommendations`, which are derived from each person's own sessions.
// The one fact it carried that was not advice — sessions that never closed
// cleanly — is already stated in `method`, where the totals it affects are
// explained.

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
        // n/a, not 0, when no call reported an outcome: a journal records none.
        let failed = if s.tools.iter().any(|t| t.success.is_some()) {
            s.tools.iter().filter(|t| t.failed()).count().to_string()
        } else if s.tools.is_empty() {
            "0".into()
        } else {
            "n/a".into()
        };
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
    // Derived the same way for every ACP, so stated once rather than per tool.
    let _ = writeln!(
        o,
        "| Languages | the file path each tool call addresses (`file_path`, `path`, or the \
         file link in a rendered message), by extension. Search tools are excluded — their \
         path is a search root, not a file worked on |"
    );
    let _ = writeln!(
        o,
        "| Commits, pushes | `git commit` / `git push` in shell tool arguments, counted per \
         chained segment. Only a segment STARTING with `git` counts, so a commit message \
         that mentions committing does not |"
    );
    let _ = writeln!(
        o,
        "| Your reply time | the gap from the assistant finishing to the next prompt, \
         excluding gaps over 10 minutes (a break, not a reply) |"
    );
    let _ = writeln!(
        o,
        "\nPercentiles are nearest-rank over observed values, so every figure shown was \
         actually measured. Sessions without a shutdown record contribute to counts and \
         timings but not to totals — {} of {} here.\n",
        a.sessions_with_totals, a.sessions
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::ToolStat;

    /// #123 (6): the opening line quoted "covering N events", and an event was
    /// a record (Copilot CLI), a record including sub-agents' (Claude) or a
    /// request (a VS Code journal): figures that read as comparable and were
    /// not. The report says what it is drawn from without them.
    #[test]
    fn the_opening_line_quotes_no_event_count() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.sessions = 2;
        let o = report("x", None, &[], &a);
        let opening = o.lines().find(|l| l.starts_with("Drawn from")).unwrap();
        assert!(opening.starts_with("Drawn from 2 coding assistant sessions between"), "{opening}");
        assert!(!opening.contains("event"), "{opening}");
    }

    /// The tool mix is the one place the report says what someone actually
    /// does all day, so it must not silently drop the long tail.
    #[test]
    fn the_tool_mix_accounts_for_everything_it_does_not_list() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.tools = (0..14)
            .map(|i| ToolStat {
                name: format!("t{i}"),
                calls: 100 - i,
                known: 100 - i,
                failures: 0,
            })
            .collect();
        let mut o = String::new();
        tool_mix(&mut o, &a);
        assert!(o.contains("4 further tool(s)"), "the untabled tail is named: {o}");
    }

    /// #123 A6: ONE denominator for "failure rate", the calls that REPORTED
    /// an outcome. The prose used to quote every call beside a rate taken over
    /// the reported ones: "2.0% of 5,000 tool calls failed" where 2.0% was 4/200,
    /// so a reader multiplied and got 100 failures where there were 4.
    #[test]
    fn a_failure_rate_is_quoted_over_the_calls_it_was_taken_over() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.tool_calls = 5000;
        a.tool_outcomes_known = 200;
        a.tool_failures = 4;
        let mut o = String::new();
        working_well(&mut o, &a);
        assert!(o.contains("2.0% of the 200 tool calls that reported an outcome failed"), "{o}");
        assert!(!o.contains("of 5,000 tool calls failed"), "{o}");

        // Per tool, the same denominator: 4 of 200 reported, among 5,000 calls.
        let t = ToolStat { name: "bash".into(), calls: 5000, known: 200, failures: 4 };
        assert_eq!(t.failure_pct(), Some(2.0));
    }

    /// #123 (2): turn length is shown only when timing is measurable, decided
    /// by the MEDIAN like everywhere else. Gated on p90 it printed "0s typical,
    /// 5s at the 90th percentile" for a sample whose median turn is zero.
    #[test]
    fn turn_length_is_shown_only_when_timing_is_measurable() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.turn_ms_sorted = vec![0, 5000];
        let mut o = String::new();
        at_a_glance(&mut o, &a);
        assert!(!o.contains("Turn length"), "{o}");
        a.turn_ms_sorted = vec![3000, 5000];
        let mut o = String::new();
        at_a_glance(&mut o, &a);
        assert!(o.contains("| Turn length | 3s typical"), "{o}");
    }

    /// #123 (b): active time is the gaps between stamps, so where timing is
    /// not measurable it is n/a, as the comparison report already says.
    #[test]
    fn active_time_is_n_a_where_timing_is_not_measurable() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.active_ms = 3_600_000;
        a.turn_ms_sorted = vec![0, 0, 0];
        let mut o = String::new();
        at_a_glance(&mut o, &a);
        assert!(o.contains("| Time actively working | n/a"), "{o}");
    }

    /// #123 (a): the worst-tool line quotes the calls its rate was taken over.
    /// "fails 20% of the time (4 of 100 calls)" read as a contradiction; the
    /// 20% is 4 of the 20 calls that reported an outcome.
    #[test]
    fn the_worst_tool_line_quotes_its_own_denominator() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.tools = vec![ToolStat { name: "bash".into(), calls: 100, known: 20, failures: 4 }];
        let mut o = String::new();
        friction(&mut o, &a, &[]);
        assert!(o.contains("(4 of the 20 calls that reported an outcome)"), "{o}");
    }

    /// #123 (3): a session in which no tool call reported an outcome shows
    /// n/a under Failed, as the comparison report does, never 0.
    #[test]
    fn a_session_with_no_reported_outcome_shows_failed_n_a() {
        let dir = std::env::temp_dir().join("session-report-failed-na");
        let _ = std::fs::remove_dir_all(&dir);
        let chat = dir.join("workspaceStorage/ccc/chatSessions");
        std::fs::create_dir_all(&chat).unwrap();
        let journal = serde_json::json!({"kind": 0, "v": {"requests": [{
            "timestamp": 1_000_000_i64, "responseTimestamp": 1_002_000_i64,
            "message": {"text": "go"}, "requestId": "r1",
            "response": [{"kind": "toolInvocationSerialized", "toolId": "read_file"}]
        }]}});
        let f = chat.join("s.jsonl");
        std::fs::write(&f, format!("{journal}\n")).unwrap();
        let (session, _) = crate::vscode::parse_session(&f).unwrap();
        assert!(!session.tools.is_empty(), "the fixture has a tool call");
        let mut o = String::new();
        per_session(&mut o, &[session]);
        let row = o.lines().find(|l| l.starts_with("| `")).unwrap();
        assert_eq!(row.split('|').nth(6).map(str::trim), Some("n/a"), "{row}");
    }

    /// A transcript that records no outcome must show n/a, never 0.0%, which
    /// would read as "this tool never fails".
    #[test]
    fn a_tool_with_no_recorded_outcome_shows_n_a() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.tools = vec![
            ToolStat { name: "known".into(), calls: 10, known: 10, failures: 1 },
            // Called, but no call reported an outcome: n/a, not 0.0%.
            ToolStat { name: "silent".into(), calls: 7, known: 0, failures: 0 },
            ToolStat { name: "third".into(), calls: 5, known: 5, failures: 0 },
        ];
        let mut o = String::new();
        tool_mix(&mut o, &a);
        assert!(o.contains("10.0%"), "a known rate is shown");
        assert!(o.contains("n/a"), "an unrecorded one is absent, not zero");
    }
}
