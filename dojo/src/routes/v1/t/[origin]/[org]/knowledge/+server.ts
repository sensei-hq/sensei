// GET /v1/t/{origin}/{org}/knowledge — the tenant's published-artifact library
// for the maintainer Knowledge console (active / pending-prune / catalog). The
// JWT-plane console read over `dojo.artifacts` (+ `dojo.policies.retention_days`
// for the prune window), MAINTAINER-floor gated. Read-only for v1. Replaces the
// `knowledgeFor(slug)` fixture — fails closed, never a fabricated library.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { getKnowledgeLibrary, KnowledgeError } from '$lib/server/knowledge-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.maintainer
		);
		const library = await getKnowledgeLibrary(dojoDb(), tenantId);
		return Response.json(library);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof KnowledgeError) return apiError(e.status, e.message);
		throw e;
	}
};
