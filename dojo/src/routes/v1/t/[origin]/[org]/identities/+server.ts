// GET /v1/t/{origin}/{org}/identities — the tenant's identity mappings (admin
// console). The in-Worker port of dojo-mind's `list_identities`, on the JWT
// plane at the ADMIN floor. Returns `{ identities: Identity[] }` — the shape the
// shipped `(console)` client `admin-data.ts listIdentities()` unwraps. Read-only
// this chunk; create / patch / delete are follow-ups. The query lives in
// `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listIdentities, AdminError } from '$lib/server/admin-data';

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
