---
id: smart-context-delivery
type: spec
status: approved
date: 2026-03-13
---

# Smart Context Delivery — Design Spec

## Goal

Add a `context_pack` MCP tool that returns a ranked, token-budgeted set of code and doc slices relevant to a given task. Agents call it instead of reading files manually. The result is visible in the dashboard Context Pack inspector.

---

## Problem

Phase 1 gives agents `search` (substring match) and `load_context` (full file). Neither is task-aware or token-budgeted. Agents either under-load (miss relevant files) or over-load (dump entire files into context). Phase 2 adds a Rank → Slice → Assemble pipeline that selects the minimum relevant content for a task.

---

## Architecture

Two pipeline modes share the same Supabase store:

```
Indexing mode (Phase 1):  Scan → Parse → Index
Context-pack mode (new):             Rank → Slice → Assemble
```

`context_pack()` triggers the right half. `indexRepo()` feeds the left half. They never run together — Index writes, Rank/Slice/Assemble read-only.

---

## New Files

```
packages/shared/src/
  token-counter.ts          ← TokenCounter interface + 3 implementations

packages/engine/src/
  rank/
    ranking-strategy.ts     ← RankingStrategy interface, Candidate, RankContext, ScoredCandidate
    diff-first-bfs.ts       ← DiffFirstBFSStrategy
    bm25.ts                 ← BM25Strategy
    semantic.ts             ← SemanticStrategy
    chain.ts                ← RankingStrategyChain
  slice/
    ast-slicer.ts           ← ASTSlicer (code → CodeSlice[])
    section-slicer.ts       ← SectionSlicer (doc → DocSlice[])
  assemble/
    assembler.ts            ← Assembler (budget gate, dedup, ContextPack)
  adapters/markdown.ts      ← MarkdownAdapter (populates doc_sections during index)
  context-pack.ts           ← buildContextPack() orchestrator

packages/server/src/tools/
  context-pack.ts           ← context_pack MCP tool
  recommend-next.ts         ← recommend_next MCP tool
  token-stats.ts            ← token_stats MCP tool

supabase/migrations/
  20260313000001_phase2_context_pack.sql

apps/dashboard/src/routes/
  repos/[id]/context-packs/+page.server.ts
  repos/[id]/context-packs/+page.svelte
```

---

## Supabase Migration

```sql
-- pgvector for semantic search
create extension if not exists vector;

-- Embeddings: one row per file, embedding via nomic-embed-text (768-dim)
-- Embedding model: nomic-embed-text (pull with: ollama pull nomic-embed-text)
create table if not exists sensei.embeddings (
  repo_id    uuid not null references sensei.repos(id) on delete cascade,
  file_path  text not null,
  chunk_text text not null,
  embedding  vector(768) not null,
  updated_at timestamptz not null default now(),
  primary key (repo_id, file_path)
);
create index on sensei.embeddings using ivfflat (embedding vector_cosine_ops);

-- BM25 full-text search column on symbols
alter table sensei.symbols
  add column if not exists search_vec tsvector generated always as (
    to_tsvector('english',
      coalesce(name, '') || ' ' ||
      coalesce(signature, '') || ' ' ||
      coalesce(docstring, ''))
  ) stored;
create index if not exists idx_symbols_search on sensei.symbols using gin(search_vec);

-- Doc sections: populated by MarkdownAdapter during indexing
create table if not exists sensei.doc_sections (
  repo_id    uuid not null references sensei.repos(id) on delete cascade,
  file_path  text not null,
  heading    text not null,
  level      integer not null,
  start_line integer not null,
  end_line   integer not null,
  content    text not null,
  code_refs  text[] not null default '{}',
  primary key (repo_id, file_path, start_line)
);
create index if not exists idx_doc_sections_repo on sensei.doc_sections(repo_id, file_path);

-- Context packs: persisted results for dashboard inspector
create table if not exists sensei.context_packs (
  id           uuid primary key default gen_random_uuid(),
  repo_id      uuid not null references sensei.repos(id) on delete cascade,
  session_id   text,
  task         text not null,
  model_id     text,
  slices       jsonb not null default '[]',
  total_tokens integer not null default 0,
  created_at   timestamptz not null default now()
);
create index if not exists idx_context_packs_repo_session
  on sensei.context_packs(repo_id, session_id);

-- Grants (same pattern as Phase 1)
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
```

---

## Token Counter

`packages/shared/src/token-counter.ts`:

```typescript
interface TokenCounter {
  readonly name: string;
  count(text: string): number;
}
```

Three implementations:
- `AnthropicTokenCounter` — uses `tiktoken` (`cl100k_base`, local, no API call, ~95% accuracy for Claude)
- `OpenAITokenCounter` — uses `tiktoken` (`cl100k_base`, local, no API call)
- `EstimateTokenCounter` — `Math.ceil(text.length / 4)` (zero dependencies, fallback)

Single dependency: `tiktoken`. (`@anthropic-ai/tokenizer` is not a published npm package.)

Factory auto-selects by `modelId`:
```typescript
function createTokenCounter(modelId?: string): TokenCounter {
  if (!modelId) return new EstimateTokenCounter();
  if (modelId.startsWith("claude-")) return new AnthropicTokenCounter();
  if (modelId.startsWith("gpt-") || modelId.startsWith("o1-") || modelId.startsWith("o3-"))
    return new OpenAITokenCounter();
  return new EstimateTokenCounter();
}
```

No API keys required — all tokenizers run locally.

---

## Rank Stage

**Types** — defined in `packages/engine/src/rank/ranking-strategy.ts`, exported via `packages/engine/src/index.ts`. `Candidate`, `RankContext`, `ScoredCandidate`, and `RankingStrategy` live in `engine` (not `shared`) since they are engine-internal concerns.

```typescript
interface Candidate { filePath: string; type: "code" | "doc"; }

interface RankContext {
  task: string;
  repoId: string;
  changedFiles: string[];     // files with recent indexed_at in scan_state
  db: SupabaseClient;
  backend: ModelBackend;      // injected by caller (OllamaBackend from server layer)
  modelId?: string;
}

interface ScoredCandidate extends Candidate {
  score: number;
  strategyScores: Record<string, number>;
}

interface RankingStrategy {
  readonly name: string;
  rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]>;
}
```

`ModelBackend` is imported from `@sensei/shared` (already defined in `packages/shared/src/types.ts`). The caller (`buildContextPack()` in `context-pack.ts`) receives a `ModelBackend` instance and passes it via `RankContext`. `SemanticStrategy` calls `ctx.backend.embed(task)` — no direct import of `OllamaBackend`.

**`CodeSlice.score` and `DocSlice.score`** are inherited from the parent `ScoredCandidate`. The orchestrator passes the `ScoredCandidate.score` to each slice produced by that candidate's slicer.

**`DiffFirstBFSStrategy`** — seeds from `changedFiles`, traverses `sensei.imports` via SQL recursive CTE. Changed files → score 2.0; each BFS hop multiplies by 0.65, max depth 4. Returns `[]` if no changed files (falls through to next strategy).

```sql
WITH RECURSIVE bfs AS (
  SELECT source_file AS file, 2.0 AS score, 0 AS depth
  FROM sensei.imports
  WHERE repo_id = $1 AND source_file = ANY($changedFiles)
  UNION ALL
  SELECT i.target_path, bfs.score * 0.65, bfs.depth + 1
  FROM sensei.imports i
  JOIN bfs ON i.source_file = bfs.file
  WHERE bfs.depth < 4
)
SELECT file, MAX(score) AS score FROM bfs GROUP BY file
```

**`BM25Strategy`** — full-text search via `to_tsquery` against `sensei.symbols.search_vec`. Also queries `sensei.doc_sections` content. Groups by file, takes max `ts_rank` per file.

**`SemanticStrategy`** — embeds `task` string via `OllamaBackend.embed()`, queries `sensei.embeddings` using pgvector `<=>` operator. Returns `[]` gracefully if Ollama unavailable.

**`RankingStrategyChain`** — runs all three strategies concurrently via `Promise.all`, merges scores using `max` per file (not sum — avoids penalizing files absent from changed set), returns top 20 sorted descending.

---

## Slice Stage

**`ASTSlicer`** (`slice/ast-slicer.ts`) — for `"code"` candidates: reads file from disk, queries `sensei.symbols` for all symbols in that file, returns one `CodeSlice` per symbol:

```typescript
interface CodeSlice {
  kind: "code";
  filePath: string;
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  symbolName: string;
  score: number;
}
```

**`SectionSlicer`** (`slice/section-slicer.ts`) — for `"doc"` candidates: queries `sensei.doc_sections` for all sections in that file, returns one `DocSlice` per section:

```typescript
interface DocSlice {
  kind: "doc";
  filePath: string;
  heading: string;
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  score: number;
}
```

Both use `TokenCounter` to populate `tokens`.

---

## Assemble Stage

`Assembler` iterates slices in score order, accumulates `TokenCounter.count(slice.content)`, stops when `totalTokens >= maxTokens` (default 8000). Deduplicates against `sessionContext` (file paths already loaded in agent session). Returns `ContextPack`.

```typescript
interface ContextPack {
  id: string;
  task: string;
  slices: Array<CodeSlice | DocSlice>;
  totalTokens: number;
  modelId?: string;
  createdAt: string;
}
```

---

## `buildContextPack()` Orchestrator

`packages/engine/src/context-pack.ts`:

```typescript
async function buildContextPack(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,          // absolute path on disk — needed by ASTSlicer to read files
  task: string,
  opts: {
    backend: ModelBackend;   // required — injected from MCP server layer; SemanticStrategy uses it
    maxTokens?: number;
    modelId?: string;
    sessionId?: string;
    sessionContext?: string[];
  }
): Promise<ContextPack>
```

Steps:
1. Load all `(file_path, language)` from `sensei.scan_state` as `Candidate[]` (type: "code" for `.ts/.js`, "doc" for `.md`)
2. Load `changedFiles` — scan_state rows with `indexed_at` within last 24h
3. `RankingStrategyChain.rank(candidates, ctx)` → top 20 `ScoredCandidate[]`
4. Run `ASTSlicer` on "code" candidates, `SectionSlicer` on "doc" candidates → flat `Slice[]`
5. `Assembler.assemble(slices, { maxTokens, sessionContext, counter })` → `ContextPack`
6. Persist to `sensei.context_packs`
7. Return

---

## Markdown Adapter

`packages/engine/src/adapters/markdown.ts` — populates `sensei.doc_sections` during `indexRepo()`. Simple H2/H3 section splitter (no tree-sitter). Instantiated directly in `pipeline.ts` alongside `TypeScriptAdapter` (no adapter registry — same pattern as existing code). Runs on `.md` files only.

Output per section: `{ heading, level, startLine, endLine, content, codeRefs }` where `codeRefs` are backtick-delimited identifiers found in the section text.

---

## MCP Tools

### `context_pack`

```typescript
// Input
{
  task: string;
  maxTokens?: number;      // default 8000
  modelId?: string;        // e.g. "claude-sonnet-4-6" — selects token counter
  sessionId?: string;      // for persisting + token_stats
  sessionContext?: string[]; // file paths already in agent context — deduped
}
// Output: ContextPack (id, task, slices[], totalTokens, createdAt)
```

### `recommend_next`

Runs Rank stage only (no slicing). Returns top-3 files with scores and a suggested `maxTokens` budget (sum of top-3 file symbol token estimates × 1.5 headroom).

```typescript
// Input: { task: string; modelId?: string }
// Output: { recommendations: [{filePath, score, symbolCount, estimatedTokens}], suggestedBudget: number }
```

### `token_stats`

```typescript
// Input: { sessionId: string }
// Output: { totalPacks: number; totalTokensServed: number; avgPackSize: number; packs: [{id, task, totalTokens, createdAt}] }
```

---

## Dashboard — Context Pack Inspector

Route: `/repos/[id]/context-packs`

Server load: queries `sensei.context_packs` for the repo ordered by `created_at desc`.

UI (Rokkit components, matching existing dashboard patterns):
- `List` of context packs — task text, total tokens, timestamp, session_id
- Expanding a pack shows a `Table` of slices: file path, line range, token count, score, kind (code/doc)
- Token ratio displayed as text (e.g. "3,241 / 8,000 tokens") — no interactive controls

---

## MarkdownAdapter Integration with Indexer

`indexRepo()` in `pipeline.ts` is updated to:
1. Run `MarkdownAdapter` on `.md` files — upsert `doc_sections`
2. After `TypeScriptAdapter` indexing — generate embeddings via `backend.embed()` for each file's `(name + signature)` concatenation and upsert to `sensei.embeddings`

Embedding model: `nomic-embed-text` (768-dim), configured via `IndexConfig.embeddingModel` in `.sensei/config.yaml`. `OllamaBackend` uses this model only for embedding calls, not for generation. `indexRepo()` receives the `ModelBackend` instance from its caller.

Embeddings generation is best-effort: if Ollama unavailable, `embeddings` table stays empty and `SemanticStrategy` returns `[]`.

> **Note:** `doc_sections` and `embeddings` are only populated after re-running `indexRepo()` post-Phase 2 deployment. The acceptance criterion (≥3 slices) will be met by code slices alone on first run.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| correctness | `Assembler` must never exceed `maxTokens` (unit-testable with a fixed token counter) |
| graceful degradation | `SemanticStrategy` returns `[]` if Ollama unavailable; `SectionSlicer` returns `[]` if `doc_sections` empty; system continues with remaining strategies/slicers |

**Architectural constraint (not unit-testable):** `buildContextPack()` must not accumulate full file content in `engine` or `server` memory — `ASTSlicer` reads only the line range for each symbol, not the full file.

**Note:** `ivfflat` index requires ~1000+ rows to outperform exact scan. For small repos, PostgreSQL falls back to exact scan automatically — no application-layer handling needed.

---

## Done When

`context_pack({ task: "fix auth middleware" })` returns a ContextPack with at least 3 slices, totalTokens ≤ 8000 → result visible in dashboard Context Pack inspector at `/repos/[id]/context-packs`.
