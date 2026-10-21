import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { densifySeries, type MetricsNarrative } from '$lib/metrics/metric-view.js';
import type { ProjectHealth } from '$lib/metrics/health-radar.js';
import type { MetricCorrelation } from '$lib/metrics/correlation-view.js';
import { historyOffNotice } from '$lib/metrics/history-notice.js';

// The metrics pane joins three daemon surfaces: the per-project *values*
// (/metrics) — which now also carry an optional daemon-generated `narrative`
// (headline + per-signal insights, from the local ollama narration-cache pipeline;
// absent when the model is unavailable) — the *catalog* (/metrics/registry) that
// alone carries each metric's `family`, and a per-metric series for sparklines.
export const load: PageLoad = async ({ params }) => {
    const api = senseiApi(appState.port);
    const [metricsRes, registryRes, healthRes, corrRes, consentRes] = await Promise.all([
        api.getProjectMetrics(params.id),
        api.getMetricsRegistry(),
        api.getProjectHealth(params.id),
        // Portfolio-wide: per project the paired-day count rarely clears the
        // daemon's n>=20 gate. Additive — a failure just omits the section.
        api.getMetricCorrelations(),
        // Which assistants' history is read (#280). Additive: a failed read omits
        // the notice rather than claiming reading is off.
        api.tryGetTranscriptConsent(),
    ]);
    const history = historyOffNotice(consentRes.ok ? consentRes.data : null);

    // A fetch FAILURE surfaces as an error state — never an empty grid that
    // hides a broken daemon (no-fabrication). Honest-empty (a project with no
    // computed metrics yet) returns error: null with an empty rows array.
    // The registry alone carries each metric's family, which the grid groups by;
    // an unread one is not an empty catalogue (#288).
    const unread = (error: string) => ({
        rows: [],
        registry: [],
        series: {} as Record<string, (number | null)[]>,
        narrative: null as MetricsNarrative | null,
        health: null as ProjectHealth | null,
        correlations: [] as MetricCorrelation[],
        history,
        error,
    });
    if (!metricsRes.ok) return unread(metricsRes.error.message);
    if (!registryRes.ok) return unread(registryRes.error.message);

    const rows = metricsRes.data.metrics ?? [];
    const registry = registryRes.data.metrics ?? [];
    const narrative = metricsRes.data.narrative ?? null;

    // One series per metric, fetched in parallel. A missing/failed series just
    // omits that card's sparkline — the value + trend still render. The daemon
    // series is sparse (present periods only), so densify to per-day slots with
    // `null` for absent days: the sparkline breaks the line at a gap rather than
    // connecting or zero-filling across it.
    const seriesPairs = await Promise.all(
        rows.map(async (row) => {
            const res = await api.getProjectMetricSeries(params.id, row.metric, 'daily');
            const points = res.ok ? densifySeries(res.data.series, 'daily').map((p) => p.value) : [];
            return [row.metric, points] as const;
        }),
    );

    const series: Record<string, (number | null)[]> = Object.fromEntries(seriesPairs);

    // Health is additive to this screen: a project the daemon has rated nothing
    // for legitimately has no row (404 -> honest-empty, spec I4), and a daemon
    // hiccup should not blank the metric grid that loaded fine. Either way the
    // radar renders its own quiet state rather than a fabricated zero score.
    const health = healthRes.ok ? healthRes.data : null;

    // Only pairs whose BOTH metrics exist on this screen — a portfolio finding
    // about metrics this project never computes would be noise here.
    const present = new Set(rows.map((r) => r.metric));
    const correlations: MetricCorrelation[] = (corrRes.ok ? corrRes.data.correlations : []).filter(
        (c) => present.has(c.a) && present.has(c.b),
    );

    return { rows, registry, series, narrative, health, correlations, history, error: null };
};
