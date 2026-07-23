// DELETE /v1/t/{origin}/{org}/rules/{id} — tombstone (retract) a shared rule.
// The in-Worker port of dojo-mind's `retract_rule` (api.rs): contributor+
// (device-token plane), status → tombstoned, audit-stamped. 204 on success,
// 404 when no active row matched (unknown id / already tombstoned). See
// docs/plan/2026-07-23-retire-dojo-mind.md.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { retractRule, recordRulesAudit, RulesError } from '$lib/server/rules-data';

export const DELETE: RequestHandler = async ({ params, request }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const id = params.id;
		if (!id) return apiError(400, 'rule id is required');

		const db = dojoDb();
		const tombstoned = await retractRule(db, id);
		if (!tombstoned) return apiError(404, 'no such rule');
		// Audit is fire-and-forget (mirrors dojo-mind) but logged on failure.
		await recordRulesAudit(db, caller.tenantId, 'retract', id, caller.userId);
		return new Response(null, { status: 204 });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulesError) return apiError(e.status, e.message);
		throw e;
	}
};
