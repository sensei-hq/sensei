<script lang="ts">
    import DatapointDrilldown from './DatapointDrilldown.svelte';
    import type { DatapointDrilldownState } from './datapoint-drilldown.svelte.js';
    import type { MetricSeriesPoint } from '$lib/metrics/metric-view.js';

    // The drill-down is driven by an injected, fixture-backed controller so the
    // spec renders every state (loading / ready / empty / unavailable / error)
    // deterministically, without a live daemon.
    let {
        series,
        workspaceId = 'proj-1',
        metricKey = 'ftr',
        controller,
    }: {
        series: MetricSeriesPoint[];
        workspaceId?: string;
        metricKey?: string;
        controller: DatapointDrilldownState;
    } = $props();
</script>

<DatapointDrilldown {series} {workspaceId} {metricKey} {controller} />
