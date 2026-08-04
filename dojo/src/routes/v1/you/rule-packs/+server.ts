// GET /v1/you/rule-packs — the global rule-pack LIBRARY a caller can browse + adopt.
// User-wide (/v1/you plane): the catalog is the same for everyone (owner_namespace_id
// NULL + status 'active'), resolved from the JWT only to require a signed-in caller.
// Read-only; fails closed; honest-empty `{ packs: [] }` until the library is seeded —
// never a fixture. Per-caller adoption state is layered by the browse loader/adopt
// endpoint, not here.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { listLibraryPacks, listAdoptedPackSlugs, RulePacksError } from '$lib/server/rulepacks-data';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const [packs, adopted] = await Promise.all([
			listLibraryPacks(db),
			listAdoptedPackSlugs(db, userId)
		]);
		return Response.json({ packs, adopted });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulePacksError) return apiError(e.status, e.message);
		throw e;
	}
};
