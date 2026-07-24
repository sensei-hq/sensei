// GET /v1/t/{origin}/{org}/triage — the tenant's open maintainer-triage queue.
// The in-Worker port of dojo-mind's triage list, on the JWT plane (the console /
// UI auth): `resolveTenantAccess` verifies the Supabase session and the caller's
// membership role against the MAINTAINER floor. Returns `{ queue: TriageRow[] }`
// — the shape the shipped `(console)` client `triage-data.ts listTriage()`
// unwraps. The query + rank logic lives in `$lib/server/triage-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listTriage, TriageError } from '$lib/server/triage-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.maintainer
		);
		const queue = await listTriage(dojoDb(), tenantId);
		return Response.json({ queue });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof TriageError) return apiError(e.status, e.message);
		throw e;
	}
};
