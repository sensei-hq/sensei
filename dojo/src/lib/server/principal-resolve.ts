// The login → principal translation — the single place it happens.
//
// `dojo.principals` is the stable identity every dōjō foreign key points at, and
// `principals.auth_user_id` is a re-pointable POINTER at the Supabase login (see
// that table's DDL for why the indirection exists: Supabase auto-merges accounts
// sharing a verified email and offers no way to split them again). Consequently
// `memberships.user_id`, `projects.user_id` and every other `user_id` column in
// `dojo.*` hold a PRINCIPAL id — never the login id that `auth.getUser()` returns.
//
// Spec: docs/spec/dojo/dojo-auth-provisioning.md §VIII.2. The database-side
// counterpart is `dojo.current_principal_id()`, which does the same translation
// for RLS; between them there are exactly two implementations rather than one
// per call site, which is how three of them drifted apart before this existed.
//
// Fail closed, and note what "closed" means here. A defaulted or synthesised
// principal id would not raise anything — it would quietly attribute one human's
// work to another, or grant them a membership that is not theirs. So every
// failure path throws; none returns a plausible id.
import { AdminError, type DojoClient } from './admin-data';

/** The caller's principal id, or null when this login has none yet. Throws
 *  AdminError(500) on a query error — a failed read is never "no principal",
 *  which would fork a second principal for a human who already has one. */
export async function lookupPrincipalId(db: DojoClient, authUserId: string): Promise<string | null> {
	const { data, error } = await db
		.from('principals')
		.select('id')
		.eq('auth_user_id', authUserId)
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	return data ? ((data as { id: string }).id as string) : null;
}

/**
 * The principal id for a Supabase login, creating it on first sight.
 *
 * Idempotent, and safe under the concurrent sign-in of Part I Scenario 22:
 * `principals.auth_user_id` is UNIQUE, so a losing racer gets `23505` — the
 * constraint working, not an error worth surfacing — and re-reads the winner's
 * row. Both tabs converge on the same principal.
 *
 * Creating on first sight rather than refusing is deliberate: provisioning
 * itself authenticates through this resolver, so a "not provisioned yet" refusal
 * would leave a new user with no way to bootstrap. A principal row asserts only
 * "this authenticated human exists", which is true by the time we are called —
 * it grants no membership, no tenant and no entitlement.
 *
 * `displayName` is used only when creating. Keeping an existing principal's name
 * up to date belongs to provisioning, which has the forge profile in hand; a
 * resolver on the hot path of every request should not be writing.
 */
export async function resolvePrincipalId(
	db: DojoClient,
	authUserId: string,
	displayName?: string | null
): Promise<string> {
	const login = typeof authUserId === 'string' ? authUserId.trim() : '';
	if (!login) throw new AdminError(400, 'a login id is required to resolve a principal');

	const existing = await lookupPrincipalId(db, login);
	if (existing) return existing;

	const { data, error } = await db
		.from('principals')
		.insert({ auth_user_id: login, display_name: displayName ?? null })
		.select('id')
		.single();

	if (error) {
		// Anything but the unique violation is a real failure.
		if ((error as { code?: string }).code !== '23505') {
			throw new AdminError(500, error.message);
		}
		// Lost the race: the winner's row exists, so read it.
		const raced = await lookupPrincipalId(db, login);
		if (raced) return raced;
		// Unreachable in principle — 23505 means the row is there. If it happens
		// anyway, say so; a synthesised id here would be undetectable downstream.
		throw new AdminError(500, 'principal insert conflicted but no principal could be read back');
	}

	return (data as { id: string }).id;
}
