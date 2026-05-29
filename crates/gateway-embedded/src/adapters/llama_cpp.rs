//! In-process inference adapter built on [`llama_cpp_2`].
//!
//! Why this exists: calling a local Ollama daemon over HTTP for short
//! embedding queries spends 5–9x more time in protocol overhead than on
//! the actual inference (measured in `rust-embedding-bench`). Loading the
//! same GGUF in-process and calling [`llama_cpp_2`] directly recovers that
//! latency. This adapter is the same idea behind the `InferenceAdapter`
//! trait that the rest of the gateway already speaks.
//!
//! Design — one adapter holds one model and is configured at load time
//! into one of two modes:
//! - [`LlamaCppMode::Embedding`] supports [`Capability::TextEmbed`]
//!   via a single-shot encode + per-sequence pooled vector read.
//! - [`LlamaCppMode::Generation`] supports [`Capability::TextChat`] /
//!   [`Capability::TextComplete`] via an autoregressive decode loop with
//!   token sampling. Uses the model's bundled chat template to format
//!   messages.
//!
//! The adapter is loaded with a specific [`ModelEntry`]; requests whose
//! `model` field disagrees return [`GatewayError::ModelUnavailable`].
//! Streaming is not yet implemented — `stream()` returns an error.
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
    InferenceRequest, InferenceResponse, Message, MessageRole, Payload, StreamChunk,
};
use llama_cpp_2::{
    context::{
        params::{LlamaContextParams, LlamaPoolingType},
        LlamaContext,
    },
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaChatMessage, LlamaModel},
    sampling::LlamaSampler,
    token::LlamaToken,
};
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Discriminates between the two very different ways a llama.cpp context is
/// used. The choice is made at adapter load time because the context params
/// (`with_embeddings`, `with_pooling_type`) differ, and we cannot
/// re-purpose an embedding context for generation or vice versa.
#[derive(Debug, Clone)]
pub enum LlamaCppMode {
    /// Encode-only path used for BERT-class embedding models. Reads
    /// per-sequence pooled vectors after a single `encode()` call.
    Embedding { pooling: LlamaPoolingType },
    /// Autoregressive `decode()` loop with token sampling. Produces a
    /// text completion via the model's chat template.
    Generation {
        /// Cap on the number of new tokens per request when the caller
        /// doesn't specify `max_tokens`.
        default_max_tokens: u32,
        /// Temperature used when the caller doesn't specify one. `0.0`
        /// means greedy (always pick the highest-probability token).
        default_temperature: f32,
        /// Seed for the distribution sampler. The harness wants the same
        /// adapter to be reproducible across runs by default.
        seed: u32,
    },
}

/// Construction-time configuration for [`LlamaCppAdapter`].
#[derive(Debug, Clone)]
pub struct LlamaCppConfig {
    /// Adapter id surfaced to the gateway. Defaults to `"llama-cpp"`.
    pub adapter_id: String,
    /// Stable model id this adapter serves. Requests must specify this id
    /// (or send `model = None`).
    pub model_id: String,
    /// Embedding vs generation mode.
    pub mode: LlamaCppMode,
    /// Max sequence length the context will accept.
    pub n_ctx: u32,
    /// Threads used for both prompt processing and decode/encode.
    pub n_threads: i32,
    /// Maximum number of distinct sequences in a single batch. Embedding
    /// mode benefits from a generous value (concurrent batched embeds);
    /// generation typically uses 1.
    pub n_seq_max: u32,
}

impl LlamaCppConfig {
    /// Convenience builder for the embedding case — short context, mean
    /// pooling, room for up to 64 concurrent sequences in one batch.
    pub fn embed(model_id: impl Into<String>) -> Self {
        Self {
            adapter_id: "llama-cpp".into(),
            model_id: model_id.into(),
            mode: LlamaCppMode::Embedding {
                pooling: LlamaPoolingType::Mean,
            },
            n_ctx: 512,
            n_threads: 1,
            n_seq_max: 64,
        }
    }

    /// Convenience builder for the chat/generation case — 4k context,
    /// greedy decoding by default, single-sequence (no batched chat).
    pub fn chat(model_id: impl Into<String>) -> Self {
        Self {
            adapter_id: "llama-cpp".into(),
            model_id: model_id.into(),
            mode: LlamaCppMode::Generation {
                default_max_tokens: 512,
                default_temperature: 0.0,
                seed: 42,
            },
            n_ctx: 4096,
            n_threads: 1,
            n_seq_max: 1,
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

        // n_batch sizes the per-batch token budget. n_ctx works for both
        // single-sequence chat and small-batch embedding.
        let n_batch = config.n_ctx;
        let (with_embeddings, pooling) = match config.mode {
            LlamaCppMode::Embedding { pooling } => (true, pooling),
            LlamaCppMode::Generation { .. } => (false, LlamaPoolingType::Unspecified),
        };
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(config.n_ctx))
            .with_n_batch(n_batch)
            .with_n_ubatch(n_batch)
            .with_n_seq_max(config.n_seq_max)
            .with_n_threads(config.n_threads)
            .with_n_threads_batch(config.n_threads)
            .with_embeddings(with_embeddings)
            .with_pooling_type(pooling);

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
        if !matches!(self.config.mode, LlamaCppMode::Embedding { .. }) {
            return Err(self.err("adapter not configured for embedding (mode != Embedding)"));
        }
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

    /// Public, trait-free chat-generation entry point. Takes the same
    /// fields the gateway's [`Payload::Chat`] carries — messages, an
    /// optional system prompt, optional `max_tokens` / `temperature`
    /// overrides — and returns the generated text.
    pub fn generate(
        &self,
        messages: &[Message],
        system: Option<&str>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<String, GatewayError> {
        let (default_max, default_temp, seed) = match self.config.mode {
            LlamaCppMode::Generation {
                default_max_tokens,
                default_temperature,
                seed,
            } => (default_max_tokens, default_temperature, seed),
            _ => {
                return Err(
                    self.err("adapter not configured for generation (mode != Generation)")
                );
            }
        };
        let max_new = max_tokens.unwrap_or(default_max).max(1);
        let temperature = temperature.unwrap_or(default_temp);

        // Build prompt via the model's bundled chat template.
        let chat = build_chat_messages(messages, system)?;
        let template = self
            .model
            .chat_template(None)
            .map_err(|e| self.err(format!("chat template lookup: {e}")))?;
        let prompt = self
            .model
            .apply_chat_template(&template, &chat, true)
            .map_err(|e| self.err(format!("apply chat template: {e}")))?;

        let prompt_tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| self.err(format!("tokenize prompt: {e}")))?;

        // Reject prompts that don't leave room for the requested completion.
        if (prompt_tokens.len() as u32).saturating_add(max_new) > self.config.n_ctx {
            return Err(self.err(format!(
                "prompt ({} tokens) + max_new ({}) exceeds n_ctx ({})",
                prompt_tokens.len(),
                max_new,
                self.config.n_ctx
            )));
        }

        let mut guard = self
            .context
            .lock()
            .map_err(|_| self.err("context mutex poisoned"))?;
        let ctx = &mut guard.0;
        ctx.clear_kv_cache();

        // Feed the prompt: mark only the last token for logits since we
        // only need to sample from there.
        let mut batch = LlamaBatch::new(prompt_tokens.len().max(1), 1);
        let last_prompt = prompt_tokens.len().saturating_sub(1);
        for (pos, &token) in prompt_tokens.iter().enumerate() {
            batch
                .add(token, pos as i32, &[0], pos == last_prompt)
                .map_err(|e| self.err(format!("batch.add (prompt): {e}")))?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| self.err(format!("decode (prompt): {e}")))?;

        let mut sampler = if temperature <= 0.0 {
            LlamaSampler::greedy()
        } else {
            LlamaSampler::chain_simple([LlamaSampler::temp(temperature), LlamaSampler::dist(seed)])
        };

        // We accumulate raw bytes and UTF-8-decode at the end so multibyte
        // codepoints that span two tokens (common in non-Latin scripts and
        // emoji) don't get split into invalid UTF-8 fragments.
        let mut generated_bytes: Vec<u8> = Vec::new();
        let start_pos = prompt_tokens.len() as i32;
        for (offset, _) in (0..max_new).enumerate() {
            let next_pos = start_pos + offset as i32;
            // Sample from the logits at the last batch position.
            let token = sampler.sample(ctx, batch.n_tokens() - 1);
            sampler.accept(token);

            if self.model.is_eog_token(token) {
                break;
            }

            let bytes = self
                .model
                .token_to_piece_bytes(token, 32, false, None)
                .map_err(|e| self.err(format!("token_to_piece_bytes: {e}")))?;
            generated_bytes.extend_from_slice(&bytes);

            // Feed the sampled token back in for the next step.
            batch.clear();
            batch
                .add(token, next_pos, &[0], true)
                .map_err(|e| self.err(format!("batch.add (step): {e}")))?;
            ctx.decode(&mut batch)
                .map_err(|e| self.err(format!("decode (step): {e}")))?;
        }

        Ok(String::from_utf8_lossy(&generated_bytes).into_owned())
    }
}

/// Convert gateway's `Message` list (plus optional system prompt) into the
/// `LlamaChatMessage` shape that llama-cpp-2's chat template applier wants.
fn build_chat_messages(
    messages: &[Message],
    system: Option<&str>,
) -> Result<Vec<LlamaChatMessage>, GatewayError> {
    let mut chat = Vec::with_capacity(messages.len() + 1);
    if let Some(sys) = system {
        chat.push(
            LlamaChatMessage::new("system".to_string(), sys.to_string())
                .map_err(|e| chat_msg_err(&format!("system: {e}")))?,
        );
    }
    for msg in messages {
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        chat.push(
            LlamaChatMessage::new(role.to_string(), msg.content.clone())
                .map_err(|e| chat_msg_err(&format!("{role}: {e}")))?,
        );
    }
    Ok(chat)
}

fn chat_msg_err(detail: &str) -> GatewayError {
    GatewayError::ProviderError {
        adapter: "llama-cpp".into(),
        message: format!("chat message: {detail}"),
        status: None,
    }
}

fn response_with_embeddings(model_id: &str, embeddings: Vec<Vec<f32>>) -> InferenceResponse {
    InferenceResponse {
        success: true,
        content: None,
        embeddings: Some(embeddings),
        transcription: None,
        audio: None,
        images: None,
        videos: None,
        model: Some(model_id.to_string()),
        usage: None,
        estimated_cost: None,
        actual_cost: None,
        attempts: vec![],
    }
}

fn response_with_content(model_id: &str, content: String) -> InferenceResponse {
    InferenceResponse {
        success: true,
        content: Some(content),
        embeddings: None,
        transcription: None,
        audio: None,
        images: None,
        videos: None,
        model: Some(model_id.to_string()),
        usage: None,
        estimated_cost: None,
        actual_cost: None,
        attempts: vec![],
    }
}

#[async_trait]
impl InferenceAdapter for LlamaCppAdapter {
    fn id(&self) -> &str {
        &self.config.adapter_id
    }

    fn supports(&self, capability: &Capability) -> bool {
        matches!(
            (&self.config.mode, capability),
            (LlamaCppMode::Embedding { .. }, Capability::TextEmbed)
                | (LlamaCppMode::Generation { .. }, Capability::TextChat)
                | (LlamaCppMode::Generation { .. }, Capability::TextComplete)
        )
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
                Ok(response_with_embeddings(&self.config.model_id, embeddings))
            }
            Payload::Chat {
                messages,
                system,
                max_tokens,
                temperature,
            } => {
                let content = self.generate(
                    messages,
                    system.as_deref(),
                    *max_tokens,
                    *temperature,
                )?;
                Ok(response_with_content(&self.config.model_id, content))
            }
            _ => Err(self.err(
                "LlamaCppAdapter supports Payload::Embed (Embedding mode) \
                 and Payload::Chat (Generation mode) only",
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

    /// `LlamaBackend::init()` may only be called once per process. Cargo
    /// runs `#[test]`s in parallel threads inside one process, so the two
    /// integration tests below have to share a single backend handle —
    /// otherwise the second to run gets an "already initialized" error.
    fn shared_backend() -> Arc<LlamaBackend> {
        use std::sync::OnceLock;
        static BACKEND: OnceLock<Arc<LlamaBackend>> = OnceLock::new();
        BACKEND
            .get_or_init(|| Arc::new(LlamaBackend::init().expect("LlamaBackend::init")))
            .clone()
    }

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
        assert_eq!(cfg.n_seq_max, 64);
        match cfg.mode {
            LlamaCppMode::Embedding { pooling } => assert!(matches!(pooling, LlamaPoolingType::Mean)),
            _ => panic!("expected Embedding mode"),
        }
    }

    #[test]
    fn chat_config_builder_sets_sensible_defaults_for_generation() {
        let cfg = LlamaCppConfig::chat("llama-3.2");
        assert_eq!(cfg.model_id, "llama-3.2");
        assert_eq!(cfg.adapter_id, "llama-cpp");
        assert_eq!(cfg.n_ctx, 4096);
        assert_eq!(cfg.n_seq_max, 1);
        match cfg.mode {
            LlamaCppMode::Generation {
                default_max_tokens,
                default_temperature,
                ..
            } => {
                assert_eq!(default_max_tokens, 512);
                assert_eq!(default_temperature, 0.0);
            }
            _ => panic!("expected Generation mode"),
        }
    }

    #[test]
    fn build_chat_messages_translates_roles_and_prepends_system() {
        let msgs = vec![
            Message {
                role: MessageRole::User,
                content: "hi".into(),
                tool_call_id: None,
            },
            Message {
                role: MessageRole::Assistant,
                content: "hello".into(),
                tool_call_id: None,
            },
        ];
        let chat = build_chat_messages(&msgs, Some("you are a test bot")).unwrap();
        assert_eq!(chat.len(), 3, "system + user + assistant");
        // LlamaChatMessage doesn't expose role/content getters, but the
        // count + non-error construction is what we need here.
    }

    #[test]
    fn build_chat_messages_rejects_content_with_null_bytes() {
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "has\0null".into(),
            tool_call_id: None,
        }];
        let err = build_chat_messages(&msgs, None).unwrap_err();
        match err {
            GatewayError::ProviderError { message, .. } => {
                assert!(message.contains("user"), "got: {message}");
            }
            other => panic!("expected ProviderError, got {other:?}"),
        }
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

        let backend = shared_backend();
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

    /// End-to-end chat against a real generative GGUF (e.g. llama-3.2,
    /// qwen2.5, etc.). Ignored by default. Run with:
    ///
    ///     LLAMA_TEST_CHAT_GGUF=$HOME/.ollama/models/blobs/sha256-... \
    ///       cargo test -p gateway-embedded --features llama-cpp -- --ignored
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires LLAMA_TEST_CHAT_GGUF env var pointing at a generative GGUF with a chat template"]
    async fn generate_against_real_model_returns_non_empty_text() {
        let path = std::env::var("LLAMA_TEST_CHAT_GGUF")
            .expect("LLAMA_TEST_CHAT_GGUF must point at a generative GGUF");
        let entry = external_entry(path);

        let backend = shared_backend();
        let mut cfg = LlamaCppConfig::chat("test-chat-model");
        // Keep the test fast: cap to 16 new tokens.
        if let LlamaCppMode::Generation {
            default_max_tokens, ..
        } = &mut cfg.mode
        {
            *default_max_tokens = 16;
        }
        let adapter = LlamaCppAdapter::load(backend, &entry, cfg).expect("load model");

        let messages = vec![Message {
            role: MessageRole::User,
            content: "Reply with the single word: pong.".to_string(),
            tool_call_id: None,
        }];
        let text = adapter
            .generate(&messages, None, Some(16), Some(0.0))
            .expect("generate");

        assert!(!text.is_empty(), "expected non-empty generation");
        assert!(text.len() < 256, "expected short response under max_tokens cap, got {text:?}");
    }
}
