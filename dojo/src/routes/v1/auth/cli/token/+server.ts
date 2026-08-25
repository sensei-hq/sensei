// POST /v1/auth/cli/token — exchange a PKCE code for a session, on the daemon's
// behalf.
//
// This is the leg that keeps Supabase out of senseid's vocabulary. The daemon
// holds the verifier and posts it here with the code; dōjō forwards to the
// provider and returns the session verbatim.
//
// PUBLIC and unauthenticated, like /start, because it IS the sign-in. It grants
// nothing on its own: the exchange only succeeds for a caller holding both a
// live single-use code AND the verifier whose hash was pinned when the flow
// began. dōjō adds the publishable key, which is the one piece of this the
// daemon should not have to be told.
//
// The upstream response is passed through unchanged rather than reshaped. It
// carries provider_token — the GitHub token, returned exactly once, at this
// exchange — and re-modelling the payload here would be one more place for that
// to be quietly dropped, which is how `read:org` ends up granted but unusable.
import type { RequestHandler } from './$types';
import { env as pub } from '$env/dynamic/public';
import { apiError } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ request }) => {
	const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
	const code = typeof body.code === 'string' ? body.code : '';
	const verifier = typeof body.verifier === 'string' ? body.verifier : '';
	if (!code || !verifier) return apiError(400, 'code and verifier are required');

	const supabaseUrl = pub.PUBLIC_SUPABASE_URL;
	const anonKey = pub.PUBLIC_SUPABASE_ANON_KEY;
	if (!supabaseUrl || !anonKey) return apiError(503, 'dōjō is not configured for sign-in');

	let upstream: Response;
	try {
		upstream = await fetch(`${supabaseUrl.replace(/\/+$/, '')}/auth/v1/token?grant_type=pkce`, {
			method: 'POST',
			headers: { apikey: anonKey, 'content-type': 'application/json' },
			body: JSON.stringify({ auth_code: code, code_verifier: verifier })
		});
	} catch {
		// A failed exchange is reported as one. Fabricating a session shape here
		// would hand the daemon something it would store as a working sign-in.
		return apiError(502, 'could not reach the identity provider');
	}

	return new Response(await upstream.text(), {
		status: upstream.status,
		headers: { 'content-type': 'application/json' }
	});
};
