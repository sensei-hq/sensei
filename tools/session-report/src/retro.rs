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

/// What to suggest when a friction kind shows up often.
///
/// A table, not a model: these are the same few remedies every time, and
/// generating them fresh per report would vary the advice without varying the
/// evidence.
fn remedy(friction: &str) -> Option<(&'static str, &'static str)> {
    Some(match friction {
        "repeated_tool_failures" => (
            "Pin the failing tool down",
            "A tool that fails repeatedly is usually one misconfiguration, not bad luck. \
             The failure runs listed in the friction section name which tool and when.",
        ),
        "wrong_direction_taken" => (
            "State the constraint before the goal",
            "Work that goes the wrong way is usually missing a constraint the person \
             knew and did not say. Putting the non-negotiables in the first prompt costs \
             one line and saves the round trip.",
        ),
        "misunderstood_requirement" => (
            "Name the acceptance check up front",
            "Say what would prove the change correct — the test, the command, the screen \
             — in the same message as the request. It converts an interpretation into a \
             check.",
        ),
        "lost_context" => (
            "Write the decisions down where the assistant reads them",
            "Repeating the same context across sessions is a sign it belongs in a \
             project instruction file rather than in each prompt.",
        ),
        "environment_or_setup" => (
            "Fix the environment once, in the repo",
            "Setup friction repeats for everyone. A script or a documented preflight \
             turns a recurring cost into a one-time one.",
        ),
        "slow_feedback_loop" => (
            "Shorten what runs after each change",
            "Long feedback loops push people toward big unverified batches. A narrower \
             test target changes the working rhythm more than any prompt wording.",
        ),
        "rework_after_correction" => (
            "Correct earlier, in smaller pieces",
            "Rework concentrates where a large hand-off went unchecked. Smaller \
             hand-offs cost more interruptions and much less rework.",
        ),
        _ => return None,
    })
}

fn count<'a>(items: impl Iterator<Item = &'a String>) -> Vec<(String, usize)> {
    let mut m: HashMap<&str, usize> = HashMap::new();
    for i in items {
        *m.entry(i.as_str()).or_default() += 1;
    }
    let mut v: Vec<(String, usize)> =
        m.into_iter().map(|(k, c)| (k.to_string(), c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
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
    friction(&mut o, facets, label);
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

fn friction(o: &mut String, facets: &[Facet], label: &str) {
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
        let pick = matching
            .iter()
            .find(|f| !quoted.contains(f.session_id.as_str()))
            .or(matching.first())
            .copied();
        let cited = matching
            .iter()
            .filter(|f| Some(f.session_id.as_str()) != pick.map(|p| p.session_id.as_str()))
            .copied();
        if let Some(f) = pick {
            quoted.insert(f.session_id.as_str());
            let _ = writeln!(o, "{}\n", f.friction_detail.trim());
            let _ = writeln!(o, "> Session `{}` — {}\n", f.session_id, f.underlying_goal.trim());
        }
        let others: Vec<&str> = cited.map(|f| f.session_id.as_str()).take(4).collect();
        if !others.is_empty() {
            let _ = writeln!(o, "Also in: {}\n", others.join(", "));
        }
        if let Some((title, why)) = remedy(kind) {
            let _ = writeln!(o, "**Try:** {title}. {why}\n");
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
        let _ = writeln!(o, "  <br>Session `{}`\n", f.session_id);
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

    #[test]
    fn friction_carries_a_remedy_and_a_citation() {
        let f = vec![facet("partial", &["testing"], &["repeated_tool_failures"])];
        let out = friction_only(&f);
        assert!(out.contains("Repeated tool failures"));
        assert!(out.contains("**Try:**"), "a named friction must carry a remedy");
        assert!(out.contains("Session `s1`"), "must cite the session it came from");
    }

    /// Every friction in the vocabulary needs a remedy, or the report names a
    /// problem and offers nothing for it.
    #[test]
    fn every_friction_kind_has_a_remedy() {
        for k in crate::facets::FRICTION_KINDS {
            if *k == "none" {
                continue;
            }
            assert!(remedy(k).is_some(), "no remedy for {k}");
        }
    }

    fn friction_only(f: &[Facet]) -> String {
        let mut o = String::new();
        friction(&mut o, f, "someone");
        o
    }
}
