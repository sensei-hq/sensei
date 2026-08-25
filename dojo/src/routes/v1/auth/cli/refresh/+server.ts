// POST /v1/auth/cli/refresh — trade a refresh token for a live session.
//
// The daemon's session outlives its process, so every start (and every status
// check) refreshes. Routing it through dōjō is what lets senseid hold one
// setting — the dōjō URL — and no knowledge of where auth actually lives.
//
// The response carries the USER as well as the tokens, which is what the daemon
// reads the verified GitHub login and id from. That saves a second round trip
// and keeps the identity and the token from the same response — two calls could
// in principle answer for two different sessions.
//
// PUBLIC, like the rest of this group: possession of a live refresh token IS the
// credential. dōjō adds only the publishable key.
import type { RequestHandler } from './$types';
import { env as pub } from '$env/dynamic/public';
import { apiError } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ request }) => {
	const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
	const refreshToken = typeof body.refresh_token === 'string' ? body.refresh_token : '';
	if (!refreshToken) return apiError(400, 'refresh_token is required');

	const supabaseUrl = pub.PUBLIC_SUPABASE_URL;
	const anonKey = pub.PUBLIC_SUPABASE_ANON_KEY;
	if (!supabaseUrl || !anonKey) return apiError(503, 'dōjō is not configured for sign-in');

	let upstream: Response;
	try {
		upstream = await fetch(
			`${supabaseUrl.replace(/\/+$/, '')}/auth/v1/token?grant_type=refresh_token`,
			{
				method: 'POST',
				headers: { apikey: anonKey, 'content-type': 'application/json' },
				body: JSON.stringify({ refresh_token: refreshToken })
			}
		);
	} catch {
		// Reported as a failure, never as an empty session: the daemon treats a
		// rejected refresh as terminal and clears the stored token, so a network
		// blip dressed up as a rejection would sign the user out for nothing.
		return apiError(502, 'could not reach the identity provider');
	}

	return new Response(await upstream.text(), {
		status: upstream.status,
		headers: { 'content-type': 'application/json' }
	});
};
