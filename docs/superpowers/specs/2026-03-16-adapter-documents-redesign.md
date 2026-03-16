# Adapter & Document Model Redesign — Design Spec

**Date:** 2026-03-16
**Status:** Approved

---

## Goal

Fix library indexing so all three source types (llms.txt, http, github) discover and fetch full content for every document in the library. Introduce a two-level hierarchy (Library → Documents → Sections) for richer retrieval, better token efficiency in agents, and inline document browsing in the dashboard.

---

## Problems Being Fixed

1. **`LlmsTxtAdapter`** only parses the index file — never fetches the linked pages. Sections have one-liner descriptions, no real content.
2. **`HttpAdapter`** fetches a single page — no link following. All sections share the same URL.
3. **`inferSourceType`** misses `.txt` extension (e.g. `index.txt`) — classifies Rokkit's index as `http`.
4. **`GithubAdapter`** ignores `truncated: true` in the Trees API response — silently drops files in large repos.
5. **Flat section model** loses document structure — no way to know which sections belong to the same file.

---

## Table Renames

| Old name | New name |
|---|---|
| `shared_libs` | `libraries` |
| `shared_lib_sections` | `sections_in_document` |
| `repo_libs` | `referenced_libraries` |
| `lib_queries` | `queries_on_library` |
| *(new)* | `documents_in_library` |

---

## Data Model

### Hierarchy

```
libraries
  └── documents_in_library  (one per file / page discovered)
        └── sections_in_document  (one per H2 block within the document)
```

### `libraries` (renamed from `shared_libs`)

No schema changes beyond the rename. Existing columns retained: `id`, `name`, `source_type`, `base_url`, `local_path`, `section_count`, `indexed_at`, `index_status`, `index_error`, `embed_status`, `icon_url`, `category`, `created_at`.

Add: `document_count int not null default 0` — updated on each index.

### `documents_in_library` (new)

`sequence` fields are independent per level: `documents_in_library.sequence` is order within the library; `sections_in_document.sequence` is order within the document (resets to 0 for each document).

```sql
create table sensei.documents_in_library (
  id           uuid primary key default gen_random_uuid(),
  library_id   uuid not null references sensei.libraries(id) on delete cascade,
  sequence     int  not null default 0,       -- order within the library
  title        text not null,                 -- H1 heading or filename stem
  url          text,                          -- remote URL (null for local)
  local_path   text,                          -- local path (null for remote)
  summary      text not null default '',      -- first non-heading paragraph ≤200 chars
  component    text,                          -- grouping (from index, directory, or URL segment)
  source_type  text not null,
  last_fetched timestamptz not null default now(),
  embedding    vector(384)                    -- embedded from summary, for doc-level search
);
```

### `sections_in_document` (restructured from `shared_lib_sections`)

```sql
-- Columns added:
document_id  uuid not null references sensei.documents_in_library(id) on delete cascade
sequence     int  not null default 0   -- order within the document

-- Columns removed (moved to documents_in_library):
url, local_path, source_type, component, description

-- Columns retained:
id, library_id (denormalized for query perf), title, content, embedding, last_fetched
```

`library_id` stays on sections (denormalized) so `getLibDocsTool` can filter by library without an extra join through documents.

### `referenced_libraries` (renamed from `repo_libs`)

Rename only. All columns unchanged: `repo_id`, `shared_lib_id` → rename FK column to `library_id`, `name`, `source_type`, `base_url`, `local_path`.

### `queries_on_library` (renamed from `lib_queries`)

Rename only. FK column `shared_lib_id` → `library_id`.

---

## `DocPage` Interface Change

`description` is **renamed** to `summary` — it is not a coexisting field. All code that reads `page.description` must be updated to `page.summary`.

```typescript
export interface DocPage {
  title: string;
  url?: string;           // remote sources
  localPath?: string;     // local sources
  summary: string;        // ≤200 chars — short description for display (renamed from description)
  content: string;        // REQUIRED — full markdown (was optional, "null for llms.txt")
  sourceType: 'llms.txt' | 'http' | 'local' | 'github';
  component?: string;
  sequence?: number;      // hint from adapter (order in index)
}
```

`content` is now **required**. Every adapter must fetch and return full markdown content.

**`summary` derivation per source type:**
- `llms.txt`: use the description from the index entry verbatim — it is curated by the library author and takes precedence over auto-extraction
- `http`: `extractSummary(markdown)` — first non-heading paragraph of the fetched page
- `github`: `extractSummary(markdown)` — first non-heading paragraph of the `.md` file
- `local`: `extractSummary(markdown)` — first non-heading paragraph of the file

---

## `inferSourceType` Changes

```typescript
// Add before the http fallback — catches index.txt, components.txt, etc.:
if (url.pathname.endsWith('.txt')) return { source_type: 'llms.txt', base_url: input };

// .md URLs → 'http' (already handled correctly by HttpAdapter)
```

---

## Adapter Architecture: Discover + Fetch

All adapters follow the same two-phase pattern internally:

**Phase 1 — Discover:** given the entry URL/path, build a flat ordered list of document locations.
**Phase 2 — Fetch:** for each location, fetch content → markdown → `DocPage`.

The `SourceAdapter` interface stays as `fetch(entry: LibEntry): Promise<DocPage[]>`. The two phases are internal to each adapter.

### Shared utilities (new)

**`resolveUrl(base: string, relative: string): string`**
Standard absolute URL resolution: `new URL(relative, base).href`. Used everywhere relative links appear.

**`fetchAsMarkdown(url: string): Promise<string>`**
Shared fetch + convert logic:
- Fetch the URL
- If `.md` extension, `text/plain`, or `text/markdown` content-type → use body directly
- Otherwise → Readability + Turndown (HTML → Markdown)
- Returns markdown string
- **Error handling:** throws on network or HTTP error. Callers handle gracefully: log a warning and skip the document (or fall back to description for `LlmsTxtAdapter`).

**`extractSummary(markdown: string): string`**
First non-heading paragraph, trimmed to ≤200 chars.

**`splitSections(markdown: string): Array<{title: string; content: string; sequence: number}>`**
- Content before first `##` → `{title: 'Overview', sequence: 0}` (omit if empty after whitespace trim)
- Each `## Heading` block → one section (includes H3+ sub-content within it); omit if content is empty after whitespace trim
- `sequence` resets to 0 for each document independently — it is the section's order within its document, not a global counter
- Returns in document order

### `LlmsTxtAdapter` (rewritten)

**Discover:** fetch index file → parse `- [title](url): description` entries → resolve each URL to absolute using `resolveUrl(indexUrl, entryUrl)`.

**Fetch:** for each entry, call `fetchAsMarkdown(url)`. On failure → use description as content (graceful degradation, log warning).

**`DocPage` per entry:**
- `title` = link title from index
- `url` = resolved absolute URL
- `summary` = description from index (authoritative — written by the library author)
- `content` = fetched markdown from the linked URL
- `component` = H2 section heading in the index file that groups this entry
- `sequence` = position in the index file

### `HttpAdapter` (rewritten)

**Discover:** fetch entry URL → parse HTML for `<a href>` links → filter to same path prefix (e.g. entry is `/docs` → only keep `/docs/*`) → resolve to absolute → deduplicate → max 100 URLs → include entry URL at sequence 0.

**Fetch:** for each URL, call `fetchAsMarkdown(url)`. Extract `summary` from `extractSummary(markdown)`. Title = first H1 or URL path stem.

**Component:** first path segment after the base path (e.g. `/docs/hooks/use-auth` with base `/docs` → `hooks`). Entry URL itself has no component.

### `GithubAdapter` (fix only)

**Fix:** after fetching the Trees API response, check `tree.truncated`. If `true`, log:
```
[GithubAdapter] Warning: tree truncated for {owner}/{repo} — some files may be missing.
Fetched {n} of potentially more files.
```

No other changes to the adapter.

### `LocalAdapter` (unchanged interface, internal fix)

Currently reads files correctly. Ensure `content` is populated (it already is via `readFile`). `summary` extracted via `extractSummary`.

---

## Section Splitting in `LibIndexer`

`LibIndexer.indexShared` now:
1. Deletes existing `documents_in_library` for this library (cascade-deletes sections)
2. For each `DocPage`:
   a. Insert one row into `documents_in_library`
   b. Call `splitSections(page.content)` → sections array
   c. Insert rows into `sections_in_document` with `document_id` FK
3. Updates `libraries.document_count` and `libraries.section_count`
4. Returns `{ documentsIndexed, sectionsIndexed }`

Embed phase (`startLibEmbed`):
- Embed each section's `content` → `sections_in_document.embedding`
- Embed each document's `summary` → `documents_in_library.embedding`

Embed input for sections: `content.slice(0, 512)` (consistent with token limit).

---

## `match_libraries_sections` RPC (updated)

Renamed from `match_shared_lib_sections`. Returns section match + document context via JOIN:

```sql
create or replace function sensei.match_libraries_sections(
  p_library_id    uuid,
  p_component     text,
  query_embedding vector(384),
  match_count     int default 10
)
returns table (
  -- section
  section_id    uuid,
  section_title text,
  content       text,
  similarity    float,
  -- document context
  doc_id        uuid,
  doc_title     text,
  url           text,
  local_path    text,
  component     text,
  summary       text
)
```

Keyword fallback in `getLibDocsTool`: JOIN `sections_in_document` with `documents_in_library` on `document_id`, filter by `library_id`, ILIKE on `section title || content`.

---

## Dashboard UI — Library Detail Page

### Documents tab (replaces flat sections list)

- Tabbed view: **Documents** (default) | **Sections** | **Queries**
- Documents list: title, component badge, summary, section count, "Open ↗" link (remote only)
- Click document row → inline markdown preview expands below the row (or a side panel)
  - Renders stored `content` (sections concatenated in order) using a markdown renderer (`marked`)
  - No re-fetch — content from DB
  - Local files: render only (no open link, `file://` blocked by browsers)
- On mobile/narrow: full-page route `/libraries/[id]/documents/[docId]` (same markdown renderer) — **future/optional**, not part of initial implementation

### Source-type open behaviour

| Source type | "Open ↗" target |
|---|---|
| `github` | `documents_in_library.url` (GitHub blob page) in new tab |
| `llms.txt` | `documents_in_library.url` (linked page) in new tab |
| `http` | `documents_in_library.url` in new tab |
| `local` | Not shown (content rendered only) |

---

## `getLibDocsTool` Result Shape

Returns per section hit:

```typescript
{
  sections: Array<{
    title: string;        // section title (H2 heading)
    content: string;      // section markdown — short, token-efficient
    document: {
      title: string;      // document title
      url: string | null; // where to read more
      component: string | null;
      summary: string;    // document summary
    };
    similarity?: number;
  }>;
}
```

---

## What Does NOT Change

- `TransformersBackend` — unchanged
- `inferSourceType` logic for `github` and `local` — unchanged
- Phase 1 / Phase 2 split (fetch vs embed) — unchanged
- `embed_status`, `index_status` on `libraries` — unchanged
- Skill generation — uses Claude API, unaffected
- Per-repo `lib_doc_sections` — out of scope

---

## TODO (future)

- `GITHUB_TOKEN` env var support in `GithubAdapter` (60 req/hr unauthenticated limit)
- HTTP adapter: configurable max-pages limit (currently hardcoded 100)
- Document-level vector search as a separate search mode
- Pagination on documents tab for large libraries
