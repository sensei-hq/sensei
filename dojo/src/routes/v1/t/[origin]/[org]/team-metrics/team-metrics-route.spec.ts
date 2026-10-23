// Route test for the Teams view (#307 R8, D38): the MEMBER floor (a viewer, who is
// no member, is refused before anything is read — the role decides what comes
// back, per person or totals only); the reads run as the caller, the names
// through the service client; a bad window is a 400; a refusal keeps its status.
// Store mocked.
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
	readTeamMetrics: vi.fn()
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
vi.mock('$lib/server/team-metrics-data', () => ({ readTeamMetrics: mocks.readTeamMetrics }));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));

const route = await import('./+server');

const ev = (query = '') =>
	({
		params: { origin: 'organization', org: 'acme' },
		request: new Request('http://x/' + query, { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/' + query)
	}) as never;

beforeEach(() => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({
		tenantId: 't1',
		userId: 'lena',
		role: 'lead',
		access: 3,
		accessToken: 'jwt-L'
	});
	mocks.readTeamMetrics.mockResolvedValue({ since: '2026-10-16', days: 7, teams: [] });
});

describe('GET /v1/t/{origin}/{org}/team-metrics', () => {
	it('asks for the member floor', async () => {
		await route.GET(ev());
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(0);
	});

	it('below the floor nothing is read', async () => {
		mocks.resolveTenantAccess.mockRejectedValue(
			new Response(JSON.stringify({ error: 'not a member of this dōjō' }), { status: 403 })
		);
		expect((await route.GET(ev())).status).toBe(403);
		expect(mocks.readTeamMetrics).not.toHaveBeenCalled();
	});

	it('reads as the caller over the default week, names through the service client', async () => {
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({ since: '2026-10-16', days: 7, teams: [] });
		const [db, nameDb, caller, opts] = mocks.readTeamMetrics.mock.calls[0];
		expect(db).toEqual({ client: 'user', token: 'jwt-L' });
		expect(nameDb).toEqual({ client: 'service' });
		expect(caller).toMatchObject({ tenantId: 't1', userId: 'lena', role: 'lead' });
		expect(opts.days).toBe(7);
		expect(opts.today).toBeInstanceOf(Date);
	});

	it('takes a window of 1 to 90 days; anything else is a 400 before anything is read', async () => {
		await route.GET(ev('?days=30'));
		expect(mocks.readTeamMetrics.mock.calls[0][3].days).toBe(30);
		for (const bad of ['0', '91', 'abc', '7.5', '-3', '']) {
			expect((await route.GET(ev('?days=' + bad))).status).toBe(400);
		}
		expect(mocks.readTeamMetrics).toHaveBeenCalledTimes(1);
	});

	it('a failed read keeps its status — never an empty view', async () => {
		mocks.readTeamMetrics.mockRejectedValue(new AdminError(500, 'boom'));
		const res = await route.GET(ev());
		expect(res.status).toBe(500);
		expect(await res.json()).toEqual({ error: 'boom' });
	});
});
