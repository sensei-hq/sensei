import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { MetricSeriesPoint, MetricsNarrative, ToolUsage } from '$lib/metrics/metric-view.js';
import type { Recommendation } from '$lib/types.js';

// Master-detail drill-down for one signal. Reads `?grain=daily|weekly|monthly`
// so the grain toggle is a plain link (the loader re-runs on the search-param
// change and refetches the series at the new grain). The rail needs every
// signal (rows + registry family), so this joins the same surfaces the landing
// does, plus the selected key's series (at the toggled grain, for the chart) and
// its DAILY series (for the day-scoped datapoint drill-down).
type Grain = 'daily' | 'weekly' | 'monthly';
const GRAINS: readonly Grain[] = ['daily', 'weekly', 'monthly'];

export const load: PageLoad = async ({ params, url }) => {
    const api = senseiApi(appState.port);
    const requested = url.searchParams.get('grain') ?? 'daily';
    const grain: Grain = (GRAINS as readonly string[]).includes(requested)
        ? (requested as Grain)
        : 'daily';

    const [metricsRes, registryRes, recsRes, toolsRes] = await Promise.all([
        api.getProjectMetrics(params.id),
        api.getMetricsRegistry(),
        // Pending recommendations → the project-scoped "action items" panel.
        // Result-based so a recommendations-ONLY failure (this leg 500s while
        // /metrics succeeds) surfaces as recommendationsError → an error STATE,
        // never "No action items yet" masking a failure (fail-closed governance).
        api.tryGetProjectRecommendations(params.id, 'pending'),
        // Per-tool usage for the tool-usage bubble view (honest-empty on failure).
        api.getProjectTools(params.id),
    ]);

    const empty = {
        rows: [],
        registry: [],
        narrative: null as MetricsNarrative | null,
        selectedKey: params.key,
        grain,
        series: [] as MetricSeriesPoint[],
        seriesError: null as string | null,
        formula: null as string | null,
        dailySeries: [] as MetricSeriesPoint[],
        dailySeriesError: null as string | null,
        recommendations: [] as Recommendation[],
        recommendationsError: null as string | null,
        tools: [] as ToolUsage[],
    };
    if (!metricsRes.ok) return { ...empty, error: metricsRes.error.message };

    const rows = metricsRes.data.metrics ?? [];
    const registry = registryRes.ok ? (registryRes.data.metrics ?? []) : [];
    const narrative = metricsRes.data.narrative ?? null;

    // The chart reads the series at the toggled grain; the datapoint drill-down
    // is day-scoped, so it always needs the DAILY series (its per-day `explainer`
    // + the days that have data). They're the same request when the grain is
    // already daily — only fetch the daily series separately on a coarser grain.
    const [seriesRes, dailyRes] = await Promise.all([
        api.getProjectMetricSeries(params.id, params.key, grain),
        grain === 'daily'
            ? Promise.resolve(null)
            : api.getProjectMetricSeries(params.id, params.key, 'daily'),
    ]);

    // NEVER-FABRICATE: distinguish a failed FETCH from a genuinely-empty 200.
    // The primary series feeds the chart; the daily series feeds the drill-down;
    // recommendations feed the action items. A failed fetch of any of them
    // surfaces as an explicit *Error state (the chart-error line / the drill-down
    // error / the action-items error), never a masked empty chart / "no sessions"
    // / "no action items". A genuinely-empty 200 stays honest-empty.
    const series = seriesRes.ok ? seriesRes.data.series : [];
    const seriesError = seriesRes.ok ? null : seriesRes.error.message;
    const dailySeries = grain === 'daily' ? series : dailyRes && dailyRes.ok ? dailyRes.data.series : [];
    // The drill-down reads the DAILY series: on `daily` that's `seriesRes`; on a
    // coarser grain it's the separate `dailyRes`. Point dailySeriesError at
    // whichever actually feeds it.
    const dailySeriesError =
        grain === 'daily'
            ? seriesError
            : dailyRes && !dailyRes.ok
              ? dailyRes.error.message
              : null;
    const recommendations = recsRes.ok ? recsRes.data : [];
    const recommendationsError = recsRes.ok ? null : recsRes.error.message;

    return {
        rows,
        registry,
        narrative,
        selectedKey: params.key,
        grain,
        series,
        seriesError,
        // The metric's `formula` is a definition-level facet — read it from the
        // registry catalog (which always carries it) rather than the series, so
        // the "about this metric" block is complete regardless of the series.
        formula: registry.find((r) => r.key === params.key)?.formula ?? null,
        dailySeries,
        dailySeriesError,
        recommendations,
        recommendationsError,
        tools: toolsRes.tools ?? [],
        error: null,
    };
};
