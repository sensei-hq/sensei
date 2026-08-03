// POST /v1/you/contributions/adopt — Pin (adopt) an approved-for-you artifact.
// User-wide (JWT-resolved). Body: { artifactId }. Flips the caller's own inbox
// row(s) for that artifact to `pinned` (authorized by membership ownership).
// Fails closed; a missing artifactId is a 400.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { userMembershipIds, adoptDownstream, AdminError } from '$lib/server/contributions-data';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const artifactId = typeof body.artifactId === 'string' ? body.artifactId : '';
		if (!artifactId) return apiError(400, 'artifactId required');
		const membershipIds = await userMembershipIds(db, userId);
		await adoptDownstream(db, membershipIds, artifactId);
		return Response.json({ ok: true });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
