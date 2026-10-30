<script lang="ts">
  // The Matrix lens (M17): module × module, rows depending on columns, ordered by
  // group from the top so an upward dependency (red) sits below the diagonal. A
  // grid whose column names hold to the top and row names to the left as it
  // scrolls, so a cell is always read against what it joins. A row's name opens
  // its module; the cell size follows the zoom.
  import { matrix } from './lenses';
  import type { Level0 } from './types';

  let { data, zoom = 1, onopen }: { data: Level0; zoom?: number; onopen: (id: string) => void } = $props();
  const m = $derived(matrix(data));
  const CELL = 14;
  const cell = $derived(`${Math.round(CELL * zoom)}px`);
  const short = (id: string) => id.split('/').slice(-1)[0];
  const fill = (i: number, j: number) => {
    const w = m.weight[i][j];
    if (w === 0) return { background: 'var(--paper-soft)', opacity: 1 };
    return { background: m.upward[i][j] ? 'var(--danger)' : 'var(--ink)', opacity: 0.25 + 0.75 * (w / m.max) };
  };
</script>

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="overflow-auto max-h-[70vh]" tabindex="0" role="region" aria-label="Dependency matrix, scrollable" data-testid="codemap-matrix">
  <table class="border-separate border-spacing-px" aria-label="Dependency matrix: rows depend on columns">
    <thead>
      <tr>
        <th class="sticky top-0 left-0 z-20 bg-paper"></th>
        {#each m.ids as id (id)}
          <th class="sticky top-0 z-10 bg-paper align-bottom pb-1 font-normal text-xs text-ink-soft" scope="col" title={id} data-testid="matrix-col">
            <span class="inline-block whitespace-nowrap" style:writing-mode="vertical-rl" style:transform="rotate(180deg)">{short(id)}</span>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each m.ids as id, i (id)}
        <tr>
          <th class="sticky left-0 z-10 bg-paper pr-2 text-right font-normal whitespace-nowrap" scope="row" title={id} data-testid="matrix-row">
            <button class="p-0 bg-transparent border-none cursor-pointer text-xs text-ink-soft hover:underline" onclick={() => onopen(id)}>{short(id)}</button>
          </th>
          {#each m.ids as other, j (other)}
            {@const f = fill(i, j)}
            <td class="p-0" style:width={cell} style:height={cell} style:min-width={cell}
              style:background={f.background} style:opacity={f.opacity}
              title={`${id} → ${other}: ${m.weight[i][j]}`}
              data-cell={`${id}>${other}`} data-upward={m.upward[i][j] || undefined}></td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
