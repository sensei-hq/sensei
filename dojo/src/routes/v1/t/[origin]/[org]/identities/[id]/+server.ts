// PATCH/DELETE /v1/t/{origin}/{org}/identities/{id} — update an identity's email
// / display name, or remove it. The in-Worker port of dojo-mind's
// `update_identity` / `delete_identity`, on the JWT/console plane at the ADMIN
// floor. PATCH returns `{ id }`; DELETE returns `{ deleted: true }` (404 when no
// tenant identity matches) — the shapes `admin-data.ts`
// updateIdentity()/deleteIdentity() unwrap. Every write audits to
// `dojo.audit_events`. The store logic lives in `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { updateIdentity, deleteIdentity, parsePatchIdentity, AdminError } from '$lib/server/admin-data';
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
		const input = parsePatchIdentity(body);
		const db = dojoDb();
		const result = await updateIdentity(db, caller.tenantId, params.id, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'identity_updated',
			target: result.id
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};

export const DELETE: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const db = dojoDb();
		const deleted = await deleteIdentity(db, caller.tenantId, params.id);
		if (!deleted) return apiError(404, 'no such identity');
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'identity_removed',
			target: params.id
		});
		return Response.json({ deleted: true });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
