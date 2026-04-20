<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { loadSolutions, getSolutions, getSolutionsByCategory, getStandaloneLibraries, markLoaded, isSolutionsLoaded } from '$lib/solutions.svelte.js';
  import { loadAppState, getPort, getSidebarMaxItems, isAppLoaded } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ServerProject } from '$lib/types.js';
  import ServerStatus from '$lib/ServerStatus.svelte';
  import SidebarSolution from '$lib/SidebarSolution.svelte';

  let { children } = $props();

  let senseiPort = $derived(getPort());
  let MAX_VISIBLE = $derived(getSidebarMaxItems());
  let showAll = $state(false);

  // Unified recent items: solutions + projects mixed, sorted by recency
  type SidebarItem = { kind: 'solution'; id: string; name: string; count: number; updatedAt: string }
                   | { kind: 'project'; id: string; name: string; updatedAt: string };
  let recentItems = $state<SidebarItem[]>([]);
  let visibleItems = $derived(showAll ? recentItems : recentItems.slice(0, MAX_VISIBLE));
  let hiddenCount = $derived(Math.max(0, recentItems.length - MAX_VISIBLE));

  onMount(async () => {
    await loadAppState();
    await loadSolutions();
    markLoaded();

    // Build unified recent list: solutions + standalone projects
    const api = senseiApi(getPort());
    const projects = await api.getProjects();
    const solutions = getSolutions();
    const solutionRepoIds = new Set(solutions.flatMap(s => s.repos.map(r => r.repoId)));

    const items: SidebarItem[] = [];

    // Solutions always first (they're curated groupings)
    for (const s of solutions) {
      items.push({ kind: 'solution', id: s.id, name: s.name, count: s.repos.length, updatedAt: s.updatedAt ?? new Date().toISOString() });
    }

    // Then recent indexed projects not in any solution
    const recentProjects = projects
      .filter(p => p.indexed_at && !solutionRepoIds.has(p.repo_id))
      .sort((a, b) => (b.indexed_at ?? '').localeCompare(a.indexed_at ?? ''));

    for (const p of recentProjects) {
      items.push({ kind: 'project', id: p.repo_id, name: p.name, updatedAt: p.indexed_at ?? '' });
    }

    recentItems = items;
  });
</script>

<div class="flex h-screen overflow-hidden select-none bg-surface-z1">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════════ -->
  <aside class="flex w-48 shrink-0 flex-col border-r border-surface-z0/50 sidebar-vibrancy">

    <!-- Traffic light area + logo -->
    <div class="drag-region flex items-end px-4 pb-3 pt-9">
      <div class="no-drag flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-lg bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-0.5 px-2 py-2 overflow-y-auto">

      <!-- Recent (solutions + projects, mixed by recency) -->
      {#if recentItems.length > 0}
        <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Recent</p>
        {#each visibleItems as item (item.id)}
          {#if item.kind === 'solution'}
            <a
              href="/s/{item.id}"
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
                     {$page.url.pathname.startsWith(`/s/${item.id}`) ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
            >
              <span class="flex h-5 w-5 items-center justify-center rounded-md bg-primary-z3 text-[10px] font-bold text-primary-z7">
                {item.name.charAt(0).toUpperCase()}
              </span>
              <span class="truncate flex-1">{item.name}</span>
              <span class="text-[10px] text-surface-z3">{item.count}</span>
            </a>
          {:else}
            <a
              href="/p/{item.id}"
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
                     {$page.url.pathname === `/p/${item.id}` ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
            >
              <span class="flex h-5 w-5 items-center justify-center rounded-md bg-surface-z3 text-[10px] font-bold text-surface-z6">
                {item.name.charAt(0).toUpperCase()}
              </span>
              <span class="truncate flex-1">{item.name}</span>
            </a>
          {/if}
        {/each}
        {#if !showAll && hiddenCount > 0}
          <button
            onclick={() => showAll = true}
            class="flex w-full items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10px] text-surface-z4 hover:text-surface-z6 no-drag transition-colors"
          >
            +{hiddenCount} more
          </button>
        {:else if showAll && hiddenCount > 0}
          <button
            onclick={() => showAll = false}
            class="flex w-full items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10px] text-surface-z4 hover:text-surface-z6 no-drag transition-colors"
          >
            show less
          </button>
        {/if}
      {/if}

      <!-- Global navigation -->
      <div class="pt-4 border-t border-surface-z0/30 mt-3">
        <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Global</p>
        {#each [
          { href: '/overview', icon: 'i-solar-home-2-bold-duotone', label: 'Overview' },
          { href: '/libraries', icon: 'i-solar-book-2-bold-duotone', label: 'Libraries' },
          { href: '/tools', icon: 'i-solar-widget-5-bold-duotone', label: 'Tools' },
          { href: '/skills', icon: 'i-solar-star-bold-duotone', label: 'Skills & Plugins' },
          { href: '/benchmarks', icon: 'i-solar-chart-2-bold-duotone', label: 'Benchmarks' },
        ] as item (item.href)}
          <a href={item.href}
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
                   {$page.url.pathname.startsWith(item.href) ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}">
            <span class="text-base {item.icon}"></span>
            {item.label}
          </a>
        {/each}
      </div>
    </nav>

    <!-- Bottom -->
    <div class="border-t border-surface-z0/50 px-3 py-2.5 space-y-1">
      <ServerStatus bind:port={senseiPort} />
      <a
        href="/settings"
        class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 no-drag transition-colors hover:bg-surface-z3/60 hover:text-surface-z7"
      >
        <span class="text-base i-solar-settings-minimalistic-bold-duotone"></span>
        Settings
      </a>
    </div>
  </aside>

  <!-- ══ MAIN ═════════════════════════════════════════════════════════ -->
  <div class="flex min-w-0 flex-1 flex-col bg-surface-z1 overflow-hidden">
    <div class="drag-region h-7 shrink-0 border-b border-surface-z0/30"></div>
    <main class="flex-1 overflow-hidden min-h-0">
      {@render children()}
    </main>
  </div>

</div>
