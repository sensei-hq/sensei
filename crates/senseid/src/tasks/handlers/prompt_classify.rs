//! L2 prompt classifier (#70): refine the cheap regex correction/principle
//! signals with one LLM pass over the candidate prompts. Recall comes from the
//! regex pre-filter (`correction_signal`/`principle_signal`), precision from the
//! model — it drops false positives and fixes correction↔principle mislabels.
//! Graceful: returns `None` (caller falls back to the regex class) whenever the
//! gateway has no chat model configured or the call/parse fails, so the analyzer
//! never depends on the LLM being available.

use gateway::Gateway;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptClass {
    Correction,
    Principle,
    Neither,
}

const SYSTEM: &str = "You classify a developer's prompts to an AI coding agent. For each numbered prompt decide exactly one label:\n\
- \"correction\": the user is correcting, rejecting, or redirecting the agent's work (e.g. 'no, revert that', 'why did you do X', \"that's wrong\", \"doesn't work\").\n\
- \"principle\": the user states a durable rule or preference to follow going forward (e.g. 'always run the tests first', 'never edit generated files', 'make sure to use X').\n\
- \"neither\": a normal feature request, question, or one-off instruction that is not a correction or a durable rule.\n\
Reply with ONLY a JSON array of strings, one label per prompt in order. No prose, no code fences.";

/// Build the user message listing the candidate prompts (bounded per prompt to
/// keep the request small).
fn build_user_message(prompts: &[&str]) -> String {
    let mut s = String::from("Classify these prompts:\n");
    for (i, p) in prompts.iter().enumerate() {
        let snippet: String = p.chars().take(300).collect();
        s.push_str(&format!("{}. {}\n", i + 1, snippet.replace('\n', " ")));
    }
    s
}

/// Parse the model's JSON array of labels. Tolerates surrounding prose / code
/// fences by extracting the first `[ … ]`. Returns `None` unless the array
/// length matches the input count (so a malformed reply falls back to regex).
fn parse_response(content: &str, n: usize) -> Option<Vec<PromptClass>> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;
    if end <= start {
        return None;
    }
    let labels: Vec<String> = serde_json::from_str(&content[start..=end]).ok()?;
    if labels.len() != n {
        return None;
    }
    Some(
        labels
            .iter()
            .map(|s| match s.trim().to_lowercase().as_str() {
                "correction" => PromptClass::Correction,
                "principle" => PromptClass::Principle,
                _ => PromptClass::Neither,
            })
            .collect(),
    )
}

/// Max prompts per LLM call. Small batches keep each response bounded + reliable
/// — the chat model reasons before emitting the JSON array, and a long array
/// risks truncation past the token budget.
const CHUNK: usize = 10;
/// Token budget per chunk — headroom for a reasoning model's thinking plus the
/// label array (gemma4 e.g. spends ~200+ tokens reasoning before the answer).
const MAX_TOKENS: u32 = 1024;

/// Classify candidate prompts via the gateway, in bounded chunks. Returns one
/// slot per input: `Some(class)` when the model classified it, `None` when that
/// chunk was unavailable / unparseable (caller falls back to the regex class).
/// Pins the `text_chat` chain (lightweight) so it's unambiguous vs `reasoning`.
pub async fn classify_batch(gateway: &Gateway, prompts: &[&str]) -> Vec<Option<PromptClass>> {
    let mut out: Vec<Option<PromptClass>> = vec![None; prompts.len()];
    for (ci, chunk) in prompts.chunks(CHUNK).enumerate() {
        if let Some(classes) = classify_chunk(gateway, chunk).await {
            let base = ci * CHUNK;
            for (j, c) in classes.into_iter().enumerate() {
                out[base + j] = Some(c);
            }
        }
    }
    out
}

/// Classify one chunk. `None` ⇒ caller falls back to regex for these prompts.
async fn classify_chunk(gateway: &Gateway, prompts: &[&str]) -> Option<Vec<PromptClass>> {
    if prompts.is_empty() {
        return Some(Vec::new());
    }
    use gateway::types::capability::Capability;
    use gateway::types::request::*;

    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        // The L2 prompt classifier is lightweight classification — pin the
        // seed `classify` chain (embedded → ollama → cloud), not a
        // bare/code-only chain name (#76).
        chain: Some("classify".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, build_user_message(prompts))],
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
            let classes = resp.content.as_deref().and_then(|c| parse_response(c, prompts.len()));
            if classes.is_none() {
                tracing::warn!(
                    "prompt_classify: unparseable/short LLM response — chunk falls back to regex"
                );
            }
            classes
        }
        Ok(_) => None,
        Err(e) => {
            tracing::debug!(error = %e, "prompt_classify: gateway unavailable — chunk falls back to regex");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_maps_labels_in_order() {
        let c = r#"["correction","principle","neither"]"#;
        assert_eq!(
            parse_response(c, 3),
            Some(vec![PromptClass::Correction, PromptClass::Principle, PromptClass::Neither])
        );
    }

    #[test]
    fn parse_response_tolerates_prose_and_fences() {
        let c = "Here you go:\n```json\n[\"neither\", \"correction\"]\n```";
        assert_eq!(parse_response(c, 2), Some(vec![PromptClass::Neither, PromptClass::Correction]));
    }

    #[test]
    fn parse_response_rejects_length_mismatch_and_garbage() {
        assert_eq!(parse_response(r#"["correction"]"#, 2), None, "len mismatch ⇒ fall back");
        assert_eq!(parse_response("not json at all", 1), None);
        assert_eq!(parse_response("", 1), None);
    }

    #[test]
    fn unknown_labels_map_to_neither() {
        assert_eq!(
            parse_response(r#"["bogus","Correction"]"#, 2),
            Some(vec![PromptClass::Neither, PromptClass::Correction])
        );
    }

    #[test]
    fn build_user_message_numbers_and_bounds_prompts() {
        let msg = build_user_message(&["fix it", "add a page"]);
        assert!(msg.contains("1. fix it"));
        assert!(msg.contains("2. add a page"));
        let long = "x".repeat(500);
        let msg = build_user_message(&[&long]);
        // 300-char cap + "1. " prefix + newlines
        assert!(msg.matches('x').count() == 300, "prompt bounded to 300 chars");
    }
}
