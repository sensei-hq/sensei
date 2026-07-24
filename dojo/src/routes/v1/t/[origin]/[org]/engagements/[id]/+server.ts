// PATCH/DELETE /v1/t/{origin}/{org}/engagements/{id} — edit / close
// (`status: 'ended'`) or hard-delete an engagement. The in-Worker port of
// dojo-mind's `patch_engagement` / `delete_engagement`, on the JWT/console plane
// at the LEAD floor. PATCH returns `{ id }`; DELETE returns `{ deleted: true }`
// (404 when no tenant engagement matches) — the shapes `client-data.ts`
// updateEngagement()/deleteEngagement() unwrap. Every write audits to
// `dojo.audit_events`. The store logic lives in `$lib/server/engagements-data`.
// (GET/POST list+create are inline in the parent `engagements/+server.ts`.)
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { updateEngagement, deleteEngagement, parsePatchEngagement, EngagementsError } from '$lib/server/engagements-data';
import { recordAudit } from '$lib/server/audit';

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
		const input = parsePatchEngagement(body);
		const db = dojoDb();
		const result = await updateEngagement(db, caller.tenantId, params.id, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: input.status === 'ended' ? 'engagement_closed' : 'engagement_updated',
			target: result.id,
			engagementId: result.id
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof EngagementsError) return apiError(e.status, e.message);
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
		const deleted = await deleteEngagement(db, caller.tenantId, params.id);
		if (!deleted) return apiError(404, 'no such engagement');
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'engagement_deleted',
			target: params.id
		});
		return Response.json({ deleted: true });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof EngagementsError) return apiError(e.status, e.message);
		throw e;
	}
};
