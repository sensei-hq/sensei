# Phase 7: Dashboard Library Management — Design Spec

**Date:** 2026-03-15
**Status:** Approved

---

## Overview

Phase 7 moves `custom_libs` state from local config.yaml and `lib-skills.json` into Supabase, making it accessible to the dashboard without filesystem reads. It adds a `repo_libs` table as the shared source of truth, synced by the CLI after every `update-registry` run. The dashboard gains a libraries management UI: all configured libs are visible regardless of index status, unindexed libs can have a doc URL added inline, and the repo detail page shows an attention badge when libs need action.

---

## Architecture

```
CLI (update-registry)
  └─ reads config.yaml → processes lib → upserts to repo_libs

Dashboard (libraries page)
  └─ reads repo_libs + lib_doc_sections aggregate → joined LibRow[]
  └─ add action   → insert repo_libs → runUpdateRegistryCore(repoPath, name)
  └─ reindex action → runUpdateRegistryCore(repoPath, name)

Dashboard (repo detail page)
  └─ counts repo_libs with not_indexed or stale status → badge
```

Config.yaml remains the local CLI source of truth. Supabase gets a synced copy after every `update-registry` run. The dashboard never reads config.yaml or `lib-skills.json`.

---

## Component 1: `repo_libs` Supabase Table

### Migration

```sql
CREATE TABLE sensei.repo_libs (
  id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  repo_id             uuid NOT NULL REFERENCES sensei.repos(id) ON DELETE CASCADE,
  name                text NOT NULL,
  source_type         text NOT NULL CHECK (source_type IN ('llms.txt', 'http', 'local')),
  base_url            text,
  local_path          text,
  skill_path          text,
  skill_generated_at  timestamptz,
  created_at          timestamptz DEFAULT now(),
  UNIQUE(repo_id, name)
);

CREATE INDEX repo_libs_repo_id_idx ON sensei.repo_libs(repo_id);
```

`skill_path` and `skill_generated_at` replace the `lib-skills.json` manifest file for dashboard purposes. The CLI continues writing `lib-skills.json` to disk (unchanged), but the dashboard reads skill state from `repo_libs` only.

---

## Component 2: CLI Sync (`update-registry.ts`)

After the skill generation step for each lib (whether skill was generated or not), upsert into `repo_libs`:

```typescript
await (client as any)
  .schema('sensei')
  .from('repo_libs')
  .upsert({
    repo_id: repoId,
    name: lib.name,
    source_type: lib.source_type,
    base_url: lib.base_url ?? null,
    local_path: lib.local_path ?? null,
    skill_path: libSkillFile?.path ?? null,
    skill_generated_at: libSkillFile ? new Date().toISOString() : null,
  }, { onConflict: 'repo_id,name' });
```

This upsert runs for every lib successfully fetched and indexed, regardless of whether skill generation was attempted. If skill generation was skipped (no API key), `skill_path` and `skill_generated_at` remain null.

---

## Component 3: Libraries Page Rewrite

### Load Function (`+page.server.ts`)

1. Load repo by `params.id` (existing)
2. Query all `repo_libs` for this repo
3. Query `lib_doc_sections` aggregated by `lib_name`: `{ lib_name, count(*) as section_count, max(last_fetched) as last_fetched }`
4. Join on `lib_name = repo_libs.name`:
   - `sectionCount`: 0 if no matching sections row
   - `lastFetched`: null if no sections
5. Compute status:
   - `not_indexed`: `sectionCount === 0`
   - `stale`: `sectionCount > 0` and `lastFetched` older than 7 days
   - `fresh`: `sectionCount > 0` and `lastFetched` within 7 days
6. Compute attention count: entries where `status !== 'fresh'`
7. Remove `lib-skills.json` file read — skill data comes from `repo_libs.skill_path` and `repo_libs.skill_generated_at`

### Form Actions

**`add`** — Add a new lib and index it:
- Inputs: `name` (string), `url` (string)
- Validate `name` is non-empty; validate `url` with `URL` constructor (if HTTP) or accept as local path
- Call `inferSourceType(url)` to determine `source_type`, `base_url`, `local_path`
- Insert into `repo_libs` (upsert on conflict)
- Call `runUpdateRegistryCore(repoPath, name)`
- Redirect to same page

**`reindex`** — Re-index a single existing lib:
- Input: `name` (string, hidden form field)
- Call `runUpdateRegistryCore(repoPath, name)`
- Redirect to same page

**`update`** (existing) — Re-index all libs:
- Unchanged: calls `sensei update-registry` as subprocess

### UI (`+page.svelte`)

Table columns: Library | Source | Sections | Last Fetched | Status | Skill | Actions

- `not_indexed` rows: status cell shows "Not indexed" in red; Actions cell shows a URL `<input>` + Submit button (the `add` form, pre-filled with `name`)
- `stale` rows: Actions cell shows a Re-index button (the `reindex` form)
- `fresh` rows: Actions cell shows a Re-index button (always available)
- Skill column: show "Generated" if `skill_path` is set, "None" otherwise (only when `hasAnthropicKey`)

"Add Library" form at bottom of page: `name` text input + `url` text input + Add button (the `add` form without pre-filled name).

---

## Component 4: Repo Detail Page Badge

### Load Function (`+page.server.ts`)

Add to existing load:

```typescript
// Count libs needing attention
const { data: repLibs } = await db
  .from('repo_libs')
  .select('name')
  .eq('repo_id', params.id);

const { data: sections } = await db
  .from('lib_doc_sections')
  .select('lib_name, last_fetched')
  .eq('repo_id', params.id);

const sectionMap = new Map<string, string>();
for (const s of (sections ?? [])) {
  const existing = sectionMap.get(s.lib_name);
  if (!existing || s.last_fetched > existing) sectionMap.set(s.lib_name, s.last_fetched);
}

const STALE_MS = 7 * 24 * 60 * 60 * 1000;
const libAttentionCount = (repLibs ?? []).filter(l => {
  const lastFetched = sectionMap.get(l.name);
  if (!lastFetched) return true; // not indexed
  return Date.now() - new Date(lastFetched).getTime() > STALE_MS; // stale
}).length;
```

Return `libAttentionCount` alongside existing data.

### UI (`+page.svelte`)

Replace:
```svelte
<p><a href="/repos/{data.repo.id}/libraries">Library Docs →</a></p>
```

With:
```svelte
<p>
  <a href="/repos/{data.repo.id}/libraries">
    Library Docs →{#if data.libAttentionCount > 0} ⚠ {data.libAttentionCount} need attention{/if}
  </a>
</p>
```

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| `add` with malformed URL | Validate with `URL` constructor; return `fail(400, { error: "..." })` |
| `add` with duplicate lib name | Upsert on conflict — silently updates |
| `reindex` for lib not in `repo_libs` | `runUpdateRegistryCore` handles "not found" with `process.exit(1)`; catch and return `fail(500)` |
| `update-registry` upsert to `repo_libs` fails | Log warning, continue — indexing still proceeded |
| `repo_libs` query returns empty | Libraries page shows empty table with "Add Library" form only |

---

## Files Changed

| File | Change |
|------|--------|
| `supabase/migrations/20260315000001_phase7_repo_libs.sql` | New `repo_libs` table + index |
| `packages/cli/src/commands/update-registry.ts` | Upsert to `repo_libs` after each lib processed |
| `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts` | Rewrite load; add `add` and `reindex` actions; remove `lib-skills.json` read |
| `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte` | Show all libs; URL input for `not_indexed` rows; Re-index button per row; Add Library form |
| `apps/dashboard/src/routes/repos/[id]/+page.server.ts` | Add `libAttentionCount` query |
| `apps/dashboard/src/routes/repos/[id]/+page.svelte` | Attention badge on Library Docs link |

---

## Out of Scope

- Deleting libs via dashboard
- Auto periodic re-indexing of stale libs
- Removing `lib-skills.json` from disk (CLI still writes it)
- Phase 8: SQLite removal from collector daemon
