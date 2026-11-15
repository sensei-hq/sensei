// Route-level tests for the org repositories read (JWT plane):
//   GET /v1/t/{origin}/{org}/repositories → { repositories[], hidden_private_count }
//
// Phase B (org-repositories r2, tests 10-13): un-skipped against the real
// +server.ts, top-level await import per admin-routes.spec.ts. The store
// seam is $lib/server/admin-data's listOrgRepositories. Test 14 needs no
// test here: the PATCH election write path and its spec already exist and
// are untouched by this item.
import { describe, it, expect, vi, beforeEach } from 'vitest';
// The real AdminError (via the importOriginal spread below): the route maps
// it with `instanceof`, so a local lookalike would take the foreign-error
// branch and pin the wrong path.
import { AdminError } from '$lib/server/admin-data';

const caller: Record<string, unknown> = {
	tenantId: 't1',
	userId: 'admin-uuid',
	role: 'admin',
	access: 4,
	membershipId: 'm1',
	accessToken: 'jwt-A'
};

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	listOrgRepositories: vi.fn()
}));

// Fake clients (per admin-routes.spec.ts): the store is mocked and never
// touches them, but callerDb builds one through userDb, which throws
// without Supabase env.
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
vi.mock('$lib/server/admin-data', async (importOriginal) => ({
	...((await importOriginal()) as Record<string, unknown>),
	listOrgRepositories: mocks.listOrgRepositories
}));

function ev(params: Record<string, string> = {}) {
	return {
		params: { origin: 'organization', org: 'acme', ...params },
		request: new Request('http://x/', { headers: { authorization: 'Bearer [REDACTED]' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

const row = (over: Record<string, unknown> = {}) => ({
	repository_id: 'r1',
	repo_key: 'github.com/acme/core',
	name: 'core',
	provider: 'github',
	forge_visibility: 'private',
	member_count: 6,
	last_synced_at: '2026-11-02T14:01:00Z',
	elected: true,
	configurable_by_me: true,
	configured_by: 'organization',
	configured_at: '2026-10-28T09:00:00Z',
	entitlement_refusal: null,
	billing_hidden: false,
	...over
});

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.listOrgRepositories.mockClear().mockResolvedValue([]);
});

const repositories = await import('./+server');

async function GET(p?: Record<string, string>) {
	return repositories.GET(ev(p));
}

describe('GET /v1/t/{origin}/{org}/repositories (r2 tests 10-13)', () => {
	it('200 as admin with the full wire shape, gated at ACCESS.admin', async () => {
		// Asymmetric by design: two visible + one hidden, still expecting 1 —
		// the inversion mutant (!billing_hidden) yields 2 and dies here.
		mocks.listOrgRepositories.mockResolvedValue([
			row(),
			row({ name: 'portal', billing_hidden: true }),
			row({ name: 'docs' })
		]);
		const res = await GET();
		expect(res.status).toBe(200);
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4); // ACCESS.admin, before any store call
		const body = await res.json();
		expect(body.repositories).toHaveLength(3);
		expect(body.repositories[0]).toEqual(row());
		expect(body.hidden_private_count).toBe(1); // counted from the rows, not stored
	});

	// Layer split: through fully-mocked access no route test can distinguish
	// roles (the mock erases the layer that does). Per-role refusals live at
	// SQL test 2 (all five) + the ACCESS.admin floor-arg pin in the 200 test +
	// roleToAccess's own ladder spec (dojo-access-ladder.spec.ts). This pins
	// propagation: an access refusal reaches the caller, store untouched.
	it('access refusals (role floor, non-member) propagate, store untouched', async () => {
		for (const [status, message] of [
			[403, 'admin role required'],
			[403, 'not a member of this dōjō']
		] as Array<[number, string]>) {
			mocks.resolveTenantAccess.mockRejectedValueOnce(
				new Response(JSON.stringify({ error: message }), { status })
			);
			const res = await GET();
			expect(res.status).toBe(status);
			expect(await res.json()).toEqual({ error: message });
		}
		expect(mocks.listOrgRepositories).not.toHaveBeenCalled();
	});

	it('404 unknown org and origin/org mismatch propagate', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(
			new Response(JSON.stringify({ error: 'no such dōjō' }), { status: 404 })
		);
		const res = await GET({ org: 'nope' });
		expect(res.status).toBe(404);
		expect(mocks.listOrgRepositories).not.toHaveBeenCalled();
	});

	it('404 personal origin even when access resolves (orgs only)', async () => {
		// resolveTenantAccess resolves personal tenants (their keys exist),
		// so the route needs its own origin branch — else a personal read
		// falls through to the org store. Access resolves here by default.
		const res = await GET({ origin: 'personal' });
		expect(res.status).toBe(404);
		expect(mocks.listOrgRepositories).not.toHaveBeenCalled();
	});

	it('empty org reads empty, never 404 or null (D83)', async () => {
		mocks.listOrgRepositories.mockResolvedValue([]);
		const res = await GET();
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ repositories: [], hidden_private_count: 0 });
	});

	it('instants pass through as instants, nothing invented (D83)', async () => {
		mocks.listOrgRepositories.mockResolvedValue([row()]);
		const body = await (await GET()).json();
		for (const k of ['last_synced_at', 'configured_at'] as const) {
			expect(typeof body.repositories[0][k]).toBe('string');
			expect(Date.parse(body.repositories[0][k])).toBe(Date.parse(row()[k]));
		}
		expect(body.repositories[0].member_count).toBe(6); // counts from the DB, verbatim
	});

	it('store AdminError(500) returns 500 with the error shape, never partial rows', async () => {
		mocks.listOrgRepositories.mockRejectedValueOnce(new AdminError(500, 'definer failed'));
		const res = await GET();
		expect(res.status).toBe(500);
		expect(await res.json()).toEqual({ error: 'definer failed' });
	});

	it('foreign store errors reject (only AdminError maps to a Response)', async () => {
		mocks.listOrgRepositories.mockRejectedValueOnce(new Error('kaboom'));
		await expect(GET()).rejects.toThrow('kaboom');
	});
});
