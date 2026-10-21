// WS-1 · Identity resolution — a shared `principal → display name` lookup over
// `dojo.identities`. A membership carries only a `user_id` (uuid), which holds a
// PRINCIPAL id (§VIII.2); the human name lives on the identity row (one per
// provider, so a principal can have several). This resolver is the single place
// that maps a set of principal ids to their best display name + email, so every
// console surface that renders people (members, role-surfaces, the audit actor,
// my-dojos) shares one honest rule instead of each re-deriving a shortId.
//
// It used to filter `.eq('tenant_id', …)`. `dojo.identities` has no such column —
// an identity is tenant-INDEPENDENT — so that filter errored the query outright
// and, via listMembers, the whole members screen. Scoping now comes from the
// caller: the principal ids handed in are already the tenant's members.
//
// Fail-closed: a query error throws AdminError(500) — never a fabricated name. A
// principal with no identity row (or only nameless ones) resolves to null/null,
// and the caller falls back to a stable shortId (an honest label, not an
// invented name).
import { AdminError, type DojoClient } from './admin-data';

/** The columns this resolver needs off `dojo.identities`. */
export interface IdentityNameRow {
	principal_id: string;
	display_name: string | null;
	email: string | null;
	last_login_at: string | null;
}

/** A resolved person: the best display name + email for a user_id (either may be null). */
export interface ResolvedName {
	display_name: string | null;
	email: string | null;
}

const IDENTITY_NAME_COLS = 'principal_id, display_name, email, last_login_at';

/** Preference tier for a candidate identity row: a named row beats an email-only
 *  row beats a bare row. Lower is better. */
function tier(r: IdentityNameRow): number {
	if (r.display_name && r.display_name.trim()) return 0;
	if (r.email && r.email.trim()) return 1;
	return 2;
}

/**
 * Pick the best identity for one user from their identity rows (pure). Prefers a
 * row that has a display name, then one with an email; within a tier, the most
 * recently logged-in row wins (a null `last_login_at` is treated as oldest).
 * Returns null/null when `rows` is empty or every row is bare.
 */
export function pickBestIdentity(rows: IdentityNameRow[]): ResolvedName {
	let best: IdentityNameRow | null = null;
	for (const r of rows) {
		if (best === null) {
			best = r;
			continue;
		}
		const dt = tier(r) - tier(best);
		// lower tier wins; tie → later last_login_at wins (null sorts as '').
		if (dt < 0 || (dt === 0 && (r.last_login_at ?? '') > (best.last_login_at ?? ''))) {
			best = r;
		}
	}
	if (!best) return { display_name: null, email: null };
	return {
		display_name: best.display_name && best.display_name.trim() ? best.display_name : null,
		email: best.email && best.email.trim() ? best.email : null
	};
}

/**
 * Resolve a set of PRINCIPAL ids to their best display name + email, from
 * `dojo.identities`. Returns a Map keyed by principal id — only principals that
 * HAVE an identity row appear; the caller treats a missing key (or null name) as
 * "fall back to shortId". A query error throws AdminError(500) (fail-closed, no
 * fabrication).
 *
 * Only principals who are MEMBERS of `tenantId` are resolved. Identities carry
 * no tenant, and an id can come straight from a request body (an incident's
 * `owner_id`), so "the caller already scoped it" is not a property this
 * function can assume; an id outside the tenant resolves to nothing.
 */
export async function resolveDisplayNames(
	db: DojoClient,
	tenantId: string,
	principalIds: string[]
): Promise<Map<string, ResolvedName>> {
	const requested = [
		...new Set(principalIds.filter((id) => typeof id === 'string' && id.length > 0))
	];
	if (requested.length === 0) return new Map();
	// TENANT FIRST. `identities` has no tenant column, and an id can arrive from
	// a request body (an incident's owner_id) — so an unscoped read hands the
	// caller anyone's name and email. Only the tenant's members are resolved; any
	// other id resolves to nothing, exactly as an unknown id does.
	const members = await db
		.from('memberships')
		.select('user_id')
		.eq('tenant_id', tenantId)
		.in('user_id', requested);
	if (members.error) throw new AdminError(500, members.error.message);
	const unique = [
		...new Set(((members.data ?? []) as Array<{ user_id: string }>).map((m) => m.user_id))
	];
	return namesOf(db, unique);
}

/**
 * Resolve the names of a tenant's dōjō VIEWERS (#307 D8) — people in the dōjō who
 * are not members, so {@link resolveDisplayNames} would not name them. Scoped the
 * same way: only ids that are viewers of `tenantId` resolve.
 */
export async function resolveViewerNames(
	db: DojoClient,
	tenantId: string,
	principalIds: string[]
): Promise<Map<string, ResolvedName>> {
	const requested = [...new Set(principalIds.filter((id) => typeof id === 'string' && id.length > 0))];
	if (requested.length === 0) return new Map();
	const viewers = await db
		.from('dojo_viewers')
		.select('principal_id')
		.eq('tenant_id', tenantId)
		.in('principal_id', requested);
	if (viewers.error) throw new AdminError(500, viewers.error.message);
	return namesOf(db, ((viewers.data ?? []) as Array<{ principal_id: string }>).map((v) => v.principal_id));
}

/** The best name for each of `principalIds`, already scoped by the caller. */
async function namesOf(db: DojoClient, principalIds: string[]): Promise<Map<string, ResolvedName>> {
	if (principalIds.length === 0) return new Map();
	const { data, error } = await db
		.from('identities')
		.select(IDENTITY_NAME_COLS)
		.in('principal_id', principalIds);
	if (error) throw new AdminError(500, error.message);
	const byPrincipal = new Map<string, IdentityNameRow[]>();
	for (const r of (data ?? []) as unknown as IdentityNameRow[]) {
		const list = byPrincipal.get(r.principal_id);
		if (list) list.push(r);
		else byPrincipal.set(r.principal_id, [r]);
	}
	const out = new Map<string, ResolvedName>();
	for (const [pid, rows] of byPrincipal) out.set(pid, pickBestIdentity(rows));
	return out;
}
