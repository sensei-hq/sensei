// GET/POST /v1/t/{origin}/{org}/policies — the tenant's policy grid (admin
// console). The in-Worker port of dojo-mind's `list_policies` / `upsert_policy`,
// on the JWT plane at the ADMIN floor. GET returns `{ policies: Policy[] }`; POST
// creates-or-edits by `scope_key` (upsert) and returns `{ id, scope_key }` — the
// shapes `admin-data.ts` listPolicies()/upsertPolicy() unwrap. Patch/delete-by-id
// live at `…/policies/{id}`. The store logic lives in `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listPolicies, upsertPolicy, parseUpsertPolicy, AdminError } from '$lib/server/admin-data';
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
		const policies = await listPolicies(dojoDb(), tenantId);
		return Response.json({ policies });
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
		const input = parseUpsertPolicy(body);
		const db = dojoDb();
		const result = await upsertPolicy(db, caller.tenantId, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'policy_edited',
			target: result.scope_key
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
