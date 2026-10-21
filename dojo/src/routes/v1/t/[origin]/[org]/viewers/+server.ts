// GET /v1/t/{origin}/{org}/viewers — the dōjō's viewers (#307 A3, D8): people
// invited in as viewers, not members, each with the projects granted to them.
// ADMIN floor; read with the caller's own JWT so RLS decides. Fails closed.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listDojoViewers, AdminError } from '$lib/server/viewers-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.admin);
		return Response.json({ viewers: await listDojoViewers(callerDb(caller), caller.tenantId) });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
