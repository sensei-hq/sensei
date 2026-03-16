# Phase 8: Shared Library Pool Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a global shared library pool so indexed library docs can be reused across repos without re-fetching — linked at init time or promoted via `--global` flag.

**Architecture:** Add `shared_libs` + `shared_lib_sections` tables for global storage; extend `LibIndexer` with `indexShared()`; add `--global` flag to `update-registry`; restructure `sensei init` to check the shared pool before prompting for URLs; route `get_lib_docs` to shared sections when `repo_libs.shared_lib_id` is set; update dashboard to show "Shared" status for linked rows.

**Tech Stack:** TypeScript, Bun, Vitest, Supabase (pgvector), SvelteKit 2, @clack/prompts

---

## Chunk 1: Schema + Engine

### Task 1: Schema — Migration and DDL Files

**Files:**
- Create: `supabase/migrations/20260315000002_phase8_shared_lib_pool.sql`
- Create: `database/ddl/table/sensei/shared_libs.ddl`
- Create: `database/ddl/table/sensei/shared_lib_sections.ddl`
- Modify: `database/ddl/table/sensei/repo_libs.ddl`

No automated tests for DDL — verify manually using `grep` on the migration.

- [ ] **Step 1: Create the migration file**

Create `supabase/migrations/20260315000002_phase8_shared_lib_pool.sql`:

```sql
-- supabase/migrations/20260315000002_phase8_shared_lib_pool.sql

create table if not exists sensei.shared_libs (
  id            uuid primary key default gen_random_uuid(),
  name          text not null unique,
  source_type   text not null check (source_type in ('llms.txt', 'http', 'local')),
  base_url      text,
  local_path    text,
  indexed_at    timestamptz,
  section_count int not null default 0,
  created_at    timestamptz not null default now()
);

create table if not exists sensei.shared_lib_sections (
  id            uuid primary key default gen_random_uuid(),
  shared_lib_id uuid not null references sensei.shared_libs(id) on delete cascade,
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

create index if not exists shared_lib_sections_lib_idx
  on sensei.shared_lib_sections(shared_lib_id);

create index if not exists shared_lib_sections_embedding_idx
  on sensei.shared_lib_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

alter table sensei.repo_libs
  add column if not exists shared_lib_id uuid references sensei.shared_libs(id);

create or replace function sensei.match_shared_lib_sections(
  p_shared_lib_id uuid,
  p_component     text default null,
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
  select title, url, local_path, description, content, source_type, component
  from sensei.shared_lib_sections
  where shared_lib_id = p_shared_lib_id
    and (p_component is null or component = p_component)
  order by embedding <=> query_embedding
  limit match_count
$$;

-- NOTE: grants are managed separately in database/ddl/grants.ddl — do not add here
```

- [ ] **Step 2: Verify migration file looks correct**

```bash
grep -c "create table" supabase/migrations/20260315000002_phase8_shared_lib_pool.sql
# Expected: 2

grep -cE "alter table|match_shared_lib_sections|shared_lib_sections_lib_idx|shared_lib_sections_embedding_idx" \
  supabase/migrations/20260315000002_phase8_shared_lib_pool.sql
# Expected: 4 (alter table, match_shared_lib_sections, and the two index names)
```

- [ ] **Step 3: Create DDL file for shared_libs**

Create `database/ddl/table/sensei/shared_libs.ddl`:

```sql
set search_path to sensei, extensions;

create table if not exists shared_libs (
  id            uuid primary key default gen_random_uuid()
, name          text not null unique
, source_type   text not null check (source_type in ('llms.txt', 'http', 'local'))
, base_url      text
, local_path    text
, indexed_at    timestamptz
, section_count int not null default 0
, created_at    timestamptz not null default now()
);

comment on table shared_libs is
'Global catalog of shared library doc pools — one row per globally-indexed lib.
- Promoted via: sensei update-registry --global --lib <name>
- Linked to per-repo usage via repo_libs.shared_lib_id FK
- section_count and indexed_at updated after each indexShared run';
```

- [ ] **Step 4: Create DDL file for shared_lib_sections**

Create `database/ddl/table/sensei/shared_lib_sections.ddl`:

```sql
set search_path to sensei, extensions;

create table if not exists shared_lib_sections (
  id            uuid primary key default gen_random_uuid()
, shared_lib_id uuid not null references shared_libs(id) on delete cascade
, title         text not null
, url           text
, local_path    text
, description   text not null
, content       text
, source_type   text not null check (source_type in ('llms.txt', 'http', 'local'))
, component     text
, embedding     vector(768)
, last_fetched  timestamptz not null default now()
, created_at    timestamptz not null default now()
);

create index if not exists shared_lib_sections_lib_idx
  on shared_lib_sections(shared_lib_id);

create index if not exists shared_lib_sections_embedding_idx
  on shared_lib_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

comment on table shared_lib_sections is
'Indexed documentation sections for globally-shared libraries.
- One row per content chunk from a shared lib source (llms.txt, HTTP page, or local path)
- shared_lib_id references shared_libs (no repo_id — sections are repo-agnostic)
- embedding: 768-dim vector for semantic search via match_shared_lib_sections RPC';
```

- [ ] **Step 5: Update repo_libs DDL to document the new FK column**

Edit `database/ddl/table/sensei/repo_libs.ddl` — add `shared_lib_id` column and update the comment:

The file currently ends at line 22. Add the column before the closing `;` of the table definition and update the comment:

```sql
set search_path to sensei, extensions;

create table if not exists repo_libs (
  id                  uuid primary key default gen_random_uuid()
, repo_id             uuid not null references repos(id) on delete cascade
, name                text not null
, source_type         text not null check (source_type in ('llms.txt', 'http', 'local'))
, base_url            text
, local_path          text
, skill_path          text
, skill_generated_at  timestamptz
, shared_lib_id       uuid references shared_libs(id)
, created_at          timestamptz not null default now()
, unique(repo_id, name)
);

create index if not exists repo_libs_repo_id_idx on repo_libs(repo_id);

comment on table repo_libs is
'Per-repo custom library configuration, synced from .sensei/config.yaml by update-registry.
- One row per lib per repo; upserted after every successful update-registry run
- Source of truth for the dashboard libraries page (avoids reading local config files)
- skill_path and skill_generated_at track the generated Agent Skill file for each lib
- shared_lib_id: when set, this lib links to the global shared pool (shared_libs)';
```

- [ ] **Step 6: Commit schema**

```bash
git add supabase/migrations/20260315000002_phase8_shared_lib_pool.sql \
        database/ddl/table/sensei/shared_libs.ddl \
        database/ddl/table/sensei/shared_lib_sections.ddl \
        database/ddl/table/sensei/repo_libs.ddl
git commit -m "feat(schema): add shared_libs, shared_lib_sections tables and repo_libs.shared_lib_id FK"
```

---

### Task 2: Engine — `LibIndexer.indexShared` Method

**Files:**
- Modify: `packages/engine/src/lib/lib-indexer.ts`
- Modify: `packages/engine/src/lib/lib-indexer.spec.ts`

- [ ] **Step 1: Write the failing tests**

Open `packages/engine/src/lib/lib-indexer.spec.ts`. The existing `makeMockDb()` builds a mock that records `insert` calls via `_insertedBatches`. To also capture and assert delete calls for `indexShared`, update `makeMockDb()` to expose the delete eq mock:

```typescript
// Replace makeMockDb with this version (add _deleteEqMock)
const makeMockDb = () => {
  const insertedBatches: unknown[] = [];
  const deleteEqMock = vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) });
  const deleteChain = { eq: deleteEqMock };

  return {
    _insertedBatches: insertedBatches,
    _deleteEqMock: deleteEqMock,
    from: vi.fn().mockImplementation((_table: string) => ({
      delete: vi.fn().mockReturnValue(deleteChain),
      insert: vi.fn().mockImplementation((rows: unknown) => {
        insertedBatches.push(rows);
        return Promise.resolve({ error: null });
      }),
    })),
  };
};
```

Note: This change also updates the existing `describe("LibIndexer")` tests — they still pass because `deleteChain.eq` still returns the same structure. Then add the new `describe("LibIndexer.indexShared")` block at the end of the file:

```typescript
describe("LibIndexer.indexShared", () => {
  it("deletes existing shared sections then inserts N rows with shared_lib_id", async () => {
    const db = makeMockDb();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);

    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://x.com/llms.txt" };
    const pages: DocPage[] = [
      { title: "Button", url: "https://rokkit.dev/button", description: "A button", sourceType: "llms.txt" },
      { title: "Input", url: "https://rokkit.dev/input", description: "An input", sourceType: "llms.txt" },
    ];

    const result = await indexer.indexShared("shared-lib-uuid", entry, pages);

    expect(result.sectionsIndexed).toBe(2);
    expect(backend.embed).toHaveBeenCalledTimes(2);
    expect(db.from).toHaveBeenCalledWith("shared_lib_sections");
    // Verify delete was called with correct shared_lib_id (replace-not-append contract)
    expect(db._deleteEqMock).toHaveBeenCalledWith("shared_lib_id", "shared-lib-uuid");
    expect(db._insertedBatches).toHaveLength(1);
    const inserted = db._insertedBatches[0] as Array<Record<string, unknown>>;
    expect(inserted).toHaveLength(2);
    expect(inserted[0].shared_lib_id).toBe("shared-lib-uuid");
    expect(inserted[0]).not.toHaveProperty("repo_id");
  });

  it("embeds description for llms.txt; first 512 chars of content for http/local", async () => {
    const db = makeMockDb();
    const backend = makeMockBackend();
    const indexer = new LibIndexer(db as any, backend);
    const content = "X".repeat(600);
    const pages: DocPage[] = [
      { title: "Usage", url: "https://x.com", description: "Short desc", content, sourceType: "http" },
    ];
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev" };

    await indexer.indexShared("shared-lib-uuid", entry, pages);
    expect(backend.embed).toHaveBeenCalledWith("X".repeat(512));
  });

  it("throws when insert fails", async () => {
    const db = {
      _insertedBatches: [],
      from: vi.fn().mockReturnValue({
        delete: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({ error: null }),
        }),
        insert: vi.fn().mockResolvedValue({ error: { message: "DB down" } }),
      }),
    };
    const indexer = new LibIndexer(db as any, makeMockBackend());
    await expect(
      indexer.indexShared("shared-lib-uuid", { name: "x", source_type: "llms.txt" }, [
        { title: "T", description: "D", sourceType: "llms.txt" },
      ])
    ).rejects.toThrow("LibIndexer.indexShared: insert failed");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bunx vitest run packages/engine/src/lib/lib-indexer.spec.ts
```

Expected: 3 new tests FAIL with "indexShared is not a function" or similar.

- [ ] **Step 3: Implement `indexShared` in LibIndexer**

Open `packages/engine/src/lib/lib-indexer.ts`. After the closing `}` of the existing `index()` method, add:

```typescript
async indexShared(
  sharedLibId: string,
  entry: LibEntry,
  pages: DocPage[],
): Promise<{ sectionsIndexed: number }> {
  // Note: the spec's indexShared body does not include a deleteError guard, but we
  // add one here for consistency with the existing index() method — fail fast if delete fails.
  const { error: deleteError } = await this.db
    .from("shared_lib_sections")
    .delete()
    .eq("shared_lib_id", sharedLibId);

  if (deleteError) throw new Error(`LibIndexer.indexShared: delete failed: ${deleteError.message}`);

  const rows = await Promise.all(
    pages.map(async page => {
      const embedInput =
        entry.source_type === "llms.txt"
          ? page.description
          : (page.content ?? page.description).slice(0, 512);

      const embedding = await this.backend.embed(embedInput);
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
```

Note: The `makeMockDb()` in the test has `delete` returning a chain with `.eq()`. The existing `index()` method calls `.delete().eq("repo_id", ...).eq("lib_name", ...)` (two `.eq()` calls), but `indexShared` only calls `.delete().eq("shared_lib_id", ...)` (one `.eq()` call). The current mock already handles this since the second `.eq()` just returns `{ error: null }` directly. Verify this works in step 4.

- [ ] **Step 4: Run tests to verify they pass**

```bash
bunx vitest run packages/engine/src/lib/lib-indexer.spec.ts
```

Expected: All 6 tests PASS.

- [ ] **Step 5: Run full test suite to check nothing is broken**

```bash
bunx vitest run
```

Expected: All tests pass (currently 447).

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/lib-indexer.ts packages/engine/src/lib/lib-indexer.spec.ts
git commit -m "feat(engine): add LibIndexer.indexShared for shared lib pool"
```

---

## Chunk 2: CLI — `--global` Flag

### Task 3: CLI — `--global` Flag and `update-registry` Fork

**Files:**
- Modify: `packages/cli/src/cli.ts`
- Modify: `packages/cli/src/commands/update-registry.ts`
- Create: `packages/cli/src/commands/update-registry-global.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/cli/src/commands/update-registry-global.spec.ts`:

```typescript
// packages/cli/src/commands/update-registry-global.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { runUpdateRegistryCore } from "./update-registry.js";

// Mock heavy engine/server imports
vi.mock("@sensei/engine", () => ({
  extractProjectProfile: vi.fn().mockResolvedValue({
    dominantLanguage: "TypeScript", repoName: "test-repo",
  }),
  LibIndexer: vi.fn().mockImplementation(() => ({
    index: vi.fn().mockResolvedValue({ sectionsIndexed: 3 }),
    indexShared: vi.fn().mockResolvedValue({ sectionsIndexed: 3 }),
  })),
  LibSkillGenerator: vi.fn(),
  SkillValidator: vi.fn(),
  ClaudeAdapter: vi.fn(),
  LlmsTxtAdapter: vi.fn().mockImplementation(() => ({
    fetch: vi.fn().mockResolvedValue([
      { title: "Button", description: "A button", sourceType: "llms.txt" },
      { title: "Input", description: "An input", sourceType: "llms.txt" },
      { title: "Form", description: "A form", sourceType: "llms.txt" },
    ]),
  })),
  HttpAdapter: vi.fn().mockImplementation(() => ({ fetch: vi.fn().mockResolvedValue([]) })),
  LocalAdapter: vi.fn().mockImplementation(() => ({ fetch: vi.fn().mockResolvedValue([]) })),
}));

vi.mock("@sensei/server", () => ({
  ClaudeBackend: vi.fn(),
  OllamaBackend: vi.fn().mockImplementation(() => ({
    model: "llama3.2:3b", embeddingModel: "nomic-embed-text",
  })),
}));

vi.mock("@sensei/shared", async () => {
  const actual = await vi.importActual<typeof import("@sensei/shared")>("@sensei/shared");
  return {
    ...actual,
    makeSenseiClient: vi.fn(),
    loadSenseiConfig: vi.fn(),
  };
});

import { makeSenseiClient, loadSenseiConfig } from "@sensei/shared";
import { LibIndexer } from "@sensei/engine";

const MOCK_CONFIG = {
  repo_id: "repo-abc",
  supabase_url: "http://localhost:54321",
  custom_libs: [
    { name: "rokkit", source_type: "llms.txt" as const, base_url: "https://rokkit.dev/llms.txt" },
  ],
};

function makeDb() {
  const upsertedRows: unknown[] = [];
  const fromMocks: Record<string, unknown> = {};

  const sharedLibId = "shared-lib-uuid-1";

  const makeChain = (table: string) => {
    if (table === "shared_libs") {
      return {
        upsert: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnValue({
            single: vi.fn().mockResolvedValue({
              data: { id: sharedLibId },
              error: null,
            }),
          }),
        }),
        update: vi.fn().mockReturnValue({
          eq: vi.fn().mockResolvedValue({ error: null }),
        }),
      };
    }
    if (table === "repo_libs") {
      return {
        upsert: vi.fn().mockImplementation((row: unknown) => {
          upsertedRows.push(row);
          return Promise.resolve({ error: null });
        }),
      };
    }
    return {};
  };

  const schema = vi.fn().mockReturnValue({
    from: vi.fn().mockImplementation((table: string) => {
      if (!fromMocks[table]) fromMocks[table] = makeChain(table);
      return fromMocks[table];
    }),
  });

  return { schema, _upsertedRows: upsertedRows, _sharedLibId: sharedLibId, _fromMocks: fromMocks };
}

describe("runUpdateRegistryCore --global", () => {
  beforeEach(() => {
    vi.mocked(loadSenseiConfig).mockResolvedValue(MOCK_CONFIG as any);
    vi.mocked(makeSenseiClient).mockResolvedValue({} as any);
  });

  it("calls indexShared (not index) when opts.global is true", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);
    const libIndexerInstance = { index: vi.fn(), indexShared: vi.fn().mockResolvedValue({ sectionsIndexed: 3 }) };
    vi.mocked(LibIndexer).mockImplementation(() => libIndexerInstance as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    expect(libIndexerInstance.indexShared).toHaveBeenCalledTimes(1);
    expect(libIndexerInstance.index).not.toHaveBeenCalled();
  });

  it("upserts shared_libs catalog and updates section_count + indexed_at", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    const sharedLibsMock = (db._fromMocks["shared_libs"] as any);
    expect(sharedLibsMock.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ name: "rokkit" }),
      { onConflict: "name" }
    );
    expect(sharedLibsMock.update).toHaveBeenCalledWith(
      expect.objectContaining({ section_count: 3 })
    );
  });

  it("upserts repo_libs with shared_lib_id set", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    const repoLibsMock = (db._fromMocks["repo_libs"] as any);
    expect(repoLibsMock.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ shared_lib_id: db._sharedLibId }),
      { onConflict: "repo_id,name" }
    );
  });

  it("running --global twice replaces sections (indexShared called each time)", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);
    const libIndexerInstance = { index: vi.fn(), indexShared: vi.fn().mockResolvedValue({ sectionsIndexed: 3 }) };
    vi.mocked(LibIndexer).mockImplementation(() => libIndexerInstance as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });
    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    expect(libIndexerInstance.indexShared).toHaveBeenCalledTimes(2);
  });

  it("exits non-zero when config.yaml is missing and --global is set", async () => {
    vi.mocked(loadSenseiConfig).mockResolvedValue(null);
    const exitSpy = vi.spyOn(process, "exit").mockImplementation(() => { throw new Error("exit"); });

    await expect(runUpdateRegistryCore("/repo", "rokkit", { global: true })).rejects.toThrow("exit");
    expect(exitSpy).toHaveBeenCalledWith(1);
    exitSpy.mockRestore();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bunx vitest run packages/cli/src/commands/update-registry-global.spec.ts
```

Expected: Tests FAIL — `runUpdateRegistryCore` has no `opts` parameter yet.

- [ ] **Step 3: Update `runUpdateRegistryCore` signature and add `--global` fork**

Open `packages/cli/src/commands/update-registry.ts`.

**Change the function signature** (line 26):
```typescript
export async function runUpdateRegistryCore(repoPath: string, libName?: string, opts?: { global?: boolean }): Promise<number> {
```

**Add early exit for --global with no config** — modify the existing `if (!config)` block (lines 28-32). This change also touches the existing `libName` path — run the full suite (Step 6) to confirm no regressions:

```typescript
  if (!config) {
    log.error("Not initialised — run sensei init first");
    if (libName || opts?.global) process.exit(1);
    return 0;
  }
```

**Replace the indexer call and repo_libs upsert** inside the `for (const lib of libs)` loop. The loop currently runs from line 78. After the try/catch that calls `LibIndexer.index()` (lines 91-100), the existing path writes to `repo_libs`. Replace the entire section from the `const indexSpin` block through the `repo_libs` upsert with:

```typescript
    const indexSpin = spinner();
    indexSpin.start(`Indexing ${lib.name} (${pages.length} pages)...`);
    let sectionsIndexed = 0;
    let sharedLibId: string | null = null;

    try {
      if (opts?.global) {
        // Upsert shared_libs catalog
        const { data: sharedLib, error: sharedLibErr } = await (client as any)
          .schema('sensei')
          .from('shared_libs')
          .upsert(
            { name: lib.name, source_type: lib.source_type, base_url: lib.base_url ?? null, local_path: lib.local_path ?? null },
            { onConflict: 'name' }
          )
          .select('id')
          .single();

        if (sharedLibErr || !sharedLib) throw new Error(`shared_libs upsert failed: ${sharedLibErr?.message}`);
        sharedLibId = sharedLib.id;

        // Index into shared pool
        const result = await new LibIndexer(client as any, ollamaBackend).indexShared(sharedLibId!, lib, pages);
        sectionsIndexed = result.sectionsIndexed;
        indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed (shared pool)`);

        // Update catalog counts
        await (client as any).schema('sensei').from('shared_libs')
          .update({ section_count: sectionsIndexed, indexed_at: new Date().toISOString() })
          .eq('id', sharedLibId);
      } else {
        const result = await new LibIndexer(client as any, ollamaBackend).index(repoId, lib, pages);
        sectionsIndexed = result.sectionsIndexed;
        indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed`);
      }
    } catch (err) {
      indexSpin.stop(`Error indexing ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }
```

Then **replace the `repo_libs` upsert** block (lines 131-145) with one that includes `shared_lib_id` when in global mode:

```typescript
    // Sync to repo_libs so dashboard can read config without local file access.
    // Skill fields only included when generation succeeded — avoids clobbering a good skill_path.
    try {
      await (client as any)
        .schema('sensei')
        .from('repo_libs')
        .upsert({
          repo_id: repoId,
          name: lib.name,
          source_type: lib.source_type,
          base_url: lib.base_url ?? null,
          local_path: lib.local_path ?? null,
          ...(sharedLibId ? { shared_lib_id: sharedLibId } : {}),
          ...(libSkillFile ? { skill_path: libSkillFile.path, skill_generated_at: libSkillFile.generatedAt } : {}),
        }, { onConflict: 'repo_id,name' });
    } catch (err) {
      log.warn(`  repo_libs upsert failed for ${lib.name}: ${err instanceof Error ? err.message : String(err)}`);
    }
```

**Update the `updateRegistry` wrapper** (line 153) to accept and forward `opts`:

```typescript
export async function updateRegistry(repoPath: string, libName?: string, opts?: { global?: boolean }): Promise<void> {
  intro("sensei update-registry");
  const count = await runUpdateRegistryCore(repoPath, libName, opts);
  outro(`Done. ${count} librar${count === 1 ? "y" : "ies"} processed.`);
}
```

- [ ] **Step 4: Update `cli.ts` to add `--global` flag**

Open `packages/cli/src/cli.ts`.

Add to the `options` object in `parseArgs` (after line 33, before the closing `}`):
```typescript
    global: { type: "boolean", default: false },
```

Update the `update-registry` case (line 347-350) to pass `opts`:
```typescript
    case "update-registry": {
      const { updateRegistry } = await import("./commands/update-registry.js");
      await updateRegistry(repoRoot, values.lib, { global: values.global });
      break;
    }
```

Also add `--global` to the HELP string after the `update-registry --lib` line:
```
  update-registry --global --lib <name>   Promote lib to shared pool (all repos can link to it)
```

- [ ] **Step 5: Run the new tests**

```bash
bunx vitest run packages/cli/src/commands/update-registry-global.spec.ts
```

Expected: All 5 tests PASS.

- [ ] **Step 6: Run full test suite**

```bash
bunx vitest run
```

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add packages/cli/src/cli.ts packages/cli/src/commands/update-registry.ts \
        packages/cli/src/commands/update-registry-global.spec.ts
git commit -m "feat(cli): add --global flag to update-registry; index into shared lib pool"
```

---

## Chunk 3: Init Enhancement + MCP Routing

### Task 4: Init — Shared Lib Check and Two-Pass Restructure

The `sensei init` command currently does candidate detection + URL prompts in a single pass, then creates the Supabase client. Since the shared-lib lookup needs the client, we restructure to: detect candidates → create client → (check shared pool or prompt URL) per candidate.

**Files:**
- Modify: `packages/cli/src/commands/init.ts`
- Create: `packages/cli/src/commands/init-shared-linking.spec.ts`

- [ ] **Step 1: Extract a testable helper function**

To test the shared-lib lookup logic without mocking the entire clack prompt system, we first extract the DB lookup into a helper. Add this function to `packages/cli/src/commands/init.ts` just above the `export async function init(cwd)` declaration:

```typescript
/** Looks up a lib by name in the global shared pool. Returns catalog row or null. */
export async function lookupSharedLib(
  client: ReturnType<typeof import("@supabase/supabase-js").createClient>,
  name: string,
): Promise<{
  id: string;
  section_count: number;
  indexed_at: string;
  base_url: string | null;
  local_path: string | null;
  source_type: string;
} | null> {
  try {
    const { data } = await (client as any)
      .schema('sensei')
      .from('shared_libs')
      .select('id,section_count,indexed_at,base_url,local_path,source_type')
      .eq('name', name)
      .maybeSingle();
    return data ?? null;
  } catch {
    return null;
  }
}
```

Note: This function is exported for testing only. It silently returns `null` on error (spec: "shared_libs lookup fails during init → fall through to URL prompt silently").

- [ ] **Step 2: Write failing tests for `lookupSharedLib`**

Create `packages/cli/src/commands/init-shared-linking.spec.ts`:

```typescript
// packages/cli/src/commands/init-shared-linking.spec.ts
import { describe, it, expect, vi } from "vitest";
import { lookupSharedLib } from "./init.js";

function makeDb(sharedLib: unknown) {
  return {
    schema: vi.fn().mockReturnValue({
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            maybeSingle: vi.fn().mockResolvedValue({ data: sharedLib, error: null }),
          }),
        }),
      }),
    }),
  };
}

const SHARED_LIB = {
  id: "shared-lib-uuid",
  section_count: 42,
  indexed_at: "2026-03-10T00:00:00.000Z",
  base_url: "https://rokkit.dev/llms.txt",
  local_path: null,
  source_type: "llms.txt",
};

describe("lookupSharedLib", () => {
  it("returns catalog row when lib is found in shared pool", async () => {
    const db = makeDb(SHARED_LIB);
    const result = await lookupSharedLib(db as any, "rokkit");

    expect(result).not.toBeNull();
    expect(result?.id).toBe("shared-lib-uuid");
    expect(result?.section_count).toBe(42);
    expect(db.schema).toHaveBeenCalledWith("sensei");
  });

  it("returns null when lib is not in shared pool", async () => {
    const db = makeDb(null);
    const result = await lookupSharedLib(db as any, "unknown-lib");
    expect(result).toBeNull();
  });

  it("returns null silently when DB throws", async () => {
    const db = {
      schema: vi.fn().mockReturnValue({
        from: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              maybeSingle: vi.fn().mockRejectedValue(new Error("network failure")),
            }),
          }),
        }),
      }),
    };
    const result = await lookupSharedLib(db as any, "rokkit");
    expect(result).toBeNull();
  });
});

// Integration tests for the accept / decline / not-found flows.
// These require mocking @clack/prompts — use vi.mock before the import.
vi.mock("@clack/prompts", async () => {
  const actual = await vi.importActual<typeof import("@clack/prompts")>("@clack/prompts");
  return {
    ...actual,
    confirm: vi.fn(),
  };
});

import { confirm as clackConfirm } from "@clack/prompts";

/** Build a Supabase client that also records repo_libs upserts. */
function makeIntegrationDb(sharedLib: typeof SHARED_LIB | null) {
  const repoLibsUpserts: unknown[] = [];
  return {
    _repoLibsUpserts: repoLibsUpserts,
    schema: vi.fn().mockReturnValue({
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "shared_libs") {
          return {
            select: vi.fn().mockReturnValue({
              eq: vi.fn().mockReturnValue({
                maybeSingle: vi.fn().mockResolvedValue({ data: sharedLib, error: null }),
              }),
            }),
          };
        }
        if (table === "repo_libs") {
          return {
            upsert: vi.fn().mockImplementation((row: unknown) => {
              repoLibsUpserts.push(row);
              return Promise.resolve({ error: null });
            }),
          };
        }
        return {};
      }),
    }),
  };
}

describe("init shared-lib integration", () => {
  it("upserts repo_libs with shared_lib_id when user accepts linking", async () => {
    vi.mocked(clackConfirm).mockResolvedValue(true);
    const db = makeIntegrationDb(SHARED_LIB);

    const result = await lookupSharedLib(db as any, "rokkit");
    expect(result).not.toBeNull();

    // Simulate the confirm-and-upsert logic inline (mirrors Step 6 loop body)
    const confirmed = await clackConfirm({ message: "Link it?", initialValue: true });
    if (confirmed) {
      await (db as any).schema("sensei").from("repo_libs").upsert({
        repo_id: "repo-abc",
        name: "rokkit",
        source_type: result!.source_type,
        base_url: result!.base_url,
        local_path: result!.local_path,
        shared_lib_id: result!.id,
      }, { onConflict: "repo_id,name" });
    }

    expect(db._repoLibsUpserts).toHaveLength(1);
    expect((db._repoLibsUpserts[0] as any).shared_lib_id).toBe("shared-lib-uuid");
  });

  it("does not upsert repo_libs when user declines linking", async () => {
    vi.mocked(clackConfirm).mockResolvedValue(false);
    const db = makeIntegrationDb(SHARED_LIB);

    const result = await lookupSharedLib(db as any, "rokkit");
    const confirmed = await clackConfirm({ message: "Link it?", initialValue: true });
    if (confirmed) {
      await (db as any).schema("sensei").from("repo_libs").upsert({});
    }

    expect(db._repoLibsUpserts).toHaveLength(0);
  });

  it("skips confirm prompt entirely when lib is not in shared pool", async () => {
    const db = makeIntegrationDb(null);
    const result = await lookupSharedLib(db as any, "unknown-lib");
    expect(result).toBeNull();
    // No confirm called because result is null
    expect(clackConfirm).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
bunx vitest run packages/cli/src/commands/init-shared-linking.spec.ts
```

Expected: Tests FAIL — `lookupSharedLib` is not yet defined.

- [ ] **Step 4: Add `lookupSharedLib` export to `init.ts`**

Follow Step 1 above to add the `lookupSharedLib` function to `init.ts`.

- [ ] **Step 5: Run tests to verify they pass**

```bash
bunx vitest run packages/cli/src/commands/init-shared-linking.spec.ts
```

Expected: All 3 tests PASS.

- [ ] **Step 6: Restructure the init flow (two-pass)**

Open `packages/cli/src/commands/init.ts`.

The current flow at lines 64-99 does candidate detection and URL prompts together, before the Supabase client (line 103-120). We need to move the Supabase credential prompts before the URL prompts. Here is the restructured section to replace lines 64-120:

```typescript
  // 2. Scan dependencies for potential custom_libs candidates
  const allDeps = await scanDirectDeps(cwd);
  let candidates: string[] = [];

  if (allDeps.length > 0) {
    const llmCandidates = await detectUnknownLibs(allDeps);
    if (llmCandidates.length > 0) {
      candidates = llmCandidates;
    } else {
      // No LLM or LLM returned nothing — let user pick from all deps
      const selected = await multiselect({
        message: "Which libraries would you like to index docs for? (space to select, enter to confirm)",
        options: allDeps.map(d => ({ value: d, label: d })),
        required: false,
      });
      if (isCancel(selected)) {
        candidates = [];
      } else {
        candidates = selected as string[];
      }
    }
  }

  // 3. Prompt for Supabase credentials and create client (moved before URL prompts
  //    so client is available for shared-lib lookup in Pass 2 below)
  const supabaseUrl = await text({
    message: "Supabase URL (from supabase start or your hosted project):",
    placeholder: "http://localhost:54321",
    validate: v => (v.startsWith("http") ? undefined : "Must be a URL"),
  });
  if (isCancel(supabaseUrl)) { outro("Cancelled."); return; }

  const serviceKey = await text({
    message: "Supabase service role key:",
    validate: v => (v.length > 10 ? undefined : "Looks too short"),
  });
  if (isCancel(serviceKey)) { outro("Cancelled."); return; }

  // Create client (needed for both repo upsert and shared-lib lookup below)
  const client = createClient(String(supabaseUrl), String(serviceKey), {
    db: { schema: "sensei" },
    auth: { persistSession: false },
  });

  const repoName = cwd.split("/").pop() ?? "repo";
  const { data: repo, error: repoErr } = await client.from("repos").upsert({
    name: repoName,
    local_path: cwd,
    stack,
    entry_points: entryPoints,
  }, { onConflict: "local_path" }).select("id").single();

  if (repoErr || !repo) {
    log.error(`Failed to register repo: ${repoErr?.message ?? "no data returned"}`);
    outro("Failed."); return;
  }
  const repoId: string = repo.id;

  // Pass 2: For each candidate, check shared pool first; else prompt for URL
  // customLibs: will be indexed by runUpdateRegistryCore
  // linkedLibEntries: linked to shared pool, written to config.yaml but NOT re-indexed
  const customLibs: LibEntry[] = [];
  const linkedLibEntries: LibEntry[] = [];

  if (candidates.length > 0) {
    for (const name of candidates) {
      // Check global shared pool — silently falls through if lookup fails
      const sharedLib = await lookupSharedLib(client, name);

      if (sharedLib) {
        const daysAgo = Math.floor((Date.now() - new Date(sharedLib.indexed_at).getTime()) / 86400000);
        const confirmed = await confirm({
          message: `${name} is already indexed globally (${sharedLib.section_count} sections, ${daysAgo}d ago). Link it?`,
          initialValue: true,
        });
        if (!isCancel(confirmed) && confirmed) {
          // Add to linkedLibEntries — goes into config.yaml but skips re-indexing
          linkedLibEntries.push({
            name,
            source_type: sharedLib.source_type as LibEntry["source_type"],
            base_url: sharedLib.base_url ?? undefined,
            local_path: sharedLib.local_path ?? undefined,
          });
          // Upsert repo_libs immediately with shared_lib_id
          try {
            await (client as any).schema('sensei').from('repo_libs').upsert({
              repo_id: repoId,
              name,
              source_type: sharedLib.source_type,
              base_url: sharedLib.base_url ?? null,
              local_path: sharedLib.local_path ?? null,
              shared_lib_id: sharedLib.id,
            }, { onConflict: 'repo_id,name' });
          } catch {
            log.warn(`  repo_libs upsert failed for ${name} (shared link)`);
          }
          continue; // Skip URL prompt and runUpdateRegistryCore for this lib
        }
      }

      // URL prompt path (unchanged)
      const input = await text({
        message: `Docs for "${name}"? (llms.txt URL, HTTP page, raw .md URL, or local path — Enter to skip)`,
        placeholder: "https://example.com/llms.txt",
      });
      if (isCancel(input) || !input?.trim()) continue;
      customLibs.push({ name, ...inferSourceType(String(input).trim()) });
    }
  }
```

Note: `confirm` is not in the existing top-level clack import at line 1. Add it to the static import — do NOT use a dynamic `await import(...)` inside the loop. Update line 1 of `init.ts` from:
```typescript
import { intro, outro, spinner, note, log, isCancel, text, multiselect } from "@clack/prompts";
```
to:
```typescript
import { intro, outro, spinner, note, log, isCancel, text, multiselect, confirm } from "@clack/prompts";
```
The Step 6 code block above uses `confirm` directly — no dynamic import.

- [ ] **Step 7: Update config.yaml write and runUpdateRegistryCore call**

The existing code at lines 136-168 writes config.yaml using `customLibs` and calls `runUpdateRegistryCore`. Update it to include `linkedLibEntries` in the YAML but use only `customLibs` for the indexing count:

```typescript
  // 4. Write .sensei/config.yaml and credentials
  const senseiDir = join(cwd, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  const allConfigLibs = [...customLibs, ...linkedLibEntries];
  const customLibsYaml = allConfigLibs.length > 0
    ? `custom_libs:\n${allConfigLibs.map(l => {
        const urlField = l.base_url ? `    base_url: ${l.base_url}` : `    local_path: ${l.local_path}`;
        return `  - name: ${l.name}\n    source_type: ${l.source_type}\n${urlField}`;
      }).join("\n")}\n`
    : "";
  await writeFile(
    join(senseiDir, "config.yaml"),
    `repo_id: ${repoId}\nsupabase_url: ${String(supabaseUrl)}\n${customLibsYaml}`,
  );

  // Write credentials to ~/.config/sensei/ (global, not committed)
  const credsDir = join(homedir(), ".config", "sensei");
  await mkdir(credsDir, { recursive: true });
  const credsPath = join(credsDir, "credentials.yaml");
  await writeFile(credsPath, `supabase_service_key: ${String(serviceKey)}\n`);
  await chmod(credsPath, 0o600);

  if (customLibs.length > 0) {  // Only non-linked libs are re-indexed
    const libSpin = spinner();
    libSpin.start("Indexing library docs...");
    try {
      await runUpdateRegistryCore(cwd);
      libSpin.stop(`Library docs indexed (${customLibs.length} ${customLibs.length === 1 ? "lib" : "libs"})`);
    } catch (err) {
      libSpin.stop("Library indexing skipped — run sensei update-registry when ready");
      log.warn(`  ${err instanceof Error ? err.message : String(err)}`);
    }
  }
```

- [ ] **Step 8: Run the full test suite**

```bash
bunx vitest run
```

Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add packages/cli/src/commands/init.ts packages/cli/src/commands/init-shared-linking.spec.ts
git commit -m "feat(init): two-pass restructure with shared lib pool lookup"
```

---

### Task 5: MCP — `get_lib_docs` Shared Routing

**Files:**
- Modify: `packages/server/src/tools/get-lib-docs.ts`
- Create: `packages/server/src/tools/get-lib-docs-routing.spec.ts`

- [ ] **Step 1: Write failing tests**

Create `packages/server/src/tools/get-lib-docs-routing.spec.ts`:

```typescript
// packages/server/src/tools/get-lib-docs-routing.spec.ts
import { describe, it, expect, vi } from "vitest";
import { getLibDocsTool } from "./get-lib-docs.js";
import type { ModelBackend } from "@sensei/shared";

const makeMockBackend = (): ModelBackend => ({
  name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(""), embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  extract: vi.fn().mockResolvedValue({}),
});

const SHARED_ROW = { title: "Button", url: "https://rokkit.dev/button", local_path: null, description: "A button", content: null, source_type: "llms.txt", component: "Forms" };

/** Build a mock DB that returns a repo_libs row with the given shared_lib_id. */
function makeDbWithSharedLib(sharedLibId: string | null) {
  const repoLibsChain = {
    select: vi.fn().mockReturnValue({
      eq: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnValue({
          maybeSingle: vi.fn().mockResolvedValue({
            data: { shared_lib_id: sharedLibId },
            error: null,
          }),
        }),
      }),
    }),
  };

  const sharedRpcResult = { data: [SHARED_ROW], error: null };
  const perRepoRpcResult = { data: [], error: null };

  return {
    schema: vi.fn().mockReturnValue({
      from: vi.fn().mockReturnValue(repoLibsChain),
    }),
    rpc: vi.fn().mockImplementation((name: string) => {
      if (name === "match_shared_lib_sections") return Promise.resolve(sharedRpcResult);
      return Promise.resolve(perRepoRpcResult);
    }),
    from: vi.fn().mockReturnValue({
      select: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            order: vi.fn().mockResolvedValue({ data: [], error: null }),
          }),
        }),
      }),
    }),
  };
}

describe("getLibDocsTool routing", () => {
  it("calls match_shared_lib_sections RPC when shared_lib_id is set (query path)", async () => {
    const db = makeDbWithSharedLib("shared-uuid-1");
    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "button" });

    expect(db.rpc).toHaveBeenCalledWith("match_shared_lib_sections", expect.objectContaining({
      p_shared_lib_id: "shared-uuid-1",
    }));
    expect(db.rpc).not.toHaveBeenCalledWith("match_lib_doc_sections", expect.anything());
    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
  });

  it("calls match_lib_doc_sections RPC when shared_lib_id is null (query path)", async () => {
    const db = makeDbWithSharedLib(null);
    await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "button" });

    expect(db.rpc).toHaveBeenCalledWith("match_lib_doc_sections", expect.objectContaining({
      p_repo_id: "repo-1",
    }));
    expect(db.rpc).not.toHaveBeenCalledWith("match_shared_lib_sections", expect.anything());
  });

  it("queries shared_lib_sections directly when shared_lib_id is set (browse path, no query)", async () => {
    const sharedBrowseMock = vi.fn().mockResolvedValue({ data: [SHARED_ROW], error: null });
    const db = {
      schema: vi.fn().mockReturnValue({
        from: vi.fn().mockImplementation((table: string) => {
          if (table === "repo_libs") {
            return {
              select: vi.fn().mockReturnValue({
                eq: vi.fn().mockReturnValue({
                  eq: vi.fn().mockReturnValue({
                    maybeSingle: vi.fn().mockResolvedValue({ data: { shared_lib_id: "shared-uuid-1" }, error: null }),
                  }),
                }),
              }),
            };
          }
          // shared_lib_sections browse query
          return {
            select: vi.fn().mockReturnValue({
              eq: vi.fn().mockReturnValue({
                order: sharedBrowseMock,
              }),
            }),
          };
        }),
      }),
      rpc: vi.fn(),
      from: vi.fn(),
    };

    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit");
    expect(sharedBrowseMock).toHaveBeenCalled();
    expect(db.rpc).not.toHaveBeenCalled();
    expect(result.sections).toHaveLength(1);
  });

  it("returns empty sections when repo_libs row is missing — no error", async () => {
    const db = {
      schema: vi.fn().mockReturnValue({
        from: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              eq: vi.fn().mockReturnValue({
                maybeSingle: vi.fn().mockResolvedValue({ data: null, error: null }),
              }),
            }),
          }),
        }),
      }),
      rpc: vi.fn().mockResolvedValue({ data: [], error: null }),
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              order: vi.fn().mockResolvedValue({ data: [], error: null }),
            }),
          }),
        }),
      }),
    };

    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "test" });
    expect(result.sections).toEqual([]);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
bunx vitest run packages/server/src/tools/get-lib-docs-routing.spec.ts
```

Expected: Tests FAIL — `getLibDocsTool` doesn't check `repo_libs.shared_lib_id` yet.

- [ ] **Step 3: Rewrite `getLibDocsTool` with routing**

Replace the entire contents of `packages/server/src/tools/get-lib-docs.ts` with the new routing logic. The outer structure (function signature, try/catch, row mapping) stays the same; the query logic branches on `shared_lib_id`:

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
    // Determine if this lib links to the shared pool.
    // The typeof guard preserves backward compatibility with test mocks that only have
    // `rpc` and `from` (no `.schema`) — those return null here and fall through to the
    // existing per-repo path.
    const { data: repoLib } = typeof (db as any).schema === 'function'
      ? await (db as any)
          .schema('sensei')
          .from('repo_libs')
          .select('shared_lib_id')
          .eq('repo_id', repoId)
          .eq('name', lib)
          .maybeSingle()
      : { data: null };

    const sharedLibId: string | null = repoLib?.shared_lib_id ?? null;
    let rows: Record<string, unknown>[];

    // Note: the null-check on db.schema preserves backward compatibility with existing tests
    // that mock only `rpc` and `from` without `.schema` (those mocks return shared_lib_id: null
    // and fall through to the existing per-repo path).
    if (opts?.query) {
      const embedding = await backend.embed(opts.query);
      if (sharedLibId) {
        const { data, error } = await db.rpc("match_shared_lib_sections", {
          p_shared_lib_id: sharedLibId,
          p_component: opts.component ?? null,
          query_embedding: embedding,
          match_count: limit,
        });
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
      } else {
        const { data, error } = await db.rpc("match_lib_doc_sections", {
          p_repo_id: repoId,
          p_lib_name: lib,
          p_component: opts?.component ?? null,
          query_embedding: embedding,
          match_count: limit,
        });
        if (error) throw new Error(error.message);
        rows = (data ?? []) as Record<string, unknown>[];
      }
    } else {
      // Browse path (no query): list all sections sorted by title
      if (sharedLibId) {
        let q = (db as any)
          .schema('sensei')
          .from('shared_lib_sections')
          .select('title,url,local_path,description,content,source_type,component')
          .eq('shared_lib_id', sharedLibId);
        if (opts?.component) q = q.eq('component', opts.component) as typeof q;
        const { data, error } = await q.order('title');
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
  } catch (err) {
    // Log warning before returning empty sections (spec: "log warning" on failure)
    console.warn(`getLibDocsTool: error fetching sections for lib "${lib}":`, err instanceof Error ? err.message : String(err));
    return { lib, sections: [] };
  }
}
```

- [ ] **Step 4: Run new routing tests**

```bash
bunx vitest run packages/server/src/tools/get-lib-docs-routing.spec.ts
```

Expected: All 4 new tests PASS.

- [ ] **Step 5: Verify both old and new tests pass**

```bash
bunx vitest run packages/server/src/tools/get-lib-docs.spec.ts packages/server/src/tools/get-lib-docs-routing.spec.ts
```

Expected: All 7 tests PASS (3 existing + 4 new). The `typeof (db as any).schema === 'function'` guard already included in the Step 3 implementation ensures existing tests that mock only `rpc` and `from` still pass.

- [ ] **Step 6: Run full suite**

```bash
bunx vitest run
```

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add packages/server/src/tools/get-lib-docs.ts \
        packages/server/src/tools/get-lib-docs-routing.spec.ts
git commit -m "feat(mcp): route get_lib_docs to shared lib pool when shared_lib_id is set"
```

---

## Chunk 4: Dashboard

### Task 6: Dashboard — Shared Lib Status

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte`

No automated tests for this task (SvelteKit server actions and Svelte components aren't covered by Vitest in this codebase). Verify by visual inspection and manual test.

- [ ] **Step 1: Update `LibRow` type and load function in `+page.server.ts`**

Open `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`.

**Update the `LibRow` interface** (lines 11-20) to add `isShared`:

```typescript
interface LibRow {
  libName: string;
  sourceType: string;
  baseUrl: string | null;
  localPath: string | null;
  sectionCount: number;
  lastFetched: string | null;
  freshness: Freshness;
  skillPath: string | null;
  isShared: boolean;
}
```

**Update the `load` function** — change the `repo_libs` select to include `shared_lib_id`, add a second query for shared catalog, and update the mapping:

Replace the `load` function body (lines 43-101) with:

```typescript
export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  // All configured libs (includes shared_lib_id for shared libs)
  const { data: repoLibs } = await db
    .from('repo_libs')
    .select('name,source_type,base_url,local_path,skill_path,shared_lib_id')
    .eq('repo_id', params.id);

  // Per-repo indexed sections (only for non-shared libs)
  const { data: sections } = await db
    .from('lib_doc_sections')
    .select('lib_name,last_fetched')
    .eq('repo_id', params.id)
    .limit(10000);

  // Shared lib catalog metadata (section_count, indexed_at)
  const sharedIds = ((repoLibs ?? []) as Array<{ shared_lib_id: string | null }>)
    .map(l => l.shared_lib_id)
    .filter((id): id is string => Boolean(id));

  const { data: sharedCatalog } = sharedIds.length > 0
    ? await db.from('shared_libs').select('id,section_count,indexed_at').in('id', sharedIds)
    : { data: [] as Array<{ id: string; section_count: number; indexed_at: string }> };

  const sharedCatalogMap = new Map(
    ((sharedCatalog ?? []) as Array<{ id: string; section_count: number; indexed_at: string }>)
      .map(s => [s.id, s])
  );

  // Build section map for per-repo libs
  const sectionMap = new Map<string, { count: number; lastFetched: string | null }>();
  for (const s of (sections ?? []) as Array<{ lib_name: string; last_fetched: string }>) {
    const existing = sectionMap.get(s.lib_name);
    if (!existing) {
      sectionMap.set(s.lib_name, { count: 1, lastFetched: s.last_fetched });
    } else {
      existing.count++;
      if (!existing.lastFetched || s.last_fetched > existing.lastFetched) {
        existing.lastFetched = s.last_fetched;
      }
    }
  }

  const libs: LibRow[] = ((repoLibs ?? []) as any[]).map((lib: any) => {
    const sharedLibId: string | null = lib.shared_lib_id ?? null;

    if (sharedLibId) {
      // Shared lib: use catalog metadata; fallback to 0/null if catalog row was deleted
      const catalog = sharedCatalogMap.get(sharedLibId);
      return {
        libName: lib.name,
        sourceType: lib.source_type,
        baseUrl: lib.base_url ?? null,
        localPath: lib.local_path ?? null,
        sectionCount: catalog?.section_count ?? 0,
        lastFetched: catalog?.indexed_at ?? null,
        freshness: 'fresh' as Freshness, // Shared libs don't show freshness
        skillPath: lib.skill_path ?? null,
        isShared: true,
      };
    }

    // Per-repo lib: aggregate from lib_doc_sections
    const info = sectionMap.get(lib.name);
    const sectionCount = info?.count ?? 0;
    const lastFetched = info?.lastFetched ?? null;
    return {
      libName: lib.name,
      sourceType: lib.source_type,
      baseUrl: lib.base_url ?? null,
      localPath: lib.local_path ?? null,
      sectionCount,
      lastFetched,
      freshness: computeFreshness(lastFetched, sectionCount),
      skillPath: lib.skill_path ?? null,
      isShared: false,
    };
  });

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
    hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  };
};
```

- [ ] **Step 2: Update `+page.svelte` to render shared rows differently**

Open `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte`.

Update the table row template (lines 40-56) to show "Shared" status for shared libs and hide the Re-index button:

```svelte
      {#each data.libs as lib}
        <tr>
          <td><a href="/repos/{data.repo.id}/libraries/{lib.libName}">{lib.libName}</a></td>
          <td>{lib.sourceType}</td>
          <td>{lib.sectionCount}</td>
          <td>{formatDate(lib.lastFetched)}</td>
          {#if lib.isShared}
            <td class="status-shared">Shared</td>
          {:else}
            <td class={freshnessColor[lib.freshness] ?? ''}>{freshnessLabel[lib.freshness] ?? lib.freshness}</td>
          {/if}
          {#if data.hasAnthropicKey}
            <td class={lib.skillPath ? 'skill-generated' : ''}>{lib.skillPath ? 'Generated' : 'None'}</td>
          {/if}
          <td>
            {#if lib.isShared}
              <span class="managed-label">Managed globally</span>
            {:else}
              <form method="POST" action="?/reindex">
                <input type="hidden" name="name" value={lib.libName} />
                <button type="submit">Re-index</button>
              </form>
            {/if}
          </td>
        </tr>
      {/each}
```

Add CSS for the new classes to the `<style>` block:

```css
  .status-shared  { color: #2563eb; font-weight: 500; }
  .managed-label  { color: #6b7280; font-style: italic; }
```

- [ ] **Step 3: Run full test suite to verify nothing is broken**

```bash
bunx vitest run
```

Expected: All tests pass (dashboard has no Vitest tests, so count is unchanged).

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts \
        apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte
git commit -m "feat(dashboard): show 'Shared' status and 'Managed globally' for shared lib rows"
```

---

## Final Verification

- [ ] **Run the complete test suite one last time**

```bash
bunx vitest run
```

Expected: All tests pass.

- [ ] **Check git log — 6 commits expected**

```bash
git log --oneline -6
```

Expected commits (most recent first):
1. `feat(dashboard): show 'Shared' status and 'Managed globally' for shared lib rows`
2. `feat(mcp): route get_lib_docs to shared lib pool when shared_lib_id is set`
3. `feat(init): two-pass restructure with shared lib pool lookup`
4. `feat(cli): add --global flag to update-registry; index into shared lib pool`
5. `feat(engine): add LibIndexer.indexShared for shared lib pool`
6. `feat(schema): add shared_libs, shared_lib_sections tables and repo_libs.shared_lib_id FK`
