<script lang="ts">
  // The Matrix lens (M17): module × module, ordered by group from the top, so
  // an upward dependency (red) sits below the diagonal.
  import { matrix } from './lenses';
  import type { Level0 } from './types';

  let { data, onopen }: { data: Level0; onopen: (id: string) => void } = $props();
  const m = $derived(matrix(data));
  const CELL = 12;
  const LABEL = 180;
</script>

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="overflow-auto" tabindex="0" role="region" aria-label="Dependency matrix, scrollable" data-testid="codemap-matrix">
  <svg width={LABEL + m.ids.length * CELL + 8} height={LABEL + m.ids.length * CELL + 8} role="img" aria-label="Dependency matrix">
    {#each m.ids as id, i (id)}
      <text x={LABEL - 6} y={LABEL + i * CELL + CELL - 2} text-anchor="end" class="text-xs cursor-pointer" fill="var(--ink-soft)"
        role="button" tabindex="0" onclick={() => onopen(id)} onkeydown={(e) => e.key === 'Enter' && onopen(id)}>{id.split('/').slice(-1)[0]}</text>
      {#each m.ids as _, j (j)}
        {@const w = m.weight[i][j]}
        <rect
          x={LABEL + j * CELL}
          y={LABEL + i * CELL}
          width={CELL - 1}
          height={CELL - 1}
          fill={w === 0 ? 'var(--paper-soft)' : m.upward[i][j] ? 'var(--danger)' : 'var(--ink)'}
          fill-opacity={w === 0 ? 1 : 0.25 + 0.75 * (w / m.max)}
        ><title>{id} → {m.ids[j]}: {w}</title></rect>
      {/each}
    {/each}
  </svg>
</div>
