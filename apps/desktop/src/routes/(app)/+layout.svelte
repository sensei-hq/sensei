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
  const MAX_VISIBLE = 5;

  let showAllActive = $state(false);
  let showAllSide = $state(false);
  let showAllIdeas = $state(false);

  // Derived solution lists
  let activeSolutions = $derived(getSolutionsByCategory('active'));
  let sideSolutions = $derived(getSolutionsByCategory('side'));
  let ideaSolutions = $derived(getSolutionsByCategory('idea'));
  let standaloneLibs = $derived(getStandaloneLibraries());

  let visibleActive = $derived(showAllActive ? activeSolutions : activeSolutions.slice(0, MAX_VISIBLE));
  let hiddenActiveCount = $derived(Math.max(0, activeSolutions.length - MAX_VISIBLE));
  let visibleSide = $derived(showAllSide ? sideSolutions : sideSolutions.slice(0, MAX_VISIBLE));
  let hiddenSideCount = $derived(Math.max(0, sideSolutions.length - MAX_VISIBLE));
  let visibleIdeas = $derived(showAllIdeas ? ideaSolutions : ideaSolutions.slice(0, MAX_VISIBLE));
  let hiddenIdeaCount = $derived(Math.max(0, ideaSolutions.length - MAX_VISIBLE));

  onMount(() => {
    const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
    if (!isNaN(stored) && stored > 0) senseiPort = stored;
    migrate();
    loadSolutions();
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

      <!-- Active Solutions -->
      {#if activeSolutions.length > 0}
        <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Active</p>
        {#each visibleActive as s (s.id)}
          <SidebarSolution solution={s} />
        {/each}
        {#if !showAllActive && hiddenActiveCount > 0}
          <button
            onclick={() => showAllActive = true}
            class="flex w-full items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10px] text-surface-z4 hover:text-surface-z6 no-drag transition-colors"
          >
            +{hiddenActiveCount} more
          </button>
        {/if}
      {/if}

      <!-- Standalone Libraries -->
      {#if standaloneLibs.length > 0}
        <div class="pt-3">
          <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Libraries</p>
          {#each standaloneLibs as s (s.id)}
            <SidebarSolution solution={s} />
          {/each}
        </div>
      {/if}

      <!-- Side Projects -->
      {#if sideSolutions.length > 0}
        <div class="pt-3">
          <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Side Projects</p>
          {#each visibleSide as s (s.id)}
            <SidebarSolution solution={s} />
          {/each}
          {#if !showAllSide && hiddenSideCount > 0}
            <button
              onclick={() => showAllSide = true}
              class="flex w-full items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10px] text-surface-z4 hover:text-surface-z6 no-drag transition-colors"
            >
              +{hiddenSideCount} more
            </button>
          {/if}
        </div>
      {/if}

      <!-- Ideas -->
      {#if ideaSolutions.length > 0}
        <div class="pt-3">
          <p class="mb-1 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Ideas</p>
          {#each visibleIdeas as s (s.id)}
            <SidebarSolution solution={s} />
          {/each}
          {#if !showAllIdeas && hiddenIdeaCount > 0}
            <button
              onclick={() => showAllIdeas = true}
              class="flex w-full items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10px] text-surface-z4 hover:text-surface-z6 no-drag transition-colors"
            >
              +{hiddenIdeaCount} more
            </button>
          {/if}
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
