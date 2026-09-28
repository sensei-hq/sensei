// GET/POST /v1/t/{origin}/{org}/identities — the tenant's identity mappings
// (admin console). The in-Worker port of dojo-mind's `list_identities` /
// `add_identity`, on the JWT plane at the ADMIN floor. GET returns
// `{ identities: Identity[] }`; POST wires an identity provider and returns
// `{ id }` — the shapes `admin-data.ts` listIdentities()/createIdentity() unwrap.
// Patch/delete-by-id live at `…/identities/{id}`. The store logic lives in
// `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listIdentities, createIdentity, parseNewIdentity, AdminError } from '$lib/server/admin-data';
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
		const identities = await listIdentities(dojoDb(), tenantId);
		return Response.json({ identities });
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
		const input = parseNewIdentity(body);
		const db = dojoDb();
		const result = await createIdentity(db, caller.tenantId, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'identity_added',
			target: input.subject,
			detail: { provider: input.provider, principal_id: input.principal_id }
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
