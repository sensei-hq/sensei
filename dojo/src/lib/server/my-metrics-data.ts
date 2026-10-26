// A developer's own metrics, day by day (the personal console's chart).
//
// Whose rows come back is the DATABASE's decision: dojo.my_metric_series runs as
// the caller and is bound to their principal. This module asks for the window,
// keeps the Teams view's metrics and shapes one series per metric. Fails closed:
// a failed read is an error, never an empty chart.
import type { DojoClient } from './admin-data';
import { callFn } from './tenant-rows';
import { num, numOrNull, windowSince, type Num } from './team-metrics-data';
import { TEAM_METRIC_KEYS } from '$lib/team-metrics';
import type { MyMetricPoint, MyMetricsWire } from '$lib/my-metrics';

interface SeriesRow {
	metric_key: string;
	computed_on: string;
	total: Num;
	weighted: Num | null;
	denominator: Num | null;
	repositories: number;
}

const SHOWN = new Set(TEAM_METRIC_KEYS);

/** `db` is the caller's client, so the read runs as them. */
export async function readMyMetrics(db: DojoClient, opts: { days: number; today: Date }): Promise<MyMetricsWire> {
	const since = windowSince(opts.today, opts.days);
	const rows = (await callFn<SeriesRow[] | null>(db, 'my_metric_series', { p_since: since })) ?? [];
	const series: Record<string, MyMetricPoint[]> = {};
	for (const r of rows) {
		if (!SHOWN.has(r.metric_key)) continue;
		(series[r.metric_key] ??= []).push({
			day: r.computed_on,
			total: num(r.total),
			weighted: numOrNull(r.weighted),
			denominator: numOrNull(r.denominator)
		});
	}
	for (const points of Object.values(series)) points.sort((a, b) => a.day.localeCompare(b.day));
	return { since, days: opts.days, series };
}
