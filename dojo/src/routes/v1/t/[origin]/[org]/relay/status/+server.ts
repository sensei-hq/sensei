// GET /v1/t/{origin}/{org}/relay/status — THAT runs are going, for the people
// the access matrix lets know it (#252): the caller's own runs, those on a
// project a team of theirs owns, and every run for a tenant admin. Status only —
// relay content stays its owner's (#249). Read as the caller through
// dojo.peer_run_status, which is pinned to their principal.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const { data, error } = await callerDb(caller).rpc('peer_run_status', { p_tenant: caller.tenantId });
		if (error) return apiError(500, error.message);
		return Response.json({ runs: data ?? [] });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
