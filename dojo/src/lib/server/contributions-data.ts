// Console reads + the adopt write for the personal Contributions screen (F5).
// USER-primary: "mine" = the user's `dojo.artifacts` (contribute pipeline stamps
// `contributed_by` server-side); "downstream" = the `dojo.downstream_inbox`
// distribution ledger for the user's memberships, joined to the artifact. Pin =
// flip an inbox row to `pinned`. All user-wide across the user's dōjōs; fail
// closed (AdminError) — never a fabricated row/success.
import { AdminError, type DojoClient } from './admin-data';
import type { ContributionRow, DownstreamRow } from '../contributions-map';

export type { DojoClient };
export { AdminError };

// Supabase embeds a to-one join as an object (or a 1-element array depending on
// the FK shape); normalize to the single name. Mirrors `firstTenant` in dojo-orgs.
function joinName(t: unknown): string | null {
	if (!t) return null;
	const o = (Array.isArray(t) ? t[0] : t) as { name?: string } | undefined;
	return o?.name ?? null;
}

/** The caller's active membership ids across ALL their dōjōs — the key for the
 *  membership-scoped downstream reads/writes. Fails closed. */
export async function userMembershipIds(db: DojoClient, userId: string): Promise<string[]> {
	const { data, error } = await db
		.from('memberships')
		.select('id')
		.eq('user_id', userId)
		.is('disabled_at', null);
	if (error) throw new AdminError(500, error.message);
	return (data ?? []).map((r) => (r as { id: string }).id);
}

/** "What you've shared" — the user's own `dojo.artifacts`, newest-first, scoped by
 *  `contributed_by = userId` (the filter IS the authz). Fails closed. */
export async function listUserContributions(db: DojoClient, userId: string): Promise<ContributionRow[]> {
	const { data, error } = await db
		.from('artifacts')
		.select('kind, title, status, attribution, scope, created_at, tenant:tenants(name)')
		.eq('contributed_by', userId)
		.order('created_at', { ascending: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []).map((r) => {
		const row = r as Record<string, unknown>;
		return {
			kind: row.kind as string,
			title: row.title as string,
			status: row.status as string,
			attribution: (row.attribution as { mode?: string } | null) ?? null,
			scope: (row.scope as Record<string, unknown> | null) ?? null,
			created_at: row.created_at as string,
			dest: joinName(row.tenant)
		} satisfies ContributionRow;
	});
}

/** "Approved for you" — the `dojo.downstream_inbox` rows for the caller's
 *  memberships, joined to the artifact + origin tenant, newest-first. Empty
 *  membership set → [] (no rows can be theirs). Fails closed. */
export async function listUserDownstream(db: DojoClient, membershipIds: string[]): Promise<DownstreamRow[]> {
	if (membershipIds.length === 0) return [];
	const { data, error } = await db
		.from('downstream_inbox')
		.select('artifact_id, state, created_at, artifact:artifacts(kind, title, scope, tenant:tenants(name))')
		.in('membership_id', membershipIds)
		.order('created_at', { ascending: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []).map((r) => {
		const row = r as Record<string, unknown>;
		const a = (Array.isArray(row.artifact) ? row.artifact[0] : row.artifact) as
			| Record<string, unknown>
			| undefined;
		return {
			id: row.artifact_id as string,
			state: row.state as string,
			created_at: row.created_at as string,
			kind: (a?.kind as string) ?? '',
			title: (a?.title as string) ?? '',
			scope: (a?.scope as Record<string, unknown> | null) ?? null,
			from: joinName(a?.tenant)
		} satisfies DownstreamRow;
	});
}

/** Pin (adopt) an approved-for-you artifact — flip the caller's inbox row(s) for
 *  it to `pinned`. Authorized by membership ownership: the `membership_id IN`
 *  clause means a caller can only ever pin their own rows. Empty memberships →
 *  403 (never a silent no-op success). Fails closed. */
export async function adoptDownstream(
	db: DojoClient,
	membershipIds: string[],
	artifactId: string
): Promise<void> {
	if (membershipIds.length === 0) throw new AdminError(403, 'no membership to adopt under');
	const nowIso = new Date().toISOString();
	const { error } = await db
		.from('downstream_inbox')
		.update({ state: 'pinned', acted_at: nowIso, updated_at: nowIso })
		.eq('artifact_id', artifactId)
		.in('membership_id', membershipIds);
	if (error) throw new AdminError(500, error.message);
}
