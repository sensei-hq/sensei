//! Pure assembly for the Observatory · Today screen (Slot 1).
//!
//! The daemon owns every decision this screen renders — maturity, the hero
//! koan, the supporting insights, the adopted lane — so the UI stays a dumb
//! renderer (the spec: "this is a daemon decision, the screen must not
//! decide"). These functions take already-fetched data and return the wire
//! `serde_json::Value` shapes the page consumes. They are kept pure — no DB, no
//! clock — so they unit-test without a database.
//!
//! Voice: sentence case, lowercase "sensei", no filler, no marketing.

use serde_json::{Value, json};

/// Domain kanji for the hero. Early = 観 (observe), mature = 聴 (listen),
/// steady = 静 (calm) when mature but nothing is pressing.
const KANJI_EARLY: &str = "観";
const KANJI_MATURE: &str = "聴";
const KANJI_STEADY: &str = "静";

/// A pending recommendation reduced to the fields the hero + insight cards
/// need. Built by the handler from a recommendations row.
#[derive(Debug, Clone)]
pub struct RecLite {
    pub urgency: String,
    pub title: String,
    pub why: String,
    pub impact: Option<String>,
}

/// Time-of-day greeting from a 0–23 local hour.
pub fn greeting(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=16 => "Good afternoon",
        _ => "Good evening",
    }
}

/// The early-state hero. There is no teaching yet, so say so plainly — with the
/// real watched count and how many more sessions until the first lesson. The
/// action is deliberately absent (not a disabled shell) in the early state.
pub fn early_hero(watched: i64, target: i64, recent_ids: &[String]) -> Value {
    let remaining = (target - watched).max(1);
    let session_word = if watched == 1 { "session" } else { "sessions" };
    json!({
        "kanji": KANJI_EARLY,
        "koan": "Still listening.",
        "body": format!(
            "sensei has watched {watched} {session_word} so far. A few early signals are \
             forming, but nothing confident enough to teach yet."
        ),
        "impact": format!("~{remaining} more sessions until the first lesson"),
        "action": Value::Null,
        "source": recent_ids.join(" · "),
        "noticed": "since setup",
    })
}

/// The mature-state hero, built from the single most important pending
/// recommendation. `source_ids` are real session ids (extracted from the
/// recommendation's own evidence) — an empty slice yields an empty source line
/// rather than fabricated provenance.
pub fn mature_hero(top: &RecLite, source_ids: &[String], noticed: &str) -> Value {
    let impact = top.impact.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let source = if source_ids.is_empty() {
        String::new()
    } else {
        format!("from {}", source_ids.join(" · "))
    };
    json!({
        "kanji": KANJI_MATURE,
        "koan": top.title.trim(),
        "body": top.why.trim(),
        // null when the rec carries no impact → the UI hides the impact dot.
        "impact": impact,
        // A navigation call-to-action, not an Apply/Review/Dismiss disposition
        // verb — the koan CTA is exempt from the insight-card verb family, and
        // must not borrow one of those triage verbs. "Open" navigates to the
        // insights triage where this recommendation lives.
        "action": "Open insights",
        "source": source,
        "noticed": noticed,
    })
}

/// Fallback hero for the rare case of a mature project with no pending
/// recommendation to surface. Never panics the assembler.
pub fn steady_hero(recent_count: usize) -> Value {
    json!({
        "kanji": KANJI_STEADY,
        "koan": "Steady.",
        "body": format!(
            "Nothing pressing to teach today. sensei is watching your {recent_count} most \
             recent sessions and will surface the next lesson when the signal is clear."
        ),
        "impact": Value::Null,
        "action": Value::Null,
        "source": "",
        "noticed": "",
    })
}

/// Insight-card tone from a recommendation urgency. `good` is reserved for the
/// adopted lane, so recommendations only ever warn or stay muted.
pub fn insight_tone(urgency: &str) -> &'static str {
    match urgency {
        "high" => "warn",
        _ => "mute",
    }
}

/// A supporting insight card built from a recommendation.
pub fn insight_card(rec: &RecLite) -> Value {
    json!({
        "kanji": "繰",
        "label": "Recommendation",
        "text": rec.title.trim(),
        "tag": rec.urgency,
        "tone": insight_tone(&rec.urgency),
    })
}

/// The fixed early-state insight pair — what sensei is doing while it calibrates.
pub fn early_insights() -> Vec<Value> {
    vec![
        json!({
            "kanji": "耳", "label": "Listening",
            "text": "Watching your prompt and correction style. Early signals are forming.",
            "tag": "forming", "tone": "mute",
        }),
        json!({
            "kanji": "試", "label": "Calibrating",
            "text": "sensei is still learning your correction cadence. Too early to suggest rules.",
            "tag": "—", "tone": "mute",
        }),
    ]
}

/// One adopted-lane row. Trims the free-text `what` (memory titles are captured
/// from prose and can carry stray whitespace).
pub fn adopted_row(when: &str, what: &str, scope: &str, source: &str) -> Value {
    json!({
        "when": when,
        "what": what.trim(),
        "scope": scope,
        "source": source,
    })
}

/// Collect real session ids (UUID strings) out of a recommendation's `evidence`
/// json, whatever its shape — walk it recursively and keep any string that
/// parses as a UUID. Deduped, capped at `max`. Used to give the mature koan
/// honest provenance; returns empty when the evidence carries no session ids.
pub fn session_ids_from_evidence(evidence: &Value, max: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    collect_uuids(evidence, &mut out, max);
    out
}

fn collect_uuids(v: &Value, out: &mut Vec<String>, max: usize) {
    if out.len() >= max {
        return;
    }
    match v {
        Value::String(s) => {
            if uuid::Uuid::parse_str(s).is_ok() && !out.iter().any(|e| e == s) {
                out.push(s.clone());
            }
        }
        Value::Array(items) => {
            for it in items {
                collect_uuids(it, out, max);
                if out.len() >= max {
                    return;
                }
            }
        }
        Value::Object(map) => {
            for val in map.values() {
                collect_uuids(val, out, max);
                if out.len() >= max {
                    return;
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_buckets() {
        assert_eq!(greeting(5), "Good morning");
        assert_eq!(greeting(11), "Good morning");
        assert_eq!(greeting(12), "Good afternoon");
        assert_eq!(greeting(16), "Good afternoon");
        assert_eq!(greeting(17), "Good evening");
        assert_eq!(greeting(23), "Good evening");
        assert_eq!(greeting(0), "Good evening");
        assert_eq!(greeting(4), "Good evening");
    }

    #[test]
    fn early_hero_shows_real_count_and_no_action() {
        let h = early_hero(2, 3, &["s1".into(), "s2".into()]);
        assert_eq!(h["action"], Value::Null);
        assert!(h["body"].as_str().unwrap().contains("watched 2 sessions"));
        assert!(h["impact"].as_str().unwrap().contains("~1 more"));
        assert_eq!(h["source"], "s1 · s2");
        assert_eq!(h["koan"], "Still listening.");
    }

    #[test]
    fn early_hero_singular_session_word_and_min_remaining() {
        let h = early_hero(1, 3, &[]);
        assert!(h["body"].as_str().unwrap().contains("watched 1 session so far"));
        // remaining never drops below 1 even past target.
        let past = early_hero(9, 3, &[]);
        assert!(past["impact"].as_str().unwrap().contains("~1 more"));
    }

    #[test]
    fn mature_hero_koan_is_rec_title_and_action_is_nav_cta() {
        let rec = RecLite {
            urgency: "high".into(),
            title: "  Recurring corrections in sensei  ".into(),
            why: "Three sessions corrected the same thing.".into(),
            impact: Some("Projected FTR +8%".into()),
        };
        let h = mature_hero(&rec, &["u1".into()], "noticed 2 days ago");
        assert_eq!(h["koan"], "Recurring corrections in sensei"); // trimmed
        // A nav CTA, never an Apply/Review/Dismiss disposition verb.
        assert_eq!(h["action"], "Open insights");
        let action = h["action"].as_str().unwrap();
        assert!(!["Apply", "Review", "Dismiss"].iter().any(|v| action.starts_with(v)));
        assert_eq!(h["impact"], "Projected FTR +8%");
        assert_eq!(h["source"], "from u1");
        assert_eq!(h["noticed"], "noticed 2 days ago");
    }

    #[test]
    fn mature_hero_null_impact_and_empty_source_when_absent() {
        let rec = RecLite {
            urgency: "medium".into(),
            title: "Do the thing".into(),
            why: "Because.".into(),
            impact: Some("   ".into()), // blank → null
        };
        let h = mature_hero(&rec, &[], "");
        assert_eq!(h["impact"], Value::Null);
        assert_eq!(h["source"], "");
    }

    #[test]
    fn insight_tone_maps_from_urgency() {
        assert_eq!(insight_tone("high"), "warn");
        assert_eq!(insight_tone("medium"), "mute");
        assert_eq!(insight_tone("low"), "mute");
    }

    #[test]
    fn insight_card_uses_title_and_urgency() {
        let rec = RecLite {
            urgency: "high".into(),
            title: "Add an integration-test persona".into(),
            why: "…".into(),
            impact: None,
        };
        let c = insight_card(&rec);
        assert_eq!(c["text"], "Add an integration-test persona");
        assert_eq!(c["tag"], "high");
        assert_eq!(c["tone"], "warn");
    }

    #[test]
    fn early_insights_are_a_pair() {
        assert_eq!(early_insights().len(), 2);
    }

    #[test]
    fn adopted_row_maps_and_trims() {
        let r = adopted_row("2d ago", "  Prefer named tokens  ", "sensei", "from a memory");
        assert_eq!(r["when"], "2d ago");
        assert_eq!(r["what"], "Prefer named tokens");
        assert_eq!(r["scope"], "sensei");
        assert_eq!(r["source"], "from a memory");
    }

    #[test]
    fn evidence_uuid_extraction_walks_nested_json_and_caps() {
        let u1 = "11111111-1111-4111-8111-111111111111";
        let u2 = "22222222-2222-4222-8222-222222222222";
        let ev = json!({
            "sessions": [{ "session_id": u1, "note": "x" }, { "id": u2 }],
            "misc": "not-a-uuid",
        });
        let ids = session_ids_from_evidence(&ev, 3);
        assert!(ids.contains(&u1.to_string()));
        assert!(ids.contains(&u2.to_string()));
        assert_eq!(ids.len(), 2);
        // cap honored
        assert_eq!(session_ids_from_evidence(&ev, 1).len(), 1);
    }
}
