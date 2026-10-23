// The Azure DevOps connect flow's server glue (#308 R5b, D42).
//
//   1. START (an admin of an organization dōjō, or anyone from their personal
//      dōjō): a fresh `state` and a PKCE verifier are sealed in a cookie bound to
//      the person's login and to where the connect returns, and the person is
//      sent to Microsoft Entra (azure-entra.ts).
//   2. CALLBACK (Microsoft redirects back, a top-level GET): the state must open
//      for the SAME login and match the one returned; only then is the code
//      redeemed. The refresh token is sealed as a PENDING GRANT (its own cookie,
//      bound to the login, 15 minutes) and the person returns to the screen.
//   3. The screen lists the organizations the grant can see and the person picks
//      which to import (azure-import.ts stores the refresh token in Vault per
//      connection). Using the grant refreshes it; the rotated refresh token is
//      re-sealed.
// Nothing is written to the database until an organization is imported. Both
// cookies are HttpOnly, SameSite=Lax (Microsoft's redirect back is a top-level
// GET, which Lax allows), Path=/v1, Secure on https; sealed values open to
// nothing for another login (sealed-value.ts). Tokens never appear in a URL, an
// answer or a message.
import { authorizeUrl, exchangeCode, pkcePair, refreshGrant, writeScopesIn, type AzureEntraConfig } from './azure-entra';
import { openSealed, randomToken, seal } from './sealed-value';

export const AZURE_STATE_COOKIE = 'dojo_azure_state';
export const AZURE_GRANT_COOKIE = 'dojo_azure_grant';
const STATE_PURPOSE = 'azure-connect-state-v1';
const GRANT_PURPOSE = 'azure-connect-grant-v1';
const STATE_TTL_SECONDS = 10 * 60;
const GRANT_TTL_SECONDS = 15 * 60;
const COOKIE_PATH = '/v1';
const CALLBACK_PATH = '/v1/integrations/azure_devops/callback';

/** Where an imported organization goes: its own dōjō (from the personal
 *  dōjō), or the organization dōjō whose Integrations screen started it. */
export type AzureConnectTarget = { as: 'own' } | { as: 'this'; tenantId: string };

/** The connect's pending grant: the refresh token, the write scopes it carries
 *  (the screen warns), and where it imports to. */
export interface AzureGrant {
	refreshToken: string;
	writeScopes: string[];
	target: AzureConnectTarget;
}

interface SealedState {
	state: string;
	verifier: string;
	target: AzureConnectTarget;
}

/** The redirect URI registered on the app registration, for this origin. */
export function callbackUrl(origin: string): string {
	return `${origin}${CALLBACK_PATH}`;
}

/** The screen the connect returns to. Built from the sealed target, never from
 *  anything the request carries (no open redirect). */
function returnPath(target: AzureConnectTarget | undefined): string {
	if (target?.as === 'this' && /^[0-9A-Za-z-]{1,64}$/.test(target.tenantId)) return `/org/${target.tenantId}/integrations`;
	return '/you/dojos';
}

function cookie(name: string, value: string, maxAge: number, secure: boolean): string {
	return `${name}=${value}; Path=${COOKIE_PATH}; Max-Age=${maxAge}; HttpOnly; SameSite=Lax${secure ? '; Secure' : ''}`;
}

/** Step 1: the Entra URL to send the person to, and the state cookie to set. */
export async function startConnect(args: {
	cfg: AzureEntraConfig;
	authUserId: string;
	secret: string;
	origin: string;
	target: AzureConnectTarget;
	now?: number;
}): Promise<{ url: string; cookie: string }> {
	const now = args.now ?? Date.now();
	const state = randomToken(32);
	const { verifier, challenge } = await pkcePair();
	const sealed = await seal({ state, verifier, target: args.target } satisfies SealedState, args.authUserId, args.secret, STATE_PURPOSE, STATE_TTL_SECONDS, now);
	const secure = args.origin.startsWith('https:');
	return {
		url: authorizeUrl(args.cfg, { redirectUri: callbackUrl(args.origin), state, challenge }),
		cookie: cookie(AZURE_STATE_COOKIE, sealed, STATE_TTL_SECONDS, secure)
	};
}

/** Why a callback did not connect — the only codes the screen is told. */
export type AzureConnectError = 'expired' | 'state' | 'denied' | 'entra' | 'signin' | 'not_configured' | 'too_many';

/**
 * Step 2: Microsoft's redirect back. Opens the state for THIS login and checks
 * the returned state FIRST: a callback that does not carry this person's state —
 * anyone can make their browser GET it — is answered without counting an attempt
 * and without clearing their state cookie, so it neither spends their attempts
 * nor ends a connect in progress (#308 R5b review 2). Only a matching state is
 * counted (`guard`, the rate limit), clears the state (single use) and redeems
 * the code. Answers where to send the person and the cookies to set — never
 * throws for a refusal: a top-level navigation is answered with a redirect that
 * names the reason.
 */
export async function finishConnect(args: {
	cfg: AzureEntraConfig;
	authUserId: string;
	secret: string;
	url: string;
	stateCookie: string | undefined;
	/** Counts this attempt (the rate limit); throws when none are left. */
	guard?: () => Promise<void>;
	fetchImpl?: typeof fetch;
	now?: number;
}): Promise<{ location: string; cookies: string[] }> {
	const now = args.now ?? Date.now();
	const u = new URL(args.url);
	const secure = u.protocol === 'https:';
	const sealed = await openSealed<SealedState>(args.stateCookie, args.authUserId, args.secret, STATE_PURPOSE, now);
	const to = (error: AzureConnectError | null, target?: AzureConnectTarget) =>
		`${returnPath(target)}?${error ? `azure_error=${error}` : 'azure=connected'}`;
	if (!sealed || typeof sealed.state !== 'string' || typeof sealed.verifier !== 'string') {
		return { location: to('expired'), cookies: [] };
	}
	if (u.searchParams.get('state') !== sealed.state) return { location: to('state', sealed.target), cookies: [] };

	// The state is this person's: from here the attempt counts and the state is spent.
	const clearState = cookie(AZURE_STATE_COOKIE, '', 0, secure);
	const back = (error: AzureConnectError | null, extra: string[] = []) => ({
		location: to(error, sealed.target),
		cookies: [clearState, ...extra]
	});
	try {
		await args.guard?.();
	} catch {
		return back('too_many');
	}
	if (u.searchParams.get('error')) return back(u.searchParams.get('error') === 'access_denied' ? 'denied' : 'entra');
	const code = u.searchParams.get('code');
	if (!code) return back('entra');

	let tokens;
	try {
		tokens = await exchangeCode(
			args.cfg,
			{ code, verifier: sealed.verifier, redirectUri: `${u.origin}${CALLBACK_PATH}` },
			args.fetchImpl ?? fetch
		);
	} catch {
		return back('entra');
	}
	const grant: AzureGrant = { refreshToken: tokens.refreshToken, writeScopes: writeScopesIn(tokens.scopes), target: sealed.target };
	const grantValue = await seal(grant, args.authUserId, args.secret, GRANT_PURPOSE, GRANT_TTL_SECONDS, now);
	return back(null, [cookie(AZURE_GRANT_COOKIE, grantValue, GRANT_TTL_SECONDS, secure)]);
}

/** The pending grant sealed for this login, or null (none, expired, another
 *  login, tampered). */
export async function openGrant(value: string | undefined, authUserId: string, secret: string, now = Date.now()): Promise<AzureGrant | null> {
	const g = await openSealed<AzureGrant>(value, authUserId, secret, GRANT_PURPOSE, now);
	if (!g || typeof g.refreshToken !== 'string' || !g.refreshToken || !g.target) return null;
	return { refreshToken: g.refreshToken, writeScopes: Array.isArray(g.writeScopes) ? g.writeScopes.filter((s) => typeof s === 'string') : [], target: g.target };
}

/** Use the pending grant: a fresh access token for this request, and the grant
 *  re-sealed with the rotated refresh token. */
export async function useGrant(args: {
	cfg: AzureEntraConfig;
	grant: AzureGrant;
	authUserId: string;
	secret: string;
	secure: boolean;
	fetchImpl?: typeof fetch;
	now?: number;
}): Promise<{ accessToken: string; grant: AzureGrant; cookie: string }> {
	const tokens = await refreshGrant(args.cfg, args.grant.refreshToken, args.fetchImpl ?? fetch);
	const grant: AzureGrant = { ...args.grant, refreshToken: tokens.refreshToken };
	const value = await seal(grant, args.authUserId, args.secret, GRANT_PURPOSE, GRANT_TTL_SECONDS, args.now ?? Date.now());
	return { accessToken: tokens.accessToken, grant, cookie: cookie(AZURE_GRANT_COOKIE, value, GRANT_TTL_SECONDS, args.secure) };
}

/** The Set-Cookie value that removes the pending grant. */
export function clearGrantCookie(secure: boolean): string {
	return cookie(AZURE_GRANT_COOKIE, '', 0, secure);
}
