# Phase 2: Smart Context Delivery — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `context_pack` MCP tool that returns a ranked, token-budgeted set of code and doc slices relevant to a task description, visible in the dashboard Context Pack inspector.

**Architecture:** The existing `indexRepo()` pipeline (Scan → Parse → Index) is extended to also run `MarkdownAdapter` and generate embeddings. A new read-only pipeline (Rank → Slice → Assemble) is added to `packages/engine` and exposed via three new MCP tools. `buildContextPack()` orchestrates the right half. The MCP server layer constructs and injects `OllamaBackend` as the `ModelBackend` for embedding.

**Tech Stack:** PostgreSQL pgvector (embeddings), pg tsvector (BM25), SQL recursive CTE (DiffFirstBFS), tiktoken (token counting), OllamaBackend with `nomic-embed-text` model, Supabase JS client RPC, Rokkit UI (dashboard)

> **Prerequisites before running the acceptance test:**
> - `ollama pull nomic-embed-text` — required for semantic ranking
> - `supabase db push` — apply Phase 2 migration
> - Re-run `sensei init` (or `indexRepo()`) on the target repo to populate `doc_sections` and `embeddings`

---

## File Structure

**New files:**
```
supabase/migrations/20260313000001_phase2_context_pack.sql
packages/shared/src/token-counter.ts
packages/shared/src/token-counter.spec.ts
packages/engine/src/rank/ranking-strategy.ts
packages/engine/src/rank/diff-first-bfs.ts
packages/engine/src/rank/diff-first-bfs.spec.ts
packages/engine/src/rank/bm25.ts
packages/engine/src/rank/bm25.spec.ts
packages/engine/src/rank/semantic.ts
packages/engine/src/rank/semantic.spec.ts
packages/engine/src/rank/chain.ts
packages/engine/src/rank/chain.spec.ts
packages/engine/src/slice/ast-slicer.ts
packages/engine/src/slice/ast-slicer.spec.ts
packages/engine/src/slice/section-slicer.ts
packages/engine/src/slice/section-slicer.spec.ts
packages/engine/src/assemble/assembler.ts
packages/engine/src/assemble/assembler.spec.ts
packages/engine/src/adapters/markdown.ts
packages/engine/src/adapters/markdown.spec.ts
packages/engine/src/context-pack.ts
packages/engine/src/context-pack.spec.ts
packages/server/src/tools/context-pack.ts
packages/server/src/tools/recommend-next.ts
packages/server/src/tools/token-stats.ts
apps/dashboard/src/routes/repos/[id]/context-packs/+page.server.ts
apps/dashboard/src/routes/repos/[id]/context-packs/+page.svelte
```

**Modified files:**
```
packages/shared/src/domain.ts          — add CodeSlice, DocSlice, ContextPack, Candidate, ScoredCandidate
packages/shared/src/index.ts           — export token-counter
packages/shared/package.json           — add tiktoken dependency
packages/server/src/model/ollama-backend.ts  — add embeddingModel option
packages/engine/src/pipeline.ts        — add MarkdownAdapter + embedding generation
packages/engine/src/index.ts           — export rank/slice/assemble/context-pack
packages/server/src/mcp-server.ts      — register 3 new tools
apps/dashboard/src/routes/repos/[id]/+page.svelte  — add link to context packs
```

---

## Chunk 1: Foundation

### Task 1: Phase 2 Supabase Migration

**Files:**
- Create: `supabase/migrations/20260313000001_phase2_context_pack.sql`

- [ ] **Step 1: Create the migration file**

```sql
-- supabase/migrations/20260313000001_phase2_context_pack.sql

-- pgvector extension for semantic search
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
create index if not exists idx_embeddings_vector
  on sensei.embeddings using ivfflat (embedding vector_cosine_ops)
  with (lists = 100);

-- BM25 full-text search: add generated tsvector column to symbols
alter table sensei.symbols
  add column if not exists search_vec tsvector generated always as (
    to_tsvector('english',
      coalesce(name, '') || ' ' ||
      coalesce(signature, '') || ' ' ||
      coalesce(docstring, ''))
  ) stored;
create index if not exists idx_symbols_search
  on sensei.symbols using gin(search_vec);

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
create index if not exists idx_doc_sections_repo
  on sensei.doc_sections(repo_id, file_path);

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

-- SQL helper: BFS rank from changed files through import graph
create or replace function sensei.rank_bfs(
  p_repo_id uuid,
  p_changed_files text[]
)
returns table(file_path text, score double precision)
language sql
security definer
as $$
  WITH RECURSIVE bfs AS (
    SELECT p_cf AS file, 2.0::double precision AS score, 0 AS depth
    FROM unnest(p_changed_files) AS p_cf
    UNION ALL
    SELECT i.target_path, (bfs.score * 0.65)::double precision, bfs.depth + 1
    FROM sensei.imports i
    JOIN bfs ON i.source_file = bfs.file
    WHERE i.repo_id = p_repo_id
      AND bfs.depth < 4
  )
  SELECT file AS file_path, MAX(score)::double precision AS score
  FROM bfs
  GROUP BY file
$$;

-- SQL helper: BM25 rank via tsvector full-text search
create or replace function sensei.rank_bm25(
  p_repo_id uuid,
  p_query text
)
returns table(file_path text, score double precision)
language sql
security definer
as $$
  SELECT file_path, MAX(ts_rank(search_vec, plainto_tsquery('english', p_query)))::double precision AS score
  FROM sensei.symbols
  WHERE repo_id = p_repo_id
    AND search_vec @@ plainto_tsquery('english', p_query)
  GROUP BY file_path
  ORDER BY score DESC
  LIMIT 20
$$;

-- SQL helper: semantic similarity via pgvector
create or replace function sensei.match_embeddings(
  p_repo_id uuid,
  query_embedding vector(768),
  match_threshold double precision,
  match_count int
)
returns table(file_path text, similarity double precision)
language sql
security definer
as $$
  SELECT file_path, (1 - (embedding <=> query_embedding))::double precision AS similarity
  FROM sensei.embeddings
  WHERE repo_id = p_repo_id
    AND 1 - (embedding <=> query_embedding) > match_threshold
  ORDER BY embedding <=> query_embedding
  LIMIT match_count
$$;

-- Grants (same pattern as Phase 1)
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
```

- [ ] **Step 2: Apply migration to local Supabase**

```bash
cd /Users/Jerry/Developer/sensei
supabase db push --local
```

Expected: migration applies without errors.

- [ ] **Step 3: Verify tables and column were created**

```bash
supabase db diff --local
```

Expected: diff shows no pending changes (migration fully applied).

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260313000001_phase2_context_pack.sql
git commit -m "feat(db): Phase 2 migration — embeddings, doc_sections, context_packs, BFS/BM25/semantic RPCs"
```

---

### Task 2: OllamaBackend embeddingModel support

**Files:**
- Modify: `packages/server/src/model/ollama-backend.ts`
- Modify: `packages/server/src/model/ollama-backend.spec.ts`

- [ ] **Step 1: Write the failing test**

Add to `packages/server/src/model/ollama-backend.spec.ts`:

```typescript
it("embed() uses embeddingModel when set separately from generation model", async () => {
  const fetches: Array<{ model: string }> = [];
  global.fetch = vi.fn(async (_url: string, opts: any) => {
    const body = JSON.parse(opts.body);
    fetches.push({ model: body.model });
    return { ok: true, json: async () => ({ embedding: [0.1, 0.2] }) } as any;
  });

  const backend = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });
  await backend.embed("hello");

  expect(fetches[0].model).toBe("nomic-embed-text");
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/server && bunx vitest run src/model/ollama-backend.spec.ts
```

Expected: FAIL — `embeddingModel` option is not read.

- [ ] **Step 3: Add embeddingModel to OllamaBackend**

In `packages/server/src/model/ollama-backend.ts`, update the interface and class:

```typescript
export interface OllamaOptions {
  model?: string;
  baseUrl?: string;
  embeddingModel?: string;  // separate model for embed() calls
}

// In the class:
private model: string;
private embeddingModel: string;
private baseUrl: string;

constructor(opts: OllamaOptions = {}) {
  this.model = opts.model ?? "llama3.2:3b";
  this.embeddingModel = opts.embeddingModel ?? this.model;
  this.baseUrl = opts.baseUrl ?? "http://127.0.0.1:11434";
}

// In embed():
body: JSON.stringify({ model: this.embeddingModel, prompt: text }),
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd packages/server && bunx vitest run src/model/ollama-backend.spec.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/server/src/model/ollama-backend.ts packages/server/src/model/ollama-backend.spec.ts
git commit -m "feat(server): add embeddingModel option to OllamaBackend"
```

---

### Task 3: TokenCounter interface + implementations

**Files:**
- Create: `packages/shared/src/token-counter.ts`
- Create: `packages/shared/src/token-counter.spec.ts`
- Modify: `packages/shared/src/index.ts`
- Modify: `packages/shared/package.json`

- [ ] **Step 1: Add tiktoken dependency**

```bash
cd packages/shared && bun add tiktoken
```

- [ ] **Step 2: Write the failing tests**

Create `packages/shared/src/token-counter.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { createTokenCounter, EstimateTokenCounter, OpenAITokenCounter, AnthropicTokenCounter } from "./token-counter.js";

describe("EstimateTokenCounter", () => {
  it("estimates tokens as ceil(length / 4)", () => {
    const counter = new EstimateTokenCounter();
    expect(counter.count("hello")).toBe(Math.ceil("hello".length / 4));
    expect(counter.count("")).toBe(0);
  });

  it("name is estimate", () => {
    expect(new EstimateTokenCounter().name).toBe("estimate");
  });
});

describe("OpenAITokenCounter", () => {
  it("counts tokens for a known string", () => {
    const counter = new OpenAITokenCounter();
    const count = counter.count("hello world");
    expect(count).toBeGreaterThan(0);
    expect(count).toBeLessThan(10);
  });

  it("name is openai", () => {
    expect(new OpenAITokenCounter().name).toBe("openai");
  });
});

describe("AnthropicTokenCounter", () => {
  it("counts tokens for a known string", () => {
    const counter = new AnthropicTokenCounter();
    const count = counter.count("hello world");
    expect(count).toBeGreaterThan(0);
    expect(count).toBeLessThan(10);
  });

  it("name is anthropic", () => {
    expect(new AnthropicTokenCounter().name).toBe("anthropic");
  });
});

describe("createTokenCounter", () => {
  it("returns EstimateTokenCounter when no modelId", () => {
    expect(createTokenCounter().name).toBe("estimate");
  });

  it("returns AnthropicTokenCounter for claude- prefix", () => {
    expect(createTokenCounter("claude-sonnet-4-6").name).toBe("anthropic");
  });

  it("returns OpenAITokenCounter for gpt- prefix", () => {
    expect(createTokenCounter("gpt-4o").name).toBe("openai");
  });

  it("returns OpenAITokenCounter for o1- prefix", () => {
    expect(createTokenCounter("o1-mini").name).toBe("openai");
  });

  it("returns EstimateTokenCounter for unknown modelId", () => {
    expect(createTokenCounter("unknown-model-xyz").name).toBe("estimate");
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd packages/shared && bunx vitest run src/token-counter.spec.ts 2>/dev/null || echo "FAIL expected"
```

Expected: FAIL — module not found.

- [ ] **Step 4: Implement TokenCounter**

Create `packages/shared/src/token-counter.ts`:

```typescript
import { get_encoding, type Tiktoken } from "tiktoken";

export interface TokenCounter {
  readonly name: string;
  count(text: string): number;
}

export class EstimateTokenCounter implements TokenCounter {
  readonly name = "estimate";
  count(text: string): number {
    return Math.ceil(text.length / 4);
  }
}

export class OpenAITokenCounter implements TokenCounter {
  readonly name = "openai";
  private enc: Tiktoken | null = null;

  count(text: string): number {
    if (!this.enc) this.enc = get_encoding("cl100k_base");
    return this.enc.encode(text).length;
  }
}

export class AnthropicTokenCounter implements TokenCounter {
  readonly name = "anthropic";
  private enc: Tiktoken | null = null;

  count(text: string): number {
    // Claude uses a BPE tokenizer compatible with cl100k_base (~95% accuracy)
    if (!this.enc) this.enc = get_encoding("cl100k_base");
    return this.enc.encode(text).length;
  }
}

export function createTokenCounter(modelId?: string): TokenCounter {
  if (!modelId) return new EstimateTokenCounter();
  if (modelId.startsWith("claude-")) return new AnthropicTokenCounter();
  if (modelId.startsWith("gpt-") || modelId.startsWith("o1-") || modelId.startsWith("o3-")) {
    return new OpenAITokenCounter();
  }
  return new EstimateTokenCounter();
}
```

- [ ] **Step 5: Export from shared index**

In `packages/shared/src/index.ts`, add:

```typescript
export * from "./token-counter.js";
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cd packages/shared && bunx vitest run src/token-counter.spec.ts
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add packages/shared/src/token-counter.ts packages/shared/src/token-counter.spec.ts \
        packages/shared/src/index.ts packages/shared/package.json bun.lock
git commit -m "feat(shared): add TokenCounter interface + Estimate/OpenAI/Anthropic implementations"
```

---

## Chunk 2: Rank Stage

### Task 4: RankingStrategy types + DiffFirstBFSStrategy

**Files:**
- Create: `packages/engine/src/rank/ranking-strategy.ts`
- Create: `packages/engine/src/rank/diff-first-bfs.ts`
- Create: `packages/engine/src/rank/diff-first-bfs.spec.ts`

- [ ] **Step 1: Create the types file**

Create `packages/engine/src/rank/ranking-strategy.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";

export interface Candidate {
  filePath: string;
  type: "code" | "doc";
}

export interface RankContext {
  task: string;
  repoId: string;
  changedFiles: string[];     // repo-relative paths changed in last 24h
  db: SupabaseClient;
  backend: ModelBackend;      // injected — used by SemanticStrategy for embed()
  modelId?: string;
}

export interface ScoredCandidate extends Candidate {
  score: number;
  strategyScores: Record<string, number>;
}

export interface RankingStrategy {
  readonly name: string;
  rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]>;
}
```

- [ ] **Step 2: Write the failing test for DiffFirstBFSStrategy**

Create `packages/engine/src/rank/diff-first-bfs.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { DiffFirstBFSStrategy } from "./diff-first-bfs.js";
import type { Candidate, RankContext } from "./ranking-strategy.js";

function makeCtx(changedFiles: string[], rpcResult: Array<{file_path: string; score: number}>): RankContext {
  return {
    task: "fix auth",
    repoId: "repo-1",
    changedFiles,
    db: {
      rpc: vi.fn().mockResolvedValue({ data: rpcResult, error: null }),
    } as any,
    backend: { embed: vi.fn().mockResolvedValue([]) } as any,
  };
}

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
];

describe("DiffFirstBFSStrategy", () => {
  it("returns [] when no changed files", async () => {
    const ctx = makeCtx([], []);
    const strategy = new DiffFirstBFSStrategy();
    const result = await strategy.rank(candidates, ctx);
    expect(result).toHaveLength(0);
  });

  it("returns scored candidates from RPC result", async () => {
    const ctx = makeCtx(["src/auth.ts"], [
      { file_path: "src/auth.ts", score: 2.0 },
      { file_path: "src/utils.ts", score: 1.3 },
    ]);
    const strategy = new DiffFirstBFSStrategy();
    const result = await strategy.rank(candidates, ctx);

    expect(result).toHaveLength(2);
    const auth = result.find(r => r.filePath === "src/auth.ts");
    expect(auth?.score).toBe(2.0);
    expect(auth?.strategyScores["diff_first_bfs"]).toBe(2.0);
  });

  it("only returns candidates present in input list", async () => {
    const ctx = makeCtx(["src/auth.ts"], [
      { file_path: "src/auth.ts", score: 2.0 },
      { file_path: "src/unknown.ts", score: 1.5 }, // not in candidates list
    ]);
    const strategy = new DiffFirstBFSStrategy();
    const result = await strategy.rank(candidates, ctx);

    expect(result.every(r => candidates.some(c => c.filePath === r.filePath))).toBe(true);
  });

  it("calls rpc rank_bfs with correct params", async () => {
    const ctx = makeCtx(["src/auth.ts"], []);
    const strategy = new DiffFirstBFSStrategy();
    await strategy.rank(candidates, ctx);

    expect(ctx.db.rpc).toHaveBeenCalledWith("rank_bfs", {
      p_repo_id: "repo-1",
      p_changed_files: ["src/auth.ts"],
    });
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd packages/engine && bunx vitest run src/rank/diff-first-bfs.spec.ts
```

Expected: FAIL — module not found.

- [ ] **Step 4: Implement DiffFirstBFSStrategy**

Create `packages/engine/src/rank/diff-first-bfs.ts`:

```typescript
import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class DiffFirstBFSStrategy implements RankingStrategy {
  readonly name = "diff_first_bfs";

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    if (ctx.changedFiles.length === 0) return [];

    const { data, error } = await ctx.db.rpc("rank_bfs", {
      p_repo_id: ctx.repoId,
      p_changed_files: ctx.changedFiles,
    });

    if (error || !data) return [];

    const candidateSet = new Set(candidates.map(c => c.filePath));
    const scoreMap = new Map<string, number>(
      (data as Array<{ file_path: string; score: number }>).map(r => [r.file_path, r.score])
    );

    return candidates
      .filter(c => scoreMap.has(c.filePath) && candidateSet.has(c.filePath))
      .map(c => ({
        ...c,
        score: scoreMap.get(c.filePath)!,
        strategyScores: { [this.name]: scoreMap.get(c.filePath)! },
      }));
  }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd packages/engine && bunx vitest run src/rank/diff-first-bfs.spec.ts
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/rank/
git commit -m "feat(engine): add RankingStrategy types and DiffFirstBFSStrategy"
```

---

### Task 5: BM25Strategy

**Files:**
- Create: `packages/engine/src/rank/bm25.ts`
- Create: `packages/engine/src/rank/bm25.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/rank/bm25.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { BM25Strategy } from "./bm25.js";
import type { Candidate, RankContext } from "./ranking-strategy.js";

function makeCtx(rpcResult: Array<{file_path: string; score: number}>): RankContext {
  return {
    task: "fix auth middleware",
    repoId: "repo-1",
    changedFiles: [],
    db: {
      rpc: vi.fn().mockResolvedValue({ data: rpcResult, error: null }),
    } as any,
    backend: { embed: vi.fn().mockResolvedValue([]) } as any,
  };
}

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
];

describe("BM25Strategy", () => {
  it("returns scored candidates matching RPC result", async () => {
    const ctx = makeCtx([{ file_path: "src/auth.ts", score: 0.9 }]);
    const strategy = new BM25Strategy();
    const result = await strategy.rank(candidates, ctx);

    expect(result).toHaveLength(1);
    expect(result[0].filePath).toBe("src/auth.ts");
    expect(result[0].score).toBe(0.9);
    expect(result[0].strategyScores["bm25"]).toBe(0.9);
  });

  it("returns [] when RPC returns no matches", async () => {
    const ctx = makeCtx([]);
    const strategy = new BM25Strategy();
    expect(await strategy.rank(candidates, ctx)).toHaveLength(0);
  });

  it("returns [] when RPC errors", async () => {
    const ctx = makeCtx([]);
    (ctx.db.rpc as any).mockResolvedValue({ data: null, error: { message: "query syntax error" } });
    const strategy = new BM25Strategy();
    expect(await strategy.rank(candidates, ctx)).toHaveLength(0);
  });

  it("calls rpc rank_bm25 with correct params", async () => {
    const ctx = makeCtx([]);
    await new BM25Strategy().rank(candidates, ctx);
    expect(ctx.db.rpc).toHaveBeenCalledWith("rank_bm25", { p_repo_id: "repo-1", p_query: "fix auth middleware" });
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/rank/bm25.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement BM25Strategy**

Create `packages/engine/src/rank/bm25.ts`:

```typescript
import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class BM25Strategy implements RankingStrategy {
  readonly name = "bm25";

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    const { data, error } = await ctx.db.rpc("rank_bm25", {
      p_repo_id: ctx.repoId,
      p_query: ctx.task,
    });

    if (error || !data) return [];

    const candidateSet = new Set(candidates.map(c => c.filePath));
    const scoreMap = new Map<string, number>(
      (data as Array<{ file_path: string; score: number }>).map(r => [r.file_path, r.score])
    );

    return candidates
      .filter(c => scoreMap.has(c.filePath) && candidateSet.has(c.filePath))
      .map(c => ({
        ...c,
        score: scoreMap.get(c.filePath)!,
        strategyScores: { [this.name]: scoreMap.get(c.filePath)! },
      }));
  }
}
```

- [ ] **Step 4: Run to verify pass**

```bash
cd packages/engine && bunx vitest run src/rank/bm25.spec.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/rank/bm25.ts packages/engine/src/rank/bm25.spec.ts
git commit -m "feat(engine): add BM25Strategy"
```

---

### Task 6: SemanticStrategy

**Files:**
- Create: `packages/engine/src/rank/semantic.ts`
- Create: `packages/engine/src/rank/semantic.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/rank/semantic.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { SemanticStrategy } from "./semantic.js";
import type { Candidate, RankContext } from "./ranking-strategy.js";

function makeCtx(
  embedding: number[],
  rpcResult: Array<{file_path: string; similarity: number}>
): RankContext {
  return {
    task: "fix auth middleware",
    repoId: "repo-1",
    changedFiles: [],
    db: {
      rpc: vi.fn().mockResolvedValue({ data: rpcResult, error: null }),
    } as any,
    backend: { embed: vi.fn().mockResolvedValue(embedding) } as any,
  };
}

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
];

describe("SemanticStrategy", () => {
  it("returns [] when backend.embed returns empty array (Ollama unavailable)", async () => {
    const ctx = makeCtx([], []);
    expect(await new SemanticStrategy().rank(candidates, ctx)).toHaveLength(0);
  });

  it("returns scored candidates from RPC when embedding available", async () => {
    const embedding = new Array(768).fill(0.1);
    const ctx = makeCtx(embedding, [{ file_path: "src/auth.ts", similarity: 0.85 }]);
    const result = await new SemanticStrategy().rank(candidates, ctx);

    expect(result).toHaveLength(1);
    expect(result[0].filePath).toBe("src/auth.ts");
    expect(result[0].score).toBe(0.85);
    expect(result[0].strategyScores["semantic"]).toBe(0.85);
  });

  it("calls backend.embed with the task string", async () => {
    const embedding = new Array(768).fill(0.1);
    const ctx = makeCtx(embedding, []);
    await new SemanticStrategy().rank(candidates, ctx);
    expect(ctx.backend.embed).toHaveBeenCalledWith("fix auth middleware");
  });

  it("calls rpc match_embeddings with correct params", async () => {
    const embedding = new Array(768).fill(0.1);
    const ctx = makeCtx(embedding, []);
    await new SemanticStrategy().rank(candidates, ctx);
    expect(ctx.db.rpc).toHaveBeenCalledWith("match_embeddings", {
      p_repo_id: "repo-1",
      query_embedding: embedding,
      match_threshold: 0.3,
      match_count: 20,
    });
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/rank/semantic.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement SemanticStrategy**

Create `packages/engine/src/rank/semantic.ts`:

```typescript
import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class SemanticStrategy implements RankingStrategy {
  readonly name = "semantic";

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    const embedding = await ctx.backend.embed(ctx.task);
    if (embedding.length === 0) return [];  // Ollama unavailable — graceful fallback

    const { data, error } = await ctx.db.rpc("match_embeddings", {
      p_repo_id: ctx.repoId,
      query_embedding: embedding,
      match_threshold: 0.3,
      match_count: 20,
    });

    if (error || !data) return [];

    const candidateSet = new Set(candidates.map(c => c.filePath));
    const scoreMap = new Map<string, number>(
      (data as Array<{ file_path: string; similarity: number }>).map(r => [r.file_path, r.similarity])
    );

    return candidates
      .filter(c => scoreMap.has(c.filePath) && candidateSet.has(c.filePath))
      .map(c => ({
        ...c,
        score: scoreMap.get(c.filePath)!,
        strategyScores: { [this.name]: scoreMap.get(c.filePath)! },
      }));
  }
}
```

- [ ] **Step 4: Run to verify pass**

```bash
cd packages/engine && bunx vitest run src/rank/semantic.spec.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/rank/semantic.ts packages/engine/src/rank/semantic.spec.ts
git commit -m "feat(engine): add SemanticStrategy"
```

---

### Task 7: RankingStrategyChain

**Files:**
- Create: `packages/engine/src/rank/chain.ts`
- Create: `packages/engine/src/rank/chain.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/rank/chain.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { RankingStrategyChain } from "./chain.js";
import type { Candidate, RankContext, RankingStrategy } from "./ranking-strategy.js";

function makeStrategy(name: string, scores: Record<string, number>): RankingStrategy {
  return {
    name,
    rank: vi.fn(async (candidates: Candidate[]) =>
      candidates
        .filter(c => scores[c.filePath] !== undefined)
        .map(c => ({
          ...c,
          score: scores[c.filePath],
          strategyScores: { [name]: scores[c.filePath] },
        }))
    ),
  };
}

const ctx: RankContext = {
  task: "fix auth",
  repoId: "repo-1",
  changedFiles: [],
  db: {} as any,
  backend: {} as any,
};

const candidates: Candidate[] = [
  { filePath: "src/auth.ts", type: "code" },
  { filePath: "src/utils.ts", type: "code" },
  { filePath: "src/db.ts", type: "code" },
];

describe("RankingStrategyChain", () => {
  it("merges scores using max per file", async () => {
    const s1 = makeStrategy("s1", { "src/auth.ts": 2.0, "src/utils.ts": 0.5 });
    const s2 = makeStrategy("s2", { "src/auth.ts": 0.8, "src/db.ts": 1.5 });
    const chain = new RankingStrategyChain([s1, s2]);
    const result = await chain.rank(candidates, ctx);

    expect(result.find(r => r.filePath === "src/auth.ts")?.score).toBe(2.0);  // max(2.0, 0.8)
    expect(result.find(r => r.filePath === "src/db.ts")?.score).toBe(1.5);
    expect(result.find(r => r.filePath === "src/utils.ts")?.score).toBe(0.5);
  });

  it("returns results sorted descending by score", async () => {
    const s1 = makeStrategy("s1", { "src/auth.ts": 1.0, "src/utils.ts": 0.5, "src/db.ts": 1.5 });
    const chain = new RankingStrategyChain([s1]);
    const result = await chain.rank(candidates, ctx);

    expect(result[0].filePath).toBe("src/db.ts");
    expect(result[1].filePath).toBe("src/auth.ts");
    expect(result[2].filePath).toBe("src/utils.ts");
  });

  it("limits output to top 20", async () => {
    const manyCandidates: Candidate[] = Array.from({ length: 30 }, (_, i) => ({
      filePath: `src/file${i}.ts`,
      type: "code" as const,
    }));
    const scores = Object.fromEntries(manyCandidates.map((c, i) => [c.filePath, i * 0.1]));
    const chain = new RankingStrategyChain([makeStrategy("s1", scores)]);
    const result = await chain.rank(manyCandidates, ctx);
    expect(result.length).toBeLessThanOrEqual(20);
  });

  it("runs all strategies and calls each rank() once", async () => {
    const s1 = makeStrategy("s1", {});
    const s2 = makeStrategy("s2", {});
    await new RankingStrategyChain([s1, s2]).rank(candidates, ctx);
    expect(s1.rank).toHaveBeenCalledOnce();
    expect(s2.rank).toHaveBeenCalledOnce();
  });

  it("merges strategyScores from all strategies", async () => {
    const s1 = makeStrategy("s1", { "src/auth.ts": 2.0 });
    const s2 = makeStrategy("s2", { "src/auth.ts": 0.8 });
    const chain = new RankingStrategyChain([s1, s2]);
    const result = await chain.rank(candidates, ctx);
    const auth = result.find(r => r.filePath === "src/auth.ts");
    expect(auth?.strategyScores["s1"]).toBe(2.0);
    expect(auth?.strategyScores["s2"]).toBe(0.8);
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/rank/chain.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement RankingStrategyChain**

Create `packages/engine/src/rank/chain.ts`:

```typescript
import type { Candidate, RankContext, ScoredCandidate, RankingStrategy } from "./ranking-strategy.js";

export class RankingStrategyChain {
  constructor(private strategies: RankingStrategy[]) {}

  async rank(candidates: Candidate[], ctx: RankContext): Promise<ScoredCandidate[]> {
    const allResults = await Promise.all(
      this.strategies.map(s => s.rank(candidates, ctx).catch(() => [] as ScoredCandidate[]))
    );

    // Merge: max score per file, union strategyScores
    const merged = new Map<string, ScoredCandidate>();
    for (const results of allResults) {
      for (const candidate of results) {
        const existing = merged.get(candidate.filePath);
        if (!existing) {
          merged.set(candidate.filePath, { ...candidate });
        } else {
          existing.score = Math.max(existing.score, candidate.score);
          Object.assign(existing.strategyScores, candidate.strategyScores);
        }
      }
    }

    return Array.from(merged.values())
      .sort((a, b) => b.score - a.score)
      .slice(0, 20);
  }
}
```

- [ ] **Step 4: Run all rank tests**

```bash
cd packages/engine && bunx vitest run src/rank/
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/rank/chain.ts packages/engine/src/rank/chain.spec.ts
git commit -m "feat(engine): add RankingStrategyChain"
```

---

## Chunk 3: Slice + Assemble

### Task 8: Domain types + ASTSlicer

**Files:**
- Modify: `packages/shared/src/domain.ts`
- Create: `packages/engine/src/slice/ast-slicer.ts`
- Create: `packages/engine/src/slice/ast-slicer.spec.ts`

- [ ] **Step 1: Add Phase 2 types to domain.ts**

Append to `packages/shared/src/domain.ts`:

```typescript
// ─── Phase 2 context pack types ───────────────────────────────────────────────

export interface CodeSlice {
  kind: "code";
  filePath: string;
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  symbolName: string;
  score: number;
}

export interface DocSlice {
  kind: "doc";
  filePath: string;
  heading: string;
  startLine: number;
  endLine: number;
  content: string;
  tokens: number;
  score: number;
}

export type Slice = CodeSlice | DocSlice;

export interface ContextPack {
  id: string;
  task: string;
  slices: Slice[];
  totalTokens: number;
  modelId?: string;
  createdAt: string;
}
```

- [ ] **Step 2: Write the failing test for ASTSlicer**

Create `packages/engine/src/slice/ast-slicer.spec.ts`:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { ASTSlicer } from "./ast-slicer.js";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";
import type { TokenCounter } from "@sensei/shared";

const counter: TokenCounter = { name: "estimate", count: (t) => Math.ceil(t.length / 4) };

function makeDb(symbols: Array<{name: string; line_start: number; line_end: number}>) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn().mockResolvedValue({ data: symbols, error: null }),
        })),
      })),
    })),
  } as any;
}

describe("ASTSlicer", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "ast-slicer-"));
    await mkdir(join(dir, "src"), { recursive: true });
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("returns one CodeSlice per symbol with correct content", async () => {
    const lines = ["line 1", "export function foo() {", "  return 1;", "}", "export function bar() {", "  return 2;", "}"];
    await writeFile(join(dir, "src/a.ts"), lines.join("\n"));

    const db = makeDb([
      { name: "foo", line_start: 2, line_end: 4 },
      { name: "bar", line_start: 5, line_end: 7 },
    ]);

    const candidate: ScoredCandidate = { filePath: "src/a.ts", type: "code", score: 1.5, strategyScores: {} };
    const slicer = new ASTSlicer(db, dir, "repo-1");
    const slices = await slicer.slice(candidate, counter);

    expect(slices).toHaveLength(2);
    expect(slices[0].kind).toBe("code");
    expect(slices[0].symbolName).toBe("foo");
    expect(slices[0].startLine).toBe(2);
    expect(slices[0].endLine).toBe(4);
    expect(slices[0].score).toBe(1.5);
    expect(slices[0].content).toContain("export function foo()");
    expect(slices[0].tokens).toBeGreaterThan(0);
  });

  it("returns [] when file does not exist", async () => {
    const db = makeDb([{ name: "foo", line_start: 1, line_end: 3 }]);
    const candidate: ScoredCandidate = { filePath: "src/nonexistent.ts", type: "code", score: 1.0, strategyScores: {} };
    const slicer = new ASTSlicer(db, dir, "repo-1");
    expect(await slicer.slice(candidate, counter)).toHaveLength(0);
  });

  it("returns [] when no symbols found", async () => {
    await writeFile(join(dir, "src/empty.ts"), "// no symbols");
    const db = makeDb([]);
    const candidate: ScoredCandidate = { filePath: "src/empty.ts", type: "code", score: 1.0, strategyScores: {} };
    const slicer = new ASTSlicer(db, dir, "repo-1");
    expect(await slicer.slice(candidate, counter)).toHaveLength(0);
  });
});
```

- [ ] **Step 3: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/slice/ast-slicer.spec.ts
```

Expected: FAIL.

- [ ] **Step 4: Implement ASTSlicer**

Create `packages/engine/src/slice/ast-slicer.ts`:

```typescript
import { readFile } from "fs/promises";
import { join } from "path";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { CodeSlice, TokenCounter } from "@sensei/shared";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";

export class ASTSlicer {
  constructor(
    private db: SupabaseClient,
    private repoPath: string,
    private repoId: string,
  ) {}

  async slice(candidate: ScoredCandidate, counter: TokenCounter): Promise<CodeSlice[]> {
    const { data: symbols, error } = await this.db
      .from("symbols")
      .select("name,line_start,line_end")
      .eq("repo_id", this.repoId)
      .eq("file_path", candidate.filePath);

    if (error || !symbols || symbols.length === 0) return [];

    let lines: string[];
    try {
      const content = await readFile(join(this.repoPath, candidate.filePath), "utf-8");
      lines = content.split("\n");
    } catch {
      return [];
    }

    return (symbols as Array<{ name: string; line_start: number; line_end: number }>).map(sym => {
      const content = lines.slice(sym.line_start - 1, sym.line_end).join("\n");
      return {
        kind: "code" as const,
        filePath: candidate.filePath,
        startLine: sym.line_start,
        endLine: sym.line_end,
        content,
        tokens: counter.count(content),
        symbolName: sym.name,
        score: candidate.score,
      };
    });
  }
}
```

- [ ] **Step 5: Run to verify pass**

```bash
cd packages/engine && bunx vitest run src/slice/ast-slicer.spec.ts
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add packages/shared/src/domain.ts packages/engine/src/slice/ast-slicer.ts \
        packages/engine/src/slice/ast-slicer.spec.ts
git commit -m "feat(engine): add CodeSlice/DocSlice/ContextPack types and ASTSlicer"
```

---

### Task 9: SectionSlicer

**Files:**
- Create: `packages/engine/src/slice/section-slicer.ts`
- Create: `packages/engine/src/slice/section-slicer.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/slice/section-slicer.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { SectionSlicer } from "./section-slicer.js";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";
import type { TokenCounter } from "@sensei/shared";

const counter: TokenCounter = { name: "estimate", count: (t) => Math.ceil(t.length / 4) };

function makeDb(sections: Array<{heading: string; level: number; start_line: number; end_line: number; content: string}>) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn().mockResolvedValue({ data: sections, error: null }),
        })),
      })),
    })),
  } as any;
}

describe("SectionSlicer", () => {
  it("returns one DocSlice per section", async () => {
    const sections = [
      { heading: "Overview", level: 2, start_line: 1, end_line: 5, content: "This is the overview." },
      { heading: "Usage", level: 2, start_line: 6, end_line: 10, content: "Use it like this." },
    ];
    const candidate: ScoredCandidate = { filePath: "docs/guide.md", type: "doc", score: 0.9, strategyScores: {} };
    const slicer = new SectionSlicer(makeDb(sections), "repo-1");
    const slices = await slicer.slice(candidate, counter);

    expect(slices).toHaveLength(2);
    expect(slices[0].kind).toBe("doc");
    expect(slices[0].heading).toBe("Overview");
    expect(slices[0].score).toBe(0.9);
    expect(slices[0].tokens).toBeGreaterThan(0);
  });

  it("returns [] when no sections found", async () => {
    const candidate: ScoredCandidate = { filePath: "docs/empty.md", type: "doc", score: 0.5, strategyScores: {} };
    const slicer = new SectionSlicer(makeDb([]), "repo-1");
    expect(await slicer.slice(candidate, counter)).toHaveLength(0);
  });

  it("returns [] when DB errors", async () => {
    const errorDb = {
      from: vi.fn(() => ({
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            eq: vi.fn().mockResolvedValue({ data: null, error: { message: "fail" } }),
          })),
        })),
      })),
    } as any;
    const candidate: ScoredCandidate = { filePath: "docs/guide.md", type: "doc", score: 0.5, strategyScores: {} };
    expect(await new SectionSlicer(errorDb, "repo-1").slice(candidate, counter)).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/slice/section-slicer.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement SectionSlicer**

Create `packages/engine/src/slice/section-slicer.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { DocSlice, TokenCounter } from "@sensei/shared";
import type { ScoredCandidate } from "../rank/ranking-strategy.js";

export class SectionSlicer {
  constructor(
    private db: SupabaseClient,
    private repoId: string,
  ) {}

  async slice(candidate: ScoredCandidate, counter: TokenCounter): Promise<DocSlice[]> {
    const { data: sections, error } = await this.db
      .from("doc_sections")
      .select("heading,level,start_line,end_line,content")
      .eq("repo_id", this.repoId)
      .eq("file_path", candidate.filePath);

    if (error || !sections || sections.length === 0) return [];

    return (sections as Array<{
      heading: string; level: number;
      start_line: number; end_line: number; content: string;
    }>).map(s => ({
      kind: "doc" as const,
      filePath: candidate.filePath,
      heading: s.heading,
      startLine: s.start_line,
      endLine: s.end_line,
      content: s.content,
      tokens: counter.count(s.content),
      score: candidate.score,
    }));
  }
}
```

- [ ] **Step 4: Run to verify pass**

```bash
cd packages/engine && bunx vitest run src/slice/section-slicer.spec.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/slice/section-slicer.ts packages/engine/src/slice/section-slicer.spec.ts
git commit -m "feat(engine): add SectionSlicer"
```

---

### Task 10: Assembler

**Files:**
- Create: `packages/engine/src/assemble/assembler.ts`
- Create: `packages/engine/src/assemble/assembler.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/assemble/assembler.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { Assembler } from "./assembler.js";
import type { Slice, TokenCounter } from "@sensei/shared";

// 1 char = 1 token for predictable tests
const counter: TokenCounter = { name: "estimate", count: (t) => t.length };

function makeSlice(filePath: string, content: string, score: number): Slice {
  return {
    kind: "code",
    filePath,
    startLine: 1,
    endLine: 5,
    content,
    tokens: content.length,
    symbolName: "foo",
    score,
  };
}

describe("Assembler", () => {
  it("never exceeds maxTokens", () => {
    const slices = Array.from({ length: 10 }, (_, i) => makeSlice(`file${i}.ts`, "x".repeat(50), 1.0 - i * 0.1));
    const pack = new Assembler().assemble(slices, { maxTokens: 100, counter, task: "task" });
    expect(pack.totalTokens).toBeLessThanOrEqual(100);
  });

  it("deduplicates slices from sessionContext", () => {
    const slices = [
      makeSlice("src/auth.ts", "auth code", 2.0),   // in sessionContext — skip
      makeSlice("src/utils.ts", "utils code", 1.0), // not in sessionContext — include
    ];
    const pack = new Assembler().assemble(slices, { maxTokens: 8000, counter, task: "task", sessionContext: ["src/auth.ts"] });
    expect(pack.slices).toHaveLength(1);
    expect(pack.slices[0].filePath).toBe("src/utils.ts");
  });

  it("includes slices in score order", () => {
    const slices = [makeSlice("low.ts", "low", 0.3), makeSlice("high.ts", "high", 2.0), makeSlice("mid.ts", "mid", 1.0)];
    const pack = new Assembler().assemble(slices, { maxTokens: 8000, counter, task: "task" });
    expect(pack.slices[0].filePath).toBe("high.ts");
    expect(pack.slices[1].filePath).toBe("mid.ts");
    expect(pack.slices[2].filePath).toBe("low.ts");
  });

  it("sets totalTokens to sum of included slice tokens", () => {
    const slices = [makeSlice("a.ts", "hello", 1.0), makeSlice("b.ts", "world", 0.9)];
    const pack = new Assembler().assemble(slices, { maxTokens: 8000, counter, task: "task" });
    expect(pack.totalTokens).toBe(10);  // "hello".length + "world".length
  });

  it("sets task, modelId, id, createdAt on the pack", () => {
    const pack = new Assembler().assemble([], { maxTokens: 8000, counter, task: "fix bug", modelId: "gpt-4o" });
    expect(pack.task).toBe("fix bug");
    expect(pack.modelId).toBe("gpt-4o");
    expect(pack.id).toBeTruthy();
    expect(pack.createdAt).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/assemble/assembler.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement Assembler**

Create `packages/engine/src/assemble/assembler.ts`:

```typescript
import { randomUUID } from "crypto";
import type { ContextPack, Slice, TokenCounter } from "@sensei/shared";

export interface AssembleOptions {
  maxTokens?: number;
  counter: TokenCounter;
  task: string;
  modelId?: string;
  sessionContext?: string[];
}

export class Assembler {
  assemble(slices: Slice[], opts: AssembleOptions): ContextPack {
    const maxTokens = opts.maxTokens ?? 8000;
    const excluded = new Set(opts.sessionContext ?? []);
    const sorted = [...slices].sort((a, b) => b.score - a.score);

    const included: Slice[] = [];
    let totalTokens = 0;

    for (const slice of sorted) {
      if (excluded.has(slice.filePath)) continue;
      if (totalTokens + slice.tokens > maxTokens) continue;
      included.push(slice);
      totalTokens += slice.tokens;
    }

    return {
      id: randomUUID(),
      task: opts.task,
      slices: included,
      totalTokens,
      modelId: opts.modelId,
      createdAt: new Date().toISOString(),
    };
  }
}
```

- [ ] **Step 4: Run all engine tests**

```bash
cd packages/engine && bunx vitest run
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/assemble/
git commit -m "feat(engine): add Assembler with budget gate and sessionContext dedup"
```

---

## Chunk 4: MarkdownAdapter + Pipeline + Orchestrator

### Task 11: MarkdownAdapter

**Files:**
- Create: `packages/engine/src/adapters/markdown.ts`
- Create: `packages/engine/src/adapters/markdown.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/adapters/markdown.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { MarkdownAdapter } from "./markdown.js";

describe("MarkdownAdapter", () => {
  const adapter = new MarkdownAdapter();

  it("splits H2 sections", () => {
    const content = ["## Overview", "This is overview.", "", "## Usage", "Use it."].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections).toHaveLength(2);
    expect(sections[0].heading).toBe("Overview");
    expect(sections[0].level).toBe(2);
    expect(sections[0].content).toContain("overview");
    expect(sections[1].heading).toBe("Usage");
  });

  it("splits H3 sections", () => {
    const content = ["## Parent", "intro", "### Sub Section", "sub content"].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections.some(s => s.level === 3 && s.heading === "Sub Section")).toBe(true);
  });

  it("records startLine and endLine (1-indexed)", () => {
    const content = ["## First", "content", "## Second", "more"].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections[0].startLine).toBe(1);
    expect(sections[0].endLine).toBe(2);
    expect(sections[1].startLine).toBe(3);
    expect(sections[1].endLine).toBe(4);
  });

  it("extracts codeRefs from backtick identifiers", () => {
    const content = ["## Overview", "Call `createClient()` to start. Use `makeSenseiClient`."].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections[0].codeRefs).toContain("createClient");
    expect(sections[0].codeRefs).toContain("makeSenseiClient");
  });

  it("returns [] for content with no H2/H3 headings", () => {
    expect(adapter.parse("docs/guide.md", "# Title\n\nJust a paragraph.\n")).toHaveLength(0);
  });

  it("extensions includes .md and .mdx", () => {
    expect(adapter.extensions).toContain(".md");
    expect(adapter.extensions).toContain(".mdx");
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/adapters/markdown.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement MarkdownAdapter**

Create `packages/engine/src/adapters/markdown.ts`:

```typescript
export interface DocSection {
  heading: string;
  level: number;
  startLine: number;
  endLine: number;
  content: string;
  codeRefs: string[];
}

export class MarkdownAdapter {
  readonly extensions = [".md", ".mdx"];

  parse(_filePath: string, content: string): DocSection[] {
    const lines = content.split("\n");
    const sections: DocSection[] = [];
    let current: { heading: string; level: number; startLine: number; contentLines: string[] } | null = null;

    const finalize = (endLine: number) => {
      if (!current) return;
      const text = current.contentLines.join("\n").trim();
      sections.push({
        heading: current.heading,
        level: current.level,
        startLine: current.startLine,
        endLine,
        content: text,
        codeRefs: extractCodeRefs(text),
      });
    };

    for (let i = 0; i < lines.length; i++) {
      const h2 = lines[i].match(/^## (.+)/);
      const h3 = lines[i].match(/^### (.+)/);
      const heading = h2 || h3;

      if (heading) {
        finalize(i);  // end previous at line before this heading
        current = { heading: heading[1].trim(), level: h2 ? 2 : 3, startLine: i + 1, contentLines: [] };
      } else if (current) {
        current.contentLines.push(lines[i]);
      }
    }

    finalize(lines.length);
    return sections;
  }
}

function extractCodeRefs(text: string): string[] {
  const refs = new Set<string>();
  const regex = /`([A-Za-z_$][A-Za-z0-9_$]*(?:\(\))?)`/g;
  let match;
  while ((match = regex.exec(text)) !== null) {
    refs.add(match[1].replace(/\(\)$/, ""));
  }
  return Array.from(refs);
}
```

- [ ] **Step 4: Run to verify pass**

```bash
cd packages/engine && bunx vitest run src/adapters/markdown.spec.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/adapters/markdown.ts packages/engine/src/adapters/markdown.spec.ts
git commit -m "feat(engine): add MarkdownAdapter for doc_sections population"
```

---

### Task 12: pipeline.ts updates (MarkdownAdapter + embeddings)

**Files:**
- Modify: `packages/engine/src/pipeline.ts`
- Modify: `packages/engine/src/pipeline.spec.ts`

- [ ] **Step 1: Write the failing tests**

Add to `packages/engine/src/pipeline.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { indexRepo } from "./pipeline.js";
import type { ModelBackend } from "@sensei/shared";

function makePipelineClient() {
  const upsert = vi.fn().mockResolvedValue({ error: null });
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          gt: vi.fn().mockResolvedValue({ data: [], error: null }),
        })),
      })),
      upsert,
      delete: vi.fn(() => ({ eq: vi.fn(() => ({ in: vi.fn().mockResolvedValue({ error: null }) })) })),
    })),
    _upsert: upsert,
  } as any;
}

function makeMockBackend(): ModelBackend {
  return {
    name: "mock",
    init: vi.fn(),
    isAvailable: vi.fn().mockResolvedValue(true),
    embed: vi.fn().mockResolvedValue(new Array(768).fill(0.1)),
    generate: vi.fn().mockResolvedValue(""),
    extract: vi.fn().mockResolvedValue({ path: "", language: "", contentHash: "", analyzedAt: "", summary: "", symbols: [] }),
  };
}

describe("indexRepo with backend", () => {
  it("calls backend.embed for each indexed TS file when backend provided", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pipeline-embed-"));
    await mkdir(join(dir, "src"), { recursive: true });
    await writeFile(join(dir, "src/foo.ts"), "export function foo() { return 1; }");

    const backend = makeMockBackend();
    await indexRepo({ repoPath: dir, repoId: "repo-1", client: makePipelineClient(), backend });

    expect(backend.embed).toHaveBeenCalled();
    await rm(dir, { recursive: true, force: true });
  });

  it("does not throw when no backend provided", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pipeline-no-embed-"));
    await mkdir(join(dir, "src"), { recursive: true });
    await writeFile(join(dir, "src/foo.ts"), "export function foo() {}");

    const result = await indexRepo({ repoPath: dir, repoId: "repo-1", client: makePipelineClient() });
    expect(result.errors).toHaveLength(0);
    await rm(dir, { recursive: true, force: true });
  });
});
```

- [ ] **Step 2: Run to verify new tests fail**

```bash
cd packages/engine && bunx vitest run src/pipeline.spec.ts
```

Expected: new tests FAIL, existing tests PASS.

- [ ] **Step 3: Update pipeline.ts**

Replace `packages/engine/src/pipeline.ts` with:

```typescript
import { readFile } from "fs/promises";
import { join } from "path";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { IndexResult, ModelBackend } from "@sensei/shared";
import { Scanner } from "./scanner.js";
import { TypeScriptAdapter } from "./adapters/typescript.js";
import { MarkdownAdapter } from "./adapters/markdown.js";
import { Indexer } from "./indexer.js";

export interface IndexRepoOptions {
  repoPath: string;
  repoId: string;
  client: SupabaseClient;
  backend?: ModelBackend;   // optional — enables embedding generation when provided
  include?: string[];
  exclude?: string[];
}

export async function indexRepo(opts: IndexRepoOptions): Promise<IndexResult> {
  const { repoPath, repoId, client, backend } = opts;

  let priorState: Array<{ file_path: string; mtime: number; content_hash: string }> = [];
  try {
    const { data } = await client
      .from("scan_state")
      .select("file_path,mtime,content_hash")
      .eq("repo_id", repoId);
    priorState = (data ?? []) as typeof priorState;
  } catch {
    // First run — no prior state
  }

  const scanner = new Scanner({ repoPath, repoId, priorState, include: opts.include, exclude: opts.exclude });
  const scan = await scanner.scan();

  const tsAdapter = new TypeScriptAdapter();
  const mdAdapter = new MarkdownAdapter();

  // Parse and index TypeScript files
  const parsedTs = await Promise.all(
    scan.files
      .filter(f => scan.changed.includes(f.path) && tsAdapter.extensions.some(ext => f.path.endsWith(ext)))
      .map(f => tsAdapter.parse(f).catch(() => null))
  );
  const validParsedTs = parsedTs.filter((p): p is NonNullable<typeof p> => p !== null);

  const indexer = new Indexer(client);
  const result = await indexer.indexFiles(scan, validParsedTs);

  // Parse and upsert Markdown sections
  const changedMdFiles = scan.files.filter(
    f => scan.changed.includes(f.path) && mdAdapter.extensions.some(ext => f.path.endsWith(ext))
  );

  for (const file of changedMdFiles) {
    try {
      const content = await readFile(join(repoPath, file.path), "utf-8");
      const sections = mdAdapter.parse(file.path, content);
      if (sections.length > 0) {
        await client.from("doc_sections").upsert(
          sections.map(s => ({
            repo_id: repoId,
            file_path: file.path,
            heading: s.heading,
            level: s.level,
            start_line: s.startLine,
            end_line: s.endLine,
            content: s.content,
            code_refs: s.codeRefs,
          })),
          { onConflict: "repo_id,file_path,start_line" }
        );
      }
    } catch {
      // Non-fatal
    }
  }

  // Generate embeddings for changed TS files (best-effort — requires backend)
  if (backend) {
    const filesToEmbed = scan.files.filter(
      f => scan.changed.includes(f.path) && tsAdapter.extensions.some(ext => f.path.endsWith(ext))
    );

    await Promise.all(
      filesToEmbed.map(async file => {
        try {
          const fileSymbols = validParsedTs.find(p => p.filePath === file.path)?.symbols ?? [];
          const chunkText = fileSymbols.map(s => `${s.name} ${s.signature ?? ""}`.trim()).join(" ");
          if (!chunkText) return;

          const embedding = await backend.embed(chunkText);
          if (embedding.length === 0) return;

          await client.from("embeddings").upsert(
            { repo_id: repoId, file_path: file.path, chunk_text: chunkText, embedding, updated_at: new Date().toISOString() },
            { onConflict: "repo_id,file_path" }
          );
        } catch {
          // Non-fatal — embedding failure doesn't block indexing
        }
      })
    );
  }

  return result;
}
```

- [ ] **Step 4: Run all pipeline tests**

```bash
cd packages/engine && bunx vitest run src/pipeline.spec.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/pipeline.ts packages/engine/src/pipeline.spec.ts
git commit -m "feat(engine): add MarkdownAdapter and embedding generation to indexRepo pipeline"
```

---

### Task 13: buildContextPack() orchestrator + engine exports

**Files:**
- Create: `packages/engine/src/context-pack.ts`
- Create: `packages/engine/src/context-pack.spec.ts`
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/context-pack.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { buildContextPack } from "./context-pack.js";
import type { ModelBackend } from "@sensei/shared";

function makeBackend(): ModelBackend {
  return {
    name: "mock",
    init: vi.fn(),
    isAvailable: vi.fn().mockResolvedValue(false),
    embed: vi.fn().mockResolvedValue([]),  // Ollama unavailable
    generate: vi.fn().mockResolvedValue(""),
    extract: vi.fn().mockResolvedValue({ path: "", language: "", contentHash: "", analyzedAt: "", summary: "", symbols: [] }),
  };
}

function makeDb() {
  const upsert = vi.fn().mockResolvedValue({ error: null });
  return {
    upsert,
    rpc: vi.fn().mockResolvedValue({ data: [], error: null }),
    from: vi.fn((table: string) => {
      if (table === "context_packs") return { upsert };
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            gt: vi.fn().mockResolvedValue({ data: [], error: null }),
            eq: vi.fn().mockResolvedValue({ data: [], error: null }),
          })),
        })),
        upsert,
      };
    }),
  } as any;
}

describe("buildContextPack", () => {
  it("returns a ContextPack with required fields", async () => {
    const dir = await mkdtemp(join(tmpdir(), "ctx-pack-"));
    const pack = await buildContextPack(makeDb(), "repo-1", dir, "fix auth", { backend: makeBackend() });

    expect(pack.id).toBeTruthy();
    expect(pack.task).toBe("fix auth");
    expect(pack.createdAt).toBeTruthy();
    expect(Array.isArray(pack.slices)).toBe(true);
    expect(pack.totalTokens).toBeGreaterThanOrEqual(0);

    await rm(dir, { recursive: true, force: true });
  });

  it("persists the pack to context_packs table", async () => {
    const db = makeDb();
    const dir = await mkdtemp(join(tmpdir(), "ctx-pack-persist-"));

    await buildContextPack(db, "repo-1", dir, "fix auth", { backend: makeBackend() });

    const contextPacksCalls = (db.from as ReturnType<typeof vi.fn>).mock.calls
      .filter((c: any[]) => c[0] === "context_packs");
    expect(contextPacksCalls.length).toBeGreaterThan(0);

    await rm(dir, { recursive: true, force: true });
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd packages/engine && bunx vitest run src/context-pack.spec.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement buildContextPack()**

Create `packages/engine/src/context-pack.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ContextPack, ModelBackend } from "@sensei/shared";
import { createTokenCounter } from "@sensei/shared";
import type { Candidate } from "./rank/ranking-strategy.js";
import { DiffFirstBFSStrategy } from "./rank/diff-first-bfs.js";
import { BM25Strategy } from "./rank/bm25.js";
import { SemanticStrategy } from "./rank/semantic.js";
import { RankingStrategyChain } from "./rank/chain.js";
import { ASTSlicer } from "./slice/ast-slicer.js";
import { SectionSlicer } from "./slice/section-slicer.js";
import { Assembler } from "./assemble/assembler.js";

const CHANGED_FILES_WINDOW_MS = 24 * 60 * 60 * 1000;

export interface BuildContextPackOptions {
  maxTokens?: number;
  modelId?: string;
  sessionId?: string;
  sessionContext?: string[];
  backend: ModelBackend;
}

export async function buildContextPack(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,
  task: string,
  opts: BuildContextPackOptions,
): Promise<ContextPack> {
  const { maxTokens = 8000, modelId, sessionId, sessionContext, backend } = opts;
  const counter = createTokenCounter(modelId);

  // 1. Load all candidates from scan_state
  const { data: allFiles } = await db
    .from("scan_state")
    .select("file_path")
    .eq("repo_id", repoId);

  const candidates: Candidate[] = (allFiles ?? []).map((f: { file_path: string }) => ({
    filePath: f.file_path,
    type: (f.file_path.endsWith(".md") || f.file_path.endsWith(".mdx")) ? "doc" : "code",
  }));

  // 2. Load changed files (last 24h)
  const since = new Date(Date.now() - CHANGED_FILES_WINDOW_MS).toISOString();
  const { data: changedData } = await db
    .from("scan_state")
    .select("file_path")
    .eq("repo_id", repoId)
    .gt("indexed_at", since);
  const changedFiles = (changedData ?? []).map((f: { file_path: string }) => f.file_path);

  // 3. Rank
  const chain = new RankingStrategyChain([
    new DiffFirstBFSStrategy(),
    new BM25Strategy(),
    new SemanticStrategy(),
  ]);
  const ranked = await chain.rank(candidates, { task, repoId, changedFiles, db, backend, modelId });

  // 4. Slice
  const astSlicer = new ASTSlicer(db, repoPath, repoId);
  const sectionSlicer = new SectionSlicer(db, repoId);

  const sliceResults = await Promise.all(
    ranked.map(candidate =>
      candidate.type === "code"
        ? astSlicer.slice(candidate, counter)
        : sectionSlicer.slice(candidate, counter)
    )
  );
  const allSlices = sliceResults.flat();

  // 5. Assemble
  const pack = new Assembler().assemble(allSlices, { maxTokens, counter, task, modelId, sessionContext });

  // 6. Persist
  await db.from("context_packs").upsert({
    id: pack.id,
    repo_id: repoId,
    session_id: sessionId ?? null,
    task,
    model_id: modelId ?? null,
    slices: pack.slices,
    total_tokens: pack.totalTokens,
    created_at: pack.createdAt,
  });

  return pack;
}
```

- [ ] **Step 4: Update engine index.ts**

Replace `packages/engine/src/index.ts` with:

```typescript
export * from "./scanner.js";
export * from "./adapters/typescript.js";
export * from "./adapters/markdown.js";
export * from "./indexer.js";
export * from "./pipeline.js";
export * from "./rank/ranking-strategy.js";
export * from "./rank/diff-first-bfs.js";
export * from "./rank/bm25.js";
export * from "./rank/semantic.js";
export * from "./rank/chain.js";
export * from "./slice/ast-slicer.js";
export * from "./slice/section-slicer.js";
export * from "./assemble/assembler.js";
export * from "./context-pack.js";
```

- [ ] **Step 5: Run all engine tests**

```bash
cd packages/engine && bunx vitest run
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/context-pack.ts packages/engine/src/context-pack.spec.ts \
        packages/engine/src/index.ts
git commit -m "feat(engine): add buildContextPack() orchestrator and export all rank/slice/assemble"
```

---

## Chunk 5: MCP Tools

### Task 14: context_pack MCP tool

**Files:**
- Create: `packages/server/src/tools/context-pack.ts`
- Modify: `packages/server/src/mcp-server.ts`

- [ ] **Step 1: Create context-pack tool function**

Create `packages/server/src/tools/context-pack.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";
import { buildContextPack } from "@sensei/engine";

export async function contextPack(
  client: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  repoPath: string,
  task: string,
  opts: {
    maxTokens?: number;
    modelId?: string;
    sessionId?: string;
    sessionContext?: string[];
  } = {}
) {
  const pack = await buildContextPack(client, repoId, repoPath, task, { ...opts, backend });
  return {
    id: pack.id,
    task: pack.task,
    totalTokens: pack.totalTokens,
    modelId: pack.modelId ?? null,
    createdAt: pack.createdAt,
    slices: pack.slices.map(s => ({
      kind: s.kind,
      filePath: s.filePath,
      startLine: s.startLine,
      endLine: s.endLine,
      content: s.content,
      tokens: s.tokens,
      score: s.score,
      ...(s.kind === "code" ? { symbolName: s.symbolName } : { heading: (s as any).heading }),
    })),
  };
}
```

- [ ] **Step 2: Add OllamaBackend lazy init + register context_pack in mcp-server.ts**

In `packages/server/src/mcp-server.ts`:

1. Add imports at top (after existing imports):

```typescript
import { OllamaBackend } from "./model/ollama-backend.js";
import { contextPack } from "./tools/context-pack.js";
import { recommendNext } from "./tools/recommend-next.js";
import { tokenStats } from "./tools/token-stats.js";
```

2. Inside `createSenseiMcpServer()`, after the `getClient` lazy init, add:

```typescript
let backendInstance: OllamaBackend | null = null;
const getBackend = () => {
  if (!backendInstance) backendInstance = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });
  return backendInstance;
};
```

3. Register the tool (before `return server`):

```typescript
server.tool(
  "context_pack",
  "Get a ranked, token-budgeted set of code and doc slices relevant to a task. More precise than load_context — use this when you need focused context for a specific task.",
  {
    task: z.string().describe("Task description — the code problem or question you are working on"),
    max_tokens: z.number().int().min(100).max(32000).optional().default(8000).describe("Maximum tokens to include"),
    model_id: z.string().optional().describe("Your model ID (e.g. 'claude-sonnet-4-6') — selects the right token counter"),
    session_id: z.string().optional().describe("Session ID for grouping packs in the dashboard"),
    session_context: z.array(z.string()).optional().describe("File paths already in your context — excluded from the pack"),
  },
  async ({ task, max_tokens, model_id, session_id, session_context }) => {
    try {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };
      const result = await contextPack(client as any, getBackend(), opts.repoId, opts.repoPath, task, {
        maxTokens: max_tokens,
        modelId: model_id,
        sessionId: session_id,
        sessionContext: session_context,
      });
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);
```

- [ ] **Step 3: Verify existing server tests still pass**

```bash
cd packages/server && bunx vitest run src/mcp-server.spec.ts
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add packages/server/src/tools/context-pack.ts packages/server/src/mcp-server.ts
git commit -m "feat(server): register context_pack MCP tool"
```

---

### Task 15: recommend_next + token_stats MCP tools

**Files:**
- Create: `packages/server/src/tools/recommend-next.ts`
- Create: `packages/server/src/tools/token-stats.ts`
- Modify: `packages/server/src/mcp-server.ts`

- [ ] **Step 1: Create recommend-next.ts**

Create `packages/server/src/tools/recommend-next.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend } from "@sensei/shared";
import { DiffFirstBFSStrategy, BM25Strategy, SemanticStrategy, RankingStrategyChain } from "@sensei/engine";
import type { Candidate } from "@sensei/engine";
import { createTokenCounter } from "@sensei/shared";

const CHANGED_FILES_WINDOW_MS = 24 * 60 * 60 * 1000;

export async function recommendNext(
  client: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  task: string,
  modelId?: string,
) {
  const counter = createTokenCounter(modelId);

  const { data: allFiles } = await client.from("scan_state").select("file_path").eq("repo_id", repoId);
  const candidates: Candidate[] = (allFiles ?? []).map((f: { file_path: string }) => ({
    filePath: f.file_path,
    type: (f.file_path.endsWith(".md") || f.file_path.endsWith(".mdx")) ? "doc" : "code",
  }));

  const since = new Date(Date.now() - CHANGED_FILES_WINDOW_MS).toISOString();
  const { data: changedData } = await client.from("scan_state").select("file_path").eq("repo_id", repoId).gt("indexed_at", since);
  const changedFiles = (changedData ?? []).map((f: { file_path: string }) => f.file_path);

  const chain = new RankingStrategyChain([new DiffFirstBFSStrategy(), new BM25Strategy(), new SemanticStrategy()]);
  const ranked = await chain.rank(candidates, { task, repoId, changedFiles, db: client, backend, modelId });
  const top3 = ranked.slice(0, 3);

  const recs = await Promise.all(
    top3.map(async candidate => {
      const { data: syms } = await client
        .from("symbols")
        .select("name,signature")
        .eq("repo_id", repoId)
        .eq("file_path", candidate.filePath);

      const estimatedTokens = (syms ?? []).reduce(
        (sum: number, s: { name: string; signature: string | null }) =>
          sum + counter.count(`${s.name} ${s.signature ?? ""}`.trim()),
        0
      );

      return { filePath: candidate.filePath, score: candidate.score, symbolCount: (syms ?? []).length, estimatedTokens };
    })
  );

  return {
    recommendations: recs,
    suggestedBudget: Math.min(Math.ceil(recs.reduce((s, r) => s + r.estimatedTokens, 0) * 1.5), 8000),
  };
}
```

- [ ] **Step 2: Create token-stats.ts**

Create `packages/server/src/tools/token-stats.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";

export async function tokenStats(client: SupabaseClient, sessionId: string) {
  const { data, error } = await client
    .from("context_packs")
    .select("id,task,total_tokens,created_at")
    .eq("session_id", sessionId)
    .order("created_at", { ascending: false });

  if (error || !data) return { totalPacks: 0, totalTokensServed: 0, avgPackSize: 0, packs: [] };

  const packs = data as Array<{ id: string; task: string; total_tokens: number; created_at: string }>;
  const totalTokensServed = packs.reduce((sum, p) => sum + p.total_tokens, 0);

  return {
    totalPacks: packs.length,
    totalTokensServed,
    avgPackSize: packs.length > 0 ? Math.round(totalTokensServed / packs.length) : 0,
    packs: packs.map(p => ({ id: p.id, task: p.task, totalTokens: p.total_tokens, createdAt: p.created_at })),
  };
}
```

- [ ] **Step 3: Register both tools in mcp-server.ts**

Add after the `context_pack` tool registration (before `return server`):

```typescript
server.tool(
  "recommend_next",
  "Get the top 3 most relevant files for a task with estimated token counts and a suggested budget for context_pack",
  {
    task: z.string().describe("Task description"),
    model_id: z.string().optional().describe("Your model ID for accurate token counting"),
  },
  async ({ task, model_id }) => {
    try {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
      const result = await recommendNext(client as any, getBackend(), opts.repoId, task, model_id);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);

server.tool(
  "token_stats",
  "Get token usage statistics for a session — total packs requested and tokens served",
  {
    session_id: z.string().describe("Session ID to look up stats for"),
  },
  async ({ session_id }) => {
    try {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
      const result = await tokenStats(client as any, session_id);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);
```

- [ ] **Step 4: Run all server tests**

```bash
cd packages/server && bunx vitest run
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/server/src/tools/recommend-next.ts packages/server/src/tools/token-stats.ts \
        packages/server/src/mcp-server.ts
git commit -m "feat(server): register recommend_next and token_stats MCP tools"
```

---

## Chunk 6: Dashboard

### Task 16: Context Pack inspector

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/context-packs/+page.server.ts`
- Create: `apps/dashboard/src/routes/repos/[id]/context-packs/+page.svelte`
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.svelte`

- [ ] **Step 1: Create the server load function**

Create `apps/dashboard/src/routes/repos/[id]/context-packs/+page.server.ts`:

```typescript
import type { PageServerLoad } from './$types';
import { getDb } from '$lib/server/db';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const { data: packs } = await db
    .from('context_packs')
    .select('id,task,model_id,session_id,total_tokens,slices,created_at')
    .eq('repo_id', params.id)
    .order('created_at', { ascending: false })
    .limit(50);

  return {
    repo: repo as { id: string; name: string },
    packs: (packs ?? []).map(p => ({
      id: p.id as string,
      task: p.task as string,
      modelId: p.model_id as string | null,
      sessionId: p.session_id as string | null,
      totalTokens: p.total_tokens as number,
      slices: (p.slices as any[]) ?? [],
      createdAt: p.created_at as string,
    })),
  };
};
```

- [ ] **Step 2: Create the Svelte page**

Create `apps/dashboard/src/routes/repos/[id]/context-packs/+page.svelte`:

```svelte
<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data } = $props();

  let expandedId = $state<string | null>(null);

  const toggle = (id: string) => { expandedId = expandedId === id ? null : id; };
  const fmt = (iso: string) => new Date(iso).toLocaleString();

  // Columns for Rokkit Table (matches existing dashboard Table usage)
  const sliceColumns = [
    { name: 'filePath',    label: 'File',           sortable: true },
    { name: 'lines',       label: 'Lines',          sortable: false },
    { name: 'kind',        label: 'Kind',           sortable: true },
    { name: 'label',       label: 'Symbol/Heading', sortable: false },
    { name: 'tokens',      label: 'Tokens',         sortable: true },
    { name: 'score',       label: 'Score',          sortable: true },
  ];

  function sliceRows(slices: any[]) {
    return slices.map(s => ({
      filePath: s.filePath,
      lines: `${s.startLine}–${s.endLine}`,
      kind: s.kind,
      label: s.kind === 'code' ? (s.symbolName ?? '') : (s.heading ?? ''),
      tokens: s.tokens,
      score: typeof s.score === 'number' ? s.score.toFixed(2) : '—',
    }));
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Context Packs — {data.repo.name}</h1>

{#if data.packs.length === 0}
  <p>No context packs yet. Call the <code>context_pack</code> MCP tool to generate one.</p>
{:else}
  <p>{data.packs.length} pack{data.packs.length !== 1 ? 's' : ''}</p>

  {#each data.packs as pack (pack.id)}
    <div class="pack">
      <div class="pack-header" onclick={() => toggle(pack.id)} role="button" tabindex="0"
           onkeydown={(e) => e.key === 'Enter' && toggle(pack.id)}>
        <div class="pack-title">
          <span class="task">{pack.task}</span>
          {#if pack.sessionId}<span class="meta">session: {pack.sessionId}</span>{/if}
        </div>
        <div class="pack-meta">
          <span class="tokens">{pack.totalTokens.toLocaleString()} tokens</span>
          <span class="date">{fmt(pack.createdAt)}</span>
          <span>{expandedId === pack.id ? '▲' : '▼'}</span>
        </div>
      </div>

      {#if expandedId === pack.id}
        <div class="pack-slices">
          {#if pack.slices.length === 0}
            <p class="empty">No slices in this pack.</p>
          {:else}
            <Table data={sliceRows(pack.slices)} columns={sliceColumns} />
            <p class="total">{pack.totalTokens.toLocaleString()} tokens</p>
          {/if}
        </div>
      {/if}
    </div>
  {/each}
{/if}

<style>
  .pack { border: 1px solid #ccc; margin-bottom: 0.75rem; border-radius: 4px; overflow: hidden; }
  .pack-header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 1rem; cursor: pointer; background: #fafafa; }
  .pack-header:hover { background: #f0f0f0; }
  .pack-title { display: flex; flex-direction: column; gap: 0.25rem; }
  .task { font-weight: 500; }
  .meta { font-size: 0.8rem; color: #666; }
  .pack-meta { display: flex; gap: 1rem; align-items: center; font-size: 0.85rem; }
  .tokens { font-weight: 500; }
  .pack-slices { padding: 0.75rem 1rem; }
  .total { text-align: right; font-size: 0.85rem; color: #555; margin-top: 0.5rem; }
  .empty { color: #999; font-style: italic; }
</style>
```

> Note: `Table` from `@rokkit/ui` is used for the slices table (matches existing `/repos/[id]` pattern). The accordion expansion uses plain Svelte state — Rokkit `List` is navigation-only and doesn't support expandable rows. `maxTokens` is not stored in `context_packs`, so only `totalTokens` is shown (not a ratio).

- [ ] **Step 3: Add context packs link to repo detail page**

In `apps/dashboard/src/routes/repos/[id]/+page.svelte`, the file currently ends with:

```svelte
<Table data={filtered} {columns} />
```

Append after that line:

```svelte
<p><a href="/repos/{data.repo.id}/context-packs">View Context Packs →</a></p>
```

- [ ] **Step 4: Verify the dev server runs**

```bash
cd apps/dashboard && bun dev
```

Navigate to `http://localhost:5173/repos` → click a repo → verify "View Context Packs →" link → click it → verify page loads without errors.

- [ ] **Step 5: Commit**

```bash
git add apps/dashboard/src/routes/repos/
git commit -m "feat(dashboard): add Context Pack inspector at /repos/[id]/context-packs"
```

---

## Done When

- [ ] `supabase db push --local` applies migration without errors
- [ ] `cd packages/shared && bunx vitest run` — all tests pass including token-counter
- [ ] `cd packages/engine && bunx vitest run` — all tests pass across rank/slice/assemble/context-pack
- [ ] `cd packages/server && bunx vitest run` — all tests pass
- [ ] MCP server exposes 6 tools: `get_session_context`, `search`, `load_context`, `context_pack`, `recommend_next`, `token_stats`
- [ ] Dashboard `/repos/[id]/context-packs` page loads without errors
- [ ] `context_pack({ task: "fix auth middleware" })` returns a ContextPack with at least 3 slices and `totalTokens ≤ 8000` (requires prior `sensei init` run + `ollama pull nomic-embed-text`)
