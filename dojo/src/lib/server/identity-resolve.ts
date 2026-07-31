// WS-1 · Identity resolution — a shared `user_id → display name` lookup over
// `dojo.identities`. A membership carries only a `user_id` (uuid); the human name
// lives on the identity row (one per provider, so a user_id can have several).
// This resolver is the single place that maps a set of user_ids (within a tenant)
// to their best display name + email, so every console surface that renders people
// (members, role-surfaces, the audit actor, my-dojos) shares one honest rule
// instead of each re-deriving a shortId.
//
// Fail-closed: a query error throws AdminError(500) — never a fabricated name. A
// user with no identity row (or only nameless ones) resolves to null/null, and the
// caller falls back to a stable shortId (an honest label, not an invented name).
import { AdminError, type DojoClient } from './admin-data';

/** The columns this resolver needs off `dojo.identities`. */
export interface IdentityNameRow {
	user_id: string;
	display_name: string | null;
	email: string | null;
	last_login_at: string | null;
}

/** A resolved person: the best display name + email for a user_id (either may be null). */
export interface ResolvedName {
	display_name: string | null;
	email: string | null;
}

const IDENTITY_NAME_COLS = 'user_id, display_name, email, last_login_at';

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
 * Resolve a set of user_ids (within a tenant) to their best display name + email,
 * from `dojo.identities`. Returns a Map keyed by user_id — only users that HAVE an
 * identity row appear; the caller treats a missing key (or null name) as "fall back
 * to shortId". A query error throws AdminError(500) (fail-closed, no fabrication).
 */
export async function resolveDisplayNames(
	db: DojoClient,
	tenantId: string,
	userIds: string[]
): Promise<Map<string, ResolvedName>> {
	const unique = [...new Set(userIds.filter((id) => typeof id === 'string' && id.length > 0))];
	if (unique.length === 0) return new Map();
	const { data, error } = await db
		.from('identities')
		.select(IDENTITY_NAME_COLS)
		.eq('tenant_id', tenantId)
		.in('user_id', unique);
	if (error) throw new AdminError(500, error.message);
	const byUser = new Map<string, IdentityNameRow[]>();
	for (const r of (data ?? []) as unknown as IdentityNameRow[]) {
		const list = byUser.get(r.user_id);
		if (list) list.push(r);
		else byUser.set(r.user_id, [r]);
	}
	const out = new Map<string, ResolvedName>();
	for (const [uid, rows] of byUser) out.set(uid, pickBestIdentity(rows));
	return out;
}
