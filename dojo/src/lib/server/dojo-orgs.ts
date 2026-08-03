// Server-only: build the app's `DojoOrg` view-model from real dojo data (the
// signed-in user's `dojo.memberships` + `tenants`). Shared by the org picker
// (routes/orgs/+page.server.ts) and the console chrome
// (routes/(console)/+layout.server.ts) so the mapping lives in one place and the
// two surfaces can never drift. Never import from client code — uses the
// service-role client (dojo-supabase.ts).
import { error } from '@sveltejs/kit';
import { dojoDb } from './dojo-supabase';
import { membershipKindToOrgKind, orgKindKanji, type DojoOrg } from '$lib/dojo-data';

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

/** Map a tenant row + the caller's membership (role + kind) to the app's DojoOrg
 *  view-model. `kind` is the REAL `dojo.membership_kind` (was hardcoded 'Community',
 *  which collapsed every dōjō into the Communities group). Counts are left
 *  undefined — not yet computed here — so the row omits the chip rather than
 *  showing a fabricated 0 (computing real members/projects/pending is a follow-on). */
export function tenantToOrg(t: TenantRow, role: string, kind?: string | null): DojoOrg {
	const orgKind = membershipKindToOrgKind(kind);
	return {
		id: t.id,
		kanji: orgKindKanji(orgKind),
		name: t.name ?? t.org,
		kind: orgKind,
		host: t.self_hosted ? 'self' : 'saas',
		url: t.key,
		role: ROLE_LABEL[role] ?? role,
		from: `member · ${role}`
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
	const { data, error: qErr } = await dojoDb()
		.from('memberships')
		.select(`role, kind, tenant:tenants(${TENANT_COLS})`)
		.eq('user_id', userId)
		.is('disabled_at', null);
	// Fail CLOSED: a memberships-query failure must surface as an error, never a
	// fabricated empty list — an empty list would silently eject a real member to
	// the solo/personal landing (via `hasMembership`). See the #109 fabrication audit.
	if (qErr) throw error(500, 'memberships lookup failed');
	return (data ?? []).flatMap((row) => {
		const t = firstTenant(row);
		const m = row as { role: string; kind?: string | null };
		return t ? [tenantToOrg(t, m.role, m.kind)] : [];
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
	// Fail CLOSED on a query error (500); a genuine miss (no error, no row) is a
	// real "no such tenant / not a member" → undefined, never masked by a failure.
	if (te) throw error(500, 'tenant lookup failed');
	if (!tenant) return undefined;
	const { data: membership, error: me } = await db
		.from('memberships')
		.select('role, kind')
		.eq('user_id', userId)
		.eq('tenant_id', (tenant as TenantRow).id)
		.is('disabled_at', null)
		.maybeSingle();
	if (me) throw error(500, 'membership lookup failed');
	if (!membership) return undefined;
	const m = membership as { role: string; kind?: string | null };
	return tenantToOrg(tenant as TenantRow, m.role, m.kind);
}
