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

export type Caller = { tenantId: string; userId: string; role: string; access: AccessLevel };

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
	const token = tokenFrom(request, locals);
	if (!token) throw apiError(401, 'unauthenticated');

	const db = dojoDb();

	// Verify the JWT via Supabase Auth; user.id is the token `sub`.
	const { data: userData, error: userErr } = await db.auth.getUser(token);
	if (userErr || !userData?.user) throw apiError(401, 'invalid token');
	const userId = userData.user.id;

	// Resolve the tenant by its `{origin}/{org}` key.
	const { data: tenant, error: tErr } = await db
		.from('tenants').select('id').eq('key', `${origin}/${org}`).maybeSingle();
	if (tErr) throw apiError(500, 'tenant lookup failed');
	if (!tenant) throw apiError(404, 'no such tenant');

	// The caller's membership role in this tenant (active only).
	const { data: mem } = await db
		.from('memberships').select('role')
		.eq('tenant_id', tenant.id).eq('user_id', userId).is('disabled_at', null)
		.maybeSingle();

	const access = roleToAccess(mem?.role);
	if (access < floor) throw apiError(403, `${accessLabel(floor)} role required`);

	return { tenantId: tenant.id as string, userId, role: mem?.role ?? 'member', access };
}

export { ACCESS };
