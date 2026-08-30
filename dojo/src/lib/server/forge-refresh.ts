// Redeem a GitHub refresh token for a live access token.
//
// ## Why this lives in the dōjō and not in the daemon
//
// Redeeming a refresh token requires the OAuth app's CLIENT SECRET. The daemon
// runs on a user's laptop; shipping the secret there would put it on every
// machine that installs sensei, where a single extracted copy would let anyone
// mint tokens against the app. So the daemon holds the refresh token — which is
// scoped to one user and revocable — and the dōjō holds the secret. Neither can
// refresh alone, which is the point.
//
// ## Measured, not assumed
//
// Verified against the real app before this module existed: a redemption returns
// `expires_in: 28800` (8h) and `refresh_token_expires_in: 15724800` (182d), with
// `scope` preserved. The 8h lifetime is why a token signed in yesterday is dead
// this morning, and why `sensei.personas.forge_token_expires_at` is worth
// recording at all.
//
// PURE: every dependency is injected, so this is testable without the
// `$env/dynamic/private` virtual module (which vitest does not generate). The
// env-reading glue is the route.
import { AdminError } from './admin-data';

/** GitHub's OAuth token endpoint. Not the API host — `api.github.com` answers
 *  404 for this path, which reads as "no such user" rather than "wrong host". */
const TOKEN_URL = 'https://github.com/login/oauth/access_token';

/** The OAuth app's identity. Never logged, never returned, never in an error. */
export interface ForgeAppCredentials {
	clientId: string;
	clientSecret: string;
}

/** A live forge credential, with an ABSOLUTE deadline. */
export interface ForgeRefreshResult {
	accessToken: string;
	/** The refresh token to store now. GitHub rotates on every redemption, so
	 *  this is usually NEW — but it falls back to the one that was sent, because
	 *  a response without one means the old token still stands. */
	refreshToken: string;
	/** Unix seconds. Absolute, because the daemon reads it much later than the
	 *  response that produced it. */
	expiresAt: number;
	/** What the token can do, as GitHub reports it. Worth returning: a refresh
	 *  that silently narrows scope explains an authorization failure that would
	 *  otherwise look like a bug. */
	scope: string | null;
}

/**
 * A refresh that did not produce a token.
 *
 * `needsSignIn` is the load-bearing field, and it is the same distinction the
 * daemon draws for the session token: a REJECTED grant is terminal and only the
 * user can fix it; an outage is not, and telling someone to re-authenticate for
 * a 502 makes them destroy a working session for nothing.
 */
export class ForgeRefreshError extends AdminError {
	constructor(
		message: string,
		readonly needsSignIn: boolean
	) {
		// 502: this endpoint is a proxy for GitHub, and the failure is GitHub's.
		super(502, message);
	}
}

/** GitHub's terminal refusals. Anything else — a 5xx, a timeout, an unparseable
 *  body — is treated as transient, because the cost of guessing wrong in that
 *  direction is a needless sign-out. */
const TERMINAL = new Set([
	'bad_refresh_token',
	'bad_verification_code',
	'incorrect_client_credentials',
	'unauthorized_client',
	'invalid_grant',
	'access_denied'
]);

export interface RefreshDeps {
	fetchImpl?: typeof fetch;
	/** Unix seconds. Injected so the absolute expiry is assertable. */
	now?: () => number;
}

/**
 * Exchange a refresh token for a live access token.
 *
 * Throws [`ForgeRefreshError`] rather than returning a partial result: a caller
 * that cannot tell success from failure will write whatever it got into the
 * Keychain, and an empty string stored over a working credential is a sign-out
 * with no explanation.
 */
export async function refreshForgeToken(
	refreshToken: string,
	creds: ForgeAppCredentials,
	deps: RefreshDeps = {}
): Promise<ForgeRefreshResult> {
	const fetchImpl = deps.fetchImpl ?? fetch;
	const now = deps.now ?? (() => Math.floor(Date.now() / 1000));

	const res = await fetchImpl(TOKEN_URL, {
		method: 'POST',
		// Without this GitHub answers form-encoded and `res.json()` throws on a
		// response that actually succeeded.
		headers: { accept: 'application/json', 'content-type': 'application/x-www-form-urlencoded' },
		body: new URLSearchParams({
			grant_type: 'refresh_token',
			refresh_token: refreshToken,
			client_id: creds.clientId,
			client_secret: creds.clientSecret
		}).toString()
	});

	// Parsed before `res.ok` is consulted, because the failure mode that matters
	// arrives as HTTP 200.
	const body = (await res.json().catch(() => null)) as Record<string, unknown> | null;

	if (!res.ok) {
		throw new ForgeRefreshError(`GitHub refused the refresh (HTTP ${res.status})`, false);
	}
	if (!body) {
		throw new ForgeRefreshError('GitHub returned an unreadable refresh response', false);
	}

	// The OAuth quirk this module exists to survive: a refusal is an HTTP 200
	// with an `error` field. Trusting `res.ok` alone yields `accessToken:
	// undefined` reported as a success.
	const error = typeof body.error === 'string' ? body.error : null;
	if (error) {
		// The code only — never `error_description`, which echoes the request and
		// has been observed to quote the credential that was sent.
		throw new ForgeRefreshError(`GitHub refused the refresh: ${error}`, TERMINAL.has(error));
	}

	const accessToken = typeof body.access_token === 'string' ? body.access_token : '';
	if (!accessToken) {
		// A 200 with neither an error nor a token is a shape we do not
		// understand. Not terminal: it is far more likely a proxy or an outage
		// than a revoked grant, and a sign-out is the expensive guess.
		throw new ForgeRefreshError('GitHub returned no access token and no error', false);
	}

	const expiresIn = typeof body.expires_in === 'number' ? body.expires_in : null;
	return {
		accessToken,
		// Falls back to the token we sent. GitHub rotates, but a response without
		// a new one means the old is still valid, and erasing it would destroy
		// the only credential that can recover the session — after a SUCCESS.
		refreshToken: typeof body.refresh_token === 'string' ? body.refresh_token : refreshToken,
		// No `expires_in` means no deadline was stated. Inventing one would make
		// the daemon schedule a refresh against a fabricated time; `0` is
		// distinguishable and the caller records it as unknown.
		expiresAt: expiresIn === null ? 0 : now() + expiresIn,
		scope: typeof body.scope === 'string' ? body.scope : null
	};
}
