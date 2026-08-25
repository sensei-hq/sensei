//! Table-driven gateway configuration (#76).
//!
//! The gateway's routers, models, and fallback chains are **seed data**, not
//! code. They live in `gateway.*` tables (seeded from
//! `database/import/staging/*.jsonl` via the `import_*` procedures). This
//! module loads those tables into a [`GatewayConfig`] at daemon startup.
//!
//! The DB→config mapping is split into **pure builder functions** (no I/O,
//! fully unit-tested) and a thin async [`load_gateway_config`] orchestrator
//! that runs the SQL and feeds the rows to the builders. When the DB has no
//! chains configured, the loader returns `Ok(None)` so the caller can fall
//! back to the in-code baseline (`gateway_init::baseline_production_config`).
//!
//! ### Capability mapping
//!
//! The DB `model_capability` enum encodes *purpose* (`chat`, `reasoning`,
//! `classify`, `summarize`, `embed`, `vision`, `audio`). The gateway
//! [`Capability`] enum encodes *modality + operation*. Several DB purposes
//! collapse onto `TextChat` (all are text-in/text-out). Chains are addressed
//! by **name** (`classify`, `reasoning`, `embed`, …) — callers pin the chain
//! name; the capability is only used for the engine's tier-3 fallback.

use std::collections::HashMap;

use gateway::types::capability::Capability;
use gateway::types::config::{
    ChainEntry, FallbackChainConfig, FallbackTrigger, GatewayConfig, ModelConfig, RouterConfig,
};
use sqlx_postgres::PgPool;

/// Map a DB `model_capability` enum value to a gateway [`Capability`].
///
/// `chat`/`reasoning`/`classify`/`summarize` are all text-in/text-out and
/// collapse onto [`Capability::TextChat`]; the distinction between them is
/// carried by the *chain name*, not the capability. Returns `None` for an
/// unrecognised value so the caller can skip it rather than guess.
pub(crate) fn map_capability(db_cap: &str) -> Option<Capability> {
    match db_cap {
        "chat" | "reasoning" | "classify" | "summarize" => Some(Capability::TextChat),
        "embed" => Some(Capability::TextEmbed),
        "vision" => Some(Capability::ImageAnalyze),
        "audio" => Some(Capability::AudioTranscribe),
        // #77 — image gen is now a first-class capability; before this landed
        // it lived only in the code-defined baseline and was grafted in.
        "image" => Some(Capability::ImageGenerate),
        _ => None,
    }
}

/// Default fallback triggers for a chain. The DB schema doesn't store
/// per-chain triggers (it stores `max_fallback_attempts`), so every loaded
/// chain gets the same conservative set — the conditions under which moving
/// to the next candidate is always safe.
fn default_triggers() -> Vec<FallbackTrigger> {
    vec![FallbackTrigger::RateLimit, FallbackTrigger::Timeout, FallbackTrigger::ProviderError]
}

/// Extract `timeout_ms` from a router's `config` jsonb (parsed). Returns
/// `None` when absent or not a non-negative integer.
pub(crate) fn parse_timeout_ms(config: &serde_json::Value) -> Option<u64> {
    config.get("timeout_ms").and_then(|v| v.as_u64())
}

/// Convert a router's `default_headers` jsonb (parsed) into a string→string
/// map. Non-object inputs yield an empty map; non-string values are skipped.
pub(crate) fn parse_headers(default_headers: &serde_json::Value) -> HashMap<String, String> {
    default_headers
        .as_object()
        .map(|obj| {
            obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()
        })
        .unwrap_or_default()
}

/// Plain, sqlx-free row structs so the builders below stay pure and
/// unit-testable without a database.
#[derive(Debug, Clone)]
pub(crate) struct RouterRow {
    pub name: String,
    pub api_base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub is_active: bool,
    /// Parsed `default_headers` jsonb.
    pub default_headers: serde_json::Value,
    /// Parsed `config` jsonb.
    pub config: serde_json::Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ModelRow {
    pub full_name: String,
    /// `gateway.models.family` — coarse lineage (e.g. `"gemma"`, `"claude"`),
    /// nullable. Threaded to [`ModelConfig::family`] for MOE panel
    /// family-distinctness.
    pub family: Option<String>,
    /// DB `model_capability` enum values (as text).
    pub capabilities: Vec<String>,
    pub context_window: Option<i32>,
    pub max_output_tokens: Option<i32>,
    /// The router that serves this model by default (`models_in_router`,
    /// `is_default` first). Used only as the chain-entry router fallback.
    pub default_router: Option<String>,
    /// That default router's `router_model_id` (the provider-side model id).
    pub default_router_model_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChainRow {
    pub name: String,
    /// DB `model_capability` enum value (as text).
    pub capability: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ChainModelRow {
    pub chain_name: String,
    pub router_name: String,
    pub model_full_name: String,
    /// `models_in_router.router_model_id` for this (model, router) pair — the
    /// provider-side model id the chain entry should dispatch with.
    pub router_model_id: Option<String>,
    pub sequence_order: i32,
}

/// Clamp a DB i32 sequence/window into the gateway's narrower numeric types.
fn clamp_u8(n: i32) -> u8 {
    n.clamp(0, u8::MAX as i32) as u8
}
fn clamp_u32(n: Option<i32>) -> u32 {
    n.unwrap_or(0).max(0) as u32
}

/// Build the `routers` map from `gateway.routers` rows.
pub(crate) fn build_routers(rows: &[RouterRow]) -> HashMap<String, RouterConfig> {
    rows.iter()
        .map(|r| {
            let config = RouterConfig {
                url: r.api_base_url.clone().unwrap_or_default(),
                api_key_env: r.api_key_env_var.clone(),
                // Literal key is resolved later from the Keychain by
                // `Gateway::refresh_router_keys`; never seeded.
                api_key: None,
                enabled: r.is_active,
                timeout_ms: parse_timeout_ms(&r.config),
                headers: parse_headers(&r.default_headers),
            };
            (r.name.clone(), config)
        })
        .collect()
}

/// Build the `models` map from `gateway.models` (+ each model's default
/// router). A model whose DB capabilities don't map to any gateway
/// capability is skipped — it could never be selected anyway.
pub(crate) fn build_models(rows: &[ModelRow]) -> HashMap<String, ModelConfig> {
    let mut models = HashMap::new();
    for m in rows {
        let mut capabilities: Vec<Capability> = Vec::new();
        for c in &m.capabilities {
            if let Some(cap) = map_capability(c)
                && !capabilities.contains(&cap)
            {
                capabilities.push(cap);
            }
        }
        if capabilities.is_empty() {
            continue;
        }
        models.insert(
            m.full_name.clone(),
            ModelConfig {
                id: m.full_name.clone(),
                api_model_id: m
                    .default_router_model_id
                    .clone()
                    .or_else(|| Some(m.full_name.clone())),
                provider: m.default_router.clone().unwrap_or_default(),
                capabilities,
                context_window: clamp_u32(m.context_window),
                max_output_tokens: clamp_u32(m.max_output_tokens),
                pricing: None,
                // Lineage from `gateway.models.family` — powers MOE panel
                // family-distinctness. `None` ⇒ id is its own family.
                family: m.family.clone(),
            },
        );
    }
    models
}

/// Build the `chains` map from `gateway.fallback_chains` (+ their member
/// rows). Each chain's entries are ordered by `sequence_order`. A chain with
/// an unmappable capability or no members is skipped.
pub(crate) fn build_chains(
    chains: &[ChainRow],
    chain_models: &[ChainModelRow],
) -> HashMap<String, FallbackChainConfig> {
    let mut out = HashMap::new();
    for c in chains {
        let Some(capability) = map_capability(&c.capability) else {
            continue;
        };
        let mut entries: Vec<ChainEntry> = chain_models
            .iter()
            .filter(|cm| cm.chain_name == c.name)
            .map(|cm| ChainEntry {
                model: cm.model_full_name.clone(),
                router: Some(cm.router_name.clone()),
                api_model_id: cm.router_model_id.clone(),
                priority: clamp_u8(cm.sequence_order),
            })
            .collect();
        if entries.is_empty() {
            continue;
        }
        entries.sort_by_key(|e| e.priority);
        out.insert(
            c.name.clone(),
            FallbackChainConfig {
                id: c.name.clone(),
                capability,
                models: entries,
                fallback_triggers: default_triggers(),
            },
        );
    }
    out
}

/// Assemble the three maps into a [`GatewayConfig`].
pub(crate) fn assemble(
    routers: HashMap<String, RouterConfig>,
    models: HashMap<String, ModelConfig>,
    chains: HashMap<String, FallbackChainConfig>,
) -> GatewayConfig {
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

/// Load the gateway configuration from the `gateway.*` tables.
///
/// Returns `Ok(None)` when the DB defines **no chains** — the signal for the
/// caller to fall back to the in-code baseline. Any chains present mean the
/// DB is the source of truth and its config is returned in full.
// SQL row shapes. Named aliases keep the query result types readable (and
// satisfy clippy::type_complexity) — the columns map 1:1 to the Row structs.
type RouterTuple = (String, Option<String>, Option<String>, bool, String, String);
type ModelTuple =
    (String, Option<String>, Vec<String>, Option<i32>, Option<i32>, Option<String>, Option<String>);
type ChainModelTuple = (String, String, String, Option<String>, i32);

pub async fn load_gateway_config(pool: &PgPool) -> Result<Option<GatewayConfig>, String> {
    // Routers. jsonb columns are cast to text and parsed in Rust so this
    // doesn't depend on sqlx's optional `json` feature.
    let router_rows: Vec<RouterTuple> = sqlx_core::query_as::query_as(
        "SELECT name, api_base_url, api_key_env_var, is_active, \
                    default_headers::text, config::text \
             FROM gateway.routers",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("load_gateway_config routers: {e}"))?;

    let routers: Vec<RouterRow> = router_rows
        .into_iter()
        .map(|(name, api_base_url, api_key_env_var, is_active, headers, config)| RouterRow {
            name,
            api_base_url,
            api_key_env_var,
            is_active,
            default_headers: serde_json::from_str(&headers).unwrap_or(serde_json::Value::Null),
            config: serde_json::from_str(&config).unwrap_or(serde_json::Value::Null),
        })
        .collect();

    // Models + their default router (is_default first, then any active).
    let model_rows: Vec<ModelTuple> =
        sqlx_core::query_as::query_as(
            "SELECT m.full_name, m.family, m.capabilities::text[], m.context_window, m.max_output_tokens, \
                    dr.router_name, dr.router_model_id \
             FROM gateway.models m \
             LEFT JOIN LATERAL ( \
                 SELECT r.name AS router_name, mir.router_model_id \
                 FROM gateway.models_in_router mir \
                 JOIN gateway.routers r ON r.id = mir.router_id \
                 WHERE mir.model_id = m.id AND mir.is_active \
                 ORDER BY mir.is_default DESC \
                 LIMIT 1 \
             ) dr ON true \
             WHERE m.is_active",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("load_gateway_config models: {e}"))?;

    let models: Vec<ModelRow> = model_rows
        .into_iter()
        .map(
            |(
                full_name,
                family,
                capabilities,
                context_window,
                max_output_tokens,
                default_router,
                default_router_model_id,
            )| {
                ModelRow {
                    full_name,
                    family,
                    capabilities,
                    context_window,
                    max_output_tokens,
                    default_router,
                    default_router_model_id,
                }
            },
        )
        .collect();

    // Chains.
    let chain_rows: Vec<(String, String)> = sqlx_core::query_as::query_as(
        "SELECT name, capability::text FROM gateway.fallback_chains WHERE is_active",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("load_gateway_config chains: {e}"))?;

    let chain_rows: Vec<ChainRow> =
        chain_rows.into_iter().map(|(name, capability)| ChainRow { name, capability }).collect();

    // Chain members + the per-(model,router) router_model_id.
    let chain_model_rows: Vec<ChainModelTuple> = sqlx_core::query_as::query_as(
        "SELECT fc.name, r.name, m.full_name, mir.router_model_id, fcm.sequence_order \
             FROM gateway.fallback_chain_models fcm \
             JOIN gateway.fallback_chains fc ON fc.id = fcm.chain_id \
             JOIN gateway.routers r ON r.id = fcm.router_id \
             JOIN gateway.models m ON m.id = fcm.model_id \
             LEFT JOIN gateway.models_in_router mir \
                 ON mir.model_id = fcm.model_id AND mir.router_id = fcm.router_id \
             WHERE fcm.is_active AND fc.is_active",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("load_gateway_config chain_models: {e}"))?;

    let chain_models: Vec<ChainModelRow> = chain_model_rows
        .into_iter()
        .map(|(chain_name, router_name, model_full_name, router_model_id, sequence_order)| {
            ChainModelRow {
                chain_name,
                router_name,
                model_full_name,
                router_model_id,
                sequence_order,
            }
        })
        .collect();

    let chains = build_chains(&chain_rows, &chain_models);
    if chains.is_empty() {
        // No table-driven config — let the caller use the baseline.
        return Ok(None);
    }

    let config = assemble(build_routers(&routers), build_models(&models), chains);
    Ok(Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_capability_collapses_text_purposes_onto_text_chat() {
        for purpose in ["chat", "reasoning", "classify", "summarize"] {
            assert_eq!(map_capability(purpose), Some(Capability::TextChat), "{purpose}");
        }
        assert_eq!(map_capability("embed"), Some(Capability::TextEmbed));
        assert_eq!(map_capability("vision"), Some(Capability::ImageAnalyze));
        assert_eq!(map_capability("audio"), Some(Capability::AudioTranscribe));
        assert_eq!(map_capability("image"), Some(Capability::ImageGenerate));
        assert_eq!(map_capability("nonsense"), None);
    }

    #[test]
    fn parse_timeout_ms_reads_integer_or_none() {
        assert_eq!(parse_timeout_ms(&json!({"timeout_ms": 60000})), Some(60000));
        assert_eq!(parse_timeout_ms(&json!({})), None);
        assert_eq!(parse_timeout_ms(&json!({"timeout_ms": "x"})), None);
        assert_eq!(parse_timeout_ms(&serde_json::Value::Null), None);
    }

    #[test]
    fn parse_headers_keeps_string_values_only() {
        let h = parse_headers(&json!({"anthropic-version": "2023-06-01", "n": 5}));
        assert_eq!(h.get("anthropic-version").map(String::as_str), Some("2023-06-01"));
        assert!(!h.contains_key("n")); // non-string skipped
        assert!(parse_headers(&serde_json::Value::Null).is_empty());
        assert!(parse_headers(&json!("a string")).is_empty());
    }

    fn router_row(name: &str, active: bool) -> RouterRow {
        RouterRow {
            name: name.to_string(),
            api_base_url: Some(format!("https://{name}.example/v1")),
            api_key_env_var: Some("X_KEY".to_string()),
            is_active: active,
            default_headers: json!({"h": "v"}),
            config: json!({"timeout_ms": 60000}),
        }
    }

    #[test]
    fn build_routers_maps_columns_and_enabled_flag() {
        let routers = build_routers(&[router_row("anthropic", true), router_row("openai", false)]);
        let a = &routers["anthropic"];
        assert_eq!(a.url, "https://anthropic.example/v1");
        assert_eq!(a.api_key_env.as_deref(), Some("X_KEY"));
        assert!(a.api_key.is_none());
        assert!(a.enabled);
        assert_eq!(a.timeout_ms, Some(60000));
        assert_eq!(a.headers.get("h").map(String::as_str), Some("v"));
        assert!(!routers["openai"].enabled, "is_active=false → disabled router");
    }

    #[test]
    fn build_routers_tolerates_missing_url() {
        let row = RouterRow {
            name: "embedded".to_string(),
            api_base_url: None,
            api_key_env_var: None,
            is_active: true,
            default_headers: serde_json::Value::Null,
            config: serde_json::Value::Null,
        };
        let routers = build_routers(&[row]);
        assert_eq!(routers["embedded"].url, "");
        assert!(routers["embedded"].timeout_ms.is_none());
        assert!(routers["embedded"].headers.is_empty());
    }

    fn model_row(full_name: &str, caps: &[&str], router: &str, rmid: &str) -> ModelRow {
        ModelRow {
            full_name: full_name.to_string(),
            family: None,
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            context_window: Some(128_000),
            max_output_tokens: Some(8192),
            default_router: Some(router.to_string()),
            default_router_model_id: Some(rmid.to_string()),
        }
    }

    #[test]
    fn build_models_dedups_capabilities_and_keeps_provider() {
        let models = build_models(&[model_row(
            "gemma3:27b",
            &["chat", "reasoning", "classify", "summarize", "vision"],
            "ollama",
            "gemma3:27b",
        )]);
        let m = &models["gemma3:27b"];
        assert_eq!(m.id, "gemma3:27b");
        assert_eq!(m.provider, "ollama");
        assert_eq!(m.api_model_id.as_deref(), Some("gemma3:27b"));
        // chat/reasoning/classify/summarize collapse to one TextChat + vision.
        assert_eq!(m.capabilities.len(), 2);
        assert!(m.capabilities.contains(&Capability::TextChat));
        assert!(m.capabilities.contains(&Capability::ImageAnalyze));
    }

    #[test]
    fn build_models_threads_family_from_db_row() {
        // The DB `gateway.models.family` column flows through to
        // `ModelConfig::family` (MOE panel family-distinctness); `None` stays
        // `None` so the id is treated as its own family.
        let mut with_family = model_row("gemma3:27b", &["chat"], "ollama", "gemma3:27b");
        with_family.family = Some("gemma".to_string());
        let models = build_models(&[
            with_family,
            model_row("mystery:1b", &["chat"], "ollama", "mystery:1b"),
        ]);
        assert_eq!(models["gemma3:27b"].family.as_deref(), Some("gemma"));
        assert_eq!(models["mystery:1b"].family, None);
    }

    #[test]
    fn build_models_skips_models_with_no_mappable_capability() {
        let models = build_models(&[model_row("ghost", &["nonsense"], "ollama", "ghost")]);
        assert!(models.is_empty());
    }

    #[test]
    fn build_models_falls_back_to_full_name_for_api_model_id() {
        let row = ModelRow {
            full_name: "all-minilm-l6-v2".to_string(),
            family: None,
            capabilities: vec!["embed".to_string()],
            context_window: Some(512),
            max_output_tokens: Some(0),
            default_router: None,
            default_router_model_id: None,
        };
        let models = build_models(&[row]);
        let m = &models["all-minilm-l6-v2"];
        assert_eq!(m.api_model_id.as_deref(), Some("all-minilm-l6-v2"));
        assert_eq!(m.provider, "");
        assert_eq!(m.capabilities, vec![Capability::TextEmbed]);
    }

    #[test]
    fn build_chains_orders_by_sequence_and_carries_router_model_id() {
        let chains =
            vec![ChainRow { name: "classify".to_string(), capability: "classify".to_string() }];
        // Intentionally out of order to prove sorting.
        let members = vec![
            ChainModelRow {
                chain_name: "classify".to_string(),
                router_name: "ollama".to_string(),
                model_full_name: "gemma3:12b".to_string(),
                router_model_id: Some("gemma3:12b".to_string()),
                sequence_order: 2,
            },
            ChainModelRow {
                chain_name: "classify".to_string(),
                router_name: "llama-cpp-chat".to_string(),
                model_full_name: "gemma2:2b".to_string(),
                router_model_id: Some("llama-cpp-chat-default".to_string()),
                sequence_order: 1,
            },
        ];
        let built = build_chains(&chains, &members);
        let c = &built["classify"];
        assert_eq!(c.capability, Capability::TextChat);
        assert_eq!(c.models.len(), 2);
        // sequence 1 (embedded) first.
        assert_eq!(c.models[0].model, "gemma2:2b");
        assert_eq!(c.models[0].router.as_deref(), Some("llama-cpp-chat"));
        assert_eq!(c.models[0].api_model_id.as_deref(), Some("llama-cpp-chat-default"));
        assert_eq!(c.models[0].priority, 1);
        assert_eq!(c.models[1].model, "gemma3:12b");
        assert_eq!(c.models[1].priority, 2);
        assert_eq!(c.fallback_triggers, default_triggers());
    }

    #[test]
    fn build_chains_skips_empty_and_unmappable() {
        let chains = vec![
            ChainRow { name: "empty".to_string(), capability: "chat".to_string() },
            ChainRow { name: "bad".to_string(), capability: "nonsense".to_string() },
        ];
        // members only for a chain that isn't declared → "empty" stays empty.
        let members = vec![ChainModelRow {
            chain_name: "other".to_string(),
            router_name: "ollama".to_string(),
            model_full_name: "x".to_string(),
            router_model_id: None,
            sequence_order: 1,
        }];
        let built = build_chains(&chains, &members);
        assert!(built.is_empty(), "empty + unmappable chains are skipped");
    }

    /// Real-DB smoke test for the SQL decode path (`capabilities::text[]` →
    /// `Vec<String>`, jsonb-as-text parsing, the LATERAL default-router join).
    /// Ignored by default; run against a DB seeded with the gateway tables:
    ///   `GATEWAY_LOADER_TEST_URL=postgresql://localhost:5432/sensei \
    ///    cargo test -p senseid gateway_config_loader -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn load_gateway_config_reads_embedded_first_from_db() {
        let url = std::env::var("GATEWAY_LOADER_TEST_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/sensei".to_string());
        let pool = sqlx_postgres::PgPoolOptions::new().connect(&url).await.expect("connect");

        let cfg = super::load_gateway_config(&pool).await.expect("load ok").expect("DB has chains");

        // Embedded (single router) + cloud routers loaded.
        for r in ["embedded-llama", "nvidia", "ollama"] {
            assert!(cfg.routers.contains_key(r), "router {r} missing");
        }
        // Lightweight text chains lead with the in-process embedded adapter.
        for chain in ["classify", "summarize"] {
            let c = &cfg.chains[chain];
            assert_eq!(c.capability, Capability::TextChat, "{chain} capability");
            assert_eq!(
                c.models[0].router.as_deref(),
                Some("embedded-llama"),
                "{chain} should lead with embedded"
            );
            assert!(c.models.len() >= 4, "{chain} should have a cloud tail");
            for e in &c.models {
                assert!(cfg.models.contains_key(&e.model), "{} model {} missing", chain, e.model);
            }
        }
        // Reasoning is heavy synthesis — it leads with ollama gemma4 (a strong
        // local model; gemma4 is multimodal and can't be embedded), then
        // escalates to larger local models + cloud.
        let reasoning = &cfg.chains["reasoning"];
        assert_eq!(reasoning.capability, Capability::TextChat);
        assert_eq!(reasoning.models[0].router.as_deref(), Some("ollama"));
        assert_eq!(reasoning.models[0].model, "gemma4");
        assert!(reasoning.models.len() >= 4, "reasoning should have a cloud tail");
        for e in &reasoning.models {
            assert!(cfg.models.contains_key(&e.model), "reasoning model {} missing", e.model);
        }
        // Embed chain: embedded first, 384-dim all-minilm on both legs.
        let embed = &cfg.chains["embed"];
        assert_eq!(embed.capability, Capability::TextEmbed);
        assert_eq!(embed.models[0].router.as_deref(), Some("embedded-llama"));
        assert!(embed.models.iter().all(|e| e.model == "all-minilm-l6-v2"));
        // Embedded chat model carries TextChat.
        assert!(cfg.models["gemma2:2b"].capabilities.contains(&Capability::TextChat));
    }

    #[test]
    fn assemble_produces_an_embedded_first_chat_chain_end_to_end() {
        // A realistic slice: embedded → ollama → cloud, router-gated.
        let routers = build_routers(&[
            RouterRow {
                name: "llama-cpp-chat".to_string(),
                api_base_url: Some("embedded://llama-cpp-chat".to_string()),
                api_key_env_var: None,
                is_active: true,
                default_headers: serde_json::Value::Null,
                config: serde_json::Value::Null,
            },
            router_row("ollama", true),
            router_row("anthropic", true),
        ]);
        let models = build_models(&[
            model_row(
                "gemma2:2b",
                &["chat", "classify"],
                "llama-cpp-chat",
                "llama-cpp-chat-default",
            ),
            model_row("gemma3:12b", &["chat", "classify"], "ollama", "gemma3:12b"),
            model_row("claude-haiku-4-5", &["chat"], "anthropic", "claude-haiku-4-5-20251001"),
        ]);
        let chains = build_chains(
            &[ChainRow { name: "classify".to_string(), capability: "classify".to_string() }],
            &[
                ChainModelRow {
                    chain_name: "classify".to_string(),
                    router_name: "llama-cpp-chat".to_string(),
                    model_full_name: "gemma2:2b".to_string(),
                    router_model_id: Some("llama-cpp-chat-default".to_string()),
                    sequence_order: 1,
                },
                ChainModelRow {
                    chain_name: "classify".to_string(),
                    router_name: "ollama".to_string(),
                    model_full_name: "gemma3:12b".to_string(),
                    router_model_id: Some("gemma3:12b".to_string()),
                    sequence_order: 2,
                },
                ChainModelRow {
                    chain_name: "classify".to_string(),
                    router_name: "anthropic".to_string(),
                    model_full_name: "claude-haiku-4-5".to_string(),
                    router_model_id: Some("claude-haiku-4-5-20251001".to_string()),
                    sequence_order: 3,
                },
            ],
        );
        let cfg = assemble(routers, models, chains);
        let chain = &cfg.chains["classify"];
        let order: Vec<&str> = chain.models.iter().map(|e| e.router.as_deref().unwrap()).collect();
        assert_eq!(order, vec!["llama-cpp-chat", "ollama", "anthropic"]);
        // Every chain entry's model resolves in the models map (engine precondition).
        for entry in &chain.models {
            assert!(cfg.models.contains_key(&entry.model), "model {} missing", entry.model);
        }
    }
}
