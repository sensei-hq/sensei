// POST /v1/t/{origin}/{org}/engagements/{id}/bind — bind a project to an
// engagement (routes that project's work to the engagement so it dereferences).
// The in-Worker port of dojo-mind's `bind_project`, on the JWT/console plane at
// the LEAD floor. Merges `{ project_id, name }` into the engagement's
// `project_bindings` jsonb (idempotent on project_id) and returns
// `{ id, bound: true }` — the shape `client-data.ts bindEngagementProject()`
// unwraps. 404 when no tenant engagement matches. The store logic lives in
// `$lib/server/engagements-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { bindEngagementProject, parseBindProject, EngagementsError } from '$lib/server/engagements-data';
import { recordAudit } from '$lib/server/audit';

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
		const input = parseBindProject(body);
		const db = dojoDb();
		const result = await bindEngagementProject(db, caller.tenantId, params.id, input);
		await recordAudit(db, caller.tenantId, caller.userId, {
			action: 'project_bound',
			target: input.project_id,
			detail: { name: input.name ?? null },
			engagementId: result.id
		});
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof EngagementsError) return apiError(e.status, e.message);
		throw e;
	}
};
