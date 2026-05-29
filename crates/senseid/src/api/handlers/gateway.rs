use axum::extract::State;
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::state::AppState;
use gateway::purpose::*;
use gateway::types::capability::Capability;

/// GET /api/gateway/status — returns registered adapters and configuration state.
pub(crate) async fn gateway_status(State(state): State<AppState>) -> Json<Value> {
    let adapters = state.gateway.list_adapters().await;
    let configured = state.gateway.is_configured().await;
    Json(json!({
        "status": if configured { "ready" } else { "not_configured" },
        "configured": configured,
        "adapters": adapters,
    }))
}

// ── Inference types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct MessageInput {
    pub role:    String,
    pub content: String,
}

#[derive(Deserialize)]
pub(crate) struct InferRequest {
    pub capability:   String,
    pub prompt:       Option<String>,
    pub messages:     Option<Vec<MessageInput>>,
    pub system:       Option<String>,
    pub texts:        Option<Vec<String>>,
    pub model:        Option<String>,
    pub max_tokens:   Option<u32>,
}

#[derive(Deserialize)]
pub(crate) struct EmbedRequest {
    pub texts: Vec<String>,
    pub model: Option<String>,
}

// ── POST /api/gateway/infer ─────────────────────────────────────────────

pub(crate) async fn infer(
    State(state): State<AppState>,
    Json(body): Json<InferRequest>,
) -> Json<Value> {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;

    // Parse capability from string
    let capability = match body.capability.as_str() {
        "text_chat"     => Capability::TextChat,
        "text_complete" => Capability::TextComplete,
        "text_embed"    => Capability::TextEmbed,
        other => {
            return Json(json!({"error": format!("unsupported capability: {}", other)}));
        }
    };

    // Build messages
    let messages = if let Some(msgs) = body.messages {
        msgs.iter()
            .map(|m| {
                let role = match m.role.as_str() {
                    "system" => MessageRole::System,
                    "assistant" => MessageRole::Assistant,
                    "tool" => MessageRole::Tool,
                    _ => MessageRole::User,
                };
                Message::text(role, m.content.clone())
            })
            .collect()
    } else if let Some(prompt) = &body.prompt {
        vec![Message::text(MessageRole::User, prompt.clone())]
    } else {
        vec![]
    };

    // Build payload
    let payload = match capability {
        Capability::TextEmbed => {
            let texts = body.texts.unwrap_or_else(|| {
                body.prompt.map(|p| vec![p]).unwrap_or_default()
            });
            Payload::Embed { texts }
        }
        _ => Payload::Chat {
            messages,
            system: body.system,
            max_tokens: body.max_tokens,
            temperature: None,
            tools: Vec::new(),
        },
    };

    let request = InferenceRequest {
        capability,
        model: body.model,
        router: None,
        chain: None,
        payload,
        budget: None,
    };

    match state.gateway.execute(&request).await {
        Ok(response) => Json(json!({
            "success": response.success,
            "content": response.content,
            "embeddings": response.embeddings,
            "model": response.model,
            "usage": response.usage,
        })),
        Err(e) => Json(json!({
            "error": e.to_string(),
        })),
    }
}

// ── POST /api/gateway/embed ─────────────────────────────────────────────

pub(crate) async fn embed(
    State(state): State<AppState>,
    Json(body): Json<EmbedRequest>,
) -> Json<Value> {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;

    let request = InferenceRequest {
        capability: Capability::TextEmbed,
        model: body.model,
        router: None,
        chain: None,
        payload: Payload::Embed { texts: body.texts },
        budget: None,
    };

    match state.gateway.execute(&request).await {
        Ok(response) => Json(json!({
            "success": response.success,
            "embeddings": response.embeddings,
            "model": response.model,
        })),
        Err(e) => Json(json!({
            "error": e.to_string(),
        })),
    }
}

// ── MOE Consensus ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ConsensusRequest {
    /// The signal or question to analyze
    pub signal: String,
    /// Additional context (project state, metrics, etc.)
    pub context: Option<String>,
    /// Override proposer model
    pub proposer_model: Option<String>,
    /// Override challenger model
    pub challenger_model: Option<String>,
    /// Override synthesizer model
    pub synthesizer_model: Option<String>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub(crate) struct ConsensusResponse {
    pub conclusion: String,
    pub confidence: String,
    pub proposer_output: String,
    pub challenger_output: String,
    pub synthesizer_output: String,
    pub total_duration_ms: u64,
}

/// POST /api/gateway/consensus — run a 3-model MOE debate.
pub(crate) async fn consensus(
    State(state): State<AppState>,
    Json(body): Json<ConsensusRequest>,
) -> Json<Value> {
    let context_section = body
        .context
        .as_ref()
        .map(|c| format!("\n\nContext:\n{c}"))
        .unwrap_or_default();

    let signal_with_context = format!("{}{}", body.signal, context_section);

    let purpose = build_consensus_purpose(
        body.proposer_model.as_deref(),
        body.challenger_model.as_deref(),
        body.synthesizer_model.as_deref(),
    );

    match execute_purpose(&state.gateway, &purpose, &signal_with_context).await {
        Ok(result) => {
            let proposer_output = result
                .steps
                .first()
                .map(|s| s.output.clone())
                .unwrap_or_default();
            let challenger_output = result
                .steps
                .get(1)
                .map(|s| s.output.clone())
                .unwrap_or_default();
            let synthesizer_output = result
                .steps
                .get(2)
                .map(|s| s.output.clone())
                .unwrap_or_default();

            let confidence = extract_confidence(&synthesizer_output);

            Json(json!({
                "conclusion": result.output,
                "confidence": confidence,
                "proposer_output": proposer_output,
                "challenger_output": challenger_output,
                "synthesizer_output": synthesizer_output,
                "total_duration_ms": result.total_duration_ms,
                "steps": result.steps.len(),
                "models_used": result.steps.iter()
                    .filter_map(|s| s.model_used.as_ref())
                    .collect::<Vec<_>>(),
            }))
        }
        Err(e) => Json(json!({
            "error": e.to_string(),
        })),
    }
}

pub(crate) fn build_consensus_purpose(
    proposer_model: Option<&str>,
    challenger_model: Option<&str>,
    synthesizer_model: Option<&str>,
) -> Purpose {
    let proposer_hint = proposer_model
        .map(|m| ModelHint::Specific(m.to_string()))
        .unwrap_or(ModelHint::Best);
    let challenger_hint = challenger_model
        .map(|m| ModelHint::Specific(m.to_string()))
        .unwrap_or(ModelHint::Balanced);
    let synthesizer_hint = synthesizer_model
        .map(|m| ModelHint::Specific(m.to_string()))
        .unwrap_or(ModelHint::Best);

    PurposeBuilder::new("moe_consensus")
        .description("Multi-model debate: propose \u{2192} challenge \u{2192} synthesize consensus")
        .step(
            StepBuilder::new(Capability::TextChat)
                .system(
                    "You are the PROPOSER in a multi-model consensus protocol. \
                     Analyze the following signal carefully. \
                     Propose a root cause analysis and recommended action. \
                     Structure your response as:\n\
                     ROOT CAUSE: <your analysis>\n\
                     EVIDENCE: <supporting observations>\n\
                     RECOMMENDED ACTION: <specific action to take>\n\
                     CONFIDENCE: <high/medium/low with reasoning>",
                )
                .model_hint(proposer_hint)
                .build(),
        )
        .step(
            StepBuilder::new(Capability::TextChat)
                .system(
                    "You are the CHALLENGER in a multi-model consensus protocol. \
                     You are reviewing the PROPOSER's analysis below. \
                     Your job is to:\n\
                     1. Identify weaknesses or gaps in the analysis\n\
                     2. Challenge assumptions with alternative explanations\n\
                     3. Refine the recommendation if needed\n\
                     4. Note points you AGREE with and points you DISAGREE with\n\n\
                     Structure your response as:\n\
                     AGREEMENTS: <what you agree with>\n\
                     CHALLENGES: <what you disagree with or see differently>\n\
                     ALTERNATIVE EXPLANATION: <if applicable>\n\
                     REFINED ACTION: <your revised recommendation>",
                )
                .model_hint(challenger_hint)
                .input(StepInput::Template(
                    "ORIGINAL SIGNAL:\n{input}\n\n---\n\nPROPOSER'S ANALYSIS:\n{0}".to_string(),
                ))
                .build(),
        )
        .step(
            StepBuilder::new(Capability::TextChat)
                .system(
                    "You are the SYNTHESIZER in a multi-model consensus protocol. \
                     You have the original signal, the PROPOSER's analysis, and the \
                     CHALLENGER's response. Your job is to:\n\
                     1. Identify points of consensus (where both agree)\n\
                     2. Resolve disagreements with your own judgment\n\
                     3. Produce a final conclusion and action\n\
                     4. Rate overall confidence\n\n\
                     Structure your response as:\n\
                     CONSENSUS POINTS: <where models agree>\n\
                     RESOLVED DISAGREEMENTS: <how you resolved conflicts>\n\
                     FINAL CONCLUSION: <your synthesis>\n\
                     RECOMMENDED ACTION: <final action>\n\
                     CONFIDENCE: high|medium|low \u{2014} <reasoning>",
                )
                .model_hint(synthesizer_hint)
                .input(StepInput::Template(
                    "ORIGINAL SIGNAL:\n{input}\n\n---\n\nPROPOSER'S ANALYSIS:\n{0}\n\n---\n\n\
                     CHALLENGER'S RESPONSE:\n{1}"
                        .to_string(),
                ))
                .build(),
        )
        .build()
}

/// Extract confidence level from synthesizer output by looking for the
/// CONFIDENCE line.
pub(crate) fn extract_confidence(output: &str) -> String {
    let lower = output.to_lowercase();
    if lower.contains("confidence: high") {
        "high".into()
    } else if lower.contains("confidence: low") {
        "low".into()
    } else if lower.contains("confidence: medium") {
        "medium".into()
    } else {
        "unknown".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_consensus_purpose_default() {
        let purpose = build_consensus_purpose(None, None, None);
        assert_eq!(purpose.name, "moe_consensus");
        assert_eq!(purpose.steps.len(), 3);
        assert_eq!(purpose.steps[0].model_hint, Some(ModelHint::Best));
        assert_eq!(purpose.steps[1].model_hint, Some(ModelHint::Balanced));
        assert_eq!(purpose.steps[2].model_hint, Some(ModelHint::Best));
    }

    #[test]
    fn build_consensus_purpose_custom_models() {
        let purpose = build_consensus_purpose(
            Some("gpt-4"),
            Some("claude-haiku"),
            Some("gemini-pro"),
        );
        assert_eq!(
            purpose.steps[0].model_hint,
            Some(ModelHint::Specific("gpt-4".to_string())),
        );
        assert_eq!(
            purpose.steps[1].model_hint,
            Some(ModelHint::Specific("claude-haiku".to_string())),
        );
        assert_eq!(
            purpose.steps[2].model_hint,
            Some(ModelHint::Specific("gemini-pro".to_string())),
        );
    }

    #[test]
    fn extract_confidence_high() {
        assert_eq!(
            extract_confidence("CONFIDENCE: high \u{2014} all models agree"),
            "high",
        );
    }

    #[test]
    fn extract_confidence_low() {
        assert_eq!(
            extract_confidence("Some text\nCONFIDENCE: low \u{2014} significant disagreement"),
            "low",
        );
    }

    #[test]
    fn extract_confidence_medium() {
        assert_eq!(
            extract_confidence("CONFIDENCE: medium \u{2014} partial agreement"),
            "medium",
        );
    }

    #[test]
    fn extract_confidence_unknown() {
        assert_eq!(extract_confidence("No confidence line here"), "unknown");
    }

    #[test]
    fn consensus_purpose_has_3_steps() {
        let purpose = build_consensus_purpose(None, None, None);
        assert_eq!(purpose.steps.len(), 3);
        assert_eq!(purpose.steps[0].capability, Capability::TextChat);
        assert_eq!(purpose.steps[1].capability, Capability::TextChat);
        assert_eq!(purpose.steps[2].capability, Capability::TextChat);
    }

    #[test]
    fn consensus_step_inputs_chain_correctly() {
        let purpose = build_consensus_purpose(None, None, None);

        // Step 0: takes the original request input
        assert!(matches!(purpose.steps[0].input, StepInput::FromRequest));

        // Step 1: template referencing {input} and {0}
        match &purpose.steps[1].input {
            StepInput::Template(t) => {
                assert!(t.contains("{input}"), "step 1 template must reference {{input}}");
                assert!(t.contains("{0}"), "step 1 template must reference {{0}}");
            }
            other => panic!("expected Template for step 1, got {other:?}"),
        }

        // Step 2: template referencing {input}, {0}, and {1}
        match &purpose.steps[2].input {
            StepInput::Template(t) => {
                assert!(t.contains("{input}"), "step 2 template must reference {{input}}");
                assert!(t.contains("{0}"), "step 2 template must reference {{0}}");
                assert!(t.contains("{1}"), "step 2 template must reference {{1}}");
            }
            other => panic!("expected Template for step 2, got {other:?}"),
        }
    }
}
