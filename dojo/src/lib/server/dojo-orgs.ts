// Server-only: build the app's `DojoOrg` view-model from real dojo data (the
// signed-in user's `dojo.memberships` + `tenants`). Shared by the org picker
// (routes/orgs/+page.server.ts) and the console chrome
// (routes/(console)/+layout.server.ts) so the mapping lives in one place and the
// two surfaces can never drift. Never import from client code — uses the
// service-role client (dojo-supabase.ts).
import { dojoDb } from './dojo-supabase';
import type { DojoOrg, OrgKind } from '$lib/dojo-data';

export type SessionUser = { id?: string; email?: string; user_metadata?: Record<string, unknown> };
export type OrgUser = { name: string; handle: string; initials: string };

type TenantRow = { id: string; key: string; org: string; name: string | null; self_hosted: boolean };

const TENANT_COLS = 'id, key, org, name, self_hosted';
const ROLE_LABEL: Record<string, string> = {
	admin: 'Admin',
	maintainer: 'Maintainer',
	lead: 'Lead',
	contributor: 'Contributor',
	member: 'Member'
};

/** The signed-in Supabase user off `locals` (kavach sets `locals.session`). */
export function sessionUser(locals: unknown): SessionUser | undefined {
	return (locals as { session?: { user?: SessionUser } })?.session?.user;
}

/** First+last initial of a display name. For an email, derive from the local
 *  part (before @) so `jerry.thomas@…` → "JT", not "JC" (jerry…com). */
function initials(nameOrEmail: string): string {
	const base = nameOrEmail.includes('@') ? nameOrEmail.split('@')[0] : nameOrEmail;
	const parts = base.trim().split(/[\s._-]+/).filter(Boolean);
	const first = parts[0]?.[0] ?? '?';
	const last = parts.length > 1 ? parts[parts.length - 1][0] : '';
	return (first + last).toUpperCase();
}

/** Display identity for the signed-in user (name/handle/initials). Magic-link
 *  users have no name, so we fall back to the email. */
export function userProfile(su: SessionUser | undefined): OrgUser {
	const meta = su?.user_metadata ?? {};
	const name = (meta.name as string) || (meta.full_name as string) || su?.email || 'You';
	return { name, handle: su?.email ?? '', initials: initials(name) };
}

/** Map a tenant row + the caller's role to the app's DojoOrg view-model. */
export function tenantToOrg(t: TenantRow, role: string): DojoOrg {
	return {
		id: t.id,
		kanji: '群',
		name: t.name ?? t.org,
		kind: 'Community' as OrgKind,
		host: t.self_hosted ? 'self' : 'saas',
		url: t.key,
		role: ROLE_LABEL[role] ?? role,
		from: `member · ${role}`,
		members: 0,
		pending: 0
	};
}

// supabase-js types a to-one embed loosely (array); PostgREST returns a single
// object for this many-to-one FK. Normalize both shapes.
function firstTenant(row: unknown): TenantRow | null {
	const t = (row as { tenant?: TenantRow | TenantRow[] | null }).tenant;
	return Array.isArray(t) ? (t[0] ?? null) : (t ?? null);
}

/** The Dōjōs a user belongs to (active memberships), as DojoOrg records. */
export async function listUserOrgs(userId: string): Promise<DojoOrg[]> {
	const { data, error } = await dojoDb()
		.from('memberships')
		.select(`role, tenant:tenants(${TENANT_COLS})`)
		.eq('user_id', userId)
		.is('disabled_at', null);
	if (error) {
		console.warn('listUserOrgs: memberships query failed —', error.message);
		return [];
	}
	return (data ?? []).flatMap((row) => {
		const t = firstTenant(row);
		return t ? [tenantToOrg(t, (row as { role: string }).role)] : [];
	});
}

/** The single Dōjō a user is entering (by tenant key), or undefined if they have
 *  no active membership in it. Backs the console chrome. Two queries (tenant by
 *  key, then membership) to avoid embedded-filter quirks. */
export async function getUserOrg(userId: string, tenantKey: string): Promise<DojoOrg | undefined> {
	const db = dojoDb();
	const { data: tenant, error: te } = await db
		.from('tenants')
		.select(TENANT_COLS)
		.eq('key', tenantKey)
		.maybeSingle();
	if (te || !tenant) {
		if (te) console.warn('getUserOrg: tenant query failed —', te.message);
		return undefined;
	}
	const { data: membership, error: me } = await db
		.from('memberships')
		.select('role')
		.eq('user_id', userId)
		.eq('tenant_id', (tenant as TenantRow).id)
		.is('disabled_at', null)
		.maybeSingle();
	if (me || !membership) {
		if (me) console.warn('getUserOrg: membership query failed —', me.message);
		return undefined;
	}
	return tenantToOrg(tenant as TenantRow, (membership as { role: string }).role);
}
