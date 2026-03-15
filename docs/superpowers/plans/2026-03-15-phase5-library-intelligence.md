# Phase 5: Library Intelligence Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Index external library docs into Supabase via `sensei update-registry`, expose them through `get_lib_docs` MCP tool, and show status in dashboard.

**Architecture:** Three SourceAdapters fetch DocPage arrays; LibIndexer embeds and upserts to `sensei.lib_doc_sections`; LibSkillGenerator (using ClaudeBackend + SkillValidator) writes per-lib skill files; `get_lib_docs` MCP tool answers doc queries semantically via pgvector; dashboard at `/repos/[id]/libraries` shows freshness and skill status per lib.

**Tech Stack:** TypeScript, Bun, Vitest, Supabase pgvector, @mozilla/readability + jsdom + turndown (HTML→markdown), zod (config validation), @clack/prompts (CLI)

---

## Chunk 1: Foundation

### Task 1: Database Migration

**Files:**
- Create: `supabase/migrations/20260315000000_phase5_lib_doc_sections.sql`

- [ ] **Step 1: Create migration file**

```sql
-- supabase/migrations/20260315000000_phase5_lib_doc_sections.sql

create table if not exists sensei.lib_doc_sections (
  id            uuid primary key default gen_random_uuid(),
  repo_id       uuid not null references sensei.repos(id) on delete cascade,
  lib_name      text not null,
  title         text not null,
  url           text,
  local_path    text,
  description   text not null,
  content       text,
  source_type   text not null check (source_type in ('llms.txt', 'http', 'local')),
  component     text,
  embedding     vector(768),
  last_fetched  timestamptz not null default now(),
  created_at    timestamptz not null default now()
);

create index if not exists lib_doc_sections_repo_lib_idx
  on sensei.lib_doc_sections(repo_id, lib_name);

create index if not exists lib_doc_sections_embedding_idx
  on sensei.lib_doc_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

-- RPC for semantic search (mirrors match_embeddings from Phase 2)
create or replace function sensei.match_lib_doc_sections(
  p_repo_id       uuid,
  p_lib_name      text,
  p_component     text DEFAULT NULL,
  query_embedding vector(768),
  match_count     int
)
returns table(
  title       text,
  url         text,
  local_path  text,
  description text,
  content     text,
  source_type text,
  component   text
)
language sql
security definer
as $$
  SELECT title, url, local_path, description, content, source_type, component
  FROM sensei.lib_doc_sections
  WHERE repo_id = p_repo_id
    AND lib_name = p_lib_name
    AND (p_component IS NULL OR component = p_component)
  ORDER BY embedding <=> query_embedding
  LIMIT match_count
$$;

-- NOTE: grants are managed separately in database/ddl/grants.ddl — do not add here
```

- [ ] **Step 2: Apply migration**

If using local Supabase: `bunx supabase db reset` from repo root.
If using remote Supabase: paste the SQL into the Supabase dashboard SQL editor.

- [ ] **Step 3: Verify migration applied**

```sql
-- Run in Supabase SQL editor or psql to confirm the table and RPC exist:
select column_name, data_type
from information_schema.columns
where table_schema = 'sensei' and table_name = 'lib_doc_sections'
order by ordinal_position;
```
Expected: rows for `id`, `repo_id`, `lib_name`, `title`, `url`, `local_path`, `description`, `content`, `source_type`, `component`, `embedding`, `last_fetched`, `created_at`

```sql
select routine_name from information_schema.routines
where routine_schema = 'sensei' and routine_name = 'match_lib_doc_sections';
```
Expected: 1 row

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260315000000_phase5_lib_doc_sections.sql
git commit -m "feat(db): add lib_doc_sections table and match_lib_doc_sections RPC"
```

---

### Task 2: Shared Types + Config Validation

**Files:**
- Modify: `packages/shared/src/types.ts` — append lib intelligence types
- Modify: `packages/shared/src/config.ts` — add custom_libs to SenseiRepoConfig + Zod validation
- Create: `packages/shared/src/config.spec.ts` — tests for loadSenseiConfig
- Modify: `packages/shared/package.json` — add zod dependency

- [ ] **Step 1: Add zod to shared package.json**

In `packages/shared/package.json`, add to `"dependencies"`:
```json
"zod": "^3.22.0"
```

Run from repo root: `bun install`

- [ ] **Step 2: Append new types to packages/shared/src/types.ts**

```typescript
// ─── Library intelligence types ──────────────────────────────────────────────

export interface LibEntry {
  name: string;
  source_type: 'llms.txt' | 'http' | 'local';
  base_url?: string;       // llms.txt: direct URL to llms.txt file; http: root URL to crawl
  local_path?: string;     // llms.txt: local path to llms.txt file; local: directory to scan
  description?: string;    // human-readable description of the library
}

export interface DocPage {
  title: string;
  url?: string;            // remote sources
  localPath?: string;      // local sources
  description: string;     // short summary — embedding input for llms.txt; auto-extracted for others
  content?: string;        // full extracted markdown — null for llms.txt entries
  sourceType: 'llms.txt' | 'http' | 'local';
  component?: string;      // optional grouping (e.g. 'Button', 'Form')
}

export interface LibSkillFile {
  libName: string;
  path: string;            // absolute path to written skill file
  generatedAt: string;     // ISO timestamp
}

export interface LibSkillsManifest {
  repoSlug: string;
  skills: LibSkillFile[];
  updatedAt: string;
}
```

- [ ] **Step 3: Write failing tests for loadSenseiConfig**

Create `packages/shared/src/config.spec.ts`:

```typescript
// packages/shared/src/config.spec.ts
import { describe, it, expect, afterEach } from "vitest";
import { writeFile, mkdir, rm, mkdtemp } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { z } from "zod";
import { loadSenseiConfig } from "./config.js";

describe("loadSenseiConfig", () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
  });

  it("returns parsed config with custom_libs", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-config-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, ".sensei", "config.yaml"),
      `repo_id: test-repo-id\nsupabase_url: https://x.supabase.co\ncustom_libs:\n  - name: rokkit\n    source_type: llms.txt\n    base_url: https://rokkit.dev/llms.txt\n`,
      "utf-8",
    );

    const config = await loadSenseiConfig(tmpDir);

    expect(config).not.toBeNull();
    expect(config!.repo_id).toBe("test-repo-id");
    expect(config!.custom_libs).toHaveLength(1);
    expect(config!.custom_libs![0].name).toBe("rokkit");
    expect(config!.custom_libs![0].source_type).toBe("llms.txt");
  });

  it("returns null when config file is missing", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-config-test-"));
    const config = await loadSenseiConfig(tmpDir);
    expect(config).toBeNull();
  });

  it("throws ZodError when custom_libs contains invalid source_type", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-config-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, ".sensei", "config.yaml"),
      `repo_id: x\nsupabase_url: https://x.supabase.co\ncustom_libs:\n  - name: bad\n    source_type: invalid-type\n`,
      "utf-8",
    );

    await expect(loadSenseiConfig(tmpDir)).rejects.toThrow(z.ZodError);
  });
});
```

- [ ] **Step 4: Run test to confirm it fails**

```bash
cd packages/shared && bunx vitest run src/config.spec.ts
```
Expected: FAIL — `loadSenseiConfig` not found (function not yet updated)

- [ ] **Step 5: Update packages/shared/src/config.ts**

Replace the entire file content with:

```typescript
import { readFile, access } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import os from "os";
import { z } from "zod";
import type { LibEntry } from "./types.js";

const LibEntrySchema = z.object({
  name: z.string(),
  source_type: z.enum(['llms.txt', 'http', 'local']),
  base_url: z.string().optional(),
  local_path: z.string().optional(),
  description: z.string().optional(),
});

export interface SenseiRepoConfig {
  repo_id: string;
  supabase_url: string;
  custom_libs?: LibEntry[];
}

export interface SenseiCredentials {
  supabase_service_key: string;
}

/** Read .sensei/config.yaml from repoPath. Returns null if missing. */
export async function loadSenseiConfig(repoPath: string): Promise<SenseiRepoConfig | null> {
  const configPath = join(repoPath, ".sensei", "config.yaml");
  try {
    await access(configPath);
    const raw = await readFile(configPath, "utf-8");
    const parsed = yaml.load(raw) as Record<string, unknown>;

    let custom_libs: LibEntry[] | undefined;
    if (Array.isArray(parsed?.custom_libs)) {
      custom_libs = z.array(LibEntrySchema).parse(parsed.custom_libs) as LibEntry[];
    }

    return { ...(parsed as SenseiRepoConfig), custom_libs };
  } catch (err) {
    if (err instanceof z.ZodError) throw err;
    return null;
  }
}

/** Read credentials from ~/.config/sensei/credentials.yaml, or SUPABASE_SERVICE_KEY env. */
export async function loadCredentials(homeDir?: string): Promise<SenseiCredentials | null> {
  if (process.env.SUPABASE_SERVICE_KEY) {
    return { supabase_service_key: process.env.SUPABASE_SERVICE_KEY };
  }
  const home = homeDir ?? os.homedir();
  const credPath = join(home, ".config", "sensei", "credentials.yaml");
  try {
    await access(credPath);
    const raw = await readFile(credPath, "utf-8");
    return yaml.load(raw) as SenseiCredentials;
  } catch {
    return null;
  }
}
```

- [ ] **Step 6: Run test to confirm it passes**

```bash
cd packages/shared && bunx vitest run src/config.spec.ts
```
Expected: PASS (3 tests)

- [ ] **Step 7: Verify types compile**

```bash
cd packages/shared && bunx tsc --noEmit
```
Expected: no errors

- [ ] **Step 8: Commit**

```bash
git add packages/shared/src/types.ts packages/shared/src/config.ts packages/shared/src/config.spec.ts packages/shared/package.json bun.lock
git commit -m "feat(shared): add LibEntry/DocPage/LibSkillFile/LibSkillsManifest types + custom_libs Zod validation"
```

---

## Chunk 2: Source Adapters

### Task 3: SourceAdapter Interface + LlmsTxtAdapter

**Files:**
- Create: `packages/engine/src/lib/source-adapter.ts`
- Create: `packages/engine/src/lib/llms-txt-adapter.ts`
- Create: `packages/engine/src/lib/llms-txt-adapter.spec.ts`

- [ ] **Step 1: Write failing tests**

```typescript
// packages/engine/src/lib/llms-txt-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { writeFile, mkdtemp, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { LlmsTxtAdapter } from "./llms-txt-adapter.js";
import type { LibEntry } from "@sensei/shared";

const SAMPLE_LLMS_TXT = `# Rokkit UI
> Component library for SvelteKit

## Buttons

- [Button](https://rokkit.dev/docs/button): Primary button component with variants
- [IconButton](https://rokkit.dev/docs/icon-button): Icon-only button

## Forms

- [Input](https://rokkit.dev/docs/input): Text input field
- [Select](https://rokkit.dev/docs/select): Dropdown selector
`;

describe("LlmsTxtAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  it("fetches llms.txt from URL and parses into DocPages with correct shape", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      text: () => Promise.resolve(SAMPLE_LLMS_TXT),
    }));

    const adapter = new LlmsTxtAdapter();
    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(4);
    expect(pages[0].title).toBe("Button");
    expect(pages[0].url).toBe("https://rokkit.dev/docs/button");
    expect(pages[0].description).toBe("Primary button component with variants");
    expect(pages[0].component).toBe("Buttons");
    expect(pages[0].content).toBeUndefined();
    expect(pages[0].sourceType).toBe("llms.txt");
    expect(pages[2].title).toBe("Input");
    expect(pages[2].component).toBe("Forms");
  });

  it("reads llms.txt from local file path", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-llmstxt-test-"));
    try {
      const localPath = join(tmpDir, "llms.txt");
      await writeFile(localPath, SAMPLE_LLMS_TXT, "utf-8");

      const adapter = new LlmsTxtAdapter();
      const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", local_path: localPath };
      const pages = await adapter.fetch(entry);

      expect(pages).toHaveLength(4);
      expect(pages[0].sourceType).toBe("llms.txt");
    } finally {
      await rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("skips malformed lines gracefully", async () => {
    const malformed = `# Test\n\n## Section\n\n- [Valid](https://x.com/v): description\nnot-a-link\n- also not\n`;
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      text: () => Promise.resolve(malformed),
    }));

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "t", source_type: "llms.txt", base_url: "https://x.com/llms.txt" });

    expect(pages).toHaveLength(1);
    expect(pages[0].title).toBe("Valid");
  });

  it("throws if neither base_url nor local_path provided", async () => {
    const adapter = new LlmsTxtAdapter();
    await expect(adapter.fetch({ name: "t", source_type: "llms.txt" })).rejects.toThrow();
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd packages/engine && bunx vitest run src/lib/llms-txt-adapter.spec.ts
```
Expected: FAIL — `LlmsTxtAdapter` not found

- [ ] **Step 3: Create source-adapter.ts**

```typescript
// packages/engine/src/lib/source-adapter.ts
import type { LibEntry, DocPage } from "@sensei/shared";

export interface SourceAdapter {
  fetch(entry: LibEntry): Promise<DocPage[]>;
}
```

- [ ] **Step 4: Create llms-txt-adapter.ts**

```typescript
// packages/engine/src/lib/llms-txt-adapter.ts
import { readFile } from "fs/promises";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

const LINK_RE = /^-\s+\[([^\]]+)\]\(([^)]+)\):\s*(.+)$/;
const SECTION_RE = /^##\s+(.+)$/;

export class LlmsTxtAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    let text: string;
    if (entry.base_url?.startsWith("http")) {
      const res = await fetch(entry.base_url);
      if (!res.ok) throw new Error(`LlmsTxtAdapter: fetch failed for ${entry.base_url}: ${res.status}`);
      text = await res.text();
    } else if (entry.local_path) {
      text = await readFile(entry.local_path, "utf-8");
    } else {
      throw new Error(`LlmsTxtAdapter: entry "${entry.name}" must have base_url or local_path`);
    }
    return parse(text);
  }
}

function parse(text: string): DocPage[] {
  const pages: DocPage[] = [];
  let currentSection: string | undefined;

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    const sectionMatch = line.match(SECTION_RE);
    if (sectionMatch) {
      currentSection = sectionMatch[1].trim();
      continue;
    }
    const linkMatch = line.match(LINK_RE);
    if (!linkMatch) continue;
    const [, title, url, description] = linkMatch;
    pages.push({ title: title.trim(), url: url.trim(), description: description.trim(), sourceType: "llms.txt", component: currentSection });
  }
  return pages;
}
```

- [ ] **Step 5: Run test to confirm it passes**

```bash
cd packages/engine && bunx vitest run src/lib/llms-txt-adapter.spec.ts
```
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/source-adapter.ts packages/engine/src/lib/llms-txt-adapter.ts packages/engine/src/lib/llms-txt-adapter.spec.ts
git commit -m "feat(engine): add SourceAdapter interface and LlmsTxtAdapter"
```

---

### Task 4: HttpAdapter

**Files:**
- Create: `packages/engine/src/lib/http-adapter.ts`
- Create: `packages/engine/src/lib/http-adapter.spec.ts`
- Modify: `packages/engine/package.json` — add @mozilla/readability, jsdom, turndown

- [ ] **Step 1: Add dependencies to packages/engine/package.json**

In `"dependencies"` add:
```json
"@mozilla/readability": "^0.5.0",
"jsdom": "^25.0.1",
"turndown": "^7.2.0"
```
In `"devDependencies"` add:
```json
"@types/jsdom": "^21.1.7",
"@types/turndown": "^5.0.5"
```

Run from repo root: `bun install`

- [ ] **Step 2: Write failing test**

```typescript
// packages/engine/src/lib/http-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { HttpAdapter } from "./http-adapter.js";
import type { LibEntry } from "@sensei/shared";

const MULTI_SECTION_HTML = `<!DOCTYPE html><html><head><title>Kavach</title></head>
<body><article>
  <h1>Kavach Auth</h1><p>Zero-trust auth library.</p>
  <h2>Installation</h2><p>Run npm install kavach to get started.</p>
  <h2>Usage</h2><p>Import the module and call createClient with your config.</p>
</article></body></html>`;

const SINGLE_SECTION_HTML = `<!DOCTYPE html><html><body>
<article><h1>Title</h1><p>Content without any h2 headings here.</p></article>
</body></html>`;

describe("HttpAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  it("splits content at ## headings into one DocPage per section", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      text: () => Promise.resolve(MULTI_SECTION_HTML),
    }));

    const adapter = new HttpAdapter();
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev" };
    const pages = await adapter.fetch(entry);

    expect(pages.length).toBeGreaterThanOrEqual(2);
    const titles = pages.map(p => p.title);
    expect(titles).toContain("Installation");
    expect(titles).toContain("Usage");
    pages.forEach(p => {
      expect(p.sourceType).toBe("http");
      expect(p.content).toBeTruthy();
      expect(p.description.length).toBeLessThanOrEqual(200);
    });
  });

  it("returns single DocPage when page has no ## headings", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      text: () => Promise.resolve(SINGLE_SECTION_HTML),
    }));

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "http", base_url: "https://x.com" });

    expect(pages).toHaveLength(1);
    expect(pages[0].content).toBeTruthy();
  });

  it("throws when fetch returns non-ok status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 404, text: () => Promise.resolve("") }));
    const adapter = new HttpAdapter();
    await expect(adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com" })).rejects.toThrow("404");
  });
});
```

- [ ] **Step 3: Run test to confirm it fails**

```bash
cd packages/engine && bunx vitest run src/lib/http-adapter.spec.ts
```
Expected: FAIL

- [ ] **Step 4: Create http-adapter.ts**

```typescript
// packages/engine/src/lib/http-adapter.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

const td = new TurndownService();

export class HttpAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`HttpAdapter: entry "${entry.name}" requires base_url`);

    const res = await fetch(entry.base_url);
    if (!res.ok) throw new Error(`HttpAdapter: fetch failed for ${entry.base_url}: ${res.status}`);

    const html = await res.text();
    const dom = new JSDOM(html, { url: entry.base_url });
    const reader = new Readability(dom.window.document);
    const article = reader.parse();
    const markdown = td.turndown(article?.content ?? html);

    return splitIntoPages(markdown, entry.base_url);
  }
}

function splitIntoPages(markdown: string, url: string): DocPage[] {
  const sections = markdown.split(/\n(?=## )/);

  if (sections.length <= 1) {
    const content = markdown.trim();
    return [{ title: "Overview", url, description: content.slice(0, 200), content, sourceType: "http" }];
  }

  return sections
    .map(s => s.trim())
    .filter(Boolean)
    .map(section => {
      const lines = section.split("\n");
      const title = lines[0].replace(/^##\s+/, "").trim();
      const body = lines.slice(1).join(" ").trim();
      return { title, url, description: body.slice(0, 200) || section.slice(0, 200), content: section.trim(), sourceType: "http" as const };
    });
}
```

- [ ] **Step 5: Run test to confirm it passes**

```bash
cd packages/engine && bunx vitest run src/lib/http-adapter.spec.ts
```
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/http-adapter.ts packages/engine/src/lib/http-adapter.spec.ts packages/engine/package.json bun.lock
git commit -m "feat(engine): add HttpAdapter with HTML-to-markdown page splitting"
```

---

### Task 5: LocalAdapter

**Files:**
- Create: `packages/engine/src/lib/local-adapter.ts`
- Create: `packages/engine/src/lib/local-adapter.spec.ts`

- [ ] **Step 1: Write failing test**

```typescript
// packages/engine/src/lib/local-adapter.spec.ts
import { describe, it, expect, afterEach } from "vitest";
import { mkdtemp, rm, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { LocalAdapter } from "./local-adapter.js";
import type { LibEntry } from "@sensei/shared";

describe("LocalAdapter", () => {
  let tmpDir: string;
  afterEach(async () => { if (tmpDir) await rm(tmpDir, { recursive: true, force: true }); });

  it("collects .md and .txt files, sets title from filename and description from first 200 chars", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-local-test-"));
    await writeFile(join(tmpDir, "overview.md"), "# Overview\n\nThis is the overview.", "utf-8");
    await writeFile(join(tmpDir, "api.txt"), "API reference content here.", "utf-8");
    await writeFile(join(tmpDir, "ignored.json"), '{"skip":true}', "utf-8");

    const adapter = new LocalAdapter();
    const entry: LibEntry = { name: "mylib", source_type: "local", local_path: tmpDir };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(2);
    const titles = pages.map(p => p.title).sort();
    expect(titles).toEqual(["api", "overview"]);

    const overview = pages.find(p => p.title === "overview")!;
    expect(overview.content).toContain("# Overview");
    expect(overview.description).toBe(overview.content!.slice(0, 200));
    expect(overview.localPath).toContain("overview.md");
    expect(overview.url).toBeUndefined();
    expect(overview.component).toBeUndefined();
    expect(overview.sourceType).toBe("local");
  });

  it("infers component from immediate parent dir name when file is nested", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-local-test-"));
    const btnDir = join(tmpDir, "Button");
    await mkdir(btnDir, { recursive: true });
    await writeFile(join(btnDir, "button.md"), "Button component docs.", "utf-8");
    await writeFile(join(tmpDir, "intro.md"), "Introduction.", "utf-8");

    const adapter = new LocalAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "local", local_path: tmpDir });

    const btn = pages.find(p => p.title === "button")!;
    expect(btn.component).toBe("Button");

    const intro = pages.find(p => p.title === "intro")!;
    expect(intro.component).toBeUndefined();
  });

  it("throws if local_path not provided", async () => {
    const adapter = new LocalAdapter();
    await expect(adapter.fetch({ name: "x", source_type: "local" })).rejects.toThrow();
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd packages/engine && bunx vitest run src/lib/local-adapter.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Create local-adapter.ts**

```typescript
// packages/engine/src/lib/local-adapter.ts
import { readdir, readFile } from "fs/promises";
import { join, extname, basename, dirname } from "path";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

export class LocalAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.local_path) throw new Error(`LocalAdapter: entry "${entry.name}" requires local_path`);
    return walkDir(entry.local_path, entry.local_path);
  }
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
      const description = content.slice(0, 200);
      const parentDir = dirname(fullPath);
      const component = parentDir !== rootPath ? basename(parentDir) : undefined;
      pages.push({ title, localPath: fullPath, description, content, sourceType: "local", component });
    }
  }
  return pages;
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cd packages/engine && bunx vitest run src/lib/local-adapter.spec.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/local-adapter.ts packages/engine/src/lib/local-adapter.spec.ts
git commit -m "feat(engine): add LocalAdapter for scanning .md/.txt from local directories"
```

---

## Chunk 3: LibIndexer + Skill Generation

### Task 6: LibIndexer

**Files:**
- Create: `packages/engine/src/lib/lib-indexer.ts`
- Create: `packages/engine/src/lib/lib-indexer.spec.ts`

- [ ] **Step 1: Write failing test**

```typescript
// packages/engine/src/lib/lib-indexer.spec.ts
import { describe, it, expect, vi } from "vitest";
import { LibIndexer } from "./lib-indexer.js";
import type { DocPage, LibEntry, ModelBackend } from "@sensei/shared";

const makeMockBackend = (): ModelBackend => ({
  name: "mock",
  init: vi.fn().mockResolvedValue(undefined),
  isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(""),
  embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  extract: vi.fn().mockResolvedValue({}),
});

// Build a mock Supabase client that records operations
const makeMockDb = () => {
  const insertedBatches: unknown[] = [];
  let deleteChain = { eq: vi.fn() };
  deleteChain.eq = vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) });

  return {
    _insertedBatches: insertedBatches,
    from: vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue(deleteChain),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        insertedBatches.push(rows);
        return Promise.resolve({ error: null });
      }),
    })),
  };
};

describe("LibIndexer", () => {
  it("deletes existing rows then inserts N rows, calls embed N times", async () => {
    const db = makeMockDb();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://x.com/llms.txt" };
    const pages: DocPage[] = [
      { title: "Button", url: "https://rokkit.dev/button", description: "A button", sourceType: "llms.txt" },
      { title: "Input", url: "https://rokkit.dev/input", description: "An input", sourceType: "llms.txt" },
    ];

    const result = await indexer.index("repo-123", entry, pages);

    expect(result.sectionsIndexed).toBe(2);
    expect(backend.embed).toHaveBeenCalledTimes(2);
    // delete was called before insert
    expect(db.from).toHaveBeenCalledWith("lib_doc_sections");
    expect(db._insertedBatches).toHaveLength(1);
    const inserted = db._insertedBatches[0] as unknown[];
    expect(inserted).toHaveLength(2);
  });

  it("embeds description for llms.txt; first 512 chars of content for http/local", async () => {
    const backend = makeMockBackend();
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, backend);

    const content = "X".repeat(600);
    const pages: DocPage[] = [
      { title: "Usage", url: "https://x.com", description: "Short desc", content, sourceType: "http" },
    ];
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev" };

    await indexer.index("repo-123", entry, pages);

    expect(backend.embed).toHaveBeenCalledWith("X".repeat(512));
  });

  it("falls back to description when content is undefined for non-llms.txt page", async () => {
    const backend = makeMockBackend();
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, backend);

    const pages: DocPage[] = [
      { title: "Page", url: "https://x.com", description: "fallback desc", sourceType: "http" },
    ];

    await indexer.index("repo-123", { name: "x", source_type: "http" }, pages);
    expect(backend.embed).toHaveBeenCalledWith("fallback desc");
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd packages/engine && bunx vitest run src/lib/lib-indexer.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Create lib-indexer.ts**

```typescript
// packages/engine/src/lib/lib-indexer.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { LibEntry, DocPage, ModelBackend } from "@sensei/shared";

export class LibIndexer {
  constructor(
    private readonly db: SupabaseClient,
    private readonly backend: ModelBackend,
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
        const embedInput =
          entry.source_type === "llms.txt"
            ? page.description
            : (page.content ?? page.description).slice(0, 512);

        const embedding = await this.backend.embed(embedInput);
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
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cd packages/engine && bunx vitest run src/lib/lib-indexer.spec.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/lib-indexer.ts packages/engine/src/lib/lib-indexer.spec.ts
git commit -m "feat(engine): add LibIndexer with full-refresh embed-and-upsert"
```

---

### Task 7: LibSkillGenerator + ClaudeAdapter.writeLibSkill + Engine Exports

**Files:**
- Create: `packages/engine/src/lib/lib-skill-generator.ts`
- Create: `packages/engine/src/lib/lib-skill-generator.spec.ts`
- Modify: `packages/engine/src/agent/claude-adapter.ts` — add writeLibSkill()
- Modify: `packages/engine/src/agent/claude-adapter.spec.ts` — add writeLibSkill test
- Modify: `packages/engine/src/index.ts` — add lib/* exports

- [ ] **Step 1: Write failing test for LibSkillGenerator**

```typescript
// packages/engine/src/lib/lib-skill-generator.spec.ts
import { describe, it, expect, vi } from "vitest";
import { LibSkillGenerator } from "./lib-skill-generator.js";
import { SkillValidator } from "../skill-gen/skill-validator.js";
import type { ModelBackend, ProjectProfile, LibEntry, DocPage } from "@sensei/shared";

const VALID_SKILL = "---\nname: myrepo-lib-rokkit\ndescription: Use when using rokkit\n---\n# Rokkit Guide";
const INVALID = "1. Package name wrong";

const makeProfile = (): ProjectProfile => ({
  repoName: "my-repo",
  repoPath: "/tmp/repo",
  dominantLanguage: "typescript",
  framework: "sveltekit",
  packageNames: ["engine"],
  keySymbols: ["createClient"],
  testPattern: "*.spec.ts",
  cliCommands: { test: "bun test" },
  senseiConfig: "repo_id: abc",
});

describe("LibSkillGenerator", () => {
  it("calls generate() once when validator passes on first attempt", async () => {
    const generateSpy = vi.fn()
      .mockResolvedValueOnce(VALID_SKILL)  // generation
      .mockResolvedValueOnce("VALID");     // validation

    const model: ModelBackend = {
      name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
      generate: generateSpy, embed: vi.fn().mockResolvedValue([]), extract: vi.fn().mockResolvedValue({}),
    };

    const validator = new SkillValidator(model, makeProfile());
    const generator = new LibSkillGenerator(model, makeProfile(), validator);
    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", description: "UI lib" };
    const pages: DocPage[] = [{ title: "Button", description: "A button", sourceType: "llms.txt" }];

    const result = await generator.generate(entry, pages);

    expect(result).toBe(VALID_SKILL);
    expect(generateSpy).toHaveBeenCalledTimes(2); // 1 generation + 1 validation
  });

  it("retries on invalid and succeeds on second attempt", async () => {
    const generateSpy = vi.fn()
      .mockResolvedValueOnce("bad skill")  // gen 1
      .mockResolvedValueOnce(INVALID)      // validator rejects
      .mockResolvedValueOnce(VALID_SKILL)  // gen 2
      .mockResolvedValueOnce("VALID");     // validator accepts

    const model: ModelBackend = {
      name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
      generate: generateSpy, embed: vi.fn().mockResolvedValue([]), extract: vi.fn().mockResolvedValue({}),
    };

    const generator = new LibSkillGenerator(model, makeProfile(), new SkillValidator(model, makeProfile()));
    const result = await generator.generate({ name: "rokkit", source_type: "llms.txt" }, []);
    expect(result).toBe(VALID_SKILL);
  });

  it("throws after 3 failed attempts", async () => {
    const model: ModelBackend = {
      name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
      generate: vi.fn().mockResolvedValue(INVALID), embed: vi.fn().mockResolvedValue([]), extract: vi.fn().mockResolvedValue({}),
    };
    const generator = new LibSkillGenerator(model, makeProfile(), new SkillValidator(model, makeProfile()));
    await expect(generator.generate({ name: "rokkit", source_type: "llms.txt" }, [])).rejects.toThrow("3 attempts");
  });
});
```

- [ ] **Step 2: Add writeLibSkill test to packages/engine/src/agent/claude-adapter.spec.ts**

Inside the existing `describe("ClaudeAdapter", ...)` block, add:

```typescript
it("writeLibSkill writes sensei-{slug}-lib-{name}.md and returns LibSkillFile", async () => {
  tmpDir = await mkdtemp(join(tmpdir(), "sensei-adapter-test-"));
  const adapter = new ClaudeAdapter(tmpDir);

  const result = await adapter.writeLibSkill("rokkit", "---\nname: myrepo-lib-rokkit\n---\n# Rokkit", "my-repo");

  expect(result.libName).toBe("rokkit");
  expect(result.path).toBe(join(tmpDir, "sensei-my-repo-lib-rokkit.md"));
  expect(result.generatedAt).toBeTruthy();
  const { readFile } = await import("fs/promises");
  const content = await readFile(result.path, "utf-8");
  expect(content).toContain("# Rokkit");
});
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd packages/engine && bunx vitest run src/lib/lib-skill-generator.spec.ts src/agent/claude-adapter.spec.ts
```
Expected: both FAIL

- [ ] **Step 4: Create lib-skill-generator.ts**

```typescript
// packages/engine/src/lib/lib-skill-generator.ts
import type { ModelBackend, ProjectProfile, LibEntry, DocPage } from "@sensei/shared";
import type { SkillValidator } from "../skill-gen/skill-validator.js";

function buildPrompt(entry: LibEntry, pages: DocPage[], profile: ProjectProfile): string {
  const slug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const topSections = pages.slice(0, 20).map(p => `- ${p.title}: ${p.description}`).join("\n");
  const relevantSymbols = profile.keySymbols
    .filter(s => s.toLowerCase().includes(entry.name.toLowerCase()))
    .join(", ");

  return `Generate a skill file in SKILL.md format for the library "${entry.name}".

Project: ${profile.repoName}
Library description: ${entry.description ?? entry.name}
Relevant project symbols: ${relevantSymbols || "none detected"}
Project config: ${profile.senseiConfig.slice(0, 300)}

Top documentation sections:
${topSections || "(no sections available)"}

Teach an AI agent how to use "${entry.name}" in the context of the ${profile.repoName} project.
Include: what the library does, key APIs and components, usage patterns for this project.

Output exactly this format:

---
name: ${slug}-lib-${entry.name}
description: Use when working with ${entry.name} in the ${profile.repoName} project.
---

# ${entry.name} Library Guide

[skill content here — 150-300 words]`;
}

export class LibSkillGenerator {
  constructor(
    private readonly model: ModelBackend,
    private readonly profile: ProjectProfile,
    private readonly validator: SkillValidator,
  ) {}

  async generate(entry: LibEntry, pages: DocPage[]): Promise<string> {
    const category = `lib-${entry.name}`;
    let skillMarkdown = await this.model.generate(buildPrompt(entry, pages, this.profile));

    for (let attempt = 0; attempt < 3; attempt++) {
      const { valid, issues } = await this.validator.validate(category, skillMarkdown);
      if (valid) return skillMarkdown;

      if (attempt === 2) {
        throw new Error(
          `Failed to generate valid ${entry.name} skill after 3 attempts. Issues: ${issues.join("; ")}`,
        );
      }

      const retryPrompt =
        buildPrompt(entry, pages, this.profile) +
        `\n\nPrevious attempt was rejected. Fix these issues:\n${issues.join("\n")}`;
      skillMarkdown = await this.model.generate(retryPrompt);
    }

    return skillMarkdown;
  }
}
```

- [ ] **Step 5: Add writeLibSkill to ClaudeAdapter**

In `packages/engine/src/agent/claude-adapter.ts`:
- Add `LibSkillFile` to the import from `@sensei/shared`
- Add the method to the class:

```typescript
// Update the import at line 5:
import type { AgentSkillFile, LibSkillFile } from "@sensei/shared";

// Add method inside ClaudeAdapter class (after installedSkills):
async writeLibSkill(
  libName: string,
  markdown: string,
  repoSlug: string,
): Promise<LibSkillFile> {
  await mkdir(this.skillsDir, { recursive: true });
  const fileName = `sensei-${repoSlug}-lib-${libName}.md`;
  const filePath = join(this.skillsDir, fileName);
  await writeFile(filePath, markdown, "utf-8");
  return { libName, path: filePath, generatedAt: new Date().toISOString() };
}
```

- [ ] **Step 6: Add lib/* exports to engine/src/index.ts**

Append to `packages/engine/src/index.ts`:

```typescript
export * from "./lib/source-adapter.js";
export * from "./lib/llms-txt-adapter.js";
export * from "./lib/http-adapter.js";
export * from "./lib/local-adapter.js";
export * from "./lib/lib-indexer.js";
export * from "./lib/lib-skill-generator.js";
```

- [ ] **Step 7: Run all engine tests**

```bash
cd packages/engine && bun test
```
Expected: all PASS

- [ ] **Step 8: Commit**

```bash
git add packages/engine/src/lib/lib-skill-generator.ts packages/engine/src/lib/lib-skill-generator.spec.ts \
  packages/engine/src/agent/claude-adapter.ts packages/engine/src/agent/claude-adapter.spec.ts \
  packages/engine/src/index.ts
git commit -m "feat(engine): add LibSkillGenerator, ClaudeAdapter.writeLibSkill, lib/* exports"
```

---

## Chunk 4: CLI + MCP

### Task 8: CLI update-registry Command

**Files:**
- Create: `packages/cli/src/commands/update-registry.ts`
- Modify: `packages/cli/src/cli.ts` — add update-registry case + help text

- [ ] **Step 1: Create update-registry.ts**

```typescript
// packages/cli/src/commands/update-registry.ts
import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { intro, outro, log, spinner, cancel } from "@clack/prompts";
import {
  extractProjectProfile,
  LibIndexer,
  LibSkillGenerator,
  SkillValidator,
  ClaudeAdapter,
  LlmsTxtAdapter,
  HttpAdapter,
  LocalAdapter,
  type SourceAdapter,
} from "@sensei/engine";
import { ClaudeBackend, OllamaBackend } from "@sensei/server";
import { makeSenseiClient, loadSenseiConfig, type LibEntry, type LibSkillsManifest } from "@sensei/shared";

function createAdapter(sourceType: LibEntry["source_type"]): SourceAdapter {
  if (sourceType === "llms.txt") return new LlmsTxtAdapter();
  if (sourceType === "http") return new HttpAdapter();
  return new LocalAdapter();
}

export async function updateRegistry(repoPath: string): Promise<void> {
  intro("sensei update-registry");

  const config = await loadSenseiConfig(repoPath);
  if (!config) {
    cancel("Not initialised — run sensei init first");
    return;
  }

  if (!config.custom_libs?.length) {
    log.info("No custom_libs configured in .sensei/config.yaml");
    outro("Nothing to do.");
    return;
  }

  const client = await makeSenseiClient(repoPath);
  if (!client) {
    cancel("Supabase client not configured. Run sensei init first.");
    return;
  }

  const repoId = config.repo_id;

  const profileSpinner = spinner();
  profileSpinner.start("Analysing project...");
  const profile = await extractProjectProfile(client as any, repoId, repoPath);
  profileSpinner.stop(`Project analysed: ${profile.dominantLanguage}`);

  const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const ollamaBackend = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });

  // Load existing manifest to preserve entries for other libs
  const manifestPath = join(repoPath, ".sensei", "lib-skills.json");
  let manifest: LibSkillsManifest = { repoSlug, skills: [], updatedAt: new Date().toISOString() };
  try {
    const raw = await readFile(manifestPath, "utf-8");
    manifest = JSON.parse(raw) as LibSkillsManifest;
  } catch { /* start fresh */ }

  const hasAnthropicKey = Boolean(process.env.ANTHROPIC_API_KEY);
  let claudeBackend: ClaudeBackend | null = null;

  for (const lib of config.custom_libs) {
    // Fetch
    const fetchSpin = spinner();
    fetchSpin.start(`Fetching ${lib.name}...`);
    let pages;
    try {
      pages = await createAdapter(lib.source_type).fetch(lib);
      fetchSpin.stop(`Fetched ${lib.name}: ${pages.length} pages`);
    } catch (err) {
      fetchSpin.stop(`Error fetching ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }

    // Index
    const indexSpin = spinner();
    indexSpin.start(`Indexing ${lib.name} (${pages.length} pages)...`);
    try {
      const { sectionsIndexed } = await new LibIndexer(client as any, ollamaBackend).index(repoId, lib, pages);
      indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed`);
    } catch (err) {
      indexSpin.stop(`Error indexing ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }

    // Generate skill (only if ANTHROPIC_API_KEY present)
    if (hasAnthropicKey) {
      const skillSpin = spinner();
      skillSpin.start(`Generating skill for ${lib.name}...`);
      try {
        if (!claudeBackend) {
          claudeBackend = new ClaudeBackend();
          await claudeBackend.init();
        }
        const validator = new SkillValidator(claudeBackend, profile);
        const markdown = await new LibSkillGenerator(claudeBackend, profile, validator).generate(lib, pages);
        const libSkillFile = await new ClaudeAdapter().writeLibSkill(lib.name, markdown, repoSlug);

        manifest.skills = manifest.skills.filter(s => s.libName !== lib.name);
        manifest.skills.push(libSkillFile);
        manifest.updatedAt = new Date().toISOString();
        await mkdir(join(repoPath, ".sensei"), { recursive: true });
        await writeFile(manifestPath, JSON.stringify(manifest, null, 2), "utf-8");

        skillSpin.stop(`Skill written: ${libSkillFile.path}`);
      } catch (err) {
        skillSpin.stop(`Skill generation skipped for ${lib.name}`);
        log.warn(`  ${err instanceof Error ? err.message : String(err)}`);
      }
    }
  }

  outro(`Done. ${config.custom_libs.length} librar${config.custom_libs.length === 1 ? "y" : "ies"} processed.`);
}
```

- [ ] **Step 2: Add update-registry to cli.ts switch**

In `packages/cli/src/cli.ts`, add before the `default` case:

```typescript
case "update-registry": {
  const { updateRegistry } = await import("./commands/update-registry.js");
  await updateRegistry(repoRoot);
  break;
}
```

Add to the HELP `Commands:` section:
```
  update-registry          Index custom_libs from .sensei/config.yaml into Supabase
```

- [ ] **Step 3: Verify CLI compiles**

```bash
cd packages/cli && bunx tsc --noEmit
```
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add packages/cli/src/commands/update-registry.ts packages/cli/src/cli.ts
git commit -m "feat(cli): add sensei update-registry command"
```

---

### Task 9: get-lib-docs MCP Tool + Registration

**Files:**
- Create: `packages/server/src/tools/get-lib-docs.ts`
- Create: `packages/server/src/tools/get-lib-docs.spec.ts`
- Modify: `packages/server/src/mcp-server.ts` — import + register get_lib_docs

- [ ] **Step 1: Write failing test**

```typescript
// packages/server/src/tools/get-lib-docs.spec.ts
import { describe, it, expect, vi } from "vitest";
import { getLibDocsTool } from "./get-lib-docs.js";
import type { ModelBackend } from "@sensei/shared";

const makeMockBackend = (): ModelBackend => ({
  name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(""), embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  extract: vi.fn().mockResolvedValue({}),
});

const ROW = { title: "Button", url: "https://rokkit.dev/button", local_path: null, description: "A button", content: null, source_type: "llms.txt", component: "Forms" };

describe("getLibDocsTool", () => {
  it("embeds query and calls match_lib_doc_sections RPC", async () => {
    const db = {
      rpc: vi.fn().mockResolvedValue({ data: [ROW], error: null }),
      from: vi.fn(),
    };
    const backend = makeMockBackend();

    const result = await getLibDocsTool(db as any, backend, "repo-1", "rokkit", { query: "button" });

    expect(backend.embed).toHaveBeenCalledWith("button");
    expect(db.rpc).toHaveBeenCalledWith("match_lib_doc_sections", expect.objectContaining({
      p_repo_id: "repo-1", p_lib_name: "rokkit",
    }));
    expect(result.lib).toBe("rokkit");
    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
  });

  it("returns all sections sorted by title when no query provided", async () => {
    const rows = [{ ...ROW, title: "Select" }, { ...ROW, title: "Button" }];
    const orderMock = vi.fn().mockResolvedValue({ data: rows, error: null });
    const db = {
      rpc: vi.fn(),
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({ eq: vi.fn().mockReturnValue({ order: orderMock }) }),
        }),
      }),
    };
    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit");
    expect(result.sections).toHaveLength(2);
  });

  it("returns empty sections on any error — never throws", async () => {
    const db = { rpc: vi.fn().mockResolvedValue({ data: null, error: { message: "DB error" } }), from: vi.fn() };
    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "test" });
    expect(result.sections).toEqual([]);
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd packages/server && bunx vitest run src/tools/get-lib-docs.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Create get-lib-docs.ts**

```typescript
// packages/server/src/tools/get-lib-docs.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ModelBackend, DocPage } from "@sensei/shared";

export async function getLibDocsTool(
  db: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  lib: string,
  opts?: { component?: string; query?: string; limit?: number },
): Promise<{ lib: string; sections: DocPage[] }> {
  const limit = opts?.limit ?? 10;
  try {
    let rows: Record<string, unknown>[];

    if (opts?.query) {
      const embedding = await backend.embed(opts.query);
      const { data, error } = await db.rpc("match_lib_doc_sections", {
        p_repo_id: repoId,
        p_lib_name: lib,
        p_component: opts?.component ?? null,
        query_embedding: embedding,
        match_count: limit,
      });
      if (error) throw new Error(error.message);
      rows = (data ?? []) as Record<string, unknown>[];
    } else {
      let query = db
        .from("lib_doc_sections")
        .select("title,url,local_path,description,content,source_type,component")
        .eq("repo_id", repoId)
        .eq("lib_name", lib);

      if (opts?.component) query = query.eq("component", opts.component) as typeof query;

      const { data, error } = await query.order("title");
      if (error) throw new Error(error.message);
      rows = (data ?? []) as Record<string, unknown>[];
    }

    const sections: DocPage[] = rows.map(r => ({
      title: r.title as string,
      url: (r.url as string | null) ?? undefined,
      localPath: (r.local_path as string | null) ?? undefined,
      description: r.description as string,
      content: (r.content as string | null) ?? undefined,
      sourceType: r.source_type as DocPage["sourceType"],
      component: (r.component as string | null) ?? undefined,
    }));

    return { lib, sections };
  } catch {
    return { lib, sections: [] };
  }
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cd packages/server && bunx vitest run src/tools/get-lib-docs.spec.ts
```
Expected: PASS

- [ ] **Step 5: Register get_lib_docs in mcp-server.ts**

In `packages/server/src/mcp-server.ts`:

Add import (alongside existing tool imports):
```typescript
import { getLibDocsTool } from "./tools/get-lib-docs.js";
```

Add tool registration (inside `createSenseiMcpServer`, after the `install_skills` tool):
```typescript
server.tool(
  "get_lib_docs",
  "Retrieve indexed documentation sections for a library used in this repo. Use for third-party library API lookups.",
  {
    lib:       z.string().describe("Library name as registered in custom_libs"),
    component: z.string().optional().describe("Optional component/section filter"),
    query:     z.string().optional().describe("Semantic search query — omit to list all sections"),
    limit:     z.number().int().min(1).max(50).optional().default(10),
  },
  async ({ lib, component, query, limit }) => {
    try {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
      const result = await getLibDocsTool(client as any, getBackend(), opts.repoId, lib, { component, query, limit });
      beat(client, "get_lib_docs", true);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
    }
  }
);
```

- [ ] **Step 6: Run all server tests**

```bash
cd packages/server && bun test
```
Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add packages/server/src/tools/get-lib-docs.ts packages/server/src/tools/get-lib-docs.spec.ts packages/server/src/mcp-server.ts
git commit -m "feat(server): add get_lib_docs MCP tool and register in mcp-server"
```

---

## Chunk 5: Dashboard

### Task 10: Dashboard Libraries Pages

**Files:**
- Create: `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`
- Create: `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte`
- Create: `apps/dashboard/src/routes/repos/[id]/libraries/[name]/+page.server.ts`
- Create: `apps/dashboard/src/routes/repos/[id]/libraries/[name]/+page.svelte`
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.svelte` — add Library Docs link

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p apps/dashboard/src/routes/repos/\[id\]/libraries/\[name\]
```

- [ ] **Step 2: Create libraries list page server**

```typescript
// apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts
import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { LibSkillsManifest, LibSkillFile } from '@sensei/shared';

type Freshness = 'fresh' | 'stale' | 'missing';

interface LibRow {
  libName: string;
  sourceType: string;
  sectionCount: number;
  lastFetched: string | null;
  freshness: Freshness;
  skill: LibSkillFile | null;
}

const STALE_DAYS = 7;

function computeFreshness(lastFetched: string | null): Freshness {
  if (!lastFetched) return 'missing';
  const ageMs = Date.now() - new Date(lastFetched).getTime();
  return ageMs / (1000 * 60 * 60 * 24) > STALE_DAYS ? 'stale' : 'fresh';
}

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  const repoPath = (repo as { id: string; name: string; local_path: string }).local_path;

  const { data: sections } = await db
    .from('lib_doc_sections')
    .select('lib_name,source_type,last_fetched')
    .eq('repo_id', params.id);

  // Group by lib_name
  const libMap = new Map<string, { sourceType: string; count: number; lastFetched: string | null }>();
  for (const row of (sections ?? []) as Array<{ lib_name: string; source_type: string; last_fetched: string }>) {
    const existing = libMap.get(row.lib_name);
    if (!existing) {
      libMap.set(row.lib_name, { sourceType: row.source_type, count: 1, lastFetched: row.last_fetched });
    } else {
      existing.count++;
      if (!existing.lastFetched || row.last_fetched > existing.lastFetched) {
        existing.lastFetched = row.last_fetched;
      }
    }
  }

  // Load lib-skills manifest
  const manifestPath = join(repoPath, '.sensei', 'lib-skills.json');
  const skillsByLib = new Map<string, LibSkillFile>();
  if (existsSync(manifestPath)) {
    try {
      const raw = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(raw) as LibSkillsManifest;
      for (const s of manifest.skills) skillsByLib.set(s.libName, s);
    } catch { /* ignore malformed */ }
  }

  const libs: LibRow[] = Array.from(libMap.entries()).map(([libName, info]) => ({
    libName,
    sourceType: info.sourceType,
    sectionCount: info.count,
    lastFetched: info.lastFetched,
    freshness: computeFreshness(info.lastFetched),
    skill: skillsByLib.get(libName) ?? null,
  }));

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
    hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  };
};

export const actions: Actions = {
  update: async ({ params }) => {
    const db = getDb();
    const { data: repo } = await db.from('repos').select('local_path').eq('id', params.id).single();
    if (!repo) return fail(404, { error: 'Repo not found' });

    const repoPath = (repo as { local_path: string }).local_path;
    try {
      const proc = Bun.spawn(
        ['sensei', 'update-registry'],
        { cwd: repoPath, env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' }
      );
      const timeout = setTimeout(() => proc.kill(), 60_000);
      const exitCode = await proc.exited;
      clearTimeout(timeout);

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || 'sensei update-registry failed' });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei update-registry' });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },
};
```

- [ ] **Step 3: Create libraries list page svelte**

```svelte
<!-- apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte -->
<script lang="ts">
  import type { PageData, ActionData } from './$types';
  const { data, form }: { data: PageData; form: ActionData } = $props();

  const freshnessColor: Record<string, string> = {
    fresh: 'status-fresh', stale: 'status-stale', missing: 'status-missing',
  };
  const freshnessLabel: Record<string, string> = {
    fresh: 'Fresh', stale: 'Stale', missing: 'Missing',
  };

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    return new Date(iso).toLocaleString();
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Library Docs</h1>

{#if form?.error}
  <p class="error">{form.error}</p>
{/if}

{#if data.libs.length > 0}
  <table>
    <thead>
      <tr>
        <th>Library</th>
        <th>Source</th>
        <th>Sections</th>
        <th>Last Fetched</th>
        <th>Freshness</th>
        {#if data.hasAnthropicKey}<th>Skill</th>{/if}
      </tr>
    </thead>
    <tbody>
      {#each data.libs as lib}
        <tr>
          <td><a href="/repos/{data.repo.id}/libraries/{lib.libName}">{lib.libName}</a></td>
          <td>{lib.sourceType}</td>
          <td>{lib.sectionCount}</td>
          <td>{formatDate(lib.lastFetched)}</td>
          <td class={freshnessColor[lib.freshness] ?? ''}>{freshnessLabel[lib.freshness] ?? lib.freshness}</td>
          {#if data.hasAnthropicKey}
            <td class={lib.skill ? 'skill-generated' : ''}>{lib.skill ? 'Generated' : 'None'}</td>
          {/if}
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No library docs indexed yet.</p>
  <p><code>Add custom_libs to .sensei/config.yaml then run sensei update-registry</code></p>
{/if}

<form method="POST" action="?/update">
  <button type="submit">Update Registry</button>
</form>

<style>
  .status-fresh   { color: green; }
  .status-stale   { color: goldenrod; }
  .status-missing { color: red; }
  .skill-generated { color: green; }
  .error { color: red; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
</style>
```

- [ ] **Step 4: Create [name] detail page server**

```typescript
// apps/dashboard/src/routes/repos/[id]/libraries/[name]/+page.server.ts
import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getDb } from '$lib/server/db';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import type { LibSkillsManifest } from '@sensei/shared';

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos').select('id,name,local_path').eq('id', params.id).single();

  if (!repo) throw error(404, 'Repo not found');
  const repoPath = (repo as { id: string; name: string; local_path: string }).local_path;

  const { data: sections, error: dbError } = await db
    .from('lib_doc_sections')
    .select('title,url,local_path,description,content,source_type,component,last_fetched')
    .eq('repo_id', params.id)
    .eq('lib_name', params.name)
    .order('title');

  if (dbError) throw error(500, dbError.message);

  const manifestPath = join(repoPath, '.sensei', 'lib-skills.json');
  let skillPath: string | null = null;
  if (existsSync(manifestPath)) {
    try {
      const raw = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(raw) as LibSkillsManifest;
      const found = manifest.skills.find(s => s.libName === params.name);
      if (found && existsSync(found.path)) skillPath = found.path;
    } catch { /* ignore */ }
  }

  return {
    repo: repo as { id: string; name: string },
    libName: params.name,
    sections: sections ?? [],
    skillPath,
  };
};
```

- [ ] **Step 5: Create [name] detail page svelte**

```svelte
<!-- apps/dashboard/src/routes/repos/[id]/libraries/[name]/+page.svelte -->
<script lang="ts">
  import type { PageData } from './$types';
  const { data }: { data: PageData } = $props();

  function shortContent(content: string | null | undefined): string {
    if (!content) return '';
    return content.length > 300 ? content.slice(0, 300) + '...' : content;
  }
</script>

<a href="/repos/{data.repo.id}/libraries">← Library Docs</a>
<h1>{data.libName}</h1>

{#if data.skillPath}
  <p>Skill file: <code>{data.skillPath}</code></p>
{/if}

{#if data.sections.length > 0}
  <p>{data.sections.length} sections indexed</p>
  <table>
    <thead>
      <tr>
        <th>Title</th>
        <th>Component</th>
        <th>Description</th>
        <th>Content / Link</th>
      </tr>
    </thead>
    <tbody>
      {#each data.sections as section}
        <tr>
          <td>{section.title}</td>
          <td>{section.component ?? '—'}</td>
          <td>{section.description}</td>
          <td>
            {#if section.url}
              <a href={section.url} target="_blank">↗ View</a>
            {:else if section.content}
              <details>
                <summary>Preview</summary>
                <pre>{shortContent(section.content)}</pre>
              </details>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No sections indexed for this library.</p>
{/if}

<style>
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; vertical-align: top; }
  pre { white-space: pre-wrap; font-size: 0.85em; max-width: 50ch; }
</style>
```

- [ ] **Step 6: Add Library Docs link to [id]/+page.svelte**

In `apps/dashboard/src/routes/repos/[id]/+page.svelte`, after the line:
```svelte
<p><a href="/repos/{data.repo.id}/agents">Agent Skills →</a></p>
```
Add:
```svelte
<p><a href="/repos/{data.repo.id}/libraries">Library Docs →</a></p>
```

- [ ] **Step 7: Verify dashboard builds without errors**

```bash
cd apps/dashboard && bun run check
```
Expected: no type errors

- [ ] **Step 8: Start dashboard and verify pages render**

```bash
cd apps/dashboard && bun run dev
```
Navigate to `http://localhost:5173/repos/<id>/libraries` — page should render (empty state is fine if no libs indexed yet).

- [ ] **Step 9: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/libraries/ apps/dashboard/src/routes/repos/\[id\]/+page.svelte
git commit -m "feat(dashboard): add library docs pages with freshness badges and Update Registry action"
```
