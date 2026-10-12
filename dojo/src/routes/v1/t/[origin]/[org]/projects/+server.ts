// GET /v1/t/{origin}/{org}/projects — the org's projects (dojo.projects,
// tenant-scoped). The JWT-plane console read, LEAD-floor gated, read AS THE
// CALLER: the projects policy narrows the tenant's rows to the ones the access
// matrix gives them (own, their teams', every one for an admin). Read-only (the
// daemon upserts rows via the service_role on relay runs). Fails closed.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listOrgProjects, AdminError } from '$lib/server/projects-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const projects = await listOrgProjects(callerDb(caller), caller.tenantId);
		return Response.json({ projects });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
