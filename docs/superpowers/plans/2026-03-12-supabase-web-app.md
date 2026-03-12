# Supabase Web App Dashboard Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `apps/dashboard` — a SvelteKit + rokkit + kavach web app with 6 pages reading from Supabase (`sensei.*` tables) with magic link / anonymous auth.

**Architecture:** Scaffold with `sv create`, then `rokkit init` (UI components/theme), then `kavach init` (auth + route protection). Supabase-js reads `sensei.*` tables directly. No API layer — client reads from Supabase. Pages use seed data (from database plan) until backend migration is complete.

**Tech Stack:** SvelteKit 2, rokkit CLI, kavach CLI, @supabase/supabase-js, TypeScript, Bun

**Prerequisites:** Complete `2026-03-12-supabase-database-setup.md` plan first — Supabase must be running with seed data.

**Spec:** `docs/superpowers/specs/2026-03-12-supabase-central-store-design.md`

---

## Chunk 1: Scaffold app and configure Supabase

### Task 1: Create SvelteKit app with rokkit + kavach

**Files:**
- Create: `apps/dashboard/` (entire SvelteKit app)
- Create: `apps/dashboard/.env.local` (gitignored)

- [ ] **Step 1: Scaffold SvelteKit app**

```bash
cd /path/to/sensei
bunx sv create apps/dashboard
```

When prompted:
- **Which template?** → SvelteKit minimal (or Skeleton project)
- **Add type checking with TypeScript?** → Yes
- **Select additional options** → none required (rokkit handles styling)

- [ ] **Step 2: Install dependencies**

```bash
cd apps/dashboard
bun install
```

- [ ] **Step 3: Add rokkit**

```bash
rokkit init
```

Follow prompts. This adds rokkit components, theme, and layout wiring.

- [ ] **Step 4: Add kavach**

```bash
kavach init
```

When prompted for auth strategy: select **magic link** (or anonymous — whichever kavach prompts for). This wires auth routes, session handling, and route protection.

After `kavach init`, check that a root `src/routes/+layout.ts` (or `+layout.server.ts`) was created by kavach to load and propagate the auth session. If kavach did not create one, create it manually:

```typescript
// apps/dashboard/src/routes/+layout.ts
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ data }) => {
  return { session: data?.session ?? null };
};
```

This ensures auth session is available to all child pages for route protection.

- [ ] **Step 5: Install Supabase client**

```bash
bun add @supabase/supabase-js
```

- [ ] **Step 6: Create `apps/dashboard/.env.local`**

```bash
cat > .env.local << 'EOF'
PUBLIC_SUPABASE_URL=http://localhost:54321
PUBLIC_SUPABASE_ANON_KEY=<anon key from supabase status>
SUPABASE_SERVICE_ROLE_KEY=<service_role key from supabase status>
EOF
```

Get keys with:
```bash
supabase status
```

- [ ] **Step 7: Create Supabase client helper `apps/dashboard/src/lib/supabase.ts`**

```typescript
import { createClient } from '@supabase/supabase-js';
import { PUBLIC_SUPABASE_URL, PUBLIC_SUPABASE_ANON_KEY } from '$env/static/public';

export const supabase = createClient(PUBLIC_SUPABASE_URL, PUBLIC_SUPABASE_ANON_KEY);
```

- [ ] **Step 8: Verify app starts**

```bash
bun dev
```

Expected: app running at `http://localhost:5173`, no errors.

- [ ] **Step 9: Commit**

```bash
git add apps/dashboard/
git commit -m "feat(dashboard): scaffold SvelteKit app with rokkit + kavach + supabase-js"
```

---

## Chunk 2: Dashboard page

### Task 2: Build the Dashboard overview page

**Route:** `/` — repo count, recent events, index health

**Files:**
- Create/Modify: `apps/dashboard/src/routes/+page.svelte`
- Create: `apps/dashboard/src/routes/+page.ts` (load function)

- [ ] **Step 1: Create `apps/dashboard/src/routes/+page.ts`**

```typescript
import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  // Note: .schema() must come BEFORE .from() in supabase-js v2
  const db = supabase.schema('sensei');
  const [{ count: repoCount }, { data: recentEvents }, { data: repos }] = await Promise.all([
    db.from('repos').select('*', { count: 'exact', head: true }),
    db.from('events').select('tool, ts, phase')
      .order('ts', { ascending: false }).limit(10),
    db.from('repos').select('name, last_indexed_at, stack')
      .order('last_indexed_at', { ascending: false }).limit(5),
  ]);

  return { repoCount: repoCount ?? 0, recentEvents: recentEvents ?? [], repos: repos ?? [] };
};
```

- [ ] **Step 2: Create `apps/dashboard/src/routes/+page.svelte`**

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>Dashboard</h1>

<div class="stats-row">
  <div class="stat-card">
    <div class="stat-value">{data.repoCount}</div>
    <div class="stat-label">Indexed Repos</div>
  </div>
  <div class="stat-card">
    <div class="stat-value">{data.recentEvents.length}</div>
    <div class="stat-label">Recent Events</div>
  </div>
</div>

<section>
  <h2>Recent Activity</h2>
  <table>
    <thead>
      <tr><th>Tool</th><th>Phase</th><th>Time</th></tr>
    </thead>
    <tbody>
      {#each data.recentEvents as event}
        <tr>
          <td>{event.tool}</td>
          <td>{event.phase}</td>
          <td>{new Date(event.ts).toLocaleTimeString()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</section>

<section>
  <h2>Indexed Repos</h2>
  <ul>
    {#each data.repos as repo}
      <li>
        <strong>{repo.name}</strong>
        {#if repo.stack}<span class="tags">{repo.stack.join(', ')}</span>{/if}
        {#if repo.last_indexed_at}
          — indexed {new Date(repo.last_indexed_at).toLocaleDateString()}
        {/if}
      </li>
    {/each}
  </ul>
</section>
```

- [ ] **Step 3: Verify page loads with seed data**

```bash
bun dev
```

Open `http://localhost:5173`. Expected: 2 repos shown, 20 recent events.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/
git commit -m "feat(dashboard): add Dashboard overview page"
```

---

## Chunk 3: Stats page

### Task 3: Build the Stats page

**Route:** `/stats` — tool usage charts, gaps report

**Files:**
- Create: `apps/dashboard/src/routes/stats/+page.svelte`
- Create: `apps/dashboard/src/routes/stats/+page.ts`

- [ ] **Step 1: Create `apps/dashboard/src/routes/stats/+page.ts`**

```typescript
import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const db = supabase.schema('sensei');

  // Tool call counts: group by tool
  const { data: toolStats } = await db
    .from('events')
    .select('tool')
    .eq('phase', 'post');

  const toolCounts: Record<string, number> = {};
  for (const e of toolStats ?? []) {
    toolCounts[e.tool] = (toolCounts[e.tool] ?? 0) + 1;
  }

  const toolRows = Object.entries(toolCounts)
    .map(([tool, count]) => ({ tool, count }))
    .sort((a, b) => b.count - a.count);

  // Recent sessions
  const { data: sessions } = await db
    .from('events')
    .select('session_id, ts')
    .not('session_id', 'is', null)
    .order('ts', { ascending: false })
    .limit(100);

  const sessionIds = [...new Set((sessions ?? []).map(e => e.session_id))].slice(0, 10);

  // Gaps: tools used without a preceding sensei context load in the same session
  // A session with events but no 'load_context' or 'get_llmspec' tool is a gap session
  const allEvents = sessions ?? [];
  const sensei_tools = new Set(['load_context', 'get_llmspec', 'get_file_context', 'recommend_next']);
  const sessionsWithContext = new Set(
    allEvents.filter(e => sensei_tools.has(e.tool ?? '')).map(e => e.session_id)
  );
  const gapSessions = sessionIds.filter(id => !sessionsWithContext.has(id));

  return { toolRows, sessionCount: sessionIds.length, gapCount: gapSessions.length };
};
```

- [ ] **Step 2: Create `apps/dashboard/src/routes/stats/+page.svelte`**

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>Stats</h1>

<section>
  <h2>Tool Usage</h2>
  <table>
    <thead>
      <tr><th>Tool</th><th>Calls</th><th>Bar</th></tr>
    </thead>
    <tbody>
      {#each data.toolRows as { tool, count }}
        {@const max = data.toolRows[0]?.count ?? 1}
        <tr>
          <td>{tool}</td>
          <td>{count}</td>
          <td>
            <div style="width: {Math.round((count / max) * 200)}px; height: 12px; background: var(--color-primary, #6366f1)"></div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</section>

<section>
  <h2>Sessions</h2>
  <p>{data.sessionCount} recent sessions</p>
  {#if data.gapCount > 0}
    <p class="gap-warning">{data.gapCount} gap session(s) — no sensei context tool used</p>
  {/if}
</section>
```

- [ ] **Step 3: Verify**

```bash
bun dev
```

Open `http://localhost:5173/stats`. Expected: tool usage table with bars.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/stats/
git commit -m "feat(dashboard): add Stats page with tool usage table"
```

---

## Chunk 4: Repos page

### Task 4: Build the Repos page

**Route:** `/repos` — indexed repos, stack, drift status, last commit

**Files:**
- Create: `apps/dashboard/src/routes/repos/+page.svelte`
- Create: `apps/dashboard/src/routes/repos/+page.ts`

- [ ] **Step 1: Create `apps/dashboard/src/routes/repos/+page.ts`**

```typescript
import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: repos } = await supabase.schema('sensei')
    .from('repos')
    .select('id, name, remote_url, default_branch, stack, description, last_indexed_commit, last_indexed_at, is_public')
    .order('last_indexed_at', { ascending: false });

  return { repos: repos ?? [] };
};
```

- [ ] **Step 2: Create `apps/dashboard/src/routes/repos/+page.svelte`**

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>Repos</h1>

{#if data.repos.length === 0}
  <p>No repos indexed yet. Run <code>sensei index</code> to get started.</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Stack</th>
        <th>Branch</th>
        <th>Last Indexed</th>
        <th>Commit</th>
      </tr>
    </thead>
    <tbody>
      {#each data.repos as repo}
        <tr>
          <td><strong>{repo.name}</strong>{#if repo.description}<br><small>{repo.description}</small>{/if}</td>
          <td>{repo.stack?.join(', ') ?? '—'}</td>
          <td>{repo.default_branch ?? '—'}</td>
          <td>{repo.last_indexed_at ? new Date(repo.last_indexed_at).toLocaleString() : '—'}</td>
          <td><code>{repo.last_indexed_commit?.slice(0, 7) ?? '—'}</code></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
```

- [ ] **Step 3: Verify**

```bash
bun dev
```

Open `http://localhost:5173/repos`. Expected: 2 seed repos in table.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/routes/repos/
git commit -m "feat(dashboard): add Repos page"
```

---

## Chunk 5: Libraries and References pages

### Task 5: Build Libraries and References pages

**Routes:** `/libraries`, `/references`

**Files:**
- Create: `apps/dashboard/src/routes/libraries/+page.svelte`
- Create: `apps/dashboard/src/routes/libraries/+page.ts`
- Create: `apps/dashboard/src/routes/references/+page.svelte`
- Create: `apps/dashboard/src/routes/references/+page.ts`

- [ ] **Step 1: Create `apps/dashboard/src/routes/libraries/+page.ts`**

```typescript
import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: libraries } = await supabase.schema('sensei')
    .from('libraries')
    .select('name, ecosystem, version, description, homepage_url, llms_txt_url, llms_txt_fetched_at')
    .order('ecosystem')
    .order('name');

  return { libraries: libraries ?? [] };
};
```

- [ ] **Step 2: Create `apps/dashboard/src/routes/libraries/+page.svelte`**

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;

  let search = '';
  $: filtered = data.libraries.filter(l =>
    l.name.toLowerCase().includes(search.toLowerCase()) ||
    (l.description ?? '').toLowerCase().includes(search.toLowerCase())
  );
</script>

<h1>Libraries</h1>

<input type="search" placeholder="Search libraries…" bind:value={search} />

<table>
  <thead>
    <tr><th>Name</th><th>Ecosystem</th><th>Version</th><th>Description</th><th>llms.txt</th></tr>
  </thead>
  <tbody>
    {#each filtered as lib}
      <tr>
        <td>
          {#if lib.homepage_url}<a href={lib.homepage_url} target="_blank">{lib.name}</a>
          {:else}{lib.name}{/if}
        </td>
        <td>{lib.ecosystem}</td>
        <td>{lib.version ?? '—'}</td>
        <td>{lib.description ?? '—'}</td>
        <td>
          {#if lib.llms_txt_url}
            {lib.llms_txt_fetched_at ? '✓ cached' : 'not fetched'}
          {:else}—{/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
```

- [ ] **Step 3: Create `apps/dashboard/src/routes/references/+page.ts`**

```typescript
import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  // Note: "references" is a reserved SQL keyword — the table is named "references" (quoted in DDL).
  // "references" is a reserved SQL keyword but supabase-js passes it as a PostgREST path param, not raw SQL.
  const { data: references } = await supabase.schema('sensei')
    .from('references')
    .select('url, title, description, tags, fetched_at')
    .order('modified_at', { ascending: false });

  return { references: references ?? [] };
};
```

- [ ] **Step 4: Create `apps/dashboard/src/routes/references/+page.svelte`**

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>References</h1>

{#if data.references.length === 0}
  <p>No references yet.</p>
{:else}
  <ul>
    {#each data.references as ref}
      <li>
        <a href={ref.url} target="_blank">{ref.title ?? ref.url}</a>
        {#if ref.description}<br><small>{ref.description}</small>{/if}
        {#if ref.tags?.length}<br><span class="tags">{ref.tags.join(', ')}</span>{/if}
      </li>
    {/each}
  </ul>
{/if}
```

- [ ] **Step 5: Verify both pages**

```bash
bun dev
```

Open `/libraries` (5 seed libraries) and `/references` (empty — no seed data for this).

- [ ] **Step 6: Commit**

```bash
git add apps/dashboard/src/routes/libraries/ apps/dashboard/src/routes/references/
git commit -m "feat(dashboard): add Libraries and References pages"
```

---

## Chunk 6: Benchmark Reports page + navigation

### Task 6: Benchmark Reports page and app navigation

**Route:** `/benchmarks` — run results, strategy comparison

**Files:**
- Create: `apps/dashboard/src/routes/benchmarks/+page.svelte`
- Create: `apps/dashboard/src/routes/benchmarks/+page.ts`
- Modify: `apps/dashboard/src/routes/+layout.svelte` — add nav links

- [ ] **Step 1: Create `apps/dashboard/src/routes/benchmarks/+page.ts`**

```typescript
import type { PageLoad } from './$types';
import { supabase } from '$lib/supabase';

export const load: PageLoad = async () => {
  const { data: reports } = await supabase.schema('sensei')
    .from('benchmark_reports')
    .select('id, run_name, strategy, score, tokens, elapsed_ms, promoted, created_at, repo_id')
    .order('created_at', { ascending: false })
    .limit(50);

  return { reports: reports ?? [] };
};
```

- [ ] **Step 2: Create `apps/dashboard/src/routes/benchmarks/+page.svelte`**

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>Benchmark Reports</h1>

{#if data.reports.length === 0}
  <p>No benchmark runs yet.</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>Run</th>
        <th>Strategy</th>
        <th>Score</th>
        <th>Tokens</th>
        <th>Time (ms)</th>
        <th>Promoted</th>
        <th>Created</th>
      </tr>
    </thead>
    <tbody>
      {#each data.reports as r}
        <tr class:promoted={r.promoted}>
          <td>{r.run_name}</td>
          <td>{r.strategy}</td>
          <td>{r.score ?? '—'}</td>
          <td>{r.tokens ?? '—'}</td>
          <td>{r.elapsed_ms ?? '—'}</td>
          <td>{r.promoted ? '★' : ''}</td>
          <td>{new Date(r.created_at).toLocaleDateString()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  tr.promoted { background: var(--color-success-soft, #f0fdf4); }
</style>
```

- [ ] **Step 3: Add navigation to `+layout.svelte`**

Open `apps/dashboard/src/routes/+layout.svelte` (created by rokkit or SvelteKit scaffold) and add a nav bar. If rokkit provides a layout component, add links inside it. Otherwise, add a simple nav:

```svelte
<nav>
  <a href="/">Dashboard</a>
  <a href="/stats">Stats</a>
  <a href="/repos">Repos</a>
  <a href="/benchmarks">Benchmarks</a>
  <a href="/libraries">Libraries</a>
  <a href="/references">References</a>
</nav>
<slot />
```

- [ ] **Step 4: Final check — open all 6 pages**

```bash
bun dev
```

Verify all 6 routes load without errors:
- `/` — Dashboard
- `/stats` — Stats
- `/repos` — Repos (2 seed repos)
- `/benchmarks` — Benchmark Reports (empty)
- `/libraries` — Libraries (5 seed entries)
- `/references` — References (empty)

- [ ] **Step 5: Commit**

```bash
git add apps/dashboard/
git commit -m "feat(dashboard): add Benchmarks page and app navigation"
```
