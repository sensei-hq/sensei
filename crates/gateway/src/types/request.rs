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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
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
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
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
            capability: Capability::Chat,
            model: Some("claude-sonnet".to_string()),
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    tool_call_id: None,
                }],
                system: Some("You are helpful.".to_string()),
                max_tokens: Some(1024),
                temperature: Some(0.7),
            },
            budget: Some(1.0),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: InferenceRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.capability, Capability::Chat);
        assert_eq!(deserialized.model, Some("claude-sonnet".to_string()));
        if let Payload::Chat {
            messages, system, ..
        } = &deserialized.payload
        {
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].role, MessageRole::User);
            assert_eq!(messages[0].content, "Hello");
            assert_eq!(system.as_deref(), Some("You are helpful."));
        } else {
            panic!("Expected Chat payload");
        }
    }

    #[test]
    fn embed_request_serde() {
        let request = InferenceRequest {
            capability: Capability::Embed,
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
            model: Some("claude-sonnet".to_string()),
            usage: Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            }),
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
    fn stt_request_serde() {
        let audio_bytes = vec![0xFF, 0xD8, 0x00, 0x10, 0x4A, 0x46];
        let request = InferenceRequest {
            capability: Capability::VoiceStt,
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
            capability: Capability::VoiceTts,
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
            model: Some("whisper-1".to_string()),
            usage: None,
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
            model: Some("tts-1".to_string()),
            usage: None,
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
}
