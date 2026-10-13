// GET /v1/you/memberships — the signed-in person's own memberships, for the
// daemon to record which persona each local membership belongs to (#236 step 4c).
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), listMyMemberships: vi.fn() }));
const SESSION = vi.hoisted(() => ({ as: 'session' }));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({ as: 'system' }), userDb: () => SESSION }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/dojo-orgs', () => ({ listMyMemberships: mocks.listMyMemberships }));

const { GET } = await import('./+server');
const ev = () => ({ request: new Request('http://x/v1/you/memberships'), locals: {} }) as never;

beforeEach(() => {
	mocks.resolveCaller.mockReset().mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: null, accessToken: 'jwt' });
	mocks.listMyMemberships.mockReset().mockResolvedValue([
		{ id: 'm1', tenant_key: 'organization/acme', role: 'contributor', kind: 'employer' }
	]);
});

describe('GET /v1/you/memberships', () => {
	it('lists the caller’s memberships, read as them', async () => {
		const res = await GET(ev());
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({
			memberships: [{ id: 'm1', tenant_key: 'organization/acme', role: 'contributor', kind: 'employer' }]
		});
		expect(mocks.listMyMemberships).toHaveBeenCalledWith(SESSION, 'p1');
	});

	it('propagates the auth refusal', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('{}', { status: 401 }));
		expect((await GET(ev())).status).toBe(401);
		expect(mocks.listMyMemberships).not.toHaveBeenCalled();
	});
});
