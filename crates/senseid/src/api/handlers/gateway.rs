use axum::extract::State;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::state::AppState;

/// GET /api/gateway/status — returns registered adapters and overall status.
pub(crate) async fn gateway_status(State(state): State<AppState>) -> Json<Value> {
    let adapters = state.gateway.list_adapters().await;
    Json(json!({
        "status": "ok",
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
        msgs.iter().map(|m| Message {
            role: match m.role.as_str() {
                "system"    => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                "tool"      => MessageRole::Tool,
                _           => MessageRole::User,
            },
            content: m.content.clone(),
            tool_call_id: None,
        }).collect()
    } else if let Some(prompt) = &body.prompt {
        vec![Message {
            role: MessageRole::User,
            content: prompt.clone(),
            tool_call_id: None,
        }]
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
