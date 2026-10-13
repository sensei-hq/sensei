// GET /v1/you/memberships — the signed-in person's own active memberships,
// with what the daemon records of each. Signing in is joining (#236 step 4e):
// the daemon creates its local membership from this and binds it to the
// persona whose session its tenant calls present. Read as the person; fails
// closed, never a fabricated empty list.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller } from '$lib/server/dojo-auth';
import { listMyMemberships } from '$lib/server/dojo-orgs';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		return Response.json({ memberships: await listMyMemberships(callerDb(caller), caller.userId) });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
