// The Teams view (#307 R8, D38) — the wire shape and the metrics it shows. Shared
// by the server (which asks the database for these keys) and the view (which
// labels and formats them), so the two cannot drift.

/** One metric the Teams view shows. `read` picks the aggregate that fits it: a
 *  per-day count sums over the window (`total`); a rate is pooled by its own
 *  denominator (`weighted` — Σ numerator / Σ denominator). `as` says how to print it. */
export interface TeamMetricDef {
	key: string;
	label: string;
	read: 'total' | 'weighted';
	as: 'count' | 'percent';
}

/** The key numbers: sessions and AI usage (the catalogue's `throughput` and
 *  `tokens_per_day`), and the two outcome rates a team acts on — `ftr` (per
 *  session) and `rework_ratio` (per tool call), each pooled by its denominator. */
export const TEAM_METRICS: readonly TeamMetricDef[] = [
	{ key: 'throughput', label: 'Sessions', read: 'total', as: 'count' },
	{ key: 'ftr', label: 'First-try', read: 'weighted', as: 'percent' },
	{ key: 'rework_ratio', label: 'Rework share', read: 'weighted', as: 'percent' },
	{ key: 'tokens_per_day', label: 'Tokens', read: 'total', as: 'count' }
];

export const TEAM_METRIC_KEYS: readonly string[] = TEAM_METRICS.map((m) => m.key);

/** One aggregate over the window. `weighted` is null when a row carried no
 *  denominator (a rate that cannot be pooled honestly). */
export interface TeamMetricValue {
	total: number;
	weighted: number | null;
}

/** One person's aggregate of one metric, across the repositories the reader may see. */
export interface TeamMetricAgg extends TeamMetricValue {
	days: number;
	repositories: number;
	last_on: string;
}

/** A team's aggregate of one metric: how many people it covers (never below the
 *  k-anonymity floor — under it, the database answers nothing). */
export interface TeamTotal extends TeamMetricValue {
	contributors: number;
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
	member_count: number;
	/** By metric key; absent under the k-anonymity floor or when nothing was sent. */
	totals: Record<string, TeamTotal>;
	/** Per person — only for a lead or an admin; empty otherwise. */
	members: TeamMemberMetrics[];
}

/** GET /v1/t/{origin}/{org}/team-metrics. */
export interface TeamMetricsWire {
	since: string;
	days: number;
	/** Whether `members` carries per-person rows (lead and admin). */
	per_person: boolean;
	/** The k-anonymity floor the team totals honour. */
	k_min: number;
	teams: TeamMetricsTeam[];
}

/** The window read by default — the mockup's "this week". */
export const TEAM_METRICS_DEFAULT_DAYS = 7;
/** The longest window the route accepts. */
export const TEAM_METRICS_MAX_DAYS = 90;
