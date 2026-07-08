<script lang="ts">
  // Test wrapper — passes a DayBucket[] straight through so the spec can drive
  // the default chart with hand-built rollups (no enrich() plumbing needed).
  import TrendChart from './TrendChart.svelte';
  import type { DayBucket } from '$lib/sessions-digest.js';

  let { buckets, width, height }: { buckets: DayBucket[]; width?: number; height?: number } = $props();
</script>

<TrendChart {buckets} {width} {height} />
