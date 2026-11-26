// One repository's metric series over a window (#532): the per-repo read
// behind the repo screen's cards. Scoped through the caller's own
// `my_repositories` rows (the database pins the principal): a repository
// outside the addressed tenant is 404 before any row is touched, and the
// series itself reads `repository_metric_series` as the caller, whose
// invoker policy decides row by row. Fails closed throughout.
import { AdminError, type DojoClient } from './admin-data';
import { readAllPages } from './repositories';
import { windowSince, num } from './team-metrics-data';

export interface RepoMetricPoint {
	repository_id: string;
	metric_key: string;
	scope: string;
	principal_id: string | null;
	day: string;
	value: number;
	props: unknown;
}

export interface RepoMetricsWire {
	since: string;
	days: number;
	rows: RepoMetricPoint[];
}

export async function readRepoMetrics(
	db: DojoClient,
	tenantId: string,
	repoId: string,
	opts: { days: number; today: Date }
): Promise<RepoMetricsWire> {
	const mine = await db.rpc('my_repositories').select('repository_id, tenant_id').eq('repository_id', repoId);
	if (mine.error) throw new AdminError(500, mine.error.message);
	const scoped = ((mine.data ?? []) as Array<{ repository_id: string; tenant_id: string }>).some(
		(r) => r.repository_id === repoId && r.tenant_id === tenantId
	);
	if (!scoped) throw new AdminError(404, 'no such repository in this dōjō');
	const since = windowSince(opts.today, opts.days);
	const rows = await readAllPages<{
		repository_id: string;
		metric_key: string;
		scope: string;
		principal_id: string | null;
		computed_on: string;
		grain: string;
		value: number | string;
		props: unknown;
	}>((from, to) =>
		db
			.rpc('repository_metric_series', { p_repository_id: repoId, p_since: since })
			.select('repository_id, metric_key, scope, principal_id, computed_on, grain, value, props')
			.order('metric_key')
			.order('scope')
			.order('computed_on')
			.range(from, to)
	);
	return {
		since,
		days: opts.days,
		rows: rows.map((r) => ({
			repository_id: r.repository_id,
			metric_key: r.metric_key,
			scope: r.scope,
			principal_id: r.principal_id,
			day: r.computed_on,
			value: num(r.value),
			props: r.props
		}))
	};
}
