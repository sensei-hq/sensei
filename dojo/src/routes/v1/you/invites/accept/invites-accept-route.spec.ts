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

const mocks = vi.hoisted(() => ({
	resolveCaller: vi.fn(),
	acceptInvite: vi.fn(),
	takeAttempt: vi.fn(),
	releaseAttempt: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({ client: 'service' }) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/invites-data', () => ({
	acceptInvite: mocks.acceptInvite,
	AdminError
}));
vi.mock('$lib/server/rate-limit', () => ({ takeAttempt: mocks.takeAttempt, releaseAttempt: mocks.releaseAttempt }));

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
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', email: 'ada@acme.co', accessToken: 'jwt-A' });
	mocks.acceptInvite.mockClear().mockResolvedValue({ tenant_id: 't1', role: 'contributor' });
	mocks.takeAttempt.mockReset().mockResolvedValue('attempt-1');
	mocks.releaseAttempt.mockReset().mockResolvedValue(undefined);
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

// System work by name (#236, access matrix "What stays service_role"): the caller
// is resolved for WHO they are; the write runs as the service role, explicitly.
describe('invite acceptance is the system\'s', () => {
	it('accepts with the service role, named — the joining person has no membership yet', async () => {
		await route.POST(ev({ token: 'tok' }));
		expect(mocks.acceptInvite.mock.calls[0][0]).toEqual({ client: 'service' });
	});
});

// #311: failed accepts are counted per person. An attempt is TAKEN atomically
// before any invite is read (dojo.take_attempt — concurrent guesses cannot all
// pass a check made before any of them records, #308 R5b review 2), and given
// back when the accept did not fail as the caller's error.
describe('accept attempts are bounded (#311)', () => {
	it('refuses with 429 when the caller has no attempts left, reading no invite', async () => {
		mocks.takeAttempt.mockResolvedValueOnce(null);
		const res = await route.POST(ev({ token: 'tok' }));
		expect(res.status).toBe(429);
		expect(mocks.acceptInvite).not.toHaveBeenCalled();
		expect(mocks.takeAttempt).toHaveBeenCalledWith({ client: 'service' }, 'u1', 'invite_accept');
	});

	it('keeps the attempt of a failed accept, and still says why it failed', async () => {
		mocks.acceptInvite.mockRejectedValueOnce(new AdminError(404, 'invalid or expired invite'));
		const res = await route.POST(ev({ token: 'nope' }));
		expect(res.status).toBe(404);
		expect(mocks.releaseAttempt).not.toHaveBeenCalled();
	});

	it('a successful accept gives its attempt back — it counts no failure', async () => {
		expect((await route.POST(ev({ token: 'tok' }))).status).toBe(200);
		expect(mocks.releaseAttempt).toHaveBeenCalledWith({ client: 'service' }, 'attempt-1');
	});

	it('a server failure gives the attempt back too — it was not a guess', async () => {
		mocks.acceptInvite.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await route.POST(ev({ token: 'tok' }))).status).toBe(500);
		expect(mocks.releaseAttempt).toHaveBeenCalledWith({ client: 'service' }, 'attempt-1');
	});
});
