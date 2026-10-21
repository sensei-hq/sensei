// GET /v1/you/workspaces/{id}/constitution — the resolved constitution for one
// workspace (F4 drill-in), by workspace id: a workspace is a shared item, and its slug
// is unique only within one dōjō (#299). The read runs with the caller's own
// client, so RLS (dojo.can_read_workspace — whoever may read one of its
// repositories) authorizes it. The daemon OWNS resolution; this only reads the
// federated jsonb. `{ constitution: null }` when none is federated yet or the
// caller cannot read the workspace. Fails closed; never a fixture.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { getWorkspaceConstitution, AdminError } from '$lib/server/workspaces-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const db = callerDb(caller);
		const constitution = await getWorkspaceConstitution(db, params.id);
		return Response.json({ constitution });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
