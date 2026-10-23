// Connecting Azure DevOps through Microsoft Entra (#308 R5b, D42).
//
// A server-side, confidential-client authorization-code flow with PKCE
// (https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow),
// separate from the dōjō's own sign-in: it never goes through Supabase Auth, so
// connecting Azure never swaps the person's dōjō login (R5a's caveat).
//   * Authority: `organizations` — any work or school account, one multi-tenant
//     app registration for sensei. Personal Microsoft accounts (MSA) cannot use
//     the Azure DevOps resource through Entra ("Microsoft Entra apps don't
//     natively support Microsoft account (MSA) users for the Azure DevOps
//     resource" — https://learn.microsoft.com/en-us/azure/devops/integrate/get-started/authentication/entra-oauth).
//   * Scopes: the Azure DevOps resource (499b84ac-1321-427f-aa17-267ca6975798)'s
//     READ scopes, plus offline_access (the refresh token) and openid.
//   * The REFRESH token is the grant the dōjō keeps (in Vault, per connection);
//     an access token lives for one request and is never stored. Entra may rotate
//     the refresh token on each refresh — the new one replaces the old.
//   * Scopes granted are read from the token response's `scope` field ("the
//     scopes that the access_token is valid for"), NOT by decoding the access
//     token: Microsoft says not to read tokens for APIs you don't own.
//
// The client id and secret come from the server env (AZURE_DEVOPS_CLIENT_ID,
// AZURE_DEVOPS_CLIENT_SECRET); without them this is a 503, never a fallback. The
// secret and every token stay out of URLs the browser sees, logs and messages.
import { AdminError } from './admin-data';
import { AZURE_REQUIRED_SCOPES } from '../azure-view';
import { b64url, randomToken } from './sealed-value';

/** The Azure DevOps resource's application id in Entra. */
export const AZURE_DEVOPS_RESOURCE = '499b84ac-1321-427f-aa17-267ca6975798';
const AUTHORITY = 'https://login.microsoftonline.com/organizations/oauth2/v2.0';
const TOKEN_HOST = 'login.microsoftonline.com';

/** The read scopes, unprefixed. */
const READ_SCOPES: readonly string[] = AZURE_REQUIRED_SCOPES.map((s) => s.scope);
/** Scopes that are not Azure DevOps permissions at all. */
const OIDC_SCOPES = new Set(['openid', 'offline_access', 'profile', 'email']);

export interface AzureEntraConfig {
	clientId: string;
	clientSecret: string;
}

/** The app registration's client id and secret, from the server env — or a 503. */
export function azureEntraConfig(env: Record<string, string | undefined>): AzureEntraConfig {
	const clientId = (env.AZURE_DEVOPS_CLIENT_ID ?? '').trim();
	const clientSecret = (env.AZURE_DEVOPS_CLIENT_SECRET ?? '').trim();
	if (!clientId || !clientSecret) throw new AdminError(503, 'Azure DevOps is not configured on this dōjō');
	return { clientId, clientSecret };
}

/** The space-separated scopes asked for: the read scopes on the Azure DevOps
 *  resource, offline_access and openid. */
export function requestedScopes(): string {
	return [...READ_SCOPES.map((s) => `${AZURE_DEVOPS_RESOURCE}/${s}`), 'offline_access', 'openid'].join(' ');
}

/** A PKCE verifier and its S256 challenge (RFC 7636). */
export async function pkcePair(): Promise<{ verifier: string; challenge: string }> {
	const verifier = randomToken(48);
	const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)));
	return { verifier, challenge: b64url(digest) };
}

/** Where to send the person: Entra's authorize endpoint for this request. */
export function authorizeUrl(cfg: AzureEntraConfig, req: { redirectUri: string; state: string; challenge: string }): string {
	const q = new URLSearchParams({
		client_id: cfg.clientId,
		response_type: 'code',
		response_mode: 'query',
		redirect_uri: req.redirectUri,
		scope: requestedScopes(),
		state: req.state,
		code_challenge: req.challenge,
		code_challenge_method: 'S256',
		prompt: 'select_account'
	});
	return `${AUTHORITY}/authorize?${q}`;
}

/** What a token request came to. The access token is for this request only. */
export interface AzureTokens {
	accessToken: string;
	refreshToken: string;
	/** The Azure DevOps scopes granted, unprefixed (e.g. `vso.code`). */
	scopes: string[];
}

/** A refresh Entra refused with `invalid_grant`: the grant was revoked, expired
 *  or its consent withdrawn. Only a new connect mends it. */
export class AzureGrantRejected extends AdminError {
	constructor() {
		super(409, 'Microsoft no longer accepts this Azure DevOps connection — reconnect Azure DevOps');
	}
}

/** The scopes a token response names, unprefixed: `499b84ac…/vso.code`,
 *  `https://app.vssps.visualstudio.com/vso.code` and `vso.code` all read as
 *  `vso.code`; a URL-encoded field is decoded. */
function scopesFrom(field: unknown): string[] {
	if (typeof field !== 'string') return [];
	let text = field;
	try {
		text = decodeURIComponent(field);
	} catch {
		// not encoded
	}
	return text
		.split(/\s+/)
		.filter(Boolean)
		.map((s) => s.slice(s.lastIndexOf('/') + 1))
		.filter((s) => s && !OIDC_SCOPES.has(s));
}

/** The granted scopes beyond the read ones (anything that could write or
 *  manage) — the screen warns about each. */
export function writeScopesIn(scopes: string[]): string[] {
	return scopes.filter((s) => !OIDC_SCOPES.has(s) && !READ_SCOPES.includes(s));
}

async function tokenRequest(
	cfg: AzureEntraConfig,
	fields: Record<string, string>,
	fetchImpl: typeof fetch,
	previousRefresh?: string
): Promise<AzureTokens> {
	const url = `${AUTHORITY}/token`;
	if (new URL(url).host !== TOKEN_HOST) throw new AdminError(500, 'refusing to send the client secret anywhere but Entra');
	const res = await fetchImpl(url, {
		method: 'POST',
		headers: { 'content-type': 'application/x-www-form-urlencoded', accept: 'application/json' },
		body: new URLSearchParams({ client_id: cfg.clientId, client_secret: cfg.clientSecret, scope: requestedScopes(), ...fields }).toString(),
		redirect: 'manual'
	});
	const body = ((await res.json().catch(() => null)) ?? {}) as Record<string, unknown>;
	if (!res.ok) {
		const code = typeof body.error === 'string' && /^[a-z_]{1,64}$/.test(body.error) ? body.error : `http_${res.status}`;
		if (code === 'invalid_grant' && fields.grant_type === 'refresh_token') throw new AzureGrantRejected();
		throw new AdminError(502, `Microsoft Entra refused the token request (${code})`);
	}
	const accessToken = typeof body.access_token === 'string' ? body.access_token : '';
	const refreshToken = typeof body.refresh_token === 'string' && body.refresh_token ? body.refresh_token : (previousRefresh ?? '');
	if (!accessToken || !refreshToken) {
		throw new AdminError(502, 'Microsoft Entra answered without the tokens the connection needs (is offline_access granted?)');
	}
	return { accessToken, refreshToken, scopes: scopesFrom(body.scope) };
}

/** Redeem the authorization code (with the PKCE verifier and the client secret). */
export function exchangeCode(
	cfg: AzureEntraConfig,
	req: { code: string; verifier: string; redirectUri: string },
	fetchImpl: typeof fetch = fetch
): Promise<AzureTokens> {
	return tokenRequest(
		cfg,
		{ grant_type: 'authorization_code', code: req.code, code_verifier: req.verifier, redirect_uri: req.redirectUri },
		fetchImpl
	);
}

/** A new access token from the stored refresh token, and the refresh token to
 *  keep (Entra's rotated one, or the same when it did not rotate). An
 *  `invalid_grant` is AzureGrantRejected. */
export function refreshGrant(cfg: AzureEntraConfig, refreshToken: string, fetchImpl: typeof fetch = fetch): Promise<AzureTokens> {
	return tokenRequest(cfg, { grant_type: 'refresh_token', refresh_token: refreshToken }, fetchImpl, refreshToken);
}
