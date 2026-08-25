use std::sync::Arc;

use gateway::Gateway;
use gateway::adapters::AdapterRegistry;
use gateway::adapters::noop::NoopAdapter;
use gateway::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager};
use gateway::types::config::GatewayConfig;

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

/// Derive a model's lineage `family` (see [`gateway::types::config::ModelConfig::family`])
/// from its config id, for the hardcoded baseline models. Family is the axis
/// panel `distinct_by: family` checks care about (e.g. `gemma` vs `qwen`), so
/// two models on the same backend `provider` but different lineages read as
/// distinct experts. `None` ⇒ the id is treated as its own family (used for
/// image-gen models where lineage doesn't apply).
fn family_for_baseline(id: &str) -> Option<String> {
    let id = id.to_ascii_lowercase();
    let fam = if id.contains("gemma") {
        "gemma"
    } else if id.contains("qwen") {
        "qwen"
    } else if id.contains("llama") {
        "llama"
    } else if id.contains("claude") {
        "claude"
    } else if id.contains("gemini") {
        "gemini"
    } else if id.contains("minilm") {
        "minilm"
    } else if id.contains("nomic") {
        "nomic"
    } else if id.contains("gpt") {
        // Covers gpt-4o / gpt-4o-mini chat models; the gpt-image-1 image model
        // is handled by the image-gen branch below (family doesn't apply there).
        "gpt"
    } else {
        return None;
    };
    Some(fam.to_string())
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
///
/// Returns the gateway plus an optional on-demand model-provisioning service
/// ([`crate::api::model_provisioning::ModelProvisioning`], sensei-owned; pulls
/// via the Ollama registry). The service is `Some` only in an
/// `embedded-llama-cpp` build running the DEFAULT instance — the same conditions
/// under which the `embedded-llama` adapter is registered. When present it is
/// wired as the gateway's readiness probe (so an in-flight pull degrades the
/// embedded chat leg to `ModelNotReady` rather than a generic failure) and
/// handed back so the HTTP layer can drive on-demand pulls; it NEVER auto-pulls
/// at startup. `None` in every other build/instance ⇒ the gateway behaves
/// exactly as before.
pub async fn init_gateway(
    db_config: Option<GatewayConfig>,
) -> (Arc<Gateway>, Option<Arc<crate::api::model_provisioning::ModelProvisioning>>) {
    let adapters = AdapterRegistry::new();

    // Always register noop as fallback
    adapters.register(Arc::new(NoopAdapter)).await;

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
                id,
                model_id,
                dir
            ),
            Err(e) => tracing::warn!(
                "Gateway: FastembedAdapter from SENSEI_FASTEMBED_DIR={} failed: {}",
                dir,
                e
            ),
        }
    }
    #[cfg(feature = "embedded-ort")]
    if let Ok(dir) = std::env::var("SENSEI_ORT_DIR") {
        let model_id =
            std::env::var("SENSEI_ORT_MODEL_ID").unwrap_or_else(|_| "ort-default".to_string());
        match crate::api::gateway_embedded::register_ort(&adapters, &dir, &model_id).await {
            Ok(id) => tracing::info!(
                "Gateway: OrtAdapter registered as '{}' for model '{}' from {}",
                id,
                model_id,
                dir
            ),
            Err(e) => {
                tracing::warn!("Gateway: OrtAdapter from SENSEI_ORT_DIR={} failed: {}", dir, e)
            }
        }
    }
    // Embedded in-process llama.cpp, exposed as a single `embedded-llama`
    // router that serves many models across capabilities (#79) — the same
    // shape as the `ollama` router. Model bytes are resolved through a
    // registry: sensei-managed files first (`~/.sensei/models` index), then a
    // read-through view of the local Ollama cache (`~/.ollama/models`), so a
    // model already pulled by Ollama is reused in place and nothing has to be
    // shipped with the binary. Per-model workers load lazily on first use; a
    // model the resolver can't find degrades to the chain's next leg.
    // The in-process llama.cpp model is loaded only for the DEFAULT instance.
    // A named instance (`SENSEI_INSTANCE=<name>`, e.g. e2e) is a throwaway/test
    // daemon where a warming model (GGML/Metal pipeline compilation) would peg
    // the process and stall data-dependent screens; there, inference degrades
    // to the chain's fallback. `SENSEI_INSTANCE` is set from the `--instance`
    // CLI arg at process start (main.rs), so — unlike a plain env var — it
    // survives the daemonize re-spawn and is readable here.
    // The on-demand provisioning service (readiness probe + Ollama-registry
    // puller). Built only alongside the embedded-llama adapter; `None` otherwise.
    // Wired into the gateway via `.with_readiness` after `Gateway::new` below.
    // Construction is DEFERRED: its catalog is derived from the final `config`
    // (built below), so the embedded-llama block only records the managed dir
    // here and the service is constructed once the config is known.
    // `mut` is used only in the `embedded-llama-cpp` block; the default build
    // leaves it `None`, so silence the unused-mut lint there.
    #[allow(unused_mut)]
    let mut provisioning_service: Option<
        Arc<crate::api::model_provisioning::ModelProvisioning>,
    > = None;
    // The managed-store dir the provisioning service downloads into (== the dir
    // the embedded-llama adapter resolves from). `Some` only when the embedded
    // adapter path ran; `None` disables provisioning entirely.
    #[allow(unused_mut, unused_assignments)]
    let mut provisioning_managed_dir: Option<std::path::PathBuf> = None;
    #[cfg(feature = "embedded-llama-cpp")]
    if std::env::var("SENSEI_INSTANCE").ok().is_some_and(|s| !s.is_empty()) {
        tracing::info!(
            "Gateway: embedded-llama adapter skipped for named instance — inference degrades to fallback"
        );
    } else {
        use local_engine::registry::{ChainedResolver, ManagedResolver, OllamaResolver};
        use local_providers::adapters::EmbeddedLlamaAdapter;

        // Managed-store dir the embedded-llama adapter resolves from AND the
        // provisioning service downloads into — one path so a freshly-pulled GGUF
        // resolves + lazy-loads on the next inference request (no coldboot). Kept
        // as a single binding (DRY): the adapter's resolver and the provisioning
        // service are both derived from it below.
        let managed_dir = crate::paths::sensei_dir().join("models");

        // Resolver: sensei-managed files first, then a read-through view of the
        // local ollama cache. Built once as `Arc<dyn ModelResolver>` for the
        // embedded-llama adapter.
        let resolver: Arc<dyn local_engine::registry::ModelResolver> = Arc::new(
            ChainedResolver::new()
                .push(Arc::new(ManagedResolver::new(managed_dir.clone())))
                .push(Arc::new(OllamaResolver::new(crate::paths::home().join(".ollama/models")))),
        );

        match EmbeddedLlamaAdapter::with_shared_backend("embedded-llama", resolver) {
            Ok(adapter) => {
                tracing::info!(
                    "Gateway: embedded-llama adapter registered (resolver: managed → ollama)"
                );
                adapters.register(Arc::new(adapter)).await;
            }
            Err(e) => tracing::warn!("Gateway: embedded-llama adapter unavailable: {e}"),
        }

        // On-demand provisioning (sensei-owned; downloads via the Ollama registry
        // over HTTP + sha256 — NOT the gateway's HF puller, which stalls on Xet:
        // gateway#5). Writes into `managed_dir`, the SAME dir the embedded-llama
        // adapter resolves from, so a completed download lazy-loads on the next
        // request. Wired as the readiness probe below; it only pulls when the
        // provision HTTP handler asks — never at startup.
        //
        // The provisionable catalog is DERIVED from the final config (the models
        // the gateway runs on the `embedded-llama` router), so a pull is
        // constrained to what's configured. `config` isn't built until below, so
        // stash the managed dir here and construct the service once it is.
        provisioning_managed_dir = Some(managed_dir);
    }

    // Probe Ollama
    if probe_ollama().await {
        match gateway::adapters::ollama::OllamaAdapter::new() {
            Ok(adapter) => {
                tracing::info!("Gateway: Ollama adapter registered");
                adapters.register(Arc::new(adapter)).await;
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
            adapters.register(Arc::new(adapter)).await;
        }
        Err(e) => tracing::warn!("Gateway: Anthropic adapter failed: {}", e),
    }
    match gateway::adapters::openai::OpenAIAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: OpenAI adapter registered");
            adapters.register(Arc::new(adapter)).await;
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
            adapters.register(Arc::new(adapter)).await;
        }
        Err(e) => tracing::warn!("Gateway: Grok adapter failed: {}", e),
    }
    match gateway::adapters::gemini::GeminiAdapter::new() {
        Ok(adapter) => {
            tracing::info!("Gateway: Gemini adapter registered");
            adapters.register(Arc::new(adapter)).await;
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
            adapters.register(Arc::new(adapter)).await;
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
            // The DB IS the source of truth (#76 + #77): every capability the
            // gateway serves — including image generation — is expressible as
            // a seeded chain. This graft is now a defensive no-op: when the
            // seed carries an `image_generate` chain (post-#77), the DB
            // already covers `Capability::ImageGenerate` and the graft
            // touches nothing. Kept as belt-and-suspenders for the case
            // where the DB is somehow missing a capability at boot (partial
            // seed, mid-migration daemon).
            let baseline = baseline_production_config();
            merge_baseline_capability_gaps(&mut db, &baseline);
            // Specialized named chains (e.g. `insight-copy`) share an already-
            // covered capability, so the capability-gap merge skips them — the
            // DB seed predates them. Graft them explicitly by name.
            merge_required_named_chains(&mut db, &baseline);
            db
        }
        None => {
            tracing::info!("Gateway: no DB config — using in-code baseline");
            baseline_production_config()
        }
    };

    // Now that the final config is known, construct the on-demand provisioning
    // service with a catalog DERIVED from it: exactly the models the gateway
    // runs on the `embedded-llama` router. This constrains a pull to what's
    // configured — the handler rejects an id that isn't a local model. Only
    // constructed when the embedded-llama adapter path recorded a managed dir.
    if let Some(managed_dir) = provisioning_managed_dir.take() {
        let catalog = embedded_catalog(&config);
        tracing::info!(
            "Gateway: on-demand provisioning catalog derived from config: {:?}",
            catalog.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );
        provisioning_service = Some(Arc::new(
            crate::api::model_provisioning::ModelProvisioning::new(managed_dir, catalog),
        ));
        tracing::info!("Gateway: on-demand model-provisioning service wired (readiness probe)");
    }

    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());

    let mut gw = Gateway::new(config, adapters, cb);

    // Attach the provisioning service as the gateway's readiness probe: when a
    // chain exhausts to a candidate whose model is still being pulled, the
    // gateway returns a terminal `ModelNotReady { model, phase }` instead of the
    // generic failure — so callers/UI can see "downloading" rather than "broken".
    if let Some(service) = &provisioning_service {
        gw = gw.with_readiness(service.clone());
    }

    // Pre-populate any RouterConfig api_key fields from the Keychain.
    // Now meaningful: routers are present in the baseline config above.
    gw.refresh_router_keys(keychain_api_key).await;

    let adapter_list = gw.list_adapters().await;
    tracing::info!("Gateway initialized (unconfigured) with adapters: {:?}", adapter_list);

    (Arc::new(gw), provisioning_service)
}

/// Derive the on-demand provisioning catalog from the final [`GatewayConfig`]:
/// every model the gateway runs on the `embedded-llama` router, as
/// `(pullable_id, display_name)` pairs.
///
/// The pullable id is the id the embedded-llama adapter resolves + the
/// [`Provisioner`](crate::model_provision::Provisioner) pulls from the Ollama
/// registry — the model's api id: prefer the chain entry's `api_model_id`, else
/// the model's `api_model_id`, else the config model id. The display name is the
/// config model id (`gemma2:2b`, `all-minilm-l6-v2`). Results are deduped by
/// pullable id and returned in stable (sorted) order.
///
/// Pure over its input — unit-tested against a fixture config without a runtime,
/// network, or the llama.cpp backend.
fn embedded_catalog(config: &GatewayConfig) -> Vec<(String, String)> {
    use std::collections::BTreeMap;

    // Keyed by pullable id so duplicates collapse; BTreeMap yields stable
    // (sorted-by-id) order. Value is the display name (config model id).
    let mut catalog: BTreeMap<String, String> = BTreeMap::new();
    for chain in config.chains.values() {
        for entry in &chain.models {
            if entry.router.as_deref() != Some("embedded-llama") {
                continue;
            }
            let pullable_id = entry
                .api_model_id
                .clone()
                .or_else(|| config.models.get(&entry.model).and_then(|m| m.api_model_id.clone()))
                .unwrap_or_else(|| entry.model.clone());
            catalog.entry(pullable_id).or_insert_with(|| entry.model.clone());
        }
    }
    catalog.into_iter().collect()
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
    routers.insert(
        "openai".into(),
        RouterConfig {
            url: "https://api.openai.com".into(),
            api_key_env: Some("OPENAI_API_KEY".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "anthropic".into(),
        RouterConfig {
            url: "https://api.anthropic.com".into(),
            api_key_env: Some("ANTHROPIC_API_KEY".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "ollama".into(),
        RouterConfig {
            url: format!("http://localhost:{}", sensei_bootstrap::OLLAMA_PORT),
            api_key_env: None,
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    // In-process embedded llama.cpp, exposed as a single `embedded-llama`
    // router (#79). Only serves requests when the daemon is built with
    // `embedded-llama-cpp` and the registry can resolve the model bytes
    // (managed dir or ollama cache) — otherwise the adapter is absent / the
    // model unresolvable and the chain falls through to ollama.
    routers.insert(
        "embedded-llama".into(),
        RouterConfig {
            url: "embedded://embedded-llama".into(),
            api_key_env: None,
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    // OpenAI-compatible aggregators. The adapter implementation is the
    // same as OpenAI's; each router has its own base URL + key env var.
    routers.insert(
        "openrouter".into(),
        RouterConfig {
            url: "https://openrouter.ai/api/v1".into(),
            api_key_env: Some("OPENROUTER_API_KEY".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "vercel".into(),
        RouterConfig {
            url: "https://ai-gateway.vercel.sh/v1".into(),
            api_key_env: Some("AI_GATEWAY_API_KEY".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    routers.insert(
        "nvidia".into(),
        RouterConfig {
            url: "https://integrate.api.nvidia.com/v1".into(),
            api_key_env: Some("NVIDIA_API_KEY".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    // Google Gemini uses its own (non-OpenAI) wire format — see
    // adapters::gemini for the format and auth header details.
    routers.insert(
        "gemini".into(),
        RouterConfig {
            url: "https://generativelanguage.googleapis.com/v1beta".into(),
            api_key_env: Some("GEMINI_API_KEY".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );
    // AWS Bedrock — the SDK handles auth via the credential-provider
    // chain (AWS_ACCESS_KEY_ID / shared credentials / IAM role) and the
    // request URL is derived from the chosen region, so the
    // RouterConfig.url + api_key fields aren't used. The url is set to
    // a non-empty placeholder so existing config-validation paths that
    // require a non-empty url stay happy.
    routers.insert(
        "bedrock".into(),
        RouterConfig {
            url: "aws://bedrock".into(),
            api_key_env: Some("AWS_ACCESS_KEY_ID".into()),
            api_key: None,
            enabled: true,
            timeout_ms: Some(120_000),
            headers: HashMap::new(),
        },
    );

    let mut models: HashMap<String, ModelConfig> = HashMap::new();
    models.insert(
        "dall-e-3".into(),
        ModelConfig {
            id: "dall-e-3".into(),
            api_model_id: Some("dall-e-3".into()),
            provider: "openai".into(),
            capabilities: vec![Capability::ImageGenerate],
            context_window: 0,
            max_output_tokens: 0,
            pricing: None,
            // Image-gen models: lineage/family isn't a meaningful panel axis here.
            family: None,
        },
    );
    models.insert(
        "gpt-image-1".into(),
        ModelConfig {
            id: "gpt-image-1".into(),
            api_model_id: Some("gpt-image-1".into()),
            provider: "openai".into(),
            capabilities: vec![Capability::ImageGenerate],
            context_window: 0,
            max_output_tokens: 0,
            pricing: None,
            family: None,
        },
    );
    models.insert(
        "gpt-4o-mini".into(),
        ModelConfig {
            id: "gpt-4o-mini".into(),
            api_model_id: Some("gpt-4o-mini".into()),
            provider: "openai".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: None,
            family: family_for_baseline("gpt-4o-mini"),
        },
    );
    models.insert(
        "claude-sonnet".into(),
        ModelConfig {
            id: "claude-sonnet".into(),
            api_model_id: Some("claude-sonnet-4-5".into()),
            provider: "anthropic".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: None,
            family: family_for_baseline("claude-sonnet"),
        },
    );
    // One representative model per OpenAI-compatible aggregator so the
    // router entries above have something to dispatch to out of the box.
    // The DB-load path / setup wizard can add more once table-driven
    // configuration lands.
    models.insert(
        "openrouter-claude-sonnet-4-5".into(),
        ModelConfig {
            id: "openrouter-claude-sonnet-4-5".into(),
            api_model_id: Some("anthropic/claude-sonnet-4-5".into()),
            provider: "openrouter".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: None,
            family: family_for_baseline("openrouter-claude-sonnet-4-5"),
        },
    );
    models.insert(
        "vercel-gpt-4o".into(),
        ModelConfig {
            id: "vercel-gpt-4o".into(),
            api_model_id: Some("openai/gpt-4o".into()),
            provider: "vercel".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: None,
            family: family_for_baseline("vercel-gpt-4o"),
        },
    );
    models.insert(
        "nvidia-llama-3.1-70b-instruct".into(),
        ModelConfig {
            id: "nvidia-llama-3.1-70b-instruct".into(),
            api_model_id: Some("meta/llama-3.1-70b-instruct".into()),
            provider: "nvidia".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 128_000,
            max_output_tokens: 4_096,
            pricing: None,
            family: family_for_baseline("nvidia-llama-3.1-70b-instruct"),
        },
    );
    // Gemini — one chat model + one embedding model so both
    // capabilities the adapter supports have a dispatch target.
    models.insert(
        "gemini-2.0-flash".into(),
        ModelConfig {
            id: "gemini-2.0-flash".into(),
            api_model_id: Some("gemini-2.0-flash".into()),
            provider: "gemini".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 1_048_576,
            max_output_tokens: 8_192,
            pricing: None,
            family: family_for_baseline("gemini-2.0-flash"),
        },
    );
    models.insert(
        "gemini-text-embedding-004".into(),
        ModelConfig {
            id: "gemini-text-embedding-004".into(),
            api_model_id: Some("text-embedding-004".into()),
            provider: "gemini".into(),
            capabilities: vec![Capability::TextEmbed],
            context_window: 2_048,
            max_output_tokens: 0,
            pricing: None,
            family: family_for_baseline("gemini-text-embedding-004"),
        },
    );
    // Bedrock — Claude Sonnet 3.5 v2 is the most broadly-available
    // chat model. Additional Bedrock models (Llama, Mistral, Titan)
    // can be added through the DB-driven config path.
    models.insert(
        "bedrock-claude-3-5-sonnet".into(),
        ModelConfig {
            id: "bedrock-claude-3-5-sonnet".into(),
            api_model_id: Some("anthropic.claude-3-5-sonnet-20241022-v2:0".into()),
            provider: "bedrock".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: None,
            family: family_for_baseline("bedrock-claude-3-5-sonnet"),
        },
    );
    // Local embedding model (Ollama). 384-dim — matches sensei.nodes.embedding
    // vector(384). Used by the EmbedNodes indexing task and semantic search.
    // NOTE: the embedding space dimension is a schema contract; a different-dim
    // model (e.g. gemini-text-embedding-004 at 768) cannot be swapped in without
    // a matching DDL change to the embedding column.
    models.insert(
        "all-minilm".into(),
        ModelConfig {
            id: "all-minilm".into(),
            api_model_id: Some("all-minilm".into()),
            provider: "ollama".into(),
            capabilities: vec![Capability::TextEmbed],
            context_window: 512,
            max_output_tokens: 0,
            pricing: None,
            family: family_for_baseline("all-minilm"),
        },
    );
    // Local chat model (Ollama gemma4). The PRIMARY TextChat candidate so the
    // gateway works offline / without a cloud API key — used by infer, consensus,
    // and the governance Tier-2 consolidation merge. Cloud models remain as
    // fallback. The DB-driven config (Layer 2) can re-prioritise per role.
    models.insert(
        "gemma4".into(),
        ModelConfig {
            id: "gemma4".into(),
            api_model_id: Some("gemma4:latest".into()),
            provider: "ollama".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 8_192,
            max_output_tokens: 4_096,
            pricing: None,
            family: family_for_baseline("gemma4"),
        },
    );
    // In-process embedded chat (llama.cpp via the `embedded-llama` router).
    // PREFERRED candidate when registered; the `embedded-llama` adapter
    // resolves `gemma2:2b` from the managed dir or the ollama cache. Absent /
    // unresolvable ⇒ the chains below fall through to ollama gemma4.
    //
    // The config id and `api_model_id` are the SAME literal `gemma2:2b`, and the
    // on-demand provisioning catalog is DERIVED from this config (see
    // [`embedded_catalog`]) so the pullable id it keys by matches: the gateway's
    // readiness probe (the provisioning service) is keyed by the chain
    // candidate's config model id, so with them aligned, a chain that exhausts to
    // this leg while the model is being pulled degrades to a terminal
    // `ModelNotReady { model: "gemma2:2b" }` rather than a generic failure — and
    // serves it the instant the pull reaches `Ready`.
    models.insert(
        "gemma2:2b".into(),
        ModelConfig {
            id: "gemma2:2b".into(),
            api_model_id: Some("gemma2:2b".into()),
            provider: "embedded-llama".into(),
            capabilities: vec![Capability::TextChat],
            context_window: 8_192,
            max_output_tokens: 4_096,
            pricing: None,
            family: family_for_baseline("gemma2:2b"),
        },
    );

    let mut chains: HashMap<String, FallbackChainConfig> = HashMap::new();
    chains.insert(
        "image_generate".into(),
        FallbackChainConfig {
            id: "image_generate".into(),
            capability: Capability::ImageGenerate,
            models: vec![ChainEntry {
                model: "dall-e-3".into(),
                router: Some("openai".into()),
                api_model_id: None,
                priority: 1,
            }],
            fallback_triggers: vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout],
        },
    );
    // Shared TextChat fallback order: in-process embedded → local ollama → cloud.
    // `text_chat` serves lightweight tasks (e.g. the L2 prompt classifier);
    // `reasoning` serves heavier analysis (#70 consolidation / recommendations).
    // Both share the same candidate order — embedded preferred (no external
    // daemon), gemma4 the working local default today, cloud as last resort.
    let chat_candidates = || {
        vec![
            ChainEntry {
                model: "gemma2:2b".into(),
                router: Some("embedded-llama".into()),
                api_model_id: None,
                priority: 1,
            },
            ChainEntry {
                model: "gemma4".into(),
                router: Some("ollama".into()),
                api_model_id: None,
                priority: 2,
            },
            ChainEntry {
                model: "claude-sonnet".into(),
                router: Some("anthropic".into()),
                api_model_id: None,
                priority: 3,
            },
            ChainEntry {
                model: "gpt-4o-mini".into(),
                router: Some("openai".into()),
                api_model_id: None,
                priority: 4,
            },
        ]
    };
    let chat_triggers = || {
        vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout, FallbackTrigger::ProviderError]
    };
    chains.insert(
        "text_chat".into(),
        FallbackChainConfig {
            id: "text_chat".into(),
            capability: Capability::TextChat,
            models: chat_candidates(),
            fallback_triggers: chat_triggers(),
        },
    );
    chains.insert(
        "reasoning".into(),
        FallbackChainConfig {
            id: "reasoning".into(),
            capability: Capability::TextChat,
            models: chat_candidates(),
            fallback_triggers: chat_triggers(),
        },
    );
    // Mentor-voice insight copy (crates/senseid/src/analysis/insight_copy.rs).
    // LOCAL-ONLY by design: the two local legs only — no claude/gpt fallback —
    // so insight copy still generates offline and never spends a cloud call on
    // card text. The producer time-boxes each call and degrades to a static
    // template when both local legs are unavailable.
    chains.insert(
        "insight-copy".into(),
        FallbackChainConfig {
            id: "insight-copy".into(),
            capability: Capability::TextChat,
            models: vec![
                ChainEntry {
                    model: "gemma2:2b".into(),
                    router: Some("embedded-llama".into()),
                    api_model_id: None,
                    priority: 1,
                },
                ChainEntry {
                    model: "gemma4".into(),
                    router: Some("ollama".into()),
                    api_model_id: None,
                    priority: 2,
                },
            ],
            fallback_triggers: vec![
                FallbackTrigger::RateLimit,
                FallbackTrigger::Timeout,
                FallbackTrigger::ProviderError,
            ],
        },
    );
    // Embedding chain — intentionally 384-dim models only, to honour the
    // sensei.nodes.embedding vector(384) contract. Do NOT add a 768-dim model
    // (e.g. gemini-text-embedding-004) here without first migrating the column.
    chains.insert(
        "embed".into(),
        FallbackChainConfig {
            id: "embed".into(),
            capability: Capability::TextEmbed,
            models: vec![ChainEntry {
                model: "all-minilm".into(),
                router: Some("ollama".into()),
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
        constraints: Default::default(),
        // MOE panels/consensus are configured later (gh#19); empty ⇒ off.
        panels: Default::default(),
        consensus: Default::default(),
    }
}

/// Copy one baseline chain into `db`, pulling in any models/routers it
/// references that `db` lacks. Never overwrites existing DB entries. Shared by
/// [`merge_baseline_capability_gaps`] and [`merge_required_named_chains`].
fn graft_chain(
    db: &mut GatewayConfig,
    baseline: &GatewayConfig,
    name: &str,
    chain: &gateway::types::config::FallbackChainConfig,
) {
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
    db.chains.insert(name.to_string(), chain.clone());
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
        tracing::info!(
            "Gateway: grafting baseline chain '{}' ({:?}) — DB config has no chain for that capability",
            name,
            chain.capability
        );
        graft_chain(db, baseline, name, chain);
    }
}

/// Specialized baseline chains addressed by name at specific call sites that
/// must exist even when their capability is *already covered* by a differently-
/// named DB chain. `insight-copy` is local-only (no cloud legs) and voice-tuned
/// — a generic `TextChat` chain (`reasoning`/`classify`) is not a substitute —
/// yet it shares `Capability::TextChat`, so [`merge_baseline_capability_gaps`]
/// (which skips covered capabilities) silently drops it. This grafts such
/// chains by name after that merge, so the wire path can resolve them.
const REQUIRED_NAMED_CHAINS: &[&str] = &["insight-copy"];

fn merge_required_named_chains(db: &mut GatewayConfig, baseline: &GatewayConfig) {
    for &name in REQUIRED_NAMED_CHAINS {
        if db.chains.contains_key(name) {
            continue;
        }
        match baseline.chains.get(name) {
            Some(chain) => {
                tracing::info!("Gateway: grafting required named chain '{name}'");
                graft_chain(db, baseline, name, chain);
            }
            None => {
                tracing::warn!(
                    "Gateway: required named chain '{name}' absent from baseline config"
                );
            }
        }
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
            adapters.register(Arc::new(adapter)).await;
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
    adapters.register(Arc::new(NoopAdapter)).await;

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
            family: None,
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
        constraints: Default::default(),
        panels: Default::default(),
        consensus: Default::default(),
    };
    let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());
    Arc::new(Gateway::new(config, adapters, cb))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `embedded_catalog` collects only the models routed through
    /// `embedded-llama`, keyed by the pullable api id (entry api_model_id →
    /// model api_model_id → config id), deduped across chains, in stable order.
    /// Cloud legs and non-embedded legs are excluded.
    #[test]
    fn embedded_catalog_derives_pullable_ids_from_embedded_legs_only() {
        use gateway::types::capability::Capability;
        use gateway::types::config::*;
        use std::collections::HashMap;

        // Two models: a chat model whose pullable id == config id, and an embed
        // model whose embedded-router api id differs from the config id
        // (`all-minilm-l6-v2` → pullable `all-minilm`, as the DB seed maps it).
        let mut models: HashMap<String, ModelConfig> = HashMap::new();
        models.insert(
            "gemma2:2b".into(),
            ModelConfig {
                id: "gemma2:2b".into(),
                api_model_id: Some("gemma2:2b".into()),
                provider: "embedded-llama".into(),
                capabilities: vec![Capability::TextChat],
                context_window: 8192,
                max_output_tokens: 4096,
                pricing: None,
                family: family_for_baseline("gemma2:2b"),
            },
        );
        models.insert(
            "all-minilm-l6-v2".into(),
            ModelConfig {
                id: "all-minilm-l6-v2".into(),
                api_model_id: Some("all-minilm-l6-v2".into()),
                provider: "embedded-llama".into(),
                capabilities: vec![Capability::TextEmbed],
                context_window: 512,
                max_output_tokens: 0,
                pricing: None,
                family: family_for_baseline("all-minilm-l6-v2"),
            },
        );

        let mut chains: HashMap<String, FallbackChainConfig> = HashMap::new();
        // A chat chain: embedded gemma2:2b leg + a cloud leg (must NOT appear).
        chains.insert(
            "reasoning".into(),
            FallbackChainConfig {
                id: "reasoning".into(),
                capability: Capability::TextChat,
                models: vec![
                    ChainEntry {
                        model: "gemma2:2b".into(),
                        router: Some("embedded-llama".into()),
                        api_model_id: None,
                        priority: 1,
                    },
                    ChainEntry {
                        model: "claude-sonnet".into(),
                        router: Some("anthropic".into()),
                        api_model_id: None,
                        priority: 2,
                    },
                ],
                fallback_triggers: vec![],
            },
        );
        // A second chat chain repeats the embedded gemma2:2b leg → deduped.
        chains.insert(
            "classify".into(),
            FallbackChainConfig {
                id: "classify".into(),
                capability: Capability::TextChat,
                models: vec![
                    ChainEntry {
                        model: "gemma2:2b".into(),
                        router: Some("embedded-llama".into()),
                        api_model_id: None,
                        priority: 1,
                    },
                    ChainEntry {
                        model: "gemma3:12b".into(),
                        router: Some("ollama".into()),
                        api_model_id: None,
                        priority: 2,
                    },
                ],
                fallback_triggers: vec![],
            },
        );
        // An embed chain whose embedded leg overrides the api id at the ENTRY
        // level → pullable id comes from the entry, not the model.
        chains.insert(
            "embed".into(),
            FallbackChainConfig {
                id: "embed".into(),
                capability: Capability::TextEmbed,
                models: vec![
                    ChainEntry {
                        model: "all-minilm-l6-v2".into(),
                        router: Some("embedded-llama".into()),
                        api_model_id: Some("all-minilm".into()),
                        priority: 1,
                    },
                    ChainEntry {
                        model: "all-minilm-l6-v2".into(),
                        router: Some("ollama".into()),
                        api_model_id: None,
                        priority: 2,
                    },
                ],
                fallback_triggers: vec![],
            },
        );

        let config = GatewayConfig {
            routers: HashMap::new(),
            models,
            chains,
            constraints: Default::default(),
            panels: Default::default(),
            consensus: Default::default(),
        };

        let catalog = embedded_catalog(&config);
        // Only the two embedded models, deduped, sorted by pullable id. The
        // cloud (`claude-sonnet`) and ollama (`gemma3:12b`, ollama all-minilm)
        // legs are excluded. Display name is the config model id.
        assert_eq!(
            catalog,
            vec![
                ("all-minilm".to_string(), "all-minilm-l6-v2".to_string()),
                ("gemma2:2b".to_string(), "gemma2:2b".to_string()),
            ],
            "only embedded legs, keyed by pullable api id, deduped + sorted",
        );
    }

    /// The entry-level `api_model_id` wins over the model-level one; when neither
    /// is set the config id is used as the pullable id.
    #[test]
    fn embedded_catalog_prefers_entry_then_model_then_config_id() {
        use gateway::types::capability::Capability;
        use gateway::types::config::*;
        use std::collections::HashMap;

        let mut models: HashMap<String, ModelConfig> = HashMap::new();
        // Model-level api id set; used when the entry doesn't override it.
        models.insert(
            "model-with-api".into(),
            ModelConfig {
                id: "model-with-api".into(),
                api_model_id: Some("api-from-model".into()),
                provider: "embedded-llama".into(),
                capabilities: vec![Capability::TextChat],
                context_window: 8192,
                max_output_tokens: 4096,
                pricing: None,
                family: None,
            },
        );
        // No api id anywhere → falls back to the config id.
        models.insert(
            "bare-model".into(),
            ModelConfig {
                id: "bare-model".into(),
                api_model_id: None,
                provider: "embedded-llama".into(),
                capabilities: vec![Capability::TextChat],
                context_window: 8192,
                max_output_tokens: 4096,
                pricing: None,
                family: None,
            },
        );

        let mut chains: HashMap<String, FallbackChainConfig> = HashMap::new();
        chains.insert(
            "c".into(),
            FallbackChainConfig {
                id: "c".into(),
                capability: Capability::TextChat,
                models: vec![
                    // Entry override wins over the model-level api id.
                    ChainEntry {
                        model: "model-with-api".into(),
                        router: Some("embedded-llama".into()),
                        api_model_id: Some("api-from-entry".into()),
                        priority: 1,
                    },
                    ChainEntry {
                        model: "bare-model".into(),
                        router: Some("embedded-llama".into()),
                        api_model_id: None,
                        priority: 2,
                    },
                ],
                fallback_triggers: vec![],
            },
        );

        let config = GatewayConfig {
            routers: HashMap::new(),
            models,
            chains,
            constraints: Default::default(),
            panels: Default::default(),
            consensus: Default::default(),
        };

        assert_eq!(
            embedded_catalog(&config),
            vec![
                ("api-from-entry".to_string(), "model-with-api".to_string()),
                ("bare-model".to_string(), "bare-model".to_string()),
            ],
        );
    }

    /// The shipped in-code baseline leads its chat + insight chains with the
    /// embedded `gemma2:2b` leg, so the derived catalog carries it (with the
    /// config id as display name). Guards the "reasoning leads with embedded"
    /// invariant the DB seed mirrors.
    #[test]
    fn embedded_catalog_from_baseline_carries_gemma2_2b() {
        let catalog = embedded_catalog(&baseline_production_config());
        assert!(
            catalog.iter().any(|(id, name)| id == "gemma2:2b" && name == "gemma2:2b"),
            "baseline embedded chat model provisionable, got {catalog:?}",
        );
        // No cloud/ollama model leaks in (baseline embedded legs are gemma2:2b only).
        assert_eq!(catalog.len(), 1, "baseline has one embedded model, got {catalog:?}");
    }

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
            // OpenAI-compatible adapters register into every capability map
            // they implement (chat/embed/stt/tts/image); `chat` is enough to
            // confirm registration landed under the expected id.
            let got = adapters.chat(id).await;
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
            assert!(r.api_key_env.is_some(), "router '{id}' should ship an api_key_env reference");
        }

        // Each provider ships at least one representative model so the
        // routers have something to dispatch to out of the box.
        let providers_with_models: std::collections::HashSet<&str> =
            cfg.models.values().map(|m| m.provider.as_str()).collect();
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
                    family: family_for_baseline("gemma2:2b"),
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
            constraints: Default::default(),
            panels: Default::default(),
            consensus: Default::default(),
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
        // The capability-gap merge alone does NOT add the named insight-copy
        // chain either (TextChat is covered) — that is the bug the required-
        // named graft fixes; see the test below.
        assert!(
            !db.chains.contains_key("insight-copy"),
            "named TextChat chain NOT grafted by capability-gap merge"
        );
    }

    /// A DB config whose `TextChat` capability is already covered (e.g. a
    /// `classify` chain) must STILL receive the named `insight-copy` chain: the
    /// capability-gap merge skips it, and `merge_required_named_chains` grafts
    /// it by name, pulling in its local-only model + router. Regression guard
    /// for the "insight-copy silently dropped → 100% fallback" bug.
    #[test]
    fn merge_required_named_chains_grafts_insight_copy_even_when_textchat_covered() {
        use gateway::types::capability::Capability;
        use gateway::types::config::*;
        use std::collections::HashMap;

        let mut db = GatewayConfig {
            routers: HashMap::new(),
            models: HashMap::new(),
            chains: HashMap::from([(
                "classify".to_string(),
                FallbackChainConfig {
                    id: "classify".into(),
                    capability: Capability::TextChat,
                    models: vec![],
                    fallback_triggers: vec![],
                },
            )]),
            constraints: Default::default(),
            panels: Default::default(),
            consensus: Default::default(),
        };

        let baseline = baseline_production_config();

        // Capability-gap merge alone leaves insight-copy out (TextChat covered).
        merge_baseline_capability_gaps(&mut db, &baseline);
        assert!(
            !db.chains.contains_key("insight-copy"),
            "capability-gap merge must skip a covered-capability named chain"
        );

        // The required-named graft adds it by name, with its deps.
        merge_required_named_chains(&mut db, &baseline);
        let ic = db.chains.get("insight-copy").expect("insight-copy grafted by name");
        assert_eq!(ic.capability, Capability::TextChat);
        assert!(!ic.models.is_empty(), "insight-copy carries its local model legs");
        for entry in &ic.models {
            assert!(db.models.contains_key(&entry.model), "grafted model {} present", entry.model);
            if let Some(r) = &entry.router {
                assert!(db.routers.contains_key(r), "grafted router {r} present");
            }
        }

        // Idempotent — a second graft is a no-op.
        let n = db.chains.len();
        merge_required_named_chains(&mut db, &baseline);
        assert_eq!(db.chains.len(), n, "second graft is a no-op");
    }
}
