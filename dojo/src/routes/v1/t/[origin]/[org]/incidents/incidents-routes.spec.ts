// Route-level tests for the Worker lead incidents endpoint (JWT plane):
//   GET /v1/t/{origin}/{org}/incidents → { incidents, open_count }
//
// On the JWT/console plane (`resolveTenantAccess`) at the LEAD floor. Covers the
// auth floor, the returned envelope, error mapping, and auth-Response
// propagation. The `incidents-data` store is mocked (its own logic is in
// `incidents-data.spec.ts`). No live Worker/DB.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller = { tenantId: 't1', userId: 'lead-uuid', role: 'lead', access: 2, membershipId: 'm1' };

class IncidentsError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	listIncidents: vi.fn()
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
vi.mock('$lib/server/incidents-data', () => ({
	listIncidents: mocks.listIncidents,
	IncidentsError
}));

const { GET } = await import('./+server');

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
	mocks.listIncidents.mockClear().mockResolvedValue({ incidents: [], open_count: 0 });
});

describe('GET /incidents', () => {
	it('returns { incidents, open_count } and auths at the lead floor', async () => {
		mocks.listIncidents.mockResolvedValueOnce({ incidents: [{ id: 'i1' }], open_count: 1 });
		const res = await GET(ev());
		expect(await res.json()).toEqual({ incidents: [{ id: 'i1' }], open_count: 1 });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2); // ACCESS.lead
	});
	it('maps IncidentsError(500) to 500', async () => {
		mocks.listIncidents.mockRejectedValueOnce(new IncidentsError(500, 'boom'));
		expect((await GET(ev())).status).toBe(500);
	});
	it('propagates a 403 from auth without querying', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await GET(ev())).status).toBe(403);
		expect(mocks.listIncidents).not.toHaveBeenCalled();
	});
});
