<script lang="ts">
  /** Compact inline strip that visualises a session's turn count with the
   *  correction slice shaded in warning tone. Reads like a "how much of this
   *  session was rework?" glance without needing per-turn data. */
  interface Props {
    turns: number;
    corrections: number;
    width?: number;
    height?: number;
  }
  let { turns, corrections, width = 60, height = 8 }: Props = $props();

  const totalCells = $derived(Math.max(turns, 1));
  const correctedCells = $derived(Math.min(corrections, totalCells));
  const cleanCells = $derived(totalCells - correctedCells);

  const gap = $derived(totalCells > 20 ? 0 : 1);
  const cellW = $derived(Math.max(1, (width - gap * (totalCells - 1)) / totalCells));
</script>

{#if turns > 0}
  <svg
    width={width}
    height={height}
    class="block"
    data-component="turn-bar"
    data-turns={turns}
    data-corrections={corrections}
    role="img"
    aria-label={`${turns} turns, ${corrections} rework`}
  >
    {#each { length: cleanCells } as _, i (i)}
      <rect
        x={i * (cellW + gap)}
        y={0}
        width={cellW}
        height={height}
        fill="var(--paper-edge)"
      />
    {/each}
    {#each { length: correctedCells } as _, i (i)}
      <rect
        x={(cleanCells + i) * (cellW + gap)}
        y={0}
        width={cellW}
        height={height}
        fill="var(--warning)"
      />
    {/each}
  </svg>
{/if}
