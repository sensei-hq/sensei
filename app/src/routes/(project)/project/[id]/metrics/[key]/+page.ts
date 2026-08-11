import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { MetricSeriesPoint, MetricsNarrative } from '$lib/metrics/metric-view.js';
import type { ProjectSession } from '$lib/types.js';

// Master-detail drill-down for one signal. Reads `?grain=daily|weekly|monthly`
// so the grain toggle is a plain link (the loader re-runs on the search-param
// change and refetches the series at the new grain). The rail needs every
// signal (rows + registry family), so this joins the same surfaces the landing
// does, plus the selected key's series and recent sessions for the period.
type Grain = 'daily' | 'weekly' | 'monthly';
const GRAINS: readonly Grain[] = ['daily', 'weekly', 'monthly'];

export const load: PageLoad = async ({ params, url }) => {
    const api = senseiApi(appState.port);
    const requested = url.searchParams.get('grain') ?? 'daily';
    const grain: Grain = (GRAINS as readonly string[]).includes(requested)
        ? (requested as Grain)
        : 'daily';

    const [metricsRes, registryRes] = await Promise.all([
        api.getProjectMetrics(params.id),
        api.getMetricsRegistry(),
    ]);

    const empty = {
        rows: [],
        registry: [],
        narrative: null as MetricsNarrative | null,
        selectedKey: params.key,
        grain,
        series: [] as MetricSeriesPoint[],
        sessions: [] as ProjectSession[],
    };
    if (!metricsRes.ok) return { ...empty, error: metricsRes.error.message };

    const rows = metricsRes.data.metrics ?? [];
    const registry = registryRes.ok ? (registryRes.data.metrics ?? []) : [];
    const narrative = metricsRes.data.narrative ?? null;

    const [seriesRes, sessionsRes] = await Promise.all([
        api.getProjectMetricSeries(params.id, params.key, grain),
        api.getProjectSessions(params.id, 5),
    ]);

    return {
        rows,
        registry,
        narrative,
        selectedKey: params.key,
        grain,
        series: seriesRes.ok ? seriesRes.data.series : [],
        sessions: sessionsRes.sessions ?? [],
        error: null,
    };
};
