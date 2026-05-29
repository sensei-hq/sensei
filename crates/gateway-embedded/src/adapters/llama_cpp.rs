//! In-process inference adapter built on [`llama_cpp_2`].
//!
//! Why this exists: calling a local Ollama daemon over HTTP for short
//! embedding queries spends 5–9x more time in protocol overhead than on
//! the actual inference (measured in `rust-embedding-bench`). Loading the
//! same GGUF in-process and calling [`llama_cpp_2`] directly recovers that
//! latency. This adapter is the same idea behind the `InferenceAdapter`
//! trait that the rest of the gateway already speaks.
//!
//! First-cut design — one adapter holds one model.
//! - The adapter is loaded with a specific [`ModelEntry`]; requests whose
//!   `model` field disagrees return [`GatewayError::ModelUnavailable`].
//! - Streaming is unsupported (embeddings don't stream).
//! - Only [`Capability::TextEmbed`] is implemented for now. Chat/complete
//!   land in a subsequent commit on top of this scaffolding.
//!
//! Concurrency model:
//! - The [`llama_cpp_2`] [`LlamaModel`] is read-only and shared.
//! - The [`LlamaContext`] carries mutable state (kv cache, scratch
//!   buffers) and is wired through a `std::sync::Mutex`. Concurrent
//!   `execute()` calls serialise on the mutex; the work is fully blocking
//!   on llama.cpp's native side anyway.
//!
//! Lifetime gymnastics: a [`LlamaContext<'b>`] borrows from the
//! [`LlamaModel`] that created it. To store both in one struct without a
//! self-referential crate, we extend the context's lifetime to `'static`
//! via [`std::mem::transmute`] and rely on Rust's struct drop order
//! (fields drop in declaration order) to guarantee the context's storage
//! goes away before the model it borrows from. The
//! [`LlamaCppAdapter::context`] field is declared before
//! [`LlamaCppAdapter::model`] for that reason.

use crate::math::l2_normalize_in_place;
use crate::registry::{ModelEntry, ModelSource};
use async_trait::async_trait;
use futures::Stream;
use gateway::adapters::InferenceAdapter;
use gateway::types::capability::Capability;
use gateway::types::config::RouterConfig;
use gateway::types::error::GatewayError;
use gateway::types::request::{
    InferenceRequest, InferenceResponse, Payload, StreamChunk,
};
use llama_cpp_2::{
    context::{
        params::{LlamaContextParams, LlamaPoolingType},
        LlamaContext,
    },
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaModel},
    token::LlamaToken,
};
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Construction-time configuration for [`LlamaCppAdapter`].
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// Adapter id surfaced to the gateway. Defaults to `"llama-cpp"`.
    pub adapter_id: String,
    /// Stable model id this adapter serves. Requests must specify this id
    /// (or send `model = None`).
    pub model_id: String,
    /// Max sequence length the context will accept.
    pub n_ctx: u32,
    /// Threads used for both prompt processing and decode/encode.
    pub n_threads: i32,
    /// Pooling strategy for embedding-style models. `Mean` is the
    /// sentence-transformers default.
    pub pooling: LlamaPoolingType,
    /// Maximum number of distinct sequences in a single batch.
    pub n_seq_max: u32,
}

impl LlamaCppConfig {
    /// Convenience builder for the embedding case — short context, mean
    /// pooling, room for up to 64 concurrent sequences in one batch.
    pub fn embed(model_id: impl Into<String>) -> Self {
        Self {
            adapter_id: "llama-cpp".into(),
            model_id: model_id.into(),
            n_ctx: 512,
            n_threads: 1,
            pooling: LlamaPoolingType::Mean,
            n_seq_max: 64,
        }
    }
}

/// Newtype around [`LlamaContext`] that asserts `Send`.
///
/// llama-cpp-2 deliberately omits `Send`/`Sync` from [`LlamaContext`]
/// because it carries mutable native state. We restore `Send` here under
/// a stronger contract: the context is only ever accessed through the
/// surrounding `Mutex`, so at any moment exactly one thread is using it.
/// llama.cpp's own requirement is "don't share a context across threads
/// concurrently" — serialised use from different threads sequentially is
/// fine.
struct SyncContext(LlamaContext<'static>);

// SAFETY: see `SyncContext` doc — the surrounding `Mutex` ensures one
// thread at a time accesses the context, which is llama.cpp's actual
// safety invariant.
unsafe impl Send for SyncContext {}

/// In-process inference adapter backed by a single loaded GGUF model.
///
/// See module docs for the lifetime safety invariant — `context` must be
/// declared before `model` so it drops first.
pub struct LlamaCppAdapter {
    config: LlamaCppConfig,
    // Drop order: context first, then model, then backend.
    context: Mutex<SyncContext>,
    model: LlamaModel,
    _backend: Arc<LlamaBackend>,
}

impl LlamaCppAdapter {
    /// Load a GGUF model and build a context around it.
    pub fn load(
        backend: Arc<LlamaBackend>,
        entry: &ModelEntry,
        config: LlamaCppConfig,
    ) -> Result<Self, GatewayError> {
        let path = entry.source.path();
        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(backend.as_ref(), path, &model_params)
            .map_err(|e| Self::provider_err(&config, format!("model load: {e}")))?;

        // n_batch sizes the per-batch token budget. Use n_ctx so a single
        // sequence of up to n_ctx tokens fits; the bench harness used the
        // same shape and producing reliable embeddings.
        let n_batch = config.n_ctx;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(config.n_ctx))
            .with_n_batch(n_batch)
            .with_n_ubatch(n_batch)
            .with_n_seq_max(config.n_seq_max)
            .with_n_threads(config.n_threads)
            .with_n_threads_batch(config.n_threads)
            .with_embeddings(true)
            .with_pooling_type(config.pooling);

        let context = model
            .new_context(backend.as_ref(), ctx_params)
            .map_err(|e| Self::provider_err(&config, format!("context create: {e}")))?;

        // SAFETY: `context` borrows from `model` (and `_backend`). We extend
        // its lifetime to `'static` only for storage, and the surrounding
        // struct declares `context` before `model`/`_backend`, so Rust
        // drops `context` first when the adapter is dropped. The model and
        // backend are therefore guaranteed to outlive every use of
        // `context`, satisfying llama-cpp-2's actual aliasing invariant.
        let context: LlamaContext<'static> = unsafe { std::mem::transmute(context) };

        let adapter = Self {
            config,
            context: Mutex::new(SyncContext(context)),
            model,
            _backend: backend,
        };

        Ok(adapter)
    }

    /// Note about model source: we accept any [`ModelEntry`] regardless of
    /// `ModelSource` variant — the adapter just needs the on-disk path. The
    /// source kind matters at the registry layer, not at load time.
    pub fn check_source_supported(entry: &ModelEntry) -> Result<(), GatewayError> {
        if !entry.source.path().exists() {
            return Err(GatewayError::ProviderError {
                adapter: "llama-cpp".into(),
                message: format!(
                    "model bytes not found at {:?} (source: {:?})",
                    entry.source.path(),
                    match &entry.source {
                        ModelSource::Managed { .. } => "Managed",
                        ModelSource::Ollama { .. } => "Ollama",
                        ModelSource::External { .. } => "External",
                    }
                ),
                status: None,
            });
        }
        Ok(())
    }

    fn provider_err(config: &LlamaCppConfig, message: impl Into<String>) -> GatewayError {
        GatewayError::ProviderError {
            adapter: config.adapter_id.clone(),
            message: message.into(),
            status: None,
        }
    }

    fn err(&self, message: impl Into<String>) -> GatewayError {
        Self::provider_err(&self.config, message)
    }

    /// Public, trait-free embedding entry point — easier to use from tests
    /// or from sensei-internal callers that already have a borrow on the
    /// adapter and don't need the full [`InferenceRequest`] envelope.
    pub fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, GatewayError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        if texts.len() as u32 > self.config.n_seq_max {
            return Err(self.err(format!(
                "batch of {} exceeds n_seq_max={}",
                texts.len(),
                self.config.n_seq_max
            )));
        }

        let mut guard = self
            .context
            .lock()
            .map_err(|_| self.err("context mutex poisoned"))?;
        let ctx = &mut guard.0;

        let mut all_tokens: Vec<Vec<LlamaToken>> = Vec::with_capacity(texts.len());
        let mut total_tokens: usize = 0;
        for text in texts {
            let tokens = self
                .model
                .str_to_token(text, AddBos::Always)
                .map_err(|e| self.err(format!("tokenize: {e}")))?;
            total_tokens += tokens.len();
            all_tokens.push(tokens);
        }

        // Match the rust-embedding-bench llama_runner verbatim: mark only
        // the last token of each sequence as an output. llama.cpp's
        // embedding override picks up the rest. Marking every token as
        // output was reproducibly returning empty embeddings on the first
        // post-load batch on macOS with Metal.
        let mut batch = LlamaBatch::new(total_tokens.max(1), texts.len() as i32);
        for (seq_id, tokens) in all_tokens.iter().enumerate() {
            let last = tokens.len().saturating_sub(1);
            for (pos, &token) in tokens.iter().enumerate() {
                batch
                    .add(token, pos as i32, &[seq_id as i32], pos == last)
                    .map_err(|e| self.err(format!("batch.add: {e}")))?;
            }
        }

        ctx.clear_kv_cache();
        ctx.encode(&mut batch)
            .map_err(|e| self.err(format!("encode: {e}")))?;

        let mut results = Vec::with_capacity(texts.len());
        for seq_id in 0..texts.len() {
            let emb = ctx
                .embeddings_seq_ith(seq_id as i32)
                .map_err(|e| self.err(format!("read embedding[{seq_id}]: {e}")))?;
            if emb.is_empty() {
                // Defensive: encode() succeeded but the pooling layer left
                // this sequence's slot empty. Empirically this happens
                // when callers exercise different batch shapes against the
                // same context — e.g. a batch=1 call followed by a batch=N
                // call — under Metal. Surface as an error so the caller
                // can choose to retry or rebuild the adapter.
                return Err(self.err(format!(
                    "empty embedding for seq {seq_id} (batch_size={}, model={})",
                    texts.len(),
                    self.config.model_id
                )));
            }
            let mut owned = emb.to_vec();
            l2_normalize_in_place(&mut owned);
            results.push(owned);
        }
        Ok(results)
    }
}

#[async_trait]
impl InferenceAdapter for LlamaCppAdapter {
    fn id(&self) -> &str {
        &self.config.adapter_id
    }

    fn supports(&self, capability: &Capability) -> bool {
        matches!(capability, Capability::TextEmbed)
    }

    async fn execute(
        &self,
        _config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        if let Some(requested) = &request.model
            && requested != &self.config.model_id
        {
            return Err(GatewayError::ModelUnavailable {
                adapter: self.config.adapter_id.clone(),
                model: requested.clone(),
            });
        }

        match &request.payload {
            Payload::Embed { texts } => {
                let embeddings = self.embed(texts)?;
                Ok(InferenceResponse {
                    success: true,
                    content: None,
                    embeddings: Some(embeddings),
                    transcription: None,
                    audio: None,
                    images: None,
                    videos: None,
                    model: Some(self.config.model_id.clone()),
                    usage: None,
                    estimated_cost: None,
                    actual_cost: None,
                    attempts: vec![],
                })
            }
            _ => Err(self.err(
                "LlamaCppAdapter only supports Payload::Embed in this commit",
            )),
        }
    }

    async fn stream(
        &self,
        _config: &RouterConfig,
        _request: &InferenceRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>,
        GatewayError,
    > {
        Err(self.err("LlamaCppAdapter does not support streaming"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ModelFormat, ModelSource};
    use std::path::PathBuf;

    fn external_entry(path: impl Into<PathBuf>) -> ModelEntry {
        let path = path.into();
        ModelEntry {
            id: "test-model".into(),
            name: "test".into(),
            format: ModelFormat::Gguf,
            source: ModelSource::External { path },
            sha256: None,
            size_bytes: None,
        }
    }

    #[test]
    fn check_source_supported_rejects_missing_file_with_provider_error() {
        let entry = external_entry("/definitely/does/not/exist/x.gguf");
        let err = LlamaCppAdapter::check_source_supported(&entry).unwrap_err();
        match err {
            GatewayError::ProviderError { message, .. } => {
                assert!(message.contains("not found"), "got: {message}");
            }
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[test]
    fn check_source_supported_accepts_existing_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let entry = external_entry(tmp.path());
        LlamaCppAdapter::check_source_supported(&entry).expect("present file");
    }

    #[test]
    fn embed_config_builder_sets_sensible_defaults_for_minilm() {
        let cfg = LlamaCppConfig::embed("all-minilm");
        assert_eq!(cfg.model_id, "all-minilm");
        assert_eq!(cfg.adapter_id, "llama-cpp");
        assert_eq!(cfg.n_ctx, 512);
        assert!(matches!(cfg.pooling, LlamaPoolingType::Mean));
        assert_eq!(cfg.n_seq_max, 64);
    }

    /// End-to-end embedding against a real GGUF. Ignored by default so
    /// `cargo test --features llama-cpp` doesn't require a model file on
    /// disk. Run with:
    ///
    ///     LLAMA_TEST_GGUF=$HOME/.ollama/models/blobs/sha256-... \
    ///       cargo test -p gateway-embedded --features llama-cpp -- --ignored
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires LLAMA_TEST_GGUF env var pointing at a BERT-class embedding GGUF"]
    async fn embed_against_real_model_returns_unit_length_vectors() {
        let path = std::env::var("LLAMA_TEST_GGUF")
            .expect("LLAMA_TEST_GGUF must point at a BERT GGUF file");
        let entry = external_entry(path);

        let backend = Arc::new(LlamaBackend::init().expect("backend init"));
        let adapter = LlamaCppAdapter::load(backend, &entry, LlamaCppConfig::embed("test-model"))
            .expect("load model");

        let texts = vec![
            "hello world".to_string(),
            "the quick brown fox jumps over the lazy dog".to_string(),
        ];
        let embeddings = adapter.embed(&texts).expect("embed");

        assert_eq!(embeddings.len(), 2);
        for (i, v) in embeddings.iter().enumerate() {
            assert!(!v.is_empty(), "embedding[{i}] dim should be > 0");
            let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                (mag - 1.0).abs() < 1e-4,
                "embedding[{i}]: expected unit-length, got |v|={mag}",
            );
        }
    }
}
