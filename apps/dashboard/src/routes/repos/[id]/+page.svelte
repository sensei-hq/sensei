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

  // Columns definition for Rokkit Table
  const columns = [
    { name: 'name',       label: 'Name',  sortable: true },
    { name: 'kind',       label: 'Kind',  sortable: true },
    { name: 'file_path',  label: 'File',  sortable: true },
    { name: 'line_start', label: 'Line',  sortable: true },
  ];

  /** Apply SearchFilter's parsed filters to local data */
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
</script>

<a href="/repos">← All Repos</a>
<h1>{data.repo.name}</h1>

<dl>
  <dt>Stack</dt><dd>{(data.repo.stack ?? []).join(', ') || '—'}</dd>
  <dt>Last indexed</dt>
  <dd>{data.repo.last_indexed_at ? new Date(data.repo.last_indexed_at).toLocaleString() : 'Never'}</dd>
  <dt>Traceability</dt><dd>0 links</dd>
</dl>

<h2>Symbols ({data.symbols.length})</h2>

<SearchFilter bind:filters placeholder="Filter by name or file..." />

<Table data={filtered} {columns} />

<p><a href="/repos/{data.repo.id}/context-packs">View Context Packs →</a></p>
<p><a href="/repos/{data.repo.id}/sessions">View Sessions →</a></p>
<p><a href="/repos/{data.repo.id}/analytics">View Analytics →</a></p>
<p><a href="/repos/{data.repo.id}/agents">Agent Skills →</a></p>
<p><a href="/repos/{data.repo.id}/libraries">Library Docs →</a></p>
