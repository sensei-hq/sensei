import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { seriesValues } from '$lib/metrics/metric-view.js';

// The metrics pane joins two daemon surfaces: the per-project *values*
// (/metrics) and the *catalog* (/metrics/registry) that alone carries each
// metric's `family` for grouping — plus a per-metric series for the sparklines.
export const load: PageLoad = async ({ params }) => {
    const api = senseiApi(appState.port);
    const [metricsRes, registryRes] = await Promise.all([
        api.getProjectMetrics(params.id),
        api.getMetricsRegistry(),
    ]);

    // A fetch FAILURE surfaces as an error state — never an empty grid that
    // hides a broken daemon (no-fabrication). Honest-empty (a project with no
    // computed metrics yet) returns error: null with an empty rows array.
    if (!metricsRes.ok) {
        const series: Record<string, number[]> = {};
        return { rows: [], registry: [], series, error: metricsRes.error.message };
    }

    const rows = metricsRes.data.metrics ?? [];
    const registry = registryRes.ok ? (registryRes.data.metrics ?? []) : [];

    // One series per metric, fetched in parallel. A missing/failed series just
    // omits that card's sparkline — the value + trend still render.
    const seriesPairs = await Promise.all(
        rows.map(async (row) => {
            const res = await api.getProjectMetricSeries(params.id, row.metric, 'daily');
            return [row.metric, res.ok ? seriesValues(res.data.series) : []] as const;
        }),
    );

    const series: Record<string, number[]> = Object.fromEntries(seriesPairs);
    return { rows, registry, series, error: null };
};
