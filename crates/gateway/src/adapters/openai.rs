use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::base::{build_client, http_json, resolve_api_key};
use super::InferenceAdapter;
use crate::types::capability::Capability;
use crate::types::config::RouterConfig;
use crate::types::cost::TokenUsage;
use crate::types::error::GatewayError;
use crate::types::request::{
    ImageResult, InferenceRequest, InferenceResponse, Message, MessageRole, Payload, StreamChunk,
};

// ---------------------------------------------------------------------------
// Wire types — OpenAI chat/embed request/response structs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<UsageResponse>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
    usage: Option<UsageResponse>,
}

#[derive(Debug, Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

// Voice wire types — Whisper (STT) + TTS

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(Debug, Serialize)]
struct TtsRequest {
    model: String,
    input: String,
    voice: String,
    speed: f32,
    response_format: String,
}

// Image generation wire types

#[derive(Debug, Serialize)]
struct ImageGenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<String>,
    n: u8,
}

#[derive(Debug, Deserialize)]
struct ImageGenerateResponse {
    data: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
struct ImageData {
    url: Option<String>,
    b64_json: Option<String>,
    revised_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChatResponse {
    choices: Vec<StreamChoice>,
    usage: Option<UsageResponse>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DEFAULT_MODEL: &str = "gpt-4o-mini";

fn role_to_string(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn resolve_model(request: &InferenceRequest) -> String {
    request
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn build_chat_messages(messages: &[Message], system: &Option<String>) -> Vec<ChatMessage> {
    let mut out = Vec::new();
    if let Some(sys) = system {
        out.push(ChatMessage {
            role: "system".to_string(),
            content: sys.clone(),
        });
    }
    for m in messages {
        out.push(ChatMessage {
            role: role_to_string(&m.role).to_string(),
            content: m.content.clone(),
        });
    }
    out
}

fn usage_from_response(usage: &Option<UsageResponse>) -> Option<TokenUsage> {
    usage.as_ref().map(|u| {
        let input = u.prompt_tokens.unwrap_or(0);
        let output = u.completion_tokens.unwrap_or(0);
        let total = u.total_tokens.unwrap_or(input + output);
        TokenUsage {
            input_tokens: input,
            output_tokens: output,
            total_tokens: total,
        }
    })
}

/// Require an API key, returning an Authentication error when missing.
fn require_api_key(config: &RouterConfig) -> Result<String, GatewayError> {
    resolve_api_key(config).ok_or_else(|| GatewayError::Authentication {
        adapter: "openai".into(),
        message: "missing API key — set the env var specified in api_key_env".into(),
    })
}

// ---------------------------------------------------------------------------
// OpenAIAdapter
// ---------------------------------------------------------------------------

/// Adapter for the official OpenAI API.
///
/// Uses Bearer-token authentication (`Authorization: Bearer {key}`) and targets
/// `POST /v1/chat/completions` and `POST /v1/embeddings`.
pub struct OpenAIAdapter {
    client: Client,
}

impl OpenAIAdapter {
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
impl InferenceAdapter for OpenAIAdapter {
    fn id(&self) -> &str {
        "openai"
    }

    fn supports(&self, capability: &Capability) -> bool {
        matches!(
            capability,
            Capability::TextChat
                | Capability::TextEmbed
                | Capability::AudioTranscribe
                | Capability::AudioGenerate
                | Capability::ImageGenerate
        )
    }

    async fn execute(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        let api_key = require_api_key(config)?;
        let model = resolve_model(request);

        match &request.payload {
            Payload::Chat {
                messages,
                system,
                max_tokens,
                temperature,
            } => {
                let body = ChatCompletionRequest {
                    model: model.clone(),
                    messages: build_chat_messages(messages, system),
                    max_tokens: *max_tokens,
                    temperature: *temperature,
                    stream: false,
                };

                let resp: ChatCompletionResponse = http_json(
                    &self.client,
                    &config.url,
                    "/v1/chat/completions",
                    &body,
                    Some(&api_key),
                    &config.headers,
                )
                .await?;

                let content = resp
                    .choices
                    .first()
                    .and_then(|c| c.message.content.clone());
                let usage = usage_from_response(&resp.usage);

                Ok(InferenceResponse {
                    success: true,
                    content,
                    embeddings: None,
                    transcription: None,
                    audio: None,
                    images: None,
                    videos: None,
                    model: Some(model),
                    usage,
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }
            Payload::Embed { texts } => {
                let body = EmbedRequest {
                    model: model.clone(),
                    input: texts.clone(),
                };

                let resp: EmbedResponse = http_json(
                    &self.client,
                    &config.url,
                    "/v1/embeddings",
                    &body,
                    Some(&api_key),
                    &config.headers,
                )
                .await?;

                let embeddings: Vec<Vec<f32>> =
                    resp.data.into_iter().map(|d| d.embedding).collect();
                let usage = usage_from_response(&resp.usage);

                Ok(InferenceResponse {
                    success: true,
                    content: None,
                    embeddings: Some(embeddings),
                    transcription: None,
                    audio: None,
                    images: None,
                    videos: None,
                    model: Some(model),
                    usage,
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }
            Payload::Stt {
                audio,
                language,
                format,
            } => {
                let mime = match format.as_str() {
                    "mp3" => "audio/mpeg",
                    "wav" => "audio/wav",
                    "webm" => "audio/webm",
                    "m4a" => "audio/mp4",
                    other => {
                        return Err(GatewayError::ProviderError {
                            adapter: "openai".into(),
                            message: format!("unsupported audio format: {other}"),
                            status: None,
                        })
                    }
                };

                let file_part = reqwest::multipart::Part::bytes(audio.clone())
                    .file_name(format!("audio.{format}"))
                    .mime_str(mime)
                    .map_err(|e| GatewayError::ProviderError {
                        adapter: "openai".into(),
                        message: format!("failed to build multipart: {e}"),
                        status: None,
                    })?;

                let mut form = reqwest::multipart::Form::new()
                    .part("file", file_part)
                    .text("model", model.clone());

                if let Some(lang) = language {
                    form = form.text("language", lang.clone());
                }

                let url = format!(
                    "{}/v1/audio/transcriptions",
                    config.url.trim_end_matches('/')
                );
                let mut req = self
                    .client
                    .post(&url)
                    .multipart(form)
                    .bearer_auth(&api_key);

                for (k, v) in &config.headers {
                    req = req.header(k.as_str(), v.as_str());
                }

                let response = req.send().await?;
                let status = response.status();

                if !status.is_success() {
                    let body_text = response.text().await.unwrap_or_default();
                    return Err(match status.as_u16() {
                        401 | 403 => GatewayError::Authentication {
                            adapter: "openai".into(),
                            message: body_text,
                        },
                        429 => GatewayError::RateLimit {
                            adapter: "openai".into(),
                            retry_after_ms: None,
                        },
                        _ => GatewayError::ProviderError {
                            adapter: "openai".into(),
                            message: body_text,
                            status: Some(status.as_u16()),
                        },
                    });
                }

                let whisper_resp: WhisperResponse =
                    response.json().await.map_err(|e| GatewayError::ProviderError {
                        adapter: "openai".into(),
                        message: format!("failed to parse whisper response: {e}"),
                        status: Some(status.as_u16()),
                    })?;

                Ok(InferenceResponse {
                    success: true,
                    content: None,
                    embeddings: None,
                    transcription: Some(whisper_resp.text),
                    audio: None,
                    images: None,
                    videos: None,
                    model: Some(model),
                    usage: None,
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }
            Payload::Tts {
                text,
                voice,
                speed,
                output_format,
            } => {
                let body = TtsRequest {
                    model: model.clone(),
                    input: text.clone(),
                    voice: voice.clone().unwrap_or_else(|| "alloy".to_string()),
                    speed: speed.unwrap_or(1.0),
                    response_format: output_format.to_string(),
                };

                let url = format!(
                    "{}/v1/audio/speech",
                    config.url.trim_end_matches('/')
                );
                let mut req = self
                    .client
                    .post(&url)
                    .json(&body)
                    .bearer_auth(&api_key);

                for (k, v) in &config.headers {
                    req = req.header(k.as_str(), v.as_str());
                }

                let response = req.send().await?;
                let status = response.status();

                if !status.is_success() {
                    let body_text = response.text().await.unwrap_or_default();
                    return Err(match status.as_u16() {
                        401 | 403 => GatewayError::Authentication {
                            adapter: "openai".into(),
                            message: body_text,
                        },
                        429 => GatewayError::RateLimit {
                            adapter: "openai".into(),
                            retry_after_ms: None,
                        },
                        _ => GatewayError::ProviderError {
                            adapter: "openai".into(),
                            message: body_text,
                            status: Some(status.as_u16()),
                        },
                    });
                }

                let audio_bytes = response.bytes().await.map_err(|e| {
                    GatewayError::ProviderError {
                        adapter: "openai".into(),
                        message: format!("failed to read TTS audio bytes: {e}"),
                        status: None,
                    }
                })?;

                Ok(InferenceResponse {
                    success: true,
                    content: None,
                    embeddings: None,
                    transcription: None,
                    audio: Some(audio_bytes.to_vec()),
                    images: None,
                    videos: None,
                    model: Some(model),
                    usage: None,
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }
            Payload::ImageGenerate {
                prompt,
                size,
                quality,
                style,
                n,
            } => {
                let image_model = request
                    .model
                    .clone()
                    .unwrap_or_else(|| "dall-e-3".to_string());

                let body = ImageGenerateRequest {
                    model: image_model.clone(),
                    prompt: prompt.clone(),
                    size: size.clone(),
                    quality: quality.clone(),
                    style: style.clone(),
                    n: *n,
                };

                let url = format!(
                    "{}/v1/images/generations",
                    config.url.trim_end_matches('/')
                );
                let mut req = self
                    .client
                    .post(&url)
                    .json(&body)
                    .bearer_auth(&api_key);

                for (k, v) in &config.headers {
                    req = req.header(k.as_str(), v.as_str());
                }

                let response = req.send().await?;
                let status = response.status();

                if !status.is_success() {
                    let body_text = response.text().await.unwrap_or_default();
                    return Err(match status.as_u16() {
                        401 | 403 => GatewayError::Authentication {
                            adapter: "openai".into(),
                            message: body_text,
                        },
                        429 => GatewayError::RateLimit {
                            adapter: "openai".into(),
                            retry_after_ms: None,
                        },
                        _ => GatewayError::ProviderError {
                            adapter: "openai".into(),
                            message: body_text,
                            status: Some(status.as_u16()),
                        },
                    });
                }

                let image_resp: ImageGenerateResponse =
                    response.json().await.map_err(|e| GatewayError::ProviderError {
                        adapter: "openai".into(),
                        message: format!("failed to parse image generation response: {e}"),
                        status: Some(status.as_u16()),
                    })?;

                let images: Vec<ImageResult> = image_resp
                    .data
                    .into_iter()
                    .map(|d| ImageResult {
                        url: d.url,
                        b64_json: d.b64_json,
                        revised_prompt: d.revised_prompt,
                    })
                    .collect();

                Ok(InferenceResponse {
                    success: true,
                    content: None,
                    embeddings: None,
                    transcription: None,
                    audio: None,
                    images: Some(images),
                    videos: None,
                    model: Some(image_model),
                    usage: None,
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }
            Payload::VideoGenerate { .. } => Err(GatewayError::ProviderError {
                adapter: "openai".into(),
                message: "OpenAI video generation is not yet supported".into(),
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
        } = &request.payload
        else {
            return Err(GatewayError::ProviderError {
                adapter: "openai".into(),
                message: "streaming is only supported for chat payloads".into(),
                status: None,
            });
        };

        let api_key = require_api_key(config)?;
        let model = resolve_model(request);

        let body = ChatCompletionRequest {
            model,
            messages: build_chat_messages(messages, system),
            max_tokens: *max_tokens,
            temperature: *temperature,
            stream: true,
        };

        let url = format!(
            "{}/v1/chat/completions",
            config.url.trim_end_matches('/')
        );
        let mut req = self.client.post(&url).json(&body).bearer_auth(&api_key);

        for (k, v) in &config.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let response = req.send().await?;
        let status = response.status();

        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => GatewayError::Authentication {
                    adapter: "openai".into(),
                    message: body_text,
                },
                429 => GatewayError::RateLimit {
                    adapter: "openai".into(),
                    retry_after_ms: None,
                },
                _ => GatewayError::ProviderError {
                    adapter: "openai".into(),
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
                    if line.is_empty() || line == "data: [DONE]" {
                        continue;
                    }
                    let json_str = line.strip_prefix("data: ").unwrap_or(line);
                    if let Ok(parsed) = serde_json::from_str::<StreamChatResponse>(json_str)
                        && let Some(choice) = parsed.choices.first()
                    {
                        let content = choice.delta.content.clone().unwrap_or_default();
                        let usage = usage_from_response(&parsed.usage);
                        chunks.push(StreamChunk {
                            content,
                            finish_reason: choice.finish_reason.clone(),
                            usage,
                        });
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
    fn openai_id_and_supports() {
        let adapter = OpenAIAdapter::new().unwrap();
        assert_eq!(adapter.id(), "openai");

        assert!(adapter.supports(&Capability::TextChat));
        assert!(adapter.supports(&Capability::TextEmbed));
        assert!(adapter.supports(&Capability::AudioTranscribe));
        assert!(adapter.supports(&Capability::AudioGenerate));
        assert!(adapter.supports(&Capability::ImageGenerate));

        assert!(!adapter.supports(&Capability::VideoGenerate));
    }

    #[test]
    fn openai_supports_voice() {
        let adapter = OpenAIAdapter::new().unwrap();
        assert!(adapter.supports(&Capability::AudioTranscribe));
        assert!(adapter.supports(&Capability::AudioGenerate));
    }

    #[test]
    fn parse_whisper_response() {
        let json = r#"{"text":"Hello world"}"#;
        let resp: WhisperResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "Hello world");
    }

    #[test]
    fn build_tts_request() {
        let body = TtsRequest {
            model: "tts-1".to_string(),
            input: "Hello world".to_string(),
            voice: "alloy".to_string(),
            speed: 1.0,
            response_format: "mp3".to_string(),
        };

        let json = serde_json::to_value(&body).unwrap();

        assert_eq!(json["model"], "tts-1");
        assert_eq!(json["input"], "Hello world");
        assert_eq!(json["voice"], "alloy");
        assert!((json["speed"].as_f64().unwrap() - 1.0).abs() < f64::EPSILON);
        assert_eq!(json["response_format"], "mp3");
    }

    #[test]
    fn build_chat_request() {
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_call_id: None,
        }];
        let system = Some("You are helpful.".to_string());
        let chat_messages = build_chat_messages(&messages, &system);

        let body = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: chat_messages,
            max_tokens: Some(1024),
            temperature: Some(0.7),
            stream: false,
        };

        let json = serde_json::to_value(&body).unwrap();

        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["stream"], false);
        assert_eq!(json["max_tokens"], 1024);
        assert!(json["temperature"].as_f64().unwrap() > 0.69 && json["temperature"].as_f64().unwrap() < 0.71);

        let msgs = json["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "You are helpful.");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "Hello");
    }

    #[test]
    fn build_embed_request() {
        let body = EmbedRequest {
            model: "text-embedding-3-small".to_string(),
            input: vec!["hello world".to_string(), "foo bar".to_string()],
        };

        let json = serde_json::to_value(&body).unwrap();

        assert_eq!(json["model"], "text-embedding-3-small");
        let input = json["input"].as_array().unwrap();
        assert_eq!(input.len(), 2);
        assert_eq!(input[0], "hello world");
        assert_eq!(input[1], "foo bar");
    }

    #[test]
    fn parse_chat_response() {
        let json = r#"{
            "choices": [{
                "message": {"content": "Hello there!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        }"#;

        let resp: ChatCompletionResponse = serde_json::from_str(json).unwrap();

        assert_eq!(resp.choices.len(), 1);
        assert_eq!(
            resp.choices[0].message.content.as_deref(),
            Some("Hello there!"),
        );
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));

        let usage = usage_from_response(&resp.usage).unwrap();
        assert_eq!(usage.input_tokens, 12);
        assert_eq!(usage.output_tokens, 8);
        assert_eq!(usage.total_tokens, 20);
    }

    #[test]
    fn parse_embed_response() {
        let json = r#"{
            "data": [
                {"embedding": [0.1, 0.2, 0.3], "index": 0},
                {"embedding": [0.4, 0.5, 0.6], "index": 1}
            ],
            "usage": {
                "prompt_tokens": 8,
                "total_tokens": 8
            }
        }"#;

        let resp: EmbedResponse = serde_json::from_str(json).unwrap();

        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.data[0].embedding, vec![0.1, 0.2, 0.3]);
        assert_eq!(resp.data[1].embedding, vec![0.4, 0.5, 0.6]);

        let usage = usage_from_response(&resp.usage).unwrap();
        assert_eq!(usage.input_tokens, 8);
        assert_eq!(usage.total_tokens, 8);
        // completion_tokens absent in embed responses
        assert_eq!(usage.output_tokens, 0);
    }

    #[test]
    fn parse_stream_chunk() {
        let json = r#"{"choices":[{"delta":{"content":"Hi"},"finish_reason":null}]}"#;

        let resp: StreamChatResponse = serde_json::from_str(json).unwrap();

        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("Hi"));
        assert!(resp.choices[0].finish_reason.is_none());
    }

    #[tokio::test]
    async fn missing_api_key_returns_auth_error() {
        let adapter = OpenAIAdapter::new().unwrap();
        let config = RouterConfig {
            url: "https://api.openai.com".to_string(),
            api_key_env: Some("__NONEXISTENT_OPENAI_KEY_FOR_TEST__".to_string()),
            api_key: None,
            enabled: true,
            timeout_ms: None,
            headers: std::collections::HashMap::new(),
        };
        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: Some("gpt-4o-mini".to_string()),
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: Some(64),
                temperature: None,
            },
            budget: None,
        };

        let result = adapter.execute(&config, &request).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, GatewayError::Authentication { .. }),
            "expected Authentication error, got: {err:?}",
        );
    }

    #[test]
    fn openai_supports_image_generate() {
        let adapter = OpenAIAdapter::new().unwrap();
        assert!(adapter.supports(&Capability::ImageGenerate));
    }

    #[test]
    fn build_image_generate_request() {
        let body = ImageGenerateRequest {
            model: "dall-e-3".to_string(),
            prompt: "A sunset over mountains".to_string(),
            size: Some("1792x1024".to_string()),
            quality: Some("hd".to_string()),
            style: Some("vivid".to_string()),
            n: 1,
        };

        let json = serde_json::to_value(&body).unwrap();

        assert_eq!(json["model"], "dall-e-3");
        assert_eq!(json["prompt"], "A sunset over mountains");
        assert_eq!(json["size"], "1792x1024");
        assert_eq!(json["quality"], "hd");
        assert_eq!(json["style"], "vivid");
        assert_eq!(json["n"], 1);
    }

    #[test]
    fn parse_image_generate_response() {
        let json = r#"{
            "data": [
                {
                    "url": "https://oaidalleapiprodscus.blob.core.windows.net/image1.png",
                    "revised_prompt": "A breathtaking sunset over snow-capped mountains"
                },
                {
                    "b64_json": "iVBORw0KGgo=",
                    "revised_prompt": "Another sunset variation"
                }
            ]
        }"#;

        let resp: ImageGenerateResponse = serde_json::from_str(json).unwrap();

        assert_eq!(resp.data.len(), 2);
        assert_eq!(
            resp.data[0].url.as_deref(),
            Some("https://oaidalleapiprodscus.blob.core.windows.net/image1.png"),
        );
        assert!(resp.data[0].b64_json.is_none());
        assert_eq!(
            resp.data[0].revised_prompt.as_deref(),
            Some("A breathtaking sunset over snow-capped mountains"),
        );
        assert!(resp.data[1].url.is_none());
        assert_eq!(resp.data[1].b64_json.as_deref(), Some("iVBORw0KGgo="));
        assert_eq!(
            resp.data[1].revised_prompt.as_deref(),
            Some("Another sunset variation"),
        );
    }

    #[tokio::test]
    #[ignore]
    async fn openai_chat_integration() {
        // Requires OPENAI_API_KEY env var
        let adapter = OpenAIAdapter::new().unwrap();
        let config = RouterConfig {
            url: "https://api.openai.com".to_string(),
            api_key_env: Some("OPENAI_API_KEY".to_string()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(30000),
            headers: std::collections::HashMap::new(),
        };
        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: Some("gpt-4o-mini".to_string()),
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
