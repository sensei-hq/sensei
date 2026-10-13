// GET /v1/you/memberships — the signed-in person's own active memberships:
// id, tenant key, role, kind. The daemon matches its local memberships' ids
// against this to record which persona each belongs to (#236 step 4c), so its
// tenant calls can present that persona's session. Read as the person; fails
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
