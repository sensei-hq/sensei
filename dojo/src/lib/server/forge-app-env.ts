// The GitHub App's client credentials, read from the Worker env.
//
// Kept SEPARATE from forge-refresh.ts for the same reason relay-push-env.ts is
// separate from relay-push-send.ts: `$env/dynamic/private` is a SvelteKit
// virtual module that vitest does not generate, so the pure logic must not
// import it.
//
// Configuration:
//   GITHUB_OAUTH_CLIENT_ID     — the App's client id. Not secret.
//   GITHUB_OAUTH_CLIENT_SECRET — SECRET. Local: gitignored .dev.vars.
//                                Prod: `wrangler secret put`.
//
// These name the same GitHub App registration the auth provider signs users in
// with; the value is one credential, configured in each service that needs it.
import { env as priv } from '$env/dynamic/private';
import type { ForgeAppCredentials } from './forge-refresh';

/**
 * The credentials, or `null` when the deployment has not been given them.
 *
 * Null rather than a throw or a placeholder, so the route can answer 503 — "this
 * dōjō cannot refresh" — instead of calling GitHub with an empty secret. That
 * call comes back `incorrect_client_credentials`, which is a TERMINAL refusal,
 * so an unconfigured deployment would tell every user their grant was revoked
 * and send them to a sign-in that cannot fix it.
 */
export function forgeAppFromEnv(): ForgeAppCredentials | null {
	const clientId = priv.GITHUB_OAUTH_CLIENT_ID;
	const clientSecret = priv.GITHUB_OAUTH_CLIENT_SECRET;
	if (!clientId || !clientSecret) return null;
	return { clientId, clientSecret };
}
