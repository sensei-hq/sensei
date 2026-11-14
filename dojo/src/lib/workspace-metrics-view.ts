// The workspace metrics view (WP5): per-repository cards from the GET
// …/workspaces/{id}/metrics wire. One card per Teams-view metric per
// repository, so a workspace's numbers read like its teams' and its people's.
// Pure: the wire in, the cards out.
//
// A day reads ONE source, in this order — the scopes are alternative framings
// of the same measurement, never additive: the pooled row when present (a
// maintainer who also views gets detail AND the pool over the same people, and
// counting both would double them); else the user detail rows; else the repo
// rows (the daemon's whole-tree twin, which also fills repo-only metrics).
// Counts sum; rates pool by denominators (Σ numerator / Σ denominator); an
// absent or unpoolable value is a dash, never a zero.
import { TEAM_METRICS, type TeamMetricDef } from './team-metrics';
import { formatMetric } from './team-metrics-view';
import type { WorkspaceMetricsWire, WorkspaceMetricPoint } from './workspace-metrics-data';

export interface WorkspaceMetricCard {
	key: string;
	label: string;
	/** The window's number: a count summed, a rate pooled by its denominators. */
	value: string;
	/** Day by day, oldest first; a rate's days that cannot be pooled are left out. */
	points: number[];
	/** Nothing was sent for this metric in the window. */
	empty: boolean;
}

export interface WorkspaceRepoMetrics {
	repository_id: string;
	repo_key: string;
	/** The key without its forge host (`acme/api`), for the card heading. */
	name: string;
	cards: WorkspaceMetricCard[];
}

export interface WorkspaceMetricsView {
	since: string;
	days: number;
	/** By repository, sorted by key; empty when nothing was sent, never an error. */
	repos: WorkspaceRepoMetrics[];
}

const DASH = '—';

/** A JSON number that may have crossed as text; null when it is no number. */
function numOrNull(v: unknown): number | null {
	const n = typeof v === 'number' ? v : Number(v);
	return Number.isFinite(n) ? n : null;
}

/** One day's reading: its total and, when poolable, its rate numerator/denominator. */
interface DayReading {
	total: number;
	numerator: number | null;
	denominator: number | null;
}

/** A pooled row's pooling, straight from its props. */
function pooledReading(r: WorkspaceMetricPoint): DayReading {
	const props = (r.props ?? {}) as Record<string, unknown>;
	const numerator = numOrNull(props['numerator']);
	const denominator = numOrNull(props['denominator']);
	return {
		total: r.value,
		numerator,
		denominator: numerator !== null && denominator !== null && denominator > 0 ? denominator : null
	};
}

/** Detail rows pooled the way the database pools them: the rate stands only
 *  when EVERY row carries a positive denominator (never an unweighted mean). */
function detailReading(rows: WorkspaceMetricPoint[]): DayReading {
	const dens = rows.map((r) => numOrNull(((r.props ?? {}) as Record<string, unknown>)['denominator']));
	const poolable = dens.every((d) => d !== null && d > 0);
	return {
		total: rows.reduce((s, r) => s + r.value, 0),
		numerator: poolable ? rows.reduce((s, r, i) => s + r.value * (dens[i] as number), 0) : null,
		denominator: poolable ? (dens as number[]).reduce((s, d) => s + d, 0) : null
	};
}

/** One day, one source: the pool when present, else the user detail, else the
 *  repo twin. Returns null only for a day with no rows, which cannot happen. */
function dayReading(rows: WorkspaceMetricPoint[]): DayReading | null {
	const pooled = rows.find((r) => r.scope === 'pooled');
	if (pooled) return pooledReading(pooled);
	const users = rows.filter((r) => r.scope === 'user');
	if (users.length > 0) return detailReading(users);
	const repos = rows.filter((r) => r.scope === 'repo');
	if (repos.length > 0) return detailReading(repos);
	return null;
}

function cardFor(def: TeamMetricDef, days: { day: string; reading: DayReading }[]): WorkspaceMetricCard {
	const base = { key: def.key, label: def.label };
	if (days.length === 0) return { ...base, value: DASH, points: [], empty: true };
	if (def.read === 'total') {
		const sum = days.reduce((s, d) => s + d.reading.total, 0);
		return { ...base, value: formatMetric(sum, def.as), points: days.map((d) => d.reading.total), empty: false };
	}
	const pooled = days.flatMap((d) =>
		d.reading.numerator !== null && d.reading.denominator !== null
			? [{ rate: d.reading.numerator / d.reading.denominator, denominator: d.reading.denominator }]
			: []
	);
	const denominator = pooled.reduce((s, d) => s + d.denominator, 0);
	const value =
		denominator > 0
			? formatMetric(pooled.reduce((s, d) => s + d.rate * d.denominator, 0) / denominator, def.as)
			: DASH;
	return { ...base, value, points: pooled.map((d) => d.rate), empty: false };
}

function shortName(repoKey: string): string {
	const parts = repoKey.split('/').filter(Boolean);
	return parts.length > 1 ? parts.slice(-2).join('/') : repoKey;
}

export function toWorkspaceMetricsView(wire: WorkspaceMetricsWire): WorkspaceMetricsView {
	const byRepo = new Map<string, { repo_key: string; rows: WorkspaceMetricPoint[] }>();
	for (const r of wire.rows) {
		const mine = byRepo.get(r.repository_id) ?? { repo_key: r.repo_key, rows: [] };
		mine.rows.push(r);
		byRepo.set(r.repository_id, mine);
	}
	const repos = [...byRepo.entries()]
		.sort(([, a], [, b]) => (a.repo_key < b.repo_key ? -1 : a.repo_key > b.repo_key ? 1 : 0))
		.map(([repository_id, { repo_key, rows }]) => ({
			repository_id,
			repo_key,
			name: shortName(repo_key),
			cards: TEAM_METRICS.map((def) => {
				const byDay = new Map<string, WorkspaceMetricPoint[]>();
				for (const r of rows.filter((x) => x.metric_key === def.key)) {
					byDay.set(r.day, [...(byDay.get(r.day) ?? []), r]);
				}
				const days = [...byDay.entries()]
					.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
					.flatMap(([day, rs]) => {
						const reading = dayReading(rs);
						return reading ? [{ day, reading }] : [];
					});
				return cardFor(def, days);
			})
		}));
	return { since: wire.since, days: wire.days, repos };
}
