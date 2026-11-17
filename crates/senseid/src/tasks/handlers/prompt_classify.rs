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
    let content = after_thinking(content);
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
pub const INTENT_SYSTEM: &str = "You classify a developer's messages to an AI coding agent. Each item shows the end of the agent's last reply, then the developer's next message. Decide exactly one label per item by comparing the message with the reply:\n\
- \"correction\": the message pushes back on something the agent did, said, proposed or chose in that reply or in its recent work. It says it is wrong, broken, unwanted, confusing, or should have been done differently — including a mild preference against the agent's own choice (\"I don't like X\", \"we keep one per file\", \"don't mock it\"), an error the agent's change produced, a \"why did you…\", or \"that didn't fix it\".\n\
- \"principle\": a durable rule to follow from now on (\"always…\", \"never…\", \"from now on…\").\n\
- \"neither\": the message moves forward without rejecting the reply — approval, a next step, a new or follow-up change, a question, or a fix for something the agent did not do. \"go ahead\", \"rename the button\", \"now add Y\", \"can we handle null here?\" are \"neither\" when the reply did not do the thing being changed.\n\
Reply with ONLY a JSON array of strings, one label per item in order. No prose, no code fences.";

/// Stage 2's strict check, asked once per stage-1 correction.
pub const VERIFY_SYSTEM: &str = "You judge one message a developer sent to an AI coding agent, right after the agent's reply.\n\
Answer \"yes\" only if the message rejects or pushes back on the agent's OWN work in that reply or just before it: it says something the agent did, wrote, chose or claimed is wrong, broken, unwanted or not what was asked, or asks why the agent did it.\n\
Answer \"no\" if the message approves, continues, asks a question, gives the next step, or asks for a new change to something the agent's reply did not touch.\n\
Answer with one word: yes or no.";

/// How much of the agent's reply each stage sees (its end), and of the message.
const REPLY_TAIL: usize = 600;
const STAGE1_PROMPT: usize = 300;
const VERIFY_PROMPT: usize = 600;
/// Token budgets: the model thinks before it answers.
const STAGE1_MAX_TOKENS: u32 = 4096;
const VERIFY_MAX_TOKENS: u32 = 1024;

/// Label candidates by intent. One slot per input: `Some(class)` when the model
/// decided it, `None` when its batch (or its stage-2 check) got no usable answer.
pub async fn classify_intent<M: IntentModel>(
    model: &M,
    items: &[IntentCandidate<'_>],
) -> Vec<Option<PromptClass>> {
    let mut out: Vec<Option<PromptClass>> = vec![None; items.len()];
    for (ci, chunk) in items.chunks(CHUNK).enumerate() {
        let answer = model.chat(INTENT_SYSTEM, intent_message(chunk), STAGE1_MAX_TOKENS).await;
        let Some(classes) = answer.as_deref().and_then(|c| parse_response(c, chunk.len())) else {
            tracing::warn!("intent: batch got no usable answer — its turns fall back to phrases");
            continue;
        };
        for (j, class) in classes.into_iter().enumerate() {
            out[ci * CHUNK + j] = Some(class);
        }
    }
    for (item, slot) in items.iter().zip(out.iter_mut()) {
        if *slot != Some(PromptClass::Correction) {
            continue;
        }
        let verdict = model.chat(VERIFY_SYSTEM, verify_message(item), VERIFY_MAX_TOKENS).await;
        *slot = match verdict.as_deref().and_then(parse_verify) {
            Some(true) => Some(PromptClass::Correction),
            Some(false) => Some(PromptClass::Neither),
            None => {
                tracing::warn!(
                    "intent: check got no usable answer — the turn falls back to phrases"
                );
                None
            }
        };
    }
    out
}

/// The seeded chain the classifier was measured on (qwen3.8, local only).
pub const INTENT_CHAIN: &str = "intent";
/// One call's wall-clock cap: a 27B model thinking over ten messages is slow,
/// but a stalled backend must not wedge the pass.
const INTENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// The gateway request for one call: the `intent` chain, deterministic, and no
/// fallback — another model would be an unmeasured classifier.
fn intent_request(
    _system: &str,
    _user: String,
    _max_tokens: u32,
) -> gateway::types::request::InferenceRequest {
    unimplemented!()
}

/// [`IntentModel`] over the daemon's gateway.
pub struct GatewayIntentModel {
    pub gateway: std::sync::Arc<Gateway>,
}

impl IntentModel for GatewayIntentModel {
    fn chat(
        &self,
        system: &str,
        user: String,
        max_tokens: u32,
    ) -> impl std::future::Future<Output = Option<String>> + Send {
        let gateway = self.gateway.clone();
        let request = intent_request(system, user, max_tokens);
        async move {
            match tokio::time::timeout(INTENT_TIMEOUT, gateway.execute(&request)).await {
                Ok(Ok(resp)) if resp.success => resp.content.filter(|c| !c.trim().is_empty()),
                Ok(Ok(_)) => None,
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "intent: the model is unavailable");
                    None
                }
                Err(_) => {
                    tracing::warn!("intent: the model timed out");
                    None
                }
            }
        }
    }
}

/// The last `n` chars of `s`.
fn tail(s: &str, n: usize) -> String {
    let len = s.chars().count();
    s.chars().skip(len.saturating_sub(n)).collect()
}

/// The first `n` chars of `s`.
fn head(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Stage 1's message for one batch.
fn intent_message(items: &[IntentCandidate<'_>]) -> String {
    let mut s = String::from("Classify these prompts:\n");
    for (i, it) in items.iter().enumerate() {
        let reply = tail(it.prev_reply, REPLY_TAIL).replace('\n', " ");
        let prompt = head(it.prompt, STAGE1_PROMPT).replace('\n', " ");
        s.push_str(&format!("{}. [agent just said: …{reply}] DEVELOPER: {prompt}\n", i + 1));
    }
    s
}

/// Stage 2's message for one candidate.
fn verify_message(item: &IntentCandidate<'_>) -> String {
    format!(
        "AGENT'S REPLY (end): …{}\n\nDEVELOPER'S NEXT MESSAGE: {}\n\n\
         Is the developer's message a correction of the agent's own work? Answer yes or no.",
        tail(item.prev_reply, REPLY_TAIL),
        head(item.prompt, VERIFY_PROMPT)
    )
}

/// What the model answered after any thinking block it printed.
fn after_thinking(content: &str) -> &str {
    content.rsplit_once("</think>").map_or(content, |(_, answer)| answer)
}

/// Stage 2's answer: `Some(true)` for yes, `Some(false)` for no, `None` otherwise.
fn parse_verify(content: &str) -> Option<bool> {
    let answer = after_thinking(content).trim().to_lowercase();
    if answer.starts_with("yes") {
        Some(true)
    } else if answer.starts_with("no") {
        Some(false)
    } else {
        None
    }
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
        let reply = format!("{}END OF REPLY", "~".repeat(700));
        let prompt = format!("no, the other file\n{}", "^".repeat(400));
        let msg = intent_message(&[cand(&prompt, &reply), cand("go ahead", "Shall I?")]);
        assert!(msg.contains("1. [agent just said: …"), "{msg}");
        assert!(msg.contains("END OF REPLY] DEVELOPER: no, the other file "));
        assert!(msg.contains("2. [agent just said: …Shall I?] DEVELOPER: go ahead"));
        assert_eq!(msg.matches('~').count(), 600 - "END OF REPLY".len(), "reply tail is 600 chars");
        assert_eq!(msg.matches('^').count(), 300 - "no, the other file ".len(), "message is 300");
    }

    #[test]
    fn the_verify_message_quotes_the_reply_tail_and_the_message() {
        let reply = format!("{}tail", "~".repeat(700));
        let msg = verify_message(&cand("why did you drop the index?", &reply));
        assert!(msg.contains("AGENT'S REPLY (end): …"));
        assert!(msg.contains("tail\n\nDEVELOPER'S NEXT MESSAGE: why did you drop the index?"));
        assert_eq!(msg.matches('~').count(), 596);
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
    fn the_request_pins_the_intent_chain_at_temperature_zero_without_fallback() {
        use gateway::types::request::Payload;
        let r = intent_request(VERIFY_SYSTEM, "AGENT'S REPLY (end): …".into(), 1024);
        assert_eq!(r.chain.as_deref(), Some("intent"));
        assert!(!r.allow_fallback, "another model is an unmeasured classifier");
        assert!(r.model.is_none() && r.router.is_none(), "the seed picks the model");
        let Payload::Chat { system, max_tokens, temperature, messages, .. } = r.payload else {
            panic!("a chat payload");
        };
        assert_eq!(system.as_deref(), Some(VERIFY_SYSTEM));
        assert_eq!(max_tokens, Some(1024));
        assert_eq!(temperature, Some(0.0), "measured at temperature 0");
        assert_eq!(messages.len(), 1);
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
