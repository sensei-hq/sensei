//! The qualitative half of the retrospective, built from facet records.
//!
//! Every section here is a GROUP-BY over [`crate::facets::Facet`] values plus a
//! lookup table, not free generation. That is deliberate: a model asked to
//! "write a retrospective" produces plausible advice that nobody can check,
//! whereas a group-by over records that each carry a verbatim quote produces
//! advice you can trace to the session that motivated it.
//!
//! Where a section makes a recommendation, the friction that triggered it is
//! named and at least one session is cited.

use crate::facets::Facet;
use crate::metrics::Analysis;
use std::collections::HashMap;
use std::fmt::Write;

/// A suggestion for a friction kind — but ONLY when this person's own numbers
/// support something specific.
///
/// The previous version was a static table keyed on the friction name, so five
/// people with the same tag got five identical paragraphs. Advice that would
/// read the same for anyone is not advice; it is filler that teaches people to
/// skim past the parts that ARE specific.
///
/// So every branch here has to reach for a real measurement and returns `None`
/// when the threshold is not met. A friction with no qualifying number is still
/// reported — with its grounded detail and a cited session, which is the part
/// worth reading — it just carries no suggestion.
fn remedy(friction: &str, a: &Analysis) -> Option<String> {
    match friction {
        // Name the tool actually doing the damage, not "a tool".
        "repeated_tool_failures" | "environment_or_setup" => {
            let overall = a.tool_failure_pct()?;
            let worst = a
                .tools
                .iter()
                .filter(|t| t.calls >= 20)
                .filter_map(|t| t.failure_pct().map(|p| (t, p)))
                .filter(|(_, p)| *p >= (overall * 3.0).max(15.0))
                .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap())?;
            Some(format!(
                "`{}` fails {:.0}% of the time ({} of {} calls) against your overall {overall:.1}%. One tool is producing most of this friction, so it is one fix rather than a habit change.",
                worst.0.name, worst.1, worst.0.failures, worst.0.calls
            ))
        }
        // Only worth saying when the hand-offs really are large.
        "rework_after_correction" | "wrong_direction_taken" => {
            let tpp = a.tools_per_prompt()?;
            (tpp >= 40.0).then(|| {
                format!(
                    "Your hand-offs average {tpp:.0} tool calls per prompt. That is a long way to travel before the first check, so a wrong direction costs the whole run rather than a turn."
                )
            })
        }
        // Only when the sessions are actually long enough to lose the thread.
        "lost_context" => {
            let per_project = a.sessions as f64 / a.projects.max(1) as f64;
            (per_project >= 8.0).then(|| {
                format!(
                    "You average {per_project:.0} sessions per project. Context you re-explain that often has stopped being conversation and become configuration."
                )
            })
        }
        // Only when the tail is genuinely slow, with the number attached.
        "slow_feedback_loop" => {
            let p90 = crate::metrics::percentile(&a.turn_ms_sorted, 90.0)?;
            (p90 >= 120_000).then(|| {
                format!(
                    "Your 90th-percentile turn is {}. At that length the loop is slow enough that batching gets tempting, which is what makes a wrong turn expensive.",
                    crate::render::dur(p90)
                )
            })
        }
        _ => None,
    }
}

fn count<'a>(items: impl Iterator<Item = &'a String>) -> Vec<(String, usize)> {
    let mut m: HashMap<&str, usize> = HashMap::new();
    for i in items {
        *m.entry(i.as_str()).or_default() += 1;
    }
    let mut v: Vec<(String, usize)> = m.into_iter().map(|(k, c)| (k.to_string(), c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// The first segment of a session id.
///
/// The rest of the report cites sessions this way; printing a full 36-character
/// uuid four times in an "also in" list buries the sentence it belongs to.
/// Still unambiguous — these are compared against ids in the same report.
fn short(id: &str) -> &str {
    id.split('-').next().unwrap_or(id)
}

fn humanise(tag: &str) -> String {
    let mut s = tag.replace('_', " ");
    if let Some(c) = s.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    s
}

/// The qualitative sections, appended to the mechanical report.
pub fn report(label: &str, facets: &[Facet], a: &Analysis) -> String {
    let mut o = String::new();
    if facets.is_empty() {
        return o;
    }

    let _ = writeln!(o, "# What the sessions say\n");
    let missing = a.sessions.saturating_sub(facets.len());
    let _ = writeln!(
        o,
        "Derived from {} of {} sessions. Each record was produced by reading that \
         session's prompts and had to quote the transcript verbatim to be kept.{}\n",
        facets.len(),
        a.sessions,
        match missing {
            0 => " Every session yielded one.".to_string(),
            1 => " One session was dropped: nothing could be said about it that the \
                  transcript would back up."
                .to_string(),
            n => format!(
                " {n} sessions were dropped rather than shown — nothing could be said \
                 about them that the transcript would back up."
            ),
        }
    );

    work(&mut o, facets);
    outcomes(&mut o, facets);
    friction(&mut o, facets, label, a);
    highlights(&mut o, facets);
    o
}

fn work(o: &mut String, facets: &[Facet]) {
    let cats = count(facets.iter().flat_map(|f| f.goal_categories.iter()));
    if cats.is_empty() {
        return;
    }
    let _ = writeln!(o, "## What you work on\n");
    let _ = writeln!(o, "| Kind of work | Sessions | Share |");
    let _ = writeln!(o, "|---|---:|---:|");
    for (cat, n) in cats.iter().take(8) {
        let _ = writeln!(
            o,
            "| {} | {n} | {:.0}% |",
            humanise(cat),
            100.0 * *n as f64 / facets.len() as f64
        );
    }
    let _ = writeln!(
        o,
        "\nShares add to more than 100% because one session usually spans several \
         kinds of work — that is the normal shape, not a rounding error.\n"
    );
}

fn outcomes(o: &mut String, facets: &[Facet]) {
    let outs = count(facets.iter().map(|f| &f.outcome));
    if outs.is_empty() {
        return;
    }
    let _ = writeln!(o, "## How sessions ended\n");
    let _ = writeln!(o, "| Outcome | Sessions |");
    let _ = writeln!(o, "|---|---:|");
    for (out, n) in &outs {
        let _ = writeln!(o, "| {} | {n} |", humanise(out));
    }
    let landed = facets
        .iter()
        .filter(|f| matches!(f.outcome.as_str(), "completed" | "mostly_achieved"))
        .count();
    let _ = writeln!(
        o,
        "\n{landed} of {} sessions reached what they set out to do. Read that as the \
         model's summary of what the transcript SAYS, not as a verified result: a \
         session that ends confidently reads as achieved whether or not the change \
         held up, and the reading skews positive because transcripts end on the \
         assistant's last word rather than on the test run afterwards. `Unclear` marks \
         a session whose transcript did not say how it ended.\n",
        facets.len()
    );
}

fn friction(o: &mut String, facets: &[Facet], label: &str, a: &Analysis) {
    let frictions: Vec<(String, usize)> =
        count(facets.iter().flat_map(|f| f.friction.iter()).filter(|f| f.as_str() != "none"))
            .into_iter()
            .collect();
    let _ = writeln!(o, "## Where things go wrong\n");
    if frictions.is_empty() {
        let _ = writeln!(o, "No recurring friction showed up across these sessions.\n");
        return;
    }

    // One session usually carries several frictions, so quoting the first match
    // each time repeats one session's detail under every heading. Prefer a
    // session not already quoted.
    let mut quoted: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (kind, n) in frictions.iter().take(5) {
        let _ = writeln!(o, "### {} — {n} session(s)\n", humanise(kind));
        // Cite the sessions, so the claim can be checked against the source.
        let matching: Vec<&Facet> = facets
            .iter()
            .filter(|f| f.friction.iter().any(|x| x == kind))
            .filter(|f| !f.friction_detail.trim().is_empty())
            .collect();
        // Only a session NOT already quoted. Falling back to the first match
        // re-printed one session's detail and citation verbatim under a second
        // heading, which reads as padding — the heading, the count and the
        // "also in" list still carry the finding without it.
        let pick = matching.iter().find(|f| !quoted.contains(f.session_id.as_str())).copied();
        let cited = matching
            .iter()
            .filter(|f| Some(f.session_id.as_str()) != pick.map(|p| p.session_id.as_str()))
            .copied();
        if let Some(f) = pick {
            quoted.insert(f.session_id.as_str());
            let _ = writeln!(o, "{}\n", f.friction_detail.trim());
            let _ = writeln!(
                o,
                "> Session `{}` — {}\n",
                short(&f.session_id),
                f.underlying_goal.trim()
            );
        }
        let others: Vec<&str> = cited.map(|f| short(&f.session_id)).take(4).collect();
        if !others.is_empty() {
            let _ = writeln!(o, "Also in: {}\n", others.join(", "));
        }
        if let Some(why) = remedy(kind, a) {
            let _ = writeln!(o, "**Worth a look:** {why}\n");
        }
    }
    let _ = writeln!(
        o,
        "These are the frictions worth {label}'s attention in that order — the count is \
         how many sessions showed each, not how costly each was.\n"
    );
}

fn highlights(o: &mut String, facets: &[Facet]) {
    let mut wins: Vec<&Facet> = facets
        .iter()
        .filter(|f| matches!(f.outcome.as_str(), "completed" | "mostly_achieved"))
        .filter(|f| !f.primary_success.trim().is_empty())
        .collect();
    if wins.is_empty() {
        return;
    }
    // Longest goal first is a crude proxy for the most substantial session, but
    // it is deterministic and checkable; a model-picked "most impressive" would
    // be neither.
    wins.sort_by_key(|f| std::cmp::Reverse(f.underlying_goal.len()));
    let _ = writeln!(o, "## What went well\n");
    for f in wins.iter().take(5) {
        let _ = writeln!(o, "- **{}**", f.primary_success.trim());
        let _ = writeln!(o, "  {}", f.underlying_goal.trim());
        let _ = writeln!(o, "  <br>Session `{}`\n", short(&f.session_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facet(outcome: &str, cats: &[&str], friction: &[&str]) -> Facet {
        Facet {
            session_id: "s1".into(),
            underlying_goal: "ship the thing".into(),
            goal_categories: cats.iter().map(|s| s.to_string()).collect(),
            outcome: outcome.into(),
            friction: friction.iter().map(|s| s.to_string()).collect(),
            friction_detail: "the build kept failing".into(),
            primary_success: "got it green".into(),
            brief_summary: "did work".into(),
            evidence: "ship the thing".into(),
        }
    }

    /// `none` is a real answer meaning "it went smoothly" — counting it as a
    /// friction kind would make every clean session look like a problem.
    #[test]
    fn none_is_not_reported_as_friction() {
        let f = vec![facet("completed", &["testing"], &["none"])];
        let out = friction_only(&f);
        assert!(out.contains("No recurring friction"), "got: {out}");
    }

    /// A reported friction must always be traceable, whether or not it earns a
    /// suggestion — the citation is the part that makes it checkable.
    #[test]
    fn a_reported_friction_always_cites_its_session() {
        let f = vec![facet("partial", &["testing"], &["repeated_tool_failures"])];
        let out = friction_only(&f);
        assert!(out.contains("Repeated tool failures"));
        assert!(out.contains("the build kept failing"), "carries the grounded detail");
        assert!(out.contains("Session `s1`"), "must cite the session it came from");
    }

    /// The inverse of the old contract. A friction with nothing measured behind
    /// it must produce NO suggestion — generic advice that reads the same for
    /// everyone is what made these reports skimmable in the bad sense.
    #[test]
    fn a_friction_with_no_supporting_data_gets_no_suggestion() {
        let empty = crate::metrics::analyse(&[], 0);
        for k in crate::facets::FRICTION_KINDS {
            assert!(remedy(k, &empty).is_none(), "{k} invented advice from an empty analysis");
        }
    }

    /// When the data DOES support it, the suggestion has to name the specific
    /// thing — the tool and its rate — not describe the category.
    #[test]
    fn a_supported_friction_names_the_tool_and_its_rate() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.tool_outcomes_known = 1000;
        a.tool_failures = 20; // 2% overall
        a.tools = vec![
            crate::metrics::ToolStat { name: "flaky_tool".into(), calls: 100, failures: 60 },
            crate::metrics::ToolStat { name: "fine_tool".into(), calls: 900, failures: 9 },
        ];
        let out = remedy("repeated_tool_failures", &a).expect("60% against 2% qualifies");
        assert!(out.contains("flaky_tool"), "must name the offender: {out}");
        assert!(out.contains("60%"), "must carry its rate: {out}");
        assert!(!out.contains("fine_tool"), "must not indict the healthy tool");
    }

    /// A tool that fails a lot but is barely used is noise, not a finding.
    #[test]
    fn a_rarely_used_tool_does_not_qualify() {
        let mut a = crate::metrics::analyse(&[], 0);
        a.tool_outcomes_known = 1000;
        a.tool_failures = 20;
        a.tools = vec![crate::metrics::ToolStat { name: "rare".into(), calls: 3, failures: 3 }];
        assert!(remedy("repeated_tool_failures", &a).is_none());
    }

    fn friction_only(f: &[Facet]) -> String {
        let mut o = String::new();
        friction(&mut o, f, "someone", &crate::metrics::analyse(&[], 0));
        o
    }
}
