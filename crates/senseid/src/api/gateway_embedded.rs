//! Optional registration of in-process inference adapters from the
//! `gateway-embedded` crate.
//!
//! Each adapter sits behind its own cargo feature so the default
//! daemon build doesn't pay the native build cost (ORT runtime for
//! `fastembed`, C++ toolchain for `llama-cpp`). Operators opt in at
//! `cargo build` time with:
//!
//! ```text
//! cargo build -p senseid --features embedded-fastembed
//! cargo build -p senseid --features embedded-llama-cpp
//! ```
//!
//! Registration happens at gateway construction (or later, against an
//! existing [`AdapterRegistry`]). The helpers here are stateless — the
//! caller decides which adapters to wire in and against which model
//! files. A future setup-wizard stage can use these helpers once the UI
//! for picking a local embedding model lands.

#[cfg(feature = "embedded-fastembed")]
pub use fastembed_init::register_fastembed;

#[cfg(feature = "embedded-fastembed")]
mod fastembed_init {
    use gateway::adapters::{AdapterRegistry, InferenceAdapter};
    use gateway_embedded::adapters::{FastembedAdapter, FastembedConfig};
    use gateway_embedded::registry::{ModelEntry, ModelFormat, ModelSource};
    use std::path::Path;
    use std::sync::Arc;

    /// Build a [`FastembedAdapter`] against an on-disk ONNX export
    /// directory and register it with the given gateway adapter
    /// registry. Returns the registered adapter id so the caller can
    /// wire it into [`gateway::types::config::GatewayConfig`] as a
    /// router/model provider.
    ///
    /// `dir` must contain `model.onnx` plus the four standard tokenizer
    /// files (`tokenizer.json`, `config.json`, `special_tokens_map.json`,
    /// `tokenizer_config.json`). That layout is produced by both
    /// `optimum-cli export onnx` and Qdrant's pre-optimised exports.
    pub async fn register_fastembed(
        registry: &AdapterRegistry,
        dir: impl AsRef<Path>,
        model_id: impl Into<String>,
    ) -> Result<String, String> {
        let model_id = model_id.into();
        let onnx_path = dir.as_ref().join("model.onnx");
        let entry = ModelEntry {
            id: model_id.clone(),
            name: model_id.clone(),
            format: ModelFormat::Onnx,
            source: ModelSource::External { path: onnx_path },
            sha256: None,
            size_bytes: None,
        };
        let cfg = FastembedConfig::bert(&model_id);
        let adapter = FastembedAdapter::load(&entry, cfg)
            .map_err(|e| format!("FastembedAdapter::load: {e}"))?;
        let id = adapter.id().to_string();
        registry
            .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
            .await;
        Ok(id)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use gateway::Gateway;
        use gateway::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager};
        use gateway::types::capability::Capability;
        use gateway::types::config::{GatewayConfig, ModelConfig, RouterConfig};
        use gateway::types::request::{InferenceRequest, Payload};
        use std::collections::HashMap;

        /// End-to-end smoke test: an embedded fastembed adapter
        /// registered via [`register_fastembed`] is reachable through
        /// `Gateway::execute` — the same call path the daemon uses for
        /// every inference request.
        ///
        /// Ignored by default; requires a fastembed-compatible ONNX
        /// directory on disk. Run with:
        ///
        /// ```text
        /// SENSEI_FASTEMBED_DIR=/path/to/all-MiniLM-L6-v2-qdrant \
        ///   cargo test -p senseid --features embedded-fastembed \
        ///     gateway_embedded -- --ignored
        /// ```
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        #[ignore = "requires SENSEI_FASTEMBED_DIR env var pointing at a fastembed-compatible ONNX directory"]
        async fn fastembed_through_gateway_returns_embeddings_for_a_real_model() {
            let dir = std::env::var("SENSEI_FASTEMBED_DIR").expect(
                "SENSEI_FASTEMBED_DIR must point at an ONNX embedding model directory",
            );

            let registry = AdapterRegistry::new();
            let adapter_id = register_fastembed(&registry, &dir, "test-fastembed-minilm")
                .await
                .expect("register fastembed adapter");
            assert_eq!(
                adapter_id, "fastembed",
                "FastembedAdapter::id() should default to \"fastembed\""
            );

            // The gateway engine looks an adapter up via
            // `model.provider` against the registry, so the model's
            // provider field must match the adapter id.
            let mut routers = HashMap::new();
            routers.insert(
                "fastembed".into(),
                RouterConfig {
                    url: "embedded://fastembed".into(),
                    api_key_env: None,
                    api_key: None,
                    enabled: true,
                    timeout_ms: None,
                    headers: HashMap::new(),
                },
            );

            let mut models = HashMap::new();
            models.insert(
                "test-fastembed-minilm".into(),
                ModelConfig {
                    id: "test-fastembed-minilm".into(),
                    api_model_id: None,
                    provider: "fastembed".into(),
                    capabilities: vec![Capability::TextEmbed],
                    context_window: 0,
                    max_output_tokens: 0,
                    pricing: None,
                },
            );

            let config = GatewayConfig {
                routers,
                models,
                chains: HashMap::new(),
            };
            let cb = CircuitBreakerManager::new(CircuitBreakerConfig::default());
            let gw = Gateway::new(config, registry, cb);

            // Direct-tier selection needs the router specified alongside
            // the model — the gateway's "tier 1: direct" path looks the
            // adapter up by router id (not provider id) and we haven't
            // configured a chain. With both set, the engine finds
            // RouterConfig["fastembed"] and ModelConfig["test-..."] and
            // dispatches to the registered adapter whose id() is
            // "fastembed".
            let request = InferenceRequest {
                capability: Capability::TextEmbed,
                model: Some("test-fastembed-minilm".into()),
                router: Some("fastembed".into()),
                chain: None,
                payload: Payload::Embed {
                    texts: vec![
                        "hello from sensei".into(),
                        "embedded adapter via gateway execute".into(),
                    ],
                },
                budget: None,
            };

            let response = gw.execute(&request).await.expect("gateway.execute");

            assert!(response.success, "expected successful response");
            let embeddings = response
                .embeddings
                .expect("embed response should include embeddings");
            assert_eq!(embeddings.len(), 2);
            for (i, v) in embeddings.iter().enumerate() {
                assert!(!v.is_empty(), "embedding[{i}] should be non-empty");
            }
            assert_eq!(
                response.model.as_deref(),
                Some("test-fastembed-minilm"),
                "response.model should echo the resolved model id",
            );
        }
    }
}
