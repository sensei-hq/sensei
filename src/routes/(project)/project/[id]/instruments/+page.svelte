<script lang="ts">
  let { data } = $props();
</script>

<div class="instruments-page">
  <header class="page-header">
    <h2>Instruments</h2>
    <span class="total">{data.tools.length} tools</span>
  </header>

  {#if data.tools.length === 0}
    <p class="empty-hint">No instruments associated with this project yet.</p>
  {:else}
    <ul class="tool-list">
      {#each data.tools as tool (tool.id)}
        <li class="tool-row">
          <span class="tool-name">{tool.name}</span>
          <span class="tool-kind">{tool.kind}</span>
          <span class="scope-badge" class:global={tool.scope === 'global'} class:proj={tool.scope === 'project'}>
            [{tool.scope}]
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .instruments-page { padding: 24px; }
  .page-header { display: flex; align-items: baseline; gap: 16px; margin-bottom: 20px; }
  .page-header h2 { margin: 0; }
  .total { font-size: 13px; opacity: 0.6; }
  .tool-list { list-style: none; margin: 0; padding: 0; }
  .tool-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .tool-name { font-weight: 600; flex: 1; }
  .tool-kind { opacity: 0.5; font-size: 12px; }
  .scope-badge { font-size: 11px; padding: 1px 6px; border-radius: 4px; font-family: monospace; }
  .scope-badge.global { background: var(--surface-3); opacity: 0.7; }
  .scope-badge.proj { background: color-mix(in srgb, var(--shu, #c0392b) 15%, transparent); color: var(--shu, #c0392b); }
  .empty-hint { opacity: 0.5; font-size: 13px; }
</style>
