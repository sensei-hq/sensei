// POST /v1/you/invites/accept — the invitee redeems a magic-link invite (F3b).
// User-wide (resolveCaller gives the JWT-verified userId + email). Body: { token }.
// acceptInvite fails closed on every gate (unknown/expired/used token, or a
// caller email that doesn't match the invite) and only provisions the membership
// when all pass. Returns { tenant_id, role }. Never a fabricated membership.
// Failed attempts are bounded per person (#311).
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { acceptInvite, acceptAttemptsLeft, recordAcceptFailure, AdminError } from '$lib/server/invites-data';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, email } = await resolveCaller(request, locals);
		// System work, named (access matrix "What stays service_role", #236).
		const db = dojoDb();
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const token = typeof body.token === 'string' ? body.token : '';
		// Bounded guessing (#311): with no failed attempts left in the window, refuse
		// before any invite is read; each failed accept is recorded.
		const now = Date.now();
		if ((await acceptAttemptsLeft(db, userId, now)) === 0) {
			return apiError(429, 'too many failed invite attempts — try again in a few minutes');
		}
		try {
			return Response.json(await acceptInvite(db, userId, email, token, now));
		} catch (e) {
			if (e instanceof AdminError && e.status >= 400 && e.status < 500) await recordAcceptFailure(db, userId);
			throw e;
		}
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
