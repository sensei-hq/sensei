// POST /v1/you/forge/refresh — turn a refresh token into a live forge token.
//
// The half of the token lifecycle the daemon cannot do itself. Redeeming a
// refresh token needs the GitHub App's CLIENT SECRET; the daemon runs on user
// machines and deliberately does not hold it, so it sends the refresh token —
// scoped to one user, revocable by that user — and gets a live access token
// back. Neither side can refresh alone.
//
// ## The token travels in the BODY
//
// Not the URL. Query strings land in access logs, browser history and referrer
// headers; a refresh token there outlives the request by however long the logs
// are kept.
//
// ## Retries redeem
//
// GitHub rotates the refresh token on every successful redemption, so a retry
// after a successful-but-lost response spends a token that has already been
// replaced. That is why authentication is checked before anything is spent, and
// why `needsSignIn` is returned rather than left for the daemon to infer from
// the message: a terminal refusal must stop the retry loop.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { refreshForgeToken, ForgeRefreshError } from '$lib/server/forge-refresh';
import { forgeAppFromEnv } from '$lib/server/forge-app-env';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		// Before anything is spent. `resolveCaller` throws a Response, which the
		// catch below re-throws intact.
		await resolveCaller(request, locals);

		const creds = forgeAppFromEnv();
		if (!creds) {
			// 503, not 502: the fault is this deployment's, not GitHub's. Calling
			// GitHub with an empty secret returns `incorrect_client_credentials`,
			// a TERMINAL refusal — so an unconfigured dōjō would tell every user
			// their grant was revoked and send them to a sign-in that cannot help.
			return apiError(503, 'this dōjō is not configured to refresh forge tokens');
		}

		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const refreshToken = typeof body.refresh_token === 'string' ? body.refresh_token.trim() : '';
		if (!refreshToken) return apiError(400, 'refresh_token is required');

		const out = await refreshForgeToken(refreshToken, creds);
		return Response.json({
			access_token: out.accessToken,
			refresh_token: out.refreshToken,
			expires_at: out.expiresAt,
			scope: out.scope
		});
	} catch (e) {
		if (e instanceof Response) return e;
		// Carries the retry decision as a FIELD. The daemon must not have to parse
		// English to learn whether trying again can ever work.
		if (e instanceof ForgeRefreshError) {
			return new Response(JSON.stringify({ error: e.message, needsSignIn: e.needsSignIn }), {
				status: e.status,
				headers: { 'content-type': 'application/json' }
			});
		}
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
