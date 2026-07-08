<script lang="ts">
  import type { ObservatoryAdopted } from '$lib/types.js';

  let { item }: { item: ObservatoryAdopted } = $props();
</script>

<div
  data-component="adopted-card"
  class="py-3 px-3 rounded bg-paper-soft border border-paper-edge border-l-2 border-l-accent"
>
  <div class="flex items-baseline gap-2 mb-1">
    <span class="font-mono text-xs text-ink-soft">{item.when}</span>
    <span class="text-xs text-ink-faint">·</span>
    <span class="font-mono text-xs text-accent">{item.scope}</span>
  </div>
  <p class="text-sm text-ink m-0 leading-snug">{item.what}</p>
  {#if item.source?.trim()}
    <p data-adopted-source class="text-xs text-ink-faint m-0 mt-1">{item.source}</p>
  {/if}
</div>
