<script lang="ts">
  // TOKENS — one vertical bar per day, input tokens (bottom) stacked under
  // output tokens (top), over the range. The volume/cost view: how heavy the
  // day's AI usage was, and the input(cache)-vs-output balance. Pure
  // presentation over the per-day rollup from sessions-digest.ts.
  import type { DayBucket } from '$lib/sessions-digest.js';
  import { compactTokens } from '$lib/sessions-digest.js';

  interface Props {
    buckets: DayBucket[];
    width?: number;
    height?: number;
  }
  let { buckets, width = 1100, height = 280 }: Props = $props();

  const padL = 48, padR = 30, padT = 20, padB = 30;
  const innerW = $derived(width - padL - padR);
  const innerH = $derived(height - padT - padB);
  const n = $derived(buckets.length);
  // A real 0-token day is legitimate; keep a floor so an all-empty range still
  // draws an axis rather than dividing by zero.
  const maxTotal = $derived(Math.max(1, ...buckets.map((b) => b.tokens)));

  // Bar geometry: evenly spaced slots, bar width a fraction of the slot.
  const slot = $derived(innerW / Math.max(n, 1));
  const barW = $derived(Math.max(1, Math.min(slot * 0.7, 26)));
  const cx = (i: number) => padL + slot * i + slot / 2;
  const h = (v: number) => (v / maxTotal) * innerH;
  const baseY = $derived(padT + innerH);

  const yTicks = $derived([0, Math.round(maxTotal / 2), maxTotal]);
  const labelStride = $derived(n > 14 ? Math.ceil(n / 8) : 1);
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="tokens-chart">
  {#each yTicks as t (t)}
    <line x1={padL} x2={width - padR} y1={baseY - h(t)} y2={baseY - h(t)} stroke="var(--paper-edge)" stroke-dasharray="2 4" />
    <text x={padL - 8} y={baseY - h(t) + 3} font-size="10" fill="var(--ink-mute)" text-anchor="end">
      {compactTokens(t)}
    </text>
  {/each}

  {#each buckets as b, i (b.day)}
    {@const inH = h(b.tokensIn)}
    {@const outH = h(b.tokensOut)}
    {@const x = cx(i) - barW / 2}
    <g>
      <title>{b.label}: {compactTokens(b.tokensIn)} in · {compactTokens(b.tokensOut)} out</title>
      <!-- input tokens (bottom) -->
      {#if inH > 0}
        <rect {x} y={baseY - inH} width={barW} height={inH} rx="1.5" fill="var(--accent-soft)" />
      {/if}
      <!-- output tokens (stacked on top) -->
      {#if outH > 0}
        <rect {x} y={baseY - inH - outH} width={barW} height={outH} rx="1.5" fill="var(--accent)" />
      {/if}
    </g>
  {/each}

  {#each buckets as b, i (b.day)}
    {#if i % labelStride === 0 || i === n - 1}
      <text x={cx(i)} y={height - 8} font-size="10" fill="var(--ink-soft)" text-anchor="middle">
        {b.label}
      </text>
    {/if}
  {/each}
</svg>
