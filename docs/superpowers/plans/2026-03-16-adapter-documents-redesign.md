# Adapter & Document Model Redesign Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix library indexing so all source types (llms.txt, http, github) discover and fetch full content for every document, and introduce a Library → Documents → Sections hierarchy for richer retrieval and inline dashboard browsing.

**Architecture:** A new DB migration renames five tables and introduces `documents_in_library` as an intermediate level. All adapters are rewritten with a discover+fetch pattern internally: discover all document URLs/paths, then fetch each one to a full-markdown `DocPage`. `LibIndexer.indexShared` stores one `documents_in_library` row per `DocPage`, then splits the content at H2 headings into `sections_in_document` rows. The dashboard shows a Documents tab with inline markdown rendering; `getLibDocsTool` returns sections enriched with parent document context.

**Tech Stack:** TypeScript, Vitest, SvelteKit, Supabase/PostgreSQL, `@mozilla/readability`, `turndown`, `jsdom`, `marked` (markdown rendering in UI)

---

## Chunk 1: Database Migration + Shared Type Update

### Task 1: DB Migration — Table Renames + New Tables + Updated RPC

**Files:**
- Create: `supabase/migrations/20260316000004_adapter_documents_redesign.sql`

This migration does the heavy lifting. It must be run once against the live Supabase DB.

- [ ] **Step 1: Write the migration file**

```sql
-- supabase/migrations/20260316000004_adapter_documents_redesign.sql

-- ─── 1. Rename tables ─────────────────────────────────────────────────────────

alter table sensei.shared_libs         rename to libraries;
alter table sensei.shared_lib_sections rename to sections_in_document;
alter table sensei.repo_libs           rename to referenced_libraries;
alter table sensei.lib_queries         rename to queries_on_library;

-- ─── 2. Rename FK columns to match new names ─────────────────────────────────

-- referenced_libraries: shared_lib_id → library_id
alter table sensei.referenced_libraries rename column shared_lib_id to library_id;

-- queries_on_library: shared_lib_id → library_id
alter table sensei.queries_on_library rename column shared_lib_id to library_id;

-- sections_in_document: shared_lib_id → library_id (kept denormalized for query perf)
alter table sensei.sections_in_document rename column shared_lib_id to library_id;

-- ─── 3. Add document_count to libraries ──────────────────────────────────────

alter table sensei.libraries
  add column if not exists document_count int not null default 0;

-- ─── 4. Create documents_in_library ──────────────────────────────────────────

create table sensei.documents_in_library (
  id           uuid primary key default gen_random_uuid(),
  library_id   uuid not null references sensei.libraries(id) on delete cascade,
  sequence     int  not null default 0,
  title        text not null,
  url          text,
  local_path   text,
  summary      text not null default '',
  component    text,
  source_type  text not null,
  last_fetched timestamptz not null default now(),
  embedding    vector(384)
);

-- ─── 5. Add document_id + sequence to sections_in_document ───────────────────

alter table sensei.sections_in_document
  add column if not exists document_id uuid references sensei.documents_in_library(id) on delete cascade;

alter table sensei.sections_in_document
  add column if not exists sequence int not null default 0;

-- ─── 6. Remove columns moved to documents_in_library ─────────────────────────
-- (Do this AFTER creating documents_in_library, but sections without document_id
--  are orphans from old data — acceptable, they'll be re-indexed)

alter table sensei.sections_in_document drop column if exists url;
alter table sensei.sections_in_document drop column if exists local_path;
alter table sensei.sections_in_document drop column if exists source_type;
alter table sensei.sections_in_document drop column if exists component;
alter table sensei.sections_in_document drop column if exists description;

-- ─── 7. Drop old RPC ─────────────────────────────────────────────────────────

drop function if exists sensei.match_shared_lib_sections(uuid, text, vector(384), int);

-- ─── 8. Create new RPC match_libraries_sections ──────────────────────────────

create or replace function sensei.match_libraries_sections(
  p_library_id    uuid,
  p_component     text,
  query_embedding vector(384),
  match_count     int default 10
)
returns table (
  section_id    uuid,
  section_title text,
  content       text,
  similarity    float,
  doc_id        uuid,
  doc_title     text,
  url           text,
  local_path    text,
  component     text,
  summary       text
)
language sql stable
as $$
  select
    s.id          as section_id,
    s.title       as section_title,
    s.content,
    1 - (s.embedding <=> query_embedding) as similarity,
    d.id          as doc_id,
    d.title       as doc_title,
    d.url,
    d.local_path,
    d.component,
    d.summary
  from sensei.sections_in_document s
  join sensei.documents_in_library d on d.id = s.document_id
  where s.library_id = p_library_id
    and (p_component is null or d.component = p_component)
    and s.embedding is not null
  order by s.embedding <=> query_embedding
  limit match_count;
$$;

-- ─── 9. Index for document lookup ────────────────────────────────────────────

create index if not exists documents_in_library_library_id_idx
  on sensei.documents_in_library (library_id);

create index if not exists sections_in_document_document_id_idx
  on sensei.sections_in_document (document_id);
```

- [ ] **Step 2: Apply migration to Supabase**

```bash
# From repo root
bunx supabase db push
```

Expected: migration applies without errors. If Supabase CLI is not linked, apply via the Supabase dashboard SQL editor by pasting the migration file content.

- [ ] **Step 3: Commit**

```bash
git add supabase/migrations/20260316000004_adapter_documents_redesign.sql
git commit -m "feat(db): rename tables, add documents_in_library, update match RPC"
```

---

### Task 2: Update DocPage Type + LibEntry

**Files:**
- Modify: `packages/shared/src/types.ts`

- [ ] **Step 1: Update the DocPage interface**

Find the `DocPage` interface and replace it:

```typescript
export interface DocPage {
  title: string;
  url?: string;            // remote sources
  localPath?: string;      // local sources
  summary: string;         // ≤200 chars — curated or auto-extracted (renamed from description)
  content: string;         // REQUIRED — full markdown
  sourceType: 'llms.txt' | 'http' | 'local' | 'github';
  component?: string;
  sequence?: number;       // hint from adapter (order in index)
}
```

- [ ] **Step 2: Remove `description` from LibEntry** (it was `description?: string` — remove, no callers use it)

The `LibEntry` interface's `description?: string` field is unused. Leave the rest of LibEntry unchanged.

- [ ] **Step 3: Run engine tests to see what breaks**

```bash
cd packages/engine && bunx vitest run 2>&1 | head -60
```

Expected: Several tests fail because they reference `page.description` or construct `DocPage` with `description`. Note each failing file.

- [ ] **Step 4: Commit the type change**

```bash
git add packages/shared/src/types.ts
git commit -m "feat(shared): rename DocPage.description→summary, make content required"
```

---

## Chunk 2: Engine Utilities + Adapter Rewrites

### Task 3: Create Shared Utilities (doc-utils.ts)

**Files:**
- Create: `packages/engine/src/lib/doc-utils.ts`
- Create: `packages/engine/src/lib/doc-utils.spec.ts`

- [ ] **Step 1: Write the failing tests first**

```typescript
// packages/engine/src/lib/doc-utils.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { resolveUrl, fetchAsMarkdown, extractSummary, splitSections } from "./doc-utils.js";

describe("resolveUrl", () => {
  it("resolves relative URL against base", () => {
    expect(resolveUrl("https://example.com/docs/", "../api/intro")).toBe("https://example.com/api/intro");
  });

  it("returns absolute URL unchanged", () => {
    expect(resolveUrl("https://example.com/docs/", "https://other.com/page")).toBe("https://other.com/page");
  });

  it("resolves root-relative path", () => {
    expect(resolveUrl("https://example.com/docs/guide", "/api/ref")).toBe("https://example.com/api/ref");
  });
});

describe("fetchAsMarkdown", () => {
  afterEach(() => vi.restoreAllMocks());

  it("returns body as-is when URL ends in .md", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/html" },
      text: () => Promise.resolve("# Markdown"),
    }));
    const result = await fetchAsMarkdown("https://example.com/docs/README.md");
    expect(result).toBe("# Markdown");
  });

  it("returns body as-is when content-type is text/plain", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/plain" },
      text: () => Promise.resolve("# Plain text"),
    }));
    const result = await fetchAsMarkdown("https://example.com/llms.txt");
    expect(result).toBe("# Plain text");
  });

  it("converts HTML to markdown via Readability+Turndown for text/html", async () => {
    const html = `<html><body><article><h1>Title</h1><p>Paragraph text.</p></article></body></html>`;
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/html" },
      text: () => Promise.resolve(html),
    }));
    const result = await fetchAsMarkdown("https://example.com/page");
    expect(result).toContain("Paragraph text");
    expect(result).not.toContain("<p>");
  });

  it("throws on HTTP error status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, status: 404,
      headers: { get: () => null },
      text: () => Promise.resolve(""),
    }));
    await expect(fetchAsMarkdown("https://example.com/missing")).rejects.toThrow("404");
  });
});

describe("extractSummary", () => {
  it("returns first non-heading paragraph trimmed to ≤200 chars", () => {
    const md = `# Title\n\nThis is the first paragraph with content.\n\n## Section\n\nMore content.`;
    expect(extractSummary(md)).toBe("This is the first paragraph with content.");
  });

  it("joins multi-line paragraph into single string", () => {
    const md = `# Title\n\nLine one.\nLine two.\nLine three.`;
    expect(extractSummary(md)).toBe("Line one. Line two. Line three.");
  });

  it("skips heading lines", () => {
    const md = `## Heading\n\nActual content here.`;
    expect(extractSummary(md)).toBe("Actual content here.");
  });

  it("truncates to ≤200 chars", () => {
    const long = "A".repeat(300);
    const md = `# Title\n\n${long}`;
    const result = extractSummary(md);
    expect(result.length).toBeLessThanOrEqual(200);
  });

  it("returns empty string when no non-heading paragraph", () => {
    const md = `# Just a heading`;
    expect(extractSummary(md)).toBe("");
  });
});

describe("splitSections", () => {
  const MD = `# Title\n\nIntro paragraph.\n\n## Installation\n\nRun npm install.\n\n## Usage\n\nCall init().`;

  it("splits at H2 headings", () => {
    const sections = splitSections(MD);
    const titles = sections.map(s => s.title);
    expect(titles).toContain("Installation");
    expect(titles).toContain("Usage");
  });

  it("includes Overview section for pre-H2 content", () => {
    const sections = splitSections(MD);
    expect(sections[0].title).toBe("Overview");
    expect(sections[0].content).toContain("Intro paragraph");
    expect(sections[0].sequence).toBe(0);
  });

  it("omits Overview when pre-H2 content is empty/whitespace", () => {
    const md = `## Installation\n\nRun npm install.`;
    const sections = splitSections(md);
    expect(sections[0].title).toBe("Installation");
  });

  it("assigns sequential sequence numbers", () => {
    const sections = splitSections(MD);
    sections.forEach((s, i) => expect(s.sequence).toBe(i));
  });

  it("omits sections with empty content after whitespace trim", () => {
    const md = `## Empty\n\n   \n\n## Real\n\nActual content.`;
    const sections = splitSections(md);
    expect(sections.map(s => s.title)).not.toContain("Empty");
    expect(sections.map(s => s.title)).toContain("Real");
  });

  it("includes H3+ content within the enclosing H2 section", () => {
    const md = `## Usage\n\n### Sub\n\nSub content.`;
    const sections = splitSections(md);
    expect(sections[0].content).toContain("Sub content");
  });

  it("returns empty array for empty input", () => {
    expect(splitSections("")).toHaveLength(0);
    expect(splitSections("   ")).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/engine && bunx vitest run src/lib/doc-utils.spec.ts 2>&1 | tail -10
```

Expected: FAIL — "Cannot find module './doc-utils.js'"

- [ ] **Step 3: Implement doc-utils.ts**

```typescript
// packages/engine/src/lib/doc-utils.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";

const td = new TurndownService({ headingStyle: "atx" });

/** Resolve a relative URL against a base URL. */
export function resolveUrl(base: string, relative: string): string {
  return new URL(relative, base).href;
}

/**
 * Fetch a URL and return its content as Markdown.
 * - .md extension or text/plain|text/markdown content-type → returned as-is
 * - Otherwise → Readability + Turndown (HTML → Markdown)
 * Throws on network or HTTP error. Callers handle gracefully.
 */
export async function fetchAsMarkdown(url: string): Promise<string> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`fetchAsMarkdown: HTTP ${res.status} for ${url}`);

  const contentType = res.headers.get("content-type") ?? "";
  const body = await res.text();

  const isMarkdown =
    url.endsWith(".md") ||
    contentType.includes("text/plain") ||
    contentType.includes("text/markdown");

  if (isMarkdown) return body;

  const dom = new JSDOM(body, { url });
  const reader = new Readability(dom.window.document);
  const article = reader.parse();
  return td.turndown(article?.content ?? body);
}

/** First non-heading paragraph, trimmed to ≤200 chars. */
export function extractSummary(markdown: string): string {
  const lines = markdown.split("\n");
  const buf: string[] = [];
  let inParagraph = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("#")) {
      if (inParagraph) break; // heading after paragraph started — stop
      continue;
    }
    if (trimmed === "") {
      if (inParagraph) break; // blank line after paragraph — end of first paragraph
      continue;
    }
    inParagraph = true;
    buf.push(trimmed);
  }

  return buf.join(" ").slice(0, 200);
}

/**
 * Split markdown at H2 headings into sections.
 * - Content before first ## → title "Overview" (omit if empty after trim)
 * - Each ## heading → one section (includes all H3+ sub-content within it)
 * - sequence is 0-based index within THIS document (independent per document)
 * - Sections with empty content after whitespace trim are omitted.
 */
export function splitSections(
  markdown: string,
): Array<{ title: string; content: string; sequence: number }> {
  if (!markdown.trim()) return [];

  const blocks = markdown.split(/\n(?=## )/);
  const result: Array<{ title: string; content: string; sequence: number }> = [];
  let sequence = 0;

  for (const block of blocks) {
    const trimmed = block.trim();
    if (!trimmed) continue;

    if (trimmed.startsWith("## ")) {
      const lines = trimmed.split("\n");
      const title = lines[0].replace(/^##\s+/, "").trim();
      const content = lines.slice(1).join("\n").trim();
      if (!content) continue; // omit empty sections
      result.push({ title, content, sequence: sequence++ });
    } else {
      // Pre-H2 content → Overview
      const content = trimmed;
      // Strip leading H1 to get the actual content
      const withoutH1 = content.replace(/^#[^#][^\n]*\n/, "").trim();
      if (!withoutH1) continue;
      result.push({ title: "Overview", content: withoutH1, sequence: sequence++ });
    }
  }

  return result;
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd packages/engine && bunx vitest run src/lib/doc-utils.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Export from index.ts**

Add to `packages/engine/src/index.ts`:
```typescript
export * from "./lib/doc-utils.js";
```

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/doc-utils.ts packages/engine/src/lib/doc-utils.spec.ts packages/engine/src/index.ts
git commit -m "feat(engine): add doc-utils (resolveUrl, fetchAsMarkdown, extractSummary, splitSections)"
```

---

### Task 4: Fix inferSourceType — Add .txt Extension Detection

**Files:**
- Modify: `packages/engine/src/lib/infer-source-type.ts`
- Modify: `packages/engine/src/lib/infer-source-type.spec.ts`

- [ ] **Step 1: Add the failing test**

Add to the existing `describe("inferSourceType")` block in `infer-source-type.spec.ts`:

```typescript
it("detects .txt URLs (not ending in /llms.txt) as llms.txt type", () => {
  const r = inferSourceType("https://rokkit.vercel.app/llms/index.txt");
  expect(r.source_type).toBe("llms.txt");
  expect(r.base_url).toBe("https://rokkit.vercel.app/llms/index.txt");
});

it("detects any .txt URL as llms.txt, not just /llms.txt suffix", () => {
  const r = inferSourceType("https://example.com/components.txt");
  expect(r.source_type).toBe("llms.txt");
});
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/engine && bunx vitest run src/lib/infer-source-type.spec.ts
```

Expected: 2 new tests FAIL.

- [ ] **Step 3: Update inferSourceType**

In `packages/engine/src/lib/infer-source-type.ts`, add the `.txt` check before the `http` fallback (after the github check):

```typescript
if (url.pathname.endsWith('/llms.txt') || url.pathname.endsWith('.txt')) {
  return { source_type: 'llms.txt', base_url: input };
}
```

Replace the existing `url.pathname.endsWith('/llms.txt')` check with this combined check.

- [ ] **Step 4: Run tests**

```bash
cd packages/engine && bunx vitest run src/lib/infer-source-type.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/infer-source-type.ts packages/engine/src/lib/infer-source-type.spec.ts
git commit -m "fix(engine): inferSourceType detects any .txt URL as llms.txt type"
```

---

### Task 5: Rewrite LlmsTxtAdapter — Discover + Fetch

**Files:**
- Modify: `packages/engine/src/lib/llms-txt-adapter.ts`
- Modify: `packages/engine/src/lib/llms-txt-adapter.spec.ts`

The adapter now fetches the index file, parses links, then for each link fetches the linked page as full markdown.

- [ ] **Step 1: Rewrite the spec**

Replace `packages/engine/src/lib/llms-txt-adapter.spec.ts` completely:

```typescript
// packages/engine/src/lib/llms-txt-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { LlmsTxtAdapter } from "./llms-txt-adapter.js";
import type { LibEntry } from "@sensei/shared";

const INDEX = `# Rokkit UI
> Component library

## Buttons

- [Button](https://rokkit.dev/docs/button): Primary button component
- [IconButton](https://rokkit.dev/docs/icon-button): Icon-only button

## Forms

- [Input](https://rokkit.dev/docs/input): Text input field
`;

const BUTTON_MD = `# Button\n\n## Overview\n\nFull button docs.\n\n## Props\n\nVariants, size.`;
const ICON_MD = `# IconButton\n\nIcon-only button docs.`;
const INPUT_MD = `# Input\n\nFull input docs.`;

describe("LlmsTxtAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  function mockFetch(responses: Record<string, string>) {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url in responses) {
        return Promise.resolve({
          ok: true,
          // headers needed by fetchAsMarkdown to check content-type
          headers: { get: () => "text/plain" },
          text: () => Promise.resolve(responses[url]),
        });
      }
      return Promise.resolve({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));
  }

  it("fetches index and then each linked page, returning one DocPage per link", async () => {
    mockFetch({
      "https://rokkit.dev/llms/index.txt": INDEX,
      "https://rokkit.dev/docs/button": BUTTON_MD,
      "https://rokkit.dev/docs/icon-button": ICON_MD,
      "https://rokkit.dev/docs/input": INPUT_MD,
    });

    const adapter = new LlmsTxtAdapter();
    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms/index.txt" };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(3);
    expect(pages[0].title).toBe("Button");
    expect(pages[0].url).toBe("https://rokkit.dev/docs/button");
    // summary = index description (authoritative)
    expect(pages[0].summary).toBe("Primary button component");
    // content = fetched markdown from the linked URL
    expect(pages[0].content).toContain("Full button docs");
    expect(pages[0].component).toBe("Buttons");
    expect(pages[0].sequence).toBe(0);
    expect(pages[0].sourceType).toBe("llms.txt");
  });

  it("resolves relative URLs in the index against the index URL", async () => {
    const indexWithRelative = `- [Page](./page): A page`;
    mockFetch({
      "https://rokkit.dev/docs/llms.txt": indexWithRelative,
      "https://rokkit.dev/docs/page": "# Page\n\nContent.",
    });

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "r", source_type: "llms.txt", base_url: "https://rokkit.dev/docs/llms.txt" });

    expect(pages[0].url).toBe("https://rokkit.dev/docs/page");
    expect(pages[0].content).toContain("Content");
  });

  it("gracefully degrades when a linked page fetch fails — uses summary as content", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url === "https://rokkit.dev/llms.txt") {
        return Promise.resolve({ ok: true, headers: { get: () => "text/plain" }, text: () => Promise.resolve(`- [Btn](https://rokkit.dev/btn): Button desc`) });
      }
      return Promise.resolve({ ok: false, status: 500, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "r", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" });

    expect(pages).toHaveLength(1);
    // Falls back to summary when fetch fails
    expect(pages[0].content).toBe("Button desc");
    expect(pages[0].summary).toBe("Button desc");
  });

  it("skips malformed lines gracefully", async () => {
    const malformed = `## Section\n\n- [Valid](https://x.com/v): desc\nnot-a-link\n`;
    // Use mockFetch which adds headers to all responses
    mockFetch({
      "https://x.com/llms.txt": malformed,
      "https://x.com/v": "# Valid\n\nContent.",
    });

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

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/engine && bunx vitest run src/lib/llms-txt-adapter.spec.ts 2>&1 | tail -20
```

Expected: Multiple FAIL — tests expect `summary` (not `description`) and `content` to be fetched.

- [ ] **Step 3: Rewrite llms-txt-adapter.ts**

```typescript
// packages/engine/src/lib/llms-txt-adapter.ts
import { readFile } from "fs/promises";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { resolveUrl, fetchAsMarkdown } from "./doc-utils.js";

const LINK_RE = /^-\s+\[([^\]]+)\]\(([^)]+)\):\s*(.+)$/;
const SECTION_RE = /^##\s+(.+)$/;

interface IndexEntry { title: string; url: string; summary: string; component?: string; }

export class LlmsTxtAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    let text: string;
    const indexUrl = entry.base_url ?? entry.local_path;

    if (entry.base_url?.startsWith("http")) {
      const res = await fetch(entry.base_url);
      if (!res.ok) throw new Error(`LlmsTxtAdapter: fetch failed for ${entry.base_url}: ${res.status}`);
      text = await res.text();
    } else if (entry.local_path) {
      text = await readFile(entry.local_path, "utf-8");
    } else {
      throw new Error(`LlmsTxtAdapter: entry "${entry.name}" must have base_url or local_path`);
    }

    const entries = parseIndex(text, indexUrl!);
    const pages: DocPage[] = [];

    for (let i = 0; i < entries.length; i++) {
      const e = entries[i];
      let content: string;
      try {
        content = await fetchAsMarkdown(e.url);
      } catch (err) {
        console.warn(`[LlmsTxtAdapter] Failed to fetch ${e.url}:`, err instanceof Error ? err.message : String(err));
        content = e.summary; // graceful degradation
      }
      pages.push({
        title: e.title,
        url: e.url,
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

function parseIndex(text: string, indexUrl: string): IndexEntry[] {
  const entries: IndexEntry[] = [];
  let currentSection: string | undefined;

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    const sectionMatch = line.match(SECTION_RE);
    if (sectionMatch) { currentSection = sectionMatch[1].trim(); continue; }
    const linkMatch = line.match(LINK_RE);
    if (!linkMatch) continue;
    const [, title, url, summary] = linkMatch;
    entries.push({
      title: title.trim(),
      url: resolveUrl(indexUrl, url.trim()),
      summary: summary.trim(),
      component: currentSection,
    });
  }
  return entries;
}
```

- [ ] **Step 4: Run tests**

```bash
cd packages/engine && bunx vitest run src/lib/llms-txt-adapter.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/llms-txt-adapter.ts packages/engine/src/lib/llms-txt-adapter.spec.ts
git commit -m "feat(engine): rewrite LlmsTxtAdapter to discover+fetch all linked pages"
```

---

### Task 6: Rewrite HttpAdapter — Link Discovery + Multi-Page Fetch

**Files:**
- Modify: `packages/engine/src/lib/http-adapter.ts`
- Modify: `packages/engine/src/lib/http-adapter.spec.ts`

The adapter now discovers all links at the same path prefix, then fetches each page.

- [ ] **Step 1: Rewrite the spec**

Replace `packages/engine/src/lib/http-adapter.spec.ts` completely:

```typescript
// packages/engine/src/lib/http-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { HttpAdapter } from "./http-adapter.js";
import type { LibEntry } from "@sensei/shared";

// Entry page HTML — links to sub-pages
const DOCS_HTML = `<!DOCTYPE html><html><body>
<nav>
  <a href="/docs/installation">Installation</a>
  <a href="/docs/usage">Usage</a>
  <a href="https://other.com/external">External (skip)</a>
  <a href="/blog/post">Blog (skip — wrong prefix)</a>
</nav>
<article><h1>Docs Home</h1><p>Welcome to the docs.</p></article>
</body></html>`;

const INSTALL_MD = `# Installation\n\nRun npm install mylib to get started.`;
const USAGE_MD = `# Usage\n\n## Quick Start\n\nImport and call init().\n\n## Advanced\n\nSee config options.`;

describe("HttpAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  function mockFetch(responses: Record<string, { body: string; type?: string }>) {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      const entry = responses[url];
      if (!entry) return Promise.resolve({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") });
      const type = entry.type ?? "text/html";
      return Promise.resolve({
        ok: true,
        headers: { get: (h: string) => h === "content-type" ? type : null },
        text: () => Promise.resolve(entry.body),
      });
    }));
  }

  it("discovers links with same path prefix and fetches each as a DocPage", async () => {
    mockFetch({
      "https://kavach.dev/docs": { body: DOCS_HTML },
      "https://kavach.dev/docs/installation": { body: INSTALL_MD, type: "text/markdown" },
      "https://kavach.dev/docs/usage": { body: USAGE_MD, type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev/docs" };
    const pages = await adapter.fetch(entry);

    // Entry URL + discovered sub-pages
    expect(pages.length).toBeGreaterThanOrEqual(2);
    const urls = pages.map(p => p.url);
    expect(urls).toContain("https://kavach.dev/docs/installation");
    expect(urls).toContain("https://kavach.dev/docs/usage");
    // External and off-prefix links are excluded
    expect(urls).not.toContain("https://other.com/external");
    expect(urls).not.toContain("https://kavach.dev/blog/post");
    pages.forEach(p => {
      expect(p.sourceType).toBe("http");
      expect(p.content).toBeTruthy();
      expect(p.summary.length).toBeLessThanOrEqual(200);
    });
  });

  it("assigns component from first path segment after base", async () => {
    const html = `<html><body><a href="/docs/hooks/use-auth">UseAuth</a></body></html>`;
    mockFetch({
      "https://example.com/docs": { body: html },
      "https://example.com/docs/hooks/use-auth": { body: "# UseAuth\n\nAuth hook.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "http", base_url: "https://example.com/docs" });

    const hookPage = pages.find(p => p.url?.includes("use-auth"));
    expect(hookPage?.component).toBe("hooks");
    // Entry URL itself has no component
    const entryPage = pages.find(p => p.url === "https://example.com/docs");
    expect(entryPage?.component).toBeUndefined();
  });

  it("deduplicates URLs", async () => {
    // Same link appears twice in the page
    const html = `<html><body><a href="/docs/page">P1</a><a href="/docs/page">P1 again</a></body></html>`;
    mockFetch({
      "https://x.com/docs": { body: html },
      "https://x.com/docs/page": { body: "# Page\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const withPage = pages.filter(p => p.url?.includes("/page"));
    expect(withPage).toHaveLength(1);
  });

  it("includes entry URL at sequence 0 even if not linked", async () => {
    const html = `<html><body><a href="/docs/sub">Sub</a></body></html>`;
    mockFetch({
      "https://x.com/docs": { body: html },
      "https://x.com/docs/sub": { body: "# Sub\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    expect(pages[0].url).toBe("https://x.com/docs");
    expect(pages[0].sequence).toBe(0);
  });

  it("throws when entry URL fetch returns non-ok status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") }));
    const adapter = new HttpAdapter();
    await expect(adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" })).rejects.toThrow("404");
  });

  it("skips sub-pages that fail to fetch (graceful degradation)", async () => {
    const html = `<html><body><a href="/docs/ok">OK</a><a href="/docs/broken">Broken</a></body></html>`;
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url === "https://x.com/docs") return Promise.resolve({ ok: true, headers: { get: () => "text/html" }, text: () => Promise.resolve(html) });
      if (url.includes("ok")) return Promise.resolve({ ok: true, headers: { get: () => "text/markdown" }, text: () => Promise.resolve("# OK\n\nContent.") });
      return Promise.resolve({ ok: false, status: 500, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const urls = pages.map(p => p.url);
    expect(urls).toContain("https://x.com/docs/ok");
    expect(urls).not.toContain("https://x.com/docs/broken");
  });
});
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/engine && bunx vitest run src/lib/http-adapter.spec.ts 2>&1 | tail -20
```

Expected: Multiple FAIL.

- [ ] **Step 3: Rewrite http-adapter.ts**

```typescript
// packages/engine/src/lib/http-adapter.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { resolveUrl, fetchAsMarkdown, extractSummary } from "./doc-utils.js";

const MAX_PAGES = 100;
const td = new TurndownService({ headingStyle: "atx" });

export class HttpAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`HttpAdapter: entry "${entry.name}" requires base_url`);
    const entryUrl = entry.base_url;

    // Phase 1: Discover — fetch entry URL, extract links at same path prefix.
    // We get the raw response here so we can reuse the body for the entry DocPage
    // without a second fetch.
    const entryRes = await fetch(entryUrl);
    if (!entryRes.ok) throw new Error(`HttpAdapter: HTTP ${entryRes.status} for ${entryUrl}`);
    const entryBody = await entryRes.text();
    const entryContentType = entryRes.headers.get("content-type") ?? "";

    const discovered = discoverLinks(entryBody, entryUrl);

    // Phase 2: Convert entry body to markdown (reuse already-fetched body)
    const pages: DocPage[] = [];
    try {
      const entryMarkdown = bodyToMarkdown(entryBody, entryUrl, entryContentType);
      pages.push(makeDocPage(entryMarkdown, entryUrl, entryUrl, 0));
    } catch (err) {
      console.warn(`[HttpAdapter] Failed to convert entry ${entryUrl}:`, err instanceof Error ? err.message : String(err));
    }

    // Fetch each discovered sub-page
    for (const url of discovered) {
      if (pages.length >= MAX_PAGES) break;
      try {
        const markdown = await fetchAsMarkdown(url);
        pages.push(makeDocPage(markdown, url, entryUrl, pages.length));
      } catch (err) {
        console.warn(`[HttpAdapter] Failed to fetch ${url}:`, err instanceof Error ? err.message : String(err));
      }
    }

    return pages;
  }
}

function bodyToMarkdown(body: string, url: string, contentType: string): string {
  const isMarkdown =
    url.endsWith(".md") ||
    contentType.includes("text/plain") ||
    contentType.includes("text/markdown");
  if (isMarkdown) return body;
  const dom = new JSDOM(body, { url });
  const reader = new Readability(dom.window.document);
  return td.turndown(reader.parse()?.content ?? body);
}

function discoverLinks(html: string, entryUrl: string): string[] {
  const dom = new JSDOM(html, { url: entryUrl });
  const baseUrl = new URL(entryUrl);
  const basePath = baseUrl.pathname;

  const seen = new Set<string>([entryUrl]);
  const links: string[] = [];

  for (const a of Array.from(dom.window.document.querySelectorAll("a[href]"))) {
    const href = (a as HTMLAnchorElement).getAttribute("href");
    if (!href) continue;
    let absolute: string;
    try {
      absolute = resolveUrl(entryUrl, href);
    } catch {
      continue;
    }
    const parsed = new URL(absolute);
    // Must be same origin, same path prefix, not already seen
    if (parsed.hostname !== baseUrl.hostname) continue;
    if (!parsed.pathname.startsWith(basePath)) continue;
    if (seen.has(absolute)) continue;
    seen.add(absolute);
    links.push(absolute);
    if (links.length >= MAX_PAGES - 1) break; // leave room for entry URL
  }
  return links;
}

function makeDocPage(markdown: string, url: string, entryUrl: string, sequence: number): DocPage {
  const summary = extractSummary(markdown);
  const title = extractTitle(markdown) ?? new URL(url).pathname.split("/").filter(Boolean).pop() ?? "Page";
  const component = inferComponent(url, entryUrl);
  return { title, url, summary, content: markdown, sourceType: "http", component, sequence };
}

function extractTitle(markdown: string): string | undefined {
  return markdown.match(/^#\s+(.+)$/m)?.[1]?.trim();
}

function inferComponent(url: string, entryUrl: string): string | undefined {
  const basePath = new URL(entryUrl).pathname;
  const path = new URL(url).pathname;
  if (path === basePath) return undefined;
  const relative = path.slice(basePath.length).replace(/^\//, "");
  const segment = relative.split("/")[0];
  return segment || undefined;
}
```

- [ ] **Step 4: Run tests**

```bash
cd packages/engine && bunx vitest run src/lib/http-adapter.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/http-adapter.ts packages/engine/src/lib/http-adapter.spec.ts
git commit -m "feat(engine): rewrite HttpAdapter to discover+fetch all same-prefix pages"
```

---

### Task 7: Fix GithubAdapter Truncation + Update description→summary

**Files:**
- Modify: `packages/engine/src/lib/github-adapter.ts`
- Modify: `packages/engine/src/lib/github-adapter.spec.ts`

- [ ] **Step 1: Add the failing tests**

In `packages/engine/src/lib/github-adapter.spec.ts`, add to the existing test suite:

```typescript
it("logs a warning when tree.truncated is true", async () => {
  const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
    if (url.includes("api.github.com")) {
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({
          tree: [{ path: "docs/intro.md", type: "blob" }],
          truncated: true,
        }),
      });
    }
    return Promise.resolve({ ok: true, text: () => Promise.resolve("# Intro\n\nContent.") });
  }));

  const adapter = new GithubAdapter();
  await adapter.fetch({ name: "dbd", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });

  expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining("truncated"));
  consoleSpy.mockRestore();
});

it("DocPage has summary field (renamed from description)", async () => {
  vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
    if (url.includes("api.github.com")) {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ tree: [{ path: "docs/guide.md", type: "blob" }], truncated: false }) });
    }
    return Promise.resolve({ ok: true, text: () => Promise.resolve("# Guide\n\nThis is a guide.") });
  }));

  const adapter = new GithubAdapter();
  const pages = await adapter.fetch({ name: "repo", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });

  expect(pages[0].summary).toBeTruthy();
  expect(pages[0].content).toContain("This is a guide");
  expect((pages[0] as any).description).toBeUndefined(); // old field gone
});
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/engine && bunx vitest run src/lib/github-adapter.spec.ts 2>&1 | tail -20
```

Expected: 2 tests FAIL.

- [ ] **Step 3: Update github-adapter.ts**

In `packages/engine/src/lib/github-adapter.ts`:
1. Replace `description: extractFirstParagraph(content)` → `summary: extractSummary(content)` (import `extractSummary` from `./doc-utils.js`)
2. Add truncation warning after fetching the tree response
3. Remove the now-redundant `extractFirstParagraph` function

```typescript
// packages/engine/src/lib/github-adapter.ts
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { extractSummary } from "./doc-utils.js";

interface GitHubTreeItem { path: string; type: "blob" | "tree"; url: string; }
interface GitHubTreeResponse { tree: GitHubTreeItem[]; truncated: boolean; }

export interface ParsedGithubUrl { owner: string; repo: string; branch: string; basePath: string; }

export function parseGithubUrl(url: string): ParsedGithubUrl | null {
  const match = url.match(/^https:\/\/github\.com\/([^/]+)\/([^/]+)\/tree\/([^/]+)\/?(.*)$/);
  if (!match) return null;
  return { owner: match[1], repo: match[2], branch: match[3], basePath: match[4] ?? "" };
}

export class GithubAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`GithubAdapter: entry "${entry.name}" requires base_url`);

    const parsed = parseGithubUrl(entry.base_url);
    if (!parsed) throw new Error(`GithubAdapter: invalid GitHub tree URL: ${entry.base_url}`);

    const { owner, repo, branch, basePath } = parsed;
    const treeUrl = `https://api.github.com/repos/${owner}/${repo}/git/trees/${branch}?recursive=1`;
    const treeRes = await fetch(treeUrl, { headers: { Accept: "application/vnd.github.v3+json" } });
    if (!treeRes.ok) throw new Error(`GithubAdapter: GitHub API error ${treeRes.status} for ${treeUrl}`);

    const tree: GitHubTreeResponse = await treeRes.json();

    if (tree.truncated) {
      console.warn(
        `[GithubAdapter] Warning: tree truncated for ${owner}/${repo} — some files may be missing.\n` +
        `Fetched ${tree.tree.length} of potentially more files.`
      );
    }

    const prefix = basePath ? basePath + "/" : "";
    const mdFiles = tree.tree.filter(item => item.type === "blob" && item.path.startsWith(prefix) && item.path.endsWith(".md"));

    const pages: DocPage[] = [];
    for (const file of mdFiles) {
      const rawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${branch}/${file.path}`;
      const res = await fetch(rawUrl);
      if (!res.ok) continue;
      const content = await res.text();
      pages.push({
        title: extractH1(content) ?? stemName(file.path),
        url: `https://github.com/${owner}/${repo}/blob/${branch}/${file.path}`,
        summary: extractSummary(content),
        content,
        sourceType: "github",
        component: inferComponent(file.path, basePath),
      });
    }
    return pages;
  }
}

function extractH1(content: string): string | undefined {
  return content.match(/^#\s+(.+)$/m)?.[1]?.trim();
}

function stemName(filePath: string): string {
  return (filePath.split("/").pop() ?? filePath).replace(/\.md$/, "");
}

function inferComponent(filePath: string, basePath: string): string | undefined {
  const parts = filePath.split("/");
  const baseDepth = basePath ? basePath.split("/").length : 0;
  if (parts.length <= baseDepth + 1) return undefined;
  return parts[baseDepth];
}
```

- [ ] **Step 4: Run tests**

```bash
cd packages/engine && bunx vitest run src/lib/github-adapter.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/lib/github-adapter.ts packages/engine/src/lib/github-adapter.spec.ts
git commit -m "fix(engine): GithubAdapter truncation warning + rename description→summary"
```

---

### Task 8: Update LocalAdapter + lib-indexer.spec.ts

**Files:**
- Modify: `packages/engine/src/lib/local-adapter.ts`
- Modify: `packages/engine/src/lib/local-adapter.spec.ts`
- Modify: `packages/engine/src/lib/lib-indexer.spec.ts`
- Modify: `packages/engine/src/lib/lib-indexer-shared.spec.ts`

The LocalAdapter needs to use `extractSummary` instead of `content.slice(0, 200)` for `description`. Tests that use `description` must be updated to `summary`.

- [ ] **Step 1: Update local-adapter.ts**

Replace `description: content.slice(0, 200)` with:

```typescript
import { extractSummary } from "./doc-utils.js";
// ...
const summary = extractSummary(content);
pages.push({ title, localPath: fullPath, summary, content, sourceType: "local", component });
```

- [ ] **Step 2: Update local-adapter.spec.ts**

Change any `page.description` assertions to `page.summary`. Expect `page.content` to be present (it already is via readFile). Add assertion that `summary` is populated.

- [ ] **Step 3: Update lib-indexer.spec.ts**

The test file uses `description: "A button"` etc. in DocPage constructors. Change all `description` keys to `summary`, and add `content: "..."` since it's now required:

```typescript
// Before
{ title: "Button", url: "https://rokkit.dev/button", description: "A button", sourceType: "llms.txt" }

// After
{ title: "Button", url: "https://rokkit.dev/button", summary: "A button", content: "A button component docs.", sourceType: "llms.txt" }
```

Also update embedding test: `lib-indexer.spec.ts` line 63 tests "content for http" — the embed call already uses `content.slice(0, 512)`. Line 71 tests fallback when content undefined — since content is now required, update to use a short string:
```typescript
// The "falls back to description" test no longer applies — content is required.
// Replace with a test that embeds content.slice(0,512) for non-llms.txt:
it("embeds summary for llms.txt, content.slice(0,512) for other types", async () => {
  // ...
});
```

- [ ] **Step 4: Update lib-indexer-shared.spec.ts**

Same — change `description` → `summary`, add `content` to DocPage constructors. Update embed assertion: for llms.txt, `embed` is now called with `summary` (the spec says embed input for sections is `content.slice(0, 512)` — but `indexShared` now works differently with two-level insert; the existing test structure may need to be simplified to just verify the current behavior).

Actually, since Task 9 rewrites `indexShared` significantly, leave `lib-indexer-shared.spec.ts` for Task 9 to rewrite completely.

- [ ] **Step 5: Run all engine tests to check state**

```bash
cd packages/engine && bunx vitest run 2>&1 | tail -30
```

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/local-adapter.ts packages/engine/src/lib/local-adapter.spec.ts packages/engine/src/lib/lib-indexer.spec.ts
git commit -m "fix(engine): update LocalAdapter and test fixtures for description→summary rename"
```

---

## Chunk 3: LibIndexer Two-Level Insert

### Task 9: Rewrite LibIndexer.indexShared — Two-Level Insert

**Files:**
- Modify: `packages/engine/src/lib/lib-indexer.ts`
- Modify (rewrite): `packages/engine/src/lib/lib-indexer-shared.spec.ts`

`indexShared` now:
1. Deletes existing `documents_in_library` rows for this library (cascade-deletes sections)
2. For each `DocPage`: inserts one `documents_in_library` row, then calls `splitSections(page.content)` and inserts `sections_in_document` rows with the document FK
3. Returns `{ documentsIndexed, sectionsIndexed }`

**The `index()` method (per-repo) is NOT changed** — it still writes to `lib_doc_sections` for per-repo indexing.

- [ ] **Step 1: Rewrite lib-indexer-shared.spec.ts**

```typescript
// packages/engine/src/lib/lib-indexer-shared.spec.ts
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

// Tracks all insert calls by table name
const makeMockDb = () => {
  const insertsByTable: Record<string, unknown[][]> = {};
  const _deleteEq = vi.fn().mockResolvedValue({ error: null });

  return {
    _insertsByTable: insertsByTable,
    _deleteEq,
    from: vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: _deleteEq }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        if (!insertsByTable[table]) insertsByTable[table] = [];
        insertsByTable[table].push(rows as unknown[]);
        return Promise.resolve({ error: null });
      }),
      select: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnValue({ data: null, error: null }),
      }),
    })),
  };
};

const makeEntry = (): LibEntry => ({ name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev" });

const makePages = (): DocPage[] => [
  {
    title: "Button",
    url: "https://rokkit.dev/button",
    summary: "A button component",
    content: "# Button\n\nFull button docs.\n\n## Props\n\nSize and variant.",
    sourceType: "llms.txt",
    component: "Components",
    sequence: 0,
  },
  {
    title: "Input",
    url: "https://rokkit.dev/input",
    summary: "An input component",
    content: "# Input\n\nFull input docs.\n\n## Props\n\nType and placeholder.",
    sourceType: "llms.txt",
    component: "Forms",
    sequence: 1,
  },
];

describe("LibIndexer.indexShared — two-level insert", () => {
  it("deletes documents_in_library (not sections) to cascade-delete sections", async () => {
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, null);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    expect(db.from).toHaveBeenCalledWith("documents_in_library");
    expect(db._deleteEq).toHaveBeenCalledWith("library_id", "lib-42");
  });

  it("inserts one row per DocPage into documents_in_library", async () => {
    const db = makeMockDb();
    const indexer = new LibIndexer(db as any, null);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    const docInserts = db._insertsByTable["documents_in_library"];
    expect(docInserts).toBeDefined();
    // May be batched as one insert([...]) or many insert(row)
    const allDocs = docInserts.flat();
    expect(allDocs).toHaveLength(2);
    const first = allDocs[0] as Record<string, unknown>;
    expect(first.library_id).toBe("lib-42");
    expect(first.title).toBe("Button");
    expect(first.summary).toBe("A button component");
    expect(first.url).toBe("https://rokkit.dev/button");
    expect(first.component).toBe("Components");
    expect(first.sequence).toBe(0);
    expect(first.source_type).toBe("llms.txt");
  });

  it("splits content into sections_in_document rows with document_id FK", async () => {
    const db = makeMockDb();
    // Mock that insert into documents_in_library returns an id
    let docInsertCall = 0;
    db.from = vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        if (table === "documents_in_library") {
          const arr = Array.isArray(rows) ? rows : [rows];
          return Promise.resolve({ data: arr.map((_, i) => ({ id: `doc-id-${docInsertCall++}-${i}` })), error: null });
        }
        if (!db._insertsByTable[table]) db._insertsByTable[table] = [];
        db._insertsByTable[table].push(rows as unknown[]);
        return Promise.resolve({ error: null });
      }),
      select: vi.fn().mockReturnThis(),
    })) as any;

    const indexer = new LibIndexer(db as any, null);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    const sectionInserts = db._insertsByTable["sections_in_document"] ?? [];
    const allSections = sectionInserts.flat() as Record<string, unknown>[];
    // Button doc: Overview + Props → 2 sections
    // Input doc: Overview + Props → 2 sections
    expect(allSections.length).toBeGreaterThanOrEqual(4);
    allSections.forEach(s => {
      expect(s.library_id).toBe("lib-42");
      expect(s.document_id).toBeDefined();
      expect(s.title).toBeTruthy();
      expect(s.content).toBeTruthy();
    });
  });

  it("returns documentsIndexed and sectionsIndexed counts", async () => {
    const db = makeMockDb();
    db.from = vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        const arr = Array.isArray(rows) ? rows : [rows];
        if (table === "documents_in_library") {
          return Promise.resolve({ data: arr.map((_, i) => ({ id: `doc-${i}` })), error: null });
        }
        return Promise.resolve({ error: null });
      }),
    })) as any;

    const indexer = new LibIndexer(db as any, null);
    const result = await indexer.indexShared("lib-42", makeEntry(), makePages());

    expect(result.documentsIndexed).toBe(2);
    expect(result.sectionsIndexed).toBeGreaterThanOrEqual(2);
  });

  it("embeds section content when backend is provided", async () => {
    const backend = makeMockBackend();
    const db = makeMockDb();
    db.from = vi.fn().mockImplementation((table: string) => ({
      delete: vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) }),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        const arr = Array.isArray(rows) ? rows : [rows];
        if (table === "documents_in_library") {
          return Promise.resolve({ data: arr.map((_, i) => ({ id: `doc-${i}` })), error: null });
        }
        return Promise.resolve({ error: null });
      }),
    })) as any;

    const indexer = new LibIndexer(db as any, backend);
    await indexer.indexShared("lib-42", makeEntry(), makePages());

    // embed called at least once for sections
    expect(backend.embed).toHaveBeenCalled();
    // embed input is content.slice(0, 512)
    const call = (backend.embed as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(typeof call).toBe("string");
    expect(call.length).toBeLessThanOrEqual(512);
  });
});
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd packages/engine && bunx vitest run src/lib/lib-indexer-shared.spec.ts 2>&1 | tail -30
```

Expected: Multiple FAIL — `indexShared` still uses old single-level insert.

- [ ] **Step 3: Rewrite LibIndexer.indexShared in lib-indexer.ts**

```typescript
async indexShared(
  libraryId: string,
  entry: LibEntry,
  pages: DocPage[],
): Promise<{ documentsIndexed: number; sectionsIndexed: number }> {
  // 1. Delete existing documents (cascade-deletes sections via FK)
  const { error: deleteError } = await this.db
    .from("documents_in_library")
    .delete()
    .eq("library_id", libraryId);
  if (deleteError) throw new Error(`LibIndexer.indexShared: delete failed: ${deleteError.message}`);

  let documentsIndexed = 0;
  let sectionsIndexed = 0;

  for (const page of pages) {
    // 2a. Insert document row
    const docRow = {
      library_id: libraryId,
      sequence: page.sequence ?? documentsIndexed,
      title: page.title,
      url: page.url ?? null,
      local_path: page.localPath ?? null,
      summary: page.summary,
      component: page.component ?? null,
      source_type: entry.source_type,
    };

    const { data: docData, error: docErr } = await this.db
      .from("documents_in_library")
      .insert(docRow)
      .select("id")
      .single();
    if (docErr) throw new Error(`LibIndexer.indexShared: doc insert failed: ${docErr.message}`);

    const documentId: string = (docData as { id: string }).id;
    documentsIndexed++;

    // 2b. Split content into sections
    const { splitSections } = await import("./doc-utils.js");
    const sections = splitSections(page.content);

    const sectionRows = await Promise.all(
      sections.map(async (section) => {
        const embedding = this.backend
          ? await this.backend.embed(section.content.slice(0, 512))
          : null;
        return {
          library_id: libraryId,
          document_id: documentId,
          sequence: section.sequence,
          title: section.title,
          content: section.content,
          embedding,
        };
      })
    );

    if (sectionRows.length > 0) {
      const { error: secErr } = await this.db.from("sections_in_document").insert(sectionRows);
      if (secErr) throw new Error(`LibIndexer.indexShared: section insert failed: ${secErr.message}`);
      sectionsIndexed += sectionRows.length;
    }
  }

  return { documentsIndexed, sectionsIndexed };
}
```

Also update the return type of `indexShared`:
```typescript
async indexShared(...): Promise<{ documentsIndexed: number; sectionsIndexed: number }>
```

- [ ] **Step 4: Run tests**

```bash
cd packages/engine && bunx vitest run src/lib/lib-indexer-shared.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Run full engine test suite**

```bash
cd packages/engine && bunx vitest run
```

Fix any remaining failures from old `description` references.

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/lib-indexer.ts packages/engine/src/lib/lib-indexer-shared.spec.ts
git commit -m "feat(engine): LibIndexer.indexShared two-level insert (documents + sections)"
```

---

## Chunk 4: Dashboard Server Updates

### Task 10: Update Dashboard lib-indexer.ts (Table Names + document_count)

**Files:**
- Modify: `apps/dashboard/src/lib/server/lib-indexer.ts`

The dashboard lib-indexer calls `indexShared` and then updates `libraries` (was `shared_libs`). It also needs to update `document_count` in addition to `section_count`. The embed phase reads from `sections_in_document` (was `shared_lib_sections`).

- [ ] **Step 1: Update runFetch**

In `runFetch`, change:
- `'shared_libs'` → `'libraries'` (two occurrences)
- After `indexShared`, update `document_count` alongside `section_count`:

```typescript
const { documentsIndexed, sectionsIndexed } = await new LibIndexer(db, null).indexShared(lib.id, entry, pages);

await db
  .from('libraries')
  .update({
    document_count: documentsIndexed,
    section_count: sectionsIndexed,
    indexed_at: new Date().toISOString(),
    index_status: 'ready',
    index_error: null,
  })
  .eq('id', lib.id);
```

- [ ] **Step 2: Update runEmbed**

Change the table references in `runEmbed`:
- `'shared_libs'` → `'libraries'` (all occurrences)
- `'shared_lib_sections'` → `'sections_in_document'`
- `eq('shared_lib_id', libId)` → `eq('library_id', libId)`
- Column `description` in the select → remove (sections no longer have description)
- Embed input: always use `section.content.slice(0, 512)` (description is gone)

Updated embed section selection:
```typescript
const { data: sections, error: fetchErr } = await db
  .from('sections_in_document')
  .select('id,content')
  .eq('library_id', libId)
  .is('embedding', null);
```

Update the batch map:
```typescript
batch.map(async (section: { id: string; content: string }) => {
  const embedding = await backend.embed(section.content.slice(0, 512));
  await db
    .from('sections_in_document')
    .update({ embedding })
    .eq('id', section.id);
})
```

Also embed document summaries:
```typescript
// After embedding sections, embed document summaries
const { data: docs } = await db
  .from('documents_in_library')
  .select('id,summary')
  .eq('library_id', libId)
  .is('embedding', null);

if (docs && docs.length > 0) {
  for (let i = 0; i < docs.length; i += BATCH) {
    const batch = docs.slice(i, i + BATCH);
    await Promise.all(
      batch.map(async (doc: { id: string; summary: string }) => {
        if (!doc.summary) return;
        const embedding = await backend.embed(doc.summary);
        await db.from('documents_in_library').update({ embedding }).eq('id', doc.id);
      })
    );
  }
}
```

- [ ] **Step 3: Verify the file compiles**

```bash
cd apps/dashboard && bunx tsc --noEmit 2>&1 | grep lib-indexer
```

Expected: No errors for lib-indexer.ts.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/lib/server/lib-indexer.ts
git commit -m "feat(dashboard): update lib-indexer for new table names + document_count"
```

---

### Task 11: Update Library Detail Page Server Actions

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/[id]/+page.server.ts`

Replace all old table/column names with the new ones.

- [ ] **Step 1: Update table references**

| Old | New |
|-----|-----|
| `'shared_libs'` | `'libraries'` |
| `'shared_lib_sections'` | `'sections_in_document'` |
| `'lib_queries'` | `'queries_on_library'` |
| `eq('shared_lib_id', ...)` | `eq('library_id', ...)` |

- [ ] **Step 2: Update the load function**

The load currently fetches `shared_lib_sections`. Instead, fetch `documents_in_library`:

```typescript
const { data: documents } = await db
  .from('documents_in_library')
  .select('id,title,url,local_path,summary,component,source_type,sequence,last_fetched')
  .eq('library_id', params.id)
  .order('sequence')
  .limit(200);
```

Update the return type: replace `sections` with `documents` in the return object.

Also add `document_count` to the `lib` select:
```
'id,name,source_type,base_url,local_path,section_count,document_count,indexed_at,index_status,index_error,created_at,icon_url,category,embed_status'
```

- [ ] **Step 3: Update simulate action**

The simulate action now queries `sections_in_document` joined with `documents_in_library`:

```typescript
simulate: async ({ params, request }) => {
  const db = getDb();
  const formData = await request.formData();
  const query = String(formData.get('query') ?? '').trim();
  if (!query) return fail(400, { error: 'Query is required' });

  const safeQuery = query.replace(/[%,()]/g, '');
  const { data: hits } = await db
    .from('sections_in_document')
    .select('id,title,content,document_id,documents_in_library!inner(title,url,component,summary)')
    .eq('library_id', params.id)
    .or(`title.ilike.%${safeQuery}%,content.ilike.%${safeQuery}%`)
    .limit(10);

  const results = (hits ?? []).map((h: any) => ({
    id: h.id,
    title: h.title,
    content: h.content,
    document: h.documents_in_library,
  }));

  await db.from('queries_on_library').insert({
    library_id: params.id,
    query_text: query,
    source: 'simulate',
    sections_hit: results.length,
  });

  return { query, results };
},
```

- [ ] **Step 4: Update reindex and embed actions**

In `reindex` and `embed` actions, replace `'shared_libs'` with `'libraries'`.

In `edit` action, update the `repo_libs` update to use `referenced_libraries`:
```typescript
await db
  .from('referenced_libraries')
  .update({ source_type, base_url: base_url ?? null, local_path: local_path ?? null })
  .eq('library_id', params.id);
```

- [ ] **Step 5: TypeScript check**

```bash
cd apps/dashboard && bunx tsc --noEmit 2>&1 | grep "page.server"
```

- [ ] **Step 6: Commit**

```bash
git add apps/dashboard/src/routes/libraries/[id]/+page.server.ts
git commit -m "feat(dashboard): update library detail server for renamed tables + documents"
```

---

### Task 12: Update Libraries List Page + Repos Library Page

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/+page.server.ts`
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`

- [ ] **Step 1: Update libraries/+page.server.ts**

Replace all occurrences:
- `'shared_libs'` → `'libraries'`
- `eq('shared_lib_id', ...)` → `eq('library_id', ...)`
- `shared_lib_id` field references → `library_id`

- [ ] **Step 2: Update repos/[id]/libraries/+page.server.ts**

Replace all occurrences:
- `'shared_libs'` → `'libraries'`
- `'repo_libs'` → `'referenced_libraries'`
- `shared_lib_id` column in `referenced_libraries` → `library_id`
- Any upsert `onConflict: 'repo_id,name'` stays the same (columns unchanged)

- [ ] **Step 3: TypeScript check**

```bash
cd apps/dashboard && bunx tsc --noEmit 2>&1 | head -30
```

Fix any type errors found.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/libraries/+page.server.ts apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts
git commit -m "feat(dashboard): update libraries list and repo library pages for renamed tables"
```

---

## Chunk 5: getLibDocsTool Update + Dashboard UI

### Task 13: Update getLibDocsTool — New Result Shape + New Queries

**Files:**
- Modify: `packages/server/src/tools/get-lib-docs.ts`

The tool now queries `sections_in_document` joined with `documents_in_library`, uses `match_libraries_sections` RPC (renamed), and returns a richer result shape with document context.

- [ ] **Step 1: Update the return type**

```typescript
export interface LibSection {
  title: string;        // section title (H2 heading)
  content: string;      // section markdown
  document: {
    title: string;
    url: string | null;
    component: string | null;
    summary: string;
  };
  similarity?: number;
}

export async function getLibDocsTool(
  db: SupabaseClient,
  backend: ModelBackend,
  repoId: string,
  lib: string,
  opts?: { component?: string; query?: string; limit?: number },
): Promise<{ lib: string; sections: LibSection[] }>
```

- [ ] **Step 2: Update the shared lib query path**

Replace `match_shared_lib_sections` RPC call with `match_libraries_sections`:

```typescript
const { data, error } = await db.rpc("match_libraries_sections", {
  p_library_id: sharedLibId,
  p_component: opts.component ?? null,
  query_embedding: embedding,
  match_count: limit,
});
```

The RPC now returns `{ section_id, section_title, content, similarity, doc_id, doc_title, url, local_path, component, summary }`.

- [ ] **Step 3: Update the keyword fallback**

```typescript
if (rows.length === 0) {
  const safeQ = opts.query.replace(/[%,()]/g, '');
  const { data: kData, error: kErr } = await (db as any)
    .schema('sensei')
    .from('sections_in_document')
    .select('id,title,content,document_id,documents_in_library!inner(title,url,local_path,component,summary)')
    .eq('library_id', sharedLibId)
    .or(`title.ilike.%${safeQ}%,content.ilike.%${safeQ}%`)
    .order('title')
    .limit(limit);
  if (kErr) throw new Error(kErr.message);
  rows = (kData ?? []) as Record<string, unknown>[];
}
```

- [ ] **Step 4: Update the row mapper**

```typescript
const sections: LibSection[] = rows.map(r => {
  // Handle both RPC shape and JOIN shape
  const doc = (r.documents_in_library as any) ?? {
    title: r.doc_title as string,
    url: r.url as string | null,
    component: r.component as string | null,
    summary: r.summary as string,
  };
  return {
    title: (r.section_title ?? r.title) as string,
    content: r.content as string,
    document: {
      title: doc.title,
      url: doc.url ?? doc.local_path ?? null,
      component: doc.component ?? null,
      summary: doc.summary ?? "",
    },
    similarity: r.similarity as number | undefined,
  };
});
```

- [ ] **Step 5: Update the query log table name**

Replace `'lib_queries'` → `'queries_on_library'` and column `shared_lib_id` → `library_id`.

- [ ] **Step 6: Update the browse path (no query)**

```typescript
const { data, error } = await (db as any)
  .schema('sensei')
  .from('sections_in_document')
  .select('id,title,content,documents_in_library!inner(title,url,local_path,component,summary)')
  .eq('library_id', sharedLibId)
  .order('title');
```

- [ ] **Step 7: TypeScript check**

```bash
cd packages/server && bunx tsc --noEmit 2>&1 | grep get-lib-docs
```

- [ ] **Step 8: Commit**

```bash
git add packages/server/src/tools/get-lib-docs.ts
git commit -m "feat(server): getLibDocsTool new result shape with document context + renamed tables"
```

---

### Task 14: Dashboard UI — Documents Tab with Inline Markdown Rendering

**Files:**
- Modify: `apps/dashboard/src/routes/libraries/[id]/+page.svelte`

Install `marked` for markdown rendering if not already present:

```bash
cd apps/dashboard && bun add marked
```

- [ ] **Step 1: Check if marked is installed**

```bash
cd apps/dashboard && cat package.json | grep marked
```

If not present: `bun add marked`

- [ ] **Step 2: Update the +page.svelte to use documents data**

The load function now returns `documents` instead of `sections`. Update the Svelte page:

**a) Update script section** — change `data.sections` references to `data.documents`, add state for expanded document:

```typescript
let expandedDocId = $state<string | null>(null);

function toggleDoc(id: string) {
  expandedDocId = expandedDocId === id ? null : id;
}

// Render markdown client-side
import { marked } from 'marked';
function renderMarkdown(content: string): string {
  // marked.parse returns string | Promise<string> — use sync option
  return marked.parse(content, { async: false }) as string;
}
```

**b) Replace the sections tab content with a Documents tab**

The tab bar should now show: **Documents** | **Sections** | **Queries**

For the Documents tab (default active):
```svelte
{#if activeTab === 'documents'}
  <div class="space-y-2">
    {#each data.documents as doc (doc.id)}
      <div class="border border-gray-200 rounded-lg overflow-hidden">
        <!-- Document row -->
        <button
          class="w-full flex items-center gap-3 p-4 text-left hover:bg-gray-50 transition-colors"
          onclick={() => toggleDoc(doc.id)}
        >
          {#if doc.component}
            <span class="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded font-mono shrink-0">
              {doc.component}
            </span>
          {/if}
          <div class="min-w-0 flex-1">
            <div class="font-medium text-gray-900 truncate">{doc.title}</div>
            {#if doc.summary}
              <div class="text-sm text-gray-500 truncate mt-0.5">{doc.summary}</div>
            {/if}
          </div>
          {#if doc.url && data.lib.source_type !== 'local'}
            <a
              href={doc.url}
              target="_blank"
              rel="noopener noreferrer"
              class="text-blue-600 hover:underline text-sm shrink-0"
              onclick={(e) => e.stopPropagation()}
            >Open ↗</a>
          {/if}
          <svg class="w-4 h-4 text-gray-400 shrink-0 transition-transform {expandedDocId === doc.id ? 'rotate-180' : ''}" ...chevron.../>
        </button>

        <!-- Inline markdown preview -->
        {#if expandedDocId === doc.id}
          <div class="border-t border-gray-200 p-4 bg-gray-50">
            <div class="prose prose-sm max-w-none">
              <!-- Note: content is not stored on documents — fetch sections and concatenate -->
              <!-- For now render summary; full content comes from sections -->
              <p class="text-gray-600 italic">
                {doc.summary || 'No summary available.'}
              </p>
            </div>
          </div>
        {/if}
      </div>
    {/each}
    {#if data.documents.length === 0}
      <p class="text-gray-500 text-sm py-8 text-center">No documents indexed yet. Click Re-index to fetch.</p>
    {/if}
  </div>
{/if}
```

**Note on content rendering:** The `documents_in_library` table stores `summary` but not the full concatenated content (sections are stored separately in `sections_in_document`). For the initial implementation, render the summary only in the inline preview. Full content rendering (fetching all sections and concatenating) is a follow-up improvement tracked in TODO.

Update the stat card to show `Documents` count alongside `Sections`:

```svelte
<div class="stat-card">
  <span class="text-2xl font-bold">{data.lib.document_count ?? 0}</span>
  <span class="text-sm text-gray-500">Documents</span>
</div>
```

- [ ] **Step 3: Update the Sections tab**

The Sections tab now queries differently — sections don't have `url`, `component`, or `description` columns. Since the server currently doesn't load sections (only documents), the Sections tab can show a "See Documents tab" message for now, or be removed. For simplicity, keep the tab but show the document count:

```svelte
{#if activeTab === 'sections'}
  <p class="text-gray-500 text-sm py-4">
    Sections are now organized under Documents. {data.lib.section_count} sections indexed across {data.lib.document_count ?? 0} documents.
  </p>
{/if}
```

- [ ] **Step 4: Test the UI manually**

```bash
cd apps/dashboard && bun run dev
```

Navigate to a library detail page. Verify:
- Documents tab shows (default)
- Documents list renders with title, component badge, summary
- "Open ↗" links appear for non-local source types
- Clicking a row expands/collapses the preview
- Sections tab shows count message
- Stat cards show Documents count

- [ ] **Step 5: Update Playwright tests**

In `apps/dashboard/tests/library-detail.test.ts`, update the stat cards test:

```typescript
test('stat cards display Documents, Sections, Repos, Queries, Last Indexed', async ({ page }) => {
  const found = await goToFirstLib(page);
  if (!found) { test.skip(true, 'No libraries in DB'); return; }

  const main = page.locator('main');
  await expect(main.getByText('Documents', { exact: true })).toBeVisible();
  await expect(main.getByText('Sections', { exact: true })).toBeVisible();
  await expect(main.getByText('Repos', { exact: true }).first()).toBeVisible();
  await expect(main.getByText('Queries', { exact: true })).toBeVisible();
  await expect(main.getByText('Last Indexed', { exact: true })).toBeVisible();
});
```

- [ ] **Step 6: Run Playwright tests**

```bash
cd apps/dashboard && bunx playwright test tests/library-detail.test.ts
```

Expected: All tests pass (some may skip if no DB data).

- [ ] **Step 7: Commit**

```bash
git add apps/dashboard/src/routes/libraries/[id]/+page.svelte apps/dashboard/tests/library-detail.test.ts apps/dashboard/package.json bun.lockb
git commit -m "feat(dashboard): documents tab with document list + inline summary preview"
```

---

## Final Verification

- [ ] **Run full engine test suite**

```bash
cd packages/engine && bunx vitest run
```

Expected: All tests pass (0 failures).

- [ ] **Run full dashboard build**

```bash
cd apps/dashboard && bun run build
```

Expected: No TypeScript errors or build failures.

- [ ] **Run all Playwright tests**

```bash
cd apps/dashboard && bunx playwright test
```

Expected: All tests pass or skip (no failures).

- [ ] **Final commit and push**

```bash
git push
```
