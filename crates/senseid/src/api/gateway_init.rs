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

    // Start with EMPTY config — no routers, models, or chains.
    // Config must be set via update_config() after loading from DB.
    let config = GatewayConfig::default();

    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());

    let gw = Gateway::new(config, adapters, cb);

    // Pre-populate any RouterConfig api_key fields from the Keychain.
    // Config starts empty here (set later via update_config from DB), so this
    // is a no-op at startup. The call site establishes the pattern used by
    // the DB config-loading path and by set/clear key handlers at runtime.
    gw.refresh_router_keys(keychain_api_key).await;

    let adapter_list = gw.list_adapters().await;
    tracing::info!(
        "Gateway initialized (unconfigured) with adapters: {:?}",
        adapter_list
    );

    Arc::new(gw)
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
