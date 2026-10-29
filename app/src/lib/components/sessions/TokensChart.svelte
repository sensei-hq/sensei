<script lang="ts">
  // TOKENS — one vertical bar per day, input tokens (bottom) stacked under
  // output tokens (top), over the range. The volume/cost view: how heavy the
  // day's AI usage was, and the input(cache)-vs-output balance. Pure
  // presentation over the tokens filed by the day each turn ran (turnsByDay). A
  // day that had turns but no captured transcript has no bar: its tokens are
  // unknown, and a tick says so rather than a zero.
  import { labelStride, compactTokens, type TurnDay, axisTicks } from '$lib/sessions-digest.js';

  interface Props {
    days: TurnDay[];
    width?: number;
    height?: number;
  }
  let { days, width = 1100, height = 280 }: Props = $props();

  const padL = 48, padR = 30, padT = 20, padB = 30;
  const innerW = $derived(width - padL - padR);
  const innerH = $derived(height - padT - padB);
  const n = $derived(days.length);
  // A real 0-token day is legitimate; keep a floor so an all-empty range still
  // draws an axis rather than dividing by zero.
  const maxTotal = $derived(Math.max(1, ...days.map((d) => d.tokens ?? 0)));

  // Bar geometry: evenly spaced slots, bar width a fraction of the slot.
  const slot = $derived(innerW / Math.max(n, 1));
  const barW = $derived(Math.max(1, Math.min(slot * 0.7, 26)));
  const cx = (i: number) => padL + slot * i + slot / 2;
  const h = (v: number) => (v / maxTotal) * innerH;
  const baseY = $derived(padT + innerH);

  const yTicks = $derived(axisTicks(maxTotal));
  const stride = $derived(labelStride(n));
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="tokens-chart">
  {#each yTicks as t (t)}
    <line x1={padL} x2={width - padR} y1={baseY - h(t)} y2={baseY - h(t)} stroke="var(--paper-edge)" stroke-dasharray="2 4" />
    <text x={padL - 8} y={baseY - h(t) + 3} font-size="10" fill="var(--ink-soft)" text-anchor="end">
      {compactTokens(t)}
    </text>
  {/each}

  {#each days as b, i (b.day)}
    {@const inH = h(b.tokensIn ?? 0)}
    {@const outH = h(b.tokensOut ?? 0)}
    {@const x = cx(i) - barW / 2}
    {#if b.tokens === null && b.turns > 0}
      <g data-unknown-tokens={b.day}>
        <title>{b.label}: {b.turns} turns, no token usage captured</title>
        <line x1={x} x2={x + barW} y1={baseY - 2} y2={baseY - 2} stroke="var(--ink-faint)" stroke-dasharray="2 2" />
      </g>
    {/if}
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

  {#each days as b, i (b.day)}
    {#if i % stride === 0 || i === n - 1}
      <text x={cx(i)} y={height - 8} font-size="10" fill="var(--ink-mute)" text-anchor="middle">
        {b.label}
      </text>
    {/if}
  {/each}
</svg>
