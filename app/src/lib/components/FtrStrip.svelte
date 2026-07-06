<script lang="ts">
  /** One bar per point; height ∝ rate (0..1). */
  interface Point { rate: number }
  interface Props {
    data: Point[];
    width?: number;
    height?: number;
    /** Renders in a muted palette (used for early-mode / low-confidence states). */
    dim?: boolean;
  }
  let { data, width = 168, height = 56, dim = false }: Props = $props();

  const n = $derived(data.length);
  const gap = 2;
  const barW = $derived(n > 0 ? Math.max(1, (width - gap * (n - 1)) / n) : 0);
  const barColor = $derived(dim ? 'var(--ink-faint)' : 'var(--accent)');
  const bgColor  = $derived(dim ? 'var(--paper-mute)' : 'var(--paper-edge)');
</script>

{#if n > 0}
  <svg
    width={width}
    height={height + 14}
    class="block overflow-visible"
    data-component="ftr-strip"
    data-dim={dim || undefined}
  >
    <line
      x1="0"
      x2={width}
      y1={height * 0.5}
      y2={height * 0.5}
      stroke="var(--paper-edge)"
      stroke-dasharray="2 3"
    />
    {#each data as p, i (i)}
      {@const bh = Math.max(3, Math.min(1, p.rate) * height)}
      {@const isLast = i === n - 1}
      <rect
        x={i * (barW + gap)}
        y={0}
        width={barW}
        height={height}
        fill={bgColor}
        opacity="0.35"
      />
      <rect
        x={i * (barW + gap)}
        y={height - bh}
        width={barW}
        height={bh}
        fill={isLast ? barColor : bgColor}
        opacity={isLast ? 1 : 0.85}
        data-testid={isLast ? 'ftr-bar-today' : undefined}
      />
    {/each}
    <text
      x="0"
      y={height + 11}
      font-size="9"
      fill="currentColor"
      class="text-ink-mute"
      style="letter-spacing: 0.08em"
    >14d ago</text>
    <text
      x={width}
      y={height + 11}
      font-size="9"
      fill="currentColor"
      class="text-ink-mute"
      style="letter-spacing: 0.08em"
      text-anchor="end"
    >today</text>
  </svg>
{/if}
