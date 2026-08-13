<script lang="ts">
    import AboutMetric from './AboutMetric.svelte';
    import type { MetricAbout } from '$lib/metrics/metric-view.js';

    let { about }: { about: MetricAbout } = $props();
</script>

<AboutMetric {about} />
