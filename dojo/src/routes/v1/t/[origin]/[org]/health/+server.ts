// GET /v1/t/{origin}/{org}/health — the admin health strip (read-only). The
// in-Worker port of dojo-mind's `health_rollup`, on the JWT plane at the ADMIN
// floor, aggregated over `dojo.relay_sessions` (heartbeat), `dojo.triage_queue`
// (queue depth), `dojo.audit_events` (publish rate) and `dojo.memberships`
// (sync errors). Returns the bare `HealthRollup` the shipped `(console)` client
// `admin-data.ts getHealth()` reads. The aggregation lives in
// `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { getHealth, AdminError } from '$lib/server/admin-data';
import { getContribVsApprove } from '$lib/server/health-series';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const db = dojoDb();
		// The 4-count rollup + the contributions-vs-approvals weekly series (was an
		// empty chart). Independent reads composed into one response.
		const [rollup, contrib_vs_approve] = await Promise.all([
			getHealth(db, tenantId),
			getContribVsApprove(db, tenantId)
		]);
		return Response.json({ ...rollup, contrib_vs_approve });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
