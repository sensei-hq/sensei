// POST /v1/you/rule-packs/{slug}/adopt — adopt (or drop) a library pack for the
// caller's USER-scoped namespace. Body: { adopt: boolean } (default true). Writes
// through dojo.set_pack_adoption (SECURITY DEFINER — no sensei grant). User-wide
// (/v1/you plane): resolved from the JWT, adopts into the caller's own namespace.
// 404 when the slug isn't a known global library pack; fails closed.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { setPackAdoption, RulePacksError } from '$lib/server/rulepacks-data';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { userId, email, db } = await resolveCaller(request, locals);
		const body = await request.json().catch(() => ({}));
		const adopt = body?.adopt !== false; // default: adopt
		const ok = await setPackAdoption(db, params.slug, userId, email, adopt);
		if (!ok) return apiError(404, 'unknown rule pack');
		return Response.json({ adopted: adopt });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulePacksError) return apiError(e.status, e.message);
		throw e;
	}
};
