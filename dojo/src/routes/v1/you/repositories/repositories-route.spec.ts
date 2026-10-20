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
	syncPlan: vi.fn(),
	listMyRepositories: vi.fn()
}));

// The service role vs the caller's own JWT — which client each store call got.
vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
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
	syncPlan: mocks.syncPlan,
	listMyRepositories: mocks.listMyRepositories
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
	mocks.resolveCaller
		.mockClear()
		.mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: null, db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.registerRepositories.mockClear().mockResolvedValue({ mapped: [], unmapped: [] });
	mocks.syncPlan.mockClear().mockResolvedValue({ allowed: [], denied: [] });
	mocks.listMyRepositories.mockClear().mockResolvedValue([]);
});

describe('POST /v1/you/repositories', () => {
	it('registers as the caller and returns mapped + unmapped', async () => {
		const result = {
			mapped: [{ repo_key: 'github.com/acme/api', tenant: 'organization/acme', repo_id: 'r1' }],
			unmapped: [{ repo_key: 'git.internal/x/y', reason: 'unknown_host' }]
		};
		mocks.registerRepositories.mockResolvedValue(result);
		const res = await register.POST(post({ repos: [{ repo_key: 'github.com/acme/api' }] }));
		expect(await res.json()).toEqual(result);
		// The caller's own JWT: dojo.register_repository maps to THEIR tenants (#236).
		expect(mocks.registerRepositories.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
	});

	it('drops entries with no repo_key instead of inventing an identity for them', async () => {
		await register.POST(
			post({ repos: [{ repo_key: 'github.com/acme/api' }, { name: 'nameless' }, { repo_key: '  ' }] })
		);
		expect(mocks.registerRepositories.mock.calls[0][1]).toEqual([
			{ repo_key: 'github.com/acme/api', remote_url: null, name: null }
		]);
	});

	// #299: each repository's sensei project reaches registerRepositories. Dropping
	// it here is how a synced repository would silently never show a project.
	it('passes each repository\'s project through', async () => {
		const project = { slug: 'sensei', name: 'sensei', classification: 'company' };
		await register.POST(post({ repos: [{ repo_key: 'github.com/acme/api', project }] }));
		expect(mocks.registerRepositories.mock.calls[0][1][0].project).toEqual(project);
	});

	it('400s on a malformed project rather than dropping it', async () => {
		for (const project of [{ slug: '', name: 'x', classification: 'company' }, { slug: 's', name: 'n', classification: 'galaxy' }]) {
			const res = await register.POST(post({ repos: [{ repo_key: 'github.com/acme/api', project }] }));
			expect(res.status).toBe(400);
		}
		expect(mocks.registerRepositories).not.toHaveBeenCalled();
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
		// The caller's own JWT: dojo.my_repositories() pins their principal (#236).
		expect(mocks.syncPlan.mock.calls[0]).toEqual([{ client: 'user', token: 'jwt-A' }]);
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

// The repository list and the sync plan are read with the caller's JWT, through
// dojo.my_repositories(), which pins their principal in the database (#236).
describe('GET /v1/you/repositories', () => {
	it('lists the caller\'s repositories with their JWT', async () => {
		mocks.listMyRepositories.mockResolvedValueOnce([{ repo_key: 'github.com/acme/api' }]);
		expect(await (await register.GET(get)).json()).toEqual({ repositories: [{ repo_key: 'github.com/acme/api' }] });
		expect(mocks.listMyRepositories.mock.calls[0]).toEqual([{ client: 'user', token: 'jwt-A' }]);
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await register.GET(get)).status).toBe(401);
		expect(mocks.listMyRepositories).not.toHaveBeenCalled();
	});
});

describe('GET /v1/you/sync/plan without a session', () => {
	it('refuses rather than planning as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await plan.GET(get)).status).toBe(401);
		expect(mocks.syncPlan).not.toHaveBeenCalled();
	});
});

describe('POST /v1/you/repositories without a session', () => {
	it('refuses rather than registering as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await register.POST(post({ repos: [{ repo_key: 'github.com/acme/api' }] }))).status).toBe(401);
		expect(mocks.registerRepositories).not.toHaveBeenCalled();
	});
});
