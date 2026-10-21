// Route tests for the dōjō's viewers (#307 A3, D8): list, remove, grant and revoke
// a workspace. ADMIN floor; run with the caller's own JWT (the database decides);
// a refusal keeps its status. Store mocked.
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
	listDojoViewers: vi.fn(),
	removeDojoViewer: vi.fn(),
	grantWorkspaceView: vi.fn(),
	revokeWorkspaceView: vi.fn()
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
vi.mock('$lib/server/viewers-data', () => ({ ...mocks, AdminError }));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));

const list = await import('./+server');
const one = await import('./[principalId]/+server');
const grant = await import('./[principalId]/workspaces/[workspaceId]/+server');

const ev = (params: Record<string, string> = {}) =>
	({
		params: { origin: 'organization', org: 'acme', ...params },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	}) as never;

const USER = { client: 'user', token: 'jwt-A' };

beforeEach(() => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({ tenantId: 't1', userId: 'admin', role: 'admin', access: 4, accessToken: 'jwt-A' });
});

describe('the viewers routes', () => {
	it('every route asks for the admin floor', async () => {
		mocks.listDojoViewers.mockResolvedValue([]);
		await list.GET(ev());
		await one.DELETE(ev({ principalId: 'v1' }));
		await grant.PUT(ev({ principalId: 'v1', workspaceId: 'p1' }));
		await grant.DELETE(ev({ principalId: 'v1', workspaceId: 'p1' }));
		for (const call of mocks.resolveTenantAccess.mock.calls) expect(call[4]).toBe(4);
		expect(mocks.resolveTenantAccess).toHaveBeenCalledTimes(4);
	});

	it("GET lists the dōjō's viewers as the caller", async () => {
		mocks.listDojoViewers.mockResolvedValue([{ principal_id: 'v1', workspaces: [] }]);
		const res = await list.GET(ev());
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ viewers: [{ principal_id: 'v1', workspaces: [] }] });
		expect(mocks.listDojoViewers).toHaveBeenCalledWith(USER, 't1');
	});

	it('DELETE removes a viewer; PUT/DELETE grant and revoke a workspace', async () => {
		expect((await one.DELETE(ev({ principalId: 'v1' }))).status).toBe(204);
		expect(mocks.removeDojoViewer).toHaveBeenCalledWith(USER, 't1', 'v1');
		expect((await grant.PUT(ev({ principalId: 'v1', workspaceId: 'p1' }))).status).toBe(204);
		expect(mocks.grantWorkspaceView).toHaveBeenCalledWith(USER, 't1', 'p1', 'v1');
		expect((await grant.DELETE(ev({ principalId: 'v1', workspaceId: 'p1' }))).status).toBe(204);
		expect(mocks.revokeWorkspaceView).toHaveBeenCalledWith(USER, 't1', 'p1', 'v1');
	});

	it('a refusal keeps its status and says why', async () => {
		mocks.grantWorkspaceView.mockRejectedValue(new AdminError(409, 'invite them to this dōjō as a viewer first'));
		const res = await grant.PUT(ev({ principalId: 'v1', workspaceId: 'p1' }));
		expect(res.status).toBe(409);
		expect(await res.json()).toEqual({ error: 'invite them to this dōjō as a viewer first' });
	});

	it('a failed list is an error, never an empty one', async () => {
		mocks.listDojoViewers.mockRejectedValue(new AdminError(500, 'boom'));
		expect((await list.GET(ev())).status).toBe(500);
	});
});
