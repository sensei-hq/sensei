// GET /v1/t/{origin}/{org}/team-metrics[?days=N] — the Teams view (#307 R8, D38)
// over the last N days (default 7, at most 90). MEMBER floor: every member reads
// their teams' totals (k-anonymous); a lead also reads the people on their teams,
// an admin every team and its people. The reads run as the caller, so the
// database decides whose numbers come back. Teammates are named through the
// service client (names for ids the caller already read — the access matrix's
// resolveDisplayNames exception). Fails closed.
import { ACCESS } from '$lib/server/dojo-auth';
import { dojoDb } from '$lib/server/dojo-supabase';
import { tenantRoute } from '$lib/server/tenant-route';
import { readTeamMetrics } from '$lib/server/team-metrics-data';
import { windowDays } from '$lib/server/window-days';
import { TEAM_METRICS_DEFAULT_DAYS } from '$lib/team-metrics';

export const GET = tenantRoute(ACCESS.member, async (db, caller, { url }) => {
	const days = windowDays(url, TEAM_METRICS_DEFAULT_DAYS);
	return Response.json(await readTeamMetrics(db, dojoDb(), caller, { days, today: new Date() }));
});
