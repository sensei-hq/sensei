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
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

const caller: Record<string, unknown> = {
	tenantId: 't1',
	userId: 'admin-uuid',
	role: 'admin',
	access: 4,
	membershipId: 'm1',
	accessToken: 'jwt-A'
};

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
	upsertPolicy: vi.fn(),
	parseUpsertPolicy: vi.fn(),
	patchPolicy: vi.fn(),
	parsePatchPolicy: vi.fn(),
	deletePolicy: vi.fn(),
	recordAudit: vi.fn()
}));

// The service role vs the caller's own JWT — which client each store call got.
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
vi.mock('$lib/server/admin-data', () => ({
	listMembers: mocks.listMembers,
	listPolicies: mocks.listPolicies,
	listIdentities: mocks.listIdentities,
	listAudit: mocks.listAudit,
	getHealth: mocks.getHealth,
	setMemberRole: mocks.setMemberRole,
	upsertPolicy: mocks.upsertPolicy,
	parseUpsertPolicy: mocks.parseUpsertPolicy,
	patchPolicy: mocks.patchPolicy,
	parsePatchPolicy: mocks.parsePatchPolicy,
	deletePolicy: mocks.deletePolicy,
	AdminError
}));
vi.mock('$lib/server/health-series', () => ({ getContribVsApprove: mocks.getContribVsApprove }));
vi.mock('$lib/server/audit', () => ({ recordAudit: mocks.recordAudit }));

const members = await import('./+server');
const memberRole = await import('./[userId]/role/+server');
const policies = await import('../policies/+server');
const policyId = await import('../policies/[id]/+server');
const identities = await import('../identities/+server');
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
	mocks.upsertPolicy.mockClear().mockResolvedValue({ id: 'p1', scope_key: 'all-org' });
	mocks.parseUpsertPolicy.mockClear().mockReturnValue({ scope_key: 'all-org' });
	mocks.patchPolicy.mockClear().mockResolvedValue({ id: 'p1' });
	mocks.parsePatchPolicy.mockClear().mockReturnValue({ retention_days: 90 });
	mocks.deletePolicy.mockClear().mockResolvedValue(true);
	mocks.recordAudit.mockClear().mockResolvedValue(undefined);
	caller.accessToken = 'jwt-A';
});

// Members and identities are read — and a role set — as the signed-in admin, so
// the memberships and identities policies decide (#236). The role change's audit
// row is the system's.
describe('members and identities run as the signed-in admin (#236)', () => {
	it('GET /members, PATCH a role and GET /identities use the caller\'s JWT', async () => {
		await members.GET(ev());
		await memberRole.PATCH(ev('http://x/', { role: 'lead' }, { userId: 'u1' }));
		await identities.GET(ev());
		for (const m of [mocks.listMembers, mocks.setMemberRole, mocks.listIdentities]) {
			expect(m.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		}
		expect(mocks.recordAudit.mock.calls[0][0]).toEqual({ client: 'service' });
	});
	it('refuses a caller with no session rather than acting as the service role', async () => {
		caller.accessToken = undefined;
		expect((await members.GET(ev())).status).toBe(401);
		expect((await memberRole.PATCH(ev('http://x/', { role: 'lead' }, { userId: 'u1' }))).status).toBe(401);
		expect((await identities.GET(ev())).status).toBe(401);
		expect(mocks.listMembers).not.toHaveBeenCalled();
		expect(mocks.setMemberRole).not.toHaveBeenCalled();
		expect(mocks.listIdentities).not.toHaveBeenCalled();
	});
});

describe('GET /audit runs as the signed-in admin (#236)', () => {
	it('reads with the caller\'s JWT, so the audit_events policy decides the rows', async () => {
		await audit.GET(ev());
		expect(mocks.listAudit.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		caller.accessToken = undefined;
		expect((await audit.GET(ev())).status).toBe(401);
		expect(mocks.listAudit).not.toHaveBeenCalled();
	});
});

// Policies are read and written as the signed-in admin, so the policies table's
// policy decides (#236). The audit row is the system's record of the action:
// written by the server from the resolved caller, never by the person's session.
describe('policies run as the signed-in person (#236)', () => {
	it('GET, POST, PATCH and DELETE use the caller\'s JWT; the audit row is the system\'s', async () => {
		await policies.GET(ev());
		await policies.POST(ev('http://x/', { scope_key: 'all-org' }));
		await policyId.PATCH(ev('http://x/', { retention_days: 90 }, { id: 'p1' }));
		await policyId.DELETE(ev('http://x/', {}, { id: 'p1' }));
		for (const m of [mocks.listPolicies, mocks.upsertPolicy, mocks.patchPolicy, mocks.deletePolicy]) {
			expect(m.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		}
		for (const call of mocks.recordAudit.mock.calls) expect(call[0]).toEqual({ client: 'service' });
		expect(mocks.recordAudit).toHaveBeenCalledTimes(3);
	});
	it('refuses a caller with no session rather than acting as the service role', async () => {
		caller.accessToken = undefined;
		expect((await policies.GET(ev())).status).toBe(401);
		expect((await policies.POST(ev('http://x/', { scope_key: 'all-org' }))).status).toBe(401);
		expect((await policyId.PATCH(ev('http://x/', { retention_days: 90 }, { id: 'p1' }))).status).toBe(401);
		expect((await policyId.DELETE(ev('http://x/', {}, { id: 'p1' }))).status).toBe(401);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
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
	// Read as the signed-in admin; the session count is asked of the database,
	// which answers an admin with a number (#236).
	it('reads with the caller\'s JWT, never the service role', async () => {
		await health.GET(ev());
		expect(mocks.getHealth.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		expect(mocks.getContribVsApprove.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		caller.accessToken = undefined;
		expect((await health.GET(ev())).status).toBe(401);
		expect(mocks.getHealth).not.toHaveBeenCalled();
	});
});

// ── mutation routes (admin floor · audited) ──────────────────────────────────

describe('PATCH /members/{userId}/role', () => {
	it('sets the role, audits, and returns { user_id, role } at the admin floor', async () => {
		const res = await memberRole.PATCH(ev('http://x/', { role: 'lead' }, { userId: 'u1' }));
		expect(await res.json()).toEqual({ user_id: 'u1', role: 'lead' });
		expect(mocks.setMemberRole.mock.calls[0]).toEqual([{ client: 'user', token: 'jwt-A' }, 't1', 'u1', 'lead']);
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

// A person joins a dōjō by accepting an invite — their consent is the invite
// token. An admin adding an arbitrary principal id directly put someone in a
// tenant without their consent (#236, access matrix "Tenancy, identity,
// billing"). The console never called it; invites and dōjō creation keep using
// the server's addMember internally.
describe('members are added by invitation, not by id', () => {
	it('the members route answers GET and nothing else', () => {
		expect(members.GET).toBeDefined();
		expect((members as Record<string, unknown>).POST).toBeUndefined();
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

// An identity is the PERSON's proof (provider, subject) and spans every tenant
// they belong to (`dojo.identities` has no tenant). A tenant admin therefore
// READS their members' identities and never writes them: an edit by tenant A's
// admin would change the row tenant B relies on, and binding a forge account to
// a principal needs that forge's proof — provisioning's job, not an admin form
// (#236, docs/spec/dojo/access-matrix.md "Tenancy, identity, billing"). The
// console never called these writes.
describe('identities are read, never written, by a tenant admin', () => {
	it('the identities route answers GET and nothing else', () => {
		expect(identities.GET).toBeDefined();
		expect((identities as Record<string, unknown>).POST).toBeUndefined();
		expect((identities as Record<string, unknown>).PATCH).toBeUndefined();
		expect((identities as Record<string, unknown>).DELETE).toBeUndefined();
	});
	it('there is no per-identity write route', () => {
		const route = resolve(__dirname, '../identities/[id]/+server.ts');
		expect(existsSync(route), route).toBe(false);
	});
});
