// GET /v1/you/projects/{slug}/constitution — the caller's resolved constitution
// for one of their projects (F4 drill-in). User-wide (no tenant, no role floor):
// the caller is resolved from the JWT and the read is authorized by a `user_id`
// filter. `{slug}` is a rest param — a dereferenced project slug can contain a
// slash (e.g. "acme/ledger"). The daemon OWNS resolution; this only reads the
// federated jsonb. `{ constitution: null }` honestly when none is federated yet.
// Fails closed; never a fixture.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { getUserProjectConstitution, AdminError } from '$lib/server/projects-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const constitution = await getUserProjectConstitution(db, userId, params.slug);
		return Response.json({ constitution });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
