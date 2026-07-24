// GET /v1/t/{origin}/{org}/audit?limit=N — the tenant's audit log, most recent
// first (admin console). The in-Worker port of dojo-mind's `list_audit_events`,
// on the JWT plane at the ADMIN floor. Returns `{ events: AuditEvent[] }` — the
// shape the shipped `(console)` client `admin-data.ts listAudit()` unwraps. The
// query + limit clamp live in `$lib/server/admin-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listAudit, AdminError } from '$lib/server/admin-data';

export const GET: RequestHandler = async ({ params, request, locals, url }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const limitRaw = url.searchParams.get('limit');
		const parsed = limitRaw == null ? NaN : Number.parseInt(limitRaw, 10);
		const events = await listAudit(dojoDb(), tenantId, Number.isFinite(parsed) ? parsed : undefined);
		return Response.json({ events });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
