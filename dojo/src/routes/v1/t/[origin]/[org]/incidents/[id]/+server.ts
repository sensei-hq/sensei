// PATCH/DELETE /v1/t/{origin}/{org}/incidents/{id} — update / resolve / reopen an
// incident, or delete it. The in-Worker port of dojo-mind's `patch_incident` /
// `delete_incident`, on the JWT/console plane at the LEAD floor. PATCH returns
// `{ id }` (`resolved: true` or `status: 'resolved'` stamps resolved_at; an
// explicit open|investigating reopens); DELETE returns `{ deleted: true }` (404
// when no tenant incident matches) — the shapes `client-data.ts`
// updateIncident()/deleteIncident() unwrap. Every write audits to
// `dojo.audit_events`. The store logic lives in `$lib/server/incidents-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import {
	getIncidentDetail,
	updateIncident,
	deleteIncident,
	parsePatchIncident,
	IncidentsError
} from '$lib/server/incidents-data';
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
		const detail = await getIncidentDetail(dojoDb(), tenantId, params.id);
		return Response.json(detail);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof IncidentsError) return apiError(e.status, e.message);
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};

export const PATCH: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.lead
		);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const input = parsePatchIncident(body);
		const db = dojoDb();
		const result = await updateIncident(db, caller.tenantId, params.id, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: input.resolve ? 'incident_resolved' : 'incident_updated',
			target: result.id,
			detail: { status: input.status ?? null }
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof IncidentsError) return apiError(e.status, e.message);
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
			ACCESS.lead
		);
		const db = dojoDb();
		const deleted = await deleteIncident(db, caller.tenantId, params.id);
		if (!deleted) return apiError(404, 'no such incident');
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'incident_deleted',
			target: params.id
		});
		return Response.json({ deleted: true });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof IncidentsError) return apiError(e.status, e.message);
		throw e;
	}
};
