// The forge-token REFRESH, which only the dōjō can perform.
//
// The daemon holds the refresh token and cannot use it: redeeming one needs the
// OAuth app's client secret, which lives here and deliberately never reaches a
// user's machine. So the daemon sends the refresh token and the dōjō returns a
// live access token.
//
// Verified against the real GitHub App before this was written: a refresh
// returns `expires_in: 28800` (8h) and `refresh_token_expires_in: 15724800`
// (182d), with the scope preserved. The 8h figure is why the token dies
// overnight and why the scheduled check exists at all.
import { describe, it, expect, vi } from 'vitest';
import { refreshForgeToken, ForgeRefreshError } from './forge-refresh';

const CREDS = { clientId: 'Ov23xxxx', clientSecret: 's3cret' };
const NOW = 1_788_120_000;

/** A fetch that answers with one canned body, and records what it was sent. */
function fetchWith(body: unknown, status = 200) {
	const calls: Array<{ url: string; init: RequestInit }> = [];
	const impl = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
		calls.push({ url: String(url), init: init ?? {} });
		return new Response(JSON.stringify(body), {
			status,
			headers: { 'content-type': 'application/json' }
		});
	}) as unknown as typeof fetch;
	return { impl, calls };
}

const OK_BODY = {
	access_token: 'gho_new',
	refresh_token: 'ghr_new',
	expires_in: 28800,
	refresh_token_expires_in: 15724800,
	scope: 'read:org,repo,user:email',
	token_type: 'bearer'
};

describe('refreshForgeToken', () => {
	it('returns an ABSOLUTE expiry, not the relative one GitHub sends', async () => {
		// `expires_in` is relative to a response the daemon never saw. Storing it
		// raw would make a token minted at 09:00 look valid until 8h after
		// whenever it was next read — the token would be reported alive for as
		// long as anything kept checking it.
		const { impl } = fetchWith(OK_BODY);
		const out = await refreshForgeToken('ghr_old', CREDS, { fetchImpl: impl, now: () => NOW });

		expect(out.expiresAt).toBe(NOW + 28800);
		expect(out.accessToken).toBe('gho_new');
		expect(out.refreshToken).toBe('ghr_new');
		expect(out.scope).toBe('read:org,repo,user:email');
	});

	it('sends the grant GitHub requires and asks for JSON', async () => {
		// Without `Accept: application/json` GitHub answers form-encoded, and
		// `res.json()` throws on a response that actually succeeded.
		const { impl, calls } = fetchWith(OK_BODY);
		await refreshForgeToken('ghr_old', CREDS, { fetchImpl: impl, now: () => NOW });

		expect(calls).toHaveLength(1);
		expect(calls[0].url).toBe('https://github.com/login/oauth/access_token');
		expect(calls[0].init.method).toBe('POST');
		expect(new Headers(calls[0].init.headers).get('accept')).toBe('application/json');
		const sent = new URLSearchParams(calls[0].init.body as string);
		expect(sent.get('grant_type')).toBe('refresh_token');
		expect(sent.get('refresh_token')).toBe('ghr_old');
		expect(sent.get('client_id')).toBe(CREDS.clientId);
		expect(sent.get('client_secret')).toBe(CREDS.clientSecret);
	});

	it('treats a 200 carrying an `error` field as a FAILURE', async () => {
		// The OAuth quirk that makes this endpoint dangerous: GitHub answers HTTP
		// 200 with `{"error":"bad_refresh_token"}`. Checking `res.ok` alone would
		// return `accessToken: undefined` as a success, and the daemon would
		// overwrite a working credential in the Keychain with nothing.
		const { impl } = fetchWith({
			error: 'bad_refresh_token',
			error_description: 'The refresh token passed is incorrect or expired.'
		});
		await expect(
			refreshForgeToken('ghr_old', CREDS, { fetchImpl: impl, now: () => NOW })
		).rejects.toBeInstanceOf(ForgeRefreshError);
	});

	it('says whether a sign-in is the only remedy', async () => {
		// The same split the daemon makes: a rejected grant is terminal and needs
		// the user, an outage is not. Collapsing them either nags about a blip or
		// stays silent about a grant that will never work again.
		const terminal = fetchWith({ error: 'bad_refresh_token' });
		await refreshForgeToken('x', CREDS, { fetchImpl: terminal.impl, now: () => NOW }).catch(
			(e: ForgeRefreshError) => expect(e.needsSignIn).toBe(true)
		);

		const transient = fetchWith({ message: 'bad gateway' }, 502);
		await refreshForgeToken('x', CREDS, { fetchImpl: transient.impl, now: () => NOW }).catch(
			(e: ForgeRefreshError) => expect(e.needsSignIn).toBe(false)
		);
		expect.assertions(2);
	});

	it('refuses a response with no access token instead of returning a blank one', async () => {
		// A 200 with neither `error` nor `access_token` is a shape we do not
		// understand. Returning it would store an empty string as a credential;
		// every later call then 401s with nothing explaining why.
		const { impl } = fetchWith({ token_type: 'bearer', expires_in: 28800 });
		await expect(
			refreshForgeToken('ghr_old', CREDS, { fetchImpl: impl, now: () => NOW })
		).rejects.toThrow(/no access token/i);
	});

	it('keeps the OLD refresh token when GitHub returns none', async () => {
		// GitHub rotates on every redemption, but a response without a new
		// refresh token means the old one still stands. Returning `undefined`
		// would make the caller erase the only credential that can recover the
		// session — unrecoverable without a sign-in, for a refresh that WORKED.
		const { impl } = fetchWith({ access_token: 'gho_new', expires_in: 28800 });
		const out = await refreshForgeToken('ghr_old', CREDS, { fetchImpl: impl, now: () => NOW });
		expect(out.refreshToken).toBe('ghr_old');
	});

	it('never puts the client secret in the error it throws', async () => {
		// This error is returned to the daemon and logged on both sides. A
		// failure that echoes the request would publish the OAuth app's secret
		// into the daemon's log file on every outage.
		const { impl } = fetchWith({ error: 'incorrect_client_credentials' }, 200);
		const err = await refreshForgeToken('ghr_old', CREDS, {
			fetchImpl: impl,
			now: () => NOW
		}).catch((e: Error) => e);
		const text = `${(err as Error).message} ${JSON.stringify(err)}`;
		expect(text).not.toContain(CREDS.clientSecret);
		expect(text).not.toContain('ghr_old');
	});
});
