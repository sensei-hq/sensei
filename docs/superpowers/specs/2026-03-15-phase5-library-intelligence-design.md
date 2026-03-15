---
id: phase5-library-intelligence
type: spec
status: approved
date: 2026-03-15
---

# Phase 5: Library Intelligence — Design Spec

## Goal

`sensei update-registry` indexes external libraries declared in `.sensei/config.yaml` into Supabase. An MCP tool `get_lib_docs` lets agents retrieve relevant doc sections by library name, optional component, and optional semantic query. A `LibSkillGenerator` produces a per-library skill file teaching the agent how to use the library in the context of this specific project. Library status is visible in the dashboard.

---

## Problem

Agents working in repos that use external libraries (e.g. Rokkit, kavach) have no structured access to their documentation. Generic search returns noise; reading raw doc sites is slow and token-heavy. Phase 5 indexes lib docs once, stores them in a searchable form, and exposes them through a fast MCP tool — so `get_lib_docs("rokkit", "Button")` returns the exact doc chunks an agent needs.

---

## Architecture

```
sensei update-registry
  → read custom_libs from .sensei/config.yaml
  → for each lib: SourceAdapter.fetch(entry) → DocPage[]
  → new LibIndexer(db, ollamaBackend).index(repoId, entry, pages) → lib_doc_sections in Supabase
  → if ANTHROPIC_API_KEY: LibSkillGenerator.generate(entry, pages) → ~/.claude/skills/

get_lib_docs MCP tool
  → embed query (OllamaBackend) → cosine search on lib_doc_sections.embedding
  → filtered by lib_name (and component if provided)
  → returns description+URL (llms.txt entries) or content chunks (http/local entries)

Dashboard /repos/[id]/libraries
  → section counts, freshness, skill file status per lib
  → "Update Registry" button → spawns sensei update-registry (60s timeout)
  → click lib name → /repos/[id]/libraries/[name] → doc sections list
```

**Key design decision — storage by source type:**
- `llms.txt` sources: store index only — `title`, `url`, `description` (from llms.txt entry), `embedding` on description. Full content lives at the source URL. Lightweight, avoids DB bloat.
- `http` / `local` sources: transform HTML → markdown (readability/turndown), chunk by section, store `content` + `embedding` on content. Necessary since there is no pre-built index to link back to.

Libraries are **repo-scoped** — `lib_doc_sections` rows include `repo_id`. The same library indexed in two repos is stored independently (different usage context, independent freshness).

---

## Types (`packages/shared/src/types.ts`)

```typescript
// ─── Library intelligence types ──────────────────────────────────────────────

export interface LibEntry {
  name: string;
  source_type: 'llms.txt' | 'http' | 'local';
  base_url?: string;       // llms.txt: direct URL to the llms.txt file; http: root URL to crawl
  local_path?: string;     // llms.txt: local path to the llms.txt file; local: directory path to scan
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

Config schema: `packages/shared/src/config.ts` gains `custom_libs` on `SenseiRepoConfig` and Zod validation in `loadSenseiConfig()`:

```typescript
// Updated SenseiRepoConfig (packages/shared/src/config.ts)
export interface SenseiRepoConfig {
  repo_id: string;
  supabase_url: string;
  custom_libs?: LibEntry[];  // added in Phase 5
}
```

`loadSenseiConfig()` parses `custom_libs` through a Zod schema (array of `LibEntry` objects) before returning. Other existing fields are left as plain cast — only `custom_libs` gains strict Zod validation.

---

## Database Migration

```sql
create table lib_doc_sections (
  id            uuid primary key default gen_random_uuid(),
  repo_id       uuid not null references repos(id) on delete cascade,
  lib_name      text not null,
  title         text not null,
  url           text,
  local_path    text,
  description   text not null,   -- embedding input: llms.txt short desc; first 512 chars of content for others
  content       text,             -- null for llms.txt entries; full extracted markdown for http/local
  source_type   text not null,   -- 'llms.txt' | 'http' | 'local'
  component     text,
  embedding     vector(768),     -- nomic-embed-text via OllamaBackend
  last_fetched  timestamptz not null default now(),
  created_at    timestamptz not null default now()
);

create index lib_doc_sections_repo_lib_idx on lib_doc_sections(repo_id, lib_name);
create index lib_doc_sections_embedding_idx on lib_doc_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);
```

---

## Engine Layer (`packages/engine/src/lib/`)

### New Files

```
packages/engine/src/lib/
  source-adapter.ts          ← SourceAdapter interface
  llms-txt-adapter.ts        ← LlmsTxtAdapter (URL or local path, same parser)
  llms-txt-adapter.spec.ts
  http-adapter.ts            ← HttpAdapter (crawl + HTML→markdown)
  http-adapter.spec.ts
  local-adapter.ts           ← LocalAdapter (scan dir for .md/.txt)
  local-adapter.spec.ts
  lib-indexer.ts             ← LibIndexer
  lib-indexer.spec.ts
  lib-skill-generator.ts     ← LibSkillGenerator
  lib-skill-generator.spec.ts
```

### `SourceAdapter` interface

```typescript
export interface SourceAdapter {
  fetch(entry: LibEntry): Promise<DocPage[]>;
}
```

### `LlmsTxtAdapter`

- If `entry.base_url` starts with `http`: fetch text via HTTP
- If `entry.local_path` set: read file from disk
- Parse llms.txt format — standard markdown structure:
  ```
  # Project Name
  > Optional description

  ## Optional Section Header

  - [Page Title](URL): short description
  - [Page Title](URL): short description
  ```
  Each `- [Title](URL): description` line → one `DocPage` with `title`, `url`, `description`; `component` = parent `##` section heading if present
- `content` is always `undefined` (index-only storage)
- Lines not matching the link pattern are skipped gracefully
- Throws if neither `base_url` nor `local_path` provided

### `HttpAdapter`

- Fetches the single root URL `entry.base_url` (no recursive crawl — single page only)
- Extracts readable content using `@mozilla/readability` + `jsdom`, converts to markdown via `turndown`
- Splits content at `##` headings → one `DocPage` per section
- `description` = first 200 chars of section content
- Dependencies: `@mozilla/readability`, `jsdom`, `turndown` — added to `packages/engine/package.json`

### `LocalAdapter`

- Walks `entry.local_path` recursively, collects `.md` and `.txt` files
- Each file → one `DocPage`: `title` from filename (without extension), `content` from file text, `description` from first 200 chars
- `url` is undefined; `localPath` is absolute path
- `component`: inferred from immediate parent directory name if nested (e.g. `components/Button/button.md` → `component: 'Button'`); undefined for files directly in the root path

### `LibIndexer`

```typescript
export class LibIndexer {
  constructor(private db: SupabaseClient, private backend: ModelBackend) {}

  async index(
    repoId: string,
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<{ sectionsIndexed: number }>
}
```

- Deletes existing rows for `(repo_id, lib_name)` before inserting — full refresh
- For each page: embed `description` (llms.txt) or first 512 chars of `content` (http/local) using `backend.embed()`; if `content` is unexpectedly undefined for a non-llms.txt page, fall back to `description`
- Upserts rows to `lib_doc_sections`
- Returns `{ sectionsIndexed: N }`

### `LibSkillGenerator`

```typescript
export class LibSkillGenerator {
  constructor(
    private model: ModelBackend,        // ClaudeBackend
    private profile: ProjectProfile,    // from extractProjectProfile
    private validator: SkillValidator,  // injected — same pattern as SkillGenerator
  ) {}

  async generate(
    entry: LibEntry,
    pages: DocPage[],
  ): Promise<string>                    // skill markdown
}
```

- Prompt includes: lib name, description, top 20 doc sections (title + description), how lib appears in repo's symbol index (inferred from `profile.keySymbols` and `profile.senseiConfig`)
- Uses injected `SkillValidator` to validate before returning — category string passed as `"lib-{entry.name}"` (e.g. `"lib-rokkit"`), retries up to 3 attempts
- Returns validated skill markdown; caller (CLI) is responsible for writing and recording the manifest

### `ClaudeAdapter` extension

Phase 5 adds a new method to the existing `ClaudeAdapter` class:

```typescript
async writeLibSkill(
  libName: string,
  markdown: string,
  repoSlug: string,
): Promise<LibSkillFile>
// Writes sensei-{repoSlug}-lib-{libName}.md to skillsDir
// Returns LibSkillFile { libName, path, generatedAt }
```

- `LibSkillFile` is imported from `@sensei/shared` (alongside the existing `AgentSkillFile` import)
- This avoids the `AgentSkillFile["category"]` union constraint on `writeSkills` which only accepts the four Phase 4 category literals
- `installedSkills()` is not updated — it reads `AgentSkillFile` entries only and does not enumerate lib skills. Lib skill status is tracked exclusively via `.sensei/lib-skills.json`
- `claude-adapter.spec.ts` gains a test for `writeLibSkill`: writes correct filename, returns `LibSkillFile` with correct `libName`, `path`, and `generatedAt`

---

## CLI (`packages/cli/src/commands/update-registry.ts`)

New `updateRegistry(repoPath: string): Promise<void>` function:

```
1. intro("sensei update-registry")
2. const config = await loadSenseiConfig(repoPath)  // returns null if not initialised
   if (!config) { cancel("Not initialised — run sensei init first"); return; }
   const client = makeSenseiClient(config)
   const repoId = config.repo_id                   // from SenseiRepoConfig
3. Read config.custom_libs — exits cleanly if empty ("No custom_libs configured")
4. const profile = await extractProjectProfile(client, repoId, repoPath)
   const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-")
   const ollamaBackend = new OllamaBackend()
5. For each lib:
   a. spinner: "Fetching {lib.name}..."
   b. Create SourceAdapter from source_type
   c. adapter.fetch(entry) → DocPage[]
   d. spinner: "Indexing {lib.name} ({N} pages)..."
   e. new LibIndexer(client, ollamaBackend).index(repoId, entry, pages)
   f. spinner stop: "{lib.name}: N sections indexed"
   g. if ANTHROPIC_API_KEY:
      - const claudeBackend = new ClaudeBackend(); await claudeBackend.init()
      - const validator = new SkillValidator(claudeBackend, profile)
      - const markdown = await new LibSkillGenerator(claudeBackend, profile, validator).generate(entry, pages)
      - const libSkillFile = await new ClaudeAdapter().writeLibSkill(lib.name, markdown, repoSlug)
        // writes sensei-{slug}-lib-{name}.md, returns LibSkillFile
      - append libSkillFile to manifest; write {repoPath}/.sensei/lib-skills.json
6. outro: summary
```

The manifest at `{repoPath}/.sensei/lib-skills.json` (`LibSkillsManifest`) is read at start of step 5 if it exists (to preserve entries for other libs), then rewritten after each lib's skill file is generated. Created fresh if missing.

`cli.ts` gains `update-registry` as a new subcommand. No flags required. Help text: `update-registry     Index custom_libs from .sensei/config.yaml into Supabase`.

---

## MCP Tool (`packages/server/src/tools/get-lib-docs.ts`)

```typescript
export async function getLibDocsTool(
  db: SupabaseClient,
  backend: ModelBackend,       // OllamaBackend for embedding
  repoId: string,
  lib: string,
  opts?: { component?: string; query?: string; limit?: number }
): Promise<{ lib: string; sections: DocPage[] }>
```

- If `query` provided: embed query → cosine similarity search on `lib_doc_sections.embedding` WHERE `repo_id = repoId AND lib_name = lib [AND component = opts.component]`, return top `limit` (default 10)
- If no `query`: return all sections for `(repo_id, lib_name[, component])` sorted by title
- Returns `DocPage[]` (same shape as the type used in the engine layer — no separate result type needed)

Registered in `mcp-server.ts`:

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
  async ({ lib, component, query, limit }) => { ... }
);
```

---

## Dashboard (`apps/dashboard/src/routes/repos/[id]/libraries/`)

### `+page.server.ts`

- Load all `lib_doc_sections` rows for `repo_id`, group by `lib_name`, count sections per lib
- Compute freshness: ≤7 days = Fresh, >7 days = Stale, 0 rows = Missing
- Check `.sensei/lib-skills.json` for skill file status per lib
- Form action `update`: spawns `sensei update-registry` via `Bun.spawn`, 60s timeout, stderr on failure, redirect on success

### `+page.svelte`

- Header: `Library Docs`
- Table: Library | Source | Sections | Last Fetched | Freshness | Skill
- Freshness badge: green "Fresh" / yellow "Stale" / red "Missing"
- Skill badge: green "Generated" / grey "None" (only present if ANTHROPIC_API_KEY available)
- "Update Registry" button (form POST to `?/update`)
- Empty state: `Add custom_libs to .sensei/config.yaml then run sensei update-registry`

### `[name]/+page.server.ts` + `[name]/+page.svelte`

- Load all sections for `(repo_id, lib_name)`
- Display: title, description, URL link (for llms.txt), content preview (for http/local)
- Link to skill file path if generated

**Link added** to `apps/dashboard/src/routes/repos/[id]/+page.svelte`:
```html
<p><a href="/repos/{data.repo.id}/libraries">Library Docs →</a></p>
```

---

## Error Handling

| Operation | On error |
|-----------|----------|
| `SourceAdapter.fetch()` | Throws — caught by CLI, shown as error for that lib, continues to next |
| `LibIndexer.index()` | Throws — caught by CLI, shown as error |
| `LibSkillGenerator.generate()` | Throws after 3 retries — caught by CLI, skill skipped, warning shown |
| `getLibDocsTool` | Returns `{ lib, sections: [] }` on error — never throws |
| Dashboard load | Returns empty state on missing sections |
| Regenerate action | Returns `fail(500, { error })` on non-zero exit or timeout |

---

## Test Scope

**`llms-txt-adapter.spec.ts`:**
- Fixture llms.txt URL → correct DocPage list with title/url/description
- Local path llms.txt → same output
- Malformed line → skipped gracefully

**`http-adapter.spec.ts`:**
- Mock HTTP server returning HTML → correct markdown DocPage per section
- Page with no `##` headings → single DocPage with full content

**`local-adapter.spec.ts`:**
- Temp dir with `.md` files → correct DocPage list
- Nested subdirectories → all files discovered

**`lib-indexer.spec.ts`:**
- Mock backend + mock Supabase: given N DocPages, inserts N rows with embeddings
- Deletes old rows before inserting (full refresh verified)

**`lib-skill-generator.spec.ts`:**
- Mock ClaudeBackend + injected SkillValidator: generate() called once, validator validates, returns skill markdown
- Retries on invalid validation (same pattern as Phase 4 SkillGenerator tests)

**`claude-adapter.spec.ts` (extended):**
- `writeLibSkill()` writes `sensei-{slug}-lib-{name}.md` in a temp dir
- Returns `LibSkillFile` with correct `libName`, `path`, and ISO `generatedAt`

**`get-lib-docs.spec.ts`:**
- Mock Supabase + OllamaBackend: query provided → embed called, cosine search returns matching rows
- No query → returns all sections for (repo_id, lib_name) sorted by title
- component filter applied when provided
- Empty result → returns `{ lib, sections: [] }` without throwing

---

## New Files

```
supabase/migrations/YYYYMMDDXXXXXX_phase5_lib_doc_sections.sql

packages/shared/src/types.ts                        ← add LibEntry, DocPage, LibSkillFile, LibSkillsManifest
packages/shared/src/config.ts                       ← add custom_libs to SenseiRepoConfig + Zod validation

packages/engine/src/lib/
  source-adapter.ts
  llms-txt-adapter.ts + spec
  http-adapter.ts + spec
  local-adapter.ts + spec
  lib-indexer.ts + spec
  lib-skill-generator.ts + spec

packages/engine/src/agent/claude-adapter.ts         ← add writeLibSkill() method
packages/engine/src/agent/claude-adapter.spec.ts    ← add writeLibSkill() test
packages/engine/src/index.ts                        ← add lib/* exports

packages/server/src/tools/get-lib-docs.ts + spec
packages/server/src/mcp-server.ts                   ← register get_lib_docs

packages/cli/src/commands/update-registry.ts
packages/cli/src/cli.ts                             ← add update-registry subcommand

apps/dashboard/src/routes/repos/[id]/libraries/
  +page.server.ts
  +page.svelte
apps/dashboard/src/routes/repos/[id]/libraries/[name]/
  +page.server.ts
  +page.svelte
apps/dashboard/src/routes/repos/[id]/+page.svelte   ← add "Library Docs →" link

.sensei/lib-skills.json                             ← generated at runtime (not checked in)
```

---

## Done When

- `sensei update-registry` on the sensei repo indexes Rokkit (llms.txt) and kavach (http) sections into Supabase
- `get_lib_docs({ lib: "rokkit", component: "Button" })` returns at least 2 relevant doc sections
- `get_lib_docs({ lib: "rokkit", query: "theming" })` returns semantically relevant results
- Lib skills written to `~/.claude/skills/sensei-sensei-lib-rokkit.md` and `sensei-sensei-lib-kavach.md` (if `ANTHROPIC_API_KEY` present) — double `sensei` is expected: first is the prefix, second is the repo slug for this repo
- Dashboard `/repos/[id]/libraries` shows both libs with section counts and freshness badges
- "Update Registry" button re-runs the pipeline and updates the view
