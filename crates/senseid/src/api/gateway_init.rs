use std::sync::Arc;

use gateway::adapters::noop::NoopAdapter;
use gateway::adapters::{AdapterRegistry, InferenceAdapter};
use gateway::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager};
use gateway::types::config::GatewayConfig;
use gateway::Gateway;

use crate::gateway_keys;

/// Resolve the literal api_key for a router from the Keychain.
/// Returns None for routers that don't need a key or whose key isn't set.
///
/// # Blocking
///
/// Shells out to `/usr/bin/security` via `gateway_keys::get_key` (~50ms
/// per call). Callers in an async context MUST wrap invocations in
/// `tokio::task::spawn_blocking`. The `init_gateway` startup call site
/// is safe today only because the empty default config makes the
/// resolver a no-op; once the DB-load path populates routers, this
/// constraint becomes load-bearing.
pub fn keychain_api_key(router_id: &str) -> Option<String> {
    gateway_keys::get_key(router_id).ok()
}

/// Initialize the gateway with detected adapters but NO config.
///
/// The gateway starts unconfigured — it will return `NotConfigured` for any
/// inference calls until config is set via `gateway.update_config()`.
/// Config comes from the database (settings/services tables), set during
/// `sensei init` or through the API.
///
/// Adapters (providers) are auto-detected at startup:
/// - Ollama: probed at localhost:11434
/// - Anthropic: ANTHROPIC_API_KEY env var
/// - OpenAI: OPENAI_API_KEY env var
/// - Grok: XAI_API_KEY env var
/// - Noop: always registered as graceful degradation fallback
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

    // Register external-provider adapters unconditionally. With the
    // Keychain key-store wired in, the env var is no longer the only
    // signal that an API key exists. resolve_api_key (in
    // gateway::adapters::base) now prefers the literal `api_key` on
    // RouterConfig (populated from Keychain) before falling back to the
    // env var. Requests against an unconfigured router still fail
    // clearly at request time.
    match gateway::adapters::anthropic::AnthropicAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: Anthropic adapter registered");
            adapters.register(Arc::new(adapter) as Arc<dyn InferenceAdapter>).await;
        }
        Err(e) => tracing::warn!("Gateway: Anthropic adapter failed: {}", e),
    }
    match gateway::adapters::openai::OpenAIAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: OpenAI adapter registered");
            adapters.register(Arc::new(adapter) as Arc<dyn InferenceAdapter>).await;
        }
        Err(e) => tracing::warn!("Gateway: OpenAI adapter failed: {}", e),
    }
    match gateway::adapters::grok::GrokAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: Grok adapter registered");
            adapters.register(Arc::new(adapter) as Arc<dyn InferenceAdapter>).await;
        }
        Err(e) => tracing::warn!("Gateway: Grok adapter failed: {}", e),
    }

    // Baseline production config — ships the known router/model entries
    // that the setup wizard's Inference stage exposes. Without these,
    // refresh_router_keys would have no routers to populate even after the
    // user pastes a key. A future DB-load path can replace this via
    // `gw.update_config(...)` once table-driven configuration lands.
    let config = baseline_production_config();

    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());

    let gw = Gateway::new(config, adapters, cb);

    // Pre-populate any RouterConfig api_key fields from the Keychain.
    // Now meaningful: routers are present in the baseline config above.
    gw.refresh_router_keys(keychain_api_key).await;

    let adapter_list = gw.list_adapters().await;
    tracing::info!(
        "Gateway initialized (unconfigured) with adapters: {:?}",
        adapter_list
    );

    Arc::new(gw)
}

/// Minimal baseline production config — one entry per shipped router that
/// the setup wizard's Inference stage can configure. Model + chain
/// entries cover the headline capabilities (text_chat for OpenAI /
/// Anthropic / Ollama, image_generate for OpenAI). When more routers /
/// models are added, this is the one-place edit.
fn baseline_production_config() -> GatewayConfig {
    use gateway::types::capability::Capability;
    use gateway::types::config::{
        ChainEntry, FallbackChainConfig, FallbackTrigger, ModelConfig, RouterConfig,
    };
    use std::collections::HashMap;

    let mut routers: HashMap<String, RouterConfig> = HashMap::new();
    routers.insert("openai".into(), RouterConfig {
        url: "https://api.openai.com".into(),
        api_key_env: Some("OPENAI_API_KEY".into()),
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });
    routers.insert("anthropic".into(), RouterConfig {
        url: "https://api.anthropic.com".into(),
        api_key_env: Some("ANTHROPIC_API_KEY".into()),
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });
    routers.insert("ollama".into(), RouterConfig {
        url: format!("http://localhost:{}", sensei_bootstrap::OLLAMA_PORT),
        api_key_env: None,
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });

    let mut models: HashMap<String, ModelConfig> = HashMap::new();
    models.insert("dall-e-3".into(), ModelConfig {
        id: "dall-e-3".into(),
        api_model_id: Some("dall-e-3".into()),
        provider: "openai".into(),
        capabilities: vec![Capability::ImageGenerate],
        context_window: 0,
        max_output_tokens: 0,
        pricing: None,
    });
    models.insert("gpt-image-1".into(), ModelConfig {
        id: "gpt-image-1".into(),
        api_model_id: Some("gpt-image-1".into()),
        provider: "openai".into(),
        capabilities: vec![Capability::ImageGenerate],
        context_window: 0,
        max_output_tokens: 0,
        pricing: None,
    });
    models.insert("gpt-4o-mini".into(), ModelConfig {
        id: "gpt-4o-mini".into(),
        api_model_id: Some("gpt-4o-mini".into()),
        provider: "openai".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 128_000,
        max_output_tokens: 16_384,
        pricing: None,
    });
    models.insert("claude-sonnet".into(), ModelConfig {
        id: "claude-sonnet".into(),
        api_model_id: Some("claude-sonnet-4-5".into()),
        provider: "anthropic".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 200_000,
        max_output_tokens: 8_192,
        pricing: None,
    });

    let mut chains: HashMap<String, FallbackChainConfig> = HashMap::new();
    chains.insert("image_generate".into(), FallbackChainConfig {
        id: "image_generate".into(),
        capability: Capability::ImageGenerate,
        models: vec![ChainEntry {
            model: "dall-e-3".into(),
            router: Some("openai".into()),
            api_model_id: None,
            priority: 1,
        }],
        fallback_triggers: vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout],
    });
    chains.insert("text_chat".into(), FallbackChainConfig {
        id: "text_chat".into(),
        capability: Capability::TextChat,
        models: vec![
            ChainEntry {
                model: "claude-sonnet".into(),
                router: Some("anthropic".into()),
                api_model_id: None,
                priority: 1,
            },
            ChainEntry {
                model: "gpt-4o-mini".into(),
                router: Some("openai".into()),
                api_model_id: None,
                priority: 2,
            },
        ],
        fallback_triggers: vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout, FallbackTrigger::ProviderError],
    });

    GatewayConfig { routers, models, chains }
}

/// Probe Ollama at localhost:11434.
async fn probe_ollama() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    client
        .get(format!("http://localhost:{}/api/tags", sensei_bootstrap::OLLAMA_PORT))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Create a lightweight gateway for tests (noop adapter only, no HTTP probes).
#[cfg(test)]
pub async fn init_gateway_test() -> Arc<Gateway> {
    use gateway::types::capability::Capability;
    use gateway::types::config::*;
    use std::collections::HashMap;

    let adapters = AdapterRegistry::new();
    adapters
        .register(Arc::new(NoopAdapter) as Arc<dyn InferenceAdapter>)
        .await;

    // Tests need a minimal config so execute() doesn't return NotConfigured
    let mut routers = HashMap::new();
    routers.insert(
        "noop".into(),
        RouterConfig {
            url: "http://noop".into(),
            api_key_env: None,
            api_key: keychain_api_key("noop"),
            enabled: true,
            timeout_ms: None,
            headers: HashMap::new(),
        },
    );

    let mut models = HashMap::new();
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
    chains.insert(
        "text_chat".into(),
        FallbackChainConfig {
            id: "text_chat".into(),
            capability: Capability::TextChat,
            models: vec![ChainEntry {
                model: "noop".into(),
                router: Some("noop".into()),
                api_model_id: None,
                priority: 1,
            }],
            fallback_triggers: vec![],
        },
    );

    let config = GatewayConfig {
        routers,
        models,
        chains,
    };
    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());
    Arc::new(Gateway::new(config, adapters, cb))
}
