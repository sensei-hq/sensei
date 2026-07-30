// Route-level tests for the Worker lead engagements LIST/CREATE endpoints (JWT
// plane), which are inline in the parent `+server.ts` (they call `dojoDb()`
// directly rather than the `engagements-data` store):
//   GET  /v1/t/{origin}/{org}/engagements → { engagements: [...] }
//   POST /v1/t/{origin}/{org}/engagements → { id }
//
// Covers the LEAD auth floor, the tenant filter, honest-empty on null data,
// error mapping, auth-Response propagation, and the Rule C `client` split:
// `client_name` required + `client_tenant_id` optional (FK to the client's
// own tenant, else null). No live Worker/DB — `dojoDb()` is a chainable stub.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AdminError } from '$lib/server/admin-data';

const caller = { tenantId: 't1', userId: 'lead-uuid', role: 'lead', access: 2, membershipId: 'm1' };

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	countEngagementArtifacts: vi.fn(),
	// terminals the chainable stub resolves to (GET ends on `.order()`, POST on `.single()`)
	db: { order: { data: [] as unknown, error: null as unknown }, single: { data: { id: 'e9' } as unknown, error: null as unknown } },
	captured: { insert: undefined as unknown, tenantEq: undefined as unknown }
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => {
		const b: Record<string, unknown> = {};
		b.from = () => b;
		b.select = () => b;
		b.eq = (_col: string, val: unknown) => {
			mocks.captured.tenantEq = val;
			return b;
		};
		b.order = () => Promise.resolve(mocks.db.order);
		b.insert = (payload: unknown) => {
			mocks.captured.insert = payload;
			return b;
		};
		b.single = () => Promise.resolve(mocks.db.single);
		return b;
	}
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
// The per-engagement count aggregate is unit-tested in engagement-artifact-counts.spec;
// here it's mocked so the route test isolates the GET enrichment shaping.
vi.mock('$lib/server/engagement-artifact-counts', () => ({
	countEngagementArtifacts: mocks.countEngagementArtifacts
}));

const route = await import('./+server');

function ev(method: string, body?: unknown) {
	return {
		params: { origin: 'github', org: 'acme' },
		request: new Request('http://x/', {
			method,
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			...(body !== undefined ? { body: JSON.stringify(body) } : {})
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.countEngagementArtifacts.mockClear().mockResolvedValue(new Map());
	mocks.db.order = { data: [], error: null };
	mocks.db.single = { data: { id: 'e9' }, error: null };
	mocks.captured.insert = undefined;
	mocks.captured.tenantEq = undefined;
});

describe('GET /engagements', () => {
	it('returns the tenant engagements at the lead floor, filtered by tenant_id', async () => {
		mocks.db.order = { data: [{ id: 'e1' }, { id: 'e2' }], error: null };
		const res = await route.GET(ev('GET'));
		expect(await res.json()).toEqual({
			engagements: [
				{ id: 'e1', lessons_kept: 0, stripped: 0 },
				{ id: 'e2', lessons_kept: 0, stripped: 0 }
			]
		});
		expect(mocks.captured.tenantEq).toBe('t1');
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2); // ACCESS.lead
	});
	it('enriches each engagement with its kept/stripped artifact counts', async () => {
		mocks.db.order = { data: [{ id: 'e1' }, { id: 'e2' }], error: null };
		mocks.countEngagementArtifacts.mockResolvedValueOnce(new Map([['e1', { lessonsKept: 3, stripped: 1 }]]));
		const body = await (await route.GET(ev('GET'))).json();
		expect(body.engagements[0]).toMatchObject({ id: 'e1', lessons_kept: 3, stripped: 1 });
		expect(body.engagements[1]).toMatchObject({ id: 'e2', lessons_kept: 0, stripped: 0 }); // no counts → 0
	});
	it('surfaces a count-query error as 500 (fail-closed)', async () => {
		mocks.db.order = { data: [{ id: 'e1' }], error: null };
		mocks.countEngagementArtifacts.mockRejectedValueOnce(new AdminError(500, 'count boom'));
		expect((await route.GET(ev('GET'))).status).toBe(500);
	});
	it('honest-empty [] when the query returns null data', async () => {
		mocks.db.order = { data: null, error: null };
		expect(await (await route.GET(ev('GET'))).json()).toEqual({ engagements: [] });
	});
	it('surfaces a query error as 500 rather than blanking', async () => {
		mocks.db.order = { data: null, error: { message: 'boom' } };
		expect((await route.GET(ev('GET'))).status).toBe(500);
	});
	it('propagates a 403 auth Response', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.GET(ev('GET'))).status).toBe(403);
	});
});

describe('POST /engagements (Rule C: client_name + client_tenant_id)', () => {
	it('inserts client_name + client_tenant_id + tenant_id and returns { id }', async () => {
		const res = await route.POST(ev('POST', { client_name: 'Globex', client_tenant_id: 'ct-1', description: 'auth work' }));
		expect(await res.json()).toEqual({ id: 'e9' });
		const p = mocks.captured.insert as Record<string, unknown>;
		expect(p.tenant_id).toBe('t1');
		expect(p.client_name).toBe('Globex');
		expect(p.client_tenant_id).toBe('ct-1');
		expect(p.description).toBe('auth work');
	});
	it('defaults client_tenant_id to null and description to null when absent', async () => {
		await route.POST(ev('POST', { client_name: 'Solo' }));
		const p = mocks.captured.insert as Record<string, unknown>;
		expect(p.client_tenant_id).toBeNull();
		expect(p.description).toBeNull();
	});
	it('400s (no insert) when client_name is missing', async () => {
		const res = await route.POST(ev('POST', { description: 'x' }));
		expect(res.status).toBe(400);
		expect(await res.json()).toEqual({ error: 'client_name is required' });
		expect(mocks.captured.insert).toBeUndefined();
	});
	it('400s when client_name is blank/whitespace (trim)', async () => {
		expect((await route.POST(ev('POST', { client_name: '   ' }))).status).toBe(400);
		expect(mocks.captured.insert).toBeUndefined();
	});
	it('surfaces an insert error as 500', async () => {
		mocks.db.single = { data: null, error: { message: 'insert boom' } };
		expect((await route.POST(ev('POST', { client_name: 'X' }))).status).toBe(500);
	});
	it('propagates a 403 auth Response without inserting', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.POST(ev('POST', { client_name: 'X' }))).status).toBe(403);
		expect(mocks.captured.insert).toBeUndefined();
	});
});
