// GET /v1/t/{origin}/{org}/members — the tenant's memberships (admin console),
// on the JWT plane at the ADMIN floor. Returns `{ members: Membership[] }`.
// Set-role lives at `…/members/{userId}/role`.
//
// No POST: a person joins a dōjō by accepting an INVITE — their consent is the
// invite token. An admin adding an arbitrary principal id put someone in a
// tenant without it (#236, access matrix "Tenancy, identity, billing").
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listMembers, AdminError } from '$lib/server/admin-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const members = await listMembers(callerDb(caller), caller.tenantId);
		return Response.json({ members });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
