<!-- apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte -->
<script lang="ts">
  import type { PageData, ActionData } from './$types';
  import AddLibrarySidebar from '$lib/components/AddLibrarySidebar.svelte';

  const { data, form }: { data: PageData; form: ActionData } = $props();

  let addOpen = $state(false);
  let catalogSearch = $state('');

  const filteredCatalog = $derived(
    catalogSearch.trim()
      ? data.catalog.filter((l: any) => l.name.toLowerCase().includes(catalogSearch.toLowerCase()))
      : data.catalog
  );

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }

  const freshnessClass: Record<string, string> = {
    fresh: 'text-success-z6',
    stale: 'text-warning-z6',
    missing: 'text-error-z6',
  };
  const freshnessLabel: Record<string, string> = {
    fresh: 'Fresh', stale: 'Stale', missing: 'Not indexed',
  };
</script>

<div class="mb-6">
  <a href="/repos/{data.repo.id}" class="text-sm text-surface-z5 hover:text-surface-z7 transition-colors">← {data.repo.name}</a>
</div>

<div class="flex items-center justify-between mb-6">
  <h1 class="text-2xl font-semibold text-surface-z8">Library Docs</h1>
  <div class="flex items-center gap-2">
    <button
      onclick={() => addOpen = true}
      class="flex items-center gap-2 px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 text-sm hover:border-primary-z5 hover:text-surface-z8 transition-colors"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
        <path d="M12 5v14M5 12h14" stroke-linecap="round"/>
      </svg>
      Add Library
    </button>
    <form method="POST" action="?/update">
      <button type="submit" class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors">
        Re-index All
      </button>
    </form>
  </div>
</div>

{#if form?.error}
  <div class="mb-4 px-4 py-3 rounded-lg border border-error-z4 bg-error-z1 text-error-z7 text-sm">
    {form.error}
  </div>
{/if}

{#if data.libs.length > 0}
  <div class="rounded-lg border border-surface-z3 overflow-hidden mb-8">
    <table class="w-full border-collapse text-sm">
      <thead>
        <tr class="bg-surface-z2 border-b border-surface-z3">
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Library</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Source</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Sections</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Last Indexed</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Status</th>
          {#if data.hasAnthropicKey}
            <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Skill</th>
          {/if}
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each data.libs as lib}
          <tr class="border-b border-surface-z2 last:border-b-0">
            <td class="px-4 py-3">
              {#if lib.sharedLibId}
                <a href="/libraries/{lib.sharedLibId}" class="font-medium text-primary-z6 hover:text-primary-z7 transition-colors">
                  {lib.libName}
                </a>
              {:else}
                <span class="font-medium text-surface-z8">{lib.libName}</span>
              {/if}
            </td>
            <td class="px-4 py-3 text-surface-z5 text-xs font-mono">{lib.sourceType}</td>
            <td class="px-4 py-3 text-surface-z7">{lib.sectionCount}</td>
            <td class="px-4 py-3 text-surface-z5 text-xs">{formatDate(lib.lastFetched)}</td>
            <td class="px-4 py-3">
              <span class="text-xs {freshnessClass[lib.freshness] ?? ''}">
                {freshnessLabel[lib.freshness] ?? lib.freshness}
              </span>
            </td>
            {#if data.hasAnthropicKey}
              <td class="px-4 py-3 text-xs {lib.skillPath ? 'text-success-z6' : 'text-surface-z4'}">
                {lib.skillPath ? 'Generated' : 'None'}
              </td>
            {/if}
            <td class="px-4 py-3">
              <form method="POST" action="?/reindex">
                <input type="hidden" name="name" value={lib.libName} />
                <button type="submit" class="text-xs text-primary-z6 hover:text-primary-z7 transition-colors">
                  Re-index
                </button>
              </form>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else}
  <p class="text-surface-z5 mb-8">No library docs registered yet. Add one below.</p>
{/if}

<!-- Catalog: link existing shared libs to this repo -->
{#if data.catalog.length > 0}
  <div class="mt-8">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Library Catalog</h2>
      <input
        type="search"
        placeholder="Search catalog…"
        bind:value={catalogSearch}
        class="px-3 py-1.5 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-xs focus:border-primary-z5 focus:outline-none w-44"
      />
    </div>
    <div class="rounded-lg border border-surface-z3 overflow-hidden">
      <table class="w-full border-collapse text-sm">
        <tbody>
          {#each filteredCatalog as lib}
            <tr class="border-b border-surface-z2 last:border-b-0 hover:bg-surface-z2 transition-colors">
              <td class="px-4 py-3">
                <a href="/libraries/{lib.id}" class="font-medium text-primary-z6 hover:text-primary-z7 transition-colors">
                  {lib.name}
                </a>
              </td>
              <td class="px-4 py-3 text-surface-z5 text-xs">{lib.section_count} sections</td>
              <td class="px-4 py-3 text-right">
                {#if lib.linked}
                  <span class="text-xs text-success-z6">✓ Linked</span>
                {:else}
                  <form method="POST" action="?/link" class="inline">
                    <input type="hidden" name="shared_lib_id" value={lib.id} />
                    <button type="submit" class="text-xs px-2 py-1 rounded border border-surface-z3 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors">
                      Link
                    </button>
                  </form>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
{/if}

<AddLibrarySidebar open={addOpen} action="?/add" onclose={() => addOpen = false} />
