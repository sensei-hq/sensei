// Regression tests for the JWT-plane authz resolver (`dojo-auth.ts`). The focus
// is the FAIL-CLOSED contract on the membership lookup: a Supabase error must
// surface as 500 and MUST NOT fall through to the default `member` access —
// which, on any `member`-floor route, would grant a non-member member-level
// access (the #109 fabrication audit). A chainable supabase-js stub (no live DB),
// like the sibling `-data` specs.
import { describe, it, expect, vi } from 'vitest';

type Terminal = { data: unknown; error: unknown };

// Minimal chainable stub: `.auth.getUser` returns `authResult`; each terminal
// (`.maybeSingle()` / `.single()`) shifts the next queued result.
//
// Round-trip order after §VIII.2: getUser → PRINCIPAL lookup → tenants lookup →
// membership. The principal lookup is first because every `user_id` column in
// `dojo.*` holds a principal id, so the login has to be translated before it can
// be matched against anything.
function makeDb(authResult: Terminal, ...results: Terminal[]) {
	const queue = [...results];
	const b: Record<string, unknown> = {};
	/** Every column an `.eq` filtered on, in order — which lookup ran. */
	const eqColumns: string[] = [];
	b.eqColumns = eqColumns;
	b.from = () => b;
	b.select = () => b;
	b.eq = (col: string) => {
		eqColumns.push(col);
		return b;
	};
	b.is = () => b;
	b.insert = () => b;
	const next = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	b.maybeSingle = next;
	b.single = next;
	b.auth = { getUser: vi.fn(() => Promise.resolve(authResult)) };
	return b;
}

/** The caller's principal, as the lookup returns it. `u1` is the LOGIN id that
 *  `getUser` reports; `p1` is what every downstream `user_id` must actually be. */
const PRINCIPAL_OK: Terminal = { data: { id: 'p1' }, error: null };

let stub: unknown;
vi.mock('./dojo-supabase', async (importOriginal) => {
	const actual = await (importOriginal as () => Promise<Record<string, unknown>>)();
	return { ...actual, dojoDb: () => stub };
});

// Imported after the mock is registered.
const { resolveTenantAccess, resolveCaller, resolveTenantCaller, isJwtShaped } = await import('./dojo-auth');
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
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: null, error: { message: 'db down' } });
		const err = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member).catch(
			(e) => e
		);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(500);
	});

	it('resolves a real member to their role + access + membershipId', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: { id: 'm1', role: 'contributor' }, error: null });
		const caller = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member);
		expect(caller).toMatchObject({
			tenantId: 't1',
			userId: 'p1',
			role: 'contributor',
			access: ACCESS.contributor,
			membershipId: 'm1'
		});
	});

	it('403 when a genuine non-member (no row, no error) is below the floor', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: null, error: null });
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

	// A missing (or disabled) membership is NO access — never "level 0". The
	// member floor admitted any signed-in person to any tenant's member-floor
	// routes; the routes that mattered each patched it locally. One rule here
	// instead of one per route (#236, access matrix "Who").
	it('403 for a signed-in non-member even at the MEMBER floor', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: null, error: null });
		const err = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member).catch(
			(e) => e
		);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(403);
	});

	// The caller carries the JWT it was authenticated with, so a route can read
	// AS the person through `userDb` — RLS deciding, not route code (#236 step 3).
	it('carries the verified access token for user-session reads', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: { id: 'm1', role: 'contributor' }, error: null });
		const caller = await resolveTenantAccess('gh', 'acme', bearerReq(), locals, ACCESS.member);
		expect(caller.accessToken).toBe('tok');
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
	it('returns the PRINCIPAL id as userId, never the Supabase login id', async () => {
		// The whole point of §VIII.2. A resolver that passed the login id straight
		// through would satisfy "returns a string" and break every downstream
		// ownership check silently, so this asserts the translation happened.
		stub = makeDb(AUTH_OK, PRINCIPAL_OK);
		const { userId, authUserId } = await resolveCaller(bearerReq(), locals);
		expect(userId).toBe('p1');
		expect(userId).not.toBe('u1');
		expect(authUserId).toBe('u1');
	});

	// A caller is an identity, not a client. Handing out the service role here
	// is how a route reads as the system by destructuring one more field; system
	// work names dojoDb() instead, and a session reads through callerDb (#236).
	it('hands out no database client — only who the caller is and their JWT', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK);
		const caller = await resolveCaller(bearerReq(), locals);
		expect(caller).not.toHaveProperty('db');
		expect(Object.keys(caller).sort()).toEqual(['accessToken', 'authUserId', 'email', 'userId']);
	});

	it('throws 500 rather than falling back to the login id when the principal lookup fails', async () => {
		// Fail closed. Degrading to the login id would produce a caller whose
		// user_id matches no membership — an empty console that looks like "you
		// belong to nothing" instead of an error.
		stub = makeDb(AUTH_OK, { data: null, error: { message: 'db down' } });
		const err = await resolveCaller(bearerReq(), locals).catch((e) => e);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(500);
	});

	it('throws 401 when unauthenticated (no bearer token)', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK);
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

// The device-plane routes accept the person's session OR, for one release, the
// legacy device token (#236 step 4, docs/plan/2026-10-12-dojo-device-plane-as-person.md).
// The bearer's SHAPE picks the path; it is never a fallback from one to the other.
const JWT = 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1MSJ9.c2lnbmF0dXJl';
function bearer(token: string): Request {
	return new Request('https://dojo.test/v1', { headers: { authorization: `Bearer ${token}` } });
}
const getUserOf = () => (stub as { auth: { getUser: ReturnType<typeof vi.fn> } }).auth.getUser;
const eqColumnsOf = () => (stub as { eqColumns: string[] }).eqColumns;

describe('isJwtShaped', () => {
	it('is three non-empty base64url segments', () => {
		expect(isJwtShaped(JWT)).toBe(true);
		expect(isJwtShaped('a-b_c.d-e_f.g-h_i')).toBe(true);
	});
	it('is not a device token, a short or long dotted string, or an empty segment', () => {
		expect(isJwtShaped('relay-dev-device-token')).toBe(false);
		expect(isJwtShaped('a.b')).toBe(false);
		expect(isJwtShaped('a.b.c.d')).toBe(false);
		expect(isJwtShaped('a..c')).toBe(false);
		expect(isJwtShaped('a.b.c=')).toBe(false);
		expect(isJwtShaped('')).toBe(false);
	});
});

describe('resolveTenantCaller — the credential shape picks the path', () => {
	it('a JWT acts as the person: verified, a member, and carrying the session', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: { id: 'm1', role: 'contributor' }, error: null });
		const caller = await resolveTenantCaller('gh', 'acme', bearer(JWT), ACCESS.member);
		expect(getUserOf()).toHaveBeenCalledWith(JWT);
		expect(caller).toMatchObject({ tenantId: 't1', userId: 'p1', membershipId: 'm1', accessToken: JWT });
		expect(eqColumnsOf()).not.toContain('device_token_hash');
	});

	it('a JWT that fails verification is 401 — never retried as a device token', async () => {
		stub = makeDb({ data: { user: null }, error: { message: 'bad jwt' } }, TENANT_OK, {
			data: { id: 'm1', user_id: 'p1', role: 'contributor' },
			error: null
		});
		const err = await resolveTenantCaller('gh', 'acme', bearer(JWT), ACCESS.member).catch((e) => e);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(401);
		expect(eqColumnsOf()).not.toContain('device_token_hash');
	});

	it('anything else is the legacy device token: no session, the membership it hashes to', async () => {
		stub = makeDb(AUTH_OK, TENANT_OK, { data: { id: 'm1', user_id: 'p1', role: 'contributor' }, error: null });
		const caller = await resolveTenantCaller('gh', 'acme', bearer('relay-dev-device-token'), ACCESS.member);
		expect(getUserOf()).not.toHaveBeenCalled();
		expect(eqColumnsOf()).toContain('device_token_hash');
		expect(caller).toMatchObject({ tenantId: 't1', userId: 'p1', membershipId: 'm1' });
		expect(caller.accessToken).toBeUndefined();
	});

	it('only the Authorization header counts — a cookie session does not stand in', async () => {
		stub = makeDb(AUTH_OK, PRINCIPAL_OK, TENANT_OK, { data: { id: 'm1', role: 'contributor' }, error: null });
		const err = await resolveTenantCaller('gh', 'acme', new Request('https://dojo.test/v1'), ACCESS.member).catch(
			(e) => e
		);
		expect(err).toBeInstanceOf(Response);
		expect((err as Response).status).toBe(401);
		expect(getUserOf()).not.toHaveBeenCalled();
	});
});
