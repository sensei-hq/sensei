<script lang="ts">
  import type { LegendItem } from './atlas-graph.svelte.js';

  let { items }: { items: LegendItem[] } = $props();
</script>

<!-- Colour key — one swatch per node kind present in the current view. Fills
     resolve to named tokens, so a data-driven inline colour is the only way to
     paint the swatch. -->
<div class="flex items-center flex-wrap gap-3" data-component="atlas-legend">
  {#each items as it (it.kind)}
    <span class="inline-flex items-center gap-2 text-xs text-ink-mute">
      <span
        class="inline-block w-[11px] h-[11px] rounded-full"
        class:border={it.ring}
        class:border-ink-faint={it.ring}
        style="background: {it.color};"
      ></span>
      {it.kind}
    </span>
  {/each}
</div>
