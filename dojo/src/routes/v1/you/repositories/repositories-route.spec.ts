// Route tests for POST /v1/you/repositories and GET /v1/you/sync/plan — the
// daemon's two calls (§VIII.1).
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
	registerRepositories: vi.fn(),
	syncPlan: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/repositories', () => ({
	registerRepositories: mocks.registerRepositories,
	syncPlan: mocks.syncPlan
}));

const register = await import('./+server');
const plan = await import('../sync/plan/+server');

function post(body: unknown) {
	return {
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

const get = {
	request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
	locals: {},
	url: new URL('http://x/')
} as never;

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: null, db: {} });
	mocks.registerRepositories.mockClear().mockResolvedValue({ mapped: [], unmapped: [] });
	mocks.syncPlan.mockClear().mockResolvedValue({ allowed: [], denied: [] });
});

describe('POST /v1/you/repositories', () => {
	it('registers against the caller PRINCIPAL id and returns mapped + unmapped', async () => {
		const result = {
			mapped: [{ repo_key: 'github.com/acme/api', tenant: 'organization/acme', repo_id: 'r1' }],
			unmapped: [{ repo_key: 'git.internal/x/y', reason: 'unknown_host' }]
		};
		mocks.registerRepositories.mockResolvedValue(result);
		const res = await register.POST(post({ repos: [{ repo_key: 'github.com/acme/api' }] }));
		expect(await res.json()).toEqual(result);
		expect(mocks.registerRepositories.mock.calls[0][1]).toBe('p1');
	});

	it('drops entries with no repo_key instead of inventing an identity for them', async () => {
		await register.POST(
			post({ repos: [{ repo_key: 'github.com/acme/api' }, { name: 'nameless' }, { repo_key: '  ' }] })
		);
		expect(mocks.registerRepositories.mock.calls[0][2]).toEqual([
			{ repo_key: 'github.com/acme/api', remote_url: null, name: null }
		]);
	});

	it('400s when repos is missing or not an array', async () => {
		expect((await register.POST(post({}))).status).toBe(400);
		expect((await register.POST(post({ repos: 'all of them' }))).status).toBe(400);
		expect(mocks.registerRepositories).not.toHaveBeenCalled();
	});

	it('400s on an unreasonably large batch rather than attempting it', async () => {
		const repos = Array.from({ length: 1001 }, (_, i) => ({ repo_key: `github.com/acme/r${i}` }));
		expect((await register.POST(post({ repos }))).status).toBe(400);
	});

	it('propagates a 401 from the resolver', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('nope', { status: 401 }));
		expect((await register.POST(post({ repos: [] }))).status).toBe(401);
	});
});

describe('GET /v1/you/sync/plan', () => {
	it('returns the plan for the caller principal', async () => {
		const p = {
			allowed: [{ repo_key: 'github.com/acme/api', tenant: 'organization/acme', repo_id: 'r1' }],
			denied: []
		};
		mocks.syncPlan.mockResolvedValue(p);
		expect(await (await plan.GET(get)).json()).toEqual(p);
		expect(mocks.syncPlan.mock.calls[0][1]).toBe('p1');
	});

	it('always carries denied, even when empty', async () => {
		// Present-but-empty, so the daemon's handling of denials is exercised from
		// day one and phase 2 changes no shape (§V.5).
		const body = await (await plan.GET(get)).json();
		expect(body).toHaveProperty('denied');
		expect(body.denied).toEqual([]);
	});

	it('401s rather than returning an empty plan when unauthenticated', async () => {
		// An empty allow-list and a refusal are indistinguishable to a daemon that
		// just syncs what it is given — so the refusal has to be the status code.
		mocks.resolveCaller.mockRejectedValue(new Response('nope', { status: 401 }));
		expect((await plan.GET(get)).status).toBe(401);
	});

	it('500s rather than reporting an empty plan when the lookup fails', async () => {
		mocks.syncPlan.mockRejectedValue(new AdminError(500, 'db down'));
		expect((await plan.GET(get)).status).toBe(500);
	});
});
