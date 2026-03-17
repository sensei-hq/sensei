<script lang="ts">
  import { Table, SearchFilter } from '@rokkit/ui';
  import type { PageData } from './$types';

  interface FilterObject {
    column?: string;
    operator: string;
    value: string | number | RegExp;
  }

  const { data } = $props();

  let filters = $state<FilterObject[]>([]);

  const columns = [
    { name: 'name',       label: 'Name',  sortable: true },
    { name: 'kind',       label: 'Kind',  sortable: true },
    { name: 'file_path',  label: 'File',  sortable: true },
    { name: 'line_start', label: 'Line',  sortable: true },
  ];

  function applyFilters(rows: typeof data.symbols, activeFilters: FilterObject[]) {
    if (!activeFilters.length) return rows;
    return rows.filter(row => {
      return activeFilters.every(f => {
        const target = f.column
          ? String((row as Record<string, unknown>)[f.column] ?? '')
          : `${row.name} ${row.file_path} ${row.kind}`;
        return target.toLowerCase().includes(String(f.value).toLowerCase());
      });
    });
  }

  const filtered = $derived(applyFilters(data.symbols, filters));

  const navSections = $derived([
    { href: `/repos/${data.repo.id}/context-packs`, label: 'Context Packs', desc: 'Curated code contexts' },
    { href: `/repos/${data.repo.id}/sessions`,      label: 'Sessions',       desc: 'Agent session history' },
    { href: `/repos/${data.repo.id}/analytics`,     label: 'Analytics',      desc: 'Usage & metrics' },
    { href: `/repos/${data.repo.id}/agents`,         label: 'Agent Skills',   desc: 'Skill definitions' },
    {
      href: `/repos/${data.repo.id}/libraries`,
      label: 'Library Docs',
      desc: data.libAttentionCount > 0 ? `${data.libAttentionCount} need attention` : 'Indexed library docs',
      attention: data.libAttentionCount > 0,
    },
    { href: `/repos/${data.repo.id}/simulate`, label: 'Simulate', desc: 'Test library queries as Claude sees them' },
    { href: `/repos/${data.repo.id}/drift`, label: 'Doc Drift', desc: 'Docs out of sync with code' },
  ]);
</script>

<div class="mb-6">
  <a href="/repos" class="text-sm text-surface-z5 hover:text-surface-z7 transition-colors">← All Repos</a>
</div>

<div class="flex items-start justify-between gap-4 mb-6">
  <div>
    <h1 class="text-2xl font-semibold text-surface-z8 mb-1">{data.repo.name}</h1>
    {#if data.repo.local_path}
      <p class="text-xs text-surface-z4 font-mono">{data.repo.local_path}</p>
    {/if}
  </div>
  {#if (data.repo.stack ?? []).length > 0}
    <div class="flex flex-wrap gap-1 justify-end">
      {#each (data.repo.stack ?? []) as tag}
        <span class="text-xs px-2 py-0.5 rounded-full bg-surface-z2 border border-surface-z3 text-surface-z6">{tag}</span>
      {/each}
    </div>
  {/if}
</div>

<!-- Stats row -->
<div class="grid gap-3 mb-8" style="grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); max-width: 480px">
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
    <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.symbols.length}</div>
    <div class="text-xs uppercase tracking-wider text-surface-z5">Symbols</div>
  </div>
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
    <div class="text-lg font-semibold text-primary-z6 leading-tight mb-1">
      {data.repo.last_indexed_at
        ? new Date(data.repo.last_indexed_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
        : '—'}
    </div>
    <div class="text-xs uppercase tracking-wider text-surface-z5">Last Indexed</div>
  </div>
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
    <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.libAttentionCount > 0 ? data.libAttentionCount : '✓'}</div>
    <div class="text-xs uppercase tracking-wider text-surface-z5">{data.libAttentionCount > 0 ? 'Libs stale' : 'Libs ok'}</div>
  </div>
</div>

<!-- Navigation sections -->
<h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Sections</h2>
<div class="grid gap-3 mb-10" style="grid-template-columns: repeat(auto-fill, minmax(160px, 1fr))">
  {#each navSections as section}
    <a
      href={section.href}
      class="block p-3.5 rounded-lg border no-underline transition-colors {section.attention
        ? 'border-warning-z4 bg-surface-z1 hover:bg-surface-z2'
        : 'border-surface-z3 bg-surface-z1 hover:border-primary-z5 hover:bg-surface-z2'}"
    >
      <div class="font-semibold text-sm text-surface-z8 mb-0.5">{section.label}</div>
      <div class="text-xs text-surface-z5">{section.desc}</div>
    </a>
  {/each}
</div>

<!-- Symbols table -->
<h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">
  Exported Symbols ({filtered.length}{filtered.length !== data.symbols.length ? ` of ${data.symbols.length}` : ''})
</h2>

<div class="mb-3">
  <SearchFilter bind:filters placeholder="Filter by name, kind, or file..." />
</div>

<Table data={filtered} {columns} />
