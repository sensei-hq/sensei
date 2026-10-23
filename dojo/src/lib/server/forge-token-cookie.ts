// Where a "Connect GitHub" re-authorization's token lives (#308 R5a).
//
// The token exists on the server once: in the provider session the browser POSTs
// to kavach's session route after the OAuth redirect (the session cookie keeps
// only access/refresh tokens — see provision-on-signin.ts). The Integrations
// reads, claim and map run on LATER requests, so the token has to survive until
// then without the database holding it.
//
// It rides in its own cookie, set on that same /auth/session response:
//   * SEALED — AES-GCM under a key derived (HKDF) from the server's secret key,
//     so the browser holds ciphertext it cannot read or forge;
//   * BOUND to the Supabase login the session synced as, and to an expiry inside
//     the seal — another login's cookie, or a stale one, opens to nothing;
//   * HttpOnly, SameSite=Lax, Path=/v1/t (only the tenant API sees it), one hour.
// Signing out clears it. A cookie that does not open is "not connected": the
// screen offers Connect GitHub again — there is no other source of the token.

/** The cookie's name. */
export const FORGE_TOKEN_COOKIE = 'dojo_forge_github';
/** How long a connected token is honoured. */
export const FORGE_TOKEN_TTL_SECONDS = 3600;

import { githubIdentityId, isGithubToken, requireGithubTokenOwner } from './github-identity';
import { openSealed, seal } from './sealed-value';

const COOKIE_PATH = '/v1/t';
/** The seal's purpose — its own HKDF key (sealed-value.ts). Unchanged from R5a,
 *  so a cookie sealed before the primitive moved still opens. */
const PURPOSE = 'forge-token-cookie-v1';

/** Seal `token` for `authUserId`, good for FORGE_TOKEN_TTL_SECONDS from `now`. */
export function sealForgeToken(token: string, authUserId: string, secret: string, now = Date.now()): Promise<string> {
	return seal(token, authUserId, secret, PURPOSE, FORGE_TOKEN_TTL_SECONDS, now);
}

/** The token sealed in `value`, or null when it is absent, tampered with,
 *  expired, sealed for another login, or under another secret. */
export async function openForgeToken(
	value: string | undefined,
	authUserId: string,
	secret: string,
	now = Date.now()
): Promise<string | null> {
	const t = await openSealed(value, authUserId, secret, PURPOSE, now);
	return typeof t === 'string' && t ? t : null;
}

function attributes(maxAge: number, secure: boolean): string {
	return `Path=${COOKIE_PATH}; Max-Age=${maxAge}; HttpOnly; SameSite=Lax${secure ? '; Secure' : ''}`;
}

/** The Set-Cookie value carrying a sealed token. */
export function forgeTokenCookie(sealed: string, secure: boolean): string {
	return `${FORGE_TOKEN_COOKIE}=${sealed}; ${attributes(FORGE_TOKEN_TTL_SECONDS, secure)}`;
}

/** The Set-Cookie value that removes it. */
export function clearForgeTokenCookie(secure: boolean): string {
	return `${FORGE_TOKEN_COOKIE}=; ${attributes(0, secure)}`;
}

/** Auth events that carry a fresh provider session (`TOKEN_REFRESHED` never
 *  carries a provider token). */
const CONNECT_EVENTS = new Set(['SIGNED_IN', 'INITIAL_SESSION', 'USER_UPDATED']);

/**
 * Wrap kavach's /auth/session answer: after a GitHub sign-in (a provider token
 * in GitHub's own format), add the sealed provider token as its own cookie — only once GitHub's `GET /user` says the
 * token is the GitHub account linked to the synced login; after a sign-out,
 * clear it. Bound to the
 * login the session SYNCED as (the answer's session, which kavach validated),
 * never to anything the request claims. Anything else passes through untouched.
 *
 * `request` must be an unread clone of the sync request (kavach reads the
 * original).
 */
export async function captureForgeToken(
	request: Request,
	response: Response,
	secret: string,
	now = Date.now(),
	fetchImpl: typeof fetch = fetch
): Promise<Response> {
	const secure = new URL(request.url).protocol === 'https:';
	const sent = (await request.json().catch(() => ({}))) as {
		event?: unknown;
		session?: { provider_token?: unknown } | null;
	};
	let cookie: string | null = null;
	if (!sent.session) {
		cookie = clearForgeTokenCookie(secure);
	} else if (
		response.ok &&
		typeof sent.event === 'string' &&
		CONNECT_EVENTS.has(sent.event) &&
		// A GitHub token only: a Microsoft sign-in's (#308 R5c) is never sent to
		// GitHub, nor sealed as a GitHub connection.
		isGithubToken(sent.session.provider_token)
	) {
		// Judged from what kavach VALIDATED, never from what the request claims: the
		// synced login, and the GitHub account Supabase linked to it. The posted
		// token is sealed only if GitHub says it IS that account (github-identity.ts)
		// — otherwise a person holding someone else's token could make it theirs.
		const synced = (await response
			.clone()
			.json()
			.catch(() => null)) as { session?: { user?: { id?: unknown } } | null } | null;
		const user = synced?.session?.user;
		if (typeof user?.id === 'string' && user.id) {
			const owned = await requireGithubTokenOwner(sent.session.provider_token, githubIdentityId(user), fetchImpl)
				.then(() => true)
				.catch(() => false);
			if (owned) {
				cookie = forgeTokenCookie(await sealForgeToken(sent.session.provider_token, user.id, secret, now), secure);
			}
		}
	}
	if (!cookie) return response;
	const headers = new Headers(response.headers);
	headers.append('set-cookie', cookie);
	return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
}
