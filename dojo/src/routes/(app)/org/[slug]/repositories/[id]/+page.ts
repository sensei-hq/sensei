import type { PageLoad } from './$types';
import { loadRepoContext } from '$lib/repo-context';
import { getRepoMetrics } from '$lib/client-data';
import { toWorkspaceMetricsView, type WorkspaceMetricsView } from '$lib/workspace-metrics-view';

// The org repository loader (#532): the shared repo context plus the repo's
// metric series over the window (?days= when whole, the server default
// otherwise), shaped by the shared cards view. A failed metrics read is
// carried as an error, never as empty charts.
export const load: PageLoad = async ({ parent, params, url, fetch }) => {
	const { memberships, accessToken } = await parent();
	const ctx = await loadRepoContext({
		slug: params.slug,
		id: params.id,
		branch: url.searchParams.get('branch'),
		memberships,
		accessToken,
		fetch
	});
	const rawDays = url.searchParams.get('days');
	const days = rawDays !== null && /^\d+$/.test(rawDays) ? Number(rawDays) : undefined;
	let metricsView: WorkspaceMetricsView | null = null;
	let metricsError: string | null = null;
	try {
		const wire = await getRepoMetrics(ctx.tenantKey, ctx.repo.id, {
			fetch,
			accessToken,
			...(days === undefined ? {} : { days })
		});
		metricsView = toWorkspaceMetricsView({
			...wire,
			activity: [{ repository_id: ctx.repo.id, last_activity_at: ctx.repo.last_activity_at }]
		});
	} catch (e) {
		metricsError = e instanceof Error ? e.message : 'the request failed';
	}
	return { ...ctx, metricsView, metricsError };
};
