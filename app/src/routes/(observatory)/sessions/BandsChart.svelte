<script lang="ts">
  // BANDS — one horizontal row per day, segment widths = minutes spent in
  // each quality class. Newest day on top. Pure presentation over the
  // per-day rollup from sessions-digest.ts.
  import type { DayBucket } from './sessions-digest.js';

  interface Props {
    buckets: DayBucket[];
    width?: number;
    height?: number;
  }
  let { buckets, width = 1100, height = 320 }: Props = $props();

  const padL = 60, padR = 70, padT = 14, padB = 14;
  const innerW = $derived(width - padL - padR);
  // Newest day first (top of the stack), matching the mockup.
  const rows = $derived([...buckets].reverse());
  const rowH = $derived((height - padT - padB) / Math.max(rows.length, 1));
  const maxTotal = $derived(Math.max(60, ...rows.map((r) => r.chartedMins)));
  const w = (mins: number) => (mins / maxTotal) * innerW;

  // Ordered good → bad → ugly with their minute field + fill token.
  const SEGMENTS: Array<{ tone: 'good' | 'bad' | 'ugly'; color: string }> = [
    { tone: 'good', color: 'var(--success)' },
    { tone: 'bad', color: 'var(--warning)' },
    { tone: 'ugly', color: 'var(--accent)' },
  ];
  function segMins(r: DayBucket, tone: 'good' | 'bad' | 'ugly'): number {
    return tone === 'good' ? r.goodMins : tone === 'bad' ? r.badMins : r.uglyMins;
  }
  function offsets(r: DayBucket): Array<{ color: string; x: number; w: number }> {
    let cx = padL;
    const out: Array<{ color: string; x: number; w: number }> = [];
    for (const s of SEGMENTS) {
      const width = w(segMins(r, s.tone));
      if (width > 0) out.push({ color: s.color, x: cx, w: width });
      cx += width;
    }
    return out;
  }
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="bands-chart">
  {#each rows as r, i (r.day)}
    {@const cy = padT + i * rowH + rowH / 2}
    <text x={padL - 10} y={cy + 3} font-size="10.5" fill="var(--ink-soft)" text-anchor="end">
      {r.label}
    </text>
    <rect x={padL} y={cy - 9} width={innerW} height="18" rx="3" fill="var(--paper-mute)" opacity="0.5" />
    {#each offsets(r) as seg, s (s)}
      <rect x={seg.x} y={cy - 9} width={seg.w} height="18" rx="2" fill={seg.color} opacity="0.85" />
    {/each}
    <text x={padL + innerW + 10} y={cy + 3} font-size="10" fill="var(--ink-soft)" class="font-mono">
      {r.total}× · {r.chartedMins}m
    </text>
  {/each}
</svg>
