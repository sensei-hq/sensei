// POST /v1/you/dojos — self-serve create a dōjō (F3a). Any authenticated caller
// may create one; they become its `admin`. User-wide plane (JWT-resolved, no
// tenant). Body: { name, kind }. A name collision is a 409; an invalid body a 400.
// Fails closed; never a fabricated tenant.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { createDojo, parseNewDojo, AdminError } from '$lib/server/admin-data';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const input = parseNewDojo(body);
		const dojo = await createDojo(db, userId, input);
		return Response.json(dojo, { status: 201 });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
