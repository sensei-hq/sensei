# file:// URL Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the separate `local_path` field on library entries with `file://` URLs stored in `base_url`, making all library sources uniformly URL-addressed.

**Architecture:** Normalize absolute local paths to `file://` URLs at the point of user input (`inferSourceType`), store them in the existing `base_url` column, and have adapters dispatch on URL scheme. DB migration populates `base_url` from `local_path` then drops the `local_path` columns from the two library tables. The `documents_in_library.local_path` column is kept — it's document-level output (where fetched content was read from), not source configuration.

**Tech Stack:** TypeScript, Vitest, SvelteKit, Supabase SQL migrations

---

## Chunk 1: Core types and inferSourceType

### Task 1: Update `inferSourceType` in engine to emit `file://` URLs

**Files:**
- Modify: `packages/engine/src/lib/infer-source-type.ts`
- Modify: `packages/engine/src/lib/infer-source-type.spec.ts`

The function currently returns `{ source_type: 'local', local_path: input }` for non-URL inputs. After this change it returns `{ source_type: 'local', base_url: 'file://' + input }`. The `InferredSource` discriminated union drops the `local_path` variant — all four source types use only `base_url`.

Also handle:
- Input already starting with `file://` → pass through, detect `.txt` for llms.txt type
- Input ending with `.txt` and being an absolute path → `llms.txt` source type with `file://` base_url

- [ ] **Step 1: Write the failing tests**

Replace the "detects local path" test and add new cases in `infer-source-type.spec.ts`:

```typescript
it("converts absolute path to file:// URL with source_type local", () => {
  const r = inferSourceType("/Users/jerry/projects/mylib/docs");
  expect(r.source_type).toBe("local");
  expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/docs");
  expect((r as any).local_path).toBeUndefined();
});

it("converts absolute .txt path to file:// URL with source_type llms.txt", () => {
  const r = inferSourceType("/Users/jerry/projects/mylib/docs/llms.txt");
  expect(r.source_type).toBe("llms.txt");
  expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/docs/llms.txt");
});

it("passes through file:// URL as local source type", () => {
  const r = inferSourceType("file:///Users/jerry/projects/mylib/docs");
  expect(r.source_type).toBe("local");
  expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/docs");
});

it("passes through file:// .txt URL as llms.txt source type", () => {
  const r = inferSourceType("file:///Users/jerry/projects/mylib/llms.txt");
  expect(r.source_type).toBe("llms.txt");
  expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/llms.txt");
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Users/Jerry/Developer/sensei
pnpm --filter @sensei/engine test src/lib/infer-source-type.spec.ts
```

Expected: 4 new tests FAIL

- [ ] **Step 3: Update `InferredSource` type and `inferSourceType`**

```typescript
// packages/engine/src/lib/infer-source-type.ts
export type InferredSource =
  | { source_type: 'llms.txt'; base_url: string }
  | { source_type: 'github';   base_url: string }
  | { source_type: 'http';     base_url: string }
  | { source_type: 'local';    base_url: string };

/**
 * Infer source_type and base_url from a user-provided URL or path.
 *
 * Detection order:
 *  1. file:// URL ending in .txt           → llms.txt
 *  2. file:// URL                          → local
 *  3. http(s) URL ending in .txt           → llms.txt
 *  4. github.com/{owner}/{repo}/tree/*     → github
 *  5. Any other http(s) URL               → http
 *  6. Absolute path ending in .txt         → llms.txt (file://)
 *  7. Anything else                        → local   (file://)
 */
export function inferSourceType(input: string): InferredSource {
  // Already a file:// URL
  if (input.startsWith('file://')) {
    return { source_type: input.endsWith('.txt') ? 'llms.txt' : 'local', base_url: input };
  }

  // HTTP/HTTPS URLs
  if (input.startsWith('http://') || input.startsWith('https://')) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith('/llms.txt') || url.pathname.endsWith('.txt')) {
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

  // Absolute filesystem path — normalize to file:// URL
  const fileUrl = input.startsWith('/') ? `file://${input}` : `file:///${input}`;
  return { source_type: input.endsWith('.txt') ? 'llms.txt' : 'local', base_url: fileUrl };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
pnpm --filter @sensei/engine test src/lib/infer-source-type.spec.ts
```

Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/infer-source-type.ts packages/engine/src/lib/infer-source-type.spec.ts
git commit -m "feat(engine): inferSourceType emits file:// URLs instead of local_path"
```

---

### Task 2: Update CLI `inferSourceType` and `detect-libs` spec

**Files:**
- Modify: `packages/cli/src/lib/detect-libs.ts`
- Modify: `packages/cli/src/lib/detect-libs.spec.ts`

The CLI has its own copy of `inferSourceType` in `detect-libs.ts` (line 47) that still returns `local_path`. It needs the same treatment as the engine version. The spec tests `inferSourceType("/docs").local_path` directly — update it to test `base_url`.

- [ ] **Step 1: Update failing tests in `detect-libs.spec.ts`**

Replace the last two tests in the `inferSourceType` describe block:

```typescript
it("detects local path", () => {
  expect(inferSourceType("/home/user/mylib/docs").source_type).toBe("local");
  expect(inferSourceType("./docs").source_type).toBe("local");
});

it("returns base_url for all source types — no local_path", () => {
  expect(inferSourceType("https://rokkit.dev/llms.txt").base_url).toBe("https://rokkit.dev/llms.txt");
  expect(inferSourceType("/docs").base_url).toBe("file:///docs");
  expect((inferSourceType("/docs") as any).local_path).toBeUndefined();
});
```

- [ ] **Step 2: Run to verify they fail**

```bash
pnpm --filter @sensei/cli test src/lib/detect-libs.spec.ts
```

- [ ] **Step 3: Update `detect-libs.ts`**

```typescript
// packages/cli/src/lib/detect-libs.ts
// Replace the inferSourceType function (lines 43-58):

/**
 * Infer source_type and base_url from a user-provided string.
 * Local paths are normalized to file:// URLs.
 * Evaluation order: file:// → llms.txt URL → github URL → http URL → local path.
 */
export function inferSourceType(input: string): Pick<LibEntry, "source_type" | "base_url"> {
  if (input.startsWith('file://')) {
    return { source_type: input.endsWith('.txt') ? 'llms.txt' : 'local', base_url: input };
  }
  if (input.startsWith("http://") || input.startsWith("https://")) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith("/llms.txt") || url.pathname.endsWith('.txt')) {
        return { source_type: "llms.txt", base_url: input };
      }
      if (url.hostname === 'github.com' && /^\/[^/]+\/[^/]+\/tree\//.test(url.pathname)) {
        return { source_type: 'github', base_url: input };
      }
    } catch { /* malformed URL — fall through */ }
    return { source_type: "http", base_url: input };
  }
  // Absolute filesystem path
  const fileUrl = input.startsWith('/') ? `file://${input}` : `file:///${input}`;
  return { source_type: input.endsWith('.txt') ? 'llms.txt' : 'local', base_url: fileUrl };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
pnpm --filter @sensei/cli test src/lib/detect-libs.spec.ts
```

- [ ] **Step 5: Commit**

```bash
git add packages/cli/src/lib/detect-libs.ts packages/cli/src/lib/detect-libs.spec.ts
git commit -m "feat(cli): inferSourceType emits file:// URLs instead of local_path"
```

---

### Task 3: Update `LibEntry` type and `LibEntrySchema`

**Files:**
- Modify: `packages/shared/src/types.ts`
- Modify: `packages/shared/src/config.ts`

`LibEntry.local_path` is removed; `base_url` is now required. The `LibEntrySchema` Zod schema in `config.ts` needs matching changes. It also needs backward-compatibility handling: existing `.sensei/config.yaml` files may still contain `local_path` entries (written before this refactor). The config loader coerces them to `file://` base_url on read.

- [ ] **Step 1: Update `LibEntry` in `packages/shared/src/types.ts`**

```typescript
// BEFORE
export interface LibEntry {
  name: string;
  source_type: 'llms.txt' | 'http' | 'local' | 'github';
  base_url?: string;
  local_path?: string;
}

// AFTER
export interface LibEntry {
  name: string;
  source_type: 'llms.txt' | 'http' | 'local' | 'github';
  base_url: string;        // All sources: http(s)://, file://, or https://github.com/...
}
```

- [ ] **Step 2: Update `LibEntrySchema` and `loadSenseiConfig` in `packages/shared/src/config.ts`**

The schema keeps `local_path` as optional for backward-compat parsing, but the loader coerces it:

```typescript
// packages/shared/src/config.ts

const LibEntrySchema = z.object({
  name: z.string(),
  source_type: z.enum(['llms.txt', 'http', 'local', 'github']),
  base_url: z.string().optional(),
  local_path: z.string().optional(),  // legacy — coerced to base_url on load
  description: z.string().optional(),
});

// Inside loadSenseiConfig, after parsing custom_libs:
if (Array.isArray(parsed?.custom_libs)) {
  const raw = z.array(LibEntrySchema).parse(parsed.custom_libs);
  custom_libs = raw.map(entry => {
    // Coerce legacy local_path to file:// base_url
    if (!entry.base_url && entry.local_path) {
      const fileUrl = entry.local_path.startsWith('/')
        ? `file://${entry.local_path}`
        : `file:///${entry.local_path}`;
      return { ...entry, base_url: fileUrl, local_path: undefined } as LibEntry;
    }
    return entry as LibEntry;
  });
}
```

- [ ] **Step 3: Run engine tests to see what breaks**

```bash
pnpm --filter @sensei/engine test 2>&1 | grep -E "FAIL|Error" | head -20
```

This is a diagnostic step — note failures. Do NOT commit yet. Adapter fixes come next.

- [ ] **Step 4: Commit the type + schema changes**

Wait until after Tasks 4 and 5 fix the adapters before committing, so the commit compiles cleanly. Skip this commit step for now — it will be bundled with Task 5 Step 7.

---

## Chunk 2: Adapter updates

### Task 4: Update `LlmsTxtAdapter` to use `base_url` only

**Files:**
- Modify: `packages/engine/src/lib/llms-txt-adapter.ts`
- Modify: `packages/engine/src/lib/llms-txt-adapter.spec.ts`

- [ ] **Step 1: Update the local_path test to use `base_url: 'file://...'`**

In `llms-txt-adapter.spec.ts`, find the test "reads local files when local_path provided" and update it:

```typescript
it("reads local files when base_url is a file:// URL", async () => {
  let tmpDir!: string;
  try {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-llms-test-"));
    const llmsTxt = `## Core\n\n- [Auth](./auth.txt): Core auth module\n- [UI](./ui.txt): UI components\n`;
    await writeFile(join(tmpDir, "llms.txt"), llmsTxt, "utf-8");
    await writeFile(join(tmpDir, "auth.txt"), "# Auth\n\nFull auth docs.", "utf-8");
    await writeFile(join(tmpDir, "ui.txt"), "# UI\n\nFull UI docs.", "utf-8");

    const pages = await new LlmsTxtAdapter().fetch({
      name: "lib",
      source_type: "llms.txt",
      base_url: `file://${join(tmpDir, "llms.txt")}`,
    });

    expect(pages).toHaveLength(2);
    expect(pages.map(p => p.title)).toEqual(["Auth", "UI"]);
    expect(pages[0].summary).toBe("Core auth module");
    expect(pages[0].component).toBe("Core");
    expect(pages[0].content).toContain("Full auth docs");
    expect(pages[0].url).toBeUndefined();
    expect(pages[0].localPath).toContain("auth.txt");
    expect(pages[0].sourceType).toBe("llms.txt");
  } finally {
    if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
  }
});
```

Also update the throws test:

```typescript
it("throws if base_url not provided", async () => {
  // @ts-expect-error — testing runtime guard
  await expect(new LlmsTxtAdapter().fetch({ name: "t", source_type: "llms.txt" })).rejects.toThrow();
});
```

- [ ] **Step 2: Run to verify test fails**

```bash
pnpm --filter @sensei/engine test src/lib/llms-txt-adapter.spec.ts
```

- [ ] **Step 3: Rewrite `LlmsTxtAdapter`**

```typescript
// packages/engine/src/lib/llms-txt-adapter.ts
import { readFile } from "fs/promises";
import { fileURLToPath } from "url";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { fetchAsMarkdown, parseLlmsIndex } from "./doc-utils.js";

export class LlmsTxtAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`LlmsTxtAdapter: entry "${entry.name}" requires base_url`);

    let text: string;
    if (entry.base_url.startsWith("file://")) {
      text = await readFile(fileURLToPath(entry.base_url), "utf-8");
    } else {
      const res = await fetch(entry.base_url);
      if (!res.ok) throw new Error(`LlmsTxtAdapter: fetch failed for ${entry.base_url}: ${res.status}`);
      text = await res.text();
    }

    const entries = parseLlmsIndex(text, entry.base_url);
    const pages: DocPage[] = [];

    for (let i = 0; i < entries.length; i++) {
      const e = entries[i];
      let content: string;
      try {
        content = e.url.startsWith("file://")
          ? await readFile(fileURLToPath(e.url), "utf-8")
          : await fetchAsMarkdown(e.url);
      } catch (err) {
        console.warn(`[LlmsTxtAdapter] Failed to fetch ${e.url}:`, err instanceof Error ? err.message : String(err));
        content = e.summary;
      }
      const isLocal = e.url.startsWith("file://");
      pages.push({
        title: e.title,
        url: isLocal ? undefined : e.url,
        localPath: isLocal ? fileURLToPath(e.url) : undefined,
        summary: e.summary,
        content,
        sourceType: "llms.txt",
        component: e.component,
        sequence: i,
      });
    }
    return pages;
  }
}
```

- [ ] **Step 4: Run tests**

```bash
pnpm --filter @sensei/engine test src/lib/llms-txt-adapter.spec.ts
```

Expected: all PASS

---

### Task 5: Update `LocalAdapter` and consistency spec

**Files:**
- Modify: `packages/engine/src/lib/local-adapter.ts`
- Modify: `packages/engine/src/lib/local-adapter.spec.ts`
- Modify: `packages/engine/src/lib/adapter-consistency.spec.ts`

- [ ] **Step 1: Update all tests in `local-adapter.spec.ts`**

Every `LibEntry` in this spec uses `local_path: tmpDir`. Replace with `base_url: \`file://${tmpDir}\``:

```typescript
// Pattern — replace in all test entries:
// BEFORE: { name: "mylib", source_type: "local", local_path: tmpDir }
// AFTER:  { name: "mylib", source_type: "local", base_url: `file://${tmpDir}` }

// Also update the llms.txt index test entry:
// BEFORE: { name: "mylib", source_type: "local", local_path: tmpDir }
// AFTER:  { name: "mylib", source_type: "local", base_url: `file://${tmpDir}` }

// Update the throws test:
it("throws if base_url not a file:// URL", async () => {
  // @ts-expect-error — testing runtime guard
  await expect(new LocalAdapter().fetch({ name: "x", source_type: "local" })).rejects.toThrow();
});
```

- [ ] **Step 2: Update `adapter-consistency.spec.ts`**

Two test entries need updating:

```typescript
// Test: "LlmsTxtAdapter (local_path) produces same..."
// Change entry to:
await new LlmsTxtAdapter().fetch({
  name: "mylib",
  source_type: "llms.txt",
  base_url: `file://${join(tmpDir, "llms.txt")}`,
});

// Test: "LocalAdapter (with llms.txt) produces same..."
// Change entry to:
await new LocalAdapter().fetch({
  name: "mylib",
  source_type: "local",
  base_url: `file://${tmpDir}`,
});

// Also update the top-of-file comment block (line ~14):
// Change: "/path/to/docs/llms/llms.txt       (LlmsTxtAdapter via local_path)"
// To:     "file:///path/to/docs/llms/llms.txt (LlmsTxtAdapter via file:// URL)"
```

- [ ] **Step 3: Run to verify tests fail**

```bash
pnpm --filter @sensei/engine test src/lib/local-adapter.spec.ts
```

- [ ] **Step 4: Rewrite `LocalAdapter`**

```typescript
// packages/engine/src/lib/local-adapter.ts
import { readdir, readFile, access } from "fs/promises";
import { join, extname, basename, dirname } from "path";
import { fileURLToPath } from "url";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { extractSummary, parseLlmsIndex } from "./doc-utils.js";

export class LocalAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url?.startsWith("file://")) {
      throw new Error(`LocalAdapter: entry "${entry.name}" requires a file:// base_url`);
    }
    const dirPath = fileURLToPath(entry.base_url);

    const stat = await access(dirPath).then(() => true).catch(() => false);
    if (!stat) throw new Error(`LocalAdapter: path not found: ${dirPath}`);

    const llmsTxtPath = dirPath.endsWith("llms.txt")
      ? dirPath
      : join(dirPath, "llms.txt");

    const hasIndex = await access(llmsTxtPath).then(() => true).catch(() => false);
    if (hasIndex) return fetchFromIndex(llmsTxtPath);

    return walkDir(dirPath, dirPath);
  }
}

async function fetchFromIndex(llmsTxtPath: string): Promise<DocPage[]> {
  const text = await readFile(llmsTxtPath, "utf-8");
  const indexUrl = `file://${llmsTxtPath}`;
  const entries = parseLlmsIndex(text, indexUrl);
  const pages: DocPage[] = [];

  for (let i = 0; i < entries.length; i++) {
    const e = entries[i];
    const localPath = fileURLToPath(e.url);
    let content: string;
    try {
      content = await readFile(localPath, "utf-8");
    } catch (err) {
      console.warn(`[LocalAdapter] Failed to read ${localPath}:`, err instanceof Error ? err.message : String(err));
      content = e.summary;
    }
    pages.push({ title: e.title, localPath, summary: e.summary, content, sourceType: "local", component: e.component, sequence: i });
  }
  return pages;
}

async function walkDir(dir: string, rootPath: string): Promise<DocPage[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const pages: DocPage[] = [];

  for (const dirent of entries) {
    const fullPath = join(dir, dirent.name);
    if (dirent.isDirectory()) {
      pages.push(...await walkDir(fullPath, rootPath));
    } else if (dirent.isFile()) {
      const ext = extname(dirent.name).toLowerCase();
      if (ext !== ".md" && ext !== ".txt") continue;
      const content = await readFile(fullPath, "utf-8");
      const title = basename(dirent.name, ext);
      const summary = extractSummary(content);
      const parentDir = dirname(fullPath);
      const component = parentDir !== rootPath ? basename(parentDir) : undefined;
      pages.push({ title, localPath: fullPath, summary, content, sourceType: "local", component });
    }
  }
  return pages;
}
```

- [ ] **Step 5: Run all engine tests**

```bash
pnpm --filter @sensei/engine test
```

Expected: all PASS

- [ ] **Step 6: Now commit Types + Schema + both adapters together**

All the pieces that depend on `LibEntry` having no `local_path` are now fixed — commit them together so the tree is never broken:

```bash
git add \
  packages/shared/src/types.ts \
  packages/shared/src/config.ts \
  packages/engine/src/lib/llms-txt-adapter.ts \
  packages/engine/src/lib/llms-txt-adapter.spec.ts \
  packages/engine/src/lib/local-adapter.ts \
  packages/engine/src/lib/local-adapter.spec.ts \
  packages/engine/src/lib/adapter-consistency.spec.ts
git commit -m "feat: drop LibEntry.local_path — all sources use file:// base_url"
```

---

## Chunk 3: Dashboard server layer

### Task 6: Update `lib-indexer.ts` — drop `local_path` from `LibInfo`

**Files:**
- Modify: `apps/dashboard/src/lib/server/lib-indexer.ts`

`LibInfo.local_path: string | null` is removed. `base_url` is now always a string (never null) for all library types.

- [ ] **Step 1: Edit `lib-indexer.ts`**

```typescript
// BEFORE
export interface LibInfo {
  id: string;
  name: string;
  source_type: string;
  base_url: string | null;
  local_path: string | null;
}

// Inside runFetch, entry construction:
const entry = {
  name: lib.name,
  source_type: sourceType,
  base_url: lib.base_url ?? undefined,
  local_path: lib.local_path ?? undefined,
};

// AFTER
export interface LibInfo {
  id: string;
  name: string;
  source_type: string;
  base_url: string;
}

// Entry construction:
const entry = {
  name: lib.name,
  source_type: sourceType,
  base_url: lib.base_url,
};
```

- [ ] **Step 2: Verify TypeScript**

```bash
pnpm --filter @sensei/dashboard check
```

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/lib/server/lib-indexer.ts
git commit -m "feat(dashboard): LibInfo drops local_path — base_url always set"
```

---

### Task 7: DB migration — move `local_path` → `base_url`, drop columns

**Files:**
- Create: `supabase/migrations/20260316000005_file_url_refactor.sql`

This migration:
1. Populates `base_url` from `local_path` (with `file://` prefix) where `local_path` is not null
2. Drops `local_path` from `libraries` and `referenced_libraries` tables
3. Makes `base_url` NOT NULL on both tables

Note: `documents_in_library.local_path` is **NOT** touched — it records where document content was read from at index time, not source configuration.

- [ ] **Step 1: Create migration file**

```sql
-- supabase/migrations/20260316000005_file_url_refactor.sql
-- Migrate local_path → file:// base_url, then drop local_path columns.
-- documents_in_library.local_path is intentionally kept (document-level output).

-- 1. Populate base_url from local_path for libraries
UPDATE sensei.libraries
  SET base_url = 'file://' || local_path
  WHERE local_path IS NOT NULL
    AND (base_url IS NULL OR base_url = '');

-- 2. Populate base_url from local_path for referenced_libraries
UPDATE sensei.referenced_libraries
  SET base_url = 'file://' || local_path
  WHERE local_path IS NOT NULL
    AND (base_url IS NULL OR base_url = '');

-- 3. Drop local_path from libraries
ALTER TABLE sensei.libraries DROP COLUMN IF EXISTS local_path;

-- 4. Drop local_path from referenced_libraries
ALTER TABLE sensei.referenced_libraries DROP COLUMN IF EXISTS local_path;

-- 5. Make base_url NOT NULL (every library now has a URL)
ALTER TABLE sensei.libraries ALTER COLUMN base_url SET NOT NULL;
ALTER TABLE sensei.referenced_libraries ALTER COLUMN base_url SET NOT NULL;
```

- [ ] **Step 2: Apply migration**

```bash
supabase db push
```

- [ ] **Step 3: Commit**

```bash
git add supabase/migrations/20260316000005_file_url_refactor.sql
git commit -m "feat(db): migrate local_path → file:// base_url, drop local_path columns"
```

---

### Task 8: Update `libraries/+page.server.ts`

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/+page.server.ts`

Remove all `local_path` from DB queries and upserts. `inferSourceType` now always returns `base_url` so the `'local_path' in inferred` check is gone.

- [ ] **Step 1: Update DB select (line ~12)**

```typescript
// BEFORE:
.select('id,name,source_type,base_url,local_path,...')
// AFTER:
.select('id,name,source_type,base_url,...')
```

- [ ] **Step 2: Update inferSourceType extraction and upsert (lines ~49-57)**

```typescript
// BEFORE:
const inferred = inferSourceType(url);
const { source_type } = inferred;
const base_url = 'base_url' in inferred ? inferred.base_url : null;
const local_path = 'local_path' in inferred ? inferred.local_path : null;
// ...upsert with: { name, source_type, base_url: base_url ?? null, local_path: local_path ?? null, ... }

// AFTER:
const { source_type, base_url } = inferSourceType(url);
// ...upsert with: { name, source_type, base_url, index_status: 'pending' }
```

- [ ] **Step 3: Update post-upsert select and startLibFetch cast (lines ~60-66)**

```typescript
// BEFORE:
.select('id,name,source_type,base_url,local_path')
await startLibFetch(db, existing as { id: string; name: string; source_type: string; base_url: string | null; local_path: string | null });

// AFTER:
.select('id,name,source_type,base_url')
await startLibFetch(db, existing as { id: string; name: string; source_type: string; base_url: string });
```

- [ ] **Step 4: Verify TypeScript**

```bash
pnpm --filter @sensei/dashboard check
```

- [ ] **Step 5: Commit**

```bash
git add apps/dashboard/src/routes/libraries/+page.server.ts
git commit -m "feat(dashboard): libraries add page — remove local_path, use base_url"
```

---

### Task 9: Update `libraries/[id]/+page.server.ts`

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/[id]/+page.server.ts`

Multiple actions (load, edit, reindex) all reference `local_path`. Remove all. Keep `local_path` in the `documents_in_library` select (line ~23) — that column still exists.

- [ ] **Step 1: Update library load query (line ~15)**

```typescript
// BEFORE: .select('id,name,source_type,base_url,local_path,...')
// AFTER:  .select('id,name,source_type,base_url,...')
// NOTE: the documents select on line ~23 keeps local_path — DO NOT remove it.
```

- [ ] **Step 2: Update lib type assertion (line ~44)**

```typescript
// BEFORE: lib as { ...; base_url: string | null; local_path: string | null; ... }
// AFTER:  lib as { ...; base_url: string; ... }
```

- [ ] **Step 3: Update edit action (lines ~67-89)**

```typescript
// BEFORE:
const inferred = inferSourceType(url);
const { source_type } = inferred;
const base_url = 'base_url' in inferred ? (inferred.base_url ?? null) : null;
const local_path = 'local_path' in inferred ? (inferred.local_path ?? null) : null;
// ...
.update({ source_type, base_url: base_url ?? null, local_path: local_path ?? null })

// AFTER:
const { source_type, base_url } = inferSourceType(url);
// ...
.update({ source_type, base_url })
```

- [ ] **Step 4: Update all select + cast in reindex action (lines ~94, ~173, ~179)**

```typescript
// BEFORE: .select('id,name,source_type,base_url,local_path')
// AFTER:  .select('id,name,source_type,base_url')

// BEFORE: lib as { ...; base_url: string | null; local_path: string | null }
// AFTER:  lib as { ...; base_url: string }
```

- [ ] **Step 5: Verify TypeScript**

```bash
pnpm --filter @sensei/dashboard check
```

- [ ] **Step 6: Commit**

```bash
git add "apps/dashboard/src/routes/libraries/[id]/+page.server.ts"
git commit -m "feat(dashboard): library detail page — remove local_path, use base_url"
```

---

## Chunk 4: Repos library page + UI

### Task 10: Update `repos/[id]/libraries/+page.server.ts`

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`

This file has the most references (~18). Key areas: DB queries, `inferSourceType` extraction, YAML config writing, and `startLibIndexing` calls. Also covers the `link` action (lines ~232-243) which selects `local_path` from `libraries` and writes it to `referenced_libraries`.

- [ ] **Step 1: Remove `local_path` from all DB select queries**

Four selects contain `local_path`: lines ~50, ~232, ~259, ~285.

```typescript
// Pattern — remove local_path from each:
// BEFORE: .select('name,source_type,base_url,local_path,...')
// AFTER:  .select('name,source_type,base_url,...')
```

- [ ] **Step 2: Update `inferSourceType` extraction (line ~166)**

```typescript
// BEFORE:
const local_path = 'local_path' in inferred ? inferred.local_path : undefined;

// AFTER: (delete this line — base_url is always present in inferred)
```

- [ ] **Step 3: Update YAML config writing (line ~175)**

```typescript
// BEFORE:
customLibs.push({ name, source_type, ...(base_url ? { base_url } : { local_path }) });

// AFTER:
customLibs.push({ name, source_type, base_url });
```

- [ ] **Step 4: Remove `local_path` from all upsert objects**

Lines ~187, ~204, ~243: remove `local_path: local_path ?? null` lines from each upsert.

- [ ] **Step 5: Remove `local_path` from all `startLibIndexing` calls**

Lines ~217, ~273, ~297: remove `local_path: ...` from each call argument object.

- [ ] **Step 6: Remove `localPath` from `LibRow` type and mapping**

Lines ~97, ~114 map `lib.local_path` to `localPath` in the `LibRow` type. Remove these. Also remove `localPath` from the `LibRow` interface if it exists.

- [ ] **Step 7: Verify TypeScript**

```bash
pnpm --filter @sensei/dashboard check
```

- [ ] **Step 8: Commit**

```bash
git add "apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts"
git commit -m "feat(dashboard): repos/libraries page — remove local_path, use base_url"
```

---

### Task 11: Update `libraries/[id]/+page.svelte` display

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/[id]/+page.svelte`

Three changes: form input default, source URL display, and document link guard.

- [ ] **Step 1: Update form input fallback (line ~97)**

```svelte
<!-- BEFORE: -->
value={data.lib.base_url ?? data.lib.local_path ?? ''}
<!-- AFTER: -->
value={data.lib.base_url ?? ''}
```

- [ ] **Step 2: Update source URL display (lines ~241-244)**

```svelte
<!-- BEFORE: -->
{#if data.lib.base_url}
  · <a href={data.lib.base_url} target="_blank" rel="noopener noreferrer" class="...">{data.lib.base_url}</a>
{:else if data.lib.local_path}
  · <span class="font-mono">{data.lib.local_path}</span>
{/if}

<!-- AFTER: -->
{#if data.lib.base_url}
  {#if data.lib.base_url.startsWith('file://')}
    · <span class="font-mono text-surface-z5">{data.lib.base_url.replace('file://', '')}</span>
  {:else}
    · <a href={data.lib.base_url} target="_blank" rel="noopener noreferrer" class="text-primary-z6 hover:text-primary-z7 transition-colors">{data.lib.base_url}</a>
  {/if}
{/if}
```

- [ ] **Step 3: Verify TypeScript**

```bash
pnpm --filter @sensei/dashboard check
```

- [ ] **Step 4: Start dev server and visually verify**

```bash
pnpm --filter @sensei/dashboard dev
```

Open http://localhost:5173/libraries — confirm local libraries show plain path text, remote show clickable links.

- [ ] **Step 5: Commit**

```bash
git add "apps/dashboard/src/routes/libraries/[id]/+page.svelte"
git commit -m "feat(dashboard): display file:// base_url as plain path, not clickable link"
```

---

### Task 12: Final verification

- [ ] **Step 1: Run all tests**

```bash
pnpm --filter @sensei/engine test
pnpm --filter @sensei/server test
pnpm --filter @sensei/cli test
```

Expected: all PASS

- [ ] **Step 2: TypeScript check**

```bash
pnpm --filter @sensei/dashboard check
```

Expected: no errors

- [ ] **Step 3: Grep for remaining `local_path` in library-related files**

```bash
grep -rn "local_path" \
  packages/engine/src/lib/ \
  packages/shared/src/ \
  packages/cli/src/lib/detect-libs.ts \
  apps/dashboard/src/lib/server/lib-indexer.ts \
  apps/dashboard/src/routes/libraries/ \
  "apps/dashboard/src/routes/repos/[id]/libraries/"
```

Expected: only hits in `documents_in_library` column selects (document output, intentionally kept) and `repos` table (repo working directory, unrelated).

- [ ] **Step 4: Commit any remaining cleanup**

```bash
git add -p
git commit -m "chore: final cleanup after file:// URL refactor"
```
