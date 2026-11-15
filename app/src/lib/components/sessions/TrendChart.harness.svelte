<script lang="ts">
  // Test wrapper — passes a TurnDay[] straight through so the spec can drive the
  // default chart with hand-built days (no turnsByDay plumbing needed).
  import TrendChart from './TrendChart.svelte';
  import type { TurnDay } from '$lib/sessions-digest.js';

  let { days, width, height }: { days: TurnDay[]; width?: number; height?: number } = $props();
</script>

<TrendChart {days} {width} {height} />
