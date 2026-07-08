<script lang="ts">
  // STREAM — minutes per day stacked by quality (ugly → bad → good, bottom
  // up) as three filled areas over the range. Pure presentation over the
  // per-day rollup from sessions-digest.ts.
  import type { DayBucket } from './sessions-digest.js';

  interface Props {
    buckets: DayBucket[];
    width?: number;
    height?: number;
  }
  let { buckets, width = 1100, height = 280 }: Props = $props();

  const padL = 40, padR = 30, padT = 20, padB = 30;
  const innerW = $derived(width - padL - padR);
  const innerH = $derived(height - padT - padB);
  const n = $derived(buckets.length);
  const maxTotal = $derived(Math.max(60, ...buckets.map((b) => b.chartedMins)));

  const x = (i: number) => padL + (i / Math.max(n - 1, 1)) * innerW;
  const y = (v: number) => padT + innerH - (v / maxTotal) * innerH;

  // Stacked bottom-up: each layer sits on the sum of the layers beneath it.
  const LAYERS: Array<{ tone: 'ugly' | 'bad' | 'good'; color: string; below: Array<'ugly' | 'bad' | 'good'> }> = [
    { tone: 'ugly', color: 'var(--accent)', below: [] },
    { tone: 'bad', color: 'var(--warning)', below: ['ugly'] },
    { tone: 'good', color: 'var(--success)', below: ['ugly', 'bad'] },
  ];
  function mins(b: DayBucket, tone: 'ugly' | 'bad' | 'good'): number {
    return tone === 'good' ? b.goodMins : tone === 'bad' ? b.badMins : b.uglyMins;
  }
  function areaPath(tone: 'ugly' | 'bad' | 'good', below: Array<'ugly' | 'bad' | 'good'>): string {
    if (n === 0) return '';
    const top = buckets.map((b, i) => {
      const base = below.reduce((sum, k) => sum + mins(b, k), 0);
      return `${x(i).toFixed(1)} ${y(base + mins(b, tone)).toFixed(1)}`;
    });
    const bottom = buckets
      .map((b, i) => {
        const base = below.reduce((sum, k) => sum + mins(b, k), 0);
        return `${x(i).toFixed(1)} ${y(base).toFixed(1)}`;
      })
      .reverse();
    return `M ${top.join(' L ')} L ${bottom.join(' L ')} Z`;
  }

  const yTicks = $derived([0, Math.round(maxTotal / 2), maxTotal]);
  const labelStride = $derived(n > 14 ? Math.ceil(n / 8) : 1);
  const fmtMins = (t: number) => (t < 60 ? `${t}m` : `${Math.floor(t / 60)}h`);
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="stream-chart">
  {#each yTicks as t (t)}
    <line x1={padL} x2={width - padR} y1={y(t)} y2={y(t)} stroke="var(--paper-edge)" stroke-dasharray="2 4" />
    <text x={padL - 8} y={y(t) + 3} font-size="10" fill="var(--ink-mute)" text-anchor="end">
      {fmtMins(t)}
    </text>
  {/each}

  {#each LAYERS as layer (layer.tone)}
    <path d={areaPath(layer.tone, layer.below)} fill={layer.color} opacity="0.7" />
  {/each}

  {#each buckets as b, i (b.day)}
    {#if i % labelStride === 0 || i === n - 1}
      <text x={x(i)} y={height - 8} font-size="10" fill="var(--ink-soft)" text-anchor="middle">
        {b.label}
      </text>
    {/if}
  {/each}
</svg>
