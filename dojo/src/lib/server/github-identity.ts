// Whose GitHub token is this? (#308 R5a security review.)
//
// A GitHub token a caller hands us — in the session the browser POSTs to
// /auth/session, or in the daemon's POST /v1/you/provision body — proves only
// that SOMEONE holds it. Trusted as-is, a signed-in person holding another
// account's token could seal it as their own connection and claim or map that
// account's organizations, or provision themselves into them.
//
// So a token is trusted only as the GitHub account linked to the caller's LOGIN,
// and the authority on that link is Supabase Auth: the `github` identity GoTrue
// recorded when this login signed in with GitHub (`user.identities[].id`,
// GitHub's numeric user id — GoTrue's provider_id). Not `dojo.identities`: that
// row is written BY provisioning from the token itself, so it cannot vouch for
// the token on a first pass. The token passes only when GitHub's own
// `GET /user`, read with it, answers that same id. Mismatch, no linked account,
// or an unreadable answer: refused — never assumed.
import { AdminError } from './admin-data';
import { fetchGithubUser, type ForgeUser } from './forge-github';

/** The GitHub user id linked to a validated Supabase user, or null when the
 *  login has none. `user` must come from Supabase Auth (getUser, a synced or
 *  exchanged session), never from a request's own claim. */
export function githubIdentityId(user: unknown): string | null {
	const identities = (user as { identities?: unknown } | null | undefined)?.identities;
	if (!Array.isArray(identities)) return null;
	const github = identities.find((i) => (i as { provider?: unknown })?.provider === 'github') as
		| { id?: unknown }
		| undefined;
	const id = github?.id;
	if (typeof id === 'number' && Number.isFinite(id)) return String(id);
	return typeof id === 'string' && id.trim() ? id.trim() : null;
}

/** GitHub's documented token formats: a prefix naming the kind (gho_ OAuth,
 *  ghu_ user-to-server, ghp_ / github_pat_ personal, ghs_ / ghr_) then
 *  [A-Za-z0-9_] — github.blog, "Behind GitHub's new authentication token
 *  formats". */
const GITHUB_TOKEN = /^(gh[opusr]_|github_pat_)[A-Za-z0-9_]+$/;

/**
 * Whether `token` is in one of GitHub's own token formats — the only tokens the
 * dōjō ever sends to GitHub. A session's provider token is whatever provider the
 * person just signed in with: after a Microsoft sign-in (#308 R5c) it is a
 * Microsoft Graph token, and for a login linked to GitHub too, reading it as
 * GitHub's would post a Microsoft credential to api.github.com and take GitHub's
 * 401 for "your GitHub grant is dead".
 */
export function isGithubToken(token: unknown): token is string {
	return typeof token === 'string' && GITHUB_TOKEN.test(token);
}

/** Refuse (403) unless `actual` is the GitHub account linked to the login. */
export function assertGithubAccount(actual: string, expected: string | null | undefined): void {
	if (!expected) throw new AdminError(403, 'this login has no GitHub account linked; sign in with GitHub');
	if (actual !== expected) {
		throw new AdminError(403, 'that GitHub token belongs to a different GitHub account than this login');
	}
}

/** Read `GET /user` with the token and require it to be the linked account.
 *  Asks GitHub nothing when the login has no linked account. */
export async function requireGithubTokenOwner(
	token: string,
	expected: string | null | undefined,
	fetchImpl: typeof fetch = fetch
): Promise<ForgeUser> {
	if (!expected) assertGithubAccount('', expected); // no linked account: refuse before any read
	const user = await fetchGithubUser(token, fetchImpl);
	assertGithubAccount(user.id, expected);
	return user;
}
