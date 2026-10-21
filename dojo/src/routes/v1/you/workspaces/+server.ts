// GET /v1/you/workspaces — every workspace the caller may read, across every dōjō
// they belong to. Backs the personal `/you/workspaces` list; the `/v1/you` plane
// mirrors the `/you` UI namespace (vs the tenant `/v1/t/{origin}/{org}`). Workspaces
// are shared items (#299): the read runs with the caller's own client and RLS
// (dojo.can_read_workspace — whoever may read one of its repositories) decides the
// rows. Read-only; the sync files workspaces. Fails closed, honest-empty until
// populated; never a fixture.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { listUserWorkspaces, AdminError } from '$lib/server/workspaces-data';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const db = callerDb(caller);
		const workspaces = await listUserWorkspaces(db);
		return Response.json({ workspaces });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
