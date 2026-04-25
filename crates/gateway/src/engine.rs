use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

use crate::adapters::AdapterRegistry;
use crate::circuit_breaker::CircuitBreakerManager;
use crate::selection::{ModelSelectionService, SelectionCriteria};
use crate::types::config::GatewayConfig;
use crate::types::error::GatewayError;
use crate::types::request::{InferenceRequest, InferenceResponse, Payload};
use crate::types::trace::{Attempt, AttemptStatus};

/// Core gateway orchestrator.
///
/// Resolves model candidates via [`ModelSelectionService`], walks fallback
/// chains, records attempts, and integrates the circuit breaker.
pub struct Gateway {
    config: Arc<RwLock<GatewayConfig>>,
    adapters: AdapterRegistry,
    circuit_breaker: CircuitBreakerManager,
}

impl Gateway {
    pub fn new(
        config: GatewayConfig,
        adapters: AdapterRegistry,
        circuit_breaker: CircuitBreakerManager,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            adapters,
            circuit_breaker,
        }
    }

    /// Execute an inference request, walking the fallback chain on failure.
    pub async fn execute(
        &self,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        // 1. Clone config from RwLock
        let config = self.config.read().await.clone();

        // 2. Build SelectionCriteria from request
        let input_tokens = estimate_input_tokens(&request.payload);
        let criteria = SelectionCriteria {
            capability: request.capability.clone(),
            model: request.model.clone(),
            router: request.router.clone(),
            chain: request.chain.clone(),
            budget: request.budget,
            input_tokens: Some(input_tokens),
        };

        // 3. Select all candidates
        let svc = ModelSelectionService::new(&config, &self.circuit_breaker);
        let result = svc.select_all(&criteria);

        // 4. No candidates?
        if result.all_candidates.is_empty() {
            return Err(GatewayError::NoCandidates {
                capability: request.capability.clone(),
            });
        }

        // 5. Get fallback triggers from chain (or empty)
        let fallback_triggers = result
            .chain
            .as_ref()
            .map(|c| c.fallback_triggers.as_slice())
            .unwrap_or(&[]);

        // 6. Walk candidates in order
        let mut attempts: Vec<Attempt> = Vec::new();

        for (sequence, candidate) in (1_u8..).zip(result.all_candidates.iter()) {
            let start = Instant::now();

            // Get adapter from registry
            let adapter = match self.adapters.get(&candidate.router).await {
                Some(a) => a,
                None => {
                    attempts.push(Attempt {
                        sequence,
                        adapter: candidate.router.clone(),
                        model: candidate.model.clone(),
                        api_model_id: candidate.api_model_id.clone(),
                        status: AttemptStatus::Failed,
                        duration_ms: start.elapsed().as_millis() as u64,
                        tokens: None,
                        cost: None,
                        error: Some(format!(
                            "no adapter registered for router '{}'",
                            candidate.router
                        )),
                        fallback_triggered: false,
                    });
                    continue;
                }
            };

            // Execute via adapter
            let endpoint = format!("{}:{}", candidate.router, candidate.model);
            match adapter
                .execute(&candidate.router_config, request)
                .await
            {
                Ok(mut response) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    self.circuit_breaker.record_success(&endpoint);

                    attempts.push(Attempt {
                        sequence,
                        adapter: candidate.router.clone(),
                        model: candidate.model.clone(),
                        api_model_id: candidate.api_model_id.clone(),
                        status: AttemptStatus::Success,
                        duration_ms,
                        tokens: response.usage.clone(),
                        cost: response.actual_cost.as_ref().map(|c| c.total_cost),
                        error: None,
                        fallback_triggered: false,
                    });

                    response.attempts = attempts;
                    response.model = Some(candidate.model.clone());
                    return Ok(response);
                }
                Err(err) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    self.circuit_breaker.record_failure(&endpoint);

                    let should_fallback = err.should_trigger_fallback(fallback_triggers);

                    attempts.push(Attempt {
                        sequence,
                        adapter: candidate.router.clone(),
                        model: candidate.model.clone(),
                        api_model_id: candidate.api_model_id.clone(),
                        status: AttemptStatus::Failed,
                        duration_ms,
                        tokens: None,
                        cost: None,
                        error: Some(err.to_string()),
                        fallback_triggered: should_fallback,
                    });

                    if should_fallback {
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

        // 7. All candidates exhausted
        Err(GatewayError::AllAttemptsFailed {
            attempts: attempts.len(),
        })
    }

    /// Replace the gateway configuration at runtime.
    pub async fn update_config(&self, config: GatewayConfig) {
        let mut guard = self.config.write().await;
        *guard = config;
    }
}

/// Estimate input token count from the request payload.
///
/// Rough heuristic: 1 token ~ 4 characters.
fn estimate_input_tokens(payload: &Payload) -> u32 {
    match payload {
        Payload::Chat {
            messages, system, ..
        } => {
            let msg_chars: usize = messages.iter().map(|m| m.content.len()).sum();
            let sys_chars: usize = system.as_ref().map(|s| s.len()).unwrap_or(0);
            ((msg_chars + sys_chars) / 4) as u32
        }
        Payload::Embed { texts } => {
            let total_chars: usize = texts.iter().map(|t| t.len()).sum();
            (total_chars / 4) as u32
        }
        // STT has no meaningful text input to estimate.
        Payload::Stt { .. } => 0,
        // For TTS, rough heuristic on text length.
        Payload::Tts { text, .. } => (text.len() / 4) as u32,
        // Image generation: estimate based on prompt length.
        Payload::ImageGenerate { prompt, .. } => (prompt.len() / 4) as u32,
        // Video generation: estimate based on prompt length.
        Payload::VideoGenerate { prompt, .. } => (prompt.len() / 4) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::noop::NoopAdapter;
    use crate::circuit_breaker::CircuitBreakerConfig;
    use crate::types::capability::Capability;
    use crate::types::config::{
        ChainEntry, FallbackChainConfig, FallbackTrigger, GatewayConfig, ModelConfig, RouterConfig,
    };
    use crate::types::request::{Message, MessageRole, Payload};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    fn test_config_with_noop() -> GatewayConfig {
        let mut routers = HashMap::new();
        routers.insert(
            "noop".to_string(),
            RouterConfig {
                url: "http://localhost".to_string(),
                api_key_env: None,
                enabled: true,
                timeout_ms: None,
                headers: HashMap::new(),
            },
        );

        let mut models = HashMap::new();
        models.insert(
            "noop".to_string(),
            ModelConfig {
                id: "noop".to_string(),
                api_model_id: None,
                provider: "noop".to_string(),
                capabilities: vec![Capability::TextChat, Capability::TextEmbed],
                context_window: 4096,
                max_output_tokens: 1024,
                pricing: None,
            },
        );

        let mut chains = HashMap::new();
        chains.insert(
            "chat_chain".to_string(),
            FallbackChainConfig {
                id: "chat_chain".to_string(),
                capability: Capability::TextChat,
                models: vec![ChainEntry {
                    model: "noop".to_string(),
                    router: Some("noop".to_string()),
                    api_model_id: None,
                    priority: 1,
                }],
                fallback_triggers: vec![
                    FallbackTrigger::RateLimit,
                    FallbackTrigger::Timeout,
                    FallbackTrigger::ProviderError,
                ],
            },
        );

        GatewayConfig {
            routers,
            models,
            chains,
        }
    }

    fn test_gateway() -> Gateway {
        let config = test_config_with_noop();
        let adapters = AdapterRegistry::new();
        let cb = CircuitBreakerManager::new(CircuitBreakerConfig {
            threshold: 5,
            timeout: Duration::from_secs(300),
            half_open_max_requests: 3,
        });

        let gw = Gateway::new(config, adapters, cb);
        gw
    }

    async fn register_noop(gw: &Gateway) {
        gw.adapters
            .register(Arc::new(NoopAdapter) as Arc<dyn crate::adapters::InferenceAdapter>)
            .await;
    }

    fn chat_request() -> InferenceRequest {
        InferenceRequest {
            capability: Capability::TextChat,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "Hello, world!".to_string(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        }
    }

    #[tokio::test]
    async fn execute_with_noop_adapter() {
        let gw = test_gateway();
        register_noop(&gw).await;

        let response = gw.execute(&chat_request()).await.unwrap();

        assert!(!response.success);
        assert!(response
            .content
            .as_ref()
            .unwrap()
            .contains("No inference provider"));
    }

    #[tokio::test]
    async fn execute_no_candidates_errors() {
        let gw = test_gateway();
        register_noop(&gw).await;

        // VoiceStt has no chain configured
        let request = InferenceRequest {
            capability: Capability::AudioTranscribe,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "transcribe".to_string(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        };

        let result = gw.execute(&request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            GatewayError::NoCandidates { capability } => {
                assert_eq!(capability, Capability::AudioTranscribe);
            }
            other => panic!("Expected NoCandidates, got: {other}"),
        }
    }

    #[tokio::test]
    async fn execute_with_direct_model() {
        let gw = test_gateway();
        register_noop(&gw).await;

        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: Some("noop".to_string()),
            router: Some("noop".to_string()),
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        };

        let response = gw.execute(&request).await.unwrap();
        assert_eq!(response.model, Some("noop".to_string()));
    }

    #[tokio::test]
    async fn execute_records_attempts() {
        let gw = test_gateway();
        register_noop(&gw).await;

        let response = gw.execute(&chat_request()).await.unwrap();

        assert!(!response.attempts.is_empty());
        assert_eq!(response.attempts[0].adapter, "noop");
    }

    #[tokio::test]
    async fn execute_update_config_takes_effect() {
        let gw = test_gateway();
        register_noop(&gw).await;

        // First: verify execute works
        let response = gw.execute(&chat_request()).await;
        assert!(response.is_ok());

        // Update config to empty — no routers, no models, no chains
        gw.update_config(GatewayConfig::default()).await;

        // Now execute should fail with NoCandidates
        let result = gw.execute(&chat_request()).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GatewayError::NoCandidates { .. }
        ));
    }
}
