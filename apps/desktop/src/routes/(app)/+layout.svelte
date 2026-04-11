<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { migrate } from '$lib/migration.js';
  import { loadSolutions, getSolutions, getSolutionsByCategory, getStandaloneLibraries } from '$lib/solutions.svelte.js';
  import ServerStatus from '$lib/ServerStatus.svelte';
  import SidebarSolution from '$lib/SidebarSolution.svelte';

  let { children } = $props();

  const DEFAULT_PORT = 7744;
  let senseiPort = $state(DEFAULT_PORT);

  // Expanded solution IDs in sidebar
  let expandedIds = $state<Set<string>>(new Set());

  function toggleExpanded(id: string) {
    const next = new Set(expandedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    expandedIds = next;
  }

  // Derived solution lists
  let activeSolutions = $derived(getSolutionsByCategory('active'));
  let sideSolutions = $derived(getSolutionsByCategory('side'));
  let ideaSolutions = $derived(getSolutionsByCategory('idea'));
  let standaloneLibs = $derived(getStandaloneLibraries());

  // Auto-expand the solution that matches the current route
  $effect(() => {
    const match = $page.url.pathname.match(/^\/s\/([^/]+)/);
    if (match) {
      const id = match[1];
      if (!expandedIds.has(id)) {
        expandedIds = new Set([...expandedIds, id]);
      }
    }
  });

  onMount(() => {
    const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
    if (!isNaN(stored) && stored > 0) senseiPort = stored;

    // Run migration + load solutions
    migrate();
    loadSolutions();
  });
</script>

<div class="flex h-screen overflow-hidden select-none bg-surface-z1">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════════ -->
  <aside class="flex w-52 shrink-0 flex-col border-r border-surface-z0/50 sidebar-vibrancy">

    <!-- Traffic light area + logo -->
    <div class="drag-region flex items-end px-4 pb-3 pt-9">
      <div class="no-drag flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-lg bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-0.5 px-2 py-2 overflow-y-auto">

      <!-- Active Solutions -->
      {#if activeSolutions.length > 0}
        <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Active</p>
        {#each activeSolutions as s (s.id)}
          <SidebarSolution
            solution={s}
            expanded={expandedIds.has(s.id)}
            onToggle={() => toggleExpanded(s.id)}
          />
        {/each}
      {/if}

      <!-- Standalone Libraries -->
      {#if standaloneLibs.length > 0}
        <div class="pt-3">
          <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Libraries</p>
          {#each standaloneLibs as s (s.id)}
            <a
              href="/s/{s.id}"
              class="flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
                     {$page.url.pathname.startsWith(`/s/${s.id}`)
                       ? 'bg-info-z2 text-info-z7 font-medium'
                       : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
            >
              <span class="text-xs i-solar-box-bold-duotone"></span>
              <span class="truncate">{s.name}</span>
            </a>
          {/each}
        </div>
      {/if}

      <!-- Side Projects -->
      {#if sideSolutions.length > 0}
        <div class="pt-3">
          <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Side Projects</p>
          {#each sideSolutions as s (s.id)}
            <SidebarSolution
              solution={s}
              expanded={expandedIds.has(s.id)}
              onToggle={() => toggleExpanded(s.id)}
            />
          {/each}
        </div>
      {/if}

      <!-- Ideas -->
      {#if ideaSolutions.length > 0}
        <div class="pt-3">
          <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Ideas</p>
          {#each ideaSolutions as s (s.id)}
            <a
              href="/s/{s.id}"
              class="flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-sm no-drag transition-colors
                     {$page.url.pathname.startsWith(`/s/${s.id}`)
                       ? 'bg-surface-z3 text-surface-z7 font-medium'
                       : 'text-surface-z4 hover:bg-surface-z3/40 hover:text-surface-z6'}"
            >
              <span class="text-xs i-solar-lightbulb-bold-duotone"></span>
              <span class="truncate">{s.name}</span>
            </a>
          {/each}
        </div>
      {/if}

      <!-- Separator + global links -->
      <div class="pt-4 border-t border-surface-z0/30 mt-3">
        <a href="/all"
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm no-drag transition-colors
                 {($page.url.pathname as string) === '/all' ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}">
          <span class="text-base i-solar-layers-bold-duotone"></span>
          All Repos
        </a>
        <a href="/acp"
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm no-drag transition-colors
                 {$page.url.pathname.startsWith('/acp') ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}">
          <span class="text-base i-solar-cpu-bold-duotone"></span>
          ACP Registry
        </a>
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
