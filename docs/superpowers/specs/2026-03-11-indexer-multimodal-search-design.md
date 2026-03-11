# Indexer Multi-Modal Search — Design

**Date:** 2026-03-11
**Status:** Approved — ready for implementation
**Scope:** Multi-modal search only (semantic + BM25 + symbol). LLMSpec auto-population, symbol graph, and llms.txt generation are separate follow-on cycles.

---

## Objective

The current indexer extracts symbols via regex and exposes them through `list_exports` and `get_file_context`. There is no unified search — agents must know which file to look at before retrieving it. Multi-modal search closes this gap: a single `search(query)` tool that combines exact symbol matching, BM25 full-text, and semantic (embedding) search into one ranked result list. The benchmark shows regex-based symbol extraction beats chunk-vector search for symbol queries; this design keeps both and adds them together.

---

## Architecture

Three search layers, computed at index time, merged at query time:

```
query
  │
  ├─► Symbol search    — exact/prefix match on symbol-map.json names
  │                      always available, zero latency
  │
  ├─► BM25 full-text   — TF-IDF/BM25 over chunk text corpus
  │                      built at index time, queried in-process
  │
  └─► Semantic search  — cosine similarity against embedding vectors
                         Transformer.js all-MiniLM-L6-v2 (~80MB, ONNX)
                         generated at index time, queried in-process

All three results merged via Reciprocal Rank Fusion → unified ranked list
```

**No server dependency for search.** Everything runs in `@sensei/tools`. The `@sensei/server` / Ollama path remains for L2 deep code analysis — a separate concern.

**Graceful degradation:** if the embedding model is unavailable (first run, offline), symbol + BM25 layers still return results. Semantic layer is additive.

---

## Chunking Strategy

Chunks are the unit of both BM25 and embedding. Symbols and doc sections are natural chunks — no arbitrary text splitting needed.

### Code files → one chunk per symbol

```
chunk id:   "src/auth.ts:login"
chunk text: "login(email: string, password: string): Promise<User | null>
             // Authenticate user and return session token or null on failure"
```

L0 signature + L1 description combined. Symbols without L1 use L0 only.

### Markdown files → one chunk per section (H2/H3)

```
chunk id:   "docs/design/05-indexing.md#symbol-map"
chunk text: "Symbol Map — extracted exports stored at L0 and L1 per file.
             Code file extensions: .ts .tsx .js .jsx .py .go .rs ..."
```

Heading text + first 400 characters of section content.

---

## Storage Schema

Two new artifacts in `.sensei/`:

### `.sensei/chunks.json`

Text corpus for BM25 + chunk registry. Includes pre-computed BM25 term frequencies.

```json
{
  "version": 1,
  "corpusSize": 1240,
  "avgChunkLength": 87,
  "chunks": {
    "src/auth.ts:login": {
      "file": "src/auth.ts",
      "type": "symbol",
      "text": "login(email: string, password: string): Promise<User | null> // Authenticate user...",
      "contentHash": "a3f8c21d...",
      "tf": { "login": 1, "email": 1, "authenticate": 1, "user": 2 }
    },
    "docs/design/05-indexing.md#symbol-map": {
      "file": "docs/design/05-indexing.md",
      "type": "doc",
      "text": "Symbol Map — extracted exports stored at L0 and L1...",
      "contentHash": "b9d1e47c...",
      "tf": { "symbol": 3, "map": 2, "extracted": 1 }
    }
  }
}
```

### `.sensei/embeddings.json`

Vector embeddings per chunk. Stored separately so chunks can be read without loading all vectors.

```json
{
  "version": 1,
  "model": "Xenova/all-MiniLM-L6-v2",
  "dimensions": 384,
  "vectors": {
    "src/auth.ts:login": [0.12, -0.34, 0.08, "...384 floats total"]
  }
}
```

**Size estimate:** 6,000 chunks × 384 floats × 4 bytes ≈ 9MB. Acceptable for `.sensei/`.

---

## Incremental Update

`contentHash` (sha256 of chunk text) gates re-embedding:

```
For each changed file (from git diff or mtime fallback):
  1. Re-extract chunks from file
  2. For each chunk:
     → If contentHash unchanged: keep existing vector
     → If contentHash changed: re-embed, update vector
  3. Remove chunks for deleted files
  4. Write updated chunks.json and embeddings.json
```

Unchanged chunks never re-embed — incremental runs touching 5 files add near-zero overhead.

---

## Search Algorithm

### Symbol search

Exact match on symbol name (score 1.0), then prefix match (score 0.8), then substring (score 0.5). Operates directly on `symbol-map.json` — no index needed.

### BM25

Standard BM25 (k1=1.5, b=0.75). Term frequencies stored in `chunks.json` at index time. Query tokenized (lowercase, split on non-alphanumeric). IDF computed from `corpusSize` and per-term document frequency at query time.

### Semantic search

Query embedded using the same model (`all-MiniLM-L6-v2`). Cosine similarity against all stored vectors. Top-k by similarity score.

### Unified ranking via Reciprocal Rank Fusion

```
RRF score(chunk) = Σ  1 / (60 + rank_in_layer)
                  layers where chunk appears

Final list: sort descending by RRF score, deduplicate by chunk id, return top N
```

k=60 is the standard RRF constant. No tuning required. Chunks appearing in multiple layers rank higher naturally.

---

## MCP Tool Contract

### `search`

```typescript
search(query: string, options?: {
  top?: number          // default: 10
  type?: 'all' | 'symbol' | 'fulltext' | 'semantic'  // default: 'all'
}): SearchResult[]

interface SearchResult {
  id: string            // "src/auth.ts:login"
  file: string          // "src/auth.ts"
  type: 'symbol' | 'doc'
  excerpt: string       // chunk text, max 200 chars
  score: number         // RRF score
  matchedBy: ('symbol' | 'bm25' | 'semantic')[]
}
```

**Zero-hit behaviour:** when all three layers return 0 results, fires a non-blocking background `reindexRepo()` and returns:
```
"No results found. Index may be stale — reindexing in background, retry in a moment."
```

---

## File Watcher — `sensei watch`

New CLI command. Watches the repo for file changes and runs incremental `reindexRepo()` automatically.

```
sensei watch [--repo <path>]

Behaviour:
  - Uses chokidar to watch src/, docs/, package.json (configurable)
  - Debounce: 500ms quiet period before triggering reindex
  - On change: runs reindexRepo() incrementally, prints "reindexed N files (Xms)"
  - Runs as a foreground process — Ctrl+C to stop
  - Does NOT watch .sensei/ (avoids reindex loops)
```

Typical developer flow: `sensei watch &` in background, or as a VS Code task.

---

## Error Handling

```
Model unavailable (offline / first run):
  → Return symbol + BM25 results
  → result.matchedBy omits 'semantic' — no silent failure
  → Log: "Semantic search unavailable — run sensei index to generate embeddings"

Model downloading:
  → Same as unavailable — skip semantic layer for this query

embeddings.json corrupt or missing:
  → Semantic layer skipped silently
  → Rebuilt on next reindexRepo() call

chunks.json missing:
  → BM25 skipped, symbol search still works
  → Rebuilt on next reindexRepo()

Zero results across all layers:
  → Background reindex fired (non-blocking)
  → Informational message returned immediately
```

---

## File Structure

New files:

```
packages/tools/src/tools/
  search.ts               ← search() implementation (all 3 layers + RRF merge)
  search.spec.ts          ← unit tests
  chunker.ts              ← chunk extraction from symbol-map + markdown
  chunker.spec.ts
  bm25.ts                 ← BM25 scorer (pure function, no state)
  embedder.ts             ← Transformer.js wrapper, lazy model load
  embedder.spec.ts

packages/cli/src/commands/
  watch.ts                ← sensei watch command
```

Modified files:

```
packages/tools/src/tools/reindex.ts   ← add chunk + embedding generation
packages/tools/src/index.ts           ← export search()
packages/mcp/src/index.ts             ← register search MCP tool
packages/cli/src/cli.ts               ← register watch command
```

---

## Testing Strategy

```
Unit: packages/tools/src/tools/search.spec.ts
  - symbol match: "login" → src/auth.ts:login
  - bm25: "authenticate user" → auth-related chunks
  - semantic: "verify identity" → auth-related results (mock embedder)
  - RRF: chunk in 2 layers ranks above single-layer chunk
  - zero-hit: background reindexRepo() called (mock)
  - offline model: returns symbol+bm25 only, no thrown error

Unit: packages/tools/src/tools/chunker.spec.ts
  - code file → one chunk per exported symbol
  - markdown → one chunk per H2/H3 section
  - contentHash changes when chunk text changes
  - empty file → no chunks

Unit: packages/tools/src/tools/bm25.spec.ts
  - score("login", corpus) > score("unrelated", corpus)
  - longer documents penalised correctly (b=0.75)

Unit: packages/tools/src/tools/embedder.spec.ts
  - embed() returns 384-dimension vector
  - same text → same vector (deterministic)
  - isAvailable() returns false when model not cached

E2E: index sensei repo → search("reindex repository") → reindex.ts in top 3 results
```

---

## Traceability Updates

After implementation, update `docs/traceability.yaml`:

```yaml
features:
  indexing:
    items:
      - id: multi-modal-search
        status: done    # was: planned

code:
  packages/tools/src/tools/search.ts:
    implements-design: [indexer]
    status: done
  packages/tools/src/tools/chunker.ts:
    implements-design: [indexer]
    status: done
  packages/tools/src/tools/bm25.ts:
    implements-design: [indexer]
    status: done
  packages/tools/src/tools/embedder.ts:
    implements-design: [indexer, local-model-indexer]
    status: done
  packages/cli/src/commands/watch.ts:
    implements-design: [incremental-indexer]
    status: done
```
