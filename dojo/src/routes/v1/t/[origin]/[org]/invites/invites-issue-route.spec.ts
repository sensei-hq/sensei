// Route tests for POST /v1/t/{origin}/{org}/invites — admin issues an invite
// (F3b). ADMIN floor; 201 + the invite; parse 400 / 403 propagate. Store mocked.
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
	createInvite: vi.fn(),
	parseNewInvite: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/invites-data', () => ({
	createInvite: mocks.createInvite,
	parseNewInvite: mocks.parseNewInvite,
	AdminError
}));

const route = await import('./+server');

function ev(body: unknown) {
	return {
		params: { origin: 'github', org: 'acme' },
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue({ tenantId: 't1', userId: 'admin-uid', role: 'admin', access: 4 });
	mocks.parseNewInvite.mockClear().mockImplementation((b) => ({ email: b.email, role: 'contributor', kind: b.kind }));
	mocks.createInvite.mockClear().mockResolvedValue({ id: 'inv1', token: 'tok', email: 'a@b.co', role: 'contributor', expires_at: 'x' });
});

describe('POST /v1/t/{origin}/{org}/invites', () => {
	it('issues at the ADMIN floor and returns 201 + the invite', async () => {
		const res = await route.POST(ev({ email: 'a@b.co', kind: 'client' }));
		expect(res.status).toBe(201);
		expect(await res.json()).toMatchObject({ id: 'inv1', token: 'tok' });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4); // ACCESS.admin
		expect(mocks.createInvite.mock.calls[0][1]).toBe('t1'); // tenant
		expect(mocks.createInvite.mock.calls[0][2]).toBe('admin-uid'); // invited_by
	});
	it('maps a parse 400 without issuing', async () => {
		mocks.parseNewInvite.mockImplementationOnce(() => {
			throw new AdminError(400, 'a valid email is required');
		});
		expect((await route.POST(ev({ kind: 'client' }))).status).toBe(400);
		expect(mocks.createInvite).not.toHaveBeenCalled();
	});
	it('propagates a 403 (below admin) without issuing', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.POST(ev({ email: 'a@b.co', kind: 'client' }))).status).toBe(403);
		expect(mocks.createInvite).not.toHaveBeenCalled();
	});
});
