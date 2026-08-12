<script lang="ts">
    import MetricCard from './MetricCard.svelte';
    import type { MetricCardVM } from '$lib/metrics/metric-view.js';

    let { card, series = [] }: { card: MetricCardVM; series?: (number | null)[] } = $props();
</script>

<MetricCard {card} {series} />
