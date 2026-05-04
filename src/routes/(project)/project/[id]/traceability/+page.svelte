<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Traceability</h2>
  <div class="drift-stats">
    <span>{data.total} tracked</span> · <span class="warn">{data.drifted} drifted</span> · <span class="danger">{data.broken} broken</span>
  </div>
  <ul class="drift-list">
    {#each data.driftItems as item (item.id)}
      <li class="drift-row" class:drifted={item.status === 'drifted'} class:broken={item.status === 'broken'}>
        <span class="status-dot"></span>
        <span class="detail">{item.detail ?? item.status}</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .drift-stats { font-size: 13px; margin-bottom: 16px; opacity: 0.7; }
  .warn { color: var(--amber, orange); }
  .danger { color: var(--red, red); }
  .drift-list { list-style: none; margin: 0; padding: 0; }
  .drift-row { display: flex; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .drift-row.drifted .status-dot { background: var(--amber, orange); }
  .drift-row.broken .status-dot { background: var(--red, red); }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--green, green); margin-top: 4px; flex-shrink: 0; }
  .detail { flex: 1; }
</style>
