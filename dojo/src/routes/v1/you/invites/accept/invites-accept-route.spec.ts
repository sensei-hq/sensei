// Route tests for POST /v1/you/invites/accept — the invitee redeems an invite
// (F3b). resolveCaller passes the JWT-verified email to the gate; success →
// { tenant_id, role }; a gate rejection (403/410) maps to status; 401 propagates.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), acceptInvite: vi.fn() }));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/invites-data', () => ({ acceptInvite: mocks.acceptInvite, AdminError }));

const route = await import('./+server');

function ev(body: unknown) {
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

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', email: 'ada@acme.co', db: {} });
	mocks.acceptInvite.mockClear().mockResolvedValue({ tenant_id: 't1', role: 'contributor' });
});

describe('POST /v1/you/invites/accept', () => {
	it('passes the caller email to the gate and returns { tenant_id, role }', async () => {
		const res = await route.POST(ev({ token: 'tok' }));
		expect(await res.json()).toEqual({ tenant_id: 't1', role: 'contributor' });
		// (db, userId, email, token, now)
		expect(mocks.acceptInvite.mock.calls[0][2]).toBe('ada@acme.co');
		expect(mocks.acceptInvite.mock.calls[0][3]).toBe('tok');
	});
	it('maps a 403 email-mismatch (the real gate) to 403', async () => {
		mocks.acceptInvite.mockRejectedValueOnce(new AdminError(403, 'this invite is for a different email'));
		expect((await route.POST(ev({ token: 'tok' }))).status).toBe(403);
	});
	it('maps a 410 expired invite to 410', async () => {
		mocks.acceptInvite.mockRejectedValueOnce(new AdminError(410, 'invite expired'));
		expect((await route.POST(ev({ token: 'tok' }))).status).toBe(410);
	});
	it('propagates a 401 without accepting', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await route.POST(ev({ token: 'tok' }))).status).toBe(401);
		expect(mocks.acceptInvite).not.toHaveBeenCalled();
	});
});
