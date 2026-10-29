<script lang="ts">
  // BANDS — a vertical bar per day, oldest left, stacked by minutes spent in each
  // quality class (good · corrected · abandoned). Pure presentation over
  // `bandsView` (sessions-digest.ts), which lays out the bars and thins the date
  // labels so 90 days stay readable.
  import { BAND_FILL, bandsView, compactDuration, type DayBucket } from '$lib/sessions-digest.js';

  interface Props {
    buckets: DayBucket[];
    width?: number;
    height?: number;
  }
  let { buckets, width = 1100, height = 320 }: Props = $props();

  const view = $derived(bandsView(buckets, width, height));
</script>

<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="bands-chart">
  {#each view.yTicks as t (t.label)}
    <line x1={view.plot.left} x2={view.plot.right} y1={t.y} y2={t.y} stroke="var(--paper-edge)" stroke-dasharray="2 4" />
    <text x={view.plot.left - 8} y={t.y + 3} font-size="10" fill="var(--ink-mute)" text-anchor="end" class="font-mono">{t.label}</text>
  {/each}
  {#each view.bars as b (b.day)}
    <g data-day={b.day}>
      <title>{b.label} · {b.total} {b.total === 1 ? 'session' : 'sessions'} · {compactDuration(b.mins)}</title>
      {#each b.segments as s (s.tone)}
        <rect x={b.x} y={s.y} width={b.w} height={s.h} rx="1.5" fill={BAND_FILL[s.tone]} opacity="0.85" data-tone={s.tone} />
      {/each}
    </g>
  {/each}
  {#each view.ticks as t (t.label)}
    <text x={t.x} y={view.plot.bottom + 16} font-size="10" fill="var(--ink-mute)" text-anchor="middle">{t.label}</text>
  {/each}
</svg>
