// PATCH /v1/t/{origin}/{org}/members/{userId}/role — override a member's dōjō
// role. The in-Worker port of dojo-mind's `set_member_role`, on the JWT/console
// plane at the ADMIN floor. Validates the role (400 on a bad value), updates
// `dojo.memberships.role` for `userId`, audits the change to `dojo.audit_events`,
// and returns `{ user_id, role }` (the shape `admin-data.ts setMemberRole()`
// unwraps). 404 when no membership matches. The store logic lives in
// `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { setMemberRole, AdminError } from '$lib/server/admin-data';
import { recordAudit } from '$lib/server/audit';

export const PATCH: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const db = dojoDb();
		const result = await setMemberRole(db, caller.tenantId, params.userId, body.role);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'role_changed',
			target: result.user_id,
			detail: { role: result.role }
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
