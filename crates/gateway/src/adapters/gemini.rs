//! Google Gemini adapter.
//!
//! Talks the Gemini generative-language REST API at
//! `https://generativelanguage.googleapis.com/v1beta`. Unlike OpenAI- /
//! Anthropic-compatible providers, Gemini has its own wire format:
//!
//! - Role names: `user` / `model` (not `assistant`)
//! - Messages are nested as `{role, parts: [{text}]}`
//! - System prompt is a top-level `systemInstruction` field, not a message
//! - Auth is `x-goog-api-key: <key>` header (or `?key=<key>` query
//!   string; header is preferred)
//! - Endpoint encodes the model id: `/models/{id}:generateContent`
//! - Embeddings use a separate endpoint with `embedding.values`
//!
//! Streaming uses `:streamGenerateContent?alt=sse` and Server-Sent
//! Events. Each SSE `data:` payload is the same JSON shape as a non-
//! streaming response — a list of candidates whose `parts[].text` is
//! an incremental delta — so the chunk parser reuses
//! [`GeminiChatResponse`] plus a per-candidate `finishReason` field.
//! The final chunk carries `usageMetadata` and `finishReason: "STOP"`.

use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::adapters::InferenceAdapter;
use crate::adapters::base::{build_client, resolve_api_key};
use crate::types::capability::Capability;
use crate::types::config::RouterConfig;
use crate::types::cost::TokenUsage;
use crate::types::error::GatewayError;
use crate::types::request::{
    InferenceRequest, InferenceResponse, Message, MessageRole, Payload, StreamChunk,
};

const ADAPTER_ID: &str = "gemini";
const DEFAULT_MODEL: &str = "gemini-2.0-flash";
const DEFAULT_EMBED_MODEL: &str = "text-embedding-004";
const DEFAULT_MAX_TOKENS: u32 = 1024;

// ---------------------------------------------------------------------------
// Wire format
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct GeminiContent<'a> {
    role: &'static str,
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Debug, Serialize, Default)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct GeminiChatRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction<'a>>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Deserialize)]
struct GeminiChatResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata", default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiResponseContent>,
    /// Streaming responses set this on the final chunk (`"STOP"`,
    /// `"MAX_TOKENS"`, `"SAFETY"`, …). Non-streaming responses also
    /// emit it on the single candidate.
    #[serde(rename = "finishReason", default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    #[serde(default)]
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount", default)]
    total_token_count: u32,
}

#[derive(Debug, Serialize)]
struct GeminiEmbedRequestItem<'a> {
    model: String,
    content: GeminiContent<'a>,
}

#[derive(Debug, Serialize)]
struct GeminiBatchEmbedRequest<'a> {
    requests: Vec<GeminiEmbedRequestItem<'a>>,
}

#[derive(Debug, Deserialize)]
struct GeminiBatchEmbedResponse {
    #[serde(default)]
    embeddings: Vec<GeminiEmbeddingValues>,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbeddingValues {
    #[serde(default)]
    values: Vec<f32>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Gemini uses `user` / `model` rather than OpenAI/Anthropic's
/// `user` / `assistant`. Tool and system messages get mapped to `user`
/// (system is hoisted into `systemInstruction` separately).
fn gemini_role(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::Assistant => "model",
        MessageRole::User | MessageRole::Tool | MessageRole::System => "user",
    }
}

/// Convert gateway messages into Gemini `contents`, skipping system-role
/// messages (which are hoisted into `systemInstruction` by the caller).
fn build_contents(messages: &[Message]) -> Vec<GeminiContent<'_>> {
    messages
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .map(|m| GeminiContent {
            role: gemini_role(&m.role),
            parts: vec![GeminiPart { text: m.as_text() }],
        })
        .collect()
}

/// Resolve the system instruction: prefer the explicit `system` field on
/// the Chat payload; fall back to concatenating any `MessageRole::System`
/// messages in the conversation.
fn extract_system_instruction<'a>(
    messages: &'a [Message],
    system: &'a Option<String>,
) -> Option<GeminiSystemInstruction<'a>> {
    if let Some(s) = system.as_deref() {
        return Some(GeminiSystemInstruction {
            parts: vec![GeminiPart { text: s }],
        });
    }
    let system_parts: Vec<&'a str> = messages
        .iter()
        .filter(|m| m.role == MessageRole::System)
        .map(|m| m.as_text())
        .collect();
    if system_parts.is_empty() {
        None
    } else {
        Some(GeminiSystemInstruction {
            parts: system_parts
                .into_iter()
                .map(|text| GeminiPart { text })
                .collect(),
        })
    }
}

/// Strip a leading `models/` prefix so callers can specify either form.
fn normalize_model_id(model: &str) -> &str {
    model.strip_prefix("models/").unwrap_or(model)
}

fn resolve_chat_model(request: &InferenceRequest) -> String {
    let raw = request.model.as_deref().unwrap_or(DEFAULT_MODEL);
    normalize_model_id(raw).to_string()
}

fn resolve_embed_model(request: &InferenceRequest) -> String {
    let raw = request.model.as_deref().unwrap_or(DEFAULT_EMBED_MODEL);
    normalize_model_id(raw).to_string()
}

fn extract_text(resp: &GeminiChatResponse) -> String {
    resp.candidates
        .iter()
        .filter_map(|c| c.content.as_ref())
        .flat_map(|c| c.parts.iter())
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

fn usage_from_gemini(usage: &Option<GeminiUsageMetadata>) -> Option<TokenUsage> {
    usage.as_ref().map(|u| TokenUsage {
        input_tokens: u.prompt_token_count,
        output_tokens: u.candidates_token_count,
        total_tokens: if u.total_token_count > 0 {
            u.total_token_count
        } else {
            u.prompt_token_count + u.candidates_token_count
        },
    })
}

/// Run a JSON POST against a Gemini endpoint with the `x-goog-api-key`
/// header and a content-type of `application/json`, applying any
/// `RouterConfig.headers` overrides. Returns the parsed JSON or maps the
/// HTTP status to the right [`GatewayError`] variant.
async fn gemini_post<Req: Serialize, Resp: serde::de::DeserializeOwned>(
    client: &Client,
    url: &str,
    api_key: &str,
    body: &Req,
    extra_headers: &HashMap<String, String>,
) -> Result<Resp, GatewayError> {
    let mut req = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(body);
    for (k, v) in extra_headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 | 403 => GatewayError::Authentication {
                adapter: ADAPTER_ID.into(),
                message: body_text,
            },
            429 => GatewayError::RateLimit {
                adapter: ADAPTER_ID.into(),
                retry_after_ms: None,
            },
            _ => GatewayError::ProviderError {
                adapter: ADAPTER_ID.into(),
                message: body_text,
                status: Some(status.as_u16()),
            },
        });
    }
    resp.json::<Resp>().await.map_err(|e| GatewayError::ProviderError {
        adapter: ADAPTER_ID.into(),
        message: format!("failed to parse response: {e}"),
        status: Some(status.as_u16()),
    })
}

/// Parse a single SSE `data: …` line emitted by
/// `:streamGenerateContent?alt=sse` into a [`StreamChunk`].
///
/// Returns `None` for empty / non-`data:` lines (keep-alive comments,
/// blank separators). Returns `Some(Err(…))` if the line looks like a
/// `data:` payload but the JSON does not parse — the caller surfaces
/// this as a provider error so a malformed stream is not silently
/// dropped.
///
/// Gemini sends the same payload shape as a non-streaming response, so
/// the parser reuses [`GeminiChatResponse`]. The chunk's `content` is
/// the concatenation of text parts across all candidates (matching
/// [`extract_text`] semantics); `finish_reason` is taken from the
/// first candidate that carries one. `usage` is sourced from
/// `usageMetadata`, which Gemini only sets on the terminal chunk.
fn parse_stream_line(line: &str) -> Option<Result<StreamChunk, GatewayError>> {
    let line = line.trim();
    let payload = line.strip_prefix("data:")?.trim();
    if payload.is_empty() {
        return None;
    }
    let parsed: GeminiChatResponse = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(e) => {
            return Some(Err(GatewayError::ProviderError {
                adapter: ADAPTER_ID.into(),
                message: format!("failed to parse SSE chunk: {e}"),
                status: None,
            }));
        }
    };
    let content = extract_text(&parsed);
    let finish_reason = parsed
        .candidates
        .iter()
        .find_map(|c| c.finish_reason.clone());
    let usage = usage_from_gemini(&parsed.usage_metadata);
    Some(Ok(StreamChunk {
        content,
        finish_reason,
        usage,
    }))
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

pub struct GeminiAdapter {
    client: Client,
}

impl GeminiAdapter {
    pub fn new() -> Result<Self, GatewayError> {
        Ok(Self {
            client: Client::new(),
        })
    }

    pub fn from_config(config: &RouterConfig) -> Result<Self, GatewayError> {
        Ok(Self {
            client: build_client(config)?,
        })
    }

    fn missing_key_err() -> GatewayError {
        GatewayError::Authentication {
            adapter: ADAPTER_ID.into(),
            message: "missing API key — set the env var specified in api_key_env (e.g. GEMINI_API_KEY)"
                .into(),
        }
    }
}

#[async_trait]
impl InferenceAdapter for GeminiAdapter {
    fn id(&self) -> &str {
        ADAPTER_ID
    }

    fn supports(&self, capability: &Capability) -> bool {
        matches!(capability, Capability::TextChat | Capability::TextEmbed)
    }

    async fn execute(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        let api_key = resolve_api_key(config).ok_or_else(Self::missing_key_err)?;

        match &request.payload {
            Payload::Chat {
                messages,
                system,
                max_tokens,
                temperature,
                tools: _,
            } => {
                let model = resolve_chat_model(request);
                let url = format!(
                    "{}/models/{}:generateContent",
                    config.url.trim_end_matches('/'),
                    model
                );

                let generation_config = (max_tokens.is_some() || temperature.is_some()).then(|| {
                    GeminiGenerationConfig {
                        max_output_tokens: Some(max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
                        temperature: *temperature,
                    }
                });

                let body = GeminiChatRequest {
                    contents: build_contents(messages),
                    system_instruction: extract_system_instruction(messages, system),
                    generation_config,
                };

                let resp: GeminiChatResponse =
                    gemini_post(&self.client, &url, &api_key, &body, &config.headers).await?;

                let content = extract_text(&resp);
                let usage = usage_from_gemini(&resp.usage_metadata);

                Ok(InferenceResponse {
                    success: true,
                    content: if content.is_empty() { None } else { Some(content) },
                    embeddings: None,
                    transcription: None,
                    audio: None,
                    images: None,
                    videos: None,
                    model: Some(model),
                    usage,
                    tool_calls: Vec::new(),
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }

            Payload::Embed { texts } => {
                if texts.is_empty() {
                    return Ok(InferenceResponse {
                        success: true,
                        content: None,
                        embeddings: Some(vec![]),
                        transcription: None,
                        audio: None,
                        images: None,
                        videos: None,
                        model: request.model.clone(),
                        tool_calls: Vec::new(),
                        usage: None,
                        estimated_cost: None,
                        actual_cost: None,
                        attempts: vec![],
                    });
                }
                let model = resolve_embed_model(request);
                let url = format!(
                    "{}/models/{}:batchEmbedContents",
                    config.url.trim_end_matches('/'),
                    model
                );
                let qualified_model = format!("models/{model}");
                let body = GeminiBatchEmbedRequest {
                    requests: texts
                        .iter()
                        .map(|t| GeminiEmbedRequestItem {
                            model: qualified_model.clone(),
                            content: GeminiContent {
                                role: "user",
                                parts: vec![GeminiPart { text: t.as_str() }],
                            },
                        })
                        .collect(),
                };

                let resp: GeminiBatchEmbedResponse =
                    gemini_post(&self.client, &url, &api_key, &body, &config.headers).await?;

                let embeddings: Vec<Vec<f32>> =
                    resp.embeddings.into_iter().map(|e| e.values).collect();

                Ok(InferenceResponse {
                    success: true,
                    content: None,
                    embeddings: Some(embeddings),
                    transcription: None,
                    audio: None,
                    images: None,
                    videos: None,
                    model: Some(model),
                    usage: None,
                    tool_calls: Vec::new(),
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }

            _ => Err(GatewayError::ProviderError {
                adapter: ADAPTER_ID.into(),
                message: "Gemini supports Payload::Chat and Payload::Embed only".into(),
                status: None,
            }),
        }
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
            tools: _,
        } = &request.payload
        else {
            return Err(GatewayError::ProviderError {
                adapter: ADAPTER_ID.into(),
                message: "Gemini streaming is only supported for Payload::Chat".into(),
                status: None,
            });
        };

        let api_key = resolve_api_key(config).ok_or_else(Self::missing_key_err)?;
        let model = resolve_chat_model(request);
        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse",
            config.url.trim_end_matches('/'),
            model
        );

        let generation_config = (max_tokens.is_some() || temperature.is_some()).then(|| {
            GeminiGenerationConfig {
                max_output_tokens: Some(max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
                temperature: *temperature,
            }
        });

        let body = GeminiChatRequest {
            contents: build_contents(messages),
            system_instruction: extract_system_instruction(messages, system),
            generation_config,
        };

        let mut req = self
            .client
            .post(&url)
            .header("x-goog-api-key", &api_key)
            .header("content-type", "application/json")
            .json(&body);
        for (k, v) in &config.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let response = req.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => GatewayError::Authentication {
                    adapter: ADAPTER_ID.into(),
                    message: body_text,
                },
                429 => GatewayError::RateLimit {
                    adapter: ADAPTER_ID.into(),
                    retry_after_ms: None,
                },
                _ => GatewayError::ProviderError {
                    adapter: ADAPTER_ID.into(),
                    message: body_text,
                    status: Some(status.as_u16()),
                },
            });
        }

        let byte_stream = response.bytes_stream();
        let stream = byte_stream
            .map(|result| -> Result<Vec<Result<StreamChunk, GatewayError>>, GatewayError> {
                let bytes = result?;
                let text = String::from_utf8_lossy(&bytes);
                Ok(text.lines().filter_map(parse_stream_line).collect())
            })
            .map(|result| match result {
                Ok(chunks) => futures::stream::iter(chunks),
                Err(e) => futures::stream::iter(vec![Err(e)]),
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

    fn user(content: &str) -> Message {
        Message::text(MessageRole::User, content)
    }

    fn assistant(content: &str) -> Message {
        Message::text(MessageRole::Assistant, content)
    }

    fn system_msg(content: &str) -> Message {
        Message::text(MessageRole::System, content)
    }

    #[test]
    fn id_and_supports_match_chat_and_embed() {
        let a = GeminiAdapter::new().unwrap();
        assert_eq!(a.id(), "gemini");
        assert!(a.supports(&Capability::TextChat));
        assert!(a.supports(&Capability::TextEmbed));
        assert!(!a.supports(&Capability::ImageGenerate));
        assert!(!a.supports(&Capability::VideoGenerate));
    }

    #[test]
    fn gemini_role_maps_assistant_to_model_and_others_to_user() {
        assert_eq!(gemini_role(&MessageRole::User), "user");
        assert_eq!(gemini_role(&MessageRole::Assistant), "model");
        assert_eq!(gemini_role(&MessageRole::Tool), "user");
        assert_eq!(gemini_role(&MessageRole::System), "user");
    }

    #[test]
    fn build_contents_skips_system_messages_and_uses_gemini_role_names() {
        let msgs = vec![
            system_msg("be concise"),
            user("hi"),
            assistant("hello"),
            user("how are you"),
        ];
        let contents = build_contents(&msgs);
        assert_eq!(contents.len(), 3, "system message should be skipped");
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
        assert_eq!(contents[2].role, "user");
        assert_eq!(contents[0].parts[0].text, "hi");
    }

    #[test]
    fn extract_system_prefers_explicit_field_over_message_role() {
        let msgs = vec![system_msg("inline rules"), user("hi")];
        let explicit = Some("explicit rules".to_string());
        let si = extract_system_instruction(&msgs, &explicit).unwrap();
        assert_eq!(si.parts.len(), 1);
        assert_eq!(si.parts[0].text, "explicit rules");
    }

    #[test]
    fn extract_system_falls_back_to_system_role_messages_when_no_explicit_field() {
        let msgs = vec![system_msg("rule one"), system_msg("rule two"), user("hi")];
        let si = extract_system_instruction(&msgs, &None).unwrap();
        assert_eq!(si.parts.len(), 2);
        assert_eq!(si.parts[0].text, "rule one");
        assert_eq!(si.parts[1].text, "rule two");
    }

    #[test]
    fn extract_system_returns_none_when_no_system_present() {
        let msgs = vec![user("hi")];
        assert!(extract_system_instruction(&msgs, &None).is_none());
    }

    #[test]
    fn normalize_model_id_strips_models_prefix_when_present() {
        assert_eq!(normalize_model_id("gemini-2.0-flash"), "gemini-2.0-flash");
        assert_eq!(
            normalize_model_id("models/gemini-2.0-flash"),
            "gemini-2.0-flash"
        );
        assert_eq!(
            normalize_model_id("text-embedding-004"),
            "text-embedding-004"
        );
    }

    #[test]
    fn resolve_chat_model_uses_default_when_request_omits_model() {
        let req = InferenceRequest {
            capability: Capability::TextChat,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![user("hi")],
                system: None,
                max_tokens: None,
                temperature: None,
                tools: Vec::new(),
            },
            budget: None,
        };
        assert_eq!(resolve_chat_model(&req), DEFAULT_MODEL);
    }

    #[test]
    fn extract_text_concatenates_parts_across_candidates() {
        let resp = GeminiChatResponse {
            candidates: vec![GeminiCandidate {
                content: Some(GeminiResponseContent {
                    parts: vec![
                        GeminiResponsePart {
                            text: Some("hello".into()),
                        },
                        GeminiResponsePart { text: None },
                        GeminiResponsePart {
                            text: Some(" world".into()),
                        },
                    ],
                }),
                finish_reason: None,
            }],
            usage_metadata: None,
        };
        assert_eq!(extract_text(&resp), "hello world");
    }

    #[test]
    fn usage_from_gemini_computes_total_when_field_is_zero() {
        let u = Some(GeminiUsageMetadata {
            prompt_token_count: 3,
            candidates_token_count: 5,
            total_token_count: 0,
        });
        let got = usage_from_gemini(&u).unwrap();
        assert_eq!(got.input_tokens, 3);
        assert_eq!(got.output_tokens, 5);
        assert_eq!(got.total_tokens, 8);
    }

    #[test]
    fn usage_from_gemini_uses_explicit_total_when_provided() {
        let u = Some(GeminiUsageMetadata {
            prompt_token_count: 3,
            candidates_token_count: 5,
            total_token_count: 9, // includes overhead the response decided to attribute
        });
        let got = usage_from_gemini(&u).unwrap();
        assert_eq!(got.total_tokens, 9);
    }

    #[test]
    fn missing_api_key_surfaces_authentication_error() {
        let err = GeminiAdapter::missing_key_err();
        assert!(matches!(err, GatewayError::Authentication { .. }));
    }

    #[test]
    fn parse_stream_line_returns_none_for_empty_or_non_data_lines() {
        assert!(parse_stream_line("").is_none());
        assert!(parse_stream_line("   ").is_none());
        assert!(parse_stream_line(": keep-alive comment").is_none());
        // `data:` with empty payload (Gemini sometimes emits these
        // between events) should be skipped, not parsed.
        assert!(parse_stream_line("data:").is_none());
        assert!(parse_stream_line("data:   ").is_none());
    }

    #[test]
    fn parse_stream_line_extracts_incremental_text_from_candidate_parts() {
        let line = r#"data: {"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"},"index":0}]}"#;
        let chunk = parse_stream_line(line).expect("Some").expect("Ok");
        assert_eq!(chunk.content, "Hello");
        assert!(chunk.finish_reason.is_none());
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn parse_stream_line_carries_finish_reason_on_terminal_chunk() {
        let line = r#"data: {"candidates":[{"content":{"parts":[{"text":"."}],"role":"model"},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":4,"candidatesTokenCount":6,"totalTokenCount":10}}"#;
        let chunk = parse_stream_line(line).expect("Some").expect("Ok");
        assert_eq!(chunk.content, ".");
        assert_eq!(chunk.finish_reason.as_deref(), Some("STOP"));
        let usage = chunk.usage.expect("usage present on terminal chunk");
        assert_eq!(usage.input_tokens, 4);
        assert_eq!(usage.output_tokens, 6);
        assert_eq!(usage.total_tokens, 10);
    }

    #[test]
    fn parse_stream_line_yields_provider_error_on_bad_json() {
        let line = "data: {not json";
        let err = parse_stream_line(line)
            .expect("Some")
            .expect_err("Err on bad json");
        assert!(matches!(err, GatewayError::ProviderError { .. }));
    }

    #[test]
    fn parse_stream_line_handles_data_payload_without_space_after_colon() {
        // SSE allows `data:payload` (no leading space) in addition to
        // the more common `data: payload`.
        let line = r#"data:{"candidates":[{"content":{"parts":[{"text":"hi"}],"role":"model"}}]}"#;
        let chunk = parse_stream_line(line).expect("Some").expect("Ok");
        assert_eq!(chunk.content, "hi");
    }

    #[test]
    fn chat_request_serialises_with_camel_case_keys() {
        let body = GeminiChatRequest {
            contents: vec![GeminiContent {
                role: "user",
                parts: vec![GeminiPart { text: "hi" }],
            }],
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart { text: "be brief" }],
            }),
            generation_config: Some(GeminiGenerationConfig {
                max_output_tokens: Some(64),
                temperature: Some(0.2),
            }),
        };
        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("systemInstruction").is_some());
        assert!(json.get("generationConfig").is_some());
        let gc = &json["generationConfig"];
        assert_eq!(gc["maxOutputTokens"], 64);
    }
}
