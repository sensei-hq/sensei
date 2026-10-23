// Route tests for POST /v1/you/provision — the web sign-in caller of §II.7,
// and (by re-export) the console's "Sync from GitHub" button.
//
// The wiring is what is under test here, not the provisioning logic: which
// token gets used, that the caller's PRINCIPAL id is what provisioning is keyed
// on, and that a refusal names itself rather than 200-ing into silence.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveCaller: vi.fn(),
	provisionWithToken: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({ client: 'service' }) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/provisioning', () => ({ provisionWithToken: mocks.provisionWithToken }));

const route = await import('./+server');
const alias = await import('../github/sync/+server');

const RESULT = {
	synced: true,
	personal: { id: 't1', key: 'personal/dev', origin: 'personal', role: 'admin', created: true },
	tenants: []
};

function ev(opts: { sessionToken?: string | null; body?: unknown } = {}) {
	return {
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: JSON.stringify(opts.body ?? {})
		}),
		locals: { session: { provider_token: opts.sessionToken ?? null } },
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller
		.mockClear()
		.mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: 'j@example.com', accessToken: 'jwt-A', githubId: '4242', microsoft: { state: 'ok', objectId: '00000000-0000-4000-8000-0000000000a1', name: null } });
	mocks.provisionWithToken.mockClear().mockResolvedValue(RESULT);
});

describe('POST /v1/you/provision', () => {
	it('provisions with the session token and the caller PRINCIPAL id', async () => {
		const res = await route.POST(ev({ sessionToken: 'gh-session' }));
		expect(await res.json()).toEqual(RESULT);
		const [, principalId, token, fallback] = mocks.provisionWithToken.mock.calls[0];
		// The principal, not the login — provisioning writes memberships, and
		// memberships key on principal ids (§VIII.2).
		expect(principalId).toBe('p1');
		expect(token).toBe('gh-session');
		// The GitHub account linked to the LOGIN: provisioning refuses a token of
		// any other account (#308 R5a review).
		expect(fallback).toEqual({ email: 'j@example.com', githubId: '4242', microsoft: { state: 'ok', objectId: '00000000-0000-4000-8000-0000000000a1', name: null } });
	});

	it('refuses a token of another GitHub account with 403, naming why', async () => {
		mocks.provisionWithToken.mockRejectedValue(
			new AdminError(403, 'that GitHub token belongs to a different GitHub account than this login')
		);
		const res = await route.POST(ev({ body: { provider_token: 'someone-elses' } }));
		expect(res.status).toBe(403);
		expect((await res.json()).error).toMatch(/different GitHub account/);
	});

	it('prefers a token in the body over the session one', async () => {
		// The daemon persists provider_token to the OS keychain, so its copy
		// outlives the web session's, which evaporates after the exchange (§IV.8).
		const res = await route.POST(ev({ sessionToken: 'gh-session', body: { provider_token: 'gh-daemon' } }));
		expect(res.status).toBe(200);
		expect(mocks.provisionWithToken.mock.calls[0][2]).toBe('gh-daemon');
	});

	it('passes null when neither source has a token', async () => {
		// Not "" and not undefined — provisionWithToken distinguishes no-token
		// from forge-unreachable, and the console says different things about them.
		await route.POST(ev({ sessionToken: null }));
		expect(mocks.provisionWithToken.mock.calls[0][2]).toBeNull();
	});

	it('ignores a blank provider_token in the body', async () => {
		await route.POST(ev({ sessionToken: 'gh-session', body: { provider_token: '   ' } }));
		expect(mocks.provisionWithToken.mock.calls[0][2]).toBe('gh-session');
	});

	it('returns the refusal verbatim rather than an empty success', async () => {
		// The failure mode this whole slice exists to remove: a 200 that means
		// "nothing happened" and does not say so.
		mocks.provisionWithToken.mockResolvedValue({
			synced: false,
			reason: 'no_forge_token',
			personal: null,
			tenants: []
		});
		const res = await route.POST(ev());
		expect(await res.json()).toMatchObject({ synced: false, reason: 'no_forge_token' });
	});

	it('propagates a 401 Response from the resolver', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('nope', { status: 401 }));
		expect((await route.POST(ev())).status).toBe(401);
	});

	it('maps an AdminError to its status', async () => {
		mocks.provisionWithToken.mockRejectedValue(
			new AdminError(409, 'that forge account is already linked to a different person')
		);
		const res = await route.POST(ev({ sessionToken: 'gh' }));
		expect(res.status).toBe(409);
		expect(await res.json()).toMatchObject({ error: expect.stringContaining('already linked') });
	});

	it('survives a body that is not JSON', async () => {
		const res = await route.POST({
			request: new Request('http://x/', { method: 'POST', body: 'not json' }),
			locals: { session: { provider_token: 'gh' } },
			url: new URL('http://x/')
		} as never);
		expect(res.status).toBe(200);
	});
});

describe('POST /v1/you/github/sync', () => {
	it('is the same handler, so the two cannot drift', async () => {
		// It re-exports rather than keeping a second copy in step. Asserting
		// identity is what makes that guarantee real instead of a comment.
		expect(alias.POST).toBe(route.POST);
	});
});

// System work by name (#236, access matrix "What stays service_role"): provisioning
// establishes the principal and memberships a session would need, so it runs as
// the service role — named, not borrowed from the caller.
describe('provisioning is the system\'s', () => {
	it('provisions with the service role, named', async () => {
		await route.POST(ev({ sessionToken: 'gh-session' }));
		expect(mocks.provisionWithToken.mock.calls[0][0]).toEqual({ client: 'service' });
	});
});
