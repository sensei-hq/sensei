<script lang="ts">
  // BANDS — the activity hotspots: a vertical bar per local day, oldest left, the
  // minutes the agent worked stacked under the minutes spent on you. Pure
  // presentation over `bandsView` (sessions-digest.ts); the days come from each
  // session's active time filed by the hour it happened (`activityByDay`).
  import { BAND_FILL, bandsView, compactDuration, unmeasuredLine, type ActivityDay, type SessionSpan } from '$lib/sessions-digest.js';

  interface Props {
    activity: { days: ActivityDay[]; unmeasured: number; spans?: SessionSpan[] };
    width?: number;
    height?: number;
  }
  let { activity, width = 1100, height = 320 }: Props = $props();

  const view = $derived(bandsView(activity.days, width, height, activity.spans ?? []));
  const unmeasured = $derived(unmeasuredLine(activity.unmeasured));
</script>

<div class="flex items-center gap-4 text-xs text-ink-mute mb-2" data-component="bands-legend">
  <span class="inline-flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-sm" style:background={BAND_FILL.agent}></span>agent</span>
  <span class="inline-flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-sm" style:background={BAND_FILL.user}></span>you</span>
  {#if unmeasured}
    <span data-unmeasured>{unmeasured}</span>
  {/if}
</div>
<svg viewBox={`0 0 ${width} ${height}`} width="100%" class="block" data-component="bands-chart">
  {#each view.yTicks as t (t.label)}
    <line x1={view.plot.left} x2={view.plot.right} y1={t.y} y2={t.y} stroke="var(--paper-edge)" stroke-dasharray="2 4" />
    <text x={view.plot.left - 8} y={t.y + 3} font-size="10" fill="var(--ink-mute)" text-anchor="end" class="font-mono">{t.label}</text>
  {/each}
  {#each view.bars as b (b.day)}
    <g data-day={b.day}>
      <title>{b.label} · agent {compactDuration(b.agentMins)} · you {compactDuration(b.userMins)}</title>
      {#each b.segments as s (s.tone)}
        <rect x={b.x} y={s.y} width={b.w} height={s.h} rx="1.5" fill={BAND_FILL[s.tone]} opacity="0.85" data-tone={s.tone} />
      {/each}
    </g>
  {/each}
  <!-- Multi-day sessions: one piece of work across several bars. -->
  {#each view.links as l (l.id)}
    <g data-span={l.id}>
      <line x1={l.x1} x2={l.x2} y1={l.y} y2={l.y} stroke="var(--ink-mute)" stroke-width="1" opacity="0.7" />
      {#each l.dots as x, i (i)}
        <circle cx={x} cy={l.y} r="2" fill="var(--ink-mute)" />
      {/each}
    </g>
  {/each}
  {#each view.ticks as t (t.label)}
    <text x={t.x} y={view.plot.bottom + 16} font-size="10" fill="var(--ink-mute)" text-anchor="middle">{t.label}</text>
  {/each}
</svg>
