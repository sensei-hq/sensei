//! Per-cluster canonicalization (#65 step 5): one gateway call turns a cluster of
//! similar corrective prompts into a clean rule statement, an advisory suggestion,
//! and an optional link to an existing memory. Mirrors `prompt_classify`: pure
//! build/parse plus a graceful async call that degrades to `None` (the caller
//! then falls back to the cluster's representative snippet).

use gateway::Gateway;

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterSummary {
    pub text: String,
    pub suggestion: Option<String>,
    pub memory_id: Option<uuid::Uuid>,
}

const SYSTEM: &str = "You distill a cluster of a developer's repeated corrections to an AI coding agent into one durable rule. \
Given the example prompts and an optional list of existing memories, reply with ONLY a JSON object: \
{\"text\": <one-sentence canonical correction phrased as an imperative rule>, \
\"suggestion\": <one sentence on what to do about it, e.g. reinforce a memory, add a rule, or write a skill>, \
\"memory_id\": <the id of the most related memory from the list, or null>}. \
No prose, no code fences.";

/// Token budget for the summary (one short JSON object; reasoning headroom).
const MAX_TOKENS: u32 = 512;
/// Max representative prompts shown to the model per cluster.
const MAX_REPS: usize = 5;

/// Build the user message: the representative prompts + the candidate memories to
/// match against. Bounded per item to keep the request small.
pub fn build_user_message(reps: &[&str], memories: &[(uuid::Uuid, String)]) -> String {
    let mut s = String::from("Repeated corrections:\n");
    for (i, p) in reps.iter().take(MAX_REPS).enumerate() {
        let snippet: String = p.chars().take(300).collect();
        s.push_str(&format!("{}. {}\n", i + 1, snippet.replace('\n', " ")));
    }
    if memories.is_empty() {
        s.push_str("\nExisting memories: (none)\n");
    } else {
        s.push_str("\nExisting memories (id — title):\n");
        for (id, title) in memories {
            let t: String = title.chars().take(120).collect();
            s.push_str(&format!("- {} — {}\n", id, t.replace('\n', " ")));
        }
    }
    s
}

/// Parse the model's JSON object. `memory_ids` is the allowed shortlist — an id
/// outside it (or unparseable) becomes `None`. Returns `None` when there is no
/// usable `text` (caller falls back to the representative snippet). Tolerates
/// surrounding prose / code fences by extracting the first `{ … }`.
pub fn parse_response(content: &str, memory_ids: &[uuid::Uuid]) -> Option<ClusterSummary> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&content[start..=end]).ok()?;
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let suggestion = v
        .get("suggestion")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let memory_id = v
        .get("memory_id")
        .and_then(|m| m.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s.trim()).ok())
        .filter(|id| memory_ids.contains(id));
    Some(ClusterSummary { text: text.to_string(), suggestion, memory_id })
}

/// Summarize one cluster via the gateway `reasoning` chain. `None` ⇒ caller falls
/// back. Graceful: never errors out.
pub async fn summarize_cluster(
    gateway: &Gateway,
    reps: &[&str],
    memories: &[(uuid::Uuid, String)],
) -> Option<ClusterSummary> {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;
    let memory_ids: Vec<uuid::Uuid> = memories.iter().map(|(id, _)| *id).collect();
    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        chain: Some("reasoning".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, build_user_message(reps, memories))],
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
    match gateway.execute(&request).await {
        Ok(resp) if resp.success => {
            let parsed = resp.content.as_deref().and_then(|c| parse_response(c, &memory_ids));
            if parsed.is_none() {
                tracing::warn!("corrections_llm: unparseable summary — cluster falls back to snippet");
            }
            parsed
        }
        Ok(_) => None,
        Err(e) => {
            tracing::debug!(error = %e, "corrections_llm: gateway unavailable — cluster falls back");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_full_object() {
        let mid = uuid::Uuid::new_v4();
        let c = format!(
            r#"{{"text":"Use $state for reactive locals","suggestion":"Reinforce the svelte5 memory","memory_id":"{mid}"}}"#
        );
        let got = parse_response(&c, &[mid]).unwrap();
        assert_eq!(got.text, "Use $state for reactive locals");
        assert_eq!(got.suggestion.as_deref(), Some("Reinforce the svelte5 memory"));
        assert_eq!(got.memory_id, Some(mid));
    }

    #[test]
    fn parse_response_tolerates_fences_and_null_memory() {
        let c = "```json\n{\"text\":\"Revert unwanted edits\",\"suggestion\":null,\"memory_id\":null}\n```";
        let got = parse_response(c, &[]).unwrap();
        assert_eq!(got.text, "Revert unwanted edits");
        assert_eq!(got.suggestion, None);
        assert_eq!(got.memory_id, None);
    }

    #[test]
    fn parse_response_drops_memory_id_not_in_shortlist() {
        let other = uuid::Uuid::new_v4();
        let c = format!(r#"{{"text":"x","memory_id":"{other}"}}"#);
        let got = parse_response(&c, &[uuid::Uuid::new_v4()]).unwrap();
        assert_eq!(got.memory_id, None, "id outside shortlist rejected");
    }

    #[test]
    fn parse_response_none_without_text() {
        assert_eq!(parse_response(r#"{"suggestion":"do x"}"#, &[]), None);
        assert_eq!(parse_response("not json", &[]), None);
        assert_eq!(parse_response("", &[]), None);
    }

    #[test]
    fn build_user_message_bounds_and_lists_memories() {
        let id = uuid::Uuid::new_v4();
        let msg = build_user_message(&["fix it", "revert that"], &[(id, "svelte5 state".into())]);
        assert!(msg.contains("1. fix it"));
        assert!(msg.contains("2. revert that"));
        assert!(msg.contains(&id.to_string()));
        let long = "x".repeat(500);
        let msg = build_user_message(&[&long], &[]);
        // 300 snippet chars + 1 'x' from "Existing" in the memories header
        assert_eq!(msg.matches('x').count(), 301, "prompt bounded to 300 chars");
        assert!(msg.contains("(none)"));
    }
}
