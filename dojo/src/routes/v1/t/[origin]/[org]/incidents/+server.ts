// GET /v1/t/{origin}/{org}/incidents — the tenant's confidentiality incidents
// (lead console), worst-severity first, with the open-count done-gate. The
// in-Worker port of dojo-mind's `list_incidents`, on the JWT plane at the LEAD
// floor. Returns `{ incidents: Incident[], open_count }` — the shape the shipped
// `(console)` client `client-data.ts listIncidents()` unwraps. Read-only this
// chunk; open / patch are follow-ups. The ordering + open-count live in
// `$lib/server/incidents-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listIncidents, IncidentsError } from '$lib/server/incidents-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.lead
		);
		const { incidents, open_count } = await listIncidents(dojoDb(), tenantId);
		return Response.json({ incidents, open_count });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof IncidentsError) return apiError(e.status, e.message);
		throw e;
	}
};
