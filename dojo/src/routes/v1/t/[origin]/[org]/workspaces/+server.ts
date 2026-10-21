// GET /v1/t/{origin}/{org}/workspaces — the org's workspaces (dojo.workspaces,
// tenant-scoped). The JWT-plane console read, LEAD-floor gated, read AS THE
// CALLER: the workspaces policy narrows the tenant's rows to the ones the access
// matrix gives them (own, their teams', every one for an admin). Read-only (the
// daemon upserts rows via the service_role on relay runs). Fails closed.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listOrgWorkspaces, AdminError } from '$lib/server/workspaces-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const workspaces = await listOrgWorkspaces(callerDb(caller), caller.tenantId);
		return Response.json({ workspaces });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
