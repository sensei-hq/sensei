//! Embed phase: generate vector embeddings for code-graph nodes so semantic
//! search and duplicate detection can rank by cosine similarity. Runs as a
//! barrier task after a folder's files are processed.

use super::super::Task;
use super::super::executor::TaskContext;

/// Expected embedding width — matches the `vector(384)` column on sensei.nodes.
const EMBED_DIM: usize = 384;
/// The embed chain's micro-batch token limit. gateway-embedded builds the
/// `embed` model with `n_ctx = n_batch = n_ubatch = 512` (`LlamaCppConfig::embed`
/// in gateway-embedded `adapters/llama_cpp.rs`). Its BERT-class encoder
/// processes a whole batch in ONE `encode()` with no ubatch splitting, so
/// llama.cpp asserts `n_ubatch >= (SUM of tokens over EVERY sequence in the
/// batch)`. Violating it calls `abort()` (a `GGML_ASSERT`) which is UNCATCHABLE
/// and kills the daemon mid-scan — so each batch's *total* token count must be
/// kept under this, conservatively, BEFORE the call. A char cap on a single
/// input can't help: the encoder sees the batch total, not the longest input.
const EMBED_UBATCH_TOKENS: usize = 512;
/// Conservative per-batch token budget, held well under `EMBED_UBATCH_TOKENS`
/// to absorb tokenizer variance and the per-sequence BOS token the model
/// prepends. `pack_embed_batches` never lets a batch's estimated tokens exceed
/// this, so the encoder's `n_ubatch >= total_tokens` assert always holds.
const EMBED_BATCH_TOKEN_BUDGET: usize = 384;
/// Max sequences per batch — the embed model's `n_seq_max`. The token budget
/// usually binds first, but a flood of tiny texts must still not exceed it.
const EMBED_MAX_SEQS: usize = 64;
/// Hard character cap for a single embed input. Guarantees one text alone can
/// never approach n_ubatch: the wordpiece embed model emits at most one token
/// per character, so ≤ 256 chars ⇒ ≤ 257 tokens (with BOS) < 512. The head
/// (kind + name + signature start) carries the semantic signal.
const EMBED_MAX_CHARS: usize = 256;

// Machine-check the abort-prevention invariants at compile time, or a batch
// could trip the GGML abort: the full-batch budget must stay under the
// encoder's n_ubatch, and a lone maximal input (EMBED_MAX_CHARS chars + 1 BOS
// token) must fit within that budget so it always packs into some batch.
const _: () = assert!(EMBED_BATCH_TOKEN_BUDGET < EMBED_UBATCH_TOKENS);
const _: () = assert!(EMBED_MAX_CHARS < EMBED_BATCH_TOKEN_BUDGET);
/// Per-batch wall-clock cap for the embedding call. Embedding is best-effort and
/// backfillable, so a slow/stalled backend (e.g. Ollama not responding) must
/// never hang a worker — that would starve resolve_libs and block the folder
/// from ever being marked indexed.
const EMBED_TIMEOUT_SECS: u64 = 30;
/// Give up embedding a folder after this many consecutive batch failures — the
/// backend is unhealthy; leave the remaining nodes for `/api/embed/backfill`.
const EMBED_MAX_CONSECUTIVE_FAILURES: u32 = 5;
/// Default per-run cap on embedding backend calls (#359). A first scan of a
/// monorepo packs thousands of batches; firing them all in one run bursts the
/// local encoder and wedges the app. Each run spends at most this many calls,
/// then a follow-up task continues the rest in the background. Config
/// `embed.calls_per_run` overrides.
const DEFAULT_EMBED_CALLS_PER_RUN: usize = 64;

/// Parse `embed.calls_per_run`: positive integers win; anything else (unset,
/// garbage, zero, negative) falls back to the default (batch_per_tick precedent).
fn embed_calls_per_run(cfg: Option<String>) -> usize {
    cfg.and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|&n| n > 0)
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_EMBED_CALLS_PER_RUN)
}

/// Per-run call budget (#359): every gateway invocation spends one `take()`.
/// Past zero the run stops and the remainder waits for the follow-up task, so
/// the burst is bounded no matter how many batches or per-text retries remain.
struct CallBudget {
    remaining: usize,
}

impl CallBudget {
    fn new(cap: usize) -> Self {
        Self { remaining: cap }
    }
    /// Spend one call. False = budget exhausted, stop the run. (Callers track
    /// the denial itself: an empty budget at loop end may mean the work fit
    /// exactly, which must NOT reenqueue.)
    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

/// Build the text fed to the embedding model for a node. Combines kind, name,
/// signature, and file location so semantically related symbols land near each
/// other in vector space.
fn embed_text(kind: &str, name: &str, signature: Option<&str>, file_path: &str) -> String {
    let mut s = String::with_capacity(name.len() + 32);
    s.push_str(kind);
    s.push(' ');
    s.push_str(name);
    if let Some(sig) = signature.map(str::trim).filter(|s| !s.is_empty()) {
        s.push_str(" — ");
        s.push_str(sig);
    }
    if !file_path.is_empty() {
        s.push_str(" [");
        s.push_str(file_path);
        s.push(']');
    }
    cap_embed_input(s)
}

/// Truncate a single embed input to a token-safe length on a char boundary
/// (`EMBED_MAX_CHARS`). Pure. Keeps a lone input's token count well under the
/// encoder's n_ubatch (see `EMBED_MAX_CHARS`) — but note this alone does NOT
/// prevent the daemon-killing abort: the encoder asserts on the *batch* total,
/// so `pack_embed_batches` bounds the per-batch sum as well.
///
/// `s.len()` is a BYTE count; comparing it to a char cap only ever truncates
/// *earlier* for multi-byte text (fewer chars ⇒ fewer tokens), so it stays safe.
pub(crate) fn cap_embed_input(mut s: String) -> String {
    if s.len() > EMBED_MAX_CHARS {
        let mut end = EMBED_MAX_CHARS;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    s
}

/// Conservative upper bound on the tokens one capped embed input produces, used
/// only to *pack* batches — never to reject. The wordpiece embed model emits
/// ≤ 1 token per character and prepends a single BOS, so `chars + 1` can never
/// under-count the real length (an under-count is what would abort the daemon).
fn est_tokens(text: &str) -> usize {
    text.chars().count() + 1
}

/// Group `texts` (each already `cap_embed_input`-bounded by `embed_text`) into
/// batches whose combined conservative token estimate stays within
/// `EMBED_BATCH_TOKEN_BUDGET` and whose length stays within `EMBED_MAX_SEQS`,
/// returning in-order index groups into `texts`.
///
/// This is the load-bearing guard against the GGML abort: the embed chain's
/// BERT encoder processes a whole batch in a single `encode()` bounded by
/// n_ubatch, so the *sum* of tokens over every sequence — not any one input —
/// must stay under it. A fixed 64-count batch of ordinary code-symbol texts
/// sums to thousands of tokens and aborts. Because each input is pre-capped to
/// ≤ `EMBED_MAX_CHARS`, a lone text (est ≤ 257) always fits a batch, so no node
/// is ever dropped. Pure — unit-tested below. Shared with the corrections-embed
/// path (`corrections::embed_items`) so both bound the encoder identically.
pub(crate) fn pack_embed_batches(texts: &[String]) -> Vec<Vec<usize>> {
    let mut batches: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_tokens = 0usize;
    for (i, text) in texts.iter().enumerate() {
        let t = est_tokens(text);
        // Close the current batch before adding this text would breach the
        // token budget or the sequence cap. Never split off the very first
        // element (a lone capped text is always within budget).
        let would_overflow = !current.is_empty()
            && (current_tokens + t > EMBED_BATCH_TOKEN_BUDGET || current.len() >= EMBED_MAX_SEQS);
        if would_overflow {
            batches.push(std::mem::take(&mut current));
            current_tokens = 0;
        }
        current.push(i);
        current_tokens += t;
    }
    if !current.is_empty() {
        batches.push(current);
    }
    batches
}

/// Outcome of embedding one batch of texts.
enum BatchOutcome {
    Ok(Vec<Vec<f32>>),
    /// Backend returned an error (e.g. a text still exceeds the context window).
    Failed(String),
    /// The call exceeded the per-batch timeout (stalled backend).
    TimedOut,
}

/// Embed a batch of texts via the pinned 384-dim `embed` chain, bounded by the
/// per-batch timeout.
async fn embed_batch(ctx: &TaskContext, texts: Vec<String>) -> BatchOutcome {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Payload};
    let request = InferenceRequest {
        capability: Capability::TextEmbed,
        model: None,
        router: None,
        // Pin the 384-dim `embed` chain so node embeddings always match the
        // vector(384) column, regardless of other TextEmbed models present.
        chain: Some("embed".to_string()),
        payload: Payload::Embed { texts },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
        routing: None,
    };
    match tokio::time::timeout(
        std::time::Duration::from_secs(EMBED_TIMEOUT_SECS),
        ctx.app_state.gateway.execute(&request),
    )
    .await
    {
        Ok(Ok(r)) => BatchOutcome::Ok(r.embeddings.unwrap_or_default()),
        Ok(Err(e)) => BatchOutcome::Failed(e.to_string()),
        Err(_) => BatchOutcome::TimedOut,
    }
}

/// Persist one node embedding, bumping `embedded`. A wrong width means the
/// configured embed chain doesn't match the `vector(384)` column — a config
/// error worth failing the task loudly.
async fn store_embedding(
    ctx: &TaskContext,
    id: &uuid::Uuid,
    vector: &[f32],
    embedded: &mut u32,
) -> Result<(), String> {
    if vector.len() != EMBED_DIM {
        return Err(format!(
            "expected {EMBED_DIM}-dim embeddings (sensei.nodes.embedding is vector({EMBED_DIM})), \
             got {} — check the embedding model bound to the inference role",
            vector.len()
        ));
    }
    ctx.pg().set_node_embedding(id, vector).await?;
    *embedded += 1;
    Ok(())
}

/// Embed every not-yet-embedded code-graph node for the task's folder.
/// `folder_path` is the repo abs_path (Task contract).
pub async fn embed_nodes(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = match ctx.pg().get_repo_by_path(task.folder_abs_path()).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("embed_nodes: {} — get_repo_by_path failed: {e}", task.folder_path);
            return Ok(0);
        }
    };
    let Some(folder_id) = folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"]))
    else {
        tracing::warn!("embed_nodes: {} — folder not found by path", task.folder_path);
        return Ok(0); // folder removed before this barrier ran — nothing to embed
    };

    // (id, kind, name, signature, file_path) for each node still missing an embedding.
    let nodes = ctx.pg().nodes_without_embeddings(&folder_id, 10_000).await?;
    tracing::debug!(
        "embed_nodes: {} (id={folder_id}) — {} pending nodes",
        task.folder_path,
        nodes.len()
    );
    if nodes.is_empty() {
        return Ok(0);
    }

    // Build the capped embed text for every pending node once, then pack them
    // into token-bounded batches. Batching by a fixed count (the old 64) let a
    // batch's summed tokens exceed the encoder's n_ubatch and abort the whole
    // daemon; `pack_embed_batches` guarantees each encode stays under it.
    let texts: Vec<String> = nodes
        .iter()
        .map(|(_, kind, name, sig, fp)| embed_text(kind, name, sig.as_deref(), fp))
        .collect();
    let batches = pack_embed_batches(&texts);

    // #359: bound the burst — batch attempts AND per-text retries share one
    // per-run call budget, spent via `take()` at both gateway call sites below.
    let cap = embed_calls_per_run(ctx.pg().get_config("embed.calls_per_run").await.ok().flatten());
    let mut budget = CallBudget::new(cap);
    let mut embedded = 0u32;
    let mut consecutive_failures = 0u32;
    let mut exhausted = false;
    for batch in &batches {
        if !budget.take() {
            exhausted = true;
            break;
        }
        let batch_texts: Vec<String> = batch.iter().map(|&i| texts[i].clone()).collect();

        // Embedding is best-effort and backfillable: a stalled or erroring
        // backend must never fail the task or freeze the worker.
        match embed_batch(ctx, batch_texts.clone()).await {
            BatchOutcome::Ok(vectors) if vectors.len() == batch.len() => {
                consecutive_failures = 0;
                for (&i, vector) in batch.iter().zip(vectors.iter()) {
                    store_embedding(ctx, &nodes[i].0, vector, &mut embedded).await?;
                }
            }
            // A count mismatch or a backend error (typically one text exceeding
            // the context window — e.g. a const holding a large data array)
            // retries each text on its own so a single bad input can't sink the
            // whole batch. Only the offending node is skipped.
            outcome @ (BatchOutcome::Ok(_) | BatchOutcome::Failed(_)) => {
                if let BatchOutcome::Failed(e) = &outcome {
                    tracing::debug!(
                        "embed_nodes: {} — batch failed, retrying per-text: {e}",
                        task.folder_name()
                    );
                }
                let mut any_ok = false;
                for (&i, text) in batch.iter().zip(batch_texts) {
                    if !budget.take() {
                        exhausted = true;
                        break;
                    }
                    match embed_batch(ctx, vec![text]).await {
                        BatchOutcome::Ok(mut v) => {
                            if let Some(vector) = v.pop() {
                                store_embedding(ctx, &nodes[i].0, &vector, &mut embedded).await?;
                                any_ok = true;
                            }
                        }
                        BatchOutcome::Failed(e) => {
                            let (_, kind, name, ..) = &nodes[i];
                            tracing::debug!("embed_nodes: skip {kind} {name} — {e}");
                        }
                        BatchOutcome::TimedOut => {
                            tracing::warn!(
                                "embed_nodes: {} — embed timed out, leaving rest for backfill",
                                task.folder_name()
                            );
                            return Ok(embedded);
                        }
                    }
                }
                if exhausted {
                    break;
                }
                consecutive_failures = if any_ok { 0 } else { consecutive_failures + 1 };
                if consecutive_failures >= EMBED_MAX_CONSECUTIVE_FAILURES {
                    tracing::warn!(
                        "embed_nodes: {} — backend unhealthy, leaving remaining nodes for backfill",
                        task.folder_name()
                    );
                    break;
                }
            }
            BatchOutcome::TimedOut => {
                tracing::warn!(
                    "embed_nodes: {} — batch timed out after {EMBED_TIMEOUT_SECS}s, leaving remaining nodes for backfill",
                    task.folder_name()
                );
                break;
            }
        }
    }

    // #359: budget spent with progress made and nodes still pending — continue
    // in the background instead of bursting. Plain `enqueue`, not
    // `enqueue_unique`: the running self covers the twin check, so unique
    // would drop the follow-up. No reenqueue on unhealthy backends (failures /
    // timeouts return early or break above with `exhausted` false) or without
    // progress (a wedged backend must not spin the queue). The pending query
    // is limit-capped, so leftovers are re-checked cheaply rather than inferred.
    if exhausted && embedded > 0 {
        let leftover = ctx.pg().nodes_without_embeddings(&folder_id, 1).await?;
        if !leftover.is_empty() {
            let follow = Task::new(super::super::TaskKind::EmbedNodes, &task.folder_path, "");
            ctx.queue.enqueue(follow).await;
            tracing::info!(
                "embed_nodes: {} — call budget spent after {embedded} nodes, follow-up queued",
                task.folder_name(),
            );
        }
    }

    tracing::info!("embed_nodes: {} — embedded {} nodes", task.folder_name(), embedded);
    Ok(embedded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskKind;

    #[test]
    fn embed_calls_per_run_defaults_parses_and_rejects_garbage() {
        // #359: the per-run LLM-call budget is configurable, with a sane
        // default when unset or unparseable (batch_per_tick precedent).
        assert_eq!(embed_calls_per_run(None), DEFAULT_EMBED_CALLS_PER_RUN);
        assert_eq!(embed_calls_per_run(Some("nonsense".into())), DEFAULT_EMBED_CALLS_PER_RUN);
        assert_eq!(embed_calls_per_run(Some("0".into())), DEFAULT_EMBED_CALLS_PER_RUN);
        assert_eq!(embed_calls_per_run(Some("-5".into())), DEFAULT_EMBED_CALLS_PER_RUN);
        assert_eq!(embed_calls_per_run(Some("32".into())), 32);
    }

    #[test]
    fn call_budget_denies_past_zero() {
        // #359: every gateway invocation spends one call; past zero the run
        // stops and the rest waits for the follow-up (backfill), so the burst
        // is bounded no matter how many batches or per-text retries remain.
        let mut budget = CallBudget::new(2);
        assert!(budget.take());
        assert!(budget.take());
        assert!(!budget.take());
        assert!(!budget.take());
    }

    #[test]
    fn embed_text_combines_kind_name_signature_and_path() {
        assert_eq!(
            embed_text("function", "get_user", Some("(id: Uuid) -> User"), "src/api.rs"),
            "function get_user — (id: Uuid) -> User [src/api.rs]"
        );
    }

    #[test]
    fn embed_text_omits_blank_signature_and_path() {
        assert_eq!(embed_text("class", "UserStore", None, ""), "class UserStore");
        assert_eq!(embed_text("const", "MAX", Some("   "), "lib.rs"), "const MAX [lib.rs]");
    }

    #[test]
    fn embed_text_caps_length() {
        let huge = "x".repeat(5000);
        let t = embed_text("function", "f", Some(&huge), "a.rs");
        assert!(t.len() <= 256, "embed text should be capped, got {}", t.len());
    }

    #[test]
    fn cap_embed_input_leaves_short_text_unchanged() {
        assert_eq!(cap_embed_input("function get_user".into()), "function get_user");
    }

    #[test]
    fn cap_embed_input_truncates_overlong_ascii_under_ubatch() {
        // A 1-char-per-token worst case (ASCII): capped chars ⇒ capped tokens.
        let huge = "x".repeat(5000);
        let capped = cap_embed_input(huge);
        assert!(capped.len() <= EMBED_MAX_CHARS, "byte length capped");
        assert!(
            est_tokens(&capped) < EMBED_UBATCH_TOKENS,
            "a single input must stay under n_ubatch even worst-case: est {} vs {}",
            est_tokens(&capped),
            EMBED_UBATCH_TOKENS,
        );
    }

    #[test]
    fn cap_embed_input_worst_case_multibyte_under_ubatch() {
        // All multi-byte codepoints: byte-based cap truncates even earlier, so
        // the char (⇒ token upper-bound) count is well under n_ubatch.
        let emoji = "🚀".repeat(5000); // 4 bytes each
        let capped = cap_embed_input(emoji);
        assert!(capped.len() <= EMBED_MAX_CHARS);
        assert!(
            est_tokens(&capped) < EMBED_UBATCH_TOKENS,
            "multibyte worst case must stay under n_ubatch: est {}",
            est_tokens(&capped),
        );
    }

    /// Every batch a packer produces must be safe to hand to the encoder: its
    /// summed conservative token estimate must not exceed n_ubatch (the abort
    /// threshold), it must respect the sequence cap, and — flattened, in order —
    /// it must cover every input exactly once.
    fn assert_batches_safe(texts: &[String], batches: &[Vec<usize>]) {
        let mut flat: Vec<usize> = Vec::new();
        for b in batches {
            assert!(!b.is_empty(), "no empty batches");
            assert!(b.len() <= EMBED_MAX_SEQS, "batch exceeds seq cap: {}", b.len());
            let sum: usize = b.iter().map(|&i| est_tokens(&texts[i])).sum();
            assert!(
                sum <= EMBED_BATCH_TOKEN_BUDGET,
                "batch est {sum} exceeds budget {EMBED_BATCH_TOKEN_BUDGET}",
            );
            assert!(sum < EMBED_UBATCH_TOKENS, "batch est {sum} would abort the encoder");
            flat.extend(b.iter().copied());
        }
        assert_eq!(flat, (0..texts.len()).collect::<Vec<_>>(), "batches cover all inputs in order");
    }

    #[test]
    fn pack_empty_yields_no_batches() {
        assert!(pack_embed_batches(&[]).is_empty());
    }

    #[test]
    fn pack_worst_case_max_length_texts_never_exceed_ubatch() {
        // Many maximal (capped) inputs — the exact shape that used to abort as a
        // fixed 64-count batch. Every produced batch must stay under n_ubatch.
        let texts: Vec<String> = (0..200).map(|_| "x".repeat(EMBED_MAX_CHARS)).collect();
        let batches = pack_embed_batches(&texts);
        assert_batches_safe(&texts, &batches);
        // At ~257 est tokens each and a 384 budget, each batch holds exactly one.
        assert!(batches.iter().all(|b| b.len() == 1), "max-length texts pack one per batch");
    }

    #[test]
    fn pack_small_texts_group_together_up_to_seq_cap() {
        let texts: Vec<String> = (0..150).map(|_| "fn a".to_string()).collect();
        let batches = pack_embed_batches(&texts);
        assert_batches_safe(&texts, &batches);
        // est_tokens("fn a") = 5; the 64-seq cap binds before the token budget
        // (64*5 = 320 ≤ 384), so full batches are exactly EMBED_MAX_SEQS.
        assert_eq!(batches[0].len(), EMBED_MAX_SEQS, "small texts fill to the seq cap");
    }

    #[test]
    fn pack_mixed_lengths_stay_safe() {
        let mut texts: Vec<String> = Vec::new();
        for i in 0..300 {
            texts.push(if i % 7 == 0 {
                "x".repeat(EMBED_MAX_CHARS)
            } else {
                "kind name".to_string()
            });
        }
        let batches = pack_embed_batches(&texts);
        assert_batches_safe(&texts, &batches);
    }

    // ── #359 behavioral pin: the wiring, not the parts ─────────────────────
    // The r1 unit tests pin the config parser and the `CallBudget` struct, but
    // nothing pinned `take()` AT the call sites, exhaustion stopping the run,
    // or the follow-up conditions — L1's unwired mutant (both `take()` gates
    // removed + the `exhausted` breaks neutralized) passed the whole suite.
    // This handler-level test kills it: with a 2-call cap over 5 one-per-batch
    // nodes, the run must stop after exactly 2 embeddings and a follow-up
    // `EmbedNodes` must be queued. Unwired embeds all 5 with no follow-up;
    // inverted (`if budget.take()`) embeds 0 with no follow-up.

    /// A test-only embed adapter returning real `EMBED_DIM`-wide vectors. The
    /// noop adapter answers no `embed` chain at all, so `embedded` stays 0 and
    /// the follow-up path is unreachable with it.
    struct FakeEmbedAdapter;

    impl gateway::adapters::capability::Model for FakeEmbedAdapter {
        fn id(&self) -> &str {
            "fake-embed"
        }
    }

    #[async_trait::async_trait]
    impl gateway::adapters::capability::EmbedModel for FakeEmbedAdapter {
        async fn embed(
            &self,
            _cfg: &gateway::types::config::RouterConfig,
            req: &gateway::types::io::EmbedRequest,
        ) -> Result<gateway::types::io::EmbedResponse, gateway::types::error::GatewayError>
        {
            Ok(gateway::types::io::EmbedResponse {
                embeddings: req.texts.iter().map(|_| vec![0.1f32; EMBED_DIM]).collect(),
                usage: None,
                degraded: false,
            })
        }
    }

    #[async_trait::async_trait]
    impl gateway::adapters::RegisterInto for FakeEmbedAdapter {
        async fn register_into(
            self: std::sync::Arc<Self>,
            reg: &gateway::adapters::AdapterRegistry,
        ) {
            reg.register_embed(self.clone()).await;
        }
    }

    /// [`make_ctx`](crate::tasks::test_support::make_ctx) with a gateway that
    /// can actually embed: the fake adapter above behind an `embed` chain (the
    /// chain `embed_batch` pins). Same shape as `init_gateway_test`.
    async fn make_embed_ctx() -> std::sync::Arc<TaskContext> {
        use gateway::types::capability::Capability;
        use gateway::types::config::*;
        use std::collections::HashMap;

        let adapters = gateway::adapters::AdapterRegistry::new();
        adapters.register(std::sync::Arc::new(FakeEmbedAdapter)).await;

        let mut routers = HashMap::new();
        routers.insert(
            "fake-embed".into(),
            RouterConfig {
                url: "http://fake-embed".into(),
                api_key_env: None,
                api_key: None,
                enabled: true,
                timeout_ms: None,
                headers: HashMap::new(),
            },
        );

        let mut models = HashMap::new();
        models.insert(
            "fake-embed".into(),
            ModelConfig {
                id: "fake-embed".into(),
                api_model_id: None,
                provider: "fake-embed".into(),
                capabilities: vec![Capability::TextEmbed],
                context_window: 1_000_000,
                max_output_tokens: 0,
                pricing: None,
                family: None,
                catalog: None,
            },
        );

        let mut chains = HashMap::new();
        chains.insert(
            "embed".into(),
            FallbackChainConfig {
                id: "embed".into(),
                capability: Capability::TextEmbed,
                models: vec![ChainEntry {
                    model: "fake-embed".into(),
                    router: Some("fake-embed".into()),
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
        let cb = gateway::circuit_breaker::CircuitBreakerManager::new(
            gateway::circuit_breaker::CircuitBreakerConfig::default(),
        );
        let gateway = std::sync::Arc::new(gateway::Gateway::new(config, adapters, cb));

        let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
        let app_state = std::sync::Arc::new(crate::api::state::SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx: {
                let (tx, _) = tokio::sync::broadcast::channel(16);
                tx
            },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
            diagrams: std::sync::Arc::new(crate::api::diagram_cache::DiagramCache::new(
                crate::api::diagram_cache::DIAGRAM_CACHE_ENTRIES,
            )),
            trackers: Default::default(),
        });
        std::sync::Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        })
    }

    #[tokio::test]
    async fn call_budget_stops_run_early_and_queues_followup() {
        // #359 pin: the wired behavior end to end — the budget is spent at the
        // call sites, the run stops past zero with nodes still pending, and a
        // follow-up `EmbedNodes` continues in the background.
        let ctx = make_embed_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let root = pg
            .add_watch_root(&format!("/_test/embed-cap-root-{uniq}"), "t", &serde_json::json!([]))
            .await
            .unwrap();
        let abs_path = format!("/_test/embed-cap-{uniq}");
        let fid = crate::tasks::test_support::seed_repo_folder(
            pg,
            &root,
            &format!("embed-cap-{uniq}"),
            &abs_path,
        )
        .await
        .unwrap();
        // Five nodes, each packing one-per-batch: ~200-char names push every
        // text's token estimate past half the 384 budget, so no batch holds two.
        for i in 0..5 {
            let name = format!("embed_cap_node_{i}_{}", "n".repeat(180));
            crate::tasks::test_support::seed_node(
                pg,
                &fid,
                "function",
                &name,
                "cap.rs",
                None,
                None,
                Some(i),
                Some(i),
            )
            .await
            .unwrap();
        }
        let prior = pg.get_config("embed.calls_per_run").await.unwrap();
        pg.set_config("embed.calls_per_run", "2").await.unwrap();

        let task = Task::new(TaskKind::EmbedNodes, &abs_path, "");
        let embedded = embed_nodes(&ctx, &task).await.unwrap();

        assert_eq!(
            embedded, 2,
            "a 2-call cap over 5 one-per-batch nodes stops the run after 2 (unwired: 5, inverted: 0)"
        );
        assert!(
            ctx.queue.has_pending_kind_folder(TaskKind::EmbedNodes, &abs_path).await,
            "exhaustion with progress made and nodes pending queues a follow-up EmbedNodes"
        );
        let leftover = pg.nodes_without_embeddings(&fid, 10).await.unwrap();
        assert_eq!(leftover.len(), 3, "the 3 unembedded nodes are the follow-up's work");

        // cleanup
        match prior {
            Some(v) => pg.set_config("embed.calls_per_run", &v).await.unwrap(),
            None => pg.delete_config("embed.calls_per_run").await.unwrap(),
        }
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1")
            .bind(fid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.files WHERE folder_id = $1")
            .bind(fid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id = $1")
            .bind(root)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(root)
            .execute(pool)
            .await
            .ok();
    }
}
