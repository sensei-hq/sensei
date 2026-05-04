<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Memories</h2>
  {#if data.pendingShare > 0}
    <div class="pending-banner">{data.pendingShare} memories pending collective share</div>
  {/if}
  <ul class="memory-list">
    {#each data.memories as m (m.id)}
      <li class="memory-row">
        <span class="memory-title">{m.title}</span>
        <span class="memory-type">{m.type}</span>
        <span class="memory-strength">{Math.round((m.strength ?? 0) * 100)}%</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .pending-banner { background: color-mix(in srgb, var(--shu,#c0392b) 12%, transparent); color: var(--shu,#c0392b); padding: 10px 14px; border-radius: 6px; margin-bottom: 16px; font-size: 13px; }
  .memory-list { list-style: none; margin: 0; padding: 0; }
  .memory-row { display: flex; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .memory-title { flex: 1; }
  .memory-type { opacity: 0.5; font-size: 12px; }
  .memory-strength { font-size: 12px; font-family: monospace; }
</style>
