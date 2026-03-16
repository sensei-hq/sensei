<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import type { PageData, ActionData } from './$types';
  import AddLibrarySidebar from '$lib/components/AddLibrarySidebar.svelte';
  const { data, form }: { data: PageData; form: ActionData } = $props();

  let search = $state('');
  let viewMode = $state<'grid' | 'table'>('grid');
  let sidebarOpen = $state(false);

  const filtered = $derived(
    search.trim()
      ? data.libs.filter((lib: any) => lib.name?.toLowerCase().includes(search.toLowerCase()))
      : data.libs
  );

  // Poll every 3s while any lib is indexing
  $effect(() => {
    const hasIndexing = data.libs.some((l: any) => l.index_status === 'indexing');
    if (!hasIndexing) return;
    const id = setInterval(() => invalidateAll(), 3000);
    return () => clearInterval(id);
  });

  // Derive a stable color from the name
  const PALETTE = [
    '#6366f1', '#8b5cf6', '#ec4899', '#f59e0b',
    '#10b981', '#3b82f6', '#ef4444', '#14b8a6',
  ];
  function libColor(name: string): string {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
    return PALETTE[h % PALETTE.length];
  }

  type Category = 'ui' | 'auth' | 'api' | 'data' | 'test' | 'build' | 'other';
  function libCategory(name: string): Category {
    const n = name.toLowerCase();
    if (/ui|component|design|rokkit|svelte|react|vue|angular|tailwind|uno/.test(n)) return 'ui';
    if (/auth|kavach|jwt|oauth|session|identity|supabase/.test(n)) return 'auth';
    if (/api|client|sdk|fetch|axios|http|rest|graphql/.test(n)) return 'api';
    if (/db|database|orm|prisma|drizzle|mongo|postgres|sql/.test(n)) return 'data';
    if (/test|vitest|jest|cypress|playwright/.test(n)) return 'test';
    if (/vite|webpack|rollup|esbuild|bun|build|bundl/.test(n)) return 'build';
    return 'other';
  }

  const CATEGORY_LABEL: Record<Category, string> = {
    ui: 'UI', auth: 'Auth', api: 'API', data: 'Data',
    test: 'Testing', build: 'Build', other: 'Library',
  };

  const CATEGORY_ICON: Record<Category, string> = {
    ui:    'M4 5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5zm10 0a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1V5zM4 15a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-4zm10 0a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1v-4z',
    auth:  'M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm0 4a3 3 0 1 1 0 6 3 3 0 0 1 0-6zm0 14c-2 0-5-1.46-5-4.5v-.24c.87-.41 2.5-.76 5-.76s4.13.35 5 .76v.24c0 3.04-3 4.5-5 4.5z',
    api:   'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2z',
    data:  'M4 7c0-1.1 3.58-2 8-2s8 .9 8 2v2c0 1.1-3.58 2-8 2S4 10.1 4 9V7zm0 5v3c0 1.1 3.58 2 8 2s8-.9 8-2v-3c0 1.1-3.58 2-8 2s-8-.9-8-2z',
    test:  'M9 12l2 2 4-4m6 2a9 9 0 1 1-18 0 9 9 0 0 1 18 0z',
    build: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 0 0-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 0 0-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 0 0-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 0 0-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 0 0 1.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065zM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
    other: 'M20 7H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2zM4 9h16v6H4V9zm2 8h12v2H6v-2z',
  };

  function displayUrl(lib: any): string | null {
    if (lib.base_url) {
      if (lib.base_url.startsWith('file://')) return lib.base_url.replace('file://', '');
      try { return new URL(lib.base_url).hostname; } catch { return lib.base_url; }
    }
    return null;
  }

  function formatDate(iso: string | null) {
    if (!iso) return null;
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }

  // Close sidebar on successful add
  $effect(() => {
    if ((form as any)?.success) sidebarOpen = false;
  });

  function statusBadge(lib: any): { label: string; cls: string; spin?: boolean } {
    switch (lib.index_status) {
      case 'indexing': return { label: 'Indexing…', cls: 'text-primary-z6', spin: true };
      case 'error':    return { label: 'Error', cls: 'text-error-z6' };
      case 'ready':    return { label: '', cls: '' };
      default:         return { label: 'Pending', cls: 'text-surface-z4' };
    }
  }
</script>

<AddLibrarySidebar open={sidebarOpen} action="?/add" onclose={() => sidebarOpen = false} />

<!-- Header -->
<div class="flex items-center justify-between mb-6">
  <div>
    <h1 class="text-2xl font-semibold text-surface-z8">Shared Libraries</h1>
    <p class="text-sm text-surface-z5 mt-0.5">{data.libs.length} in pool</p>
  </div>
  <button
    onclick={() => sidebarOpen = true}
    class="flex items-center gap-2 px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 text-sm hover:border-primary-z5 hover:text-surface-z8 transition-colors"
  >
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
      <path d="M12 5v14M5 12h14" stroke-linecap="round"/>
    </svg>
    Add Library
  </button>
</div>

<!-- Toolbar -->
<div class="flex items-center gap-3 mb-5">
  <input
    type="search"
    placeholder="Search…"
    bind:value={search}
    class="px-3 py-1.5 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none max-w-xs flex-1"
  />
  <!-- View toggle -->
  <div class="flex items-center rounded border border-surface-z3 overflow-hidden">
    <button
      onclick={() => viewMode = 'grid'}
      class="px-2.5 py-1.5 transition-colors {viewMode === 'grid' ? 'bg-surface-z3 text-surface-z8' : 'bg-surface-z1 text-surface-z5 hover:text-surface-z7'}"
      title="Grid view"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
        <rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/>
        <rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
      </svg>
    </button>
    <button
      onclick={() => viewMode = 'table'}
      class="px-2.5 py-1.5 transition-colors {viewMode === 'table' ? 'bg-surface-z3 text-surface-z8' : 'bg-surface-z1 text-surface-z5 hover:text-surface-z7'}"
      title="Table view"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
        <path d="M3 6h18M3 12h18M3 18h18" stroke-linecap="round"/>
      </svg>
    </button>
  </div>
</div>

{#if filtered.length === 0}
  <p class="text-surface-z5">
    {search ? 'No libraries match your search.' : 'No shared libraries yet.'}
  </p>
{:else if viewMode === 'grid'}
  <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))">
    {#each filtered as lib}
      {@const category = libCategory(lib.name)}
      {@const color = libColor(lib.name)}
      {@const url = displayUrl(lib)}
      {@const date = formatDate(lib.indexed_at)}
      {@const repos = lib.repos as Array<{ id: string; name: string }>}
      {@const badge = statusBadge(lib)}
      <a
        href="/libraries/{lib.id}"
        class="flex items-start gap-3.5 p-4 rounded-lg border border-surface-z3 bg-surface-z1 no-underline hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
      >
        <div class="shrink-0 w-10 h-10 rounded-lg flex items-center justify-center" style="background: {color}22; color: {color}">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" width="20" height="20">
            <path d={CATEGORY_ICON[category]} />
          </svg>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 mb-0.5">
            <span class="font-semibold text-surface-z8">{lib.name}</span>
            <span class="text-xs px-1.5 py-0.5 rounded bg-surface-z3 text-surface-z5">{CATEGORY_LABEL[category]}</span>
          </div>
          {#if url}
            <p class="text-xs text-surface-z4 truncate mb-2">{url}</p>
          {/if}
          <div class="flex items-center gap-2 text-xs text-surface-z5 mb-2">
            {#if badge.label}
              <span class="{badge.cls} flex items-center gap-1">
                {#if badge.spin}
                  <svg class="animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round"/></svg>
                {/if}
                {badge.label}
              </span>
            {:else}
              <span>{lib.section_count ?? 0} sections</span>
              {#if date}
                <span class="text-surface-z3">·</span>
                <span>{date}</span>
              {/if}
            {/if}
            {#if lib.index_status === 'error' && lib.index_error}
              <span class="text-surface-z3">·</span>
              <span class="text-error-z5 truncate" title={lib.index_error}>see detail</span>
            {/if}
          </div>
          {#if repos.length > 0}
            <div class="flex flex-wrap gap-1">
              {#each repos as repo}
                <span class="text-xs px-1.5 py-0.5 rounded bg-surface-z2 border border-surface-z3 text-surface-z5">{repo.name}</span>
              {/each}
            </div>
          {:else}
            <span class="text-xs text-surface-z4">no repos linked</span>
          {/if}
        </div>
      </a>
    {/each}
  </div>
{:else}
  <!-- Table view -->
  <div class="rounded-lg border border-surface-z3 overflow-hidden">
    <table class="w-full border-collapse text-sm">
      <thead>
        <tr class="bg-surface-z2 border-b border-surface-z3">
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Library</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Source</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Sections</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Last Indexed</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Repos</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as lib}
          {@const repos = lib.repos as Array<{ id: string; name: string }>}
          {@const date = formatDate(lib.indexed_at)}
          <tr class="border-b border-surface-z2 last:border-b-0 hover:bg-surface-z2 transition-colors">
            <td class="px-4 py-3">
              <a href="/libraries/{lib.id}" class="font-medium text-primary-z6 hover:text-primary-z7 transition-colors">
                {lib.name}
              </a>
            </td>
            <td class="px-4 py-3 text-surface-z5 text-xs font-mono">{lib.source_type}</td>
            <td class="px-4 py-3 text-surface-z7">{lib.section_count ?? 0}</td>
            <td class="px-4 py-3 text-surface-z5 text-xs">
              {#if lib.index_status === 'indexing'}
                <span class="text-primary-z6 flex items-center gap-1">
                  <svg class="animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round"/></svg>
                  Indexing…
                </span>
              {:else if lib.index_status === 'error'}
                <span class="text-error-z6">Error</span>
              {:else if date}
                {date}
              {:else}
                <span class="text-surface-z4">pending</span>
              {/if}
            </td>
            <td class="px-4 py-3">
              {#if repos.length > 0}
                <div class="flex flex-wrap gap-1">
                  {#each repos as repo}
                    <a
                      href="/repos/{repo.id}"
                      class="text-xs px-1.5 py-0.5 rounded bg-surface-z2 border border-surface-z3 text-surface-z6 hover:border-primary-z5 transition-colors"
                    >
                      {repo.name}
                    </a>
                  {/each}
                </div>
              {:else}
                <span class="text-xs text-surface-z4">—</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
