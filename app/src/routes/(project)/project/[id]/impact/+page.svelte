<script lang="ts">
  let { data } = $props();
</script>

<div class="px-6 py-6">
  <h2 class="text-xl font-normal m-0 mb-3">Impact</h2>
  <div class="flex gap-4 text-xl font-bold mb-5">
    <span style="color: oklch(var(--color-success-z5) / 1);">↑ {data.positiveCount}</span>
    <span style="color: oklch(var(--color-primary-z5) / 1);">↓ {data.negativeCount}</span>
    <span class="opacity-50">? {data.pendingCount}</span>
  </div>
  <ul class="list-none m-0 p-0">
    {#each data.verdicts as v (v.id)}
      <li class="verdict-row flex gap-3 py-2 border-b border-surface-z2 text-ui" class:pos={v.verdict==='positive'} class:neg={v.verdict==='negative'}>
        <span class="flex-1">{v.title}</span>
        <span class="opacity-50 text-xs font-mono">{v.urgency}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .verdict-row:last-child { border-bottom: none; }
  .verdict-row.pos { border-left: 3px solid oklch(var(--color-success-z5) / 1); padding-left: 10px; }
  .verdict-row.neg { border-left: 3px solid oklch(var(--color-primary-z5) / 1); padding-left: 10px; }
</style>
