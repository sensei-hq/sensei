// The lead's team view (#307 R8, D38) — the wire shape and the metrics it shows.
// Shared by the server (which asks the database for these keys) and the view
// (which labels and formats them), so the two cannot drift.

/** One metric the team view shows. `read` picks the aggregate that fits it: a
 *  per-day count sums over the window, a rate averages. `as` says how to print it. */
export interface TeamMetricDef {
	key: string;
	label: string;
	read: 'total' | 'mean';
	as: 'count' | 'percent';
	/** Which way is better, for the reader — never a judgement the view colours. */
	sub: string;
}

/** The key per-person numbers a lead reads: sessions and AI usage (the sensei
 *  catalogue's `throughput` and `tokens_per_day`), and the two outcome signals a
 *  team acts on (`ftr`, `rework_ratio`). */
export const TEAM_METRICS: readonly TeamMetricDef[] = [
	{ key: 'throughput', label: 'Sessions', read: 'total', as: 'count', sub: 'in the window' },
	{ key: 'ftr', label: 'First-try', read: 'mean', as: 'percent', sub: 'mean of daily rates' },
	{ key: 'rework_ratio', label: 'Rework share', read: 'mean', as: 'percent', sub: 'mean of daily rates' },
	{ key: 'tokens_per_day', label: 'Tokens', read: 'total', as: 'count', sub: 'in the window' }
];

export const TEAM_METRIC_KEYS: readonly string[] = TEAM_METRICS.map((m) => m.key);

/** One person's aggregate of one metric over the window, across the repositories
 *  the reader may see. */
export interface TeamMetricAgg {
	total: number;
	mean: number;
	days: number;
	repositories: number;
	last_on: string;
}

export interface TeamMemberMetrics {
	principal_id: string;
	display_name: string | null;
	email: string | null;
	/** The caller themselves. */
	you: boolean;
	/** By metric key; a key is absent when nothing the reader may see was sent. */
	metrics: Record<string, TeamMetricAgg>;
}

export interface TeamMetricsTeam {
	id: string;
	name: string;
	members: TeamMemberMetrics[];
}

/** GET /v1/t/{origin}/{org}/team-metrics. */
export interface TeamMetricsWire {
	since: string;
	days: number;
	teams: TeamMetricsTeam[];
}

/** The window a lead reads by default — the mockup's "this week". */
export const TEAM_METRICS_DEFAULT_DAYS = 7;
/** The longest window the route accepts. */
export const TEAM_METRICS_MAX_DAYS = 90;
