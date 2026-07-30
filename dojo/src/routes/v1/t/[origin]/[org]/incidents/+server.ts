// GET/POST /v1/t/{origin}/{org}/incidents — the tenant's confidentiality
// incidents (lead console). The in-Worker port of dojo-mind's `list_incidents` /
// `open_incident`, on the JWT plane at the LEAD floor. GET returns
// `{ incidents: Incident[], open_count }` (worst-severity first); POST opens an
// incident and returns `{ id, severity }` — the shapes `client-data.ts`
// listIncidents()/createIncident() unwrap. Patch/delete-by-id live at
// `…/incidents/{id}`. The store logic lives in `$lib/server/incidents-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listIncidents, createIncident, parseNewIncident, IncidentsError } from '$lib/server/incidents-data';
import { resolveEngagementClientNames } from '$lib/server/engagement-client-names';
import { AdminError } from '$lib/server/admin-data';
import { recordAudit } from '$lib/server/audit';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.lead
		);
		const db = dojoDb();
		const { incidents, open_count } = await listIncidents(db, tenantId);
		// Resolve each incident's engagement → client name (Rule C); the row
		// otherwise carries only the engagement uuid (rendered as a short id).
		const names = await resolveEngagementClientNames(
			db,
			tenantId,
			incidents.map((i) => i.engagement_id).filter((x): x is string => typeof x === 'string')
		);
		const enriched = incidents.map((i) => ({
			...i,
			client_name: i.engagement_id ? (names.get(i.engagement_id) ?? null) : null
		}));
		return Response.json({ incidents: enriched, open_count });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof IncidentsError) return apiError(e.status, e.message);
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
			ACCESS.lead
		);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const input = parseNewIncident(body);
		const db = dojoDb();
		const result = await createIncident(db, caller.tenantId, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'incident_opened',
			target: result.id,
			detail: { severity: result.severity, title: input.title },
			engagementId: input.engagement_id ?? null
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof IncidentsError) return apiError(e.status, e.message);
		throw e;
	}
};
