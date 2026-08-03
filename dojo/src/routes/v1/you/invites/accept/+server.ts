// POST /v1/you/invites/accept — the invitee redeems a magic-link invite (F3b).
// User-wide (resolveCaller gives the JWT-verified userId + email). Body: { token }.
// acceptInvite fails closed on every gate (unknown/expired/used token, or a
// caller email that doesn't match the invite) and only provisions the membership
// when all pass. Returns { tenant_id, role }. Never a fabricated membership.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { acceptInvite, AdminError } from '$lib/server/invites-data';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, email, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const token = typeof body.token === 'string' ? body.token : '';
		const result = await acceptInvite(db, userId, email, token, Date.now());
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
