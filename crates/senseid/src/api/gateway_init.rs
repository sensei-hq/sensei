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

    // Optional in-process embedding adapter. Only active when the binary
    // is built with `--features embedded-fastembed` AND the operator
    // points `SENSEI_FASTEMBED_DIR` at a fastembed-compatible ONNX
    // directory. Lets the daemon serve embeddings without going through
    // the local Ollama HTTP layer — see docs/backlog.md (Future scope —
    // gateway-embedded) for the design rationale.
    #[cfg(feature = "embedded-fastembed")]
    if let Ok(dir) = std::env::var("SENSEI_FASTEMBED_DIR") {
        let model_id = std::env::var("SENSEI_FASTEMBED_MODEL_ID")
            .unwrap_or_else(|_| "fastembed-default".to_string());
        match crate::api::gateway_embedded::register_fastembed(&adapters, &dir, &model_id).await {
            Ok(id) => tracing::info!(
                "Gateway: FastembedAdapter registered as '{}' for model '{}' from {}",
                id, model_id, dir
            ),
            Err(e) => tracing::warn!(
                "Gateway: FastembedAdapter from SENSEI_FASTEMBED_DIR={} failed: {}",
                dir, e
            ),
        }
    }

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
    // OpenAI-compatible aggregators / routers. Each uses the same wire
    // format as OpenAI but with a different base URL + API key. The
    // `with_id` constructor lets a single adapter implementation be
    // registered under multiple router-matching ids so the gateway
    // engine (which dispatches by router id) can pick the right
    // RouterConfig per request.
    register_openai_compatible(&adapters, "openrouter").await;
    register_openai_compatible(&adapters, "vercel").await;
    register_openai_compatible(&adapters, "nvidia").await;
    match gateway::adapters::grok::GrokAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: Grok adapter registered");
            adapters.register(Arc::new(adapter) as Arc<dyn InferenceAdapter>).await;
        }
        Err(e) => tracing::warn!("Gateway: Grok adapter failed: {}", e),
    }
    match gateway::adapters::gemini::GeminiAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: Gemini adapter registered");
            adapters.register(Arc::new(adapter) as Arc<dyn InferenceAdapter>).await;
        }
        Err(e) => tracing::warn!("Gateway: Gemini adapter failed: {}", e),
    }
    // Bedrock loads AWS credentials lazily via the SDK's provider
    // chain (env vars → shared credentials → IAM role → IMDS). The
    // adapter constructs without credentials present; requests will
    // fail at execute-time if no credentials resolve.
    match gateway::adapters::bedrock::BedrockAdapter::new().await {
        Ok(adapter) => {
            tracing::info!("Gateway: Bedrock adapter registered");
            adapters.register(Arc::new(adapter) as Arc<dyn InferenceAdapter>).await;
        }
        Err(e) => tracing::warn!("Gateway: Bedrock adapter failed: {}", e),
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
    // OpenAI-compatible aggregators. The adapter implementation is the
    // same as OpenAI's; each router has its own base URL + key env var.
    routers.insert("openrouter".into(), RouterConfig {
        url: "https://openrouter.ai/api/v1".into(),
        api_key_env: Some("OPENROUTER_API_KEY".into()),
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });
    routers.insert("vercel".into(), RouterConfig {
        url: "https://ai-gateway.vercel.sh/v1".into(),
        api_key_env: Some("AI_GATEWAY_API_KEY".into()),
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });
    routers.insert("nvidia".into(), RouterConfig {
        url: "https://integrate.api.nvidia.com/v1".into(),
        api_key_env: Some("NVIDIA_API_KEY".into()),
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });
    // Google Gemini uses its own (non-OpenAI) wire format — see
    // adapters::gemini for the format and auth header details.
    routers.insert("gemini".into(), RouterConfig {
        url: "https://generativelanguage.googleapis.com/v1beta".into(),
        api_key_env: Some("GEMINI_API_KEY".into()),
        api_key: None,
        enabled: true,
        timeout_ms: Some(120_000),
        headers: HashMap::new(),
    });
    // AWS Bedrock — the SDK handles auth via the credential-provider
    // chain (AWS_ACCESS_KEY_ID / shared credentials / IAM role) and the
    // request URL is derived from the chosen region, so the
    // RouterConfig.url + api_key fields aren't used. The url is set to
    // a non-empty placeholder so existing config-validation paths that
    // require a non-empty url stay happy.
    routers.insert("bedrock".into(), RouterConfig {
        url: "aws://bedrock".into(),
        api_key_env: Some("AWS_ACCESS_KEY_ID".into()),
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
    // One representative model per OpenAI-compatible aggregator so the
    // router entries above have something to dispatch to out of the box.
    // The DB-load path / setup wizard can add more once table-driven
    // configuration lands.
    models.insert("openrouter-claude-sonnet-4-5".into(), ModelConfig {
        id: "openrouter-claude-sonnet-4-5".into(),
        api_model_id: Some("anthropic/claude-sonnet-4-5".into()),
        provider: "openrouter".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 200_000,
        max_output_tokens: 8_192,
        pricing: None,
    });
    models.insert("vercel-gpt-4o".into(), ModelConfig {
        id: "vercel-gpt-4o".into(),
        api_model_id: Some("openai/gpt-4o".into()),
        provider: "vercel".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 128_000,
        max_output_tokens: 16_384,
        pricing: None,
    });
    models.insert("nvidia-llama-3.1-70b-instruct".into(), ModelConfig {
        id: "nvidia-llama-3.1-70b-instruct".into(),
        api_model_id: Some("meta/llama-3.1-70b-instruct".into()),
        provider: "nvidia".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 128_000,
        max_output_tokens: 4_096,
        pricing: None,
    });
    // Gemini — one chat model + one embedding model so both
    // capabilities the adapter supports have a dispatch target.
    models.insert("gemini-2.0-flash".into(), ModelConfig {
        id: "gemini-2.0-flash".into(),
        api_model_id: Some("gemini-2.0-flash".into()),
        provider: "gemini".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 1_048_576,
        max_output_tokens: 8_192,
        pricing: None,
    });
    models.insert("gemini-text-embedding-004".into(), ModelConfig {
        id: "gemini-text-embedding-004".into(),
        api_model_id: Some("text-embedding-004".into()),
        provider: "gemini".into(),
        capabilities: vec![Capability::TextEmbed],
        context_window: 2_048,
        max_output_tokens: 0,
        pricing: None,
    });
    // Bedrock — Claude Sonnet 3.5 v2 is the most broadly-available
    // chat model. Additional Bedrock models (Llama, Mistral, Titan)
    // can be added through the DB-driven config path.
    models.insert("bedrock-claude-3-5-sonnet".into(), ModelConfig {
        id: "bedrock-claude-3-5-sonnet".into(),
        api_model_id: Some("anthropic.claude-3-5-sonnet-20241022-v2:0".into()),
        provider: "bedrock".into(),
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

/// Register an OpenAI-compatible adapter under the given router id.
///
/// The adapter shares the OpenAI wire format and per-request URL +
/// API key come from the matching [`RouterConfig`] entry in
/// [`baseline_production_config`]. Used for OpenAI-compatible
/// aggregators (OpenRouter), unified gateways (Vercel AI Gateway),
/// and inference services (NVIDIA NIM).
async fn register_openai_compatible(adapters: &AdapterRegistry, id: &str) {
    match gateway::adapters::openai::OpenAIAdapter::with_id(id) {
        Ok(adapter) => {
            tracing::info!("Gateway: OpenAI-compatible adapter registered as '{id}'");
            adapters
                .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
                .await;
        }
        Err(e) => tracing::warn!("Gateway: '{id}' adapter failed: {e}"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms the OpenAI-compatible aggregators we register at startup
    /// (OpenRouter, Vercel AI Gateway, NVIDIA NIM) actually land in the
    /// registry under the matching router ids. The gateway engine
    /// dispatches by router id, so a mismatch here would make those
    /// providers unreachable even with a valid `RouterConfig`.
    #[tokio::test]
    async fn register_openai_compatible_adds_each_id_to_the_registry() {
        let adapters = AdapterRegistry::new();
        register_openai_compatible(&adapters, "openrouter").await;
        register_openai_compatible(&adapters, "vercel").await;
        register_openai_compatible(&adapters, "nvidia").await;

        for id in ["openrouter", "vercel", "nvidia"] {
            let got = adapters.get(id).await;
            assert!(got.is_some(), "expected adapter '{id}' to be registered");
            assert_eq!(got.unwrap().id(), id);
        }
    }

    /// The baseline production config must ship router entries for every
    /// non-OpenAI provider we register at startup; otherwise the
    /// adapter registration succeeds but `Gateway::execute` returns
    /// `NoCandidates` / `NotConfigured` because the router lookup misses.
    #[test]
    fn baseline_config_includes_routers_and_models_for_every_new_provider() {
        let cfg = baseline_production_config();
        for id in ["openrouter", "vercel", "nvidia", "gemini", "bedrock"] {
            assert!(
                cfg.routers.contains_key(id),
                "baseline routers should include '{id}', got {:?}",
                cfg.routers.keys().collect::<Vec<_>>()
            );
            let r = &cfg.routers[id];
            assert!(r.enabled, "router '{id}' should be enabled by default");
            assert!(
                r.api_key_env.is_some(),
                "router '{id}' should ship an api_key_env reference"
            );
        }

        // Each provider ships at least one representative model so the
        // routers have something to dispatch to out of the box.
        let providers_with_models: std::collections::HashSet<&str> = cfg
            .models
            .values()
            .map(|m| m.provider.as_str())
            .collect();
        for id in ["openrouter", "vercel", "nvidia", "gemini", "bedrock"] {
            assert!(
                providers_with_models.contains(id),
                "expected at least one model with provider='{id}'"
            );
        }
    }
}
