// GET /v1/t/{origin}/{org}/audit/ledger — the lead Client-audit confidentiality
// ledger: `dojo.audit_events` filtered to the confidentiality events (publish /
// contained / held), client-name enriched. LEAD-floor gated. This is the correct
// source for the client-audit screen — NOT the general admin action-audit
// (`…/audit`, ADMIN) the screen was previously bound to.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { getClientAuditLedger, AdminError } from '$lib/server/client-audit-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const entries = await getClientAuditLedger(callerDb(caller), caller.tenantId);
		return Response.json({ entries });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
