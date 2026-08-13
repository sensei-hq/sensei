<script lang="ts">
    import AboutMetric from './AboutMetric.svelte';
    import type { MetricAbout, TextSegment } from '$lib/metrics/metric-view.js';

    let {
        about,
        howToReadSegments = [],
        projectId = '',
    }: { about: MetricAbout; howToReadSegments?: TextSegment[]; projectId?: string } = $props();
</script>

<AboutMetric {about} {howToReadSegments} {projectId} />
