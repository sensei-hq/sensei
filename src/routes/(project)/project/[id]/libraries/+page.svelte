<script lang="ts">
  let { data } = $props();
</script>

<div class="libraries-page">
  <header class="page-header">
    <h2>Libraries</h2>
    <div class="header-stats">
      <span>{data.wrappedCount} wrapped</span>
      <span class="sep">·</span>
      <span>{data.unwrappedCount} unwrapped</span>
    </div>
  </header>

  {#if data.libraries.length === 0}
    <p class="empty-hint">No libraries associated with this project yet.</p>
  {:else}
    <ul class="lib-list">
      {#each data.libraries as lib (lib.id)}
        <li class="lib-row">
          <span class="lib-name">{lib.name}</span>
          <span class="lib-ecosystem">{lib.ecosystem}</span>
          <span class="scope-badge" class:global={lib.scope === 'global'} class:proj={lib.scope === 'project'}>
            [{lib.scope}]
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .libraries-page { padding: 24px; }
  .page-header { display: flex; align-items: baseline; gap: 16px; margin-bottom: 20px; }
  .page-header h2 { margin: 0; }
  .header-stats { font-size: 13px; opacity: 0.6; }
  .sep { opacity: 0.4; }
  .lib-list { list-style: none; margin: 0; padding: 0; }
  .lib-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .lib-name { font-weight: 600; flex: 1; }
  .lib-ecosystem { opacity: 0.5; font-size: 12px; }
  .scope-badge { font-size: 11px; padding: 1px 6px; border-radius: 4px; font-family: monospace; }
  .scope-badge.global { background: var(--surface-3); opacity: 0.7; }
  .scope-badge.proj { background: color-mix(in srgb, var(--shu, #c0392b) 15%, transparent); color: var(--shu, #c0392b); }
  .empty-hint { opacity: 0.5; font-size: 13px; }
</style>
