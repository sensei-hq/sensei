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
import { AdminError } from '$lib/server/admin-data';
import { readTeamMetrics } from '$lib/server/team-metrics-data';
import { TEAM_METRICS_DEFAULT_DAYS, TEAM_METRICS_MAX_DAYS } from '$lib/team-metrics';

/** The window in days from `?days=`, or a 400. */
function windowDays(url: URL): number {
	const raw = url.searchParams.get('days');
	if (raw === null) return TEAM_METRICS_DEFAULT_DAYS;
	const days = /^\d+$/.test(raw) ? Number(raw) : NaN;
	if (!(days >= 1 && days <= TEAM_METRICS_MAX_DAYS)) {
		throw new AdminError(400, `days must be a whole number from 1 to ${TEAM_METRICS_MAX_DAYS}`);
	}
	return days;
}

export const GET = tenantRoute(ACCESS.member, async (db, caller, { url }) => {
	const days = windowDays(url);
	return Response.json(await readTeamMetrics(db, dojoDb(), caller, { days, today: new Date() }));
});
