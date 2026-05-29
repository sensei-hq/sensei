//! Optional registration of in-process inference adapters from the
//! `gateway-embedded` crate.
//!
//! Each adapter sits behind its own cargo feature so the default
//! daemon build doesn't pay the native build cost (ORT runtime for
//! `fastembed` / `ort`, C++ toolchain for `llama-cpp`). Operators opt
//! in at `cargo build` time with:
//!
//! ```text
//! cargo build -p senseid --features embedded-fastembed
//! cargo build -p senseid --features embedded-llama-cpp
//! cargo build -p senseid --features embedded-ort
//! ```
//!
//! Registration happens at gateway construction (or later, against an
//! existing [`AdapterRegistry`]). The helpers here are stateless — the
//! caller decides which adapters to wire in and against which model
//! files. A future setup-wizard stage can use these helpers once the UI
//! for picking a local embedding model lands.

#[cfg(feature = "embedded-fastembed")]
pub use fastembed_init::register_fastembed;

#[cfg(feature = "embedded-ort")]
pub use ort_init::register_ort;

#[cfg(feature = "embedded-llama-cpp")]
pub use llama_cpp_init::{register_llama_cpp_chat, register_llama_cpp_embed};

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

#[cfg(feature = "embedded-ort")]
mod ort_init {
    use gateway::adapters::{AdapterRegistry, InferenceAdapter};
    use gateway_embedded::adapters::{OrtAdapter, OrtConfig};
    use gateway_embedded::registry::{ModelEntry, ModelFormat, ModelSource};
    use std::path::Path;
    use std::sync::Arc;

    /// Build an [`OrtAdapter`] against an on-disk ONNX export directory
    /// and register it with the given gateway adapter registry.
    /// Returns the registered adapter id so the caller can wire it
    /// into [`gateway::types::config::GatewayConfig`] as a router/model
    /// provider.
    ///
    /// Same on-disk contract as the fastembed wire-up: `dir` must
    /// contain `model.onnx` plus `tokenizer.json` (and the rest of the
    /// standard tokenizer files when the model uses them). The ORT
    /// adapter is the lower-level alternative — caller gets explicit
    /// control over pooling strategy via [`OrtConfig`] and the model
    /// surface isn't constrained to fastembed's known repos.
    pub async fn register_ort(
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
        let cfg = OrtConfig::bert(&model_id);
        let adapter = OrtAdapter::load(&entry, cfg)
            .map_err(|e| format!("OrtAdapter::load: {e}"))?;
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

        /// End-to-end smoke test: an embedded ORT adapter registered
        /// via [`register_ort`] is reachable through `Gateway::execute`
        /// — the same call path the daemon uses for every inference
        /// request. Mirrors the fastembed test exactly so the two
        /// adapters land at parity from the daemon's perspective.
        ///
        /// Ignored by default; requires a fastembed-compatible ONNX
        /// directory on disk. Run with:
        ///
        /// ```text
        /// SENSEI_ORT_DIR=/path/to/all-MiniLM-L6-v2-qdrant \
        ///   cargo test -p senseid --features embedded-ort \
        ///     gateway_embedded -- --ignored
        /// ```
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        #[ignore = "requires SENSEI_ORT_DIR env var pointing at a fastembed-compatible ONNX directory"]
        async fn ort_through_gateway_returns_embeddings_for_a_real_model() {
            let dir = std::env::var("SENSEI_ORT_DIR").expect(
                "SENSEI_ORT_DIR must point at an ONNX embedding model directory",
            );

            let registry = AdapterRegistry::new();
            let adapter_id = register_ort(&registry, &dir, "test-ort-minilm")
                .await
                .expect("register ort adapter");
            assert_eq!(
                adapter_id, "ort",
                "OrtAdapter::id() should default to \"ort\""
            );

            // Same router/model wiring as the fastembed smoke test —
            // model.provider matches the adapter id so the engine's
            // direct-tier selection finds it.
            let mut routers = HashMap::new();
            routers.insert(
                "ort".into(),
                RouterConfig {
                    url: "embedded://ort".into(),
                    api_key_env: None,
                    api_key: None,
                    enabled: true,
                    timeout_ms: None,
                    headers: HashMap::new(),
                },
            );

            let mut models = HashMap::new();
            models.insert(
                "test-ort-minilm".into(),
                ModelConfig {
                    id: "test-ort-minilm".into(),
                    api_model_id: None,
                    provider: "ort".into(),
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

            let request = InferenceRequest {
                capability: Capability::TextEmbed,
                model: Some("test-ort-minilm".into()),
                router: Some("ort".into()),
                chain: None,
                payload: Payload::Embed {
                    texts: vec![
                        "hello from sensei".into(),
                        "embedded ort adapter via gateway execute".into(),
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
                Some("test-ort-minilm"),
                "response.model should echo the resolved model id",
            );
        }
    }
}

#[cfg(feature = "embedded-llama-cpp")]
mod llama_cpp_init {
    use gateway::adapters::{AdapterRegistry, InferenceAdapter};
    use gateway_embedded::adapters::{
        LlamaCppAdapter, LlamaCppConfig, shared_backend,
    };
    use gateway_embedded::registry::{ModelEntry, ModelFormat, ModelSource};
    use std::path::Path;
    use std::sync::Arc;

    /// Common builder used by both the embed and chat helpers below.
    /// Returns the registered adapter id so the caller can wire it into
    /// [`gateway::types::config::GatewayConfig`] as a router/model
    /// provider.
    async fn register_with_config(
        registry: &AdapterRegistry,
        gguf_path: impl AsRef<Path>,
        model_id: impl Into<String>,
        config: LlamaCppConfig,
    ) -> Result<String, String> {
        let model_id = model_id.into();
        let entry = ModelEntry {
            id: model_id.clone(),
            name: model_id,
            format: ModelFormat::Gguf,
            source: ModelSource::External {
                path: gguf_path.as_ref().to_path_buf(),
            },
            sha256: None,
            size_bytes: None,
        };
        let backend =
            shared_backend().map_err(|e| format!("LlamaBackend init: {e}"))?;
        let adapter = LlamaCppAdapter::load(backend, &entry, config)
            .map_err(|e| format!("LlamaCppAdapter::load: {e}"))?;
        let id = adapter.id().to_string();
        registry
            .register(Arc::new(adapter) as Arc<dyn InferenceAdapter>)
            .await;
        Ok(id)
    }

    /// Register a LlamaCpp adapter configured for **embedding**
    /// (mean-pooled BERT-class GGUF: all-minilm, bge, nomic-embed).
    /// `gguf_path` points at a single GGUF file — no tokenizer
    /// sidecar, the tokenizer is baked into the GGUF itself.
    pub async fn register_llama_cpp_embed(
        registry: &AdapterRegistry,
        gguf_path: impl AsRef<Path>,
        model_id: impl Into<String>,
    ) -> Result<String, String> {
        let model_id = model_id.into();
        let cfg = LlamaCppConfig::embed(&model_id);
        register_with_config(registry, gguf_path, model_id, cfg).await
    }

    /// Register a LlamaCpp adapter configured for **chat / completion**
    /// (any generative GGUF: llama-3.x, qwen, mistral, …). Same model
    /// file shape as the embed variant; the configuration choice
    /// determines how the engine runs the forward pass.
    pub async fn register_llama_cpp_chat(
        registry: &AdapterRegistry,
        gguf_path: impl AsRef<Path>,
        model_id: impl Into<String>,
    ) -> Result<String, String> {
        let model_id = model_id.into();
        let cfg = LlamaCppConfig::chat(&model_id);
        register_with_config(registry, gguf_path, model_id, cfg).await
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use gateway::Gateway;
        use gateway::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager};
        use gateway::types::capability::Capability;
        use gateway::types::config::{GatewayConfig, ModelConfig, RouterConfig};
        use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};
        use std::collections::HashMap;

        fn gateway_with_one(
            registry: AdapterRegistry,
            model_id: &str,
            capability: Capability,
        ) -> Gateway {
            let mut routers = HashMap::new();
            routers.insert(
                "llama-cpp".into(),
                RouterConfig {
                    url: "embedded://llama-cpp".into(),
                    api_key_env: None,
                    api_key: None,
                    enabled: true,
                    timeout_ms: None,
                    headers: HashMap::new(),
                },
            );
            let mut models = HashMap::new();
            models.insert(
                model_id.into(),
                ModelConfig {
                    id: model_id.into(),
                    api_model_id: None,
                    provider: "llama-cpp".into(),
                    capabilities: vec![capability],
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
            Gateway::new(config, registry, cb)
        }

        /// End-to-end smoke test mirroring the fastembed / ort
        /// counterparts: embed path through Gateway::execute.
        ///
        /// ```text
        /// SENSEI_LLAMA_CPP_EMBED_GGUF=$HOME/.ollama/models/blobs/sha256-... \
        ///   cargo test -p senseid --features embedded-llama-cpp \
        ///     gateway_embedded -- --ignored
        /// ```
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        #[ignore = "requires SENSEI_LLAMA_CPP_EMBED_GGUF pointing at a BERT-class embedding GGUF"]
        async fn llama_cpp_embed_through_gateway_returns_embeddings_for_a_real_model() {
            let path = std::env::var("SENSEI_LLAMA_CPP_EMBED_GGUF")
                .expect("SENSEI_LLAMA_CPP_EMBED_GGUF must point at a GGUF file");

            let registry = AdapterRegistry::new();
            let adapter_id =
                register_llama_cpp_embed(&registry, &path, "test-llama-embed-minilm")
                    .await
                    .expect("register llama-cpp embed adapter");
            assert_eq!(adapter_id, "llama-cpp");

            let gw = gateway_with_one(
                registry,
                "test-llama-embed-minilm",
                Capability::TextEmbed,
            );

            let request = InferenceRequest {
                capability: Capability::TextEmbed,
                model: Some("test-llama-embed-minilm".into()),
                router: Some("llama-cpp".into()),
                chain: None,
                payload: Payload::Embed {
                    texts: vec![
                        "hello from sensei".into(),
                        "embedded llama-cpp embed via gateway execute".into(),
                    ],
                },
                budget: None,
            };

            let response = gw.execute(&request).await.expect("gateway.execute");
            assert!(response.success);
            let embeddings = response.embeddings.expect("embeddings present");
            assert_eq!(embeddings.len(), 2);
            for (i, v) in embeddings.iter().enumerate() {
                assert!(!v.is_empty(), "embedding[{i}] should be non-empty");
            }
        }

        /// End-to-end smoke test for the chat path. Uses a generative
        /// GGUF (llama-3.x, qwen, mistral, …) and asks for a short
        /// response.
        ///
        /// ```text
        /// SENSEI_LLAMA_CPP_CHAT_GGUF=$HOME/.ollama/models/blobs/sha256-... \
        ///   cargo test -p senseid --features embedded-llama-cpp \
        ///     gateway_embedded -- --ignored
        /// ```
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        #[ignore = "requires SENSEI_LLAMA_CPP_CHAT_GGUF pointing at a generative GGUF"]
        async fn llama_cpp_chat_through_gateway_returns_text_for_a_real_model() {
            let path = std::env::var("SENSEI_LLAMA_CPP_CHAT_GGUF")
                .expect("SENSEI_LLAMA_CPP_CHAT_GGUF must point at a chat GGUF");

            let registry = AdapterRegistry::new();
            let adapter_id =
                register_llama_cpp_chat(&registry, &path, "test-llama-chat-model")
                    .await
                    .expect("register llama-cpp chat adapter");
            assert_eq!(adapter_id, "llama-cpp");

            let gw = gateway_with_one(
                registry,
                "test-llama-chat-model",
                Capability::TextChat,
            );

            let request = InferenceRequest {
                capability: Capability::TextChat,
                model: Some("test-llama-chat-model".into()),
                router: Some("llama-cpp".into()),
                chain: None,
                payload: Payload::Chat {
                    messages: vec![Message {
                        role: MessageRole::User,
                        content: "Reply with the single word: pong.".into(),
                        tool_call_id: None,
                    }],
                    system: None,
                    max_tokens: Some(16),
                    temperature: Some(0.0),
                },
                budget: None,
            };

            let response = gw.execute(&request).await.expect("gateway.execute");
            assert!(response.success);
            let content = response.content.expect("content present");
            assert!(!content.is_empty(), "expected non-empty generation");
            assert!(
                content.len() < 256,
                "expected short response under max_tokens cap, got {content:?}"
            );
        }
    }
}
