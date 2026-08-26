//! Process-quality analyzer (spec 2026-08-20) — the first LLM-derived judgment
//! pass over the transcript corpus.
//!
//! For each measurable session that hasn't been scored yet, load its
//! `transcript_turns`, ask the embedded gateway **`reasoning`** chain (local
//! gemma4, embedded-first #79) for four process-quality judgments, and persist
//! them into `sessions.props.process` + `session_process_evidence`, stamping the
//! `process_analyzed_at` watermark. A `session_outcomes`-style aggregator later
//! rolls these into day-grain metrics.
//!
//! Discipline (spec D5/D7): every non-null judgment MUST quote a real
//! `transcript_turns.turn_index` — a judgment the model can't ground in an
//! existing turn is DROPPED, never stored. A model error / timeout / unparseable
//! response leaves the session unscored (watermark NOT advanced, retried next
//! tick) — never a fabricated judgment. Runs downstream of ingest, on the daily
//! full-refresh window; never inline with capture.

use crate::tasks::Task;
use crate::tasks::executor::TaskContext;
use crate::transcript::TranscriptTurn;

/// Sessions scored per project per tick — the batch cap so one project can't
/// dominate the queue on a hot tick. Config `process.batch_per_tick` overrides.
const DEFAULT_BATCH_PER_TICK: i64 = 25;
/// Output budget for the reasoning call. gemma4 (the reasoning chain's model) is a
/// THINKING model — it spends tokens reasoning internally BEFORE emitting the JSON,
/// so too small a budget is exhausted mid-thought and returns EMPTY content
/// (`done_reason=length`, no JSON → fail-open). Verified: 800 → empty on a real
/// transcript; 3000 → completes (`done_reason=stop`) with valid JSON. GPU-OOM
/// pressure is bounded by the INPUT caps below (turns/chars) + routing to ollama
/// (which manages its own memory), not by starving the output budget.
const MAX_TOKENS: u32 = 3000;
/// Skip transcripts with fewer than this many turns — too little to judge, and a
/// judgment would be noise. Such a session is watermarked as scored-with-nothing
/// (all-N/A), so it isn't re-read every tick.
const MIN_TURNS: usize = 2;
/// Cap turns fed to the model (head+tail window); the middle is elided with an
/// explicit marker so truncation is never silent.
const MAX_TURNS_FED: usize = 30;
/// Cap each turn's text so one giant turn can't blow the context.
const MAX_TURN_CHARS: usize = 600;
/// Hard ceiling on the WHOLE transcript prompt (chars). The embedded reasoning
/// model OOMs its GPU context on large inputs, so past this the tail is dropped
/// with an explicit marker — a belt-and-suspenders bound on top of the per-turn +
/// turn-count caps, so even pathological turns can't blow the budget.
const MAX_PROMPT_CHARS: usize = 16_000;

/// The occurrence signals — stored as `{present: bool|null}`, aggregated as
/// rates. `spec_depth` (a 0-5 magnitude) is handled separately.
/// The stage vocabulary, matching `sensei.work_stage`. A value outside this set
/// is discarded rather than coerced — a rollup that buckets an unrecognised
/// stage into a default reports a guess as a measurement.
pub(crate) const WORK_STAGES: [&str; 7] =
    ["explore", "analyze", "plan", "build", "verify", "fix", "operate"];

pub(crate) const OCCURRENCE_SIGNALS: [&str; 3] =
    ["spec_deviation", "refuted_findings", "incomplete_analysis_llm"];

const SYSTEM: &str = "You are a meticulous engineering reviewer auditing ONE AI coding session from its \
transcript. Judge only what the transcript shows; never invent. Treat the plan the assistant states in \
its OWN opening turns as the session's spec. Return ONLY a JSON object with these four keys:\n\
- spec_depth: {\"score\": <0-5 or null>, \"evidence\": [{\"turn\": int, \"quote\": str, \"kind\": \"plan\"}], \"note\": str}. \
How complete/observable the STATED plan was before implementation (acceptance criteria, inputs/outputs/deps, \
no TBDs). 5=hand-to-an-autonomous-run deep, 1=one vague line. Quote the plan turn(s). score null (and no \
evidence) if the session has NO plan-like opening.\n\
- spec_deviation: {\"present\": <true|false|null>, \"evidence\": [{\"turn\": int, \"quote\": str, \"kind\": \"plan\"|\"action\"}], \"note\": str}. \
present=true if the implementation departed from that stated plan (unplanned scope, 'instead of X do Y' pivots, \
dropped items) — quote the plan turn AND the deviating turn. present=false if it followed the plan. present=null \
if there was NO stated plan.\n\
- refuted_findings: {\"present\": <true|false>, \"evidence\": [{\"turn\": int, \"quote\": str, \"kind\": \"assertion\"|\"retraction\"}], \"note\": str}. \
present=true if the ASSISTANT asserted a finding it later reversed ITSELF (not a user correction) — quote the \
assertion turn AND the retraction turn. present=false otherwise.\n\
- incomplete_analysis_llm: {\"present\": <true|false>, \"evidence\": [{\"turn\": int, \"quote\": str}], \"note\": str}. \
present=true if the assistant built/concluded before understanding — 'I misread', 'let me actually check', \
're-read' retractions of its OWN understanding — quote the turn. present=false otherwise.\n\
- stage: one of \"explore\"|\"analyze\"|\"plan\"|\"build\"|\"verify\"|\"fix\"|\"operate\", or null. \
Which stage of the work this session was MOSTLY doing: explore=orienting in unfamiliar code, \
analyze=diagnosing a specific problem, plan=designing before implementing, build=writing new \
behaviour, verify=testing/reviewing existing behaviour, fix=repairing something known broken, \
operate=deploying/releasing/infrastructure. Use null if the session genuinely spans several with \
no dominant one — do NOT guess.\n\
RULES: whenever you set a score OR present=true, you MUST cite at least one evidence item quoting a turn index \
that EXISTS in the transcript. If you cannot quote a real turn, use score null / present=false. \
No prose outside the JSON.";

/// One session's judgments, validated + grounded. `judgments` is the jsonb
/// object stored under `props.process`; `evidence` is the flattened evidence rows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionJudgment {
    pub judgments: serde_json::Value,
    /// The stage the session was mostly doing, when the model committed to one
    /// in the vocabulary. `None` covers both "declined to say" and "said
    /// something not in the enum" — both mean the session is excluded from
    /// stage rollups rather than defaulted into a bucket.
    pub stage: Option<String>,
    pub evidence: Vec<(String, i32, String, Option<String>)>,
    /// True when at least one signal produced a real, grounded value (a depth
    /// score, or a present=true occurrence). A clean session (all present=false)
    /// is a valid result but `scored_any` is false.
    pub scored_any: bool,
}

/// Compact a session's turns into the numbered transcript the model reads. Each
/// line is `[turn <i>] user: … / assistant: …`, clipped to [`MAX_TURN_CHARS`];
/// a head+tail window past [`MAX_TURNS_FED`] elides the middle with a marker
/// (never silent). Pure.
pub(crate) fn build_transcript_prompt(turns: &[TranscriptTurn]) -> String {
    let clip = |s: &str| -> String {
        let t = s.trim();
        if t.chars().count() > MAX_TURN_CHARS {
            let mut out: String = t.chars().take(MAX_TURN_CHARS).collect();
            out.push('…');
            out
        } else {
            t.to_string()
        }
    };
    let render = |t: &TranscriptTurn, buf: &mut String| {
        if let Some(u) = t.user_text.as_deref().map(clip).filter(|s| !s.is_empty()) {
            buf.push_str(&format!("[turn {}] user: {}\n", t.turn_index, u));
        }
        let a = clip(&t.assistant_text);
        if !a.is_empty() {
            buf.push_str(&format!("[turn {}] assistant: {}\n", t.turn_index, a));
        }
    };
    let mut s = String::from("Session transcript (turn indices are stable references):\n");
    if turns.len() <= MAX_TURNS_FED {
        for t in turns {
            render(t, &mut s);
        }
    } else {
        let head = MAX_TURNS_FED * 2 / 3;
        let tail = MAX_TURNS_FED - head;
        for t in &turns[..head] {
            render(t, &mut s);
        }
        s.push_str(&format!("[… {} middle turns elided …]\n", turns.len() - head - tail));
        for t in &turns[turns.len() - tail..] {
            render(t, &mut s);
        }
    }
    // Belt-and-suspenders: hard-cap the whole prompt so a pathological transcript
    // can't OOM the embedded model. Clip to the ceiling and mark the drop (never
    // silent), preserving the head (where the plan lives) over the tail.
    if s.chars().count() > MAX_PROMPT_CHARS {
        s = s.chars().take(MAX_PROMPT_CHARS).collect();
        s.push_str("\n[… transcript truncated to fit the model context …]");
    }
    s.push_str("\nReturn the JSON object of the four judgments only.");
    s
}

/// Parse + GROUND the model output against the turns that actually exist (spec
/// D5). Extracts the outermost `{...}`; for each of the four signals, keeps the
/// score ONLY when it is a real 0-5 number AND at least one evidence item quotes a
/// `turn` present in `valid_turns`; otherwise the signal is stored as `null`
/// (N/A) with no evidence. Returns `None` when there's no JSON object at all
/// (the caller then treats the session as unscored and retries) — distinct from a
/// parsed-but-all-null result (a real "nothing to judge", which IS a valid score).
pub(crate) fn parse_and_ground(
    output: &str,
    valid_turns: &std::collections::HashSet<i32>,
) -> Option<SessionJudgment> {
    let start = output.find('{')?;
    let end = output.rfind('}')?;
    if end <= start {
        return None;
    }
    let root: serde_json::Value = serde_json::from_str(&output[start..=end]).ok()?;

    let mut judgments = serde_json::Map::new();
    let mut evidence: Vec<(String, i32, String, Option<String>)> = Vec::new();
    let mut scored_any = false;

    // Collect a signal's grounded evidence — items quoting a turn that EXISTS.
    let grounded = |obj: Option<&serde_json::Value>| -> Vec<(i32, String, Option<String>)> {
        let mut out = Vec::new();
        if let Some(items) = obj.and_then(|o| o.get("evidence")).and_then(|e| e.as_array()) {
            for it in items {
                let turn = it.get("turn").and_then(|t| t.as_i64()).map(|t| t as i32);
                let quote =
                    it.get("quote").and_then(|q| q.as_str()).unwrap_or("").trim().to_string();
                let kind = it
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty());
                if let Some(turn) = turn
                    && valid_turns.contains(&turn)
                    && !quote.is_empty()
                {
                    out.push((turn, quote, kind));
                }
            }
        }
        out
    };
    let note_of = |obj: Option<&serde_json::Value>| -> String {
        obj.and_then(|o| o.get("note")).and_then(|n| n.as_str()).unwrap_or("").trim().to_string()
    };
    let ev_json = |ev: &[(i32, String, Option<String>)]| -> Vec<serde_json::Value> {
        ev.iter().map(|(t, q, k)| serde_json::json!({ "turn": t, "quote": q, "kind": k })).collect()
    };

    // ── spec_depth: a 0-5 MAGNITUDE, kept only when grounded in a plan quote ──
    {
        let obj = root.get("spec_depth");
        let note = note_of(obj);
        let score = obj
            .and_then(|o| o.get("score"))
            .and_then(|s| s.as_f64())
            .filter(|n| (0.0..=5.0).contains(n));
        let ev = grounded(obj);
        if let Some(score) = score
            && !ev.is_empty()
        {
            scored_any = true;
            judgments.insert(
                "spec_depth".into(),
                serde_json::json!({ "score": score, "evidence": ev_json(&ev), "note": note }),
            );
            for (t, q, k) in ev {
                evidence.push(("spec_depth".into(), t, q, k));
            }
        } else {
            // No plan / ungrounded score → N/A (never a fabricated magnitude).
            judgments
                .insert("spec_depth".into(), serde_json::json!({ "score": null, "note": note }));
        }
    }

    // ── occurrence signals: `present` bool; TRUE requires grounded evidence,
    //    FALSE is a valid clean result, NULL only for spec_deviation w/o a plan ──
    for &sig in OCCURRENCE_SIGNALS.iter() {
        let obj = root.get(sig);
        let note = note_of(obj);
        // Only spec_deviation may be null (no stated plan to deviate from).
        let raw_present = obj.and_then(|o| o.get("present"));
        let is_null = sig == "spec_deviation" && raw_present.map(|v| v.is_null()).unwrap_or(false);
        if is_null {
            judgments.insert(sig.to_string(), serde_json::json!({ "present": null, "note": note }));
            continue;
        }
        let claimed = raw_present.and_then(|v| v.as_bool()).unwrap_or(false);
        let ev = grounded(obj);
        // present=true survives ONLY if grounded in ≥1 existing turn (spec D5);
        // an ungrounded "true" is coerced to false, not counted.
        if claimed && !ev.is_empty() {
            scored_any = true;
            judgments.insert(
                sig.to_string(),
                serde_json::json!({ "present": true, "evidence": ev_json(&ev), "note": note }),
            );
            for (t, q, k) in ev {
                evidence.push((sig.to_string(), t, q, k));
            }
        } else {
            judgments
                .insert(sig.to_string(), serde_json::json!({ "present": false, "note": note }));
        }
    }

    // Only a value in the vocabulary survives. A model that answers "coding" or
    // "Build " must not become a bucket nobody defined.
    let stage = root
        .get("stage")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| WORK_STAGES.contains(&s.as_str()));

    Some(SessionJudgment {
        judgments: serde_json::Value::Object(judgments),
        stage,
        evidence,
        scored_any,
    })
}

/// The all-N/A result stored for a session with too little to judge (below
/// [`MIN_TURNS`]), so it's watermarked and not re-read every tick. spec_depth
/// score null (no plan judged); the occurrence signals `present` null (not a
/// clean `false` — there was nothing to judge either way).
fn all_na() -> serde_json::Value {
    let mut m = serde_json::Map::new();
    m.insert(
        "spec_depth".into(),
        serde_json::json!({ "score": null, "note": "insufficient transcript" }),
    );
    for &sig in OCCURRENCE_SIGNALS.iter() {
        m.insert(
            sig.to_string(),
            serde_json::json!({ "present": null, "note": "insufficient transcript" }),
        );
    }
    serde_json::Value::Object(m)
}

fn batch_per_tick(cfg: Option<String>) -> i64 {
    cfg.and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_BATCH_PER_TICK)
}

/// Handler: score up to the batch cap of a project's un-scored measurable
/// sessions. `task.path` carries the project id (mirrors `AnalyzeProject`).
/// Returns the number of sessions scored (watermarked) this run — including
/// all-N/A ones, which ARE progress (they won't be retried). A model failure on
/// a session leaves it un-watermarked (retried next tick) and does not count.
pub async fn analyze_session_process(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let project_id = task.project_id()?;
    let pg = ctx.pg();
    let cap = batch_per_tick(pg.get_config("process.batch_per_tick").await.ok().flatten());

    let candidates = pg.sessions_needing_process_analysis(&project_id, cap).await?;
    if candidates.is_empty() {
        return Ok(0);
    }

    let mut scored = 0u32;
    for (session_id, client_session_id) in candidates {
        let (turns, _family) = pg.get_transcript_turns_for_session(&client_session_id).await?;
        // Too little to judge → store all-N/A + watermark (progress, not a retry).
        if turns.len() < MIN_TURNS {
            pg.save_session_process(&session_id, &client_session_id, &all_na(), &[]).await?;
            scored += 1;
            continue;
        }

        let valid_turns: std::collections::HashSet<i32> =
            turns.iter().map(|t| t.turn_index).collect();
        let user = build_transcript_prompt(&turns);

        use gateway::types::capability::Capability;
        use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};
        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: None,
            router: None,
            chain: Some("reasoning".into()),
            payload: Payload::Chat {
                messages: vec![Message::text(MessageRole::User, &user)],
                system: Some(SYSTEM.to_string()),
                max_tokens: Some(MAX_TOKENS),
                temperature: None,
                tools: Vec::new(),
            },
            budget: None,
            auth: None,
            panel: None,
            consensus: None,
            allow_fallback: true,
            credentials: std::collections::HashMap::new(),
        };
        let resp = match ctx.app_state.gateway.execute(&request).await {
            Ok(r) if r.success => r,
            Ok(_) | Err(_) => {
                // Fail-open: leave un-watermarked, retry next tick. Never fabricate.
                tracing::warn!(session = %client_session_id, "session_process: no model answered — leaving unscored (retry next tick)");
                continue;
            }
        };
        let content = resp.content.unwrap_or_default();
        let Some(result) = parse_and_ground(&content, &valid_turns) else {
            tracing::warn!(session = %client_session_id, "session_process: output did not parse — leaving unscored (retry next tick)");
            continue;
        };
        pg.save_session_process(
            &session_id,
            &client_session_id,
            &result.judgments,
            &result.evidence,
        )
        .await?;
        // Only when the model committed to a stage in the vocabulary. A session
        // it would not place stays absent from session_facets rather than
        // carrying a default that a rollup would count.
        if let Some(stage) = &result.stage {
            pg.save_session_stage(&client_session_id, stage).await?;
        }
        scored += 1;
        tracing::debug!(session = %client_session_id, scored_any = result.scored_any, evidence = result.evidence.len(), "session_process: scored");
    }

    tracing::info!(project = %project_id, scored, "session_process: scored a batch");
    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn turn(i: i32, user: Option<&str>, asst: &str) -> TranscriptTurn {
        TranscriptTurn {
            turn_index: i,
            user_text: user.map(|s| s.to_string()),
            assistant_text: asst.to_string(),
            started_at: None,
            ..Default::default()
        }
    }

    fn judgment_of(json: &str) -> Option<SessionJudgment> {
        let turns: std::collections::HashSet<i32> = [1, 2].into_iter().collect();
        parse_and_ground(json, &turns)
    }

    /// A stage in the vocabulary is kept as-is.
    #[test]
    fn a_stage_in_the_vocabulary_is_kept() {
        let j = judgment_of(r#"{"stage":"build","refuted_findings":{"present":false}}"#).unwrap();
        assert_eq!(j.stage.as_deref(), Some("build"));
    }

    /// Case and surrounding space are the model's formatting, not a different
    /// answer.
    #[test]
    fn a_stage_is_normalised_before_matching() {
        let j =
            judgment_of(r#"{"stage":"  Build ","refuted_findings":{"present":false}}"#).unwrap();
        assert_eq!(j.stage.as_deref(), Some("build"));
    }

    /// A word outside the enum must be DROPPED, not coerced. Bucketing it would
    /// put a value nobody defined into a rollup, and the DB would reject it
    /// anyway — better to notice here than to fail on insert.
    #[test]
    fn a_stage_outside_the_vocabulary_is_dropped() {
        let j = judgment_of(r#"{"stage":"coding","refuted_findings":{"present":false}}"#).unwrap();
        assert_eq!(j.stage, None, "an unrecognised stage is not a stage");
    }

    /// The prompt tells the model to answer null when no stage dominates, and
    /// that has to survive parsing as "no stage" rather than becoming one.
    #[test]
    fn a_declined_stage_stays_absent() {
        let j = judgment_of(r#"{"stage":null,"refuted_findings":{"present":false}}"#).unwrap();
        assert_eq!(j.stage, None);
        let missing = judgment_of(r#"{"refuted_findings":{"present":false}}"#).unwrap();
        assert_eq!(missing.stage, None);
    }

    /// Every value the prompt offers must be one the DB enum accepts, or the
    /// analyzer will fail on insert for a stage the model was told to use.
    #[test]
    fn the_prompt_vocabulary_matches_the_enum_vocabulary() {
        for stage in WORK_STAGES {
            assert!(
                SYSTEM.contains(stage),
                "{stage} is in the enum but never offered to the model"
            );
            let json = format!(r#"{{"stage":"{stage}","refuted_findings":{{"present":false}}}}"#);
            assert_eq!(judgment_of(&json).unwrap().stage.as_deref(), Some(stage));
        }
    }

    #[test]
    fn prompt_numbers_turns_and_elides_middle_when_long() {
        let short = vec![turn(1, Some("plan it"), "ok"), turn(2, Some("go"), "done")];
        let p = build_transcript_prompt(&short);
        assert!(p.contains("[turn 1] user: plan it"));
        assert!(p.contains("[turn 2] assistant: done"));
        assert!(!p.contains("elided"), "no elision under the cap");

        let long: Vec<TranscriptTurn> = (1..=100).map(|i| turn(i, Some("u"), "a")).collect();
        let pl = build_transcript_prompt(&long);
        assert!(
            pl.contains("middle turns elided"),
            "long transcript elides the middle, not silently"
        );
        assert!(pl.contains("[turn 1] "), "keeps the head");
        assert!(pl.contains("[turn 100] "), "keeps the tail");
    }

    #[test]
    fn prompt_hard_caps_total_chars_to_avoid_oom() {
        // Pathologically large turns (each at the per-turn clip) must not blow the
        // whole-prompt ceiling — the model OOMs otherwise. Head preserved (plan),
        // truncation marked (never silent).
        let big = "x".repeat(5_000);
        let turns: Vec<TranscriptTurn> = (1..=30).map(|i| turn(i, Some(&big), &big)).collect();
        let p = build_transcript_prompt(&turns);
        assert!(
            p.chars().count() <= super::MAX_PROMPT_CHARS + 200,
            "prompt hard-capped near the ceiling"
        );
        assert!(
            p.contains("truncated to fit the model context"),
            "truncation is marked, not silent"
        );
        assert!(p.contains("[turn 1] "), "head (plan) preserved over the tail");
    }

    #[test]
    fn depth_score_and_present_survive_only_when_grounded() {
        let valid: HashSet<i32> = [1, 2, 3].into_iter().collect();
        let out = r#"{
          "spec_depth": {"score": 4, "evidence": [{"turn": 1, "quote": "the plan is X", "kind": "plan"}], "note": "clear"},
          "spec_deviation": {"present": null, "note": "no plan"},
          "refuted_findings": {"present": true, "evidence": [{"turn": 99, "quote": "made up"}], "note": "x"},
          "incomplete_analysis_llm": {"present": true, "evidence": [], "note": "claimed but no quote"}
        }"#;
        let r = parse_and_ground(out, &valid).expect("parses");
        // spec_depth: real score + grounded plan quote → survives.
        assert_eq!(r.judgments["spec_depth"]["score"].as_f64(), Some(4.0));
        assert!(r.scored_any);
        // spec_deviation: honest null (no plan).
        assert!(r.judgments["spec_deviation"]["present"].is_null());
        // refuted_findings: present=true but evidence cites turn 99 (not real) → coerced to false (D5).
        assert_eq!(
            r.judgments["refuted_findings"]["present"].as_bool(),
            Some(false),
            "ungrounded present=true coerced to false"
        );
        // incomplete_analysis_llm: present=true with NO evidence → coerced to false (D5).
        assert_eq!(
            r.judgments["incomplete_analysis_llm"]["present"].as_bool(),
            Some(false),
            "present=true with no evidence coerced to false"
        );
        // Only the one grounded evidence row (spec_depth's plan) is persisted.
        assert_eq!(r.evidence.len(), 1);
        assert_eq!(
            r.evidence[0],
            ("spec_depth".to_string(), 1, "the plan is X".to_string(), Some("plan".to_string()))
        );
    }

    #[test]
    fn grounded_occurrence_is_flagged_with_evidence() {
        let valid: HashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        let out = r#"{
          "spec_depth": {"score": 3, "evidence": [{"turn": 1, "quote": "plan: add auth", "kind": "plan"}]},
          "spec_deviation": {"present": true, "evidence": [{"turn": 1, "quote": "plan: add auth", "kind": "plan"}, {"turn": 4, "quote": "actually refactoring the router instead", "kind": "action"}]},
          "refuted_findings": {"present": false},
          "incomplete_analysis_llm": {"present": true, "evidence": [{"turn": 3, "quote": "I misread the config"}]}
        }"#;
        let r = parse_and_ground(out, &valid).expect("parses");
        assert!(r.scored_any);
        assert_eq!(r.judgments["spec_depth"]["score"].as_f64(), Some(3.0));
        assert_eq!(
            r.judgments["spec_deviation"]["present"].as_bool(),
            Some(true),
            "grounded deviation flagged"
        );
        assert_eq!(
            r.judgments["refuted_findings"]["present"].as_bool(),
            Some(false),
            "clean = false (no evidence needed)"
        );
        assert_eq!(r.judgments["incomplete_analysis_llm"]["present"].as_bool(), Some(true));
        // Evidence rows: 1 (depth plan) + 2 (deviation pair) + 1 (incomplete) = 4.
        assert_eq!(r.evidence.len(), 4);
        assert!(r.evidence.iter().any(|(s, t, _, k)| s == "spec_deviation"
            && *t == 4
            && k.as_deref() == Some("action")));
    }

    #[test]
    fn no_json_returns_none_so_caller_retries() {
        let valid: HashSet<i32> = [1].into_iter().collect();
        assert!(parse_and_ground("the model refused", &valid).is_none());
        assert!(parse_and_ground("", &valid).is_none());
    }

    #[test]
    fn clean_session_is_a_valid_result_not_a_retry() {
        // A parseable response with a plan but no issues → depth scored, all
        // occurrences false: scored_any=true (depth), evidence just the plan quote.
        // It IS Some(..) so the caller watermarks it.
        let valid: HashSet<i32> = [1].into_iter().collect();
        let out = r#"{"spec_depth":{"score":5,"evidence":[{"turn":1,"quote":"detailed plan","kind":"plan"}]},"spec_deviation":{"present":false},"refuted_findings":{"present":false},"incomplete_analysis_llm":{"present":false}}"#;
        let r = parse_and_ground(out, &valid).expect("parses");
        assert!(r.scored_any, "depth score makes it scored");
        assert_eq!(r.judgments["spec_deviation"]["present"].as_bool(), Some(false));
        assert_eq!(r.evidence.len(), 1, "only the plan quote");
    }

    #[test]
    fn batch_cap_parses_config_with_fallback() {
        assert_eq!(batch_per_tick(None), DEFAULT_BATCH_PER_TICK);
        assert_eq!(batch_per_tick(Some("nonsense".into())), DEFAULT_BATCH_PER_TICK);
        assert_eq!(batch_per_tick(Some("0".into())), DEFAULT_BATCH_PER_TICK);
        assert_eq!(batch_per_tick(Some(" 10 ".into())), 10);
    }

    #[test]
    fn all_na_covers_every_signal() {
        let na = all_na();
        assert!(na["spec_depth"]["score"].is_null(), "spec_depth score null in all-N/A");
        for &s in OCCURRENCE_SIGNALS.iter() {
            assert!(na[s]["present"].is_null(), "{s} present null in all-N/A (not a clean false)");
        }
    }
}
