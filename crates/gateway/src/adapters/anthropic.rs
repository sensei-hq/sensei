use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{build_client, resolve_api_key};
use super::InferenceAdapter;
use crate::types::capability::Capability;
use crate::types::config::RouterConfig;
use crate::types::cost::TokenUsage;
use crate::types::error::GatewayError;
use crate::types::request::{
    InferenceRequest, InferenceResponse, Message, MessageRole, Payload, StreamChunk,
};

// ---------------------------------------------------------------------------
// Wire types — Anthropic Messages API request/response structs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

// Streaming event types

#[derive(Debug, Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<StreamDelta>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    text: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DEFAULT_MODEL: &str = "claude-haiku-4-5-20250414";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 1024;

fn resolve_model(request: &InferenceRequest) -> String {
    request
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

/// Extract system prompt from:
/// 1. The explicit `system` field on the Chat payload
/// 2. Any message with role System (first one wins)
fn extract_system(messages: &[Message], system: &Option<String>) -> Option<String> {
    if system.is_some() {
        return system.clone();
    }
    messages
        .iter()
        .find(|m| m.role == MessageRole::System)
        .map(|m| m.content.clone())
}

/// Build Anthropic message array: filter out System messages, map Tool → user.
fn build_messages(messages: &[Message]) -> Vec<AnthropicMessage> {
    messages
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .map(|m| {
            let role = match m.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "user",
                MessageRole::System => unreachable!(), // filtered above
            };
            AnthropicMessage {
                role: role.to_string(),
                content: m.content.clone(),
            }
        })
        .collect()
}

fn usage_from_anthropic(usage: &AnthropicUsage) -> TokenUsage {
    TokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.input_tokens + usage.output_tokens,
    }
}

/// Extract text from content blocks, joining all text blocks.
fn extract_text(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter(|b| b.block_type == "text")
        .filter_map(|b| b.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

// ---------------------------------------------------------------------------
// AnthropicAdapter
// ---------------------------------------------------------------------------

/// Adapter for the Anthropic Messages API.
///
/// Supports chat completions via `POST /v1/messages`. Anthropic does not expose
/// an embedding endpoint, so only the Chat capability is supported.
pub struct AnthropicAdapter {
    client: Client,
}

impl AnthropicAdapter {
    pub fn new() -> Result<Self, GatewayError> {
        Ok(Self {
            client: Client::new(),
        })
    }

    /// Create an adapter from a pre-built client (e.g. with timeout from config).
    pub fn from_config(config: &RouterConfig) -> Result<Self, GatewayError> {
        Ok(Self {
            client: build_client(config)?,
        })
    }
}

#[async_trait]
impl InferenceAdapter for AnthropicAdapter {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn supports(&self, capability: &Capability) -> bool {
        matches!(capability, Capability::Chat)
    }

    async fn execute(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        let Payload::Chat {
            messages,
            system,
            max_tokens,
            temperature,
        } = &request.payload
        else {
            return Err(GatewayError::ProviderError {
                adapter: "anthropic".into(),
                message: "Anthropic only supports chat payloads".into(),
                status: None,
            });
        };

        let api_key = resolve_api_key(config).ok_or_else(|| GatewayError::Authentication {
            adapter: "anthropic".into(),
            message: "missing API key — set the env var specified in api_key_env".into(),
        })?;

        let model = resolve_model(request);
        let extracted_system = extract_system(messages, system);

        let body = AnthropicRequest {
            model: model.clone(),
            messages: build_messages(messages),
            max_tokens: max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system: extracted_system,
            temperature: *temperature,
            stream: false,
        };

        let url = format!("{}/v1/messages", config.url.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => GatewayError::Authentication {
                    adapter: "anthropic".into(),
                    message: body_text,
                },
                429 => GatewayError::RateLimit {
                    adapter: "anthropic".into(),
                    retry_after_ms: None,
                },
                _ => GatewayError::ProviderError {
                    adapter: "anthropic".into(),
                    message: body_text,
                    status: Some(status.as_u16()),
                },
            });
        }

        let anthropic_resp: AnthropicResponse = resp.json().await.map_err(|e| {
            GatewayError::ProviderError {
                adapter: "anthropic".into(),
                message: format!("failed to parse response: {}", e),
                status: Some(status.as_u16()),
            }
        })?;

        let content = extract_text(&anthropic_resp.content);
        let usage = usage_from_anthropic(&anthropic_resp.usage);

        Ok(InferenceResponse {
            success: true,
            content: if content.is_empty() {
                None
            } else {
                Some(content)
            },
            embeddings: None,
            transcription: None,
            audio: None,
            images: None,
            model: Some(model),
            usage: Some(usage),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        })
    }

    async fn stream(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>, GatewayError>
    {
        let Payload::Chat {
            messages,
            system,
            max_tokens,
            temperature,
        } = &request.payload
        else {
            return Err(GatewayError::ProviderError {
                adapter: "anthropic".into(),
                message: "streaming is only supported for chat payloads".into(),
                status: None,
            });
        };

        let api_key = resolve_api_key(config).ok_or_else(|| GatewayError::Authentication {
            adapter: "anthropic".into(),
            message: "missing API key — set the env var specified in api_key_env".into(),
        })?;

        let model = resolve_model(request);
        let extracted_system = extract_system(messages, system);

        let body = AnthropicRequest {
            model,
            messages: build_messages(messages),
            max_tokens: max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system: extracted_system,
            temperature: *temperature,
            stream: true,
        };

        let url = format!("{}/v1/messages", config.url.trim_end_matches('/'));
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => GatewayError::Authentication {
                    adapter: "anthropic".into(),
                    message: body_text,
                },
                429 => GatewayError::RateLimit {
                    adapter: "anthropic".into(),
                    retry_after_ms: None,
                },
                _ => GatewayError::ProviderError {
                    adapter: "anthropic".into(),
                    message: body_text,
                    status: Some(status.as_u16()),
                },
            });
        }

        let byte_stream = response.bytes_stream();

        let stream = byte_stream
            .map(|result| -> Result<Vec<StreamChunk>, GatewayError> {
                let bytes = result?;
                let text = String::from_utf8_lossy(&bytes);
                let mut chunks = Vec::new();

                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with("event:") {
                        continue;
                    }
                    let json_str = line.strip_prefix("data: ").unwrap_or(line);
                    if let Ok(event) = serde_json::from_str::<StreamEvent>(json_str) {
                        match event.event_type.as_str() {
                            "content_block_delta" => {
                                if let Some(delta) = &event.delta {
                                    let content =
                                        delta.text.clone().unwrap_or_default();
                                    if !content.is_empty() {
                                        chunks.push(StreamChunk {
                                            content,
                                            finish_reason: None,
                                            usage: None,
                                        });
                                    }
                                }
                            }
                            "message_delta" => {
                                let usage = event.usage.as_ref().map(usage_from_anthropic);
                                chunks.push(StreamChunk {
                                    content: String::new(),
                                    finish_reason: Some("end_turn".to_string()),
                                    usage,
                                });
                            }
                            "message_stop" => {
                                chunks.push(StreamChunk {
                                    content: String::new(),
                                    finish_reason: Some("stop".to_string()),
                                    usage: None,
                                });
                            }
                            _ => {} // skip ping, message_start, content_block_start, etc.
                        }
                    }
                }

                Ok(chunks)
            })
            .map(|result| -> futures::stream::Iter<std::vec::IntoIter<Result<StreamChunk, GatewayError>>> {
                match result {
                    Ok(chunks) => {
                        futures::stream::iter(chunks.into_iter().map(Ok).collect::<Vec<_>>())
                    }
                    Err(e) => futures::stream::iter(vec![Err(e)]),
                }
            })
            .flatten();

        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_id_and_supports() {
        let adapter = AnthropicAdapter::new().unwrap();
        assert_eq!(adapter.id(), "anthropic");

        assert!(adapter.supports(&Capability::Chat));

        assert!(!adapter.supports(&Capability::Embed));
        assert!(!adapter.supports(&Capability::Classify));
        assert!(!adapter.supports(&Capability::Summarize));
        assert!(!adapter.supports(&Capability::VoiceStt));
        assert!(!adapter.supports(&Capability::VoiceTts));
        assert!(!adapter.supports(&Capability::ImageGenerate));
    }

    #[test]
    fn build_anthropic_request_basic() {
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_call_id: None,
        }];

        let anthropic_messages = build_messages(&messages);
        let body = AnthropicRequest {
            model: "claude-haiku-4-5-20250414".to_string(),
            messages: anthropic_messages,
            max_tokens: 1024,
            system: None,
            temperature: Some(0.7),
            stream: false,
        };

        let json = serde_json::to_value(&body).unwrap();

        assert_eq!(json["model"], "claude-haiku-4-5-20250414");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["stream"], false);
        assert!(json.get("system").is_none()); // skipped when None

        let msgs = json["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "Hello");
    }

    #[test]
    fn build_anthropic_request_with_system() {
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Hi".to_string(),
            tool_call_id: None,
        }];
        let system = Some("You are helpful.".to_string());

        let extracted = extract_system(&messages, &system);
        let anthropic_messages = build_messages(&messages);

        let body = AnthropicRequest {
            model: "claude-haiku-4-5-20250414".to_string(),
            messages: anthropic_messages,
            max_tokens: 512,
            system: extracted,
            temperature: None,
            stream: false,
        };

        let json = serde_json::to_value(&body).unwrap();

        // System prompt is at the top level, not in messages
        assert_eq!(json["system"], "You are helpful.");

        let msgs = json["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        // No system message in the messages array
        assert!(msgs.iter().all(|m| m["role"] != "system"));
    }

    #[test]
    fn build_anthropic_request_filters_system_messages() {
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: "Be concise.".to_string(),
                tool_call_id: None,
            },
            Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_call_id: None,
            },
            Message {
                role: MessageRole::Assistant,
                content: "Hi!".to_string(),
                tool_call_id: None,
            },
            Message {
                role: MessageRole::Tool,
                content: "result: 42".to_string(),
                tool_call_id: Some("tc_1".to_string()),
            },
        ];

        let extracted = extract_system(&messages, &None);
        assert_eq!(extracted.as_deref(), Some("Be concise."));

        let anthropic_messages = build_messages(&messages);

        // System message is removed, 3 remaining
        assert_eq!(anthropic_messages.len(), 3);
        assert_eq!(anthropic_messages[0].role, "user");
        assert_eq!(anthropic_messages[0].content, "Hello");
        assert_eq!(anthropic_messages[1].role, "assistant");
        assert_eq!(anthropic_messages[1].content, "Hi!");
        // Tool role mapped to user
        assert_eq!(anthropic_messages[2].role, "user");
        assert_eq!(anthropic_messages[2].content, "result: 42");
    }

    #[test]
    fn parse_anthropic_response() {
        let json = r#"{
            "content": [{"type": "text", "text": "Hello!"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;

        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();

        assert_eq!(resp.content.len(), 1);
        assert_eq!(resp.content[0].block_type, "text");
        assert_eq!(resp.content[0].text.as_deref(), Some("Hello!"));
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);

        let text = extract_text(&resp.content);
        assert_eq!(text, "Hello!");

        let usage = usage_from_anthropic(&resp.usage);
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn parse_anthropic_response_multiple_blocks() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "Hello, "},
                {"type": "tool_use", "id": "tc_1", "name": "get_weather"},
                {"type": "text", "text": "world!"}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 20, "output_tokens": 10}
        }"#;

        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();

        assert_eq!(resp.content.len(), 3);

        // Only text blocks are extracted and joined
        let text = extract_text(&resp.content);
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn parse_stream_content_delta() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#;

        let event: StreamEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.event_type, "content_block_delta");
        assert!(event.delta.is_some());
        assert_eq!(event.delta.unwrap().text.as_deref(), Some("Hi"));
    }

    #[test]
    fn parse_stream_message_delta() {
        let json = r#"{"type":"message_delta","usage":{"output_tokens":15}}"#;

        let event: StreamEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.event_type, "message_delta");
        assert!(event.usage.is_some());

        let usage = event.usage.unwrap();
        assert_eq!(usage.output_tokens, 15);
        assert_eq!(usage.input_tokens, 0); // default
    }

    #[tokio::test]
    #[ignore]
    async fn anthropic_chat_integration() {
        // Requires ANTHROPIC_API_KEY env var
        let adapter = AnthropicAdapter::new().unwrap();
        let config = RouterConfig {
            url: "https://api.anthropic.com".to_string(),
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
            enabled: true,
            timeout_ms: Some(30000),
            headers: std::collections::HashMap::new(),
        };
        let request = InferenceRequest {
            capability: Capability::Chat,
            model: Some("claude-haiku-4-5-20250414".to_string()),
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "Say hello in one sentence.".to_string(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: Some(64),
                temperature: Some(0.3),
            },
            budget: None,
        };

        let response = adapter.execute(&config, &request).await.unwrap();
        assert!(response.success);
        assert!(response.content.is_some());
        assert!(!response.content.unwrap().is_empty());
        assert!(response.usage.is_some());
    }
}
