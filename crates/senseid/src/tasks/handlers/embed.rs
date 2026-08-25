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

    let mut embedded = 0u32;
    let mut consecutive_failures = 0u32;
    for batch in &batches {
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

    tracing::info!("embed_nodes: {} — embedded {} nodes", task.folder_name(), embedded);
    Ok(embedded)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
