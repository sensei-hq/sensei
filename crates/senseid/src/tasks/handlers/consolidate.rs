//! L2 Generator — consolidation tier (F5/#70).
//!
//! Where the heuristic tier ([`super::generate`]) maps each pattern to an
//! action deterministically, the consolidation tier batches a project's
//! strongest findings through the gateway **`reasoning`** chain (the
//! "consolidation" inference role; embedded-first per #79) and asks for one
//! synthesized, higher-value recommendation. The model's output is recorded as
//! an `inference.reasoning_traces` row and a recommendation linked to it via
//! `reasoning_trace_id`, with an LLM-written `why`/`prompt`.
//!
//! Degrades cleanly: if no chat model answers (or the output won't parse), it
//! logs and writes nothing — the heuristic recs are unaffected.
//!
//! Idempotency: the set of findings is hashed into a signature stored on the
//! trace's `trigger_detail`; a project that already has a consolidation trace
//! for the current finding-set is skipped, so the (expensive) LLM call only
//! fires when the strongest signals actually change.

use crate::tasks::executor::TaskContext;
use sha2::{Digest, Sha256};

/// Minimum number of findings before consolidation is worth an LLM call.
const CONSOLIDATION_MIN: usize = 3;
/// How many of the strongest findings to feed the model.
const TOP_N: usize = 8;
/// Token budget — `reasoning` models (gemma4) spend tokens thinking before output.
const MAX_TOKENS: u32 = 1024;
/// Canonical recommendation action types (must match the generator + DDL comment).
const ACTION_TYPES: [&str; 8] = [
    "promote_pattern", "create_agent", "write_skill", "archive_memory",
    "enrich_memory", "cross_project", "revise_rule", "audit_stale",
];

const SYSTEM: &str = "You are a senior engineer reviewing recurring signals from a developer's \
coding sessions. Synthesize them into ONE highest-value, actionable recommendation. \
Respond with ONLY a JSON object: {\"headline\": str, \"reasoning\": str, \"action_type\": one of \
[promote_pattern, create_agent, write_skill, archive_memory, enrich_memory, cross_project, revise_rule, audit_stale], \
\"prompt\": str (a ready-to-send instruction for an AI coding assistant), \"urgency\": one of [low, medium, high]}. \
No prose outside the JSON.";

/// A finding fed to the consolidation model.
#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub pattern_id: uuid::Uuid,
    pub name: String,
    pub folder_label: String,
    pub instance_count: i32,
}

/// The model's synthesized output (validated).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConsolidatedInsight {
    pub headline: String,
    pub reasoning: String,
    pub action_type: String,
    pub prompt: String,
    pub urgency: String,
}

/// Stable signature for a finding-set — order-independent hash of the pattern
/// ids, so re-running with the same findings produces the same key.
pub(crate) fn findings_signature(findings: &[Finding]) -> String {
    let mut ids: Vec<String> = findings.iter().map(|f| f.pattern_id.to_string()).collect();
    ids.sort();
    let mut hasher = Sha256::new();
    hasher.update(ids.join(",").as_bytes());
    hex::encode(hasher.finalize())
}

/// Build the user message summarizing the findings for the model.
pub(crate) fn build_consolidation_prompt(findings: &[Finding]) -> String {
    let mut s = String::from("Recurring signals from recent sessions:\n");
    for f in findings {
        let kind = if f.name.starts_with("rework: ") {
            "re-edit churn"
        } else if f.name == "correction-prone" {
            "recurring corrections"
        } else if f.name == "rule-candidates" {
            "stated principle"
        } else {
            "signal"
        };
        s.push_str(&format!(
            "- {kind} in {} ({}; {} occurrence(s))\n",
            f.folder_label, f.name, f.instance_count
        ));
    }
    s.push_str("\nWhat is the single highest-value action? Respond with the JSON object only.");
    s
}

/// Normalize a model-supplied action type to the canonical set, defaulting to
/// `revise_rule` for anything unrecognized.
fn normalize_action_type(raw: &str) -> String {
    let t = raw.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    if ACTION_TYPES.contains(&t.as_str()) {
        t
    } else {
        "revise_rule".to_string()
    }
}

fn normalize_urgency(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => "low",
        "high" => "high",
        _ => "medium",
    }
    .to_string()
}

/// Parse the model output into a validated [`ConsolidatedInsight`]. Tolerant of
/// surrounding prose: extracts the outermost `{...}` and parses that. Returns
/// `None` if there's no JSON object or the required fields are empty.
pub(crate) fn parse_consolidation(output: &str) -> Option<ConsolidatedInsight> {
    let start = output.find('{')?;
    let end = output.rfind('}')?;
    if end <= start {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(&output[start..=end]).ok()?;
    let get = |k: &str| json.get(k).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let headline = get("headline");
    let reasoning = get("reasoning");
    let prompt = get("prompt");
    if headline.is_empty() || reasoning.is_empty() || prompt.is_empty() {
        return None;
    }
    Some(ConsolidatedInsight {
        headline,
        reasoning,
        action_type: normalize_action_type(&get("action_type")),
        prompt,
        urgency: normalize_urgency(&get("urgency")),
    })
}

/// Handler step: consolidate a project's strongest findings into one
/// LLM-reasoned recommendation + a reasoning trace. Returns 1 if a new
/// consolidation was written, 0 otherwise (too few findings, already done,
/// no model, or unparseable output).
pub async fn consolidate_for_project(
    ctx: &TaskContext,
    project_id: &uuid::Uuid,
) -> Result<u32, String> {
    let mut findings: Vec<Finding> = ctx
        .pg()
        .get_patterns_for_generation(project_id)
        .await?
        .into_iter()
        .map(|(id, _folder_id, folder_label, name, _is_anti, instance_count, _instances)| Finding {
            pattern_id: id,
            name,
            folder_label,
            instance_count,
        })
        .collect();
    findings.truncate(TOP_N);
    if findings.len() < CONSOLIDATION_MIN {
        return Ok(0);
    }

    let signature = findings_signature(&findings);
    if ctx
        .pg()
        .reasoning_trace_exists_with_signature(project_id, &signature)
        .await
        .unwrap_or(false)
    {
        return Ok(0); // already consolidated this finding-set
    }

    // Run the reasoning chain.
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};
    let user = build_consolidation_prompt(&findings);
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
            tracing::warn!(project = %project_id, "consolidate: no model answered — skipping (heuristic recs unaffected)");
            return Ok(0);
        }
    };
    let content = resp.content.unwrap_or_default();
    let Some(insight) = parse_consolidation(&content) else {
        tracing::warn!(project = %project_id, "consolidate: model output did not parse — skipping");
        return Ok(0);
    };
    // Record the REAL model provenance, or none — never a fabricated "unknown",
    // which would misrepresent which model produced this persisted recommendation.
    let models: Vec<String> = resp.model.into_iter().collect();

    // Persist the trace + a linked recommendation.
    let pattern_ids: Vec<String> = findings.iter().map(|f| f.pattern_id.to_string()).collect();
    let exchanges = serde_json::json!([
        { "role": "user", "content": user },
        { "role": "assistant", "content": content },
    ]);
    let consensus = serde_json::json!({ "conclusion": insight.headline });
    let trigger_detail = serde_json::json!({ "signature": signature, "patterns": pattern_ids });
    let trace_id = ctx
        .pg()
        .insert_reasoning_trace(Some(project_id), "consolidation", &trigger_detail, &models, &exchanges, &consensus)
        .await?;

    let based_on = serde_json::json!({ "patterns": pattern_ids });
    ctx.pg()
        .create_recommendation_full(
            project_id,
            &insight.headline,
            &insight.reasoning,
            None,
            &insight.action_type,
            &insight.urgency,
            &based_on,
            Some(&trace_id),
            Some(&insight.prompt),
        )
        .await?;

    tracing::info!(project = %project_id, "consolidate: wrote 1 reasoned recommendation");
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(name: &str, count: i32) -> Finding {
        Finding {
            pattern_id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            folder_label: "src/api".to_string(),
            instance_count: count,
        }
    }

    #[test]
    fn signature_is_order_independent_and_stable() {
        let a = finding("correction-prone", 3);
        let b = finding("rework: x.rs", 5);
        let s1 = findings_signature(&[a.clone(), b.clone()]);
        let s2 = findings_signature(&[b, a]);
        assert_eq!(s1, s2, "order-independent");
        assert_eq!(s1.len(), 64, "sha256 hex");
    }

    #[test]
    fn prompt_summarizes_each_finding_kind() {
        let p = build_consolidation_prompt(&[
            finding("rework: src/x.rs", 6),
            finding("correction-prone", 3),
        ]);
        assert!(p.contains("re-edit churn"));
        assert!(p.contains("recurring corrections"));
        assert!(p.contains("src/api"));
        assert!(p.contains("JSON"));
    }

    #[test]
    fn parse_clean_json() {
        let out = r#"{"headline":"Add an auth guard","reasoning":"corrections cluster in auth","action_type":"write_skill","prompt":"Write a skill that...","urgency":"high"}"#;
        let i = parse_consolidation(out).expect("parse");
        assert_eq!(i.headline, "Add an auth guard");
        assert_eq!(i.action_type, "write_skill");
        assert_eq!(i.urgency, "high");
        assert!(i.prompt.starts_with("Write a skill"));
    }

    #[test]
    fn parse_json_embedded_in_prose() {
        let out = "Here is my analysis:\n{\"headline\":\"H\",\"reasoning\":\"R\",\"action_type\":\"promote-pattern\",\"prompt\":\"P\",\"urgency\":\"weird\"}\nHope that helps.";
        let i = parse_consolidation(out).expect("parse");
        assert_eq!(i.action_type, "promote_pattern", "dash normalized");
        assert_eq!(i.urgency, "medium", "unknown urgency defaults");
    }

    #[test]
    fn parse_unknown_action_defaults_to_revise_rule() {
        let out = r#"{"headline":"H","reasoning":"R","action_type":"frobnicate","prompt":"P","urgency":"low"}"#;
        assert_eq!(parse_consolidation(out).unwrap().action_type, "revise_rule");
    }

    #[test]
    fn parse_rejects_missing_fields_and_non_json() {
        assert!(parse_consolidation("no json here").is_none());
        assert!(parse_consolidation(r#"{"headline":"","reasoning":"r","prompt":"p"}"#).is_none(), "empty headline");
        assert!(parse_consolidation(r#"{"headline":"h","reasoning":"r"}"#).is_none(), "missing prompt");
    }
}
