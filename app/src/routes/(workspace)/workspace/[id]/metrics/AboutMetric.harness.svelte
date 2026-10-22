<script lang="ts">
    import AboutMetric from './AboutMetric.svelte';
    import type { MetricAbout, TextSegment } from '$lib/metrics/metric-view.js';

    let {
        about,
        howToReadSegments = [],
        workspaceId = '',
    }: { about: MetricAbout; howToReadSegments?: TextSegment[]; workspaceId?: string } = $props();
</script>

<AboutMetric {about} {howToReadSegments} {workspaceId} />
