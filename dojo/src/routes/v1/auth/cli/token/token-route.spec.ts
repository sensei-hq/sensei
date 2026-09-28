// Route tests for POST /v1/auth/cli/token — the PKCE exchange, now also the
// daemon's provisioning caller (§II.7).
//
// Two contracts pull against each other here and both are asserted:
//   1. the upstream response passes through UNCHANGED — it carries
//      provider_token, returned exactly once, and re-modelling it is how
//      `read:org` ends up granted but unusable;
//   2. provisioning happens on the way through, because this exchange is the one
//      moment provider_token is guaranteed to be in hand.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({
	resolvePrincipalId: vi.fn(),
	provisionWithToken: vi.fn(),
	fetch: vi.fn()
}));

vi.mock('$env/dynamic/public', () => ({
	env: { PUBLIC_SUPABASE_URL: 'https://sb.test', PUBLIC_SUPABASE_ANON_KEY: 'anon' }
}));
vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/principal-resolve', () => ({ resolvePrincipalId: mocks.resolvePrincipalId }));
vi.mock('$lib/server/provisioning', () => ({ provisionWithToken: mocks.provisionWithToken }));

vi.stubGlobal('fetch', mocks.fetch);
const route = await import('./+server');

const SESSION = {
	access_token: 'at',
	refresh_token: 'rt',
	provider_token: 'gh-provider',
	user: { id: 'u1', email: 'j@example.com' }
};

function upstream(status: number, body: unknown) {
	return {
		ok: status >= 200 && status < 300,
		status,
		text: async () => (typeof body === 'string' ? body : JSON.stringify(body))
	} as Response;
}

function ev(body: unknown = { code: 'c', verifier: 'v' }) {
	return {
		request: new Request('http://x/', { method: 'POST', body: JSON.stringify(body) })
	} as never;
}

beforeEach(() => {
	mocks.resolvePrincipalId.mockClear().mockResolvedValue('p1');
	mocks.provisionWithToken.mockClear().mockResolvedValue({ synced: true, personal: null, tenants: [] });
	mocks.fetch.mockClear().mockResolvedValue(upstream(200, SESSION));
});

describe('POST /v1/auth/cli/token', () => {
	it('returns the upstream session byte-for-byte, provider_token included', async () => {
		const res = await route.POST(ev());
		expect(res.status).toBe(200);
		// Deliberately compared as TEXT: an assertion on a parsed object would
		// pass even if a field had been dropped and re-added in another shape.
		expect(await res.text()).toBe(JSON.stringify(SESSION));
	});

	it('provisions with the provider_token and the resolved principal', async () => {
		await route.POST(ev());
		expect(mocks.resolvePrincipalId.mock.calls[0][1]).toBe('u1'); // the LOGIN id
		const [, principalId, token, fallback] = mocks.provisionWithToken.mock.calls[0];
		expect(principalId).toBe('p1'); // …translated to the principal
		expect(token).toBe('gh-provider');
		expect(fallback).toEqual({ email: 'j@example.com' });
	});

	it('still returns the session when provisioning throws', async () => {
		// The exchange already succeeded. Failing here would discard a valid
		// session over something the user cannot act on; the daemon's next
		// /v1/you/provision re-attempts the same idempotent operation and reports
		// the error where it can be answered.
		mocks.provisionWithToken.mockRejectedValue(new Error('db down'));
		const res = await route.POST(ev());
		expect(res.status).toBe(200);
		expect(await res.text()).toBe(JSON.stringify(SESSION));
	});

	it('does not provision when the exchange failed', async () => {
		// A 400 body is an error payload, not a session. Provisioning from one
		// would be provisioning from nothing.
		mocks.fetch.mockResolvedValue(upstream(400, { error: 'invalid_grant' }));
		const res = await route.POST(ev());
		expect(res.status).toBe(400);
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('does not provision when the session carries no user id', async () => {
		mocks.fetch.mockResolvedValue(upstream(200, { access_token: 'at' }));
		await route.POST(ev());
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('survives an upstream body that is not JSON', async () => {
		mocks.fetch.mockResolvedValue(upstream(200, 'not json at all'));
		const res = await route.POST(ev());
		expect(await res.text()).toBe('not json at all');
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('passes null rather than undefined when the exchange returned no provider_token', async () => {
		// The unobserved assumption in the whole design. If Supabase does not
		// return one, provisioning must degrade to `no_forge_token` and SAY so,
		// not appear to succeed.
		mocks.fetch.mockResolvedValue(upstream(200, { user: { id: 'u1' }, access_token: 'at' }));
		await route.POST(ev());
		expect(mocks.provisionWithToken.mock.calls[0][2]).toBeNull();
	});

	it('400s without a code and verifier, before touching the provider', async () => {
		expect((await route.POST(ev({ code: 'c' }))).status).toBe(400);
		expect(mocks.fetch).not.toHaveBeenCalled();
	});
});
