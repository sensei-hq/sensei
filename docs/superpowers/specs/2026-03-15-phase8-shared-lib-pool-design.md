# Phase 8: Shared Library Pool — Design Spec

**Date:** 2026-03-15
**Status:** Approved

---

## Overview

Phase 8 introduces a global shared library pool so indexed library docs can be reused across repos without re-fetching. When a new repo declares it uses a library already in the shared pool (e.g. rokkit, kavach, dbd), it links to the existing sections rather than re-indexing. The CLI gains a `--global` flag to promote a lib into the shared pool. The `get_lib_docs` MCP tool transparently routes queries to the right table. The dashboard marks shared libs distinctly and omits the re-index button for them.

---

## Architecture

```
sensei update-registry --global --lib rokkit
  └─ fetch + index → shared_lib_sections (shared_lib_id FK)
  └─ upsert shared_libs catalog (section_count, indexed_at)
  └─ update current repo's repo_libs: set shared_lib_id

sensei init (new repo)
  └─ user adds lib name "dbd"
  └─ check shared_libs WHERE name = 'dbd'
  └─ found → prompt "dbd already indexed globally — link it?"
  └─ yes → config.yaml entry + repo_libs with shared_lib_id, no re-fetch
  └─ no  → URL prompt → per-repo indexing (existing flow)

get_lib_docs MCP tool
  └─ check repo_libs.shared_lib_id
  └─ set   → query shared_lib_sections via match_shared_lib_sections RPC
  └─ null  → query lib_doc_sections via match_lib_doc_sections RPC (unchanged)

Dashboard (libraries page)
  └─ shared rows → "Shared" status in blue, "Managed globally" in Actions
  └─ per-repo rows → unchanged (freshness, Re-index button)
```

---

## Component 1: Schema

### Migration

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

-- RPC for semantic search over shared lib sections
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

---

## Component 2: CLI (`update-registry.ts`)

### New `--global` flag

`cli.ts` adds `global: { type: 'boolean' }` to `parseArgs` options and passes it to both `updateRegistry` (the clack UI wrapper) and `runUpdateRegistryCore`. Both functions gain the same `opts` parameter:

```typescript
export async function updateRegistry(
  repoPath: string,
  libName?: string,
  opts?: { global?: boolean }
): Promise<void>
```

`updateRegistry` forwards `opts` to `runUpdateRegistryCore`. The `update-registry` dispatch call in `cli.ts` becomes:

```typescript
await updateRegistry(repoRoot, values.lib, { global: values.global });
```

`runUpdateRegistryCore` gains an `opts` parameter:

```typescript
export async function runUpdateRegistryCore(
  repoPath: string,
  libName?: string,
  opts?: { global?: boolean }
): Promise<number>
```

### LibIndexer extension

`LibIndexer` in `packages/engine/src/lib/lib-indexer.ts` gains a new `indexShared` method that writes to `shared_lib_sections` instead of `lib_doc_sections`. The embedding generation logic is identical to `index()`:

```typescript
async indexShared(
  sharedLibId: string,
  entry: LibEntry,
  pages: DocPage[],
): Promise<{ sectionsIndexed: number }> {
  await this.db.from('shared_lib_sections')
    .delete().eq('shared_lib_id', sharedLibId);

  const rows = await Promise.all(pages.map(async page => {
    const embedInput =
      entry.source_type === 'llms.txt'
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
  }));

  const { error } = await this.db.from('shared_lib_sections').insert(rows);
  if (error) throw new Error(`LibIndexer.indexShared: insert failed: ${error.message}`);
  return { sectionsIndexed: rows.length };
}
```

### Indexing fork

After fetch + pages are ready, the `--global` path replaces the `LibIndexer.index()` call:

```typescript
if (opts?.global) {
  // Upsert shared_libs catalog
  const { data: sharedLib, error: sharedLibErr } = await (client as any)
    .schema('sensei')
    .from('shared_libs')
    .upsert({ name: lib.name, source_type: lib.source_type, base_url: lib.base_url ?? null, local_path: lib.local_path ?? null },
      { onConflict: 'name' })
    .select('id')
    .single();

  if (sharedLibErr || !sharedLib) throw new Error(`shared_libs upsert failed: ${sharedLibErr?.message}`);
  const sharedLibId = sharedLib.id;

  // Index into shared pool using LibIndexer.indexShared
  const { sectionsIndexed } = await new LibIndexer(client as any, ollamaBackend)
    .indexShared(sharedLibId, lib, pages);

  // Update catalog counts
  await (client as any).schema('sensei').from('shared_libs')
    .update({ section_count: sectionsIndexed, indexed_at: new Date().toISOString() })
    .eq('id', sharedLibId);

  // Update current repo's repo_libs to point at shared pool
  await (client as any).schema('sensei').from('repo_libs')
    .upsert({ repo_id: repoId, name: lib.name, source_type: lib.source_type,
              base_url: lib.base_url ?? null, local_path: lib.local_path ?? null,
              shared_lib_id: sharedLibId },
      { onConflict: 'repo_id,name' });
} else {
  // existing LibIndexer.index() + repo_libs upsert path (unchanged)
}
```

Skill generation runs regardless of `--global` — skills are per-repo context and unaffected.

---

## Component 3: `sensei init` Enhancement

The shared-lib lookup requires the Supabase client, but the existing candidates URL-prompt loop runs before the client is created. The fix is a **two-pass restructure**:

1. **Pass 1 (unchanged):** Detect stack → detect candidates (LLM filter or multiselect). No URL prompts.
2. **Supabase setup (moved earlier):** Prompt Supabase URL + key, create client, upsert repo row.
3. **Pass 2 (replaces URL-prompt loop):** For each candidate `name`, check `shared_libs` using the now-available client; offer to link if found, else prompt for URL.

The URL prompt text and `customLibs.push(...)` logic are identical to the existing loop body — only the shared-lib check is prepended and the Supabase prompts are moved earlier.

```typescript
// Pass 2 — replaces lines 89-99 in the restructured init
for (const name of candidates) {
  // Check shared pool (client is now available)
  const { data: sharedLib } = await (client as any)
    .schema('sensei')
    .from('shared_libs')
    .select('id,name,section_count,indexed_at,base_url,local_path,source_type')
    .eq('name', name)
    .maybeSingle();  // returns null if not found (no error)

  if (sharedLib) {
    const daysAgo = Math.floor((Date.now() - new Date(sharedLib.indexed_at).getTime()) / 86400000);
    const confirmed = await confirm({
      message: `${name} is already indexed globally (${sharedLib.section_count} sections, ${daysAgo}d ago). Link it?`,
      initialValue: true,
    });
    if (!isCancel(confirmed) && confirmed) {
      // Push onto customLibs so the existing YAML serialization step picks it up
      customLibs.push({
        name,
        source_type: sharedLib.source_type,
        base_url: sharedLib.base_url ?? undefined,
        local_path: sharedLib.local_path ?? undefined,
      });
      // Upsert repo_libs immediately (before runUpdateRegistryCore) with shared_lib_id set
      await (client as any).schema('sensei').from('repo_libs')
        .upsert({ repo_id: repoId, name, source_type: sharedLib.source_type,
                  base_url: sharedLib.base_url ?? null, local_path: sharedLib.local_path ?? null,
                  shared_lib_id: sharedLib.id },
          { onConflict: 'repo_id,name' });
      continue; // skip URL prompt and runUpdateRegistryCore for this lib
    }
  }

  // URL prompt path (unchanged)
  const input = await text({
    message: `Docs for "${name}"? (llms.txt URL, HTTP page, raw .md URL, or local path — Enter to skip)`,
    placeholder: 'https://example.com/llms.txt',
  });
  if (isCancel(input) || !input?.trim()) continue;
  customLibs.push({ name, ...inferSourceType(String(input).trim()) });
}
```

**Separation of linked vs. indexable libs:**

Maintain two arrays in `init.ts`:
- `customLibs: LibEntry[]` — libs that need URL prompts and will be re-indexed by `runUpdateRegistryCore`
- `linkedLibEntries: LibEntry[]` — libs confirmed as shared-linked (skip URL prompt and re-indexing)

In Pass 2, confirmed shared-linked libs are pushed onto `linkedLibEntries` (not `customLibs`). When writing config.yaml, serialize both arrays together: `[...customLibs, ...linkedLibEntries]`. The existing `if (customLibs.length > 0) { runUpdateRegistryCore(cwd) }` guard uses only `customLibs`, so linked libs appear in config.yaml but are never re-indexed by this init run.

The `repo_libs` upsert for linked libs (inside the Pass 2 loop) uses `shared_lib_id: sharedLib.id` and must complete before `runUpdateRegistryCore` is called for the non-linked libs.

---

## Component 4: `get_lib_docs` MCP Tool

In `getLibDocsTool`, both the RPC path (`opts.query` set) and the browse path (no query) must route to the shared pool when `shared_lib_id` is set. The existing function wraps its entire body in `try/catch { return { lib, sections: [] } }` — the new `repo_libs` lookup goes inside that same `try` block, preserving the "never throw" contract:

```typescript
// Look up shared_lib_id for this repo+lib (via repo_libs)
const { data: repoLib } = await db
  .schema('sensei')
  .from('repo_libs')
  .select('shared_lib_id')
  .eq('repo_id', repoId)
  .eq('name', lib)
  .maybeSingle();

const sharedLibId = repoLib?.shared_lib_id ?? null;

if (opts?.query) {
  const embedding = await backend.embed(opts.query);
  if (sharedLibId) {
    // Route to shared pool (RPC)
    const { data } = await db.rpc('match_shared_lib_sections', {
      p_shared_lib_id: sharedLibId,
      p_component: opts.component ?? null,
      query_embedding: embedding,
      match_count: limit,
    });
    rows = (data ?? []) as Record<string, unknown>[];
  } else {
    // Per-repo RPC (unchanged)
    const { data, error } = await db.rpc('match_lib_doc_sections', {
      p_repo_id: repoId, p_lib_name: lib,
      p_component: opts.component ?? null,
      query_embedding: embedding, match_count: limit,
    });
    if (error) throw new Error(error.message);
    rows = (data ?? []) as Record<string, unknown>[];
  }
} else {
  // Browse path (no query): list all sections sorted by title
  if (sharedLibId) {
    // Route to shared pool (direct select)
    let q = db.schema('sensei')
      .from('shared_lib_sections')
      .select('title,url,local_path,description,content,source_type,component')
      .eq('shared_lib_id', sharedLibId);
    if (opts?.component) q = q.eq('component', opts.component) as typeof q;
    const { data, error } = await q.order('title');
    if (error) throw new Error(error.message);
    rows = (data ?? []) as Record<string, unknown>[];
  } else {
    // Per-repo direct select (unchanged)
    let q = db.from('lib_doc_sections')
      .select('title,url,local_path,description,content,source_type,component')
      .eq('repo_id', repoId).eq('lib_name', lib);
    if (opts?.component) q = q.eq('component', opts.component) as typeof q;
    const { data, error } = await q.order('title');
    if (error) throw new Error(error.message);
    rows = (data ?? []) as Record<string, unknown>[];
  }
}
```

External interface (inputs, outputs, errors) is unchanged.

---

## Component 5: Dashboard Libraries Page

### Load function (`+page.server.ts`)

Add `shared_lib_id` to the `repo_libs` select. Add join to `shared_libs` for shared rows:

```typescript
const { data: repoLibs } = await db
  .from('repo_libs')
  .select('name,source_type,base_url,local_path,skill_path,shared_lib_id')
  .eq('repo_id', params.id);

// For shared libs, fetch catalog metadata (section_count, indexed_at)
const sharedIds = (repoLibs ?? []).map(l => l.shared_lib_id).filter(Boolean);
const { data: sharedCatalog } = sharedIds.length > 0
  ? await db.from('shared_libs').select('id,section_count,indexed_at').in('id', sharedIds)
  : { data: [] };
```

`LibRow` is a local type defined in `+page.server.ts`. It gains `isShared: boolean`. The existing `sectionCount: number`, `lastFetched: string | null`, and `freshness` fields are populated from `shared_libs` catalog data for shared rows (instead of aggregating from `lib_doc_sections`). For shared rows, `freshness` is not computed — the status cell shows "Shared" directly.

```typescript
type LibRow = {
  libName: string;    // existing field name — not renamed
  sourceType: string;
  baseUrl: string | null;
  localPath: string | null;
  skillPath: string | null;
  sectionCount: number;
  lastFetched: string | null;
  freshness: 'fresh' | 'stale' | 'missing';
  isShared: boolean;  // NEW
};
```

The load function maps each `repo_libs` row: if `shared_lib_id` is set, look up the corresponding `sharedCatalog` entry and use its `section_count` and `indexed_at`; set `isShared: true` and `freshness: 'fresh'` (shared libs don't show freshness status). If the `sharedCatalog` entry is absent (e.g., the `shared_libs` row was deleted), default to `sectionCount: 0`, `lastFetched: null`, and still render the row as `isShared: true` (the dashboard shows "Shared" status with 0 sections).

### UI (`+page.svelte`)

- Shared rows: status cell shows "Shared" (blue); actions cell shows "Managed globally" (no button)
- Per-repo rows: unchanged

```svelte
{#if lib.isShared}
  <td class="status-shared">Shared</td>
  ...
  <td><span class="managed-label">Managed globally</span></td>
{:else}
  <td class={freshnessColor[lib.freshness] ?? ''}>{freshnessLabel[lib.freshness]}</td>
  ...
  <td><!-- Re-index button --></td>
{/if}
```

---

## Component 6: E2E Tests

### `packages/cli/src/commands/update-registry-global.spec.ts`
- `--global` flag writes to `shared_lib_sections`, not `lib_doc_sections`
- `shared_libs` catalog updated with correct `section_count` and `indexed_at`
- Current repo's `repo_libs` updated with `shared_lib_id`
- Running `--global` twice replaces sections (no duplicates)

### `packages/cli/src/commands/init-shared-linking.spec.ts`
- Lib found in `shared_libs` → confirm prompt shown with section count and age
- Accepting: `repo_libs` created with `shared_lib_id` set; `runUpdateRegistryCore` not called
- Declining: falls through to URL prompt (per-repo path)
- Lib not in `shared_libs`: no confirm prompt, proceeds directly to URL prompt

### `packages/server/src/tools/get-lib-docs-routing.spec.ts`
- `repo_libs.shared_lib_id` set → `match_shared_lib_sections` RPC called
- `repo_libs.shared_lib_id` null → `match_lib_doc_sections` RPC called (existing path)
- Missing `repo_libs` row → returns empty sections, no error

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| `--global` with no Supabase client | Log error, exit non-zero |
| `--global` with lib not in `config.yaml` | Same as existing `--lib` error: exit non-zero |
| `--global` with no `config.yaml` at all (no repo context) | Exit non-zero with error: `--global` requires a configured repo (config.yaml must exist); the `repo_libs` update cannot proceed without a `repo_id` |
| `shared_libs` lookup fails during init | Fall through to URL prompt silently |
| `match_shared_lib_sections` RPC fails | Return empty sections, log warning |
| `shared_lib_id` set but `shared_libs` row deleted | `match_shared_lib_sections` returns empty; dashboard shows 0 sections |

---

## Files Changed

| File | Change |
|------|--------|
| `supabase/migrations/20260315000002_phase8_shared_lib_pool.sql` | New `shared_libs`, `shared_lib_sections` tables; `repo_libs.shared_lib_id` column; `match_shared_lib_sections` RPC |
| `database/ddl/table/sensei/shared_libs.ddl` | DDL file for shared_libs |
| `database/ddl/table/sensei/shared_lib_sections.ddl` | DDL file for shared_lib_sections |
| `packages/cli/src/cli.ts` | Add `global: { type: 'boolean' }` to parseArgs; update `update-registry` dispatch to `await updateRegistry(repoRoot, values.lib, { global: values.global })` |
| `packages/engine/src/lib/lib-indexer.ts` | Add `indexShared(sharedLibId, entry, pages)` method |
| `packages/cli/src/commands/update-registry.ts` | Add `opts.global` fork; call `LibIndexer.indexShared` when set |
| `packages/cli/src/commands/init.ts` | Check `shared_libs` before URL prompt; link if found and confirmed |
| `packages/server/src/tools/get-lib-docs.ts` | Route query based on `repo_libs.shared_lib_id` |
| `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts` | Add `shared_lib_id` to select; join `shared_libs` catalog for shared rows |
| `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte` | Shared row status + "Managed globally" label |
| `packages/cli/src/commands/update-registry-global.spec.ts` | New — global indexing tests |
| `packages/cli/src/commands/init-shared-linking.spec.ts` | New — init shared linking tests |
| `packages/server/src/tools/get-lib-docs-routing.spec.ts` | New — MCP routing tests |

---

## Out of Scope

- Deleting shared libs from dashboard or CLI
- Auto-promoting all per-repo libs to shared
- Cross-user sharing (shared pool is instance-wide, not multi-tenant)
- Periodic re-indexing of stale shared libs
- Phase 9: SQLite removal from collector daemon
