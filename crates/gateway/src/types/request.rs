use serde::{Deserialize, Serialize};

use super::capability::Capability;
use super::cost::{Cost, CostEstimate, TokenUsage};
use super::trace::Attempt;

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
}
