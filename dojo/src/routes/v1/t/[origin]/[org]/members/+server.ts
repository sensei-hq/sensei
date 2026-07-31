// GET/POST /v1/t/{origin}/{org}/members — the tenant's memberships (admin
// console). The in-Worker port of dojo-mind's `list_memberships` /
// `add_membership`, on the JWT plane at the ADMIN floor. GET returns
// `{ members: Membership[] }`; POST provisions a membership and returns
// `{ id, role }` — the shapes the `(console)` client `admin-data.ts`
// listMembers()/addMember() unwrap. Set-role lives at `…/members/{userId}/role`.
// The store logic lives in `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listMembers, addMember, parseNewMember, AdminError } from '$lib/server/admin-data';
import { recordAudit } from '$lib/server/audit';

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
		const input = parseNewMember(body);
		const db = dojoDb();
		// Rule C: the dōjō url is derived from the membership's tenant
		// (`dojo.tenants.dojo_url`), not stored per membership.
		const result = await addMember(db, caller.tenantId, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'member_added',
			target: input.user_id,
			detail: { role: result.role, kind: input.kind }
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
