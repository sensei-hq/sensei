// POST /v1/you/invites/accept — the invitee redeems a magic-link invite (F3b).
// User-wide (resolveCaller gives the JWT-verified userId + email). Body: { token }.
// acceptInvite fails closed on every gate (unknown/expired/used token, or a
// caller email that doesn't match the invite) and only provisions the membership
// when all pass. Returns { tenant_id, role }. Never a fabricated membership.
// Failed attempts are bounded per person (#311).
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { acceptInvite, AdminError } from '$lib/server/invites-data';
import { releaseAttempt, takeAttempt } from '$lib/server/rate-limit';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, email } = await resolveCaller(request, locals);
		// System work, named (access matrix "What stays service_role", #236).
		const db = dojoDb();
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const token = typeof body.token === 'string' ? body.token : '';
		// Bounded guessing (#311): an attempt is taken atomically before any invite
		// is read (none left → 429), and given back unless the accept failed as the
		// caller's error — a success, or a server failure, counts no guess.
		const now = Date.now();
		const attempt = await takeAttempt(db, userId, 'invite_accept');
		if (!attempt) return apiError(429, 'too many failed invite attempts — try again in a few minutes');
		try {
			const accepted = await acceptInvite(db, userId, email, token, now);
			await releaseAttempt(db, attempt);
			return Response.json(accepted);
		} catch (e) {
			if (!(e instanceof AdminError && e.status >= 400 && e.status < 500)) await releaseAttempt(db, attempt);
			throw e;
		}
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
