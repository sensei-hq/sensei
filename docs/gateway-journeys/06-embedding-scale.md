---
name: Embedding at Scale
description: Full codebase reindex → batch embedding generation → semantic search activates
date: 2026-04-24
status: idea
---

# Journey: Embedding at Scale

## Scenario

A developer runs `sensei init` on a 500-file TypeScript project. The indexing pipeline needs to generate embeddings for all symbols and documentation to enable semantic search.

## What happens

### 1. Initial scan

```
Scanning project: 500 files
  → 2,847 symbols extracted (functions, classes, types, constants)
  → 43 documentation files
  → 2,890 items to embed
```

### 2. Chunking

Long content is split into embeddable chunks:

```
Symbols:
  - Functions < 512 tokens: embed as-is (2,340 items)
  - Functions > 512 tokens: split at logical boundaries (507 chunks)

Documentation:
  - Split by heading sections (186 chunks)

Total chunks: 3,033
```

### 3. Batch embedding

The embed chain batches requests to minimize round-trips:

```
Batch 1: 64 chunks → ollama/all-minilm:l6-v2 → 64 vectors (42ms)
Batch 2: 64 chunks → ollama/all-minilm:l6-v2 → 64 vectors (38ms)
...
Batch 48: 25 chunks → ollama/all-minilm:l6-v2 → 25 vectors (18ms)

Total: 3,033 embeddings generated
  Duration: ~2.1 seconds
  Cost: $0.00 (all local)
  Model: all-minilm:l6-v2 (384 dimensions)
```

### 4. Storage

Embeddings stored in PostgreSQL via pgvector:

```sql
INSERT INTO embeddings (source_type, source_id, model, dimensions, vector)
VALUES ('symbol', $uuid, 'all-minilm:l6-v2', 384, $vector);
```

Index created for similarity search:

```sql
CREATE INDEX ON embeddings USING ivfflat (vector vector_cosine_ops) WITH (lists = 100);
```

### 5. Semantic search activates

```
search("where do we handle token refresh?")
  → Embed query: "where do we handle token refresh?" → [0.034, -0.089, ...]
  → Similarity search:
    1. auth/token_manager.ts:refresh_token()     — 0.91 similarity
    2. auth/middleware.ts:check_expiry()          — 0.84 similarity
    3. auth/types.ts:RefreshTokenConfig           — 0.79 similarity
    4. docs/ideas/12-auth-flow.md § Token Refresh — 0.76 similarity
```

Results span code AND documentation — unified semantic search across the entire project.

### 6. Incremental updates

After initial index, file changes trigger incremental re-embedding:

```
File saved: auth/token_manager.ts
  → Re-parse: 3 symbols changed
  → Re-embed: 3 chunks (not 3,033)
  → Duration: ~15ms
  → Old embeddings replaced
```

## Performance characteristics

| Project size | Symbols | Chunks | Initial index time | Incremental (1 file) |
|-------------|---------|--------|-------------------|---------------------|
| 50 files | ~300 | ~350 | ~0.3s | ~10ms |
| 500 files | ~2,800 | ~3,000 | ~2.1s | ~15ms |
| 5,000 files | ~25,000 | ~30,000 | ~21s | ~15ms |
| 50,000 files | ~200,000 | ~250,000 | ~3.5min | ~15ms |

Initial index scales linearly. Incremental is constant (only changed symbols).

## What if Ollama isn't running?

```
Indexing src/main.ts...
  → embed("function main()") → embed_chain → ollama: connection refused
  → circuit_breaker: Open
  → noop: "embedding unavailable"
  → Embedding skipped for this file
  → search() falls back to full-text (tsvector) — still works, less smart
```

When Ollama comes back, a background job re-embeds any files that were skipped.
