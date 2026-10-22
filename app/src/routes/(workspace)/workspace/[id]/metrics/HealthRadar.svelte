<script lang="ts">
  // Composite health, all metrics at once — the radar the metrics pane was
  // missing (spec 2026-08-20-metric-rating-scales-health, phase P-B; it waited
  // on rokkit shipping a polar geom in 1.4).
  //
  // Every spoke is a metric's 0-5 RATING, not its raw value: the raw scales
  // can't share a radial axis (module_quality lives inside 0-0.005 while
  // throughput is sessions/day). Ratings come from the daemon already computed,
  // so a spoke and the composite score are the same number by construction.
  //
  // Pure template — ordering, axis specs and tone live in
  // $lib/metrics/health-radar.ts.
  import { RadarChart } from '@rokkit/chart';
  import { toSpokes, toAxes, scoreTone } from '$lib/metrics/health-radar.js';
  import type { HealthComponent } from '$lib/metrics/health-radar.js';

  let {
    components = {},
    score = null,
    ratedMetrics = 0,
    size = 320,
  }: {
    components?: Record<string, HealthComponent>;
    /** Null when the daemon rated nothing — renders the quiet state, never a 0. */
    score?: number | null;
    ratedMetrics?: number;
    size?: number;
  } = $props();

  const spokes = $derived(toSpokes(components));
  const axes = $derived(toAxes(spokes));
  const tone = $derived(score === null ? 'ink-mute' : scoreTone(score));

  // RadarChart types its rows as the library's loose `Record<string, unknown>`,
  // which a precise interface can't satisfy without an index signature. Casting
  // at this one boundary keeps `RadarSpoke` strict — typos are still caught in
  // health-radar.ts — instead of loosening the type everywhere it travels.
  const rows = $derived(spokes as unknown as Record<string, unknown>[]);
</script>

<section
  class="rounded border border-paper-edge bg-paper-soft p-4 flex flex-col gap-3"
  data-testid="health-radar"
>
  <header class="flex items-baseline justify-between gap-3">
    <div>
      <h2 class="display text-base font-normal m-0 text-ink">Health · all signals</h2>
      <p class="text-xs text-ink-mute m-0 mt-1">
        every rated metric on one 0–5 scale
      </p>
    </div>
    {#if score !== null}
      <div class="text-right shrink-0">
        <div
          class="display text-2xl leading-none"
          class:text-success={tone === 'success'}
          class:text-warning={tone === 'warning'}
          class:text-danger={tone === 'danger'}
          style='font-feature-settings: "tnum";'
          data-testid="health-score"
        >
          {score}
        </div>
        <div class="text-xs uppercase tracking-[0.12em] text-ink-faint mt-1">
          {ratedMetrics} rated
        </div>
      </div>
    {/if}
  </header>

  {#if spokes.length === 0}
    <!-- Honest-empty: nothing was rated, so there is no shape to draw. A radar
         collapsed to its centre would read as "everything failing" when it
         means "not measured yet" (spec I1/I4). -->
    <p class="text-sm text-ink-soft m-0 py-6 text-center" data-testid="health-radar-empty">
      No rated metrics yet. Signals appear here as they are computed.
    </p>
  {:else}
    <div class="flex justify-center" data-testid="health-radar-plot">
      <RadarChart
        data={rows}
        axis="metric"
        value="rating"
        {axes}
        rings={5}
        sharedDomain={true}
        alpha={0.18}
        width={size}
        height={size}
        legend={false}
      />
    </div>
  {/if}
</section>
