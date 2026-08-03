// Tenant + role resolution for the in-Worker dojo API — the TS port of
// dojo-mind's `resolve_tenant_access` (JWT plane). A request is authenticated
// by a Supabase JWT (the desktop / API plane sends `Authorization: Bearer …`;
// the web console sends its session `access_token`), whose `sub` is matched
// against `dojo.memberships.user_id`; the membership `role` gives an access
// level checked against the route's floor.
//
// The API-key plane (dojo-mind's other auth path, for machine callers) and
// SSO identity-mapping are follow-ups — the console runs on the JWT plane.

import { dojoDb, roleToAccess, accessLabel, ACCESS, type AccessLevel } from './dojo-supabase';

/** A JSON error Response (thrown to short-circuit a handler). */
export function apiError(status: number, message: string): Response {
	return new Response(JSON.stringify({ error: message }), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

export type Caller = {
	tenantId: string;
	userId: string;
	role: string;
	access: AccessLevel;
	/** Present on the API-key (device-token) plane — the caller's membership row. */
	membershipId?: string;
};

/** Lowercase hex sha256 (Web Crypto — available in the Worker runtime and Node). */
async function sha256Hex(input: string): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(input));
	return Array.from(new Uint8Array(digest))
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}

/** Resolve the `{origin}/{org}` tenant id, or throw 404/500. */
async function resolveTenantId(
	db: ReturnType<typeof dojoDb>,
	origin: string,
	org: string
): Promise<string> {
	const { data: tenant, error } = await db
		.from('tenants')
		.select('id')
		.eq('key', `${origin}/${org}`)
		.maybeSingle();
	if (error) throw apiError(500, 'tenant lookup failed');
	if (!tenant) throw apiError(404, 'no such tenant');
	return tenant.id as string;
}

/** All membership ids for a tenant — the membership-scoped replacement for the
 *  dropped relay_* `tenant_id` filter (WS-0 Rule A slice 2). The relay tables key
 *  on `membership_id` only, so a tenant-wide relay read filters
 *  `membership_id IN (this tenant's memberships)`. Returns [] for an unknown or
 *  member-less tenant (a subsequent `.in('membership_id', [])` matches nothing —
 *  the correct empty result, never a leak). */
export async function membershipIdsForTenant(
	db: ReturnType<typeof dojoDb>,
	tenantId: string
): Promise<string[]> {
	const { data, error } = await db.from('memberships').select('id').eq('tenant_id', tenantId);
	if (error) throw apiError(500, 'membership scope lookup failed');
	return (data ?? []).map((m) => m.id as string);
}

/** Extract the Supabase access token: `Authorization: Bearer <jwt>` (API/desktop
 *  plane) or the kavach session's `access_token` (web console). */
function tokenFrom(request: Request, locals: App.Locals): string | null {
	const auth = request.headers.get('authorization') ?? '';
	if (auth.toLowerCase().startsWith('bearer ')) {
		const t = auth.slice(7).trim();
		if (t) return t;
	}
	// kavach's hooks.server.ts puts the Supabase session on locals.
	const session = (locals as unknown as { session?: { access_token?: string } }).session;
	return session?.access_token ?? null;
}

/**
 * Authenticate the caller on the Supabase-JWT plane and return their user id —
 * no tenant, no role floor. For USER-WIDE (user-primary) reads that aren't
 * scoped to one dōjō (e.g. the personal `/you/projects` list, whose read is
 * authorized purely by a `user_id` filter). Throws a Response (401) on a missing
 * or invalid token, like `resolveTenantAccess`. Returns the service-role `db`
 * too so the caller reuses one client for the ownership-filtered read.
 */
export async function resolveCaller(
	request: Request,
	locals: App.Locals
): Promise<{ userId: string; db: ReturnType<typeof dojoDb> }> {
	const token = tokenFrom(request, locals);
	if (!token) throw apiError(401, 'unauthenticated');

	const db = dojoDb();
	// Verify the JWT via Supabase Auth; user.id is the token `sub`.
	const { data: userData, error: userErr } = await db.auth.getUser(token);
	if (userErr || !userData?.user) throw apiError(401, 'invalid token');
	return { userId: userData.user.id, db };
}

/**
 * Resolve the `{origin}/{org}` tenant and authenticate the caller to at least
 * `floor` access. Throws a Response (401 / 403 / 404) on failure — handlers
 * `try { … } catch (e) { if (e instanceof Response) return e; throw e }`.
 */
export async function resolveTenantAccess(
	origin: string,
	org: string,
	request: Request,
	locals: App.Locals,
	floor: AccessLevel
): Promise<Caller> {
	// Same JWT-plane identity step as a user-wide caller, then tenant + role.
	const { userId, db } = await resolveCaller(request, locals);

	// Resolve the tenant by its `{origin}/{org}` key.
	const tenantId = await resolveTenantId(db, origin, org);

	// The caller's membership (id + role) in this tenant (active only). Fail
	// CLOSED on a lookup error: a DB failure must never be read as "no
	// membership" and then fall through to the default `member` access (which
	// would grant a non-member member-level access on any `member`-floor route).
	// Mirror `resolveApiKeyAccess` below.
	const { data: mem, error } = await db
		.from('memberships').select('id, role')
		.eq('tenant_id', tenantId).eq('user_id', userId).is('disabled_at', null)
		.maybeSingle();
	if (error) throw apiError(500, 'membership lookup failed');

	const access = roleToAccess(mem?.role);
	if (access < floor) throw apiError(403, `${accessLabel(floor)} role required`);

	return {
		tenantId,
		userId,
		role: mem?.role ?? 'member',
		access,
		membershipId: mem?.id as string | undefined
	};
}

/**
 * Authenticate a MACHINE caller (the daemon) on the API-key / device-token plane
 * — relay auth plane A. The daemon sends its Keychain device token as
 * `Authorization: Bearer <token>`; we sha256 it and match
 * `dojo.memberships.device_token_hash` for the route's tenant (active only). The
 * token is **per-membership** (per tenant×user). The compared value is itself a
 * hash, so the lookup leaks nothing about the token (cf. `dojo.api_keys`). Throws
 * a Response (401/403/404) like `resolveTenantAccess`.
 *
 * Phone/console callers use `resolveTenantAccess` (Supabase-JWT plane) instead.
 * (Plane B — signed payloads via `memberships.device_key` — is a hardening
 * follow-up; see docs/plan/decisions.md.)
 */
export async function resolveApiKeyAccess(
	origin: string,
	org: string,
	request: Request,
	floor: AccessLevel
): Promise<Caller> {
	const auth = request.headers.get('authorization') ?? '';
	const token = auth.toLowerCase().startsWith('bearer ') ? auth.slice(7).trim() : '';
	if (!token) throw apiError(401, 'unauthenticated');

	const db = dojoDb();
	const tenantId = await resolveTenantId(db, origin, org);

	const hash = await sha256Hex(token);
	const { data: mem, error } = await db
		.from('memberships')
		.select('id, user_id, role')
		.eq('tenant_id', tenantId)
		.eq('device_token_hash', hash)
		.is('disabled_at', null)
		.maybeSingle();
	if (error) throw apiError(500, 'device auth lookup failed');
	if (!mem) throw apiError(401, 'invalid device token');

	const access = roleToAccess(mem.role);
	if (access < floor) throw apiError(403, `${accessLabel(floor)} role required`);

	return {
		tenantId,
		userId: mem.user_id as string,
		role: (mem.role as string) ?? 'member',
		access,
		membershipId: mem.id as string
	};
}

export { ACCESS };
