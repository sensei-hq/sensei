<script lang="ts">
  // TREND — first-try-right rate over the range as a smooth line, with a
  // 7-day rolling average and daily dots colored by the day's dominant
  // quality. Pure presentation: the per-day rollup is computed upstream in
  // sessions-digest.ts and handed in as `buckets`.
  import type { DayBucket, QualityTone } from '$lib/sessions-digest.js';

  interface Props {
    buckets: DayBucket[];
    width?: number;
    height?: number;
  }
  let { buckets, width = 1100, height = 300 }: Props = $props();

  const padL = 46, padR = 30, padT = 26, padB = 34;
  const innerW = $derived(width - padL - padR);
  const innerH = $derived(height - padT - padB);
  const n = $derived(buckets.length);

  // Geometry helpers (inline SVG maths — not view state).
  const x = (i: number) => padL + (i / Math.max(n - 1, 1)) * innerW;
  const yFtr = (v: number) => padT + innerH - v * innerH;

  // Colored dots read the day's dominant quality; only good drives the % line.
  const TONE_COLOR: Record<QualityTone, string> = {
    good: 'var(--success)',
    bad: 'var(--warning)',
    ugly: 'var(--accent)',
    neutral: 'var(--ink-mute)',
  };
  function dominant(b: DayBucket): QualityTone {
    if (b.ugly > 0) return 'ugly';
    if (b.bad > b.good) return 'bad';
    if (b.good > 0) return 'good';
    return 'neutral';
  }

  function linePath(points: Array<[number, number]>): string {
    if (points.length === 0) return '';
    let d = `M ${points[0][0].toFixed(1)} ${points[0][1].toFixed(1)}`;
    for (let i = 1; i < points.length; i++) {
      const [px, py] = points[i - 1];
      const [cx, cy] = points[i];
      const mx = (px + cx) / 2;
      d += ` C ${mx.toFixed(1)} ${py.toFixed(1)}, ${mx.toFixed(1)} ${cy.toFixed(1)}, ${cx.toFixed(1)} ${cy.toFixed(1)}`;
    }
    return d;
  }

  // Daily FTR points (skip empty days) + a 7-day rolling mean line.
  const dailyPts = $derived(
    buckets
      .map((b, i): [number, number] | null => (b.ftr == null ? null : [x(i), yFtr(b.ftr)]))
      .filter((p): p is [number, number] => p !== null),
  );
  const rollingPts = $derived(
    buckets
      .map((b, i): [number, number] | null => {
        const slice = buckets.slice(Math.max(0, i - 6), i + 1).filter((d) => d.ftr != null);
        if (slice.length === 0 || b.ftr == null) return null;
        const avg = slice.reduce((a, d) => a + (d.ftr ?? 0), 0) / slice.length;
        return [x(i), yFtr(avg)];
      })
      .filter((p): p is [number, number] => p !== null),
  );

  const labelStride = $derived(n > 14 ? Math.ceil(n / 8) : 1);
  const hasData = $derived(dailyPts.length > 0);
</script>

<svg
  viewBox={`0 0 ${width} ${height}`}
  width="100%"
  class="block"
  data-component="trend-chart"
  data-empty={hasData ? undefined : true}
>
  <!-- Y grid · 0 / 50 / 100% -->
  {#each [0, 0.5, 1] as t (t)}
    <line
      x1={padL}
      x2={width - padR}
      y1={yFtr(t)}
      y2={yFtr(t)}
      stroke="var(--paper-edge)"
      stroke-dasharray="2 4"
    />
    <text x={padL - 8} y={yFtr(t) + 3} font-size="10" fill="var(--ink-mute)" text-anchor="end">
      {Math.round(t * 100)}%
    </text>
  {/each}

  {#if hasData}
    <!-- Daily FTR — faint line + colored dots -->
    <path d={linePath(dailyPts)} fill="none" stroke="var(--ink-mute)" stroke-width="1" opacity="0.4" />
    <!-- 7-day rolling mean — the quiet narrative line -->
    <path d={linePath(rollingPts)} fill="none" stroke="var(--success)" stroke-width="2.2" />
    {#each buckets as b, i (b.day)}
      {#if b.ftr != null}
        <circle
          cx={x(i)}
          cy={yFtr(b.ftr)}
          r="3.5"
          fill={TONE_COLOR[dominant(b)]}
          stroke="var(--paper)"
          stroke-width="1"
        />
      {/if}
    {/each}
  {:else}
    <text x={width / 2} y={height / 2} font-size="12" fill="var(--ink-mute)" text-anchor="middle">
      no scored sessions in this range yet
    </text>
  {/if}

  <!-- X labels -->
  {#each buckets as b, i (b.day)}
    {#if i % labelStride === 0 || i === n - 1}
      <text x={x(i)} y={height - 10} font-size="10" fill="var(--ink-soft)" text-anchor="middle">
        {b.label}
      </text>
    {/if}
  {/each}
</svg>
