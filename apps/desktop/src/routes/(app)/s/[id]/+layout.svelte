<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getSolutionById, isSolutionsLoaded } from '$lib/solutions.svelte.js';
  import type { Solution } from '$lib/types.js';

  let { children } = $props();
  let solution = $state<Solution | undefined>(undefined);

  const tabs = $derived(solution ? [
    { label: 'Overview',      href: `/s/${solution.id}`,           active: $page.url.pathname === `/s/${solution.id}` },
    { label: 'Repos',         href: `/s/${solution.id}/repos`,    active: $page.url.pathname === `/s/${solution.id}/repos` },
    { label: 'Architecture',  href: `/s/${solution.id}/arch`,     active: $page.url.pathname === `/s/${solution.id}/arch` },
    { label: 'Traceability',  href: `/s/${solution.id}/trace`,    active: $page.url.pathname === `/s/${solution.id}/trace` },
    { label: 'Sessions',      href: `/s/${solution.id}/sessions`, active: $page.url.pathname === `/s/${solution.id}/sessions` },
    { label: 'Skills',        href: `/s/${solution.id}/skills`,   active: $page.url.pathname === `/s/${solution.id}/skills` },
  ] : []);

  $effect(() => {
    const id = $page.params.id as string;
    solution = getSolutionById(id);
    // Only redirect if solutions are fully loaded and this ID doesn't exist
    if (!solution && isSolutionsLoaded()) {
      goto('/all', { replaceState: true });
    }
  });
</script>

{#if solution}
  <div class="flex h-full flex-col min-h-0">
    <!-- Sub-navigation tabs -->
    <div class="flex items-center gap-1 border-b border-surface-z0/50 px-4 py-1.5 shrink-0">
      <h1 class="text-sm font-semibold text-surface-z8 mr-4 truncate">{solution.name}</h1>
      {#each tabs as tab}
        <a
          href={tab.href}
          class="rounded-md px-3 py-1 text-xs font-medium transition-colors
                 {tab.active
                   ? 'bg-primary-z2 text-primary-z7'
                   : 'text-surface-z4 hover:bg-surface-z3/60 hover:text-surface-z6'}"
        >
          {tab.label}
        </a>
      {/each}
    </div>
    <div class="flex-1 overflow-hidden min-h-0">
      {@render children()}
    </div>
  </div>
{:else if !isSolutionsLoaded()}
  <div class="flex items-center justify-center h-full">
    <p class="text-sm text-surface-z4">Loading...</p>
  </div>
{/if}
