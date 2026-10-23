// The Microsoft Entra connect for Azure DevOps (#308 R5b, D42): a server-side,
// confidential-client authorization-code flow with PKCE against the multi-tenant
// `organizations` authority, asking for the Azure DevOps resource's READ scopes
// and offline_access. The refresh token is the grant the dōjō keeps; access
// tokens live for one request. A missing configuration is a 503, never a
// fallback; a refresh Entra refuses (invalid_grant) is a dead grant.
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import {
	AZURE_DEVOPS_RESOURCE,
	AzureGrantRejected,
	authorizeUrl,
	azureEntraConfig,
	exchangeCode,
	pkcePair,
	refreshGrant,
	requestedScopes,
	writeScopesIn
} from './azure-entra';
import { fakeFetch } from './fake-forge-fetch';

const CFG = { clientId: 'client-id-0000', clientSecret: 'client-secret-0000' };
const TOKEN = 'https://login.microsoftonline.com/organizations/oauth2/v2.0/token';
const REDIRECT = 'https://dojo.example.com/v1/integrations/azure_devops/callback';
const answer = (over: Record<string, unknown> = {}) => ({
	token_type: 'Bearer',
	expires_in: 3599,
	access_token: 'ztest-access-0000',
	refresh_token: 'ztest-refresh-0001',
	scope: `${AZURE_DEVOPS_RESOURCE}/vso.code ${AZURE_DEVOPS_RESOURCE}/vso.project ${AZURE_DEVOPS_RESOURCE}/vso.graph ${AZURE_DEVOPS_RESOURCE}/vso.memberentitlementmanagement ${AZURE_DEVOPS_RESOURCE}/vso.profile`,
	...over
});
const form = (init?: RequestInit) => new URLSearchParams(String(init?.body ?? ''));

describe('azureEntraConfig', () => {
	it('reads the client id and secret from the server env', () => {
		expect(azureEntraConfig({ AZURE_DEVOPS_CLIENT_ID: ' id ', AZURE_DEVOPS_CLIENT_SECRET: 's' })).toEqual({
			clientId: 'id',
			clientSecret: 's'
		});
	});

	it('is a 503 "not configured" when either is missing — never a fallback', () => {
		for (const env of [{}, { AZURE_DEVOPS_CLIENT_ID: 'id' }, { AZURE_DEVOPS_CLIENT_SECRET: 's' }, { AZURE_DEVOPS_CLIENT_ID: ' ', AZURE_DEVOPS_CLIENT_SECRET: 's' }]) {
			const err = (() => {
				try {
					azureEntraConfig(env);
				} catch (e) {
					return e;
				}
			})() as AdminError;
			expect(err).toBeInstanceOf(AdminError);
			expect(err.status).toBe(503);
			expect(err.message).toBe('Azure DevOps is not configured on this dōjō');
		}
	});
});

describe('the authorize request', () => {
	it('asks for the Azure DevOps read scopes, offline_access and openid — nothing that writes', () => {
		expect(requestedScopes().split(' ')).toEqual([
			`${AZURE_DEVOPS_RESOURCE}/vso.profile`,
			`${AZURE_DEVOPS_RESOURCE}/vso.project`,
			`${AZURE_DEVOPS_RESOURCE}/vso.graph`,
			`${AZURE_DEVOPS_RESOURCE}/vso.memberentitlementmanagement`,
			`${AZURE_DEVOPS_RESOURCE}/vso.code`,
			'offline_access',
			'openid'
		]);
	});

	it('is the organizations authority, the code flow with S256 PKCE, the state and the redirect', async () => {
		const { verifier, challenge } = await pkcePair();
		expect(verifier).toMatch(/^[A-Za-z0-9_-]{43,128}$/);
		const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)));
		const expected = btoa(String.fromCharCode(...digest)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
		expect(challenge).toBe(expected);

		const url = new URL(authorizeUrl(CFG, { redirectUri: REDIRECT, state: 'state-1', challenge }));
		expect(`${url.origin}${url.pathname}`).toBe('https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize');
		expect(Object.fromEntries(url.searchParams)).toEqual({
			client_id: 'client-id-0000',
			response_type: 'code',
			response_mode: 'query',
			redirect_uri: REDIRECT,
			scope: requestedScopes(),
			state: 'state-1',
			code_challenge: challenge,
			code_challenge_method: 'S256',
			prompt: 'select_account'
		});
		// The secret is never in a URL the browser sees.
		expect(url.toString()).not.toContain('client-secret');
	});
});

describe('exchangeCode', () => {
	it('redeems the code with the verifier and the client secret, and keeps the refresh token', async () => {
		const { fn, calls } = fakeFetch({ [TOKEN]: { body: answer() } });
		const out = await exchangeCode(CFG, { code: 'code-1', verifier: 'verifier-1', redirectUri: REDIRECT }, fn);
		expect(out).toEqual({
			accessToken: 'ztest-access-0000',
			refreshToken: 'ztest-refresh-0001',
			scopes: ['vso.code', 'vso.project', 'vso.graph', 'vso.memberentitlementmanagement', 'vso.profile']
		});
		expect(calls[0].url).toBe(TOKEN);
		expect(calls[0].init?.method).toBe('POST');
		expect(calls[0].init?.redirect).toBe('manual');
		expect(Object.fromEntries(form(calls[0].init))).toEqual({
			client_id: 'client-id-0000',
			client_secret: 'client-secret-0000',
			grant_type: 'authorization_code',
			code: 'code-1',
			code_verifier: 'verifier-1',
			redirect_uri: REDIRECT,
			scope: requestedScopes()
		});
	});

	it('throws when Entra answers without a refresh token (offline_access not granted) or an access token', async () => {
		for (const body of [answer({ refresh_token: undefined }), answer({ access_token: '' })]) {
			const { fn } = fakeFetch({ [TOKEN]: { body } });
			await expect(exchangeCode(CFG, { code: 'c', verifier: 'v', redirectUri: REDIRECT }, fn)).rejects.toBeInstanceOf(AdminError);
		}
	});

	it('an Entra error is said by its code — never with a token or the secret', async () => {
		const { fn } = fakeFetch({
			[TOKEN]: { status: 400, body: { error: 'invalid_client', error_description: 'AADSTS7000215: Invalid client secret provided.' } }
		});
		const err = await exchangeCode(CFG, { code: 'c', verifier: 'v', redirectUri: REDIRECT }, fn).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(502);
		expect(err.message).toContain('invalid_client');
		expect(err.message).not.toContain('client-secret-0000');
	});
});

describe('refreshGrant', () => {
	it('refreshes with the stored token and hands back the rotated one', async () => {
		const { fn, calls } = fakeFetch({ [TOKEN]: { body: answer({ refresh_token: 'ztest-refresh-0002' }) } });
		const out = await refreshGrant(CFG, 'ztest-refresh-0001', fn);
		expect(out.refreshToken).toBe('ztest-refresh-0002');
		expect(Object.fromEntries(form(calls[0].init))).toMatchObject({
			grant_type: 'refresh_token',
			refresh_token: 'ztest-refresh-0001',
			client_secret: 'client-secret-0000',
			scope: requestedScopes()
		});
	});

	it('keeps the same refresh token when Entra does not rotate it', async () => {
		const { fn } = fakeFetch({ [TOKEN]: { body: answer({ refresh_token: undefined }) } });
		expect((await refreshGrant(CFG, 'ztest-refresh-0001', fn)).refreshToken).toBe('ztest-refresh-0001');
	});

	it('invalid_grant is a dead grant (revoked, expired, consent withdrawn); anything else stays an error to retry', async () => {
		const dead = fakeFetch({ [TOKEN]: { status: 400, body: { error: 'invalid_grant', error_description: 'AADSTS70043: expired' } } });
		await expect(refreshGrant(CFG, 'r', dead.fn)).rejects.toBeInstanceOf(AzureGrantRejected);
		const down = fakeFetch({ [TOKEN]: { status: 503, body: { error: 'temporarily_unavailable' } } });
		const err = await refreshGrant(CFG, 'r', down.fn).catch((e) => e);
		expect(err).not.toBeInstanceOf(AzureGrantRejected);
		expect(err.status).toBe(502);
	});
});

describe('writeScopesIn', () => {
	it('names every granted scope beyond the read ones — the write-scope warning', () => {
		expect(writeScopesIn(['vso.code', 'vso.project', 'openid', 'offline_access', 'profile', 'email'])).toEqual([]);
		expect(writeScopesIn(['vso.code_write', 'vso.code', 'vso.project_manage', 'user_impersonation'])).toEqual([
			'vso.code_write',
			'vso.project_manage',
			'user_impersonation'
		]);
	});

	it('reads the response scope however Entra spells it: resource-prefixed, by URI, or URL-encoded', async () => {
		const { fn } = fakeFetch({
			[TOKEN]: {
				body: answer({
					scope: encodeURIComponent(`https://app.vssps.visualstudio.com/vso.code_write ${AZURE_DEVOPS_RESOURCE}/vso.graph`)
				})
			}
		});
		const out = await refreshGrant(CFG, 'r', fn);
		expect(out.scopes).toEqual(['vso.code_write', 'vso.graph']);
	});
});
