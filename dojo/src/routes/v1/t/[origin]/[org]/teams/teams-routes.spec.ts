// Route tests for a dōjō's teams (#307 A3, D7): list, create, rename, delete,
// membership and a team-wide workspace grant. ADMIN floor; the caller's own JWT;
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
	listTeams: vi.fn(),
	createTeam: vi.fn(),
	renameTeam: vi.fn(),
	deleteTeam: vi.fn(),
	addTeamMember: vi.fn(),
	removeTeamMember: vi.fn(),
	grantTeamWorkspaceView: vi.fn()
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
vi.mock('$lib/server/teams-data', () => ({ ...mocks, AdminError }));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));

const list = await import('./+server');
const one = await import('./[teamId]/+server');
const member = await import('./[teamId]/members/[principalId]/+server');
const workspace = await import('./[teamId]/workspaces/[workspaceId]/+server');

const ev = (params: Record<string, string> = {}, body?: unknown) =>
	({
		params: { origin: 'organization', org: 'acme', ...params },
		request: new Request('http://x/', {
			method: body === undefined ? 'GET' : 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	}) as never;

const USER = { client: 'user', token: 'jwt-A' };

beforeEach(() => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({ tenantId: 't1', userId: 'admin', role: 'admin', access: 4, accessToken: 'jwt-A' });
});

describe('the teams routes', () => {
	it('every write asks for the admin floor; the list for a member', async () => {
		mocks.listTeams.mockResolvedValue([]);
		await list.GET(ev());
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(0);
		await list.POST(ev({}, { name: 'Platform' }));
		await one.PATCH(ev({ teamId: 'tm1' }, { name: 'X' }));
		await one.DELETE(ev({ teamId: 'tm1' }));
		await member.PUT(ev({ teamId: 'tm1', principalId: 'u1' }));
		await member.DELETE(ev({ teamId: 'tm1', principalId: 'u1' }));
		await workspace.PUT(ev({ teamId: 'tm1', workspaceId: 'p1' }));
		for (const call of mocks.resolveTenantAccess.mock.calls.slice(1)) expect(call[4]).toBe(4);
	});

	it('GET lists, POST creates (201), and each write runs as the caller on this dōjō', async () => {
		mocks.listTeams.mockResolvedValue([{ id: 'tm1', name: 'Payments', members: [] }]);
		expect(await (await list.GET(ev())).json()).toEqual({ teams: [{ id: 'tm1', name: 'Payments', members: [] }] });
		// Read as the caller; named through the service client — a non-admin member
		// may read the team's ids but not the identities (access matrix).
		expect(mocks.listTeams).toHaveBeenCalledWith(USER, 't1', { client: 'service' });
		expect((await list.POST(ev({}, { name: 'Platform' }))).status).toBe(201);
		expect(mocks.createTeam).toHaveBeenCalledWith(USER, 't1', 'Platform');
		expect((await one.PATCH(ev({ teamId: 'tm1' }, { name: 'Core' }))).status).toBe(204);
		expect(mocks.renameTeam).toHaveBeenCalledWith(USER, 't1', 'tm1', 'Core');
		expect((await one.DELETE(ev({ teamId: 'tm1' }))).status).toBe(204);
		expect(mocks.deleteTeam).toHaveBeenCalledWith(USER, 't1', 'tm1');
		expect((await member.PUT(ev({ teamId: 'tm1', principalId: 'u1' }))).status).toBe(204);
		expect(mocks.addTeamMember).toHaveBeenCalledWith(USER, 't1', 'tm1', 'u1');
		expect((await member.DELETE(ev({ teamId: 'tm1', principalId: 'u1' }))).status).toBe(204);
		expect(mocks.removeTeamMember).toHaveBeenCalledWith(USER, 't1', 'tm1', 'u1');
		expect((await workspace.PUT(ev({ teamId: 'tm1', workspaceId: 'p1' }))).status).toBe(204);
		expect(mocks.grantTeamWorkspaceView).toHaveBeenCalledWith(USER, 't1', 'tm1', 'p1');
	});

	it('a body without a name is a 400 before anything is written', async () => {
		expect((await list.POST(ev({}, {}))).status).toBe(400);
		expect(mocks.createTeam).not.toHaveBeenCalled();
	});

	it('a refusal keeps its status and says why', async () => {
		mocks.createTeam.mockRejectedValue(new AdminError(409, 'that name is taken'));
		const res = await list.POST(ev({}, { name: 'Payments' }));
		expect(res.status).toBe(409);
		expect(await res.json()).toEqual({ error: 'that name is taken' });
	});
});
