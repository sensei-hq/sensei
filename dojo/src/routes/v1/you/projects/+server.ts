// GET /v1/you/projects — every project the caller may read, across every dōjō
// they belong to. Backs the personal `/you/projects` list; the `/v1/you` plane
// mirrors the `/you` UI namespace (vs the tenant `/v1/t/{origin}/{org}`). Projects
// are shared items (#299): the read runs with the caller's own client and RLS
// (dojo.can_read_project — whoever may read one of its repositories) decides the
// rows. Read-only; the sync files projects. Fails closed, honest-empty until
// populated; never a fixture.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { listUserProjects, AdminError } from '$lib/server/projects-data';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const db = callerDb(caller);
		const projects = await listUserProjects(db);
		return Response.json({ projects });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
