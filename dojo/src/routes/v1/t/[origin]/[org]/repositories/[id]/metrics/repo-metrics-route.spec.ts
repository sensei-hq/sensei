// Route test for GET /v1/t/{origin}/{org}/repositories/[id]/metrics (#532):
// the member floor, the caller's client, the repo id through to the read,
// the default window unless ?days= says otherwise.
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
	resolveTenantAccess: vi.fn(),
	readRepoMetrics: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/repo-metrics-data', () => ({ readRepoMetrics: mocks.readRepoMetrics }));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));

const route = await import('./+server');

const ev = (query = '') =>
	({
		params: { origin: 'organization', org: 'acme', id: 'r1' },
		request: new Request('http://x/' + query, { headers: { authorization: 'Bearer [REDACTED]' } }),
		locals: {},
		url: new URL('http://x/' + query)
	}) as never;

beforeEach(() => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({
		tenantId: 't1',
		userId: 'lena',
		role: 'member',
		access: 0,
		accessToken: 'jwt-L'
	});
	mocks.readRepoMetrics.mockResolvedValue({ since: '2026-10-16', days: 7, rows: [] });
});

describe('GET /v1/t/{origin}/{org}/repositories/[id]/metrics', () => {
	it('asks for the member floor and reads as the caller', async () => {
		const res = await route.GET(ev());
		expect(res.status).toBe(200);
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(0);
		const [db, tenantId, repoId, opts] = mocks.readRepoMetrics.mock.calls[0];
		expect(db).toEqual({ client: 'user', token: 'jwt-L' });
		expect(tenantId).toBe('t1');
		expect(repoId).toBe('r1');
		expect(opts.days).toBe(7);
		expect(opts.today).toBeInstanceOf(Date);
		expect(await res.json()).toEqual({ since: '2026-10-16', days: 7, rows: [] });
	});

	it('?days= overrides the window; a bad window is 400', async () => {
		await route.GET(ev('?days=30'));
		expect(mocks.readRepoMetrics.mock.calls[0][3].days).toBe(30);
		expect((await route.GET(ev('?days=nope'))).status).toBe(400);
	});

	it('below the floor nothing is read', async () => {
		mocks.resolveTenantAccess.mockRejectedValue(
			new Response(JSON.stringify({ error: 'not a member of this dōjō' }), { status: 403 })
		);
		expect((await route.GET(ev())).status).toBe(403);
		expect(mocks.readRepoMetrics).not.toHaveBeenCalled();
	});
});
