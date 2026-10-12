// Route-level tests for GET /v1/t/{origin}/{org}/projects — the org projects
// read. Covers the LEAD floor, the { projects } envelope, AdminError → status,
// and auth-Response propagation. The store is mocked.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller: Record<string, unknown> = {
	tenantId: 't1',
	userId: 'lead-uuid',
	role: 'lead',
	access: 3,
	membershipId: 'm1',
	accessToken: 'jwt-A'
};

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
	listOrgProjects: vi.fn(),
	dojoDb: vi.fn(() => ({ client: 'service' })),
	userDb: vi.fn((token: string) => ({ client: 'user', token }))
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: mocks.dojoDb, userDb: mocks.userDb }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/projects-data', () => ({ listOrgProjects: mocks.listOrgProjects, AdminError }));

const route = await import('./+server');

function ev() {
	return {
		params: { origin: 'github', org: 'acme' },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.listOrgProjects.mockClear().mockResolvedValue([]);
	mocks.dojoDb.mockClear();
	mocks.userDb.mockClear();
	caller.accessToken = 'jwt-A';
});

describe('GET /projects', () => {
	it('returns { projects } at the LEAD floor', async () => {
		mocks.listOrgProjects.mockResolvedValueOnce([{ id: 'p1', slug: 'acme/x' }]);
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({ projects: [{ id: 'p1', slug: 'acme/x' }] });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(3); // ACCESS.lead
	});
	it('maps an AdminError(500) to 500 (fail-closed, never a fixture)', async () => {
		mocks.listOrgProjects.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});
	it('propagates a 403 without reading', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.GET(ev())).status).toBe(403);
		expect(mocks.listOrgProjects).not.toHaveBeenCalled();
	});
	it('reads as the signed-in person, so the projects policy decides the rows (#236)', async () => {
		await route.GET(ev());
		expect(mocks.userDb).toHaveBeenCalledWith('jwt-A');
		expect(mocks.listOrgProjects.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		expect(mocks.dojoDb).not.toHaveBeenCalled();
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		caller.accessToken = undefined;
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.dojoDb).not.toHaveBeenCalled();
		expect(mocks.listOrgProjects).not.toHaveBeenCalled();
	});
});
