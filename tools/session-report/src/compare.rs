//! Side-by-side across everyone in a folder.
//!
//! The hard part is that these are different tools. Copilot CLI reports lines
//! changed and premium requests; Claude Code reports neither. Comparing on a
//! signal only one of them has would rank people by which assistant they happen
//! to use, which is worthless and unfair.
//!
//! So the comparison sticks to what every transcript records and means the same
//! thing in each — prompts, tool calls, active time, tool outcomes — and puts the
//! tool-specific figures in a separate table.
//!
//! Cache reuse is excluded even though both tools report it, because they define
//! it differently: Claude's `input_tokens` counts only the non-cached portion, so
//! reuse computes near 100%, while Copilot's appears to include cached tokens and
//! lands near 50%. One column holding both would rank people by which assistant
//! they use. It stays in the individual reports, where the comparison is
//! within-tool.
//!
//! There is deliberately no composite score and no ranking. "Velocity" here is
//! throughput per active hour, which says how much moved, not how well; two
//! people at the same rate can be doing very different work.

use crate::metrics::Analysis;
use std::fmt::Write;

pub struct Person {
    pub name: String,
    pub tool: Option<crate::Tool>,
    pub analysis: Analysis,
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

fn hours(ms: i64) -> f64 {
    ms as f64 / 3_600_000.0
}

pub fn report(people: &[Person]) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# Working patterns across the team\n");
    let _ = writeln!(
        o,
        "{} people, {} sessions, {} of active time. Read this as a description of \
         different working styles, not a league table — there is no score here and no \
         ranking, because the numbers below measure how much moved, not how well.\n",
        people.len(),
        people.iter().map(|p| p.analysis.sessions).sum::<usize>(),
        {
            let h = hours(people.iter().map(|p| p.analysis.active_ms).sum());
            format!("{h:.0} hours")
        }
    );

    let mixed = people.iter().map(|p| p.tool).collect::<std::collections::HashSet<_>>().len() > 1;
    if mixed {
        let _ = writeln!(
            o,
            "> **Different tools.** The table below uses only signals every transcript \
             records. Lines changed and premium requests exist in Copilot CLI and not in \
             Claude Code, so they are kept separate rather than shown as blanks or zeros.\n"
        );
    }

    // ── Shared ground ────────────────────────────────────────────────────────
    let _ = writeln!(o, "## Comparable across tools\n");
    let _ = writeln!(
        o,
        "| Person | Tool | Sessions | Active | Prompts | Tool calls | Calls/hour | Tools/prompt | Tool failures |"
    );
    let _ = writeln!(o, "|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for p in people {
        let a = &p.analysis;
        let h = hours(a.active_ms);
        // Only when the transcript times its turns. Deriving a rate from a
        // format that stamps request and response identically produces a number
        // that says more about the format than the person.
        let per_hour = if h > 0.0 && a.timing_is_measurable() {
            format!("{:.0}", a.tool_calls as f64 / h)
        } else {
            "n/a".into()
        };
        let active = if a.timing_is_measurable() { format!("{h:.0}h") } else { "n/a".into() };
        let _ = writeln!(
            o,
            "| **{}** | {} | {} | {} | {} | {} | {} | {} | {} |",
            p.name,
            match p.tool {
                Some(crate::Tool::CopilotCli) => "Copilot CLI",
                Some(crate::Tool::ClaudeCode) => "Claude Code",
                Some(crate::Tool::VsCode) => "VS Code",
                None => "—",
            },
            a.sessions,
            active,
            n(a.prompts as i64),
            n(a.tool_calls as i64),
            per_hour,
            a.tools_per_prompt().map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".into()),
            a.tool_failure_pct().map(|v| format!("{v:.1}%")).unwrap_or_else(|| "n/a".into()),
        );
    }
    let _ = writeln!(o);

    // ── Reading it ───────────────────────────────────────────────────────────
    let _ = writeln!(o, "## What the spread says\n");
    observations(&mut o, people);

    // ── Tool-specific ────────────────────────────────────────────────────────
    let cop: Vec<&Person> =
        people.iter().filter(|p| p.tool == Some(crate::Tool::CopilotCli)).collect();
    if !cop.is_empty() {
        let _ = writeln!(o, "## Copilot CLI only\n");
        let _ = writeln!(
            o,
            "Not available for Claude Code, so shown separately rather than as an empty \
             column.\n"
        );
        let _ = writeln!(
            o,
            "| Person | Lines added | Lines removed | Files touched | Premium requests |"
        );
        let _ = writeln!(o, "|---|---:|---:|---:|---:|");
        for p in cop {
            let a = &p.analysis;
            let _ = writeln!(
                o,
                "| **{}** | +{} | −{} | {} | {} |",
                p.name,
                n(a.lines_added),
                n(a.lines_removed),
                n(a.files_modified as i64),
                n(a.premium_requests)
            );
        }
        let _ = writeln!(o);
    }

    let _ = writeln!(o, "## How to read the columns\n");
    let _ = writeln!(o, "| Column | Means | Does not mean |");
    let _ = writeln!(o, "|---|---|---|");
    let _ = writeln!(
        o,
        "| Active | Time with something happening; gaps over 10 minutes dropped | Hours at the desk |"
    );
    let _ = writeln!(
        o,
        "| Calls/hour | Throughput — how fast the loop turns | Productivity, or value delivered |"
    );
    let _ = writeln!(
        o,
        "| Tools/prompt | How much is handed over per instruction | Quality of the instruction |"
    );
    let _ = writeln!(
        o,
        "| Tool failures | Share of calls the tool itself reported as failed | Mistakes by the person |"
    );
    let _ = writeln!(
        o,
        "| `n/a` | The transcript does not record it — VS Code stamps a request and its response identically and reports no tool outcome | Zero, or nothing to report |"
    );
    let _ = writeln!(
        o,
        "\nA high calls/hour with a high failure rate is churn. A low calls/hour with \
         large tools/prompt is delegation — fewer, bigger hand-offs. Neither is better \
         in the abstract.\n"
    );
    o
}

fn observations(o: &mut String, people: &[Person]) {
    let mut items: Vec<String> = Vec::new();

    // Spread in how much is handed over per prompt — the clearest style signal.
    let mut by_tpp: Vec<(&str, f64)> = people
        .iter()
        .filter_map(|p| p.analysis.tools_per_prompt().map(|v| (p.name.as_str(), v)))
        .collect();
    by_tpp.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    if let (Some(hi), Some(lo)) = (by_tpp.first(), by_tpp.last())
        && hi.1 >= lo.1 * 3.0
    {
        items.push(format!(
            "**Very different hand-off sizes.** {} averages {:.0} tool calls per prompt; \
             {} averages {:.0}. That is a working style, not a skill gap: bigger hand-offs \
             mean fewer interruptions but a longer way back when the direction is wrong.",
            hi.0, hi.1, lo.0, lo.1
        ));
    }

    // Friction outliers.
    let rates: Vec<(&str, f64)> = people
        .iter()
        .filter(|p| p.analysis.tool_calls > 500)
        .filter_map(|p| p.analysis.tool_failure_pct().map(|v| (p.name.as_str(), v)))
        .collect();
    if rates.len() > 1 {
        let mean = rates.iter().map(|r| r.1).sum::<f64>() / rates.len() as f64;
        for (name, r) in &rates {
            if *r > mean * 2.0 && *r > 3.0 {
                items.push(format!(
                    "**{name} hits roughly twice the failure rate of the group** ({r:.1}% \
                     against a {mean:.1}% average). Worth looking at which tools — a single \
                     misconfigured one usually explains it, and it is fixable."
                ));
            }
        }
    }

    // Cache reuse is deliberately NOT compared across tools. Both report it, but
    // they mean different things: Claude's `input_tokens` counts only the
    // NON-cached portion, so reuse computes near 100%, while Copilot's
    // `inputTokens` appears to include cached tokens and lands near 50%. Putting
    // them in one column would rank people by which assistant they use. It stays
    // in the individual reports, where the comparison is within a tool.

    // Delegation, as one line — three near-identical bullets read as padding.
    let mut delegators: Vec<(&str, usize, usize)> = people
        .iter()
        .filter(|p| p.analysis.delegated > 0)
        .map(|p| (p.name.as_str(), p.analysis.delegated, p.analysis.sessions))
        .collect();
    delegators.sort_by_key(|d| std::cmp::Reverse(d.1));
    if !delegators.is_empty() {
        let list = delegators
            .iter()
            .map(|(n, d, s)| format!("{n} ({d} across {s} sessions)"))
            .collect::<Vec<_>>()
            .join(", ");
        items.push(format!(
            "**Sub-agents carry a large share of the work** — {list}. Each delegated agent \
             writes its own transcript, so the parent session records only the hand-off. \
             That work is folded into the figures above; counting sessions alone would have \
             missed most of it."
        ));
    }

    if items.is_empty() {
        let _ = writeln!(o, "Nothing stands out as an outlier across the group.\n");
        return;
    }
    for i in items {
        let _ = writeln!(o, "- {i}\n");
    }
}
