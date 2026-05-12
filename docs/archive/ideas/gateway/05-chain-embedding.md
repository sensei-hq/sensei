---
name: Embedding Chain
description: Vector generation for semantic search — local-only by design, no external fallback, batch-optimized
date: 2026-04-24
status: idea
related: 04-chain-inference.md, 09-fallback-degradation.md
---

# Embedding Chain

## Purpose

Generate dense vector embeddings for:
- **Code symbols** — functions, classes, types → semantic code search
- **Documentation** — docs, ideas, journeys → natural language codebase queries
- **Commit messages / PR descriptions** — change-level semantic indexing
- **User queries** — embed the question to match against stored embeddings

This is the foundation of the semantic search layer (sensei idea 31).

## Why local-only

Embeddings are different from other inference tasks:

1. **Volume** — initial indexing generates thousands of embeddings; every file change triggers re-embedding
2. **Privacy** — code content is sent to the model; local keeps it on-device
3. **Latency** — embedding calls are simple (no reasoning); local models are fast enough
4. **Cost** — at scale, external embedding APIs add up; local is free
5. **Consistency** — switching embedding models changes the vector space; all embeddings must use the same model

For these reasons, the embedding chain has **no external fallback**. If the local model is unavailable, embeddings are skipped (the system degrades to keyword search only).

## Default chain

```yaml
embed_chain:
  capability: embed
  fallback_triggers: [timeout, model_unavailable]
  models:
    - model: all-minilm:l6-v2
      router: ollama
      priority: 1          # 384-dim, fast, good for code
    - model: nomic-embed-text
      router: ollama
      priority: 2          # 768-dim, higher quality, slower
```

No external providers. Fallback is between local models only.

## Embedding dimensions

| Model | Dimensions | Speed | Quality | Use case |
|-------|-----------|-------|---------|----------|
| all-minilm:l6-v2 | 384 | Fast | Good | Default for most indexing |
| nomic-embed-text | 768 | Medium | Better | Higher-quality semantic search |
| mxbai-embed-large | 1024 | Slow | Best | When accuracy matters more than speed |

**Important:** All embeddings in a project must use the same model/dimension. Switching models requires re-indexing. The chain locks to one model at a time; the fallback is only for availability, not quality.

## Batch processing

Embedding is naturally batch-friendly:

```rust
pub struct EmbedRequest {
    pub texts: Vec<String>,          // batch of texts to embed
    pub model: Option<String>,       // override chain default
}

pub struct EmbedResponse {
    pub embeddings: Vec<Vec<f32>>,   // one vector per input text
    pub model: String,
    pub dimensions: usize,
    pub duration_ms: u64,
}
```

Ollama's `/v1/embeddings` accepts multiple inputs in one call. The adapter batches to reduce round-trips.

## Chunking strategy

Long content (full files, large docs) must be chunked before embedding:

| Content type | Chunk strategy | Max tokens |
|-------------|---------------|------------|
| Code symbols | Per-function/class (natural boundaries) | ~512 |
| Documentation | Per-section (heading boundaries) | ~512 |
| Large files | Sliding window with overlap | ~512, 64 overlap |

Chunking happens before the embedding chain is called — the chain receives pre-chunked text.

## Storage

Embeddings are stored in PostgreSQL via pgvector:

```sql
-- in sensei's schema
CREATE TABLE embeddings (
    id          uuid PRIMARY KEY,
    source_type text NOT NULL,       -- symbol, doc, chunk
    source_id   uuid NOT NULL,       -- FK to symbols, doc_sections, etc.
    model       text NOT NULL,       -- which model generated this
    dimensions  smallint NOT NULL,
    vector      vector(384),         -- or vector(768) depending on model
    created_at  timestamptz DEFAULT now()
);
```

Similarity search via `<=>` (cosine distance) or `<->` (L2 distance).

## Degradation

When Ollama is not running:
- New embeddings are not generated
- Existing embeddings remain queryable
- Semantic search falls back to full-text search (tsvector)
- No error to the user — the search layer transparently degrades

## Open questions

| # | Question |
|---|----------|
| 1 | Should we support switching embedding models with automatic re-indexing? Or require manual `gateway reindex --embeddings`? |
| 2 | What's the right chunking overlap for code? Too little misses context, too much wastes tokens. |
| 3 | Should embedding generation be async (queue + worker) or synchronous (block until done)? Async is better for large batch reindex. |
| 4 | Should we store raw text alongside the vector for debugging/re-embedding? Storage cost vs. convenience. |
