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

/// Initialize the gateway with detected adapters and a config.
///
/// `db_config` is the table-driven config loaded from the `gateway.*` tables
/// (see [`crate::api::gateway_config_loader`]). When `None` — the DB has no
/// chains configured, or the load failed — the in-code
/// [`baseline_production_config`] is used as a last-resort fallback so the
/// daemon always starts with *some* working routing.
///
/// Adapters (providers) are auto-detected at startup:
/// - Ollama: probed at localhost:11434
/// - Anthropic: ANTHROPIC_API_KEY env var
/// - OpenAI: OPENAI_API_KEY env var
/// - Grok: XAI_API_KEY env var
/// - Noop: always registered as graceful degradation fallback
pub async fn init_gateway(db_config: Option<GatewayConfig>) -> Arc<Gateway> {
    let adapters = AdapterRegistry::new();

    // Always register noop as fallback
    adapters
        .register(Arc::new(NoopAdapter) as Arc<dyn InferenceAdapter>)
        .await;

    // Optional in-process embedding adapters. Each is only active when
    // the daemon binary is built with the matching `embedded-*` cargo
    // feature AND the operator points the corresponding env var at a
    // local model directory. The daemon serves embeddings in-process
    // without going through Ollama's HTTP layer — see
    // docs/backlog.md (Future scope — gateway-embedded) for rationale.
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
    #[cfg(feature = "embedded-ort")]
    if let Ok(dir) = std::env::var("SENSEI_ORT_DIR") {
        let model_id = std::env::var("SENSEI_ORT_MODEL_ID")
            .unwrap_or_else(|_| "ort-default".to_string());
        match crate::api::gateway_embedded::register_ort(&adapters, &dir, &model_id).await {
            Ok(id) => tracing::info!(
                "Gateway: OrtAdapter registered as '{}' for model '{}' from {}",
                id, model_id, dir
            ),
            Err(e) => tracing::warn!(
                "Gateway: OrtAdapter from SENSEI_ORT_DIR={} failed: {}",
                dir, e
            ),
        }
    }
    // LlamaCpp has two distinct modes (embedding + chat). The daemon
    // can register either or both, depending on which GGUF files the
    // operator supplies. Each call shares the same process-singleton
    // LlamaBackend behind the scenes, so loading two adapters is safe.
    // Resolve the embed GGUF: explicit `SENSEI_LLAMA_CPP_EMBED_GGUF` override,
    // else the stable managed path `<data-dir>/models/embed.gguf` (mirrors the
    // chat resolution below). Makes embedded 384-dim embeddings the default
    // with NO env/plist wiring — drop a 384-dim GGUF (e.g. all-MiniLM-L6-v2)
    // there and a feature-built daemon serves it in-process (the embed chain
    // lists it first). Absent ⇒ the chain falls through to ollama all-minilm.
    #[cfg(feature = "embedded-llama-cpp")]
    if let Some(path) = std::env::var("SENSEI_LLAMA_CPP_EMBED_GGUF").ok().or_else(|| {
        let p = crate::paths::sensei_dir().join("models/embed.gguf");
        p.exists().then(|| p.to_string_lossy().into_owned())
    }) {
        let model_id = std::env::var("SENSEI_LLAMA_CPP_EMBED_MODEL_ID")
            .unwrap_or_else(|_| "llama-cpp-embed-default".to_string());
        match crate::api::gateway_embedded::register_llama_cpp_embed(
            &adapters, &path, &model_id,
        )
        .await
        {
            Ok(id) => tracing::info!(
                "Gateway: LlamaCppAdapter (embed) registered as '{}' for model '{}' from {}",
                id, model_id, path
            ),
            Err(e) => tracing::warn!(
                "Gateway: LlamaCppAdapter (embed) from {} failed: {}",
                path, e
            ),
        }
    }
    // Resolve the chat GGUF: explicit `SENSEI_LLAMA_CPP_CHAT_GGUF` override, else
    // the stable managed path `<data-dir>/models/chat.gguf`. The managed path
    // makes embedded chat the default with NO env/plist wiring — drop a
    // single-modality chat GGUF there and a feature-built daemon serves it
    // in-process (the text_chat/reasoning chains list it first). Absent ⇒ the
    // chain falls through to ollama/cloud.
    #[cfg(feature = "embedded-llama-cpp")]
    if let Some(path) = std::env::var("SENSEI_LLAMA_CPP_CHAT_GGUF").ok().or_else(|| {
        let p = crate::paths::sensei_dir().join("models/chat.gguf");
        p.exists().then(|| p.to_string_lossy().into_owned())
    }) {
        let model_id = std::env::var("SENSEI_LLAMA_CPP_CHAT_MODEL_ID")
            .unwrap_or_else(|_| "llama-cpp-chat-default".to_string());
        match crate::api::gateway_embedded::register_llama_cpp_chat(
            &adapters, &path, &model_id,
        )
        .await
        {
            Ok(id) => tracing::info!(
                "Gateway: LlamaCppAdapter (chat) registered as '{}' for model '{}' from {}",
                id, model_id, path
            ),
            Err(e) => tracing::warn!(
                "Gateway: LlamaCppAdapter (chat) from {} failed: {}",
                path, e
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

    // Config source: the table-driven config loaded from the `gateway.*`
    // tables (the source of truth, #76). The in-code baseline is only the
    // fallback for a fresh/unseeded DB or a load failure — without some
    // config, refresh_router_keys would have no routers to populate even
    // after the user pastes a key.
    let config = match db_config {
        Some(mut db) => {
            // The DB `model_capability` enum can't yet express image
            // generation, so it has no image_generate chain. Graft the
            // baseline's chains for any capability the DB doesn't cover so
            // those features (e.g. image generation) don't regress when the
            // DB becomes the source of truth. Tracked by #77 (seed an image
            // chain once the enum gains an `image` value), after which this
            // is a no-op.
            merge_baseline_capability_gaps(&mut db, &baseline_production_config());
            db
        }
        None => {
            tracing::info!("Gateway: no DB config — using in-code baseline");
            baseline_production_config()
        }
    };

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
    // In-process embedded chat (llama.cpp GGUF). Only serves requests when the
    // daemon is built with `embedded-llama-cpp` AND SENSEI_LLAMA_CPP_CHAT_GGUF is
    // set (see init_gateway) — otherwise the adapter is absent and the chain
    // falls through to ollama. Listed so the chain prefers local in-process when
    // available (the preferred path; removes the external Ollama dependency).
    routers.insert("llama-cpp-chat".into(), RouterConfig {
        url: "embedded://llama-cpp-chat".into(),
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
    // Local embedding model (Ollama). 384-dim — matches sensei.nodes.embedding
    // vector(384). Used by the EmbedNodes indexing task and semantic search.
    // NOTE: the embedding space dimension is a schema contract; a different-dim
    // model (e.g. gemini-text-embedding-004 at 768) cannot be swapped in without
    // a matching DDL change to the embedding column.
    models.insert("all-minilm".into(), ModelConfig {
        id: "all-minilm".into(),
        api_model_id: Some("all-minilm".into()),
        provider: "ollama".into(),
        capabilities: vec![Capability::TextEmbed],
        context_window: 512,
        max_output_tokens: 0,
        pricing: None,
    });
    // Local chat model (Ollama gemma4). The PRIMARY TextChat candidate so the
    // gateway works offline / without a cloud API key — used by infer, consensus,
    // and the governance Tier-2 consolidation merge. Cloud models remain as
    // fallback. The DB-driven config (Layer 2) can re-prioritise per role.
    models.insert("gemma4".into(), ModelConfig {
        id: "gemma4".into(),
        api_model_id: Some("gemma4:latest".into()),
        provider: "ollama".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 8_192,
        max_output_tokens: 4_096,
        pricing: None,
    });
    // In-process embedded chat (llama.cpp). PREFERRED candidate when registered
    // (no external Ollama dependency); absent unless the daemon is built with
    // `embedded-llama-cpp` + SENSEI_LLAMA_CPP_CHAT_GGUF, in which case the chains
    // below use it first and fall through to ollama gemma4 otherwise.
    models.insert("gemma-embedded".into(), ModelConfig {
        id: "gemma-embedded".into(),
        api_model_id: Some("llama-cpp-chat-default".into()),
        provider: "llama-cpp-chat".into(),
        capabilities: vec![Capability::TextChat],
        context_window: 8_192,
        max_output_tokens: 4_096,
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
    // Shared TextChat fallback order: in-process embedded → local ollama → cloud.
    // `text_chat` serves lightweight tasks (e.g. the L2 prompt classifier);
    // `reasoning` serves heavier analysis (#70 consolidation / recommendations).
    // Both share the same candidate order — embedded preferred (no external
    // daemon), gemma4 the working local default today, cloud as last resort.
    let chat_candidates = || {
        vec![
            ChainEntry { model: "gemma-embedded".into(), router: Some("llama-cpp-chat".into()), api_model_id: None, priority: 1 },
            ChainEntry { model: "gemma4".into(),         router: Some("ollama".into()),         api_model_id: None, priority: 2 },
            ChainEntry { model: "claude-sonnet".into(),  router: Some("anthropic".into()),      api_model_id: None, priority: 3 },
            ChainEntry { model: "gpt-4o-mini".into(),    router: Some("openai".into()),         api_model_id: None, priority: 4 },
        ]
    };
    let chat_triggers = || vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout, FallbackTrigger::ProviderError];
    chains.insert("text_chat".into(), FallbackChainConfig {
        id: "text_chat".into(),
        capability: Capability::TextChat,
        models: chat_candidates(),
        fallback_triggers: chat_triggers(),
    });
    chains.insert("reasoning".into(), FallbackChainConfig {
        id: "reasoning".into(),
        capability: Capability::TextChat,
        models: chat_candidates(),
        fallback_triggers: chat_triggers(),
    });
    // Embedding chain — intentionally 384-dim models only, to honour the
    // sensei.nodes.embedding vector(384) contract. Do NOT add a 768-dim model
    // (e.g. gemini-text-embedding-004) here without first migrating the column.
    chains.insert("embed".into(), FallbackChainConfig {
        id: "embed".into(),
        capability: Capability::TextEmbed,
        models: vec![ChainEntry {
            model: "all-minilm".into(),
            router: Some("ollama".into()),
            api_model_id: None,
            priority: 1,
        }],
        fallback_triggers: vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout, FallbackTrigger::ProviderError],
    });

    GatewayConfig { routers, models, chains }
}

/// Graft baseline chains for any [`Capability`] the DB config doesn't cover.
///
/// The DB is the source of truth for every capability it can express. Some
/// capabilities (image generation) have no `model_capability` enum value yet,
/// so they can't be seeded — without this, switching to table-driven config
/// would silently drop them. For each baseline chain whose capability is
/// absent from `db`, this copies the chain plus any models/routers it
/// references that `db` lacks. It never overwrites existing DB entries.
fn merge_baseline_capability_gaps(db: &mut GatewayConfig, baseline: &GatewayConfig) {
    use std::collections::HashSet;

    let covered: HashSet<_> = db.chains.values().map(|c| c.capability.clone()).collect();
    for (name, chain) in &baseline.chains {
        if covered.contains(&chain.capability) || db.chains.contains_key(name) {
            continue;
        }
        for entry in &chain.models {
            if !db.models.contains_key(&entry.model)
                && let Some(m) = baseline.models.get(&entry.model)
            {
                db.models.insert(entry.model.clone(), m.clone());
            }
            if let Some(router) = &entry.router
                && !db.routers.contains_key(router)
                && let Some(r) = baseline.routers.get(router)
            {
                db.routers.insert(router.clone(), r.clone());
            }
        }
        tracing::info!(
            "Gateway: grafting baseline chain '{}' ({:?}) — DB config has no chain for that capability",
            name, chain.capability
        );
        db.chains.insert(name.clone(), chain.clone());
    }
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

    /// A DB config that only covers text capabilities must still get the
    /// baseline image_generate chain grafted (the enum can't express it yet),
    /// pulling in the model + router that chain needs — without clobbering the
    /// DB's own text chains.
    #[test]
    fn merge_baseline_capability_gaps_grafts_image_chain_only() {
        use gateway::types::capability::Capability;
        use gateway::types::config::*;
        use std::collections::HashMap;

        let mut db = GatewayConfig {
            routers: HashMap::from([(
                "ollama".to_string(),
                RouterConfig {
                    url: "http://localhost:11434".into(),
                    api_key_env: None,
                    api_key: None,
                    enabled: true,
                    timeout_ms: None,
                    headers: HashMap::new(),
                },
            )]),
            models: HashMap::from([(
                "gemma2:2b".to_string(),
                ModelConfig {
                    id: "gemma2:2b".into(),
                    api_model_id: None,
                    provider: "ollama".into(),
                    capabilities: vec![Capability::TextChat],
                    context_window: 8192,
                    max_output_tokens: 4096,
                    pricing: None,
                },
            )]),
            chains: HashMap::from([(
                "classify".to_string(),
                FallbackChainConfig {
                    id: "classify".into(),
                    capability: Capability::TextChat,
                    models: vec![ChainEntry {
                        model: "gemma2:2b".into(),
                        router: Some("ollama".into()),
                        api_model_id: None,
                        priority: 1,
                    }],
                    fallback_triggers: vec![],
                },
            )]),
        };

        let baseline = baseline_production_config();
        merge_baseline_capability_gaps(&mut db, &baseline);

        // image_generate (no DB enum value) is grafted, with its model+router.
        let img = db.chains.get("image_generate").expect("image chain grafted");
        assert_eq!(img.capability, Capability::ImageGenerate);
        let m = &img.models[0].model;
        assert!(db.models.contains_key(m), "grafted image model {m} present");
        if let Some(r) = &img.models[0].router {
            assert!(db.routers.contains_key(r), "grafted image router {r} present");
        }

        // The DB's own text chain is untouched and no extra text chain is added.
        assert_eq!(db.chains["classify"].models.len(), 1);
        assert!(!db.chains.contains_key("text_chat"), "text-capability chains NOT grafted");
        assert!(!db.chains.contains_key("reasoning"), "text-capability chains NOT grafted");
    }
}
