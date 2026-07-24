// PATCH/DELETE /v1/t/{origin}/{org}/policies/{id} — update or remove a policy by
// id. The in-Worker port of dojo-mind's `patch_policy` / `delete_policy`, on the
// JWT/console plane at the ADMIN floor. PATCH returns `{ id }`; DELETE returns
// `{ deleted: true }` (404 when no tenant policy matches) — the shapes
// `admin-data.ts` patchPolicy()/deletePolicy() unwrap. Every write audits to
// `dojo.audit_events`. The store logic lives in `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { patchPolicy, deletePolicy, parsePatchPolicy, AdminError } from '$lib/server/admin-data';
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
		const input = parsePatchPolicy(body);
		const db = dojoDb();
		const result = await patchPolicy(db, caller.tenantId, params.id, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'policy_edited',
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
		const deleted = await deletePolicy(db, caller.tenantId, params.id);
		if (!deleted) return apiError(404, 'no such policy');
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'policy_deleted',
			target: params.id
		});
		return Response.json({ deleted: true });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
