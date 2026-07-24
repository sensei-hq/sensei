// GET /v1/t/{origin}/{org}/members — the tenant's memberships (admin console).
// The in-Worker port of dojo-mind's `list_memberships`, on the JWT plane at the
// ADMIN floor. Returns `{ members: Membership[] }` — the shape the shipped
// `(console)` client `admin-data.ts listMembers()` unwraps. Read-only this
// chunk; provision / set-role are follow-ups. The query lives in
// `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listMembers, AdminError } from '$lib/server/admin-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const members = await listMembers(dojoDb(), tenantId);
		return Response.json({ members });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
