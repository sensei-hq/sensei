<script lang="ts">
  import { labelStride, type TurnDay, axisTicks } from '$lib/sessions-digest.js';
  // STREAM — turns per day, the corrected ones under the first-try ones, as two
  // filled areas over the range. Pure presentation over the turns filed by day
  // upstream (turnsByDay), so a session that ran for days counts on each day.

  interface Props {
    days: TurnDay[];
    width?: number;
    height?: number;
  }
  let { days, width = 1100, height = 280 }: Props = $props();

  const padL = 40, padR = 30, padT = 20, padB = 30;
  const innerW = $derived(width - padL - padR);
  const innerH = $derived(height - padT - padB);
  const n = $derived(days.length);
  const maxTotal = $derived(Math.max(4, ...days.map((d) => d.turns)));

  const x = (i: number) => padL + (i / Math.max(n - 1, 1)) * innerW;
  const y = (v: number) => padT + innerH - (v / maxTotal) * innerH;

  // Stacked bottom-up: each layer sits on the sum of the layers beneath it.
  type Layer = 'corrected' | 'firstTry';
  const LAYERS: Array<{ tone: Layer; color: string; below: Layer[] }> = [
    { tone: 'corrected', color: 'var(--warning)', below: [] },
    { tone: 'firstTry', color: 'var(--success)', below: ['corrected'] },
  ];
  const count = (d: TurnDay, tone: Layer) => (tone === 'corrected' ? d.corrections : d.turns - d.corrections);
  function areaPath(tone: Layer, below: Layer[]): string {
    if (n === 0) return '';
    const top = days.map((d, i) => {
      const base = below.reduce((sum, k) => sum + count(d, k), 0);
      return `${x(i).toFixed(1)} ${y(base + count(d, tone)).toFixed(1)}`;
    });
    const bottom = days
      .map((d, i) => {
        const base = below.reduce((sum, k) => sum + count(d, k), 0);
        return `${x(i).toFixed(1)} ${y(base).toFixed(1)}`;
      })
      .reverse();
    return `M ${top.join(' L ')} L ${bottom.join(' L ')} Z`;
  }

  const yTicks = $derived(axisTicks(maxTotal));
  const stride = $derived(labelStride(n));
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="stream-chart">
  {#each yTicks as t (t)}
    <line x1={padL} x2={width - padR} y1={y(t)} y2={y(t)} stroke="var(--paper-edge)" stroke-dasharray="2 4" />
    <text x={padL - 8} y={y(t) + 3} font-size="10" fill="var(--ink-soft)" text-anchor="end">
      {t}
    </text>
  {/each}

  {#each LAYERS as layer (layer.tone)}
    <path d={areaPath(layer.tone, layer.below)} fill={layer.color} opacity="0.7" />
  {/each}

  {#each days as b, i (b.day)}
    {#if i % stride === 0 || i === n - 1}
      <text x={x(i)} y={height - 8} font-size="10" fill="var(--ink-mute)" text-anchor="middle">
        {b.label}
      </text>
    {/if}
  {/each}
</svg>
