<script lang="ts">
  import { ThemeSwitcherToggle } from '@rokkit/app';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  // ── navigation state ────────────────────────────────────────────
  let wsId        = $state('personal');
  let expandedRepo = $state<string | null>('r1');
  let activeRepo  = $state<string | null>('r1');
  let activeView  = $state<'sessions'|'analytics'|'context'|'agents'|'drift'|'libraries'|'home'>('sessions');
  let wsOpen      = $state(false);

  // ── derived ─────────────────────────────────────────────────────
  const workspace    = $derived(data.workspaces.find(w => w.id === wsId)!);
  const wsRepos      = $derived(data.repos.filter(r => r.workspaceId === wsId));
  const wsTeams      = $derived(data.teams.filter(t => t.workspaceId === wsId));
  const currentRepo  = $derived(data.repos.find(r => r.id === activeRepo) ?? null);
  const viewSessions = $derived(data.sessions.filter(s => s.repoId === activeRepo));

  function selectRepo(id: string) {
    activeRepo   = id;
    activeView   = 'sessions';
    expandedRepo = id;
  }

  function ftrBadge(ftr: number) {
    if (ftr >= 0.95) return 'bg-success-z1 text-success-z7 border-success-z3';
    if (ftr >= 0.8)  return 'bg-warning-z1 text-warning-z7 border-warning-z3';
    return 'bg-error-z1 text-error-z7 border-error-z3';
  }
  function ftrText(ftr: number) {
    if (ftr >= 0.95) return 'text-success-z6';
    if (ftr >= 0.8)  return 'text-warning-z6';
    return 'text-error-z6';
  }

  const avgFtr   = $derived(viewSessions.length ? viewSessions.reduce((a, s) => a + s.ftr, 0) / viewSessions.length : 0);
  const totalCost = $derived(viewSessions.reduce((a, s) => a + s.cost, 0));

  const subNav: Array<{ key: typeof activeView; label: string }> = [
    { key: 'sessions',  label: 'Sessions'      },
    { key: 'analytics', label: 'Analytics'     },
    { key: 'context',   label: 'Context Packs' },
    { key: 'agents',    label: 'Agents'        },
    { key: 'drift',     label: 'Drift'         },
  ];
</script>

<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<!-- Floating option switcher -->
<div class="fixed bottom-4 right-4 z-50 flex items-center gap-1 rounded-full border border-surface-z4 bg-surface-z2 px-3 py-1.5 shadow-lg text-xs text-surface-z6">
  <span class="font-semibold text-surface-z8">A</span>
  <span class="text-surface-z3">·</span>
  <a href="/option-b" class="hover:text-primary-z6 transition-colors">Option B →</a>
</div>

<!-- Full-screen layout -->
<div class="flex h-screen w-screen overflow-hidden bg-surface-z1 text-surface-z8">

  <!-- ══ SIDEBAR ══════════════════════════════════════════════════ -->
  <aside class="flex w-58 shrink-0 flex-col border-r border-surface-z3 bg-surface-z2">

    <!-- Workspace switcher -->
    <div class="relative shrink-0 border-b border-surface-z3">
      <button
        class="flex w-full items-center gap-2.5 px-4 py-3.5 hover:bg-surface-z3 transition-colors"
        onclick={() => wsOpen = !wsOpen}
      >
        <span class="flex h-6 w-6 shrink-0 items-center justify-center rounded bg-primary-z5 text-xs font-bold text-white">
          {workspace.name.charAt(0)}
        </span>
        <span class="flex-1 min-w-0 truncate text-left text-sm font-semibold text-surface-z8">{workspace.name}</span>
        <svg class="h-3.5 w-3.5 shrink-0 text-surface-z4 transition-transform {wsOpen ? 'rotate-180' : ''}"
             viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M4 6l4 4 4-4"/>
        </svg>
      </button>

      {#if wsOpen}
        <div class="absolute inset-x-0 top-full z-50 border-b border-surface-z3 bg-surface-z2 shadow-xl">
          {#each data.workspaces as ws}
            <button
              class="flex w-full items-center gap-2.5 px-4 py-2 hover:bg-surface-z3 transition-colors"
              onclick={() => { wsId = ws.id; wsOpen = false; expandedRepo = null; activeRepo = null; }}
            >
              <span class="flex h-5 w-5 shrink-0 items-center justify-center rounded bg-primary-z4 text-xs font-bold text-white">
                {ws.name.charAt(0)}
              </span>
              <span class="flex-1 min-w-0 truncate text-sm text-surface-z7">{ws.name}</span>
              <span class="text-xs text-surface-z4 capitalize shrink-0">{ws.type}</span>
              {#if ws.id === wsId}
                <svg class="h-3.5 w-3.5 text-primary-z6 shrink-0" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/>
                </svg>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Nav body -->
    <nav class="flex flex-1 flex-col gap-5 overflow-y-auto py-3">

      <!-- Repos section -->
      <div>
        <div class="flex items-center justify-between px-4 mb-1">
          <span class="text-xs font-semibold uppercase tracking-wider text-surface-z4">Repos</span>
          <button class="text-base text-surface-z4 hover:text-surface-z7 transition-colors leading-none" title="Add repo">+</button>
        </div>

        {#each wsRepos as repo}
          <div>
            <button
              class="flex w-full items-center gap-1.5 rounded-md mx-2 py-1.5 pl-2 pr-2 text-left transition-colors
                     {activeRepo === repo.id ? 'bg-primary-z1 text-primary-z7' : 'text-surface-z6 hover:bg-surface-z3 hover:text-surface-z8'}"
              onclick={() => expandedRepo === repo.id && activeRepo === repo.id ? expandedRepo = null : selectRepo(repo.id)}
            >
              <svg class="h-3 w-3 shrink-0 text-surface-z4 transition-transform {expandedRepo === repo.id ? 'rotate-90' : ''}"
                   viewBox="0 0 16 16" fill="currentColor">
                <path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 011.06 0l3.5 3.5a.75.75 0 010 1.06l-3.5 3.5a.75.75 0 01-1.06-1.06L9.19 8 6.22 5.03a.75.75 0 010-1.06z"/>
              </svg>
              <span class="flex-1 min-w-0 truncate text-sm font-medium">{repo.name}</span>
              {#if repo.status === 'indexing'}
                <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-warning-z6 animate-pulse"></span>
              {:else}
                <span class="shrink-0 text-xs tabular-nums text-surface-z4">{repo.sessions}</span>
              {/if}
            </button>

            {#if expandedRepo === repo.id}
              <div class="ml-7 mr-2 mt-0.5 mb-1">
                {#each subNav as nav}
                  <button
                    class="flex w-full items-center rounded px-2 py-1 text-left text-sm transition-colors
                           {activeRepo === repo.id && activeView === nav.key
                             ? 'text-primary-z6 font-medium'
                             : 'text-surface-z5 hover:text-surface-z8 hover:bg-surface-z3'}"
                    onclick={() => { activeRepo = repo.id; activeView = nav.key; }}
                  >
                    {nav.label}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Teams (org workspaces only) -->
      {#if wsTeams.length > 0}
        <div>
          <div class="px-4 mb-1">
            <span class="text-xs font-semibold uppercase tracking-wider text-surface-z4">Teams</span>
          </div>
          {#each wsTeams as team}
            <button class="flex w-full items-center gap-2 rounded-md mx-2 px-2 py-1.5 text-sm text-surface-z6 hover:bg-surface-z3 hover:text-surface-z8 transition-colors">
              <span class="flex h-4 w-4 shrink-0 items-center justify-center rounded bg-surface-z4/40 text-xs font-bold text-surface-z7">
                {team.name.charAt(0)}
              </span>
              <span class="flex-1 min-w-0 truncate">{team.name}</span>
              <span class="shrink-0 text-xs text-surface-z4">{team.memberCount}</span>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Libraries -->
      <div>
        <div class="flex items-center justify-between px-4 mb-1">
          <span class="text-xs font-semibold uppercase tracking-wider text-surface-z4">Libraries</span>
        </div>
        {#each data.libraries as lib}
          <button
            class="flex w-full items-center gap-2 rounded-md mx-2 px-2 py-1.5 text-sm transition-colors
                   {activeView === 'libraries' && activeRepo === lib.id
                     ? 'bg-primary-z1 text-primary-z7'
                     : 'text-surface-z6 hover:bg-surface-z3 hover:text-surface-z8'}"
            onclick={() => { activeRepo = lib.id; activeView = 'libraries'; }}
          >
            <span class="h-2 w-2 shrink-0 rounded-full
              {lib.category === 'ui' ? 'bg-primary-z5' : lib.category === 'auth' ? 'bg-success-z5' : 'bg-warning-z5'}">
            </span>
            <span class="flex-1 min-w-0 truncate">{lib.name}</span>
            <span class="shrink-0 text-xs tabular-nums text-surface-z4">{lib.sections}</span>
          </button>
        {/each}
      </div>

      <!-- Benchmarks -->
      <div>
        <button
          class="flex w-full items-center gap-2 rounded-md mx-2 px-2 py-1.5 text-sm text-surface-z5 hover:bg-surface-z3 hover:text-surface-z8 transition-colors"
          onclick={() => { activeRepo = null; activeView = 'home'; }}
        >
          Benchmarks
        </button>
      </div>
    </nav>

    <!-- User footer -->
    <div class="shrink-0 border-t border-surface-z3 px-3 py-3 flex items-center gap-2">
      <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary-z6 text-xs font-bold text-white">
        {data.user.initials}
      </div>
      <div class="min-w-0 flex-1">
        <p class="text-xs font-medium text-surface-z8 truncate">{data.user.name}</p>
        <p class="text-xs text-surface-z4 truncate">{data.user.email}</p>
      </div>
      <ThemeSwitcherToggle />
    </div>
  </aside>

  <!-- ══ CONTENT ══════════════════════════════════════════════════ -->
  <div class="flex flex-1 flex-col min-w-0 overflow-hidden">

    <!-- Breadcrumb bar -->
    <div class="shrink-0 flex items-center justify-between border-b border-surface-z3 px-6 py-3">
      <div class="flex items-center gap-1.5 text-sm text-surface-z5">
        <span>{workspace.name}</span>
        {#if currentRepo && activeView !== 'libraries'}
          <span class="text-surface-z3">/</span>
          <button class="font-medium text-surface-z7 hover:text-surface-z8 transition-colors"
                  onclick={() => activeView = 'sessions'}>{currentRepo.name}</button>
          <span class="text-surface-z3">/</span>
          <span class="text-surface-z8 capitalize">
            {activeView === 'context' ? 'Context Packs' : activeView}
          </span>
        {:else if activeView === 'libraries'}
          <span class="text-surface-z3">/</span>
          <span class="font-medium text-surface-z8">Libraries</span>
        {/if}
      </div>

      {#if currentRepo && activeView !== 'libraries'}
        <div class="flex items-center gap-3">
          {#if currentRepo.status === 'indexing'}
            <span class="flex items-center gap-1.5 text-xs text-warning-z6">
              <span class="h-1.5 w-1.5 rounded-full bg-warning-z6 animate-pulse"></span>
              Indexing…
            </span>
          {:else}
            <span class="text-xs text-surface-z4">{currentRepo.symbols.toLocaleString()} symbols</span>
            <span class="text-xs text-surface-z4">indexed {currentRepo.indexed}</span>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Main scrollable area -->
    <div class="flex-1 overflow-auto p-6">

      <!-- ── Sessions ─────────────────────────────────────────── -->
      {#if activeView === 'sessions' && currentRepo}
        <div class="space-y-5">
          <div class="grid grid-cols-4 gap-3">
            {#each [
              { label: 'Sessions',  value: viewSessions.length                                      },
              { label: 'Avg FTR',   value: Math.round(avgFtr * 100) + '%',   ftr: avgFtr             },
              { label: 'Turns',     value: viewSessions.reduce((a,s)=>a+s.turns,0).toLocaleString() },
              { label: 'Spend',     value: '$' + totalCost.toFixed(2)                               },
            ] as stat}
              <div class="rounded-lg border border-surface-z3 bg-surface-z2 p-4">
                <div class="text-2xl font-semibold {stat.ftr != null ? ftrText(stat.ftr) : 'text-surface-z8'}">
                  {stat.value}
                </div>
                <div class="mt-0.5 text-xs text-surface-z5">{stat.label}</div>
              </div>
            {/each}
          </div>

          <div class="overflow-hidden rounded-lg border border-surface-z3">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-surface-z3 bg-surface-z2">
                  <th class="px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">Task</th>
                  <th class="w-16 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">FTR</th>
                  <th class="w-14 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">Turns</th>
                  <th class="w-20 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">Duration</th>
                  <th class="w-16 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">Cost</th>
                  <th class="w-20 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">When</th>
                </tr>
              </thead>
              <tbody>
                {#each viewSessions as s}
                  <tr class="cursor-pointer border-b border-surface-z2 last:border-0 hover:bg-surface-z2 transition-colors">
                    <td class="px-4 py-3">
                      <span class="font-medium text-surface-z8">{s.task}</span>
                      <span class="ml-2 text-xs text-surface-z4">{s.model}</span>
                    </td>
                    <td class="px-4 py-3">
                      <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-semibold {ftrBadge(s.ftr)}">
                        {Math.round(s.ftr * 100)}%
                      </span>
                    </td>
                    <td class="px-4 py-3 tabular-nums text-surface-z6">{s.turns}</td>
                    <td class="px-4 py-3 text-surface-z6">{s.duration}</td>
                    <td class="px-4 py-3 tabular-nums text-surface-z6">${s.cost.toFixed(2)}</td>
                    <td class="px-4 py-3 text-surface-z4">{s.when}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

      <!-- ── Analytics ─────────────────────────────────────────── -->
      {:else if activeView === 'analytics' && currentRepo}
        <div class="space-y-5">
          <div class="grid grid-cols-3 gap-4">
            {#each [
              { label: 'Sessions this week', value: viewSessions.length, sub: '+3 from last week'   },
              { label: 'Avg FTR',            value: Math.round(avgFtr * 100) + '%', sub: 'First-try-right' },
              { label: 'Total spend',        value: '$' + totalCost.toFixed(2), sub: 'Claude API cost' },
            ] as stat}
              <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
                <div class="mb-1 text-xs text-surface-z5">{stat.label}</div>
                <div class="text-3xl font-semibold text-surface-z8">{stat.value}</div>
                <div class="mt-1 text-xs text-surface-z4">{stat.sub}</div>
              </div>
            {/each}
          </div>

          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
            <h3 class="mb-4 text-sm font-semibold text-surface-z7">Model usage</h3>
            <div class="space-y-3">
              {#each [
                { model: 'claude-sonnet-4-6', pct: 85, cls: 'bg-primary-z5' },
                { model: 'claude-haiku-4-5',  pct: 10, cls: 'bg-surface-z5' },
                { model: 'claude-opus-4-6',   pct: 5,  cls: 'bg-warning-z5' },
              ] as m}
                <div class="flex items-center gap-3">
                  <span class="w-40 shrink-0 text-xs text-surface-z6">{m.model}</span>
                  <div class="h-2 flex-1 overflow-hidden rounded-full bg-surface-z3">
                    <div class="{m.cls} h-full rounded-full transition-all" style="width:{m.pct}%"></div>
                  </div>
                  <span class="w-8 shrink-0 text-right text-xs text-surface-z4">{m.pct}%</span>
                </div>
              {/each}
            </div>
          </div>
        </div>

      <!-- ── Context Packs ──────────────────────────────────────── -->
      {:else if activeView === 'context' && currentRepo}
        <div class="space-y-2">
          {#each ['Authentication flow', 'Database layer', 'MCP server tools', 'CLI commands', 'Test helpers'] as pack, i}
            <div class="flex items-center gap-3 rounded-lg border border-surface-z3 bg-surface-z2 px-4 py-3 hover:border-primary-z4 transition-colors">
              <div class="flex-1">
                <span class="text-sm font-medium text-surface-z8">{pack}</span>
                <span class="ml-2 text-xs text-surface-z4">{3 + i} files · {12 + i * 4}k tokens</span>
              </div>
              <button class="text-xs text-primary-z6 hover:text-primary-z7 transition-colors">Copy</button>
            </div>
          {/each}
        </div>

      <!-- ── Agents ─────────────────────────────────────────────── -->
      {:else if activeView === 'agents' && currentRepo}
        <div class="space-y-2">
          {#each ['code-explorer', 'code-architect', 'code-reviewer', 'test-writer'] as agent}
            <div class="flex items-center gap-3 rounded-lg border border-surface-z3 bg-surface-z2 px-4 py-3">
              <span class="flex-1 text-sm font-medium text-surface-z8">{agent}</span>
              <span class="rounded-full border border-success-z3 bg-success-z1 px-2 py-0.5 text-xs text-success-z7">Active</span>
            </div>
          {/each}
        </div>

      <!-- ── Drift ──────────────────────────────────────────────── -->
      {:else if activeView === 'drift' && currentRepo}
        <div class="flex items-center justify-center rounded-lg border border-surface-z3 bg-surface-z2 p-12">
          <p class="text-sm text-surface-z5">No drift detected — all docs are in sync.</p>
        </div>

      <!-- ── Libraries ─────────────────────────────────────────── -->
      {:else if activeView === 'libraries'}
        <div class="space-y-2">
          {#each data.libraries as lib}
            <div class="flex items-center gap-4 rounded-lg border border-surface-z3 bg-surface-z2 px-4 py-4 hover:border-primary-z4 transition-colors cursor-pointer">
              <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-surface-z3 text-sm font-bold text-surface-z7">
                {lib.name.charAt(0)}
              </span>
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium text-surface-z8">{lib.name}</p>
                <p class="text-xs text-surface-z4">{lib.sections} sections · {lib.documents} documents</p>
              </div>
              <span class="rounded bg-surface-z3 px-2 py-0.5 text-xs capitalize text-surface-z6">{lib.category}</span>
            </div>
          {/each}
        </div>

      <!-- ── Home / overview ────────────────────────────────────── -->
      {:else}
        <div class="space-y-5">
          <div class="grid grid-cols-3 gap-3">
            {#each [
              { label: 'Repos',    value: wsRepos.length },
              { label: 'Sessions', value: data.sessions.filter(s => wsRepos.some(r => r.id === s.repoId)).length },
              { label: 'Teams',    value: wsTeams.length },
            ] as stat}
              <div class="rounded-lg border border-surface-z3 bg-surface-z2 p-4">
                <div class="text-2xl font-semibold text-surface-z8">{stat.value}</div>
                <div class="mt-0.5 text-xs text-surface-z5">{stat.label}</div>
              </div>
            {/each}
          </div>

          <div>
            <h2 class="mb-3 text-sm font-semibold text-surface-z6">Recent sessions</h2>
            <div class="space-y-1.5">
              {#each data.sessions.slice(0, 7) as s}
                {@const repo = data.repos.find(r => r.id === s.repoId)}
                {#if repo && wsRepos.some(r => r.id === repo.id)}
                  <button
                    class="flex w-full items-center gap-3 rounded-lg border border-surface-z3 bg-surface-z2 px-4 py-2.5 text-left hover:border-primary-z4 transition-colors"
                    onclick={() => selectRepo(repo.id)}
                  >
                    <span class="shrink-0 rounded bg-surface-z3 px-1.5 py-0.5 font-mono text-xs text-surface-z5">{repo.name}</span>
                    <span class="flex-1 min-w-0 truncate text-sm text-surface-z8">{s.task}</span>
                    <span class="shrink-0 text-xs text-surface-z4">{s.when}</span>
                    <span class="shrink-0 inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-semibold {ftrBadge(s.ftr)}">
                      {Math.round(s.ftr * 100)}%
                    </span>
                  </button>
                {/if}
              {/each}
            </div>
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>
