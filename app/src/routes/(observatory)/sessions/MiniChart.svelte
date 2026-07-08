<script lang="ts">
  // Compact visualization for the collapsed-header mini-cycler. Renders one of
  // the five chart-ish modes (trend / stream / constellation / bands / pulse);
  // the sixth cycler mode, `numbers`, is the resting stat trio drawn by the
  // hero itself. `pulse` is reachable ONLY through the cycler — never a chip.
  import type { DayBucket, EnrichedSession, QualityTone } from './sessions-digest.js';

  type MiniChartMode = 'trend' | 'stream' | 'constellation' | 'bands' | 'pulse';

  interface Props {
    mode: MiniChartMode;
    buckets: DayBucket[];
    sessions: EnrichedSession[];
    days: string[];
    width?: number;
    height?: number;
  }
  let { mode, buckets, sessions, days, width = 168, height = 56 }: Props = $props();

  const TONE_COLOR: Record<QualityTone, string> = {
    good: 'var(--success)',
    bad: 'var(--warning)',
    ugly: 'var(--accent)',
    neutral: 'var(--ink-mute)',
  };
  const n = $derived(buckets.length);
  const xStep = $derived(n > 1 ? width / (n - 1) : 0);
  const qTotal = (b: DayBucket) => b.good + b.bad + b.ugly;

  // ── trend — per-day FTR sparkline ──────────────────────────────────────
  const trendPts = $derived(
    buckets
      .map((b, i): [number, number] | null =>
        b.ftr == null ? null : [i * xStep, height - 3 - b.ftr * (height - 6)],
      )
      .filter((p): p is [number, number] => p !== null),
  );
  const trendLine = $derived(
    trendPts.length ? `M ${trendPts.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join(' L ')}` : '',
  );
  const trendArea = $derived(
    trendPts.length
      ? `${trendLine} L ${trendPts[trendPts.length - 1][0].toFixed(1)} ${height} L ${trendPts[0][0].toFixed(1)} ${height} Z`
      : '',
  );

  // ── stream — stacked quality counts per day ────────────────────────────
  const streamMax = $derived(Math.max(1, ...buckets.map(qTotal)));
  const STREAM_LAYERS: Array<{ tone: 'ugly' | 'bad' | 'good'; below: Array<'ugly' | 'bad' | 'good'> }> = [
    { tone: 'ugly', below: [] },
    { tone: 'bad', below: ['ugly'] },
    { tone: 'good', below: ['ugly', 'bad'] },
  ];
  const count = (b: DayBucket, tone: 'ugly' | 'bad' | 'good') =>
    tone === 'good' ? b.good : tone === 'bad' ? b.bad : b.ugly;
  function streamPath(tone: 'ugly' | 'bad' | 'good', below: Array<'ugly' | 'bad' | 'good'>): string {
    if (n === 0) return '';
    const top = buckets.map((b, i) => {
      const base = below.reduce((s, k) => s + count(b, k), 0);
      const v = base + count(b, tone);
      return `${(i * xStep).toFixed(1)} ${(height - (v / streamMax) * (height - 2)).toFixed(1)}`;
    });
    return `M ${top.join(' L ')} L ${((n - 1) * xStep).toFixed(1)} ${height} L 0 ${height} Z`;
  }

  // ── bands — stacked bars per day ───────────────────────────────────────
  const barW = $derived(n > 0 ? Math.max(1, (width - (n - 1) * 2) / n) : 0);
  const BAR_TONES: Array<'good' | 'bad' | 'ugly'> = ['good', 'bad', 'ugly'];
  function barSegments(b: DayBucket, i: number): Array<{ color: string; x: number; y: number; h: number }> {
    let yAcc = height;
    const x = i * (barW + 2);
    const out: Array<{ color: string; x: number; y: number; h: number }> = [];
    for (const tone of BAR_TONES) {
      const c = tone === 'good' ? b.good : tone === 'bad' ? b.bad : b.ugly;
      const h = (c / streamMax) * (height - 2);
      if (h > 0) {
        yAcc -= h;
        out.push({ color: TONE_COLOR[tone], x, y: yAcc, h });
      }
    }
    return out;
  }

  // ── constellation / pulse — per-session marks ──────────────────────────
  const maxMins = $derived(Math.max(60, ...sessions.map((s) => s.mins ?? 0)));
  const marks = $derived(
    sessions
      .map((s) => ({ s, idx: days.indexOf(s.dayKey) }))
      .filter((m) => m.idx >= 0),
  );
</script>

<svg
  width={width}
  height={height}
  class="block overflow-visible"
  data-component="mini-chart"
  data-mode={mode}
>
  {#if mode === 'trend'}
    <path d={trendArea} fill="var(--success)" opacity="0.12" />
    <path d={trendLine} fill="none" stroke="var(--success)" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    {#each trendPts as p, i (i)}
      <circle cx={p[0]} cy={p[1]} r={i === trendPts.length - 1 ? 3 : 1.6} fill="var(--success)" />
    {/each}
  {:else if mode === 'stream'}
    {#each STREAM_LAYERS as layer (layer.tone)}
      <path d={streamPath(layer.tone, layer.below)} fill={TONE_COLOR[layer.tone]} opacity="0.85" />
    {/each}
  {:else if mode === 'bands'}
    {#each buckets as b, i (b.day)}
      {#each barSegments(b, i) as seg, s (s)}
        <rect x={seg.x} y={seg.y} width={barW} height={seg.h} fill={seg.color} opacity="0.85" />
      {/each}
    {/each}
  {:else if mode === 'constellation'}
    {#each marks as m (m.s.id)}
      <circle
        cx={m.idx * xStep}
        cy={height - 2 - ((m.s.mins ?? 0) / maxMins) * (height - 4)}
        r="2.2"
        fill={TONE_COLOR[m.s.quality]}
        opacity="0.85"
      />
    {/each}
  {:else}
    <line x1="0" x2={width} y1={height - 2} y2={height - 2} stroke="var(--paper-edge)" stroke-width="0.5" />
    {#each marks as m (m.s.id)}
      {@const len = Math.max(4, Math.min(height - 4, (m.s.mins ?? 0) / 10))}
      <line
        x1={m.idx * xStep}
        x2={m.idx * xStep}
        y1={height - 2 - len}
        y2={height - 2}
        stroke={TONE_COLOR[m.s.quality]}
        stroke-width="1.4"
        stroke-linecap="round"
        opacity="0.85"
      />
    {/each}
  {/if}
</svg>
