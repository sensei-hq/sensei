# Phase 7: Dashboard Library Management Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `custom_libs` and skill state into a Supabase `repo_libs` table so the dashboard can display and manage all configured libraries without reading local files.

**Architecture:** A new `repo_libs` table is the shared source of truth; the CLI upserts into it after every `update-registry` run; the dashboard joins it with `lib_doc_sections` to show freshness status, adds a URL-entry form for unindexed libs, and shows an attention badge on the repo detail page.

**Tech Stack:** TypeScript, Bun, SvelteKit 2, Supabase JS, `js-yaml`, `@clack/prompts`, Vitest

---

## File Map

| File | Role |
|------|------|
| `supabase/migrations/20260315000001_phase7_repo_libs.sql` | Create `repo_libs` table |
| `packages/cli/src/commands/update-registry.ts` | Add `repo_libs` upsert after successful fetch+index |
| `apps/dashboard/package.json` | Add `js-yaml` direct dependency |
| `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts` | Rewrite: join `repo_libs` + `lib_doc_sections`; `add` and `reindex` actions |
| `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte` | Rewrite: all libs visible; URL input for missing; Re-index button per row |
| `apps/dashboard/src/routes/repos/[id]/+page.server.ts` | Add `libAttentionCount` query |
| `apps/dashboard/src/routes/repos/[id]/+page.svelte` | Add attention badge on Library Docs link |

---

## Chunk 1: Database + CLI Sync

### Task 1: Create repo_libs migration

**Files:**
- Create: `supabase/migrations/20260315000001_phase7_repo_libs.sql`

No tests — SQL migration file.

- [ ] **Step 1: Create the migration file**

```sql
-- supabase/migrations/20260315000001_phase7_repo_libs.sql
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

- [ ] **Step 2: Commit**

```bash
git add supabase/migrations/20260315000001_phase7_repo_libs.sql
git commit -m "feat(db): add repo_libs table for dashboard-accessible lib config"
```

---

### Task 2: Add repo_libs upsert to update-registry.ts

**Files:**
- Modify: `packages/cli/src/commands/update-registry.ts`

The current file declares `const libSkillFile` inside the `if (hasAnthropicKey)` block (line 112). We need to move the declaration before the block so the upsert can reference it regardless of whether skill generation ran.

- [ ] **Step 1: Read the current file**

Read `packages/cli/src/commands/update-registry.ts` in full to confirm current structure before editing.

- [ ] **Step 2: Add LibSkillFile to the shared import**

Find the import on line 17:
```typescript
import { makeSenseiClient, loadSenseiConfig, type LibEntry, type LibSkillsManifest } from "@sensei/shared";
```

Replace with:
```typescript
import { makeSenseiClient, loadSenseiConfig, type LibEntry, type LibSkillFile, type LibSkillsManifest } from "@sensei/shared";
```

- [ ] **Step 3: Restructure the skill block and add upsert**

Find the `if (hasAnthropicKey)` block (approximately lines 102–125). Replace the entire block (from `if (hasAnthropicKey)` through its closing `}`) with:

```typescript
    let libSkillFile: LibSkillFile | undefined;
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
        libSkillFile = await new ClaudeAdapter().writeLibSkill(lib.name, markdown, repoSlug);

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

    // Sync to repo_libs so dashboard can read config without local file access
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
          skill_path: libSkillFile?.path ?? null,
          skill_generated_at: libSkillFile ? new Date().toISOString() : null,
        }, { onConflict: 'repo_id,name' });
    } catch (err) {
      log.warn(`  repo_libs upsert failed for ${lib.name}: ${err instanceof Error ? err.message : String(err)}`);
    }
    // ← upsert closes here; next line is the closing `}` of the for loop
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd packages/cli && bunx tsc --noEmit 2>&1 | head -20
```

Expected: no output (zero TypeScript errors across the package)

- [ ] **Step 5: Run the test suite to confirm no regressions**

```bash
bunx vitest run 2>&1 | tail -5
```

Expected: all tests pass (447+)

- [ ] **Step 6: Commit**

```bash
git add packages/cli/src/commands/update-registry.ts
git commit -m "feat(cli): upsert lib config to repo_libs after each successful update-registry run"
```

---

## Chunk 2: Libraries Page Rewrite

### Task 3: Rewrite libraries +page.server.ts

**Files:**
- Modify: `apps/dashboard/package.json`
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts`

The current file reads `lib-skills.json` from disk — that read is removed. All lib data now comes from Supabase.

- [ ] **Step 1: Add js-yaml to dashboard dependencies**

In `apps/dashboard/package.json`, add to the `dependencies` object:
```json
"js-yaml": "^4.1.0",
```

Then install:
```bash
bun install
```

- [ ] **Step 2: Rewrite the file**

Replace the entire contents of `apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts` with:

```typescript
// apps/dashboard/src/routes/repos/[id]/libraries/+page.server.ts
import type { PageServerLoad, Actions } from './$types';
import { error, fail, redirect } from '@sveltejs/kit';
import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { getDb } from '$lib/server/db';
import yaml from 'js-yaml';

type Freshness = 'fresh' | 'stale' | 'missing';

interface LibRow {
  libName: string;
  sourceType: string;
  baseUrl: string | null;
  localPath: string | null;
  sectionCount: number;
  lastFetched: string | null;
  freshness: Freshness;
  skillPath: string | null;
}

const STALE_DAYS = 7;

function computeFreshness(lastFetched: string | null, sectionCount: number): Freshness {
  if (sectionCount === 0) return 'missing';
  const ageMs = Date.now() - new Date(lastFetched!).getTime();
  return ageMs / (1000 * 60 * 60 * 24) > STALE_DAYS ? 'stale' : 'fresh';
}

function inferSourceType(input: string): { source_type: 'llms.txt' | 'http' | 'local'; base_url?: string; local_path?: string } {
  if (input.startsWith('http://') || input.startsWith('https://')) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith('/llms.txt') || url.pathname === '/llms.txt') {
        return { source_type: 'llms.txt', base_url: input };
      }
    } catch { /* fall through */ }
    return { source_type: 'http', base_url: input };
  }
  return { source_type: 'local', local_path: input };
}

export const load: PageServerLoad = async ({ params }) => {
  const db = getDb();

  const { data: repo } = await db
    .from('repos')
    .select('id,name,local_path')
    .eq('id', params.id)
    .single();

  if (!repo) throw error(404, 'Repo not found');

  // All configured libs from Supabase (replaces config.yaml + lib-skills.json reads)
  const { data: repoLibs } = await db
    .from('repo_libs')
    .select('name,source_type,base_url,local_path,skill_path')
    .eq('repo_id', params.id);

  // All indexed sections — aggregate by lib_name in JS (Supabase JS client has no GROUP BY)
  const { data: sections } = await db
    .from('lib_doc_sections')
    .select('lib_name,last_fetched')
    .eq('repo_id', params.id);

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

  const libs: LibRow[] = (repoLibs ?? []).map((lib: any) => {
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
    };
  });

  return {
    repo: repo as { id: string; name: string; local_path: string },
    libs,
    hasAnthropicKey: Boolean(process.env.ANTHROPIC_API_KEY),
  };
};

export const actions: Actions = {
  add: async ({ params, request }) => {
    const db = getDb();
    const { data: repo } = await db.from('repos').select('local_path').eq('id', params.id).single();
    if (!repo) return fail(404, { error: 'Repo not found' });
    const repoPath = (repo as { local_path: string }).local_path;

    const formData = await request.formData();
    const name = String(formData.get('name') ?? '').trim();
    const url = String(formData.get('url') ?? '').trim();

    if (!name) return fail(400, { error: 'Library name is required' });
    if (!url) return fail(400, { error: 'URL or path is required' });

    if (url.startsWith('http://') || url.startsWith('https://')) {
      try { new URL(url); } catch { return fail(400, { error: 'Invalid URL' }); }
    }

    const { source_type, base_url, local_path } = inferSourceType(url);

    // Append to config.yaml so CLI tools can find it
    try {
      const configPath = join(repoPath, '.sensei', 'config.yaml');
      const raw = await readFile(configPath, 'utf-8');
      const config = yaml.load(raw) as Record<string, unknown>;
      const customLibs = (Array.isArray(config.custom_libs) ? config.custom_libs : []) as unknown[];
      if (!customLibs.some((l: unknown) => (l as { name: string }).name === name)) {
        customLibs.push({ name, source_type, ...(base_url ? { base_url } : { local_path }) });
        config.custom_libs = customLibs;
        await writeFile(configPath, yaml.dump(config), 'utf-8');
      }
    } catch (err) {
      return fail(500, { error: `Could not update config.yaml: ${err instanceof Error ? err.message : String(err)}` });
    }

    // Upsert to repo_libs for immediate dashboard visibility
    await db.from('repo_libs').upsert(
      { repo_id: params.id, name, source_type, base_url: base_url ?? null, local_path: local_path ?? null },
      { onConflict: 'repo_id,name' }
    );

    // Index the new lib
    try {
      const proc = Bun.spawn(
        ['sensei', 'update-registry', '--lib', name],
        { cwd: repoPath, env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' }
      );
      const timeout = setTimeout(() => proc.kill(), 60_000);
      const exitCode = await proc.exited;
      clearTimeout(timeout);
      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || `sensei update-registry --lib ${name} failed` });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei update-registry' });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },

  reindex: async ({ params, request }) => {
    const db = getDb();
    const { data: repo } = await db.from('repos').select('local_path').eq('id', params.id).single();
    if (!repo) return fail(404, { error: 'Repo not found' });
    const repoPath = (repo as { local_path: string }).local_path;

    const formData = await request.formData();
    const name = String(formData.get('name') ?? '').trim();
    if (!name) return fail(400, { error: 'Library name is required' });

    try {
      const proc = Bun.spawn(
        ['sensei', 'update-registry', '--lib', name],
        { cwd: repoPath, env: { ...process.env }, stdout: 'pipe', stderr: 'pipe' }
      );
      const timeout = setTimeout(() => proc.kill(), 60_000);
      const exitCode = await proc.exited;
      clearTimeout(timeout);
      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text();
        return fail(500, { error: stderr || `sensei update-registry --lib ${name} failed` });
      }
    } catch (err) {
      return fail(500, { error: err instanceof Error ? err.message : 'Failed to run sensei update-registry' });
    }

    redirect(303, `/repos/${params.id}/libraries`);
  },

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

- [ ] **Step 3: Verify TypeScript**

```bash
cd apps/dashboard && bunx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | grep "libraries/+page.server" | head -10 || echo "no errors in libraries page.server"
```

Expected: no type errors in the libraries page.server file

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/package.json apps/dashboard/src/routes/repos/\[id\]/libraries/+page.server.ts
git commit -m "feat(dashboard): rewrite libraries page.server — join repo_libs+lib_doc_sections, add add/reindex actions"
```

---

### Task 4: Rewrite libraries +page.svelte

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte`

- [ ] **Step 1: Read the current file**

Read `apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte` to understand what's currently there.

- [ ] **Step 2: Replace the file**

```svelte
<!-- apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte -->
<script lang="ts">
  import type { PageData, ActionData } from './$types';
  const { data, form }: { data: PageData; form: ActionData } = $props();

  const freshnessColor: Record<string, string> = {
    fresh: 'status-fresh', stale: 'status-stale', missing: 'status-missing',
  };
  const freshnessLabel: Record<string, string> = {
    fresh: 'Fresh', stale: 'Stale', missing: 'Not indexed',
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
        <th>Status</th>
        {#if data.hasAnthropicKey}<th>Skill</th>{/if}
        <th>Actions</th>
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
            <td class={lib.skillPath ? 'skill-generated' : ''}>{lib.skillPath ? 'Generated' : 'None'}</td>
          {/if}
          <td>
            {#if lib.freshness === 'missing'}
              <form method="POST" action="?/add" class="inline-form">
                <input type="hidden" name="name" value={lib.libName} />
                <input type="text" name="url" placeholder="https://example.com/llms.txt" required />
                <button type="submit">Add Docs</button>
              </form>
            {:else}
              <form method="POST" action="?/reindex">
                <input type="hidden" name="name" value={lib.libName} />
                <button type="submit">Re-index</button>
              </form>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No library docs configured yet.</p>
{/if}

<h2>Add Library</h2>
<form method="POST" action="?/add" class="add-form">
  <label>Name <input type="text" name="name" placeholder="my-lib" required /></label>
  <label>URL or path <input type="text" name="url" placeholder="https://example.com/llms.txt" required /></label>
  <button type="submit">Add &amp; Index</button>
</form>

<form method="POST" action="?/update">
  <button type="submit">Re-index All</button>
</form>

<style>
  .status-fresh   { color: green; }
  .status-stale   { color: goldenrod; }
  .status-missing { color: red; }
  .skill-generated { color: green; }
  .error { color: red; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
  .inline-form { display: flex; gap: 0.5rem; align-items: center; }
  .add-form { display: flex; flex-direction: column; gap: 0.5rem; max-width: 400px; margin: 1rem 0; }
</style>
```

- [ ] **Step 3: Verify TypeScript**

```bash
cd apps/dashboard && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | grep "libraries/+page" | head -10 || echo "no errors in libraries page"
```

Expected: no errors in the libraries page files

- [ ] **Step 4: Run full test suite to confirm no regressions**

```bash
bunx vitest run 2>&1 | tail -5
```

Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/libraries/+page.svelte
git commit -m "feat(dashboard): rewrite libraries UI — all libs visible, add URL input for missing, re-index buttons"
```

---

## Chunk 3: Repo Detail Attention Badge

### Task 5: Add libAttentionCount to repo detail +page.server.ts

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.server.ts`

- [ ] **Step 1: Read the current file**

Read `apps/dashboard/src/routes/repos/[id]/+page.server.ts` to see the current load function.

- [ ] **Step 2: Add the attention count query**

In the load function, after the existing `symbols` query and before the `return` statement, insert:

```typescript
  // Count libs needing attention (missing or stale)
  const { data: repoLibs } = await db
    .from('repo_libs')
    .select('name')
    .eq('repo_id', params.id);

  const { data: libSections } = await db
    .from('lib_doc_sections')
    .select('lib_name,last_fetched')
    .eq('repo_id', params.id);

  const libSectionMap = new Map<string, string>();
  for (const s of (libSections ?? []) as Array<{ lib_name: string; last_fetched: string }>) {
    const existing = libSectionMap.get(s.lib_name);
    if (!existing || s.last_fetched > existing) libSectionMap.set(s.lib_name, s.last_fetched);
  }

  const STALE_MS = 7 * 24 * 60 * 60 * 1000;
  const libAttentionCount = (repoLibs ?? []).filter((l: any) => {
    const lastFetched = libSectionMap.get(l.name);
    if (!lastFetched) return true;
    return Date.now() - new Date(lastFetched).getTime() > STALE_MS;
  }).length;
```

Update the return statement to include `libAttentionCount`:

```typescript
  return { repo, symbols: symbols ?? [], libAttentionCount };
```

- [ ] **Step 3: Verify TypeScript**

```bash
cd apps/dashboard && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | grep "repos/\[id\]/+page" | head -10 || echo "no errors"
```

Expected: no errors in the repo detail page files

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/+page.server.ts
git commit -m "feat(dashboard): add libAttentionCount to repo detail page load"
```

---

### Task 6: Add attention badge to repo detail +page.svelte

**Files:**
- Modify: `apps/dashboard/src/routes/repos/[id]/+page.svelte`

- [ ] **Step 1: Read the current file**

Read `apps/dashboard/src/routes/repos/[id]/+page.svelte` to find the Library Docs link.

- [ ] **Step 2: Replace the Library Docs link**

Find the line:
```svelte
<p><a href="/repos/{data.repo.id}/libraries">Library Docs →</a></p>
```

Replace with:
```svelte
<p>
  <a href="/repos/{data.repo.id}/libraries">
    Library Docs →{#if data.libAttentionCount > 0} ⚠ {data.libAttentionCount} need attention{/if}
  </a>
</p>
```

- [ ] **Step 3: Verify TypeScript**

```bash
cd apps/dashboard && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | grep "repos/\[id\]" | grep -v "libraries" | head -10 || echo "no errors"
```

Expected: no errors in repo detail page files

- [ ] **Step 4: Run full test suite**

```bash
bunx vitest run 2>&1 | tail -5
```

Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add apps/dashboard/src/routes/repos/\[id\]/+page.svelte
git commit -m "feat(dashboard): add attention badge to Library Docs link on repo detail page"
```
