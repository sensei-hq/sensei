<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { enhance } from '$app/forms';
  import type { PageData, ActionData } from './$types';
  const { data, form }: { data: PageData; form: ActionData } = $props();

  function formatDate(iso: string | null) {
    if (!iso) return '—';
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }
  function timeAgo(iso: string) {
    const s = (Date.now() - new Date(iso).getTime()) / 1000;
    if (s < 60) return 'just now';
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    return formatDate(iso);
  }

  const CATEGORIES = ['ui','auth','api','data','test','build','other'] as const;

  let search = $state('');
  let sidebarMode = $state<'simulate' | 'edit' | null>(null);

  type SimResult = { id: string; title: string; url: string | null; description: string; component: string | null };
  let simResults = $state<SimResult[]>([]);
  let lastQuery = $state('');

  $effect(() => {
    const f = form as any;
    if (f?.results) {
      simResults = f.results;
      lastQuery = f.query ?? '';
    }
    if (f?.edited) sidebarMode = null;
  });

  const filteredSections = $derived(
    search.trim()
      ? data.sections.filter(s =>
          s.title.toLowerCase().includes(search.toLowerCase()) ||
          s.description.toLowerCase().includes(search.toLowerCase())
        )
      : data.sections
  );

  const components = $derived(
    [...new Set(data.sections.map(s => s.component ?? 'General'))].sort()
  );

  $effect(() => {
    const busy = data.lib.index_status === 'indexing' || data.lib.embed_status === 'embedding';
    if (!busy) return;
    const id = setInterval(() => invalidateAll(), 3000);
    return () => clearInterval(id);
  });
</script>

<!-- Sidebar overlay -->
{#if sidebarMode !== null}
  <div
    class="fixed inset-0 bg-black/40 z-40"
    role="presentation"
    onclick={() => sidebarMode = null}
  ></div>
{/if}

<!-- Edit Sidebar -->
{#if sidebarMode === 'edit'}
  <div class="fixed right-0 top-0 h-full w-96 bg-surface-z1 border-l border-surface-z3 z-50 flex flex-col shadow-xl overflow-y-auto">
    <div class="flex items-center justify-between px-5 py-4 border-b border-surface-z3 sticky top-0 bg-surface-z1">
      <h2 class="text-sm font-semibold text-surface-z8">Edit Library</h2>
      <button
        onclick={() => sidebarMode = null}
        class="text-surface-z5 hover:text-surface-z8 transition-colors"
        aria-label="Close"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18">
          <path d="M18 6L6 18M6 6l12 12" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <form method="POST" action="?/edit" use:enhance class="flex flex-col gap-4 px-5 py-5 flex-1">
      {#if (form as any)?.error}
        <p class="text-xs text-error-z6">{(form as any).error}</p>
      {/if}
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Doc URL or path <span class="text-error-z6">*</span></span>
        <input
          type="text"
          name="url"
          value={data.lib.base_url ?? data.lib.local_path ?? ''}
          required
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        />
        <span class="text-xs text-surface-z4">Append <code>/llms.txt</code> for llms.txt source type</span>
      </label>
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Icon URL <span class="text-surface-z4">(optional)</span></span>
        <input
          type="url"
          name="icon_url"
          value={data.lib.icon_url ?? ''}
          placeholder="https://example.com/icon.svg"
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        />
      </label>
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Category <span class="text-surface-z4">(optional — auto-derived if blank)</span></span>
        <select
          name="category"
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        >
          <option value="">Auto-detect</option>
          {#each CATEGORIES as cat}
            <option value={cat} selected={data.lib.category === cat}>{cat.charAt(0).toUpperCase() + cat.slice(1)}</option>
          {/each}
        </select>
      </label>
      <p class="text-xs text-surface-z4">
        Changing the URL clears existing sections and re-indexes automatically. Query history is preserved.
      </p>
      <div class="flex gap-2 mt-auto">
        <button
          type="submit"
          class="flex-1 px-4 py-2 rounded bg-primary-z6 text-white text-sm font-medium hover:bg-primary-z7 transition-colors"
        >
          Save & Re-index
        </button>
        <button
          type="button"
          onclick={() => sidebarMode = null}
          class="px-4 py-2 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 text-sm hover:border-primary-z5 transition-colors"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
{/if}

<!-- Simulate Sidebar -->
{#if sidebarMode === 'simulate'}
  <div class="fixed right-0 top-0 h-full w-96 bg-surface-z1 border-l border-surface-z3 z-50 flex flex-col shadow-xl overflow-y-auto">
    <div class="flex items-center justify-between px-5 py-4 border-b border-surface-z3 sticky top-0 bg-surface-z1">
      <h2 class="text-sm font-semibold text-surface-z8">Simulate Request</h2>
      <button
        onclick={() => sidebarMode = null}
        class="text-surface-z5 hover:text-surface-z8 transition-colors"
        aria-label="Close"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18">
          <path d="M18 6L6 18M6 6l12 12" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <div class="flex flex-col gap-5 px-5 py-5">
      <form method="POST" action="?/simulate" use:enhance class="flex flex-col gap-3">
        <label class="flex flex-col gap-1.5">
          <span class="text-xs font-medium text-surface-z6">Query</span>
          <textarea
            name="query"
            rows="3"
            placeholder="e.g. How do I create a button?"
            class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none resize-none"
          ></textarea>
        </label>
        <button
          type="submit"
          class="px-4 py-2 rounded bg-primary-z6 text-white text-sm font-medium hover:bg-primary-z7 transition-colors"
        >
          Run
        </button>
      </form>

      {#if simResults.length > 0}
        <div>
          <p class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">
            {simResults.length} sections matched "{lastQuery}"
          </p>
          <div class="flex flex-col gap-3">
            {#each simResults as r, i}
              <div class="rounded-lg border border-surface-z3 bg-surface-z2 p-3">
                <div class="flex items-start gap-2 mb-1">
                  <span class="text-xs text-surface-z4 font-mono mt-0.5">#{i + 1}</span>
                  <div class="flex-1 min-w-0">
                    {#if r.url}
                      <a href={r.url} target="_blank" rel="noopener noreferrer" class="text-sm font-medium text-primary-z6 hover:text-primary-z7 leading-snug transition-colors">
                        {r.title}
                      </a>
                    {:else}
                      <p class="text-sm font-medium text-surface-z8 leading-snug">{r.title}</p>
                    {/if}
                    {#if r.component}
                      <span class="text-xs text-surface-z4">{r.component}</span>
                    {/if}
                  </div>
                </div>
                <p class="text-xs text-surface-z6 leading-relaxed line-clamp-3">{r.description}</p>
              </div>
            {/each}
          </div>
        </div>
      {:else if lastQuery}
        <p class="text-sm text-surface-z5">No sections matched "{lastQuery}"</p>
      {/if}

      {#if data.queries.length > 0}
        <div>
          <p class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Recent Queries</p>
          <div class="flex flex-col gap-1">
            {#each data.queries as q}
              <div class="flex items-center gap-2 py-1.5 border-b border-surface-z2 last:border-b-0">
                <p class="flex-1 text-xs text-surface-z7 truncate">{q.query_text}</p>
                <span class="text-xs text-surface-z4 shrink-0">{q.sections_hit} hits</span>
                <span class="text-xs text-surface-z3 shrink-0">{timeAgo(q.created_at)}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- Main content -->
<div class="mb-6">
  <a href="/libraries" class="text-sm text-surface-z5 hover:text-surface-z7 transition-colors">← Shared Libraries</a>
</div>

<div class="flex items-start justify-between mb-8">
  <div>
    <h1 class="text-2xl font-semibold text-surface-z8 mb-1">{data.lib.name}</h1>
    <p class="text-sm text-surface-z5">
      {data.lib.source_type}
      {#if data.lib.base_url}
        · <a href={data.lib.base_url} target="_blank" rel="noopener noreferrer" class="text-primary-z6 hover:text-primary-z7 transition-colors">{data.lib.base_url}</a>
      {:else if data.lib.local_path}
        · <span class="font-mono">{data.lib.local_path}</span>
      {/if}
    </p>
    {#if data.lib.index_status === 'error' && data.lib.index_error}
      <p class="text-xs text-error-z6 mt-1">{data.lib.index_error}</p>
    {/if}
  </div>
  <div class="flex items-center gap-2 shrink-0">
    <button
      onclick={() => sidebarMode = 'edit'}
      class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors"
    >
      Edit
    </button>

    {#if data.lib.source_type === 'local'}
      <span
        title="Re-index via CLI: sensei index"
        class="text-xs px-3 py-1.5 rounded border border-surface-z2 bg-surface-z1 text-surface-z4 cursor-not-allowed"
      >
        Re-index (CLI only)
      </span>
    {:else}
      <form method="POST" action="?/reindex">
        <button
          type="submit"
          disabled={data.lib.index_status === 'indexing'}
          class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {data.lib.index_status === 'indexing' ? 'Indexing…' : 'Re-index'}
        </button>
      </form>
    {/if}

    {#if data.lib.embed_status !== 'ready'}
      <form method="POST" action="?/embed">
        <button
          type="submit"
          disabled={data.lib.index_status === 'indexing' || data.lib.embed_status === 'embedding' || data.lib.section_count === 0}
          class="text-xs px-3 py-1.5 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          title="Generate embeddings for semantic search"
        >
          {data.lib.embed_status === 'embedding' ? 'Embedding…' : 'Build Index'}
        </button>
      </form>
    {:else}
      <span class="text-xs px-2 py-1 rounded bg-success-z2 text-success-z6 border border-success-z3">
        Indexed
      </span>
    {/if}

    <button
      onclick={() => sidebarMode = 'simulate'}
      class="text-xs px-3 py-1.5 rounded border border-primary-z4 bg-surface-z1 text-primary-z6 hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
    >
      Simulate
    </button>
  </div>
</div>

<!-- Stat row — uniform sizing for all four cards -->
<div class="grid gap-3 mb-8" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr))">
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1">
    {#if data.lib.index_status === 'indexing'}
      <svg class="animate-spin text-primary-z6 mb-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="24" height="24"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round"/></svg>
    {:else}
      <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.lib.section_count}</div>
    {/if}
    <div class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Sections</div>
  </div>
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1">
    <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.repos.length}</div>
    <div class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Repos</div>
  </div>
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1">
    <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.queries.length}</div>
    <div class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Queries</div>
  </div>
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1">
    <div class="text-sm font-semibold text-primary-z6 leading-tight mb-1">{formatDate(data.lib.indexed_at)}</div>
    <div class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Last Indexed</div>
  </div>
</div>

<!-- Linked repos -->
{#if data.repos.length > 0}
  <div class="mb-8">
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Used by</h2>
    <div class="flex flex-wrap gap-2">
      {#each data.repos as repo}
        <a
          href="/repos/{repo.id}"
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-surface-z7 text-sm hover:border-primary-z5 hover:bg-surface-z2 transition-colors"
        >
          {repo.name}
        </a>
      {/each}
    </div>
  </div>
{/if}

<!-- Sections table -->
<div class="mb-4 flex items-center justify-between">
  <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">
    Index Contents
    <span class="text-surface-z4 font-normal normal-case">
      ({filteredSections.length}{filteredSections.length !== data.sections.length ? ` of ${data.sections.length}` : ''} sections)
    </span>
  </h2>
  {#if data.sections.length > 0}
    <input
      type="search"
      placeholder="Filter sections…"
      bind:value={search}
      class="px-3 py-1.5 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-xs focus:border-primary-z5 focus:outline-none w-48"
    />
  {/if}
</div>

{#if data.lib.index_status === 'indexing'}
  <div class="rounded-lg border border-surface-z3 bg-surface-z1 p-8 text-center">
    <svg class="animate-spin mx-auto mb-3 text-primary-z6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="24" height="24"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round"/></svg>
    <p class="text-surface-z6 text-sm">Indexing in progress…</p>
    <p class="text-surface-z4 text-xs mt-1">This page will update automatically.</p>
  </div>
{:else if data.sections.length === 0}
  <div class="rounded-lg border border-surface-z3 bg-surface-z1 p-8 text-center">
    <p class="text-surface-z5 text-sm mb-1">No sections indexed yet</p>
    <p class="text-surface-z4 text-xs">Click Re-index to fetch docs.</p>
  </div>
{:else if filteredSections.length === 0}
  <p class="text-surface-z5 text-sm">No sections match your filter.</p>
{:else}
  <div class="rounded-lg border border-surface-z3 overflow-hidden">
    <table class="w-full border-collapse text-sm">
      <thead>
        <tr class="bg-surface-z2 border-b border-surface-z3">
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Title</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5">Component</th>
          <th class="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-surface-z5 max-w-xs">Description</th>
        </tr>
      </thead>
      <tbody>
        {#each filteredSections as section}
          <tr class="border-b border-surface-z2 last:border-b-0 hover:bg-surface-z2 transition-colors">
            <td class="px-4 py-3">
              {#if section.url}
                <a href={section.url} target="_blank" rel="noopener noreferrer" class="text-primary-z6 hover:text-primary-z7 transition-colors font-medium">
                  {section.title}
                </a>
              {:else}
                <span class="font-medium text-surface-z8">{section.title}</span>
              {/if}
            </td>
            <td class="px-4 py-3 text-surface-z5 text-xs">{section.component ?? '—'}</td>
            <td class="px-4 py-3 text-surface-z6 text-xs max-w-xs">
              <p class="line-clamp-2">{section.description}</p>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
