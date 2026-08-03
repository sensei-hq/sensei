// Regression tests for the JWT-plane authz resolver (`dojo-auth.ts`). The focus
// is the FAIL-CLOSED contract on the membership lookup: a Supabase error must
// surface as 500 and MUST NOT fall through to the default `member` access —
// which, on any `member`-floor route, would grant a non-member member-level
// access (the #109 fabrication audit). A chainable supabase-js stub (no live DB),
// like the sibling `-data` specs.
import { describe, it, expect, vi } from 'vitest';

type Terminal = { data: unknown; error: unknown };

// Minimal chainable stub: `.auth.getUser` returns `authResult`; each terminal
// `.maybeSingle()` shifts the next queued result (tenants lookup, then membership).
function makeDb(authResult: Terminal, ...results: Terminal[]) {
	const queue = [...results];
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.is = () => b;
	b.maybeSingle = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	b.auth = { getUser: () => Promise.resolve(authResult) };
	return b;
}

let stub: unknown;
vi.mock('./dojo-supabase', async (importOriginal) => {
	const actual = await (importOriginal as () => Promise<Record<string, unknown>>)();
	return { ...actual, dojoDb: () => stub };
});

// Imported after the mock is registered.
const { resolveTenantAccess, resolveCaller } = await import('./dojo-auth');
const { ACCESS } = await import('./dojo-supabase');

const AUTH_OK: Terminal = { data: { user: { id: 'u1' } }, error: null };
const TENANT_OK: Terminal = { data: { id: 't1' }, error: null };

function bearerReq(): Request {
	return new Request('https://dojo.test/v1', { headers: { authorization: 'Bearer tok' } });
}
const locals = {} as unknown as App.Locals;

describe('resolveTenantAccess — fail closed on the membership lookup', () => {
	it('throws 500 (not a fabricated member grant) when the memberships query errors', async () => {
		// member floor: before the fix, an errored lookup → mem=null → access=member(0)
		// → 0 < 0 is false → GRANTED as a phantom member. Now it must throw 500.
		stub = makeDb(AUTH_OK, TENANT_OK, { data: null, error: { message: 'db down' } });
		const err = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member).catch(
			(e) => e
		);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(500);
	});

	it('resolves a real member to their role + access + membershipId', async () => {
		stub = makeDb(AUTH_OK, TENANT_OK, { data: { id: 'm1', role: 'contributor' }, error: null });
		const caller = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member);
		expect(caller).toMatchObject({
			tenantId: 't1',
			userId: 'u1',
			role: 'contributor',
			access: ACCESS.contributor,
			membershipId: 'm1'
		});
	});

	it('403 when a genuine non-member (no row, no error) is below the floor', async () => {
		stub = makeDb(AUTH_OK, TENANT_OK, { data: null, error: null });
		const err = await resolveTenantAccess(
			'gh',
			'acme',
			bearerReq(),
			locals,
			ACCESS.contributor
		).catch((e) => e);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(403);
	});

	it('401 when the JWT is invalid (no user)', async () => {
		stub = makeDb({ data: { user: null }, error: { message: 'bad jwt' } });
		const err = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member).catch(
			(e) => e
		);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(401);
	});
});

describe('resolveCaller — user-wide JWT identity (no tenant, no role floor)', () => {
	it('returns the token subject (user id) on a valid JWT', async () => {
		stub = makeDb(AUTH_OK);
		const { userId } = await resolveCaller(bearerReq(), locals);
		expect(userId).toBe('u1');
	});

	it('throws 401 when unauthenticated (no bearer token)', async () => {
		stub = makeDb(AUTH_OK);
		const noAuth = new Request('https://dojo.test/v1');
		const err = await resolveCaller(noAuth, locals).catch((e) => e);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(401);
	});

	it('throws 401 when the JWT is invalid (no user)', async () => {
		stub = makeDb({ data: { user: null }, error: { message: 'bad jwt' } });
		const err = await resolveCaller(bearerReq(), locals).catch((e) => e);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(401);
	});
});
