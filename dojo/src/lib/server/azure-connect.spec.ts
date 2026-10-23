// The Azure DevOps connect flow's server glue (#308 R5b, D42): start (a sealed
// state cookie bound to the login and to where the connect returns), Microsoft's
// callback (state checked, code redeemed, the refresh token sealed as a pending
// grant for the same login), and opening that grant to list and import
// organizations. Nothing here is stored in the database; tokens never appear in
// a URL, an answer or a message.
import { describe, it, expect } from 'vitest';
import { AZURE_DEVOPS_RESOURCE } from './azure-entra';
import {
	AZURE_GRANT_COOKIE,
	AZURE_STATE_COOKIE,
	callbackUrl,
	finishConnect,
	openGrant,
	startConnect,
	useGrant
} from './azure-connect';
import { fakeFetch } from './fake-forge-fetch';

const CFG = { clientId: 'client-id-0000', clientSecret: 'client-secret-0000' };
const SECRET = 'azure-connect-test-secret';
const ORIGIN = 'https://dojo.example.com';
const ME = 'auth-me';
const TOKEN = 'https://login.microsoftonline.com/organizations/oauth2/v2.0/token';
const NOW = 1_800_000_000_000;

/** The value of the named cookie in a Set-Cookie string. */
const cookieValue = (setCookie: string, name: string): string | undefined => {
	const first = setCookie.split(';')[0];
	return first.startsWith(`${name}=`) ? first.slice(name.length + 1) : undefined;
};

async function started(target: Parameters<typeof startConnect>[0]['target'] = { as: 'this', tenantId: 't-here' }) {
	const out = await startConnect({ cfg: CFG, authUserId: ME, secret: SECRET, origin: ORIGIN, target, now: NOW });
	const state = new URL(out.url).searchParams.get('state') as string;
	return { ...out, state, sealed: cookieValue(out.cookie, AZURE_STATE_COOKIE) };
}

describe('startConnect', () => {
	it('sends the person to Entra with a fresh state, and seals the verifier for this login in a cookie', async () => {
		const s = await started();
		const url = new URL(s.url);
		expect(url.host).toBe('login.microsoftonline.com');
		expect(url.searchParams.get('redirect_uri')).toBe(callbackUrl(ORIGIN));
		expect(callbackUrl(ORIGIN)).toBe('https://dojo.example.com/v1/integrations/azure_devops/callback');
		expect(s.state).toMatch(/^[A-Za-z0-9_-]{32,}$/);
		expect(s.cookie).toMatch(/HttpOnly/);
		expect(s.cookie).toMatch(/SameSite=Lax/);
		expect(s.cookie).toMatch(/Path=\/v1;/);
		expect(s.cookie).toMatch(/Secure/);
		// The verifier is in the seal, never in the URL.
		expect(s.url).not.toContain(s.sealed as string);
	});

	it('two starts never share a state', async () => {
		expect((await started()).state).not.toBe((await started()).state);
	});
});

describe('finishConnect', () => {
	const callback = (q: Record<string, string>) => `${callbackUrl(ORIGIN)}?${new URLSearchParams(q)}`;

	it('redeems the code and seals the refresh token as a pending grant for the same login, back where it started', async () => {
		const s = await started({ as: 'this', tenantId: 't-here' });
		const { fn, calls } = fakeFetch({
			[TOKEN]: {
				body: {
					access_token: 'ztest-access',
					refresh_token: 'ztest-refresh-1',
					scope: `${AZURE_DEVOPS_RESOURCE}/vso.code ${AZURE_DEVOPS_RESOURCE}/vso.code_write`
				}
			}
		});
		const out = await finishConnect({
			cfg: CFG,
			authUserId: ME,
			secret: SECRET,
			url: callback({ code: 'code-1', state: s.state }),
			stateCookie: s.sealed,
			fetchImpl: fn,
			now: NOW + 1000
		});
		expect(out.location).toBe('/org/t-here/integrations?azure=connected');
		expect(new URLSearchParams(String(calls[0].init?.body)).get('code_verifier')).toBeTruthy();
		const grantCookie = out.cookies.find((c) => c.startsWith(`${AZURE_GRANT_COOKIE}=`)) as string;
		expect(out.cookies.some((c) => c.startsWith(`${AZURE_STATE_COOKIE}=;`))).toBe(true);
		const grant = await openGrant(cookieValue(grantCookie, AZURE_GRANT_COOKIE), ME, SECRET, NOW + 2000);
		expect(grant).toEqual({ refreshToken: 'ztest-refresh-1', writeScopes: ['vso.code_write'], target: { as: 'this', tenantId: 't-here' } });
		expect(await openGrant(cookieValue(grantCookie, AZURE_GRANT_COOKIE), 'auth-someone-else', SECRET, NOW + 2000)).toBeNull();
		expect(out.location).not.toContain('ztest');
	});

	it("the personal dōjō's connect returns to My dōjōs", async () => {
		const s = await started({ as: 'own' });
		const { fn } = fakeFetch({ [TOKEN]: { body: { access_token: 'a', refresh_token: 'r', scope: '' } } });
		const out = await finishConnect({
			cfg: CFG,
			authUserId: ME,
			secret: SECRET,
			url: callback({ code: 'c', state: s.state }),
			stateCookie: s.sealed,
			fetchImpl: fn,
			now: NOW
		});
		expect(out.location).toBe('/you/dojos?azure=connected');
	});

	it('refuses — and redeems nothing — on a missing, foreign or expired state, or a state that does not match', async () => {
		const s = await started();
		const { fn, calls } = fakeFetch({ [TOKEN]: { body: { access_token: 'a', refresh_token: 'r' } } });
		const cases: [string | undefined, string, string, number][] = [
			[undefined, s.state, ME, NOW],
			[s.sealed, 'another-state', ME, NOW],
			[s.sealed, s.state, 'auth-someone-else', NOW],
			[s.sealed, s.state, ME, NOW + 11 * 60 * 1000]
		];
		for (const [cookie, state, who, at] of cases) {
			const out = await finishConnect({
				cfg: CFG,
				authUserId: who,
				secret: SECRET,
				url: callback({ code: 'c', state }),
				stateCookie: cookie,
				fetchImpl: fn,
				now: at
			});
			expect(out.location).toMatch(/azure_error=(expired|state)$/);
			expect(out.cookies.some((c) => c.startsWith(`${AZURE_GRANT_COOKIE}=`) && !c.startsWith(`${AZURE_GRANT_COOKIE}=;`))).toBe(false);
		}
		expect(calls).toHaveLength(0);
	});

	it('says so when the person declined at Microsoft, or Entra would not redeem the code', async () => {
		const s = await started();
		const declined = await finishConnect({
			cfg: CFG,
			authUserId: ME,
			secret: SECRET,
			url: callback({ error: 'access_denied', error_description: 'the user canceled', state: s.state }),
			stateCookie: s.sealed,
			fetchImpl: fakeFetch({}).fn,
			now: NOW
		});
		expect(declined.location).toBe('/org/t-here/integrations?azure_error=denied');
		const refused = await finishConnect({
			cfg: CFG,
			authUserId: ME,
			secret: SECRET,
			url: callback({ code: 'c', state: s.state }),
			stateCookie: s.sealed,
			fetchImpl: fakeFetch({ [TOKEN]: { status: 400, body: { error: 'invalid_grant' } } }).fn,
			now: NOW
		});
		expect(refused.location).toBe('/org/t-here/integrations?azure_error=entra');
	});
});

describe('useGrant', () => {
	it('refreshes the pending grant and re-seals the rotated refresh token', async () => {
		const grant = { refreshToken: 'ztest-refresh-1', writeScopes: [], target: { as: 'own' as const } };
		const { fn } = fakeFetch({ [TOKEN]: { body: { access_token: 'ztest-access-2', refresh_token: 'ztest-refresh-2', scope: '' } } });
		const out = await useGrant({ cfg: CFG, grant, authUserId: ME, secret: SECRET, secure: true, fetchImpl: fn, now: NOW });
		expect(out.accessToken).toBe('ztest-access-2');
		expect(out.grant.refreshToken).toBe('ztest-refresh-2');
		const reopened = await openGrant(cookieValue(out.cookie, AZURE_GRANT_COOKIE), ME, SECRET, NOW);
		expect(reopened?.refreshToken).toBe('ztest-refresh-2');
	});
});
