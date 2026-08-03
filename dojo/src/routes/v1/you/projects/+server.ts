// GET /v1/you/projects — the caller's projects across EVERY dōjō they belong to
// (user-primary). Backs the personal `/you/projects` list; the `/v1/you` plane
// mirrors the `/you` UI namespace (vs the tenant `/v1/t/{origin}/{org}`). Unlike
// the tenant-scoped /v1/t/…/projects (LEAD-floor), this has no tenant and no role
// floor: the caller is resolved from the JWT and the read is authorized by a
// `user_id` filter (a caller only ever sees their own rows). Read-only — the
// daemon upserts rows via the service_role on relay runs. Fails closed, honest-
// empty until populated; never a fixture.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { listUserProjects, AdminError } from '$lib/server/projects-data';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const projects = await listUserProjects(db, userId);
		return Response.json({ projects });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
