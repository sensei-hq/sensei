<script lang="ts">
  // Mount host for HealthRadar specs. Props are forwarded verbatim so a spec can
  // exercise the populated, quiet and score-less states without a route or a
  // daemon.
  import HealthRadar from './HealthRadar.svelte';
  import type { HealthComponent } from '$lib/metrics/health-radar.js';

  let {
    components = {},
    score = null,
    ratedMetrics = 0,
    size = 320,
  }: {
    components?: Record<string, HealthComponent>;
    score?: number | null;
    ratedMetrics?: number;
    size?: number;
  } = $props();
</script>

<HealthRadar {components} {score} {ratedMetrics} {size} />
