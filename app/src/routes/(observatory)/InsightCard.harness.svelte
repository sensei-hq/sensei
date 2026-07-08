<script lang="ts">
  import InsightCard from './InsightCard.svelte';
  import type { ObservatoryInsight } from '$lib/types.js';

  let { insight, href }: { insight: ObservatoryInsight; href?: string } = $props();
</script>

<InsightCard {insight} {href} />
