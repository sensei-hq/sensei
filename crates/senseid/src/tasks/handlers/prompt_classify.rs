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
        routing: None,
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

// ── Intent classifier (#447) ──
//
// Every user turn that follows an agent reply is a candidate, and the model —
// not a phrase list — decides whether it corrects that reply. Two stages, chosen
// by measurement (openspec `update-correction-detection-intent`, design.md):
// stage 1 labels a batch with the end of the agent's previous reply beside each
// message; stage 2 asks one strict yes/no per stage-1 correction.

/// A chat model the intent classifier asks. `None` = unavailable, timed out or
/// empty: the caller labels those turns by the phrase list, marked as such.
pub trait IntentModel {
    fn chat(
        &self,
        system: &str,
        user: String,
        max_tokens: u32,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

/// One candidate: the user's message and the agent reply it answers.
#[derive(Debug, Clone, Copy)]
pub struct IntentCandidate<'a> {
    pub prompt: &'a str,
    pub prev_reply: &'a str,
}

/// Stage 1's definition: compare the message with the reply it answers.
pub const INTENT_SYSTEM: &str = "";

/// Stage 2's strict check, asked once per stage-1 correction.
pub const VERIFY_SYSTEM: &str = "";

/// Label candidates by intent. One slot per input: `Some(class)` when the model
/// decided it, `None` when its batch (or its stage-2 check) got no usable answer.
pub async fn classify_intent<M: IntentModel>(
    _model: &M,
    items: &[IntentCandidate<'_>],
) -> Vec<Option<PromptClass>> {
    vec![None; items.len()]
}

/// Stage 1's message for one batch.
fn intent_message(_items: &[IntentCandidate<'_>]) -> String {
    String::new()
}

/// Stage 2's message for one candidate.
fn verify_message(_item: &IntentCandidate<'_>) -> String {
    String::new()
}

/// Stage 2's answer: `Some(true)` for yes, `Some(false)` for no, `None` otherwise.
fn parse_verify(_content: &str) -> Option<bool> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model that answers from a script: stage-1 batches in order, then
    /// stage-2 checks by the message they quote. Records every call.
    struct Scripted {
        batches: std::sync::Mutex<std::collections::VecDeque<Option<String>>>,
        verdicts: Vec<(&'static str, Option<&'static str>)>,
        calls: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl Scripted {
        fn new(
            batches: Vec<Option<&str>>,
            verdicts: Vec<(&'static str, Option<&'static str>)>,
        ) -> Self {
            Self {
                batches: std::sync::Mutex::new(
                    batches.into_iter().map(|b| b.map(str::to_string)).collect(),
                ),
                verdicts,
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl IntentModel for Scripted {
        fn chat(
            &self,
            system: &str,
            user: String,
            _max_tokens: u32,
        ) -> impl std::future::Future<Output = Option<String>> + Send {
            self.calls.lock().unwrap().push((system.to_string(), user.clone()));
            let answer = if system == VERIFY_SYSTEM {
                self.verdicts
                    .iter()
                    .find(|(quoted, _)| user.contains(quoted))
                    .and_then(|(_, v)| v.map(str::to_string))
            } else {
                self.batches.lock().unwrap().pop_front().flatten()
            };
            async move { answer }
        }
    }

    fn cand<'a>(prompt: &'a str, prev_reply: &'a str) -> IntentCandidate<'a> {
        IntentCandidate { prompt, prev_reply }
    }

    #[tokio::test]
    async fn stage_two_keeps_confirmed_corrections_and_clears_the_rest() {
        let items = [
            cand("Hmm, I meant the other config file", "I edited config/app.toml."),
            cand("fix the lint errors in the parser", "Tests pass."),
            cand("always run the tests first", "Done."),
        ];
        let model = Scripted::new(
            vec![Some(r#"["correction","correction","principle"]"#)],
            vec![("I meant the other config", Some("yes")), ("fix the lint errors", Some("no"))],
        );
        let labels = classify_intent(&model, &items).await;
        assert_eq!(
            labels,
            vec![
                Some(PromptClass::Correction),
                Some(PromptClass::Neither),
                Some(PromptClass::Principle)
            ]
        );
        let calls = model.calls.lock().unwrap();
        assert_eq!(calls.len(), 3, "one batch + one check per stage-1 correction");
        assert_eq!(calls[0].0, INTENT_SYSTEM);
        assert!(calls[1..].iter().all(|(s, _)| s == VERIFY_SYSTEM));
    }

    #[tokio::test]
    async fn a_failed_batch_or_check_is_unknown_not_neither() {
        let items: Vec<IntentCandidate> =
            (0..11).map(|_| cand("that didn't fix it", "I changed the loop.")).collect();
        // Batch one (10) answers; batch two (1) gets nothing. Every stage-2 check fails.
        let model = Scripted::new(
            vec![
                Some(
                    r#"["correction","neither","neither","neither","neither","neither","neither","neither","neither","neither"]"#,
                ),
                None,
            ],
            vec![("that didn't fix it", None)],
        );
        let labels = classify_intent(&model, &items).await;
        assert_eq!(labels[0], None, "an unanswered check leaves the turn unknown");
        assert!(labels[1..10].iter().all(|l| *l == Some(PromptClass::Neither)));
        assert_eq!(labels[10], None, "an unanswered batch leaves its turns unknown");
    }

    #[test]
    fn the_intent_message_puts_the_reply_tail_beside_each_message() {
        let reply = format!("{}END OF REPLY", "x".repeat(700));
        let prompt = format!("no, the other file\n{}", "y".repeat(400));
        let msg = intent_message(&[cand(&prompt, &reply), cand("go ahead", "Shall I?")]);
        assert!(msg.contains("1. [agent just said: …"), "{msg}");
        assert!(msg.contains("END OF REPLY] DEVELOPER: no, the other file "));
        assert!(msg.contains("2. [agent just said: …Shall I?] DEVELOPER: go ahead"));
        assert_eq!(msg.matches('x').count(), 600 - "END OF REPLY".len(), "reply tail is 600 chars");
        assert_eq!(msg.matches('y').count(), 300 - "no, the other file ".len(), "message is 300");
    }

    #[test]
    fn the_verify_message_quotes_the_reply_tail_and_the_message() {
        let reply = format!("{}tail", "x".repeat(700));
        let msg = verify_message(&cand("why did you drop the index?", &reply));
        assert!(msg.contains("AGENT'S REPLY (end): …"));
        assert!(msg.contains("tail\n\nDEVELOPER'S NEXT MESSAGE: why did you drop the index?"));
        assert_eq!(msg.matches('x').count(), 596);
    }

    #[test]
    fn parse_verify_reads_yes_or_no_after_any_thinking() {
        assert_eq!(parse_verify("Yes."), Some(true));
        assert_eq!(parse_verify(" no"), Some(false));
        assert_eq!(parse_verify("<think>is it? no idea</think>\nyes"), Some(true));
        assert_eq!(parse_verify("maybe"), None);
        assert_eq!(parse_verify(""), None);
    }

    #[test]
    fn parse_response_skips_a_thinking_block() {
        let c = "<think>labels like [\"x\"] would be wrong</think>[\"correction\"]";
        assert_eq!(parse_response(c, 1), Some(vec![PromptClass::Correction]));
    }

    #[test]
    fn the_prompts_carry_the_measured_definitions() {
        assert!(INTENT_SYSTEM.contains("comparing the message with the reply"));
        assert!(INTENT_SYSTEM.contains("a fix for something the agent did not do"));
        assert!(VERIFY_SYSTEM.contains("the agent's OWN work"));
        assert!(VERIFY_SYSTEM.ends_with("Answer with one word: yes or no."));
    }

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
