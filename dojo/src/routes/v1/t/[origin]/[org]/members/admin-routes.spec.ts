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
	getHealth: vi.fn(),
	getContribVsApprove: vi.fn(),
	setMemberRole: vi.fn(),
	addMember: vi.fn(),
	parseNewMember: vi.fn(),
	upsertPolicy: vi.fn(),
	parseUpsertPolicy: vi.fn(),
	patchPolicy: vi.fn(),
	parsePatchPolicy: vi.fn(),
	deletePolicy: vi.fn(),
	createIdentity: vi.fn(),
	parseNewIdentity: vi.fn(),
	updateIdentity: vi.fn(),
	parsePatchIdentity: vi.fn(),
	deleteIdentity: vi.fn(),
	recordAudit: vi.fn()
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
	setMemberRole: mocks.setMemberRole,
	addMember: mocks.addMember,
	parseNewMember: mocks.parseNewMember,
	upsertPolicy: mocks.upsertPolicy,
	parseUpsertPolicy: mocks.parseUpsertPolicy,
	patchPolicy: mocks.patchPolicy,
	parsePatchPolicy: mocks.parsePatchPolicy,
	deletePolicy: mocks.deletePolicy,
	createIdentity: mocks.createIdentity,
	parseNewIdentity: mocks.parseNewIdentity,
	updateIdentity: mocks.updateIdentity,
	parsePatchIdentity: mocks.parsePatchIdentity,
	deleteIdentity: mocks.deleteIdentity,
	AdminError
}));
vi.mock('$lib/server/health-series', () => ({ getContribVsApprove: mocks.getContribVsApprove }));
vi.mock('$lib/server/audit', () => ({ recordAudit: mocks.recordAudit }));

const members = await import('./+server');
const memberRole = await import('./[userId]/role/+server');
const policies = await import('../policies/+server');
const policyId = await import('../policies/[id]/+server');
const identities = await import('../identities/+server');
const identityId = await import('../identities/[id]/+server');
const audit = await import('../audit/+server');
const health = await import('../health/+server');

function ev(urlStr = 'http://x/', body?: unknown, params: Record<string, string> = {}) {
	return {
		params: { origin: 'github', org: 'acme', ...params },
		request: new Request(urlStr, {
			method: body === undefined ? 'GET' : 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
		}),
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
	mocks.getContribVsApprove.mockClear().mockResolvedValue([]);
	mocks.setMemberRole.mockClear().mockResolvedValue({ user_id: 'u1', role: 'lead' });
	mocks.addMember.mockClear().mockResolvedValue({ id: 'm1', role: 'contributor' });
	mocks.parseNewMember.mockClear().mockReturnValue({ user_id: 'u1', kind: 'client', authenticated_via: 'sso', role: 'contributor' });
	mocks.upsertPolicy.mockClear().mockResolvedValue({ id: 'p1', scope_key: 'all-org' });
	mocks.parseUpsertPolicy.mockClear().mockReturnValue({ scope_key: 'all-org' });
	mocks.patchPolicy.mockClear().mockResolvedValue({ id: 'p1' });
	mocks.parsePatchPolicy.mockClear().mockReturnValue({ retention_days: 90 });
	mocks.deletePolicy.mockClear().mockResolvedValue(true);
	mocks.createIdentity.mockClear().mockResolvedValue({ id: 'id1' });
	mocks.parseNewIdentity.mockClear().mockReturnValue({ user_id: 'u1', provider: 'sso', subject: 's' });
	mocks.updateIdentity.mockClear().mockResolvedValue({ id: 'id1' });
	mocks.parsePatchIdentity.mockClear().mockReturnValue({ email: 'a@b.co' });
	mocks.deleteIdentity.mockClear().mockResolvedValue(true);
	mocks.recordAudit.mockClear().mockResolvedValue(undefined);
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
	it('returns the rollup plus the contrib-vs-approve weekly series', async () => {
		mocks.getHealth.mockResolvedValueOnce({ connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 1 });
		mocks.getContribVsApprove.mockResolvedValueOnce([{ wk: 'W4', c: 2, a: 1 }]);
		expect(await (await health.GET(ev())).json()).toEqual({
			connections: 3,
			queue_depth: 12,
			publish_rate_1h: 5,
			error_rate_1h: 1,
			contrib_vs_approve: [{ wk: 'W4', c: 2, a: 1 }]
		});
	});
	it('auths at the admin floor', async () => {
		await health.GET(ev());
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4);
	});
});

// ── mutation routes (admin floor · audited) ──────────────────────────────────

describe('PATCH /members/{userId}/role', () => {
	it('sets the role, audits, and returns { user_id, role } at the admin floor', async () => {
		const res = await memberRole.PATCH(ev('http://x/', { role: 'lead' }, { userId: 'u1' }));
		expect(await res.json()).toEqual({ user_id: 'u1', role: 'lead' });
		expect(mocks.setMemberRole.mock.calls[0]).toEqual([{}, 't1', 'u1', 'lead']);
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4);
		expect(mocks.recordAudit).toHaveBeenCalledOnce();
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'role_changed', target: 'u1' });
	});
	it('maps AdminError(400) from a bad role', async () => {
		mocks.setMemberRole.mockRejectedValueOnce(new AdminError(400, 'bad role'));
		expect((await memberRole.PATCH(ev('http://x/', { role: 'x' }, { userId: 'u1' }))).status).toBe(400);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
	it('propagates a 403 without writing', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await memberRole.PATCH(ev('http://x/', { role: 'lead' }, { userId: 'u1' }))).status).toBe(403);
		expect(mocks.setMemberRole).not.toHaveBeenCalled();
	});
});

describe('POST /members (provision)', () => {
	it('provisions, audits, and returns { id, role }', async () => {
		const res = await members.POST(ev('http://x/', { user_id: 'u1', kind: 'client', authenticated_via: 'sso' }));
		expect(await res.json()).toEqual({ id: 'm1', role: 'contributor' });
		// Rule C: no dojo_url arg — the store gets (db, tenantId, input); dōjō url
		// is derived from the tenant. The parsed member input is forwarded.
		expect(mocks.addMember.mock.calls[0][1]).toBe('t1');
		expect(mocks.addMember.mock.calls[0][2]).toMatchObject({ user_id: 'u1', kind: 'client' });
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'member_added' });
	});
	it('maps a 409 duplicate from the store', async () => {
		mocks.addMember.mockRejectedValueOnce(new AdminError(409, 'already a member'));
		expect((await members.POST(ev('http://x/', {}))).status).toBe(409);
	});
});

describe('POST/PATCH/DELETE /policies', () => {
	it('POST upserts, audits, returns { id, scope_key }', async () => {
		const res = await policies.POST(ev('http://x/', { scope_key: 'all-org' }));
		expect(await res.json()).toEqual({ id: 'p1', scope_key: 'all-org' });
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'policy_edited', target: 'all-org' });
	});
	it('PATCH updates by id, audits, returns { id }', async () => {
		const res = await policyId.PATCH(ev('http://x/', { retention_days: 90 }, { id: 'p1' }));
		expect(await res.json()).toEqual({ id: 'p1' });
		expect(mocks.patchPolicy.mock.calls[0][2]).toBe('p1');
	});
	it('DELETE removes, audits, returns { deleted: true }', async () => {
		const res = await policyId.DELETE(ev('http://x/', {}, { id: 'p1' }));
		expect(await res.json()).toEqual({ deleted: true });
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'policy_deleted' });
	});
	it('DELETE 404s (and skips audit) when nothing matched', async () => {
		mocks.deletePolicy.mockResolvedValueOnce(false);
		expect((await policyId.DELETE(ev('http://x/', {}, { id: 'p9' }))).status).toBe(404);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
});

describe('POST/PATCH/DELETE /identities', () => {
	it('POST creates, audits, returns { id }', async () => {
		const res = await identities.POST(ev('http://x/', { user_id: 'u1', provider: 'sso', subject: 's' }));
		expect(await res.json()).toEqual({ id: 'id1' });
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'identity_added' });
	});
	it('PATCH updates by id, returns { id }', async () => {
		const res = await identityId.PATCH(ev('http://x/', { email: 'a@b.co' }, { id: 'id1' }));
		expect(await res.json()).toEqual({ id: 'id1' });
	});
	it('DELETE removes, returns { deleted: true }; 404 when none', async () => {
		expect(await (await identityId.DELETE(ev('http://x/', {}, { id: 'id1' }))).json()).toEqual({ deleted: true });
		mocks.deleteIdentity.mockResolvedValueOnce(false);
		expect((await identityId.DELETE(ev('http://x/', {}, { id: 'id9' }))).status).toBe(404);
	});
	it('propagates a 403 without writing', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await identities.POST(ev('http://x/', {}))).status).toBe(403);
		expect(mocks.createIdentity).not.toHaveBeenCalled();
	});
});
