<script lang="ts">
  // What the lens's marks mean (M17): lines for bands, swatches for tints.
  import type { LegendEntry } from './view';

  let { entries }: { entries: LegendEntry[] } = $props();
</script>

{#if entries.length}
  <ul class="m-0 p-0 list-none flex flex-wrap items-center gap-3 text-xs text-ink-mute" data-testid="legend">
    {#each entries as e (e.label)}
      <li class="flex items-center gap-1">
        {#if e.stroke}
          <span class="inline-block w-4 border-t-2" style:border-color={e.stroke} style:border-top-style={e.dashed ? 'dashed' : 'solid'}></span>
        {:else}
          <span class="inline-block w-3 h-3 rounded-sm border border-paper-edge" style:background={e.fill}></span>
        {/if}
        {e.label}
      </li>
    {/each}
  </ul>
{/if}
