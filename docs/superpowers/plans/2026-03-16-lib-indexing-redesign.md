# Library Indexing Redesign Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove Ollama dependency from library indexing, split into Fetch and Embed phases, and add GitHub folder support as a source type.

**Architecture:** Phase 1 (Fetch) fetches/parses docs and stores sections without embeddings — runs anywhere, immediately browseable with keyword search. Phase 2 (Embed) is optional, generates 384-dim vectors using Transformers.js (`all-MiniLM-L6-v2`) for semantic search. A new `GithubAdapter` handles `github.com/*/tree/*` URLs identically to `LocalAdapter`.

**Tech Stack:** TypeScript, Vitest, Supabase/pgvector, `@xenova/transformers`, SvelteKit 2 / Svelte 5 runes, UnoCSS (Rokkit tokens)

---

## File Map

| File | Change |
|------|--------|
| `supabase/migrations/20260316000003_lib_phase_split.sql` | Create: change `vector(768)→384`, add `embed_status`, add `'github'` to CHECK constraints |
| `packages/shared/src/types.ts` | Modify: add `'github'` to `LibEntry.source_type` and `DocPage.sourceType` unions |
| `packages/engine/src/lib/github-adapter.ts` | Create: fetches `.md` files from GitHub tree URL |
| `packages/engine/src/lib/github-adapter.spec.ts` | Create: tests for GithubAdapter |
| `packages/engine/src/lib/lib-indexer.ts` | Modify: make `backend` optional (`null` → skip embedding) |
| `packages/engine/src/lib/infer-source-type.ts` | Create: shared `inferSourceType()` utility (github + llms.txt detection) |
| `packages/engine/src/lib/transformers-backend.ts` | Create: `TransformersBackend` using `@xenova/transformers`, 384-dim |
| `packages/engine/src/index.ts` | Modify: export new files |
| `packages/engine/package.json` | Modify: add `@xenova/transformers` dependency |
| `apps/dashboard/src/lib/server/lib-indexer.ts` | Modify: Phase 1 only (no Ollama), use `inferSourceType`, add `startEmbedding()` |
| `apps/dashboard/src/routes/libraries/+page.server.ts` | Modify: `add` action uses shared `inferSourceType` |
| `apps/dashboard/src/routes/libraries/[id]/+page.server.ts` | Modify: add `embed` action, use shared `inferSourceType` |
| `apps/dashboard/src/routes/libraries/[id]/+page.svelte` | Modify: "Build Index" button, disable Re-index for local libs, poll embed_status |
| `packages/server/src/tools/get-lib-docs.ts` | Modify: keyword fallback when vector search errors |

---

## Chunk 1: DB migration + shared types

### Task 1: DB migration — phase split schema

**Files:**
- Create: `supabase/migrations/20260316000003_lib_phase_split.sql`

- [ ] **Step 1: Write the migration**

```sql
-- supabase/migrations/20260316000003_lib_phase_split.sql

-- 1. Drop objects that depend on the 768-dim embedding column
drop index if exists sensei.shared_lib_sections_embedding_idx;
-- Full signature required: PostgreSQL needs arg types to resolve the function
drop function if exists sensei.match_shared_lib_sections(uuid, text, vector(768), int);

-- 2. Change embedding dimension: 768 → 384
alter table sensei.shared_lib_sections drop column if exists embedding;
alter table sensei.shared_lib_sections add column embedding vector(384);

-- 3. Recreate IVFFlat index (384-dim, cosine)
create index if not exists shared_lib_sections_embedding_idx
  on sensei.shared_lib_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

-- 4. Recreate RPC with 384-dim query vector
create or replace function sensei.match_shared_lib_sections(
  p_shared_lib_id uuid,
  p_component     text,
  query_embedding vector(384),
  match_count     int default 10
)
returns table (
  id          uuid,
  title       text,
  url         text,
  local_path  text,
  description text,
  content     text,
  source_type text,
  component   text,
  similarity  float
)
language sql stable
as $$
  select
    id, title, url, local_path, description, content, source_type, component,
    1 - (embedding <=> query_embedding) as similarity
  from sensei.shared_lib_sections
  where shared_lib_id = p_shared_lib_id
    and (p_component is null or component = p_component)
    and embedding is not null
  order by embedding <=> query_embedding
  limit match_count;
$$;

-- 5. Add 'github' to source_type check on shared_libs
alter table sensei.shared_libs
  drop constraint if exists shared_libs_source_type_check;
alter table sensei.shared_libs
  add constraint shared_libs_source_type_check
  check (source_type in ('llms.txt', 'http', 'local', 'github'));

-- 6. Add 'github' to source_type check on shared_lib_sections
alter table sensei.shared_lib_sections
  drop constraint if exists shared_lib_sections_source_type_check;
alter table sensei.shared_lib_sections
  add constraint shared_lib_sections_source_type_check
  check (source_type in ('llms.txt', 'http', 'local', 'github'));

-- 7. Add embed_status to shared_libs (null = never embedded)
alter table sensei.shared_libs
  add column if not exists embed_status text
  check (embed_status in ('pending', 'embedding', 'ready'));
```

- [ ] **Step 2: Apply the migration**

```bash
cd /Users/Jerry/Developer/sensei
psql "$DATABASE_URL" -f supabase/migrations/20260316000003_lib_phase_split.sql
```

Expected: No errors. If constraint names differ, check with:
```bash
psql "$DATABASE_URL" -c "\d sensei.shared_libs" | grep check
psql "$DATABASE_URL" -c "\d sensei.shared_lib_sections" | grep check
```
Then adjust the `DROP CONSTRAINT` names accordingly.

- [ ] **Step 3: Verify schema**

```bash
psql "$DATABASE_URL" -c "select column_name, data_type, character_maximum_length from information_schema.columns where table_schema='sensei' and table_name='shared_lib_sections' and column_name='embedding';"
```

Expected: `embedding | USER-DEFINED | null` (pgvector type, dimension changed)

```bash
psql "$DATABASE_URL" -c "select column_name from information_schema.columns where table_schema='sensei' and table_name='shared_libs' and column_name='embed_status';"
```

Expected: one row `embed_status`

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260316000003_lib_phase_split.sql
git commit -m "feat(db): split lib indexing — 384-dim embeddings, embed_status, github source type"
```

---

### Task 2: Update shared types

**Files:**
- Modify: `packages/shared/src/types.ts`

- [ ] **Step 1: Add `'github'` to `LibEntry.source_type` and `DocPage.sourceType`**

In `packages/shared/src/types.ts`, line 138:
```typescript
// Before:
source_type: 'llms.txt' | 'http' | 'local';
// After:
source_type: 'llms.txt' | 'http' | 'local' | 'github';
```

In `packages/shared/src/types.ts`, line 151:
```typescript
// Before:
sourceType: 'llms.txt' | 'http' | 'local';
// After:
sourceType: 'llms.txt' | 'http' | 'local' | 'github';
```

- [ ] **Step 2: Verify no type errors**

```bash
cd /Users/Jerry/Developer/sensei/packages/shared
bun run tsc --noEmit 2>&1 | head -20
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add packages/shared/src/types.ts
git commit -m "feat(shared): add 'github' to LibEntry and DocPage source type unions"
```

---

## Chunk 2: GithubAdapter + inferSourceType utility

### Task 3: `inferSourceType` utility

**Files:**
- Create: `packages/engine/src/lib/infer-source-type.ts`
- Create: `packages/engine/src/lib/infer-source-type.spec.ts`

This extracts the URL-to-source-type logic currently duplicated in dashboard server files.

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/engine/src/lib/infer-source-type.spec.ts
import { describe, it, expect } from "vitest";
import { inferSourceType } from "./infer-source-type.js";

describe("inferSourceType", () => {
  it("detects llms.txt URL", () => {
    const r = inferSourceType("https://kavach.dev/llms.txt");
    expect(r.source_type).toBe("llms.txt");
    expect(r.base_url).toBe("https://kavach.dev/llms.txt");
  });

  it("detects GitHub tree URL", () => {
    const r = inferSourceType("https://github.com/org/repo/tree/main/docs");
    expect(r.source_type).toBe("github");
    expect(r.base_url).toBe("https://github.com/org/repo/tree/main/docs");
  });

  it("detects plain HTTP URL as http", () => {
    const r = inferSourceType("https://docs.example.com/api");
    expect(r.source_type).toBe("http");
    expect(r.base_url).toBe("https://docs.example.com/api");
  });

  it("detects local path", () => {
    const r = inferSourceType("/Users/jerry/projects/mylib/docs");
    expect(r.source_type).toBe("local");
    expect((r as any).local_path).toBe("/Users/jerry/projects/mylib/docs");
  });

  it("falls through to http (not github) for non-tree github URLs", () => {
    const r = inferSourceType("https://github.com/org/repo");
    expect(r.source_type).toBe("http");
  });

  it("falls through to http on malformed URL that starts with https://", () => {
    // Should not throw — falls back to 'http' on URL parse failure
    const r = inferSourceType("https://");
    expect(r.source_type).toBe("http");
  });
});
```

- [ ] **Step 2: Run tests — confirm FAIL**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bunx vitest run src/lib/infer-source-type.spec.ts 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './infer-source-type.js'`

- [ ] **Step 3: Write the implementation**

```typescript
// packages/engine/src/lib/infer-source-type.ts

export type InferredSource =
  | { source_type: 'llms.txt'; base_url: string; local_path?: undefined }
  | { source_type: 'github';   base_url: string; local_path?: undefined }
  | { source_type: 'http';     base_url: string; local_path?: undefined }
  | { source_type: 'local';    local_path: string; base_url?: undefined };

/**
 * Infer source_type, base_url, and local_path from a user-provided URL or path.
 *
 * Detection order:
 *  1. URL ending in /llms.txt          → llms.txt
 *  2. github.com/{owner}/{repo}/tree/* → github
 *  3. Any other http(s) URL            → http
 *  4. Anything else                    → local
 */
export function inferSourceType(input: string): InferredSource {
  if (input.startsWith('http://') || input.startsWith('https://')) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith('/llms.txt')) {
        return { source_type: 'llms.txt', base_url: input };
      }
      if (url.hostname === 'github.com' && /^\/[^/]+\/[^/]+\/tree\//.test(url.pathname)) {
        return { source_type: 'github', base_url: input };
      }
    } catch {
      // Malformed URL — fall through to http type
    }
    return { source_type: 'http', base_url: input };
  }
  return { source_type: 'local', local_path: input };
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
bunx vitest run src/lib/infer-source-type.spec.ts
```

Expected: all PASS

- [ ] **Step 5: Export from engine index**

In `packages/engine/src/index.ts`, add:
```typescript
export * from "./lib/infer-source-type.js";
```

- [ ] **Step 6: Run full suite**

```bash
bunx vitest run
```

Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add packages/engine/src/lib/infer-source-type.ts packages/engine/src/lib/infer-source-type.spec.ts packages/engine/src/index.ts
git commit -m "feat(engine): add inferSourceType utility with github detection"
```

---

### Task 4: GithubAdapter

**Files:**
- Create: `packages/engine/src/lib/github-adapter.ts`
- Create: `packages/engine/src/lib/github-adapter.spec.ts`
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
// packages/engine/src/lib/github-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { GithubAdapter, parseGithubUrl } from "./github-adapter.js";
import type { LibEntry } from "@sensei/shared";

const TREE_RESPONSE = {
  tree: [
    { path: "docs/llms/README.md",  type: "blob", url: "" },
    { path: "docs/llms/api.md",     type: "blob", url: "" },
    { path: "docs/llms/Button.md",  type: "blob", url: "" },
    { path: "docs/llms/subdir",     type: "tree", url: "" },
    { path: "docs/other/skip.md",   type: "blob", url: "" }, // outside basePath
    { path: "docs/llms/config.json",type: "blob", url: "" }, // wrong extension
  ],
  truncated: false,
};

const README_CONTENT = `# My Library\n\nThis is the main introduction paragraph.\n\n## Section\n\nMore content.`;
const API_CONTENT = `# API Reference\n\nDetailed API docs here.`;
const BUTTON_CONTENT = `Button component documentation.`;

describe("parseGithubUrl", () => {
  it("parses owner/repo/branch/path from a GitHub tree URL", () => {
    const result = parseGithubUrl("https://github.com/org/repo/tree/main/docs/llms");
    expect(result).toEqual({ owner: "org", repo: "repo", branch: "main", basePath: "docs/llms" });
  });

  it("handles URLs with no trailing path", () => {
    const result = parseGithubUrl("https://github.com/org/repo/tree/develop");
    expect(result).toEqual({ owner: "org", repo: "repo", branch: "develop", basePath: "" });
  });

  it("returns null for non-GitHub-tree URLs", () => {
    expect(parseGithubUrl("https://example.com/docs")).toBeNull();
    expect(parseGithubUrl("https://github.com/org/repo")).toBeNull();
  });
});

describe("GithubAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  it("fetches tree API, filters .md files under basePath, returns DocPages", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(TREE_RESPONSE) }) // tree API
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(README_CONTENT) })  // README.md
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(API_CONTENT) })     // api.md
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(BUTTON_CONTENT) }); // Button.md
    vi.stubGlobal("fetch", fetchMock);

    const adapter = new GithubAdapter();
    const entry: LibEntry = {
      name: "mylib",
      source_type: "github",
      base_url: "https://github.com/org/repo/tree/main/docs/llms",
    };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(3); // skip docs/other/skip.md and config.json
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.github.com/repos/org/repo/git/trees/main?recursive=1",
      expect.any(Object)
    );
  });

  it("sets title from first H1, description from first paragraph, url to github blob URL", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({
        tree: [{ path: "docs/readme.md", type: "blob", url: "" }],
        truncated: false,
      }) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(README_CONTENT) });
    vi.stubGlobal("fetch", fetchMock);

    const adapter = new GithubAdapter();
    const pages = await adapter.fetch({
      name: "lib", source_type: "github",
      base_url: "https://github.com/org/repo/tree/main/docs",
    });

    expect(pages[0].title).toBe("My Library");
    expect(pages[0].description).toBe("This is the main introduction paragraph.");
    expect(pages[0].url).toBe("https://github.com/org/repo/blob/main/docs/readme.md");
    expect(pages[0].sourceType).toBe("github");
    expect(pages[0].content).toBe(README_CONTENT);
  });

  it("infers component from immediate parent dir when nested deeper than basePath", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({
        tree: [
          { path: "docs/Button/usage.md", type: "blob", url: "" },
          { path: "docs/intro.md",        type: "blob", url: "" },
        ],
        truncated: false,
      }) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("Button usage docs.") })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("Introduction.") });
    vi.stubGlobal("fetch", fetchMock);

    const pages = await new GithubAdapter().fetch({
      name: "lib", source_type: "github",
      base_url: "https://github.com/org/repo/tree/main/docs",
    });

    const btn = pages.find(p => p.title === "usage")!;
    expect(btn.component).toBe("Button");

    const intro = pages.find(p => p.title === "intro")!;
    expect(intro.component).toBeUndefined();
  });

  it("falls back to filename as title when no H1 found", async () => {
    vi.stubGlobal("fetch", vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({
        tree: [{ path: "docs/no-heading.md", type: "blob", url: "" }],
        truncated: false,
      }) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("Just some text, no heading.") })
    );

    const pages = await new GithubAdapter().fetch({
      name: "lib", source_type: "github",
      base_url: "https://github.com/org/repo/tree/main/docs",
    });

    expect(pages[0].title).toBe("no-heading");
  });

  it("throws if no base_url provided", async () => {
    await expect(
      new GithubAdapter().fetch({ name: "x", source_type: "github" })
    ).rejects.toThrow("requires base_url");
  });

  it("throws if base_url is not a valid GitHub tree URL", async () => {
    await expect(
      new GithubAdapter().fetch({ name: "x", source_type: "github", base_url: "https://example.com/docs" })
    ).rejects.toThrow("invalid GitHub tree URL");
  });

  it("throws if GitHub API returns an error status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 403 }));
    await expect(
      new GithubAdapter().fetch({ name: "x", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" })
    ).rejects.toThrow("GitHub API error 403");
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bunx vitest run src/lib/github-adapter.spec.ts 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './github-adapter.js'`

- [ ] **Step 3: Write the implementation**

```typescript
// packages/engine/src/lib/github-adapter.ts
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

interface GitHubTreeItem {
  path: string;
  type: "blob" | "tree";
  url: string;
}

interface GitHubTreeResponse {
  tree: GitHubTreeItem[];
  truncated: boolean;
}

export interface ParsedGithubUrl {
  owner: string;
  repo: string;
  branch: string;
  basePath: string;
}

export function parseGithubUrl(url: string): ParsedGithubUrl | null {
  const match = url.match(
    /^https:\/\/github\.com\/([^/]+)\/([^/]+)\/tree\/([^/]+)\/?(.*)$/
  );
  if (!match) return null;
  return { owner: match[1], repo: match[2], branch: match[3], basePath: match[4] ?? "" };
}

export class GithubAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) {
      throw new Error(`GithubAdapter: entry "${entry.name}" requires base_url`);
    }

    const parsed = parseGithubUrl(entry.base_url);
    if (!parsed) {
      throw new Error(`GithubAdapter: invalid GitHub tree URL: ${entry.base_url}`);
    }

    const { owner, repo, branch, basePath } = parsed;

    const treeUrl = `https://api.github.com/repos/${owner}/${repo}/git/trees/${branch}?recursive=1`;
    const treeRes = await fetch(treeUrl, {
      headers: { Accept: "application/vnd.github.v3+json" },
    });
    if (!treeRes.ok) {
      throw new Error(`GithubAdapter: GitHub API error ${treeRes.status} for ${treeUrl}`);
    }

    const tree: GitHubTreeResponse = await treeRes.json();
    const prefix = basePath ? basePath + "/" : "";
    const mdFiles = tree.tree.filter(
      (item) =>
        item.type === "blob" &&
        item.path.startsWith(prefix) &&
        item.path.endsWith(".md")
    );

    const pages: DocPage[] = [];
    for (const file of mdFiles) {
      const rawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${file.path}`;
      const res = await fetch(rawUrl);
      if (!res.ok) continue;
      const content = await res.text();

      const title = extractH1(content) ?? stemName(file.path);
      const description = extractFirstParagraph(content);
      const blobUrl = `https://github.com/${owner}/${repo}/blob/${branch}/${file.path}`;
      const component = inferComponent(file.path, basePath);

      pages.push({ title, url: blobUrl, description, content, sourceType: "github", component });
    }

    return pages;
  }
}

function extractH1(content: string): string | undefined {
  const match = content.match(/^#\s+(.+)$/m);
  return match?.[1]?.trim();
}

function extractFirstParagraph(content: string): string {
  const lines = content.split("\n");
  let started = false;
  const buf: string[] = [];
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("#")) continue;
    if (trimmed === "") {
      if (started) break;
      continue;
    }
    started = true;
    buf.push(trimmed);
  }
  return buf.join(" ").slice(0, 200);
}

function stemName(filePath: string): string {
  const name = filePath.split("/").pop() ?? filePath;
  return name.replace(/\.md$/, "");
}

function inferComponent(filePath: string, basePath: string): string | undefined {
  const parts = filePath.split("/");
  const baseDepth = basePath ? basePath.split("/").length : 0;
  // file is at basePath/file.md → no component
  // file is at basePath/ComponentName/file.md → component = ComponentName
  if (parts.length <= baseDepth + 1) return undefined;
  return parts[baseDepth];
}
```

- [ ] **Step 4: Export from engine index**

In `packages/engine/src/index.ts`, add:
```typescript
export * from "./lib/github-adapter.js";
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bunx vitest run src/lib/github-adapter.spec.ts
```

Expected: all tests PASS

- [ ] **Step 6: Run full engine test suite**

```bash
bunx vitest run
```

Expected: all tests pass (no regressions)

- [ ] **Step 7: Commit**

```bash
git add packages/engine/src/lib/github-adapter.ts packages/engine/src/lib/github-adapter.spec.ts packages/engine/src/index.ts
git commit -m "feat(engine): add GithubAdapter for github.com tree URLs"
```

---

## Chunk 3: LibIndexer phase split + TransformersBackend

### Task 5: LibIndexer — optional backend (Phase 1 without embeddings)

**Files:**
- Modify: `packages/engine/src/lib/lib-indexer.ts`

- [ ] **Step 1: Write a failing test for optional backend**

Create `packages/engine/src/lib/lib-indexer.spec.ts`:

```typescript
// packages/engine/src/lib/lib-indexer.spec.ts
import { describe, it, expect, vi } from "vitest";
import { LibIndexer } from "./lib-indexer.js";
import type { ModelBackend, DocPage } from "@sensei/shared";

const pages: DocPage[] = [
  { title: "Button", description: "A button component", sourceType: "llms.txt", url: "https://x.com/btn" },
  { title: "Input",  description: "A text input",       sourceType: "llms.txt", url: "https://x.com/inp" },
];

function makeDb(rows: Record<string, unknown>[] = []) {
  const insertFn = vi.fn().mockResolvedValue({ error: null });
  const deleteFn = vi.fn().mockReturnValue({ eq: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }) });
  const deleteShared = vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) });
  return {
    from: vi.fn((table: string) => ({
      delete: table === "shared_lib_sections" ? () => ({ eq: vi.fn().mockResolvedValue({ error: null }) }) : deleteFn,
      insert: insertFn,
    })),
    _insertFn: insertFn,
  };
}

describe("LibIndexer.indexShared — no backend (Phase 1)", () => {
  it("inserts sections with null embedding when no backend provided", async () => {
    const db = makeDb() as any;
    const indexer = new LibIndexer(db, null);
    const { sectionsIndexed } = await indexer.indexShared("lib-id-123", { name: "mylib", source_type: "llms.txt" }, pages);

    expect(sectionsIndexed).toBe(2);
    const insertedRows = db._insertFn.mock.calls[0][0] as any[];
    expect(insertedRows).toHaveLength(2);
    expect(insertedRows[0].embedding).toBeNull();
    expect(insertedRows[0].title).toBe("Button");
  });

  it("inserts sections with embeddings when backend provided", async () => {
    const db = makeDb() as any;
    const backend: Partial<ModelBackend> = {
      embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
    };
    const indexer = new LibIndexer(db, backend as ModelBackend);
    await indexer.indexShared("lib-id-123", { name: "mylib", source_type: "llms.txt" }, pages);

    const insertedRows = db._insertFn.mock.calls[0][0] as any[];
    expect(insertedRows[0].embedding).toEqual([0.1, 0.2, 0.3]);
    expect(backend.embed).toHaveBeenCalledTimes(2);
  });
});
```

- [ ] **Step 2: Run test — confirm FAIL**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bunx vitest run src/lib/lib-indexer.spec.ts 2>&1 | tail -15
```

Expected: FAIL (LibIndexer constructor rejects null backend via TypeScript or embedding crashes)

- [ ] **Step 3: Update LibIndexer to accept optional backend**

Replace `packages/engine/src/lib/lib-indexer.ts` with:

```typescript
// packages/engine/src/lib/lib-indexer.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { LibEntry, DocPage, ModelBackend } from "@sensei/shared";

export class LibIndexer {
  constructor(
    private readonly db: SupabaseClient,
    private readonly backend: ModelBackend | null = null,
  ) {}

  async index(
    repoId: string,
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ sectionsIndexed: number }> {
    const { error: deleteError } = await this.db
      .from("lib_doc_sections")
      .delete()
      .eq("repo_id", repoId)
      .eq("lib_name", entry.name);

    if (deleteError) throw new Error(`LibIndexer: delete failed: ${deleteError.message}`);

    const rows = await Promise.all(
      pages.map(async page => {
        const embedding = await this.embedPage(entry, page);
        return {
          repo_id: repoId,
          lib_name: entry.name,
          title: page.title,
          url: page.url ?? null,
          local_path: page.localPath ?? null,
          description: page.description,
          content: page.content ?? null,
          source_type: entry.source_type,
          component: page.component ?? null,
          embedding,
        };
      })
    );

    const { error: insertError } = await this.db.from("lib_doc_sections").insert(rows);
    if (insertError) throw new Error(`LibIndexer: insert failed: ${insertError.message}`);

    return { sectionsIndexed: rows.length };
  }

  async indexShared(
    sharedLibId: string,
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ sectionsIndexed: number }> {
    const { error: deleteError } = await this.db
      .from("shared_lib_sections")
      .delete()
      .eq("shared_lib_id", sharedLibId);

    if (deleteError) throw new Error(`LibIndexer.indexShared: delete failed: ${deleteError.message}`);

    const rows = await Promise.all(
      pages.map(async page => {
        const embedding = await this.embedPage(entry, page);
        return {
          shared_lib_id: sharedLibId,
          title: page.title,
          url: page.url ?? null,
          local_path: page.localPath ?? null,
          description: page.description,
          content: page.content ?? null,
          source_type: entry.source_type,
          component: page.component ?? null,
          embedding,
        };
      })
    );

    const { error: insertError } = await this.db.from("shared_lib_sections").insert(rows);
    if (insertError) throw new Error(`LibIndexer.indexShared: insert failed: ${insertError.message}`);

    return { sectionsIndexed: rows.length };
  }

  private async embedPage(entry: LibEntry, page: DocPage): Promise<number[] | null> {
    if (!this.backend) return null;
    const input =
      entry.source_type === "llms.txt"
        ? page.description
        : (page.content ?? page.description).slice(0, 512);
    return this.backend.embed(input);
  }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
bunx vitest run src/lib/lib-indexer.spec.ts
```

Expected: all PASS

- [ ] **Step 5: Run full suite**

```bash
bunx vitest run
```

Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/lib-indexer.ts packages/engine/src/lib/lib-indexer.spec.ts
git commit -m "feat(engine): make LibIndexer backend optional — Phase 1 stores sections without embeddings"
```

---

### Task 6: TransformersBackend

**Files:**
- Create: `packages/engine/src/lib/transformers-backend.ts`
- Modify: `packages/engine/package.json`
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Add `@xenova/transformers` dependency**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bun add @xenova/transformers
```

- [ ] **Step 2: Write the implementation**

```typescript
// packages/engine/src/lib/transformers-backend.ts
/**
 * Embedding-only ModelBackend using @xenova/transformers.
 * Model: Xenova/all-MiniLM-L6-v2 (384-dim, ~23MB ONNX, downloads on first use).
 * Only embed() is implemented — other ModelBackend methods throw.
 */
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "@sensei/shared";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pipelineInstance: any = null;

async function getEmbeddingPipeline() {
  if (!pipelineInstance) {
    // Lazy import — avoids loading the ONNX runtime until first use
    const { pipeline } = await import("@xenova/transformers");
    pipelineInstance = await pipeline("feature-extraction", "Xenova/all-MiniLM-L6-v2");
  }
  return pipelineInstance;
}

export class TransformersBackend implements ModelBackend {
  readonly name = "transformers";

  async init(): Promise<void> {
    await getEmbeddingPipeline();
  }

  async isAvailable(): Promise<boolean> {
    return true;
  }

  async embed(text: string): Promise<number[]> {
    const extractor = await getEmbeddingPipeline();
    const result = await extractor(text, { pooling: "mean", normalize: true });
    return Array.from(result.data);
  }

  async generate(_prompt: string): Promise<string> {
    throw new Error("TransformersBackend: generate() not supported");
  }

  async extract(_content: string, _instructions: ExtractionInstructions): Promise<FileAnalysis> {
    throw new Error("TransformersBackend: extract() not supported");
  }
}
```

Note: The model (~23MB) downloads to `~/.cache/xenova/` on first call to `embed()`. Subsequent calls use the cache.

- [ ] **Step 3: Export from engine index**

In `packages/engine/src/index.ts`, add:
```typescript
export * from "./lib/transformers-backend.js";
```

- [ ] **Step 4: Verify compiles**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bun run tsc --noEmit 2>&1 | head -20
```

Expected: no errors

- [ ] **Step 5: Smoke test (manual — downloads model)**

```bash
cd /Users/Jerry/Developer/sensei/packages/engine
bun -e "
import { TransformersBackend } from './src/lib/transformers-backend.ts';
const b = new TransformersBackend();
const vec = await b.embed('hello world');
console.log('dims:', vec.length, 'sample:', vec.slice(0,3));
"
```

Expected: `dims: 384 sample: [<float>, <float>, <float>]`

(First run downloads the model — takes 10-30 seconds. Subsequent runs are instant.)

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/transformers-backend.ts packages/engine/src/index.ts packages/engine/package.json bun.lockb
git commit -m "feat(engine): add TransformersBackend (all-MiniLM-L6-v2, 384-dim) — removes Ollama dep for embedding"
```

---

## Chunk 4: Dashboard — Phase 1 fetch (remove Ollama)

### Task 7: Dashboard lib-indexer — Phase 1 only + embed phase

**Files:**
- Modify: `apps/dashboard/src/lib/server/lib-indexer.ts`

Replace the entire file:

- [ ] **Step 1: Rewrite lib-indexer.ts**

```typescript
// apps/dashboard/src/lib/server/lib-indexer.ts
/**
 * Two-phase library indexer for the dashboard.
 *
 * Phase 1 (startLibFetch): fetch + parse → store sections without embeddings.
 *   Runs on Add / Re-index. No Ollama required. Sections immediately keyword-searchable.
 *
 * Phase 2 (startLibEmbed): generate 384-dim embeddings via TransformersBackend.
 *   Runs on "Build Index". Enables semantic search in getLibDocsTool.
 */
import type { SupabaseClient } from '@supabase/supabase-js';
import { inferSourceType } from '@sensei/engine';

export interface LibInfo {
  id: string;
  name: string;
  source_type: string;
  base_url: string | null;
  local_path: string | null;
}

// ─── Phase 1: Fetch ──────────────────────────────────────────────────────────

export async function startLibFetch(db: SupabaseClient, lib: LibInfo): Promise<void> {
  await db
    .from('shared_libs')
    .update({ index_status: 'indexing', index_error: null, embed_status: null })
    .eq('id', lib.id);

  runFetch(db, lib).catch(err => {
    console.error(`[lib-indexer] fetch error for ${lib.name}:`, err);
  });
}

async function runFetch(db: SupabaseClient, lib: LibInfo): Promise<void> {
  try {
    const { LlmsTxtAdapter, HttpAdapter, LocalAdapter, GithubAdapter, LibIndexer } =
      await import('@sensei/engine');

    // Use the stored source_type (canonical) — do NOT re-derive from URL
    const sourceType = lib.source_type as 'llms.txt' | 'http' | 'local' | 'github';
    let adapter;
    if (sourceType === 'llms.txt') adapter = new LlmsTxtAdapter();
    else if (sourceType === 'github') adapter = new GithubAdapter();
    else if (sourceType === 'http') adapter = new HttpAdapter();
    else adapter = new LocalAdapter();

    const entry = {
      name: lib.name,
      source_type: sourceType,
      base_url: lib.base_url ?? undefined,
      local_path: lib.local_path ?? undefined,
    };

    const pages = await adapter.fetch(entry);

    // Phase 1: no backend → sections stored without embeddings
    await new LibIndexer(db, null).indexShared(lib.id, entry, pages);

    await db
      .from('shared_libs')
      .update({
        section_count: pages.length,
        indexed_at: new Date().toISOString(),
        index_status: 'ready',
        index_error: null,
      })
      .eq('id', lib.id);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[lib-indexer] fetch failed for ${lib.name}:`, msg);
    await db
      .from('shared_libs')
      .update({ index_status: 'error', index_error: msg })
      .eq('id', lib.id);
  }
}

// ─── Phase 2: Embed ──────────────────────────────────────────────────────────

export async function startLibEmbed(db: SupabaseClient, libId: string, libName: string): Promise<void> {
  await db
    .from('shared_libs')
    .update({ embed_status: 'embedding' })
    .eq('id', libId);

  runEmbed(db, libId, libName).catch(err => {
    console.error(`[lib-embedder] embed error for ${libName}:`, err);
  });
}

async function runEmbed(db: SupabaseClient, libId: string, libName: string): Promise<void> {
  try {
    const { TransformersBackend } = await import('@sensei/engine');
    const backend = new TransformersBackend();

    // Fetch sections that need embedding (NULL embedding)
    const { data: sections, error: fetchErr } = await db
      .from('shared_lib_sections')
      .select('id,description,content,source_type')
      .eq('shared_lib_id', libId)
      .is('embedding', null);

    if (fetchErr) throw new Error(fetchErr.message);
    if (!sections || sections.length === 0) {
      await db.from('shared_libs').update({ embed_status: 'ready' }).eq('id', libId);
      return;
    }

    // Embed and update in batches of 20
    const BATCH = 20;
    for (let i = 0; i < sections.length; i += BATCH) {
      const batch = sections.slice(i, i + BATCH);
      await Promise.all(
        batch.map(async (section: any) => {
          const input =
            section.source_type === 'llms.txt'
              ? section.description
              : (section.content ?? section.description).slice(0, 512);
          const embedding = await backend.embed(input);
          await db
            .from('shared_lib_sections')
            .update({ embedding })
            .eq('id', section.id);
        })
      );
    }

    await db.from('shared_libs').update({ embed_status: 'ready' }).eq('id', libId);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[lib-embedder] embed failed for ${libName}:`, msg);
    // Don't set index_status to error — sections are still usable via keyword search
    await db.from('shared_libs').update({ embed_status: null }).eq('id', libId);
  }
}

// Keep backward-compatible alias used by existing callers
export const startLibIndexing = startLibFetch;
```

- [ ] **Step 2: Verify dashboard compiles**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun run check 2>&1 | grep -E "error|Error" | head -20
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/lib/server/lib-indexer.ts
git commit -m "feat(dashboard): Phase 1 fetch removes Ollama — Phase 2 embed via TransformersBackend"
```

---

### Task 8: Dashboard server — `inferSourceType` + `embed` action

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/+page.server.ts`
- Modify: `apps/dashboard/src/routes/libraries/[id]/+page.server.ts`

- [ ] **Step 1: Update libraries `+page.server.ts`**

In `apps/dashboard/src/routes/libraries/+page.server.ts`, update the `add` action's `inferSourceType` inline function.

Replace the inline `inferSourceType` function call with the shared utility:

Add import at top of file:
```typescript
import { inferSourceType } from '@sensei/engine';
```

Then in the `add` action, replace the local `inferSourceType` invocation block (wherever it builds `source_type`, `base_url`, `local_path`) with:
```typescript
const inferred = inferSourceType(url);
const { source_type } = inferred;
const base_url = 'base_url' in inferred ? inferred.base_url : null;
const local_path = 'local_path' in inferred ? inferred.local_path : null;
```

- [ ] **Step 2: Update library detail `+page.server.ts`**

In `apps/dashboard/src/routes/libraries/[id]/+page.server.ts`:

1. Add import:
```typescript
import { inferSourceType } from '@sensei/engine';
import { startLibFetch, startLibEmbed } from '$lib/server/lib-indexer';
```

2. Replace the inline `inferSourceType` function in the `edit` action with the same shared utility call pattern as Step 1.

3. Update existing `edit` and `reindex` call sites to use `startLibFetch` (replacing the old `startLibIndexing` alias calls in lines ~106 and ~147):

```typescript
// In the edit action (line ~106):
if (lib) {
  await startLibFetch(db, lib as LibInfo);
}

// In the reindex action (line ~147):
await startLibFetch(db, lib as LibInfo);
return { reindexing: true };
```

4. Add `embed` action:
```typescript
embed: async ({ params }) => {
  const db = getDb();
  const { data: lib } = await db
    .from('shared_libs')
    .select('id,name')
    .eq('id', params.id)
    .single();

  if (!lib) return fail(404, { error: 'Library not found' });

  await startLibEmbed(db, lib.id, lib.name);
  return { embedding: true };
},
```

6. Update `load` to include `embed_status` in the select:
```typescript
.select('id,name,source_type,base_url,local_path,section_count,indexed_at,index_status,index_error,created_at,icon_url,category,embed_status')
```

And update the return type annotation to include `embed_status: string | null`.

- [ ] **Step 3: Verify compiles**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun run check 2>&1 | grep -E "error|Error" | head -20
```

Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/libraries/+page.server.ts apps/dashboard/src/routes/libraries/[id]/+page.server.ts
git commit -m "feat(dashboard): use shared inferSourceType, add embed action on library detail"
```

---

## Chunk 5: Dashboard UI — embed button, local lib indicator

### Task 9: Library detail page UI updates

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/[id]/+page.svelte`

Changes needed:
1. Poll while `embed_status === 'embedding'`
2. "Build Index" button (triggers `?/embed`)
3. Disable Re-index button for `source_type = 'local'` (with tooltip)
4. Show embed status in stat row

- [ ] **Step 1: Update the `$effect` to also poll during embedding**

Find the existing `$effect` for polling (around line 50-54):
```typescript
// Before:
$effect(() => {
  if (data.lib.index_status !== 'indexing') return;
  const id = setInterval(() => invalidateAll(), 3000);
  return () => clearInterval(id);
});

// After:
$effect(() => {
  const busy = data.lib.index_status === 'indexing' || data.lib.embed_status === 'embedding';
  if (!busy) return;
  const id = setInterval(() => invalidateAll(), 3000);
  return () => clearInterval(id);
});
```

- [ ] **Step 2: Add `embedding: true` handler in the `$effect` for form reactions**

```typescript
$effect(() => {
  const f = form as any;
  if (f?.results) { simResults = f.results; lastQuery = f.query ?? ''; }
  if (f?.edited) sidebarMode = null;
  // embed action closes sidebar if open
});
```

- [ ] **Step 3: Update the action buttons section**

Find the buttons row (around line 244-266) and update:

```svelte
<div class="flex items-center gap-2 shrink-0">
  <button
    onclick={() => sidebarMode = 'edit'}
    class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors"
  >
    Edit
  </button>

  {#if data.lib.source_type === 'local'}
    <span
      title="Re-index via CLI: sensei index"
      class="text-xs px-3 py-1.5 rounded border border-surface-z2 bg-surface-z1 text-surface-z4 cursor-not-allowed"
    >
      Re-index (CLI only)
    </span>
  {:else}
    <form method="POST" action="?/reindex">
      <button
        type="submit"
        disabled={data.lib.index_status === 'indexing'}
        class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {data.lib.index_status === 'indexing' ? 'Indexing…' : 'Re-index'}
      </button>
    </form>
  {/if}

  {#if data.lib.embed_status !== 'ready'}
    <form method="POST" action="?/embed">
      <button
        type="submit"
        disabled={data.lib.index_status === 'indexing' || data.lib.embed_status === 'embedding' || data.lib.section_count === 0}
        class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        title="Generate embeddings for semantic search"
      >
        {data.lib.embed_status === 'embedding' ? 'Embedding…' : 'Build Index'}
      </button>
    </form>
  {:else}
    <span class="text-xs px-2 py-1 rounded bg-success-z2 text-success-z6 border border-success-z3">
      Indexed
    </span>
  {/if}

  <button
    onclick={() => sidebarMode = 'simulate'}
    class="text-xs px-3 py-1.5 rounded border border-primary-z4 bg-surface-z1 text-primary-z6 hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
  >
    Simulate
  </button>
</div>
```

- [ ] **Step 4: Update type annotation for `data.lib`**

In the `<script>` section, the `data` is typed via `PageData`. Since the server file now returns `embed_status`, the generated types will include it automatically after `bun run check`.

- [ ] **Step 5: Verify in browser**

Start dev server:
```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun run dev
```

Navigate to a library detail page and verify:
- "Build Index" button appears when `embed_status !== 'ready'`
- Local libs show "Re-index (CLI only)" greyed out
- After clicking "Build Index", button changes to "Embedding…"
- After embedding completes, "Indexed" badge appears

- [ ] **Step 6: Commit**

```bash
git add apps/dashboard/src/routes/libraries/[id]/+page.svelte
git commit -m "feat(dashboard): add Build Index button, disable Re-index for local libs"
```

---

### Task 10: Libraries list page — remove local path input, GitHub detection

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/+page.svelte`

- [ ] **Step 1: Verify the Add Library sidebar**

Read the sidebar section in `+page.svelte` to find the URL/path input and any local path handling. The input label should say "Doc URL" (not "URL or path") since local path is CLI-only.

- [ ] **Step 2: Update label and placeholder in Add Library sidebar**

Find the URL input in the Add Library sidebar form and update:
```svelte
<!-- Before: -->
<span class="text-xs font-medium text-surface-z6">URL or local path</span>
<input ... placeholder="https://docs.example.com or /path/to/docs" ... />

<!-- After: -->
<span class="text-xs font-medium text-surface-z6">Doc URL</span>
<input ... placeholder="https://docs.example.com/llms.txt or https://github.com/org/repo/tree/main/docs" ... />
```

Also add a hint below the input:
```svelte
<span class="text-xs text-surface-z4">
  Supports llms.txt URLs, GitHub folder URLs, and HTTP doc pages.
  Local folders must be indexed via CLI: <code>sensei index</code>
</span>
```

- [ ] **Step 3: Verify compiles**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun run check 2>&1 | grep -E "error|Error" | head -10
```

Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/libraries/+page.svelte
git commit -m "feat(dashboard): update Add Library form — URL only, GitHub hint, remove local path"
```

---

## Chunk 6: getLibDocsTool — keyword fallback

### Task 11: Keyword fallback when no embeddings

**Files:**
- Modify: `packages/server/src/tools/get-lib-docs.ts`

Currently the tool calls `match_shared_lib_sections` RPC even when sections have NULL embeddings. The RPC filters out NULL-embedding rows (we added `and embedding is not null` in the migration), so it will return 0 results — but silently. We want to fall back to keyword ILIKE search in that case.

- [ ] **Step 1: Write a failing test**

Create `packages/server/src/tools/get-lib-docs-fallback.spec.ts`:

```typescript
// packages/server/src/tools/get-lib-docs-fallback.spec.ts
import { describe, it, expect, vi } from "vitest";
import { getLibDocsTool } from "./get-lib-docs.js";

function makeDb(overrides: Record<string, unknown> = {}) {
  const rpcFn = vi.fn().mockResolvedValue({ data: [], error: null });
  const fromResult = {
    select: vi.fn().mockReturnThis(),
    eq: vi.fn().mockReturnThis(),
    maybeSingle: vi.fn().mockResolvedValue({ data: { shared_lib_id: "shared-123" } }),
    or: vi.fn().mockReturnThis(),
    order: vi.fn().mockReturnThis(),
    // limit must be defined — keyword fallback calls .order(...).limit(N) and awaits the result
    limit: vi.fn().mockResolvedValue({ data: [], error: null }),
    is: vi.fn().mockReturnThis(),
  };
  return {
    rpc: rpcFn,
    from: vi.fn().mockReturnValue(fromResult),
    schema: vi.fn().mockReturnValue({ from: vi.fn().mockReturnValue(fromResult) }),
    _rpcFn: rpcFn,
    _fromResult: fromResult,
    ...overrides,
  };
}

describe("getLibDocsTool — keyword fallback", () => {
  it("falls back to ILIKE search when vector RPC returns 0 results", async () => {
    const keywordRows = [
      { title: "Button", url: "https://x.com/btn", description: "A button", content: null, source_type: "llms.txt", component: null, local_path: null },
    ];
    const db = makeDb();
    // RPC returns empty (no embeddings) — already the default mock
    // keyword path: .or() → .order() → .limit() resolves with data
    db._fromResult.limit.mockResolvedValue({ data: keywordRows, error: null });

    const backend = { embed: vi.fn().mockResolvedValue([0.1, 0.2]) } as any;
    const result = await getLibDocsTool(db as any, backend, "repo-id", "mylib", { query: "button" });

    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
  });

  it("uses vector search when RPC returns results", async () => {
    const vectorRows = [
      { title: "Vector Result", url: null, description: "Found via vector", content: null, source_type: "llms.txt", component: null, local_path: null, similarity: 0.9 },
    ];
    const db = makeDb();
    db._rpcFn.mockResolvedValue({ data: vectorRows, error: null });

    const backend = { embed: vi.fn().mockResolvedValue([0.1, 0.2]) } as any;
    const result = await getLibDocsTool(db as any, backend, "repo-id", "mylib", { query: "vector" });

    expect(result.sections[0].title).toBe("Vector Result");
  });
});
```

- [ ] **Step 2: Run test — confirm FAIL**

```bash
cd /Users/Jerry/Developer/sensei/packages/server
bunx vitest run src/tools/get-lib-docs-fallback.spec.ts 2>&1 | tail -15
```

Expected: FAIL (fallback not yet implemented)

- [ ] **Step 3: Update `getLibDocsTool` to add keyword fallback**

In `packages/server/src/tools/get-lib-docs.ts`, find the semantic search block (lines 34-55):

```typescript
if (opts?.query) {
  const embedding = await backend.embed(opts.query);
  if (sharedLibId) {
    const { data, error } = await db.rpc("match_shared_lib_sections", { ... });
    if (error) throw new Error(error.message);
    rows = (data ?? []) as Record<string, unknown>[];
  } else { ... }
}
```

Replace the `sharedLibId` branch with:
```typescript
if (sharedLibId) {
  const embedding = await backend.embed(opts.query);
  const { data, error } = await db.rpc("match_shared_lib_sections", {
    p_shared_lib_id: sharedLibId,
    p_component: opts.component ?? null,
    query_embedding: embedding,
    match_count: limit,
  });
  if (error) throw new Error(error.message);
  rows = (data ?? []) as Record<string, unknown>[];

  // Fallback to keyword search if no embeddings exist yet
  if (rows.length === 0) {
    const q = opts.query;
    let kq = (db as any)
      .schema('sensei')
      .from('shared_lib_sections')
      .select('title,url,local_path,description,content,source_type,component')
      .eq('shared_lib_id', sharedLibId)
      .or(`title.ilike.%${q}%,description.ilike.%${q}%`);
    if (opts.component) kq = kq.eq('component', opts.component);
    const { data: kData, error: kErr } = await kq.order('title').limit(limit);
    if (kErr) throw new Error(kErr.message);
    rows = (kData ?? []) as Record<string, unknown>[];
  }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd /Users/Jerry/Developer/sensei/packages/server
bunx vitest run src/tools/get-lib-docs-fallback.spec.ts
```

Expected: all PASS

- [ ] **Step 5: Run full server test suite**

```bash
bunx vitest run
```

Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add packages/server/src/tools/get-lib-docs.ts packages/server/src/tools/get-lib-docs-fallback.spec.ts
git commit -m "feat(server): keyword fallback in getLibDocsTool when no embeddings present"
```

---

---

## Chunk 7: Shared AddLibrarySidebar + repo link integration

### Task 12: Extract AddLibrarySidebar component

**Goal:** Reuse the Add Library sidebar in both `/libraries` and `/repos/[id]/libraries` without duplicating UI.

**Files:**
- Create: `apps/dashboard/src/lib/components/AddLibrarySidebar.svelte`
- Modify: `apps/dashboard/src/routes/libraries/+page.svelte` (use component)
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte` (add sidebar)
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts` (add `link` action)

The component accepts `action` (form action URL) and `open` (boolean). The parent controls open state. When used from the repo page, the form posts to `?/add` on the repo route which handles both config.yaml update AND `repo_libs` linking.

- [ ] **Step 1: Create the shared component**

```svelte
<!-- apps/dashboard/src/lib/components/AddLibrarySidebar.svelte -->
<script lang="ts">
  import { enhance } from '$app/forms';

  interface Props {
    open: boolean;
    action: string;
    onclose: () => void;
  }

  const { open, action, onclose }: Props = $props();
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/40 z-40"
    role="presentation"
    onclick={onclose}
  ></div>

  <div class="fixed right-0 top-0 h-full w-96 bg-surface-z1 border-l border-surface-z3 z-50 flex flex-col shadow-xl">
    <div class="flex items-center justify-between px-5 py-4 border-b border-surface-z3 sticky top-0 bg-surface-z1">
      <h2 class="text-sm font-semibold text-surface-z8">Add Library</h2>
      <button onclick={onclose} class="text-surface-z5 hover:text-surface-z8 transition-colors" aria-label="Close">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18">
          <path d="M18 6L6 18M6 6l12 12" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <form method="POST" {action} use:enhance class="flex flex-col gap-4 px-5 py-5 flex-1">
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Name <span class="text-error-z6">*</span></span>
        <input
          type="text"
          name="name"
          required
          placeholder="e.g. rokkit"
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        />
      </label>
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Doc URL <span class="text-error-z6">*</span></span>
        <input
          type="text"
          name="url"
          required
          placeholder="https://docs.example.com/llms.txt or https://github.com/org/repo/tree/main/docs"
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        />
        <span class="text-xs text-surface-z4">
          Supports llms.txt URLs, GitHub folder URLs, and HTTP doc pages.
          Local folders must be indexed via CLI: <code>sensei index</code>
        </span>
      </label>
      <div class="flex gap-2 mt-auto">
        <button
          type="submit"
          class="flex-1 px-4 py-2 rounded bg-primary-z6 text-white text-sm font-medium hover:bg-primary-z7 transition-colors"
        >
          Add & Index
        </button>
        <button
          type="button"
          onclick={onclose}
          class="px-4 py-2 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 text-sm hover:border-primary-z5 transition-colors"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
{/if}
```

- [ ] **Step 2: Update `/libraries/+page.svelte` to use the component**

Replace the inline Add Library sidebar in `/libraries/+page.svelte` with:
```svelte
<script lang="ts">
  import AddLibrarySidebar from '$lib/components/AddLibrarySidebar.svelte';
  // ... existing imports
</script>

<!-- Replace the existing inline sidebar with: -->
<AddLibrarySidebar open={sidebarOpen} action="?/add" onclose={() => sidebarOpen = false} />
```

- [ ] **Step 3: Add `link` action to repo libraries server**

In `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`, add a `link` action that links an existing shared lib to the repo:

```typescript
link: async ({ params, request }) => {
  const db = getDb();
  const formData = await request.formData();
  const sharedLibId = String(formData.get('shared_lib_id') ?? '').trim();
  if (!sharedLibId) return fail(400, { error: 'shared_lib_id is required' });

  const { data: lib } = await db
    .from('shared_libs')
    .select('id,name,source_type,base_url,local_path')
    .eq('id', sharedLibId)
    .single();
  if (!lib) return fail(404, { error: 'Library not found in catalog' });

  await db.from('repo_libs').upsert({
    repo_id: params.id,
    shared_lib_id: lib.id,
    name: lib.name,
    source_type: lib.source_type,
    base_url: lib.base_url ?? null,
    local_path: lib.local_path ?? null,
  }, { onConflict: 'repo_id,name' });

  return { linked: true };
},
```

- [ ] **Step 4: Update repo libraries page load to expose catalog**

In `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`, add to `load`:
```typescript
// Fetch full catalog for linking (excluding already-linked libs)
const linkedIds = new Set(libs.filter(l => l.sharedLibId).map(l => l.sharedLibId!));
const { data: catalog } = await db
  .from('shared_libs')
  .select('id,name,source_type,icon_url,category,section_count,index_status')
  .order('name');

return {
  repo,
  libs,
  hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  catalog: (catalog ?? []).map(c => ({ ...c, linked: linkedIds.has(c.id) })),
};
```

- [ ] **Step 5: Update repo libraries page Svelte**

In `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte`:

1. Import the shared component and add a catalog section:
```svelte
<script lang="ts">
  import AddLibrarySidebar from '$lib/components/AddLibrarySidebar.svelte';
  // ... existing

  let addOpen = $state(false);
  let catalogSearch = $state('');

  const filteredCatalog = $derived(
    catalogSearch.trim()
      ? data.catalog.filter((l: any) =>
          l.name.toLowerCase().includes(catalogSearch.toLowerCase())
        )
      : data.catalog
  );
</script>

<AddLibrarySidebar open={addOpen} action="?/add" onclose={() => addOpen = false} />
```

2. Add "Add Library" and catalog section to the page header:
```svelte
<div class="flex items-center justify-between mb-6">
  <h1 class="text-2xl font-semibold text-surface-z8">Library Docs</h1>
  <div class="flex gap-2">
    <form method="POST" action="?/update">
      <button type="submit" class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors">
        Re-index All
      </button>
    </form>
    <button
      onclick={() => addOpen = true}
      class="text-xs px-3 py-1.5 rounded border border-primary-z4 bg-surface-z1 text-primary-z6 hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
    >
      + Add Library
    </button>
  </div>
</div>
```

3. Below the existing libs table, add a catalog section:
```svelte
<!-- Catalog: link existing shared libs to this repo -->
{#if data.catalog.length > 0}
  <div class="mt-8">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Library Catalog</h2>
      <input
        type="search"
        placeholder="Search catalog…"
        bind:value={catalogSearch}
        class="px-3 py-1.5 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-xs focus:border-primary-z5 focus:outline-none w-44"
      />
    </div>
    <div class="rounded-lg border border-surface-z3 overflow-hidden">
      <table class="w-full border-collapse text-sm">
        <tbody>
          {#each filteredCatalog as lib}
            <tr class="border-b border-surface-z2 last:border-b-0 hover:bg-surface-z2 transition-colors">
              <td class="px-4 py-3">
                <a href="/libraries/{lib.id}" class="font-medium text-primary-z6 hover:text-primary-z7 transition-colors">
                  {lib.name}
                </a>
              </td>
              <td class="px-4 py-3 text-surface-z5 text-xs">{lib.section_count} sections</td>
              <td class="px-4 py-3 text-right">
                {#if lib.linked}
                  <span class="text-xs text-success-z6">✓ Linked</span>
                {:else}
                  <form method="POST" action="?/link" class="inline">
                    <input type="hidden" name="shared_lib_id" value={lib.id} />
                    <button type="submit" class="text-xs px-2 py-1 rounded border border-surface-z3 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors">
                      Link
                    </button>
                  </form>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
{/if}
```

- [ ] **Step 6: Verify compiles**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun run check 2>&1 | grep -E "error|Error" | head -20
```

Expected: no errors

- [ ] **Step 7: Commit**

```bash
git add apps/dashboard/src/lib/components/AddLibrarySidebar.svelte \
        apps/dashboard/src/routes/libraries/+page.svelte \
        apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts \
        apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte
git commit -m "feat(dashboard): shared AddLibrarySidebar component + repo catalog link action"
```

---

## Chunk 8: Playwright e2e tests

### Task 13: Playwright setup + libraries page smoke tests

**Files:**
- Modify: `apps/dashboard/package.json` (add Playwright)
- Create: `apps/dashboard/playwright.config.ts`
- Create: `apps/dashboard/tests/libraries.test.ts`
- Create: `apps/dashboard/tests/library-detail.test.ts`

- [ ] **Step 1: Install Playwright**

```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun add -d @playwright/test
bunx playwright install chromium
```

- [ ] **Step 2: Create playwright config**

```typescript
// apps/dashboard/playwright.config.ts
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
  webServer: {
    command: 'bun run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
```

- [ ] **Step 3: Add test script to package.json**

In `apps/dashboard/package.json`, add to `"scripts"`:
```json
"test:e2e": "playwright test"
```

- [ ] **Step 4: Write libraries page tests**

```typescript
// apps/dashboard/tests/libraries.test.ts
import { test, expect } from '@playwright/test';

test.describe('Libraries page', () => {
  test('loads and shows grid view by default', async ({ page }) => {
    await page.goto('/libraries');
    // Grid view toggle button visible
    await expect(page.locator('button[aria-label="Table view"], button[title*="table"], button[title*="Table"]').or(
      page.locator('button').filter({ hasText: /table/i }).first()
    )).toBeVisible({ timeout: 10_000 });
  });

  test('Add Library button opens sidebar', async ({ page }) => {
    await page.goto('/libraries');
    const addBtn = page.locator('button').filter({ hasText: /add library/i }).first();
    await expect(addBtn).toBeVisible({ timeout: 10_000 });
    await addBtn.click();
    // Sidebar form appears
    await expect(page.locator('input[name="name"]')).toBeVisible();
    await expect(page.locator('input[name="url"]')).toBeVisible();
  });

  test('Add Library sidebar closes on Cancel', async ({ page }) => {
    await page.goto('/libraries');
    await page.locator('button').filter({ hasText: /add library/i }).first().click();
    await page.locator('button').filter({ hasText: /cancel/i }).click();
    await expect(page.locator('input[name="name"]')).not.toBeVisible();
  });

  test('Add Library sidebar closes on overlay click', async ({ page }) => {
    await page.goto('/libraries');
    await page.locator('button').filter({ hasText: /add library/i }).first().click();
    await expect(page.locator('input[name="name"]')).toBeVisible();
    // Click overlay (outside sidebar)
    await page.mouse.click(100, 300);
    await expect(page.locator('input[name="name"]')).not.toBeVisible();
  });

  test('library cards link to detail page', async ({ page }) => {
    await page.goto('/libraries');
    const firstCard = page.locator('a[href^="/libraries/"]').first();
    const count = await firstCard.count();
    if (count === 0) {
      test.skip(true, 'No libraries in DB — skipping navigation test');
      return;
    }
    const href = await firstCard.getAttribute('href');
    await firstCard.click();
    await expect(page).toHaveURL(href!);
  });
});
```

- [ ] **Step 5: Write library detail page tests**

```typescript
// apps/dashboard/tests/library-detail.test.ts
import { test, expect, type Page } from '@playwright/test';

async function goToFirstLib(page: Page): Promise<boolean> {
  await page.goto('/libraries');
  const firstCard = page.locator('a[href^="/libraries/"]').first();
  if (await firstCard.count() === 0) return false;
  await firstCard.click();
  await page.waitForURL(/\/libraries\/[a-z0-9-]+$/);
  return true;
}

test.describe('Library detail page', () => {
  test('shows Edit, Re-index / Re-index CLI only, and Simulate buttons', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await expect(page.locator('button').filter({ hasText: /edit/i }).first()).toBeVisible();
    await expect(page.locator('button').filter({ hasText: /simulate/i }).first()).toBeVisible();
    // Either Re-index or Re-index (CLI only) visible
    const reindex = page.locator('button, span').filter({ hasText: /re-index/i }).first();
    await expect(reindex).toBeVisible();
  });

  test('Simulate button opens sidebar', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await page.locator('button').filter({ hasText: /simulate/i }).first().click();
    await expect(page.locator('textarea[name="query"]')).toBeVisible();
  });

  test('Edit button opens edit sidebar', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await page.locator('button').filter({ hasText: /^edit$/i }).first().click();
    await expect(page.locator('input[name="url"]')).toBeVisible();
    await expect(page.locator('select[name="category"]')).toBeVisible();
  });

  test('closing Simulate sidebar hides textarea', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await page.locator('button').filter({ hasText: /simulate/i }).first().click();
    await expect(page.locator('textarea[name="query"]')).toBeVisible();

    // Close via X button
    await page.locator('button[aria-label="Close"]').first().click();
    await expect(page.locator('textarea[name="query"]')).not.toBeVisible();
  });

  test('stat cards display Sections, Repos, Queries, Last Indexed', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await expect(page.getByText('Sections', { exact: true })).toBeVisible();
    await expect(page.getByText('Repos', { exact: true })).toBeVisible();
    await expect(page.getByText('Queries', { exact: true })).toBeVisible();
    await expect(page.getByText('Last Indexed', { exact: true })).toBeVisible();
  });
});
```

- [ ] **Step 6: Run e2e tests**

First ensure dev server is NOT already running (the config will start it):
```bash
cd /Users/Jerry/Developer/sensei/apps/dashboard
bun run test:e2e 2>&1 | tail -30
```

Note: Tests that navigate to detail pages will be skipped if no libraries exist in the DB. They are designed to skip gracefully rather than fail.

- [ ] **Step 7: Commit**

```bash
git add apps/dashboard/playwright.config.ts \
        apps/dashboard/tests/ \
        apps/dashboard/package.json
git commit -m "feat(dashboard): add Playwright e2e tests for libraries and library detail pages"
```

---

## TODO (future work)

- Add optional `GITHUB_TOKEN` env var to `GithubAdapter` for authenticated requests (60 req/hr → 5000 req/hr)
- Add `sensei embed-libs [name]` CLI command to trigger Phase 2 from command line
- Show "Build Index" progress (N of M sections embedded) via polling
- Per-repo `lib_doc_sections` embedding dimension migration (currently still 768-dim if used)
- Repo libraries page: Rokkit `List` + `SearchFilter` components for catalog browsing (defer — test sensei doc lookup in a future session)
