<script lang="ts">
  // CONSTELLATION — one dot per session · x = duration · y = day · color =
  // quality. Reads the enriched rows directly (needs per-session minutes,
  // not the day rollup). Pure presentation.
  import { shortDayLabel, type EnrichedSession, type QualityTone } from './sessions-digest.js';

  interface Props {
    sessions: EnrichedSession[];
    days: string[];
    width?: number;
    height?: number;
  }
  let { sessions, days, width = 1100, height = 320 }: Props = $props();

  const padL = 70, padR = 30, padT = 20, padB = 40;
  const innerW = $derived(width - padL - padR);
  const innerH = $derived(height - padT - padB);
  // Newest day at the top row.
  const dayRows = $derived([...days].reverse());
  const maxMins = $derived(Math.max(60, ...sessions.map((s) => s.mins ?? 0)));

  const x = (mins: number) => padL + (mins / maxMins) * innerW;
  const y = (rowIdx: number) => padT + (rowIdx / Math.max(dayRows.length - 1, 1)) * innerH;

  const TONE_COLOR: Record<QualityTone, string> = {
    good: 'var(--success)',
    bad: 'var(--warning)',
    ugly: 'var(--accent)',
    neutral: 'var(--ink-mute)',
  };

  // Only sessions that fall on a visible day and have a measured duration.
  const dots = $derived(
    sessions
      .map((s) => ({ s, rowIdx: dayRows.indexOf(s.dayKey) }))
      .filter((d) => d.rowIdx >= 0 && d.s.mins != null),
  );
  const labelStride = $derived(dayRows.length > 14 ? Math.ceil(dayRows.length / 8) : 1);
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="constellation-chart">
  {#each dayRows as day, i (day)}
    {#if i % labelStride === 0 || i === dayRows.length - 1}
      <line x1={padL} x2={width - padR} y1={y(i)} y2={y(i)} stroke="var(--paper-edge)" stroke-dasharray="1 3" />
      <text x={padL - 10} y={y(i) + 3} font-size="10.5" fill="var(--ink-soft)" text-anchor="end">
        {shortDayLabel(day)}
      </text>
    {/if}
  {/each}

  <text x={(padL + width - padR) / 2} y={height - 8} font-size="10" fill="var(--ink-mute)" text-anchor="middle">
    ← shorter session · longer session →
  </text>

  {#each dots as d (d.s.id)}
    <circle cx={x(d.s.mins ?? 0)} cy={y(d.rowIdx)} r="5" fill={TONE_COLOR[d.s.quality]} opacity="0.85" />
  {/each}
</svg>
