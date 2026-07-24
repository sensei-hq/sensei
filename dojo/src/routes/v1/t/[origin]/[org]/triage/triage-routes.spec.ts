// Route-level tests for the Worker maintainer-triage endpoints (JWT plane):
//   GET  /v1/t/{origin}/{org}/triage                    → { queue }
//   POST /v1/t/{origin}/{org}/triage/{signature}/decide → DecideResult
//
// Auth is the JWT/console plane (`resolveTenantAccess`) at the MAINTAINER floor.
// Covers the auth floor, the returned envelope/shape, decide-body validation
// (400), the 404-when-no-open-row, error mapping, and auth-Response propagation.
// The `triage-data` store is mocked (its own logic is covered by
// `triage-data.spec.ts`); this proves the handler wiring. No live Worker/DB.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller = {
	tenantId: 't1',
	userId: 'user-uuid',
	role: 'maintainer',
	access: 3,
	membershipId: 'm1'
};

class TriageError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	listTriage: vi.fn(),
	parseDecideBody: vi.fn(),
	decideTriage: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({}),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/triage-data', () => ({
	listTriage: mocks.listTriage,
	parseDecideBody: mocks.parseDecideBody,
	decideTriage: mocks.decideTriage,
	TriageError
}));

const { GET } = await import('./+server');
const { POST } = await import('./[signature]/decide/+server');

function req(method: string, body?: unknown): Request {
	return new Request('http://x/', {
		method,
		headers: { authorization: 'Bearer jwt-token' },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
}
function ev(method: string, body?: unknown, signature = 'sig-1') {
	return {
		params: { origin: 'github', org: 'acme', signature },
		request: req(method, body),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.listTriage.mockClear().mockResolvedValue([]);
	mocks.parseDecideBody.mockClear().mockImplementation((b: Record<string, unknown>) => ({ status: b.status }));
	mocks.decideTriage.mockClear().mockResolvedValue({ status: 'approved', artifact_id: 'a1' });
});

describe('GET /triage', () => {
	it('returns { queue } from the store', async () => {
		mocks.listTriage.mockResolvedValueOnce([{ signature: 's', title: 't' }]);
		const res = await GET(ev('GET'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ queue: [{ signature: 's', title: 't' }] });
	});
	it('auths on the JWT plane at the maintainer floor', async () => {
		await GET(ev('GET'));
		const args = mocks.resolveTenantAccess.mock.calls[0];
		expect(args[0]).toBe('github');
		expect(args[1]).toBe('acme');
		expect(args[4]).toBe(3); // ACCESS.maintainer
	});
	it('maps a store TriageError(500) to 500', async () => {
		mocks.listTriage.mockRejectedValueOnce(new TriageError(500, 'boom'));
		const res = await GET(ev('GET'));
		expect(res.status).toBe(500);
	});
	it('propagates a 403 from auth without querying', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		const res = await GET(ev('GET'));
		expect(res.status).toBe(403);
		expect(mocks.listTriage).not.toHaveBeenCalled();
	});
});

describe('POST /triage/{signature}/decide', () => {
	it('records the decision with the caller as maintainer, returns the result', async () => {
		const res = await POST(ev('POST', { status: 'approve', distribution_scope: ['Company'] }));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ status: 'approved', artifact_id: 'a1' });
		// decideTriage(db, tenantId, signature, decision, maintainerId)
		const args = mocks.decideTriage.mock.calls[0];
		expect(args[1]).toBe('t1');
		expect(args[2]).toBe('sig-1');
		expect(args[4]).toBe('user-uuid'); // caller, never the body
	});
	it('auths at the maintainer floor', async () => {
		await POST(ev('POST', { status: 'revise' }));
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(3);
	});
	it('400s when parseDecideBody rejects the body (never decides)', async () => {
		mocks.parseDecideBody.mockImplementationOnce(() => {
			throw new TriageError(400, 'bad status');
		});
		const res = await POST(ev('POST', { status: 'bogus' }));
		expect(res.status).toBe(400);
		expect(mocks.decideTriage).not.toHaveBeenCalled();
	});
	it('404s when the store finds no open row', async () => {
		mocks.decideTriage.mockRejectedValueOnce(new TriageError(404, 'no open row'));
		const res = await POST(ev('POST', { status: 'revise' }));
		expect(res.status).toBe(404);
	});
	it('propagates a 401 from auth', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		const res = await POST(ev('POST', { status: 'revise' }));
		expect(res.status).toBe(401);
	});
});
