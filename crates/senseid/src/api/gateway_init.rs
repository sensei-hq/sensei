use std::sync::Arc;

use gateway::adapters::noop::NoopAdapter;
use gateway::adapters::{AdapterRegistry, InferenceAdapter};
use gateway::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager};
use gateway::types::capability::Capability;
use gateway::types::config::*;
use gateway::Gateway;

/// Initialize the gateway with available providers.
///
/// Probes Ollama, checks env vars for external providers, then builds a
/// default fallback-chain config for text chat and embedding.
pub async fn init_gateway() -> Arc<Gateway> {
    let adapters = AdapterRegistry::new();

    // Always register noop as fallback
    adapters
        .register(Arc::new(NoopAdapter) as Arc<dyn InferenceAdapter>)
        .await;

    // Probe Ollama
    if probe_ollama().await {
        match gateway::adapters::ollama::OllamaAdapter::new() {
            Ok(adapter) => {
                tracing::info!("Gateway: Ollama adapter registered");
                adapters
                    .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
                    .await;
            }
            Err(e) => tracing::warn!("Gateway: Ollama adapter failed to initialize: {}", e),
        }
    } else {
        tracing::info!("Gateway: Ollama not available, skipping");
    }

    // Check for Anthropic API key
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        match gateway::adapters::anthropic::AnthropicAdapter::new() {
            Ok(adapter) => {
                tracing::info!("Gateway: Anthropic adapter registered");
                adapters
                    .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
                    .await;
            }
            Err(e) => tracing::warn!("Gateway: Anthropic adapter failed: {}", e),
        }
    }

    // Check for OpenAI API key
    if std::env::var("OPENAI_API_KEY").is_ok() {
        match gateway::adapters::openai::OpenAIAdapter::new() {
            Ok(adapter) => {
                tracing::info!("Gateway: OpenAI adapter registered");
                adapters
                    .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
                    .await;
            }
            Err(e) => tracing::warn!("Gateway: OpenAI adapter failed: {}", e),
        }
    }

    // Check for xAI/Grok API key
    if std::env::var("XAI_API_KEY").is_ok() {
        match gateway::adapters::grok::GrokAdapter::new() {
            Ok(adapter) => {
                tracing::info!("Gateway: Grok adapter registered");
                adapters
                    .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
                    .await;
            }
            Err(e) => tracing::warn!("Gateway: Grok adapter failed: {}", e),
        }
    }

    // Build a minimal default config.
    // In production this will load from the DB.
    let config = build_default_config();

    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());

    let gw = Gateway::new(config, adapters, cb);

    let adapter_list = gw.list_adapters().await;
    tracing::info!("Gateway initialized with adapters: {:?}", adapter_list);

    Arc::new(gw)
}

/// Probe Ollama at localhost:11434.
async fn probe_ollama() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Create a lightweight gateway for tests (noop adapter only, no HTTP probes).
#[cfg(test)]
pub async fn init_gateway_test() -> Arc<Gateway> {
    let adapters = AdapterRegistry::new();
    adapters
        .register(Arc::new(NoopAdapter) as Arc<dyn InferenceAdapter>)
        .await;
    let config = build_default_config();
    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());
    Arc::new(Gateway::new(config, adapters, cb))
}

/// Build a default gateway config with common chains.
fn build_default_config() -> GatewayConfig {
    use std::collections::HashMap;

    let mut routers = HashMap::new();
    routers.insert(
        "ollama".into(),
        RouterConfig {
            url: "http://localhost:11434".into(),
            api_key_env: None,
            enabled: true,
            timeout_ms: Some(30_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "anthropic".into(),
        RouterConfig {
            url: "https://api.anthropic.com".into(),
            api_key_env: Some("ANTHROPIC_API_KEY".into()),
            enabled: true,
            timeout_ms: Some(60_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "openai".into(),
        RouterConfig {
            url: "https://api.openai.com".into(),
            api_key_env: Some("OPENAI_API_KEY".into()),
            enabled: true,
            timeout_ms: Some(60_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "grok".into(),
        RouterConfig {
            url: "https://api.x.ai".into(),
            api_key_env: Some("XAI_API_KEY".into()),
            enabled: true,
            timeout_ms: Some(60_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "noop".into(),
        RouterConfig {
            url: "http://noop".into(),
            api_key_env: None,
            enabled: true,
            timeout_ms: None,
            headers: HashMap::new(),
        },
    );

    let mut models = HashMap::new();
    // Local models (Ollama)
    models.insert(
        "gemma3:27b".into(),
        ModelConfig {
            id: "gemma3:27b".into(),
            api_model_id: None,
            provider: "ollama".into(),
            capabilities: vec![Capability::TextChat, Capability::TextComplete],
            context_window: 8192,
            max_output_tokens: 2048,
            pricing: None,
        },
    );
    models.insert(
        "all-minilm:l6-v2".into(),
        ModelConfig {
            id: "all-minilm:l6-v2".into(),
            api_model_id: None,
            provider: "ollama".into(),
            capabilities: vec![Capability::TextEmbed],
            context_window: 512,
            max_output_tokens: 0,
            pricing: None,
        },
    );
    // Noop fallback
    models.insert(
        "noop".into(),
        ModelConfig {
            id: "noop".into(),
            api_model_id: None,
            provider: "noop".into(),
            capabilities: vec![Capability::TextChat, Capability::TextEmbed],
            context_window: 4096,
            max_output_tokens: 1024,
            pricing: None,
        },
    );

    let mut chains = HashMap::new();
    // Text chat chain: local -> noop
    chains.insert(
        "text_chat".into(),
        FallbackChainConfig {
            id: "text_chat".into(),
            capability: Capability::TextChat,
            models: vec![
                ChainEntry {
                    model: "gemma3:27b".into(),
                    router: Some("ollama".into()),
                    api_model_id: None,
                    priority: 1,
                },
                ChainEntry {
                    model: "noop".into(),
                    router: Some("noop".into()),
                    api_model_id: None,
                    priority: 99,
                },
            ],
            fallback_triggers: vec![
                FallbackTrigger::Timeout,
                FallbackTrigger::ProviderError,
                FallbackTrigger::ModelUnavailable,
            ],
        },
    );
    // Embed chain: local only
    chains.insert(
        "text_embed".into(),
        FallbackChainConfig {
            id: "text_embed".into(),
            capability: Capability::TextEmbed,
            models: vec![
                ChainEntry {
                    model: "all-minilm:l6-v2".into(),
                    router: Some("ollama".into()),
                    api_model_id: None,
                    priority: 1,
                },
                ChainEntry {
                    model: "noop".into(),
                    router: Some("noop".into()),
                    api_model_id: None,
                    priority: 99,
                },
            ],
            fallback_triggers: vec![FallbackTrigger::Timeout, FallbackTrigger::ModelUnavailable],
        },
    );

    GatewayConfig {
        routers,
        models,
        chains,
    }
}
