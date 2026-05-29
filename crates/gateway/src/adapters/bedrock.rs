//! AWS Bedrock adapter.
//!
//! Talks to the Bedrock Converse API via `aws-sdk-bedrockruntime`. Unlike
//! the HTTP-based adapters in this crate, Bedrock auth is handled by the
//! AWS SDK's credential-provider chain (env vars → shared credentials
//! file → IAM role → IMDS), and request signing is SigV4 under the
//! hood. As a result `RouterConfig.api_key` / `api_key_env` /
//! `RouterConfig.url` aren't used for auth — the SDK ignores them. The
//! adapter still uses `RouterConfig.headers` for any custom headers and
//! honours per-request `model` / `max_tokens` / `temperature`.
//!
//! Capability coverage: `TextChat` via the unified Converse API, which
//! standardises the message shape across Anthropic, Meta, Mistral, and
//! Amazon-hosted models on Bedrock. Embeddings (Titan Text Embeddings,
//! Cohere Embed) and streaming (`converse_stream`) are scoped as
//! follow-ups — the unified-chat path is the highest-value entry point.
//!
//! Configuration: [`BedrockAdapter::new`] does
//! `aws_config::load_defaults` (async, picks up region + credentials
//! from the standard provider chain). For tests or callers that need to
//! pin a region explicitly, use [`BedrockAdapter::with_region`].

use std::pin::Pin;

use async_trait::async_trait;
use aws_sdk_bedrockruntime::{
    Client,
    operation::converse::ConverseOutput as ConverseResponse,
    types::{
        ContentBlock, ConversationRole, ConverseOutput as ConverseOutputUnion,
        InferenceConfiguration, Message, SystemContentBlock,
    },
};
use futures::Stream;

use crate::adapters::InferenceAdapter;
use crate::types::capability::Capability;
use crate::types::config::RouterConfig;
use crate::types::cost::TokenUsage;
use crate::types::error::GatewayError;
use crate::types::request::{
    InferenceRequest, InferenceResponse, Message as GwMessage, MessageRole, Payload, StreamChunk,
};

const ADAPTER_ID: &str = "bedrock";
/// Sensible default model id when callers don't specify one. Anthropic's
/// Claude Sonnet 3.5 v2 is the most broadly available Bedrock chat
/// model at the time of writing.
const DEFAULT_MODEL: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";
const DEFAULT_MAX_TOKENS: i32 = 1024;

pub struct BedrockAdapter {
    client: Client,
}

impl BedrockAdapter {
    /// Load AWS config from the standard provider chain (env vars,
    /// shared credentials, IAM role, IMDS) and build a Bedrock client.
    /// Async because credential resolution may touch the filesystem or
    /// the IMDS endpoint.
    pub async fn new() -> Result<Self, GatewayError> {
        let config =
            aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Ok(Self {
            client: Client::new(&config),
        })
    }

    /// Same as [`Self::new`] but pins an explicit AWS region instead of
    /// relying on the provider chain (`AWS_REGION` env var, etc.).
    pub async fn with_region(region: impl Into<String>) -> Result<Self, GatewayError> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.into()))
            .load()
            .await;
        Ok(Self {
            client: Client::new(&config),
        })
    }

    fn err(message: impl Into<String>, status: Option<u16>) -> GatewayError {
        GatewayError::ProviderError {
            adapter: ADAPTER_ID.into(),
            message: message.into(),
            status,
        }
    }
}

#[async_trait]
impl InferenceAdapter for BedrockAdapter {
    fn id(&self) -> &str {
        ADAPTER_ID
    }

    fn supports(&self, capability: &Capability) -> bool {
        matches!(capability, Capability::TextChat)
    }

    async fn execute(
        &self,
        _config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        let Payload::Chat {
            messages,
            system,
            max_tokens,
            temperature,
        } = &request.payload
        else {
            return Err(Self::err(
                "BedrockAdapter only supports Payload::Chat",
                None,
            ));
        };

        let model_id = resolve_model(request);
        let bedrock_messages = build_messages(messages)?;
        let system_blocks = build_system(messages, system);

        let inference_cfg = InferenceConfiguration::builder()
            .max_tokens(
                max_tokens
                    .map(|n| n as i32)
                    .unwrap_or(DEFAULT_MAX_TOKENS),
            )
            .set_temperature(*temperature)
            .build();

        let mut builder = self
            .client
            .converse()
            .model_id(model_id.clone())
            .inference_config(inference_cfg);
        for m in bedrock_messages {
            builder = builder.messages(m);
        }
        for s in system_blocks {
            builder = builder.system(s);
        }

        let response = builder.send().await.map_err(map_sdk_error)?;

        let content = extract_text(&response);
        let usage = response.usage.as_ref().map(|u| TokenUsage {
            input_tokens: u.input_tokens.max(0) as u32,
            output_tokens: u.output_tokens.max(0) as u32,
            total_tokens: u.total_tokens.max(0) as u32,
        });

        Ok(InferenceResponse {
            success: true,
            content: if content.is_empty() { None } else { Some(content) },
            embeddings: None,
            transcription: None,
            audio: None,
            images: None,
            videos: None,
            model: Some(model_id),
            usage,
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        })
    }

    async fn stream(
        &self,
        _config: &RouterConfig,
        _request: &InferenceRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>, GatewayError>
    {
        // Bedrock supports `converse_stream` with event-stream framing.
        // The chunk shape is non-trivial (ContentBlockDelta /
        // MessageStop / etc.) and deserves its own commit + tests.
        Err(Self::err("Bedrock streaming is not yet implemented", None))
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-testable without an SDK client)
// ---------------------------------------------------------------------------

fn resolve_model(request: &InferenceRequest) -> String {
    request
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn role_to_bedrock(role: &MessageRole) -> Option<ConversationRole> {
    match role {
        MessageRole::User | MessageRole::Tool => Some(ConversationRole::User),
        MessageRole::Assistant => Some(ConversationRole::Assistant),
        MessageRole::System => None, // hoisted into SystemContentBlock
    }
}

/// Convert gateway messages into Bedrock Messages. System-role messages
/// are dropped here — they're hoisted into the `system` parameter by
/// [`build_system`].
fn build_messages(messages: &[GwMessage]) -> Result<Vec<Message>, GatewayError> {
    messages
        .iter()
        .filter_map(|m| role_to_bedrock(&m.role).map(|r| (r, m.content.clone())))
        .map(|(role, content)| {
            Message::builder()
                .role(role)
                .content(ContentBlock::Text(content))
                .build()
                .map_err(|e| {
                    BedrockAdapter::err(format!("build Bedrock message: {e}"), None)
                })
        })
        .collect()
}

/// Build the system-prompt blocks. The explicit `system` field on the
/// payload wins; otherwise concatenate any `MessageRole::System`
/// messages into separate blocks.
fn build_system(messages: &[GwMessage], system: &Option<String>) -> Vec<SystemContentBlock> {
    if let Some(s) = system.as_deref() {
        return vec![SystemContentBlock::Text(s.to_string())];
    }
    messages
        .iter()
        .filter(|m| m.role == MessageRole::System)
        .map(|m| SystemContentBlock::Text(m.content.clone()))
        .collect()
}

/// Pull all text out of the Converse response, concatenating across
/// content blocks. Bedrock can return a mix of `Text`, `ToolUse`,
/// `Image`, etc.; we surface only the text in this commit.
fn extract_text(response: &ConverseResponse) -> String {
    let Some(output) = response.output.as_ref() else {
        return String::new();
    };
    let ConverseOutputUnion::Message(msg) = output else {
        return String::new();
    };
    msg.content
        .iter()
        .filter_map(|cb| match cb {
            ContentBlock::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Map AWS SDK errors to the closest [`GatewayError`] variant. We don't
/// have access to HTTP status codes once the SDK has parsed the
/// response, so the matching is service-error-name based.
fn map_sdk_error<E: std::error::Error + Send + Sync + 'static>(
    err: aws_sdk_bedrockruntime::error::SdkError<E>,
) -> GatewayError {
    use aws_sdk_bedrockruntime::error::SdkError;
    let message = match &err {
        SdkError::ServiceError(svc) => format!(
            "service error: {}",
            svc.err()
        ),
        SdkError::DispatchFailure(_) => "dispatch failure (network)".into(),
        SdkError::TimeoutError(_) => "timeout".into(),
        SdkError::ResponseError(_) => "response parse error".into(),
        SdkError::ConstructionFailure(_) => "request construction failure".into(),
        _ => "unknown SDK error".into(),
    };
    // ThrottlingException is the SigV4 / Bedrock rate-limit signal; we
    // best-effort match on the rendered message rather than poking at
    // the SDK's typed errors (which differ per operation).
    if message.contains("ThrottlingException") || message.contains("TooManyRequests") {
        return GatewayError::RateLimit {
            adapter: ADAPTER_ID.into(),
            retry_after_ms: None,
        };
    }
    if message.contains("AccessDenied")
        || message.contains("UnauthorizedOperation")
        || message.contains("missing credentials")
    {
        return GatewayError::Authentication {
            adapter: ADAPTER_ID.into(),
            message,
        };
    }
    GatewayError::ProviderError {
        adapter: ADAPTER_ID.into(),
        message,
        status: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn user(content: &str) -> GwMessage {
        GwMessage {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
        }
    }

    fn assistant(content: &str) -> GwMessage {
        GwMessage {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_call_id: None,
        }
    }

    fn system_msg(content: &str) -> GwMessage {
        GwMessage {
            role: MessageRole::System,
            content: content.into(),
            tool_call_id: None,
        }
    }

    fn empty_request(model: Option<&str>) -> InferenceRequest {
        InferenceRequest {
            capability: Capability::TextChat,
            model: model.map(|s| s.to_string()),
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![user("hi")],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        }
    }

    #[test]
    fn resolve_model_falls_back_to_default_when_absent() {
        assert_eq!(resolve_model(&empty_request(None)), DEFAULT_MODEL);
    }

    #[test]
    fn resolve_model_respects_request_override() {
        let req = empty_request(Some("meta.llama3-1-70b-instruct-v1:0"));
        assert_eq!(resolve_model(&req), "meta.llama3-1-70b-instruct-v1:0");
    }

    #[test]
    fn role_to_bedrock_maps_user_and_assistant_and_drops_system() {
        assert!(matches!(
            role_to_bedrock(&MessageRole::User),
            Some(ConversationRole::User)
        ));
        assert!(matches!(
            role_to_bedrock(&MessageRole::Tool),
            Some(ConversationRole::User)
        ));
        assert!(matches!(
            role_to_bedrock(&MessageRole::Assistant),
            Some(ConversationRole::Assistant)
        ));
        assert!(role_to_bedrock(&MessageRole::System).is_none());
    }

    #[test]
    fn build_messages_skips_system_role_and_preserves_order() {
        let msgs = vec![
            system_msg("be concise"),
            user("hi"),
            assistant("hello"),
            user("how are you"),
        ];
        let bedrock_msgs = build_messages(&msgs).unwrap();
        assert_eq!(bedrock_msgs.len(), 3);
        assert!(matches!(bedrock_msgs[0].role, ConversationRole::User));
        assert!(matches!(bedrock_msgs[1].role, ConversationRole::Assistant));
        assert!(matches!(bedrock_msgs[2].role, ConversationRole::User));
        // Each message has a single text content block carrying the
        // original content string.
        match &bedrock_msgs[0].content[0] {
            ContentBlock::Text(t) => assert_eq!(t, "hi"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn build_system_prefers_explicit_field_over_messages() {
        let msgs = vec![system_msg("inline rules"), user("hi")];
        let explicit = Some("explicit rules".to_string());
        let blocks = build_system(&msgs, &explicit);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            SystemContentBlock::Text(t) => assert_eq!(t, "explicit rules"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn build_system_falls_back_to_system_role_messages() {
        let msgs = vec![system_msg("rule one"), system_msg("rule two"), user("hi")];
        let blocks = build_system(&msgs, &None);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn build_system_returns_empty_when_no_system_present() {
        let msgs = vec![user("hi")];
        let blocks = build_system(&msgs, &None);
        assert!(blocks.is_empty());
    }

    #[test]
    fn empty_messages_produce_empty_bedrock_list() {
        let msgs: Vec<GwMessage> = vec![];
        assert!(build_messages(&msgs).unwrap().is_empty());
    }
}
