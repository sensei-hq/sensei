// GET /v1/you/sync/plan — what the caller may sync this cycle (§V.4, in the
// user-scoped shape of §VIII.1).
//
// The daemon ASKS; it never remembers. An earlier draft cached the ruling on
// `sensei.repositories`, which made a second source of truth for something the
// service must own and forced a TTL whose only job was to bound how wrong the
// cache could be. Asking each cycle means a revoked seat bites on the next one,
// and there is no column that can disagree with the dōjō because no column holds
// the answer.
//
// An ALLOW-LIST, not a per-repo permission check: the daemon syncs the set it is
// handed, so it cannot accidentally include a repository it never asked about —
// which a "may I sync X?" shape permits by omission. Offline therefore degrades
// to no-sync by construction rather than needing a nullable boolean whose NULL
// means no.
//
// Scope: metrics and governance only. Repository IDENTITY is registered
// separately at POST /v1/you/repositories; this is the entitlement filter
// applied on top. Still enforced at the write — the plan stops the daemon
// shipping data that would be refused, and the dōjō re-decides on every write,
// so ignoring the plan gains a daemon nothing.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { syncPlan } from '$lib/server/repositories';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		return Response.json(await syncPlan(db, userId));
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
