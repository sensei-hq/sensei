// Route-level tests for the Worker admin-console read endpoints (JWT plane):
//   GET /v1/t/{origin}/{org}/members     → { members }
//   GET /v1/t/{origin}/{org}/policies    → { policies }
//   GET /v1/t/{origin}/{org}/identities  → { identities }
//   GET /v1/t/{origin}/{org}/audit       → { events }
//   GET /v1/t/{origin}/{org}/health      → HealthRollup (bare)
//
// All on the JWT/console plane (`resolveTenantAccess`) at the ADMIN floor. Covers
// the auth floor, each returned envelope/shape, the audit limit forwarding, error
// mapping, and auth-Response propagation. The `admin-data` store is mocked (its
// own logic is in `admin-data.spec.ts`). No live Worker/DB.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller = { tenantId: 't1', userId: 'admin-uuid', role: 'admin', access: 4, membershipId: 'm1' };

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
	listMembers: vi.fn(),
	listPolicies: vi.fn(),
	listIdentities: vi.fn(),
	listAudit: vi.fn(),
	getHealth: vi.fn()
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
vi.mock('$lib/server/admin-data', () => ({
	listMembers: mocks.listMembers,
	listPolicies: mocks.listPolicies,
	listIdentities: mocks.listIdentities,
	listAudit: mocks.listAudit,
	getHealth: mocks.getHealth,
	AdminError
}));

const members = await import('./+server');
const policies = await import('../policies/+server');
const identities = await import('../identities/+server');
const audit = await import('../audit/+server');
const health = await import('../health/+server');

function ev(urlStr = 'http://x/') {
	return {
		params: { origin: 'github', org: 'acme' },
		request: new Request(urlStr, { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL(urlStr)
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.listMembers.mockClear().mockResolvedValue([]);
	mocks.listPolicies.mockClear().mockResolvedValue([]);
	mocks.listIdentities.mockClear().mockResolvedValue([]);
	mocks.listAudit.mockClear().mockResolvedValue([]);
	mocks.getHealth.mockClear().mockResolvedValue({ connections: 0, queue_depth: 0, publish_rate_1h: 0, error_rate_1h: 0 });
});

describe('GET /members', () => {
	it('returns { members } and auths at the admin floor', async () => {
		mocks.listMembers.mockResolvedValueOnce([{ id: 'm1' }]);
		const res = await members.GET(ev());
		expect(await res.json()).toEqual({ members: [{ id: 'm1' }] });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4); // ACCESS.admin
	});
	it('maps AdminError(500) to 500', async () => {
		mocks.listMembers.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await members.GET(ev())).status).toBe(500);
	});
	it('propagates a 403 from auth without querying', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await members.GET(ev())).status).toBe(403);
		expect(mocks.listMembers).not.toHaveBeenCalled();
	});
});

describe('GET /policies', () => {
	it('returns { policies }', async () => {
		mocks.listPolicies.mockResolvedValueOnce([{ id: 'p1' }]);
		expect(await (await policies.GET(ev())).json()).toEqual({ policies: [{ id: 'p1' }] });
	});
});

describe('GET /identities', () => {
	it('returns { identities }', async () => {
		mocks.listIdentities.mockResolvedValueOnce([{ id: 'id1' }]);
		expect(await (await identities.GET(ev())).json()).toEqual({ identities: [{ id: 'id1' }] });
	});
});

describe('GET /audit', () => {
	it('returns { events }', async () => {
		mocks.listAudit.mockResolvedValueOnce([{ id: 1 }]);
		expect(await (await audit.GET(ev())).json()).toEqual({ events: [{ id: 1 }] });
	});
	it('forwards a numeric ?limit= to the store', async () => {
		await audit.GET(ev('http://x/?limit=25'));
		expect(mocks.listAudit.mock.calls[0][2]).toBe(25);
	});
	it('passes undefined limit when ?limit= is absent/non-numeric (store default)', async () => {
		await audit.GET(ev('http://x/'));
		expect(mocks.listAudit.mock.calls[0][2]).toBeUndefined();
		mocks.listAudit.mockClear();
		await audit.GET(ev('http://x/?limit=abc'));
		expect(mocks.listAudit.mock.calls[0][2]).toBeUndefined();
	});
});

describe('GET /health', () => {
	it('returns the bare rollup', async () => {
		mocks.getHealth.mockResolvedValueOnce({ connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 1 });
		expect(await (await health.GET(ev())).json()).toEqual({ connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 1 });
	});
	it('auths at the admin floor', async () => {
		await health.GET(ev());
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4);
	});
});
