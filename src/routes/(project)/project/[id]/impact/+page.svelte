<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Impact</h2>
  <div class="verdict-summary">
    <span class="pos">↑ {data.positiveCount}</span>
    <span class="neg">↓ {data.negativeCount}</span>
    <span class="pend">? {data.pendingCount}</span>
  </div>
  <ul class="verdict-list">
    {#each data.verdicts as v (v.id)}
      <li class="verdict-row" class:pos={v.verdict==='positive'} class:neg={v.verdict==='negative'}>
        <span class="v-title">{v.title}</span>
        <span class="v-urgency">{v.urgency}</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .verdict-summary { display: flex; gap: 16px; font-size: 20px; font-weight: 700; margin-bottom: 20px; }
  .pos { color: var(--green, green); }
  .neg { color: var(--red, red); }
  .pend { opacity: 0.5; }
  .verdict-list { list-style: none; margin: 0; padding: 0; }
  .verdict-row { display: flex; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .verdict-row.pos { border-left: 3px solid var(--green, green); padding-left: 10px; }
  .verdict-row.neg { border-left: 3px solid var(--red, red); padding-left: 10px; }
  .v-title { flex: 1; }
  .v-urgency { opacity: 0.5; font-size: 12px; font-family: monospace; }
</style>
