// GET /v1/t/{origin}/{org}/workspaces — the org's workspaces (dojo.workspaces,
// tenant-scoped). The JWT-plane console read for any member, read AS THE CALLER:
// the workspaces policy (dojo.can_read_workspace) narrows the tenant's rows to
// the ones the access matrix gives them — a workspace holding a repository they
// hold a role on, every one for an admin; plain membership reads none (#299).
// No role floor above member: a contributor with a repository role must reach
// the policy. Read-only (the sync files rows). Fails closed.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listOrgWorkspaces, AdminError } from '$lib/server/workspaces-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const workspaces = await listOrgWorkspaces(callerDb(caller), caller.tenantId);
		return Response.json({ workspaces });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
