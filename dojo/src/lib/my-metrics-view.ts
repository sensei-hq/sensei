// The developer console's metric cards, from the GET /v1/you/metrics wire. One
// card per Teams-view metric, so a person's numbers read like their lead's.
import { TEAM_METRICS, type TeamMetricDef } from './team-metrics';
import { formatMetric } from './team-metrics-view';
import type { MyMetricPoint, MyMetricsWire } from './my-metrics';

export interface MyMetricCard {
	key: string;
	label: string;
	/** The window's number: a count summed, a rate pooled by its denominators. */
	value: string;
	/** Day by day, oldest first; a rate's days that cannot be pooled are left out. */
	points: number[];
	/** Nothing was sent for this metric in the window. */
	empty: boolean;
}

export interface MyMetricsView {
	days: number;
	cards: MyMetricCard[];
}

const DASH = '—';

/** A day that carries a pooled rate: its rate and the denominator it pools by. */
function pooledDays(points: MyMetricPoint[]) {
	return points.flatMap((p) =>
		p.weighted !== null && p.denominator !== null ? [{ rate: p.weighted, denominator: p.denominator }] : []
	);
}

function cardFor(def: TeamMetricDef, points: MyMetricPoint[] | undefined): MyMetricCard {
	const base = { key: def.key, label: def.label };
	if (!points?.length) return { ...base, value: DASH, points: [], empty: true };
	if (def.read === 'total') {
		const sum = points.reduce((s, p) => s + p.total, 0);
		return { ...base, value: formatMetric(sum, def.as), points: points.map((p) => p.total), empty: false };
	}
	const days = pooledDays(points);
	const denominator = days.reduce((s, d) => s + d.denominator, 0);
	const value =
		denominator > 0 ? formatMetric(days.reduce((s, d) => s + d.rate * d.denominator, 0) / denominator, def.as) : DASH;
	return { ...base, value, points: days.map((d) => d.rate), empty: false };
}

export function toMyMetricsView(wire: MyMetricsWire): MyMetricsView {
	return { days: wire.days, cards: TEAM_METRICS.map((def) => cardFor(def, wire.series[def.key])) };
}
