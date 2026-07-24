// POST /v1/t/{origin}/{org}/triage/{signature}/decide — record a maintainer
// verdict (approve / revise / decline) on a triage signature. The in-Worker port
// of dojo-mind's decide, on the JWT plane at the MAINTAINER floor. Writes a
// `dojo.decisions` row and flips `dojo.triage_queue.state`, returning the
// flattened `DecideResult { status, artifact_id? }` the console client
// `triage-data.ts decideTriage()` expects. `approve` requires a
// `distribution_scope`; `decline` requires a non-empty `reason` (validated in
// `parseDecideBody`, 400 on failure). The store logic lives in
// `$lib/server/triage-data`.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { parseDecideBody, decideTriage, TriageError } from '$lib/server/triage-data';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.maintainer
		);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		// Throws TriageError(400) on a bad status / missing required field.
		const decision = parseDecideBody(body);
		// The maintainer is the authenticated caller, never the body.
		const result = await decideTriage(
			dojoDb(),
			caller.tenantId,
			params.signature,
			decision,
			caller.userId
		);
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof TriageError) return apiError(e.status, e.message);
		throw e;
	}
};
