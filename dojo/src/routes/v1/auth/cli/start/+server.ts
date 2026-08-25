// GET /v1/auth/cli/start?challenge=…&port=…&login=… — begin a daemon sign-in.
//
// Returns the URL to open rather than redirecting: senseid may be headless, and
// the caller (the desktop app, the CLI) knows better than dōjō does how to put a
// browser in front of the user.
//
// PUBLIC and unauthenticated by necessity — this IS the sign-in. It hands out no
// session and no secret: the returned URL is only usable by whoever holds the
// matching PKCE verifier, which never leaves the daemon.
import type { RequestHandler } from './$types';
import { env as pub } from '$env/dynamic/public';
import { apiError } from '$lib/server/dojo-auth';
import { authorizeUrl, isForwardablePort } from '$lib/server/cli-auth';

export const GET: RequestHandler = ({ url }) => {
	const challenge = url.searchParams.get('challenge');
	const port = url.searchParams.get('port');

	// A missing challenge would yield a URL that fails at the EXCHANGE — the
	// second leg — so the user would complete a browser sign-in and only then see
	// it break, with an error that says nothing about what was wrong.
	if (!challenge) return apiError(400, 'challenge is required');
	if (!isForwardablePort(port)) return apiError(400, 'port must be an integer in 1024–65535');

	const supabaseUrl = pub.PUBLIC_SUPABASE_URL;
	if (!supabaseUrl) return apiError(503, 'dōjō is not configured for sign-in');

	return Response.json({
		authorizeUrl: authorizeUrl({
			supabaseUrl,
			origin: url.origin,
			port: Number(port),
			challenge,
			login: url.searchParams.get('login')
		})
	});
};
