// A developer's own metrics, day by day — the wire shape the personal console
// reads. The metrics are the Teams view's (`TEAM_METRICS`), so a person's card
// and their lead's column read the same keys the same way.

/** One day of one metric, pooled across the person's repositories: `total` sums;
 *  `weighted` is the denominator-pooled rate and `denominator` its Σ, both null
 *  when a row carried no denominator (a rate that cannot be pooled honestly). */
export interface MyMetricPoint {
	day: string;
	total: number;
	weighted: number | null;
	denominator: number | null;
}

export interface MyMetricsWire {
	since: string;
	days: number;
	/** By metric key, oldest day first; a key is absent when nothing was sent. */
	series: Record<string, MyMetricPoint[]>;
}

/** A sparkline needs days: a month by default. The ceiling is the Teams view's. */
export const MY_METRICS_DEFAULT_DAYS = 30;
