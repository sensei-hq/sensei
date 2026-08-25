// GET /v1/auth/cli/callback/{port} — the provider comes back here, and we bounce
// the browser to the daemon's loopback.
//
// Why dōjō is in the middle at all: the provider redirect must be allow-listed,
// and allow-listing every loopback port a daemon might bind is not workable. One
// `…/v1/auth/cli/callback/**` entry covers every machine instead.
//
// Nothing is read or stored here. The auth code is single-use and worthless
// without the PKCE verifier, which only the daemon has — so this handler is a
// forwarder, not a party to the exchange.
import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { apiError } from '$lib/server/dojo-auth';
import { daemonRedirect, isForwardablePort } from '$lib/server/cli-auth';

export const GET: RequestHandler = ({ params, url }) => {
	// Bounded before use: this value picks the redirect target, so an unchecked
	// one is an open redirect. See isForwardablePort.
	if (!isForwardablePort(params.port)) {
		return apiError(400, 'port must be an integer in 1024–65535');
	}
	// 303 so the browser follows with GET regardless of how it arrived.
	redirect(303, daemonRedirect(Number(params.port), url.searchParams));
};
