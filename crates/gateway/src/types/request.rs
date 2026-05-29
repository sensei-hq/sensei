use serde::{Deserialize, Serialize};

use super::capability::Capability;
use super::cost::{Cost, CostEstimate, TokenUsage};
use super::trace::Attempt;

// ---------------------------------------------------------------------------
// Base64 serde helpers for audio byte fields
// ---------------------------------------------------------------------------

mod base64_serde {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

mod option_base64_serde {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match data {
            Some(bytes) => s.serialize_str(&STANDARD.encode(bytes)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let opt: Option<String> = Option::deserialize(d)?;
        match opt {
            Some(s) => STANDARD
                .decode(&s)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Audio types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Mp3,
    Wav,
    Opus,
    Pcm,
    Flac,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Mp3 => write!(f, "mp3"),
            AudioFormat::Wav => write!(f, "wav"),
            AudioFormat::Opus => write!(f, "opus"),
            AudioFormat::Pcm => write!(f, "pcm"),
            AudioFormat::Flac => write!(f, "flac"),
        }
    }
}

fn default_audio_format() -> String {
    "wav".into()
}

fn default_tts_format() -> AudioFormat {
    AudioFormat::Mp3
}

fn default_image_count() -> u8 {
    1
}

// ---------------------------------------------------------------------------
// Messages / Payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Content body of a single chat message.
///
/// `Text` carries an ordinary string (user prompt, assistant reply,
/// system instruction). `ToolResult` carries the caller's response to
/// a prior tool call and links it back via `tool_call_id`. The
/// surrounding [`Message`] still carries the assistant's tool call
/// emissions in the separate `tool_calls` field, so a single struct
/// can model both "assistant said X" and "assistant called tool T".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    Text { text: String },
    ToolResult { tool_call_id: String, content: String },
}

impl MessageContent {
    /// View the body as plain text. Returns the text for `Text`, the
    /// result body for `ToolResult`. Adapters that don't yet model
    /// tool calling natively can use this everywhere.
    pub fn as_text(&self) -> &str {
        match self {
            MessageContent::Text { text } => text.as_str(),
            MessageContent::ToolResult { content, .. } => content.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    /// Tool calls emitted by an assistant turn. Empty for any other
    /// role; non-empty implies `role == Assistant`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    /// Build a plain text message — user, assistant, or system.
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: MessageContent::Text { text: text.into() },
            tool_calls: Vec::new(),
        }
    }

    /// Build a tool-result message linked to a prior [`ToolCall`].
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::ToolResult {
                tool_call_id: tool_call_id.into(),
                content: content.into(),
            },
            tool_calls: Vec::new(),
        }
    }

    /// View the message body as plain text — shorthand for
    /// `self.content.as_text()`.
    pub fn as_text(&self) -> &str {
        self.content.as_text()
    }
}

/// A function/tool the model may call.
///
/// `input_schema` is a JSON Schema document describing the call's
/// argument object (typically `{type: "object", properties: {...},
/// required: [...]}`). We pass it through verbatim to every provider
/// — OpenAI/Anthropic/Gemini/Bedrock all accept JSON Schema, with
/// only minor wrapping differences handled by the adapters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// A tool call emitted by the model. `arguments` is the call's
/// argument object encoded as a JSON string — matches OpenAI's wire
/// shape directly; adapters whose providers emit a native JSON object
/// (Anthropic, Gemini, Bedrock) serialize it on the way out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    Chat {
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: Option<f32>,
        /// Tool definitions the model may call. Empty disables
        /// tool calling for this turn even if the underlying
        /// provider would otherwise advertise tools.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tools: Vec<ToolDefinition>,
    },
    Embed {
        texts: Vec<String>,
    },
    Stt {
        /// Raw audio bytes, base64-encoded for serde.
        #[serde(with = "base64_serde")]
        audio: Vec<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// Audio format: "mp3", "wav", "webm", "m4a".
        #[serde(default = "default_audio_format")]
        format: String,
    },
    Tts {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        voice: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        speed: Option<f32>,
        #[serde(default = "default_tts_format")]
        output_format: AudioFormat,
    },
    ImageGenerate {
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        quality: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<String>,
        #[serde(default = "default_image_count")]
        n: u8,
    },
    VideoGenerate {
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_secs: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        resolution: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub capability: Capability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,
    pub payload: Payload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<Vec<Vec<f32>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "option_base64_serde",
        default
    )]
    pub audio: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub videos: Option<Vec<VideoResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// Tool calls emitted by the assistant for this turn. Empty when
    /// the model returned plain text. Caller is expected to dispatch
    /// each call and feed the results back via a follow-up request
    /// containing one [`Message::tool_result`] per call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost: Option<CostEstimate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<Cost>,
    pub attempts: Vec<Attempt>,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Chunk {
        content: String,
    },
    ProviderSwitch {
        from_adapter: String,
        from_model: String,
        to_adapter: String,
        to_model: String,
        reason: String,
    },
    Done {
        model: String,
        tokens: TokenUsage,
        cost: f64,
    },
    Error {
        code: String,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_serde() {
        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: Some("claude-sonnet".to_string()),
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message::text(MessageRole::User, "Hello")],
                system: Some("You are helpful.".to_string()),
                max_tokens: Some(1024),
                temperature: Some(0.7),
                tools: Vec::new(),
            },
            budget: Some(1.0),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.capability, Capability::TextChat);
        assert_eq!(deserialized.model, Some("claude-sonnet".to_string()));
        if let Payload::Chat {
            messages, system, ..
        } = &deserialized.payload
        {
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].role, MessageRole::User);
            assert_eq!(messages[0].as_text(), "Hello");
            assert_eq!(system.as_deref(), Some("You are helpful."));
        } else {
            panic!("Expected Chat payload");
        }
    }

    #[test]
    fn embed_request_serde() {
        let request = InferenceRequest {
            capability: Capability::TextEmbed,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Embed {
                texts: vec!["hello world".to_string()],
            },
            budget: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"embed""#));

        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();
        if let Payload::Embed { texts } = &deserialized.payload {
            assert_eq!(texts.len(), 1);
            assert_eq!(texts[0], "hello world");
        } else {
            panic!("Expected Embed payload");
        }
    }

    #[test]
    fn response_with_attempts() {
        let response = InferenceResponse {
            success: true,
            content: Some("Hello!".to_string()),
            embeddings: None,
            transcription: None,
            audio: None,
            images: None,
            videos: None,
            model: Some("claude-sonnet".to_string()),
            usage: Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            }),
            tool_calls: Vec::new(),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: InferenceResponse = serde_json::from_str(&json).unwrap();

        assert!(deserialized.success);
        assert_eq!(deserialized.content, Some("Hello!".to_string()));
        assert!(deserialized.attempts.is_empty());
    }

    #[test]
    fn message_text_helper_builds_text_content() {
        let m = Message::text(MessageRole::User, "hello");
        assert_eq!(m.role, MessageRole::User);
        assert_eq!(m.as_text(), "hello");
        assert!(m.tool_calls.is_empty());
        match m.content {
            MessageContent::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("expected Text content"),
        }
    }

    #[test]
    fn message_tool_result_helper_carries_call_id_and_role() {
        let m = Message::tool_result("call_abc", "{\"weather\":\"sunny\"}");
        assert_eq!(m.role, MessageRole::Tool);
        match &m.content {
            MessageContent::ToolResult {
                tool_call_id,
                content,
            } => {
                assert_eq!(tool_call_id, "call_abc");
                assert_eq!(content, "{\"weather\":\"sunny\"}");
            }
            _ => panic!("expected ToolResult content"),
        }
        // as_text exposes the result body for adapters that only
        // understand plain text.
        assert_eq!(m.as_text(), "{\"weather\":\"sunny\"}");
    }

    #[test]
    fn message_content_round_trips_through_serde_as_tagged_enum() {
        let text = Message::text(MessageRole::User, "hi");
        let json = serde_json::to_value(&text).unwrap();
        assert_eq!(json["content"]["type"], "text");
        assert_eq!(json["content"]["text"], "hi");

        let result = Message::tool_result("call_1", "body");
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["content"]["type"], "tool_result");
        assert_eq!(json["content"]["tool_call_id"], "call_1");
        assert_eq!(json["content"]["content"], "body");

        // Round-trip
        let deserialized: Message = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.role, MessageRole::Tool);
        assert_eq!(deserialized.as_text(), "body");
    }

    #[test]
    fn assistant_message_carries_tool_calls_alongside_content() {
        let msg = Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text {
                text: String::new(),
            },
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "get_weather".into(),
                arguments: "{\"city\":\"Berlin\"}".into(),
            }],
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["tool_calls"][0]["id"], "call_1");
        assert_eq!(json["tool_calls"][0]["name"], "get_weather");
        let deserialized: Message = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.tool_calls.len(), 1);
        assert_eq!(deserialized.tool_calls[0].arguments, "{\"city\":\"Berlin\"}");
    }

    #[test]
    fn tool_definition_round_trips_with_json_schema_pass_through() {
        let def = ToolDefinition {
            name: "get_weather".into(),
            description: Some("Look up the weather for a city.".into()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "city": {"type": "string"}
                },
                "required": ["city"]
            }),
        };
        let json = serde_json::to_value(&def).unwrap();
        let back: ToolDefinition = serde_json::from_value(json).unwrap();
        assert_eq!(back, def);
    }

    #[test]
    fn chat_payload_tools_field_omitted_from_json_when_empty() {
        let payload = Payload::Chat {
            messages: vec![Message::text(MessageRole::User, "hi")],
            system: None,
            max_tokens: None,
            temperature: None,
            tools: Vec::new(),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(json.get("tools").is_none(), "empty tools should be omitted");
    }

    #[test]
    fn chat_payload_tools_field_serializes_when_present() {
        let payload = Payload::Chat {
            messages: vec![Message::text(MessageRole::User, "hi")],
            system: None,
            max_tokens: None,
            temperature: None,
            tools: vec![ToolDefinition {
                name: "ping".into(),
                description: None,
                input_schema: serde_json::json!({"type": "object"}),
            }],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["tools"][0]["name"], "ping");
    }

    #[test]
    fn stt_request_serde() {
        let audio_bytes = vec![0xFF, 0xD8, 0x00, 0x10, 0x4A, 0x46];
        let request = InferenceRequest {
            capability: Capability::AudioTranscribe,
            model: Some("whisper-1".to_string()),
            router: None,
            chain: None,
            payload: Payload::Stt {
                audio: audio_bytes.clone(),
                language: Some("en".to_string()),
                format: "wav".to_string(),
            },
            budget: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"stt""#));

        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();
        if let Payload::Stt {
            audio,
            language,
            format,
        } = &deserialized.payload
        {
            assert_eq!(audio, &audio_bytes);
            assert_eq!(language.as_deref(), Some("en"));
            assert_eq!(format, "wav");
        } else {
            panic!("Expected Stt payload");
        }
    }

    #[test]
    fn tts_request_serde() {
        let request = InferenceRequest {
            capability: Capability::AudioGenerate,
            model: Some("tts-1".to_string()),
            router: None,
            chain: None,
            payload: Payload::Tts {
                text: "Hello world".to_string(),
                voice: Some("alloy".to_string()),
                speed: Some(1.0),
                output_format: AudioFormat::Mp3,
            },
            budget: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"tts""#));

        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();
        if let Payload::Tts {
            text,
            voice,
            speed,
            output_format,
        } = &deserialized.payload
        {
            assert_eq!(text, "Hello world");
            assert_eq!(voice.as_deref(), Some("alloy"));
            assert!((speed.unwrap() - 1.0).abs() < f32::EPSILON);
            assert_eq!(*output_format, AudioFormat::Mp3);
        } else {
            panic!("Expected Tts payload");
        }
    }

    #[test]
    fn audio_format_serde() {
        let formats = vec![
            (AudioFormat::Mp3, "\"mp3\""),
            (AudioFormat::Wav, "\"wav\""),
            (AudioFormat::Opus, "\"opus\""),
            (AudioFormat::Pcm, "\"pcm\""),
            (AudioFormat::Flac, "\"flac\""),
        ];

        for (format, expected_json) in formats {
            let json = serde_json::to_string(&format).unwrap();
            assert_eq!(json, expected_json);

            let deserialized: AudioFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, format);
        }
    }

    #[test]
    fn response_with_transcription_serde() {
        let response = InferenceResponse {
            success: true,
            content: None,
            embeddings: None,
            transcription: Some("Hello world".to_string()),
            audio: None,
            images: None,
            videos: None,
            model: Some("whisper-1".to_string()),
            usage: None,
            tool_calls: Vec::new(),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""transcription":"Hello world""#));

        let deserialized: InferenceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.transcription.as_deref(),
            Some("Hello world"),
        );
        assert!(deserialized.audio.is_none());
    }

    #[test]
    fn response_with_audio_serde() {
        let audio_bytes = vec![0xFF, 0xFB, 0x90, 0x00];
        let response = InferenceResponse {
            success: true,
            content: None,
            embeddings: None,
            transcription: None,
            audio: Some(audio_bytes.clone()),
            images: None,
            videos: None,
            model: Some("tts-1".to_string()),
            usage: None,
            tool_calls: Vec::new(),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        // Audio should be base64-encoded
        assert!(json.contains("\"audio\""));

        let deserialized: InferenceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.audio, Some(audio_bytes));
        assert!(deserialized.transcription.is_none());
    }

    #[test]
    fn image_generate_request_serde() {
        let request = InferenceRequest {
            capability: Capability::ImageGenerate,
            model: Some("dall-e-3".to_string()),
            router: None,
            chain: None,
            payload: Payload::ImageGenerate {
                prompt: "A sunset over mountains".to_string(),
                size: Some("1024x1024".to_string()),
                quality: Some("hd".to_string()),
                style: Some("natural".to_string()),
                n: 2,
            },
            budget: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"image_generate""#));

        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.capability, Capability::ImageGenerate);
        if let Payload::ImageGenerate {
            prompt,
            size,
            quality,
            style,
            n,
        } = &deserialized.payload
        {
            assert_eq!(prompt, "A sunset over mountains");
            assert_eq!(size.as_deref(), Some("1024x1024"));
            assert_eq!(quality.as_deref(), Some("hd"));
            assert_eq!(style.as_deref(), Some("natural"));
            assert_eq!(*n, 2);
        } else {
            panic!("Expected ImageGenerate payload");
        }
    }

    #[test]
    fn image_result_serde() {
        let result = ImageResult {
            url: Some("https://example.com/image.png".to_string()),
            b64_json: None,
            revised_prompt: Some("A beautiful sunset over snow-capped mountains".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ImageResult = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.url.as_deref(),
            Some("https://example.com/image.png"),
        );
        assert!(deserialized.b64_json.is_none());
        assert_eq!(
            deserialized.revised_prompt.as_deref(),
            Some("A beautiful sunset over snow-capped mountains"),
        );
    }

    #[test]
    fn response_with_images_serde() {
        let response = InferenceResponse {
            success: true,
            content: None,
            embeddings: None,
            transcription: None,
            audio: None,
            images: Some(vec![ImageResult {
                url: Some("https://example.com/img.png".to_string()),
                b64_json: None,
                revised_prompt: None,
            }]),
            videos: None,
            model: Some("dall-e-3".to_string()),
            usage: None,
            tool_calls: Vec::new(),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"images\""));

        let deserialized: InferenceResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.images.is_some());
        let images = deserialized.images.unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].url.as_deref(),
            Some("https://example.com/img.png"),
        );
    }

    #[test]
    fn image_generate_defaults() {
        // When n is omitted, it should default to 1
        let json = r#"{"type":"image_generate","prompt":"A cat"}"#;
        let payload: Payload = serde_json::from_str(json).unwrap();

        if let Payload::ImageGenerate { prompt, n, size, quality, style } = &payload {
            assert_eq!(prompt, "A cat");
            assert_eq!(*n, 1);
            assert!(size.is_none());
            assert!(quality.is_none());
            assert!(style.is_none());
        } else {
            panic!("Expected ImageGenerate payload");
        }
    }

    #[test]
    fn video_generate_request_serde() {
        let request = InferenceRequest {
            capability: Capability::VideoGenerate,
            model: None,
            router: None,
            chain: None,
            payload: Payload::VideoGenerate {
                prompt: "A timelapse of a blooming flower".to_string(),
                duration_secs: Some(10),
                resolution: Some("1080p".to_string()),
            },
            budget: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""type":"video_generate""#));

        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.capability, Capability::VideoGenerate);
        if let Payload::VideoGenerate {
            prompt,
            duration_secs,
            resolution,
        } = &deserialized.payload
        {
            assert_eq!(prompt, "A timelapse of a blooming flower");
            assert_eq!(*duration_secs, Some(10));
            assert_eq!(resolution.as_deref(), Some("1080p"));
        } else {
            panic!("Expected VideoGenerate payload");
        }
    }

    #[test]
    fn video_result_serde() {
        let result = VideoResult {
            url: Some("https://example.com/video.mp4".to_string()),
            duration_secs: Some(10.5),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: VideoResult = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.url.as_deref(),
            Some("https://example.com/video.mp4"),
        );
        assert!((deserialized.duration_secs.unwrap() - 10.5).abs() < f32::EPSILON);

        // Test with all fields None
        let empty = VideoResult {
            url: None,
            duration_secs: None,
        };
        let json = serde_json::to_string(&empty).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn response_with_videos_serde() {
        let response = InferenceResponse {
            success: true,
            content: None,
            embeddings: None,
            transcription: None,
            audio: None,
            images: None,
            videos: Some(vec![VideoResult {
                url: Some("https://example.com/video.mp4".to_string()),
                duration_secs: Some(5.0),
            }]),
            model: Some("video-gen-1".to_string()),
            usage: None,
            tool_calls: Vec::new(),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"videos\""));

        let deserialized: InferenceResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.videos.is_some());
        let videos = deserialized.videos.unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(
            videos[0].url.as_deref(),
            Some("https://example.com/video.mp4"),
        );
        assert!((videos[0].duration_secs.unwrap() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stt_payload_base64_roundtrip() {
        // Test that Stt audio bytes survive base64 serde round-trip
        let audio_bytes: Vec<u8> = (0..=255).collect();
        let payload = Payload::Stt {
            audio: audio_bytes.clone(),
            language: Some("en".to_string()),
            format: "mp3".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: Payload = serde_json::from_str(&json).unwrap();

        if let Payload::Stt {
            audio,
            language,
            format,
        } = &deserialized
        {
            assert_eq!(audio, &audio_bytes);
            assert_eq!(language.as_deref(), Some("en"));
            assert_eq!(format, "mp3");
        } else {
            panic!("Expected Stt payload");
        }
    }

    #[test]
    fn tts_payload_default_output_format() {
        // When output_format is omitted in JSON, it should default to mp3
        let json = r#"{"type":"tts","text":"Hello"}"#;
        let payload: Payload = serde_json::from_str(json).unwrap();

        if let Payload::Tts {
            text,
            voice,
            speed,
            output_format,
        } = &payload
        {
            assert_eq!(text, "Hello");
            assert!(voice.is_none());
            assert!(speed.is_none());
            assert_eq!(*output_format, AudioFormat::Mp3);
        } else {
            panic!("Expected Tts payload");
        }
    }

    #[test]
    fn stt_default_audio_format() {
        // When format is omitted in JSON, it should default to "wav"
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let audio_b64 = STANDARD.encode([0xFFu8, 0xFB]);
        let json = format!(r#"{{"type":"stt","audio":"{}"}}"#, audio_b64);
        let payload: Payload = serde_json::from_str(&json).unwrap();

        if let Payload::Stt { format, .. } = &payload {
            assert_eq!(format, "wav");
        } else {
            panic!("Expected Stt payload");
        }
    }

    #[test]
    fn image_result_all_fields() {
        let result = ImageResult {
            url: Some("https://example.com/img.png".to_string()),
            b64_json: Some("iVBORw0KGgo=".to_string()),
            revised_prompt: Some("A revised prompt".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ImageResult = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.url.as_deref(),
            Some("https://example.com/img.png"),
        );
        assert_eq!(deserialized.b64_json.as_deref(), Some("iVBORw0KGgo="));
        assert_eq!(
            deserialized.revised_prompt.as_deref(),
            Some("A revised prompt"),
        );
    }

    #[test]
    fn video_result_roundtrip_all_none() {
        let result = VideoResult {
            url: None,
            duration_secs: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, "{}");

        let deserialized: VideoResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.url.is_none());
        assert!(deserialized.duration_secs.is_none());
    }

    #[test]
    fn audio_format_display() {
        assert_eq!(AudioFormat::Mp3.to_string(), "mp3");
        assert_eq!(AudioFormat::Wav.to_string(), "wav");
        assert_eq!(AudioFormat::Opus.to_string(), "opus");
        assert_eq!(AudioFormat::Pcm.to_string(), "pcm");
        assert_eq!(AudioFormat::Flac.to_string(), "flac");
    }

    #[test]
    fn response_with_no_audio_field() {
        // Test that response without audio field deserializes correctly
        let json = r#"{"success":true,"attempts":[]}"#;
        let deserialized: InferenceResponse = serde_json::from_str(json).unwrap();
        assert!(deserialized.audio.is_none());
        assert!(deserialized.content.is_none());
        assert!(deserialized.embeddings.is_none());
    }

    #[test]
    fn stream_event_variants() {
        // Verify StreamEvent and StreamChunk structs can be constructed
        let chunk = StreamChunk {
            content: "hello".to_string(),
            finish_reason: Some("stop".to_string()),
            usage: Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            }),
        };
        assert_eq!(chunk.content, "hello");
        assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
        assert!(chunk.usage.is_some());

        let event = StreamEvent::Chunk {
            content: "hi".to_string(),
        };
        match event {
            StreamEvent::Chunk { content } => assert_eq!(content, "hi"),
            _ => panic!("wrong variant"),
        }

        let event = StreamEvent::Done {
            model: "m".to_string(),
            tokens: TokenUsage::default(),
            cost: 0.01,
        };
        match event {
            StreamEvent::Done { model, cost, .. } => {
                assert_eq!(model, "m");
                assert!((cost - 0.01).abs() < f64::EPSILON);
            }
            _ => panic!("wrong variant"),
        }

        let event = StreamEvent::ProviderSwitch {
            from_adapter: "a".into(),
            from_model: "m1".into(),
            to_adapter: "b".into(),
            to_model: "m2".into(),
            reason: "fallback".into(),
        };
        match event {
            StreamEvent::ProviderSwitch {
                from_adapter,
                to_adapter,
                ..
            } => {
                assert_eq!(from_adapter, "a");
                assert_eq!(to_adapter, "b");
            }
            _ => panic!("wrong variant"),
        }

        let event = StreamEvent::Error {
            code: "500".into(),
            message: "fail".into(),
        };
        match event {
            StreamEvent::Error { code, message } => {
                assert_eq!(code, "500");
                assert_eq!(message, "fail");
            }
            _ => panic!("wrong variant"),
        }
    }
}
