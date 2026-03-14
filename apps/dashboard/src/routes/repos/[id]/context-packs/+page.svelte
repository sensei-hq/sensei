<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data } = $props();

  let expandedId = $state<string | null>(null);

  const toggle = (id: string) => { expandedId = expandedId === id ? null : id; };
  const fmt = (iso: string) => new Date(iso).toLocaleString();

  // Columns for Rokkit Table (matches existing dashboard Table usage)
  const sliceColumns = [
    { name: 'filePath',    label: 'File',           sortable: true },
    { name: 'lines',       label: 'Lines',          sortable: false },
    { name: 'kind',        label: 'Kind',           sortable: true },
    { name: 'label',       label: 'Symbol/Heading', sortable: false },
    { name: 'tokens',      label: 'Tokens',         sortable: true },
    { name: 'score',       label: 'Score',          sortable: true },
  ];

  function sliceRows(slices: any[]) {
    return slices.map(s => ({
      filePath: s.filePath,
      lines: `${s.startLine}–${s.endLine}`,
      kind: s.kind,
      label: s.kind === 'code' ? (s.symbolName ?? '') : (s.heading ?? ''),
      tokens: s.tokens,
      score: typeof s.score === 'number' ? s.score.toFixed(2) : '—',
    }));
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Context Packs — {data.repo.name}</h1>

{#if data.packs.length === 0}
  <p>No context packs yet. Call the <code>context_pack</code> MCP tool to generate one.</p>
{:else}
  <p>{data.packs.length} pack{data.packs.length !== 1 ? 's' : ''}</p>

  {#each data.packs as pack (pack.id)}
    <div class="pack">
      <div class="pack-header" onclick={() => toggle(pack.id)} role="button" tabindex="0"
           onkeydown={(e) => e.key === 'Enter' && toggle(pack.id)}>
        <div class="pack-title">
          <span class="task">{pack.task}</span>
          {#if pack.sessionId}<span class="meta">session: {pack.sessionId}</span>{/if}
        </div>
        <div class="pack-meta">
          <span class="tokens">{pack.totalTokens.toLocaleString()} tokens</span>
          <span class="date">{fmt(pack.createdAt)}</span>
          <span>{expandedId === pack.id ? '▲' : '▼'}</span>
        </div>
      </div>

      {#if expandedId === pack.id}
        <div class="pack-slices">
          {#if pack.slices.length === 0}
            <p class="empty">No slices in this pack.</p>
          {:else}
            <Table data={sliceRows(pack.slices)} columns={sliceColumns} />
            <p class="total">{pack.totalTokens.toLocaleString()} tokens</p>
          {/if}
        </div>
      {/if}
    </div>
  {/each}
{/if}

<style>
  .pack { border: 1px solid #ccc; margin-bottom: 0.75rem; border-radius: 4px; overflow: hidden; }
  .pack-header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 1rem; cursor: pointer; background: #fafafa; }
  .pack-header:hover { background: #f0f0f0; }
  .pack-title { display: flex; flex-direction: column; gap: 0.25rem; }
  .task { font-weight: 500; }
  .meta { font-size: 0.8rem; color: #666; }
  .pack-meta { display: flex; gap: 1rem; align-items: center; font-size: 0.85rem; }
  .tokens { font-weight: 500; }
  .pack-slices { padding: 0.75rem 1rem; }
  .total { text-align: right; font-size: 0.85rem; color: #555; margin-top: 0.5rem; }
  .empty { color: #999; font-style: italic; }
</style>
