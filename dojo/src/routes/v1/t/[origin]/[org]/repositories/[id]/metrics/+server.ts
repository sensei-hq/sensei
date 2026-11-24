// GET /v1/t/{origin}/{org}/repositories/[id]/metrics[?days=N] — one
// repository's daily metric series (#532), the repo screen's numbers. MEMBER
// floor: the read runs as the caller, so `repository_metric_series`' invoker
// policy decides row by row; a repository outside this dōjō is 404 before
// any row is touched. Fails closed.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute } from '$lib/server/tenant-route';
import { readRepoMetrics } from '$lib/server/repo-metrics-data';
import { windowDays } from '$lib/server/window-days';
import { TEAM_METRICS_DEFAULT_DAYS } from '$lib/team-metrics';

export const GET = tenantRoute(ACCESS.member, async (db, caller, { url, params }) => {
	const days = windowDays(url, TEAM_METRICS_DEFAULT_DAYS);
	return Response.json(
		await readRepoMetrics(db, caller.tenantId, params.id as string, { days, today: new Date() })
	);
});
