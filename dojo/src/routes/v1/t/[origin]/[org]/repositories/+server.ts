// GET /v1/t/{origin}/{org}/repositories — the org's repositories with member
// counts + the hidden-private count (billing tab), on the JWT plane at the
// ADMIN floor. Returns `{ repositories, hidden_private_count }` — the count
// is counted here from the rows, never stored.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listOrgRepositories, AdminError } from '$lib/server/admin-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		// The org tab does not exist on personal tenants — and access would
		// resolve them (their keys exist), so the branch is the route's own,
		// before auth: this address serves orgs only.
		if (params.origin === 'personal') return apiError(404, 'no such dōjō');
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const repositories = await listOrgRepositories(callerDb(caller), caller.tenantId);
		const hidden_private_count = repositories.filter((r) => r.billing_hidden).length;
		return Response.json({ repositories, hidden_private_count });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
