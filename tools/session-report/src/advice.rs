//! Person-level recommendations, synthesised from that person's own sessions.
//!
//! The previous version was a lookup keyed on a friction name. Gating it on a
//! threshold made the NUMBERS specific but left the sentences fixed, so five
//! people still read four of the same paragraphs. A template with a variable in
//! it is still a template.
//!
//! So this makes one more model call, at the PERSON level rather than the
//! session level, over everything already derived about them: what they were
//! trying to do, what got in the way, what they touched, and the mechanical
//! figures. One call per person, cached, so it costs nothing to re-render.
//!
//! The output is only kept when it is demonstrably about THEM — see
//! [`validate`]. That check is what stops a local model from falling back on
//! the same advice this module exists to get away from.

use crate::facets::Facet;
use crate::metrics::Analysis;
use serde::{Deserialize, Serialize};

/// How many friction details to show the model. Enough to see a pattern,
/// bounded so a 45-session person still fits a local context window.
const MAX_DETAILS: usize = 18;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Insights {
    /// Two or three sentences on how this person actually works.
    #[serde(default)]
    pub working_style: String,
    #[serde(default)]
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Recommendation {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub detail: String,
    /// A session id from the set the model was shown. Checked, so a
    /// recommendation can always be traced back to the work that prompted it.
    #[serde(default)]
    pub evidence_session: String,
    /// A concrete thing from THEIR transcripts — a tool, a technology, a
    /// repeated task. Checked against the input, which is what makes a generic
    /// recommendation fail rather than merely read poorly.
    #[serde(default)]
    pub grounded_in: String,
}

/// The material the model gets: everything already derived, nothing invented.
fn brief(name: &str, facets: &[Facet], a: &Analysis) -> String {
    let mut goals: Vec<(&str, usize)> = Vec::new();
    for f in facets {
        for g in &f.goal_categories {
            match goals.iter_mut().find(|(k, _)| *k == g.as_str()) {
                Some((_, c)) => *c += 1,
                None => goals.push((g.as_str(), 1)),
            }
        }
    }
    goals.sort_by_key(|(_, c)| std::cmp::Reverse(*c));

    let mut frictions: Vec<(&str, usize)> = Vec::new();
    for f in facets {
        for k in f.friction.iter().filter(|k| k.as_str() != "none") {
            match frictions.iter_mut().find(|(x, _)| *x == k.as_str()) {
                Some((_, c)) => *c += 1,
                None => frictions.push((k.as_str(), 1)),
            }
        }
    }
    frictions.sort_by_key(|(_, c)| std::cmp::Reverse(*c));

    let details: Vec<String> = facets
        .iter()
        .filter(|f| !f.friction_detail.trim().is_empty())
        .take(MAX_DETAILS)
        .map(|f| format!("- [{}] {}", short(&f.session_id), f.friction_detail.trim()))
        .collect();

    let goal_lines: Vec<String> = facets
        .iter()
        .take(MAX_DETAILS)
        .map(|f| format!("- [{}] {}", short(&f.session_id), f.underlying_goal.trim()))
        .collect();

    let mut langs: Vec<(&String, &usize)> = a.languages.iter().collect();
    langs.sort_by_key(|(_, c)| std::cmp::Reverse(**c));

    let tools: Vec<String> = a
        .tools
        .iter()
        .take(12)
        .map(|t| {
            let f = t.failure_pct().map(|p| format!("{p:.0}% fail")).unwrap_or_default();
            format!("{} ({} calls{}{})", t.name, t.calls, if f.is_empty() { "" } else { ", " }, f)
        })
        .collect();

    format!(
        "Developer: {name}\n\
         Sessions analysed: {}\n\
         Languages touched: {}\n\
         Tools used most: {}\n\
         Tool calls per prompt: {}\n\
         Commits: {}, pushes: {}\n\
         Kinds of work: {}\n\
         Frictions seen: {}\n\n\
         WHAT THEY WERE DOING (session id in brackets):\n{}\n\n\
         WHAT WENT WRONG (session id in brackets):\n{}\n",
        facets.len(),
        langs.iter().take(5).map(|(l, c)| format!("{l} ({c})")).collect::<Vec<_>>().join(", "),
        tools.join(", "),
        a.tools_per_prompt().map(|v| format!("{v:.0}")).unwrap_or_else(|| "unknown".into()),
        a.git_commits,
        a.git_pushes,
        goals.iter().map(|(g, c)| format!("{g} x{c}")).collect::<Vec<_>>().join(", "),
        frictions.iter().map(|(g, c)| format!("{g} x{c}")).collect::<Vec<_>>().join(", "),
        goal_lines.join("\n"),
        if details.is_empty() { "- none recorded".to_string() } else { details.join("\n") },
    )
}

fn short(id: &str) -> &str {
    id.split('-').next().unwrap_or(id)
}

fn prompt_for(name: &str, facets: &[Facet], a: &Analysis) -> String {
    format!(
        "You are writing the recommendations section of a retrospective for ONE developer, \
         from their own coding sessions. Everything below was derived from their transcripts.\n\n\
         {}\n\
         Write 3 or 4 recommendations. Each one must cover a DIFFERENT angle — pick \
         from: the specific tool or command that keeps failing them; how they direct \
         the assistant; a technology or subsystem that recurs; something about their \
         environment or setup; how they verify work. Do not write two recommendations \
         from the same angle.\n\n\
         Rules:\n\
         - Name the specific tool, technology, feature or failure you are reacting to. \
           It must be one that appears above.\n\
         - At most ONE recommendation may be about breaking work into smaller pieces, \
           phases, chunks, batches or sprints. That is the obvious answer to almost any \
           friction and it is worth saying at most once.\n\
         - Reject anything that would read the same for another developer. \"Write \
           smaller commits\", \"add more tests\", \"communicate requirements clearly\", \
           \"plan before coding\" are all too generic to be useful.\n\
         - Say what to do differently, and why it follows from the evidence above.\n\
         - `evidence_session` must be one of the bracketed session ids above.\n\
         - `grounded_in` must name the tool, technology or task from the material \
           that the recommendation is about.\n\n\
         Reply with ONLY a JSON object:\n\
         {{\"working_style\": \"2-3 sentences on how {name} actually works, from the evidence\", \
         \"recommendations\": [{{\"title\": \"short imperative\", \"detail\": \"2-3 sentences\", \
         \"evidence_session\": \"<id>\", \"grounded_in\": \"<phrase from above>\"}}]}}\n",
        brief(name, facets, a)
    )
}

/// Keep only the recommendations that are demonstrably about this person.
///
/// A local model under-instructed will happily return "adopt CI" for everyone.
/// Two checks make that fail rather than merely read badly: the cited session
/// has to be one it was shown, and the thing it claims to be reacting to has to
/// appear in the material. A recommendation that passes both is, at minimum,
/// talking about work this person actually did.
fn validate(mut ins: Insights, facets: &[Facet], material: &str) -> (Insights, Vec<String>) {
    let lower = material.to_ascii_lowercase();
    let mut dropped = Vec::new();
    ins.recommendations.retain(|r| {
        if r.title.trim().is_empty() || r.detail.trim().len() < 30 {
            dropped.push(format!("{}: body too short to be a recommendation", r.title.trim()));
            return false;
        }
        let cited = facets.iter().any(|f| {
            let s = short(&f.session_id);
            r.evidence_session.contains(s) || s == r.evidence_session
        });
        if !cited {
            dropped.push(format!("{}: cites a session it was not shown", r.title.trim()));
            return false;
        }
        if !grounded_in_material(&r.grounded_in, &lower) {
            dropped.push(format!(
                "{}: \"{}\" does not appear in their material",
                r.title.trim(),
                r.grounded_in.trim()
            ));
            return false;
        }
        true
    });

    // "Break the work into smaller pieces" is the obvious answer to almost any
    // friction, and a model reaches for it repeatedly. Asking for at most one in
    // the prompt got two through on two of five people, so it is enforced here:
    // keep the first, drop the rest. The angle is worth making once.
    let mut seen_decomposition = false;
    ins.recommendations.retain(|r| {
        if !is_decomposition(&format!("{} {}", r.title, r.detail)) {
            return true;
        }
        if seen_decomposition {
            dropped.push(format!("{}: a second 'break it up' recommendation", r.title.trim()));
            return false;
        }
        seen_decomposition = true;
        true
    });
    (ins, dropped)
}

/// Whether a recommendation is another way of saying "do it in smaller pieces".
fn is_decomposition(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    ["smaller", "chunk", "sprint", "sequential phase", "distinct phase", "batches", "break down"]
        .iter()
        .any(|k| t.contains(k))
}

/// Whether the thing a recommendation claims to react to really is in this
/// person's material.
///
/// Matched on WORDS rather than as one substring. Requiring the whole phrase
/// verbatim rejected recommendations that were correctly about the person but
/// reworded the thing slightly — "Angular migration" against a brief that says
/// "Angular case list migration" — and losing those leaves someone with no
/// recommendations at all. Most content words still have to be theirs, so
/// advice about something absent from their work is still rejected.
fn grounded_in_material(grounded_in: &str, lower_material: &str) -> bool {
    let words: Vec<String> = grounded_in
        .to_ascii_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .map(str::to_string)
        .collect();
    if words.is_empty() {
        return false;
    }
    let hits = words.iter().filter(|w| lower_material.contains(w.as_str())).count();
    hits * 10 >= words.len() * 6
}

/// Synthesise recommendations for one person.
///
/// Retried once when nothing survives validation. The generator is stochastic,
/// and one unlucky sample left a person with no recommendations at all while
/// the immediate re-run produced three good ones — that is variance, not a
/// finding about them, and it should not be the difference between a section
/// and a blank.
pub fn derive(
    name: &str,
    facets: &[Facet],
    a: &Analysis,
    endpoint: &str,
    model: &str,
) -> Result<Insights, String> {
    match attempt(name, facets, a, endpoint, model) {
        Ok(ins) => Ok(ins),
        Err(first) => attempt(name, facets, a, endpoint, model)
            .map_err(|second| format!("{first} (retried: {second})")),
    }
}

fn attempt(
    name: &str,
    facets: &[Facet],
    a: &Analysis,
    endpoint: &str,
    model: &str,
) -> Result<Insights, String> {
    if facets.len() < 3 {
        return Err("too few session records to synthesise from".into());
    }
    let material = brief(name, facets, a);
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt_for(name, facets, a),
        "stream": false,
        "format": "json",
        "options": {"temperature": 0.2, "num_ctx": 16384, "num_predict": 2000},
    });

    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "600",
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
    let payload = reply["response"].as_str().ok_or("no response field")?;
    let object = crate::facets::json_object(payload).ok_or("no JSON object in reply")?;
    let ins: Insights =
        serde_json::from_str(object).map_err(|e| format!("non-conforming JSON: {e}"))?;

    let (kept, dropped) = validate(ins, facets, &material);
    if kept.recommendations.is_empty() {
        // Say WHY, per rejected item. A silent "none" reads as "nothing to say
        // about this person", which is a different and much less useful claim.
        return Err(format!(
            "no recommendation survived the grounding check: {}",
            dropped.join("; ")
        ));
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facet(id: &str) -> Facet {
        Facet {
            session_id: format!("{id}-1111-2222-3333-444444444444"),
            underlying_goal: "migrate the Angular case list".into(),
            goal_categories: vec!["feature_implementation".into()],
            outcome: "partial".into(),
            friction: vec!["repeated_tool_failures".into()],
            friction_detail: "the PowerShell build kept running out of memory".into(),
            primary_success: "landed the migration".into(),
            brief_summary: "worked on the migration".into(),
            evidence: "migrate the Angular case list".into(),
        }
    }

    fn rec(session: &str, grounded: &str) -> Recommendation {
        Recommendation {
            title: "Do the thing".into(),
            detail: "A detail long enough to clear the minimum length bar for a body.".into(),
            evidence_session: session.into(),
            grounded_in: grounded.into(),
        }
    }

    /// A recommendation citing a session the model was never shown cannot be
    /// traced back, so it is dropped rather than printed.
    #[test]
    fn an_uncited_session_is_rejected() {
        let facets = vec![facet("aaaaaaaa")];
        let material = "PowerShell build";
        let ins = Insights {
            working_style: "x".into(),
            recommendations: vec![rec("zzzzzzzz", "PowerShell")],
        };
        assert!(validate(ins, &facets, material).0.recommendations.is_empty());
    }

    /// The anti-generic check: if the thing it claims to react to does not
    /// appear in this person's material, it is advice about nobody.
    #[test]
    fn advice_not_grounded_in_the_material_is_rejected() {
        let facets = vec![facet("aaaaaaaa")];
        let material = "PowerShell build kept running out of memory";
        let ins = Insights {
            working_style: "x".into(),
            recommendations: vec![rec("aaaaaaaa", "continuous integration")],
        };
        assert!(
            validate(ins, &facets, material).0.recommendations.is_empty(),
            "generic advice must fail the grounding check"
        );
    }

    #[test]
    fn a_grounded_and_cited_recommendation_survives() {
        let facets = vec![facet("aaaaaaaa")];
        let material = "PowerShell build kept running out of memory";
        let ins = Insights {
            working_style: "x".into(),
            recommendations: vec![rec("aaaaaaaa", "PowerShell")],
        };
        assert_eq!(validate(ins, &facets, material).0.recommendations.len(), 1);
    }

    /// One decomposition recommendation is a point; two is a tic. The model
    /// produced two for two of the five sample people despite being asked for
    /// at most one, so the cap is enforced rather than requested.
    #[test]
    fn only_one_break_it_up_recommendation_survives() {
        let facets = vec![facet("aaaaaaaa")];
        let material = "PowerShell build";
        let mk = |title: &str, detail: &str| Recommendation {
            title: title.into(),
            detail: detail.into(),
            evidence_session: "aaaaaaaa".into(),
            grounded_in: "PowerShell".into(),
        };
        let ins = Insights {
            working_style: "x".into(),
            recommendations: vec![
                mk("Split the migration", "Break the PowerShell work into smaller batches here."),
                mk("Phase the refactor", "Structure it into distinct phases so each is checkable."),
                mk("Pin the build", "Set the PowerShell memory limit explicitly in the config."),
            ],
        };
        let kept = validate(ins, &facets, material).0.recommendations;
        assert_eq!(kept.len(), 2, "one decomposition plus the unrelated one");
        assert_eq!(kept[0].title, "Split the migration");
        assert_eq!(kept[1].title, "Pin the build", "the non-decomposition one is untouched");
    }

    /// The relaxed matcher has to keep doing the job the strict one did: a
    /// reworded reference to their work passes, advice about something absent
    /// from it does not.
    #[test]
    fn grounding_tolerates_rewording_but_not_a_different_subject() {
        let material = "the angular case list migration kept failing on the powershell build";
        assert!(grounded_in_material("Angular migration", material), "reworded but theirs");
        assert!(grounded_in_material("PowerShell", material));
        assert!(!grounded_in_material("continuous integration", material), "not in their work");
        assert!(!grounded_in_material("code review culture", material));
        // Half the words matching is not enough.
        assert!(!grounded_in_material("angular kubernetes helm charts", material));
    }

    /// A one-line body is a headline, not a recommendation.
    #[test]
    fn a_stub_body_is_rejected() {
        let facets = vec![facet("aaaaaaaa")];
        let mut r = rec("aaaaaaaa", "PowerShell");
        r.detail = "Do better.".into();
        let ins = Insights { working_style: "x".into(), recommendations: vec![r] };
        assert!(validate(ins, &facets, "PowerShell").0.recommendations.is_empty());
    }

    /// The brief must actually carry the specifics the model is asked to react
    /// to — if it does not, every recommendation fails grounding downstream.
    #[test]
    fn the_brief_carries_their_specifics() {
        let a = crate::metrics::analyse(&[], 0);
        let b = brief("alex", &[facet("aaaaaaaa")], &a);
        assert!(b.contains("aaaaaaaa"), "session ids for citation");
        assert!(b.contains("Angular case list"), "what they were doing");
        assert!(b.contains("PowerShell"), "what went wrong");
    }
}
