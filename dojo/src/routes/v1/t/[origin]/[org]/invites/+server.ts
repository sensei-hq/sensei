// POST /v1/t/{origin}/{org}/invites — an admin issues a magic-link membership
// invite for their tenant (F3b). ADMIN floor (resolveTenantAccess). Body:
// { email, role?, kind }. Returns the invite incl. the single-use `token` the
// admin shares as an accept link. Delivery (email) is a follow-on; the token is
// the mechanism. Fails closed; never a fabricated invite.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { createInvite, parseNewInvite, AdminError } from '$lib/server/invites-data';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const input = parseNewInvite(body);
		const invite = await createInvite(dojoDb(), caller.tenantId, caller.userId, input, Date.now());
		return Response.json(invite, { status: 201 });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
