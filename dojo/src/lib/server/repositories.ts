// Repository registration and the sync plan — the daemon's half of §V.4, in the
// user-scoped shape §VIII.1 corrected it to.
//
// TWO CALLS, NOT ONE, and the split is the point:
//
//   POST /v1/you/repositories   IDENTITY. Which tenant does this repo belong to?
//   GET  /v1/you/sync/plan      ENTITLEMENT. Of the repos we know about, which
//                               may this user sync right now?
//
// §V.4 asks for exactly that division — "repo identity is registered separately
// on connect; the plan is the entitlement filter applied on top" — and keeping
// them apart is what lets the plan stay a cacheless GET whose answer can change
// (a seat revoked, a subscription lapsed) without the daemon re-shipping its
// repository list every cycle.
//
// WHY THE PLAN IS NOT ADDRESSED PER TENANT. §V.4 also lists `unmapped` as a
// denial reason, which cannot be reported by a per-tenant endpoint: a repo that
// maps to NO tenant has no tenant to be denied under. And the daemon cannot
// address the plan by tenant anyway, because which tenant a repo belongs to is
// the very thing it is asking. So `unmapped` is a REGISTRATION outcome here, and
// `denied[]` carries only entitlement reasons (§VIII.1).
import { AdminError, type DojoClient } from './admin-data';
import { forgeRefFromRepoKey } from './repo-mapping';

/** One repository the daemon is offering to register. */
export interface RepoInput {
	repo_key: string;
	remote_url?: string | null;
	name?: string | null;
}

/** A repository that resolved to a tenant. */
export interface MappedRepo {
	repo_key: string;
	/** The tenant's `{origin}/{slug}` discovery key. */
	tenant: string;
	repo_id: string;
}

/** Why a repository could not be attached to a tenant. Each is a different
 *  problem with different advice, so they are not collapsed into one. */
export type UnmappedReason =
	| 'unknown_host' //   a forge we cannot attribute (self-hosted, GHE)
	| 'no_connection' //  the org has never been connected to any tenant
	| 'ambiguous' //      that slug resolves to more than one tenant
	| 'not_a_member'; //  the org is connected, but not to a tenant you are in

export interface RegisterResult {
	mapped: MappedRepo[];
	unmapped: { repo_key: string; reason: UnmappedReason }[];
}

/** The daemon's per-cycle answer. Phase 1 allows everything registered; the
 *  shape does not change when the gate arrives in phase 2, which is the whole
 *  argument for the plan endpoint (§V.5). */
export interface SyncPlan {
	allowed: MappedRepo[];
	denied: { repo_key: string; tenant: string; reason: string }[];
}

/** A display name from the key when the daemon did not send one — the last
 *  segment, which is the repository's own name. Not a fabricated value: it is
 *  derived from the identity we were given, and `name` is NOT NULL. */
function nameFromKey(repoKey: string): string {
	const parts = repoKey.split('/').filter(Boolean);
	return parts[parts.length - 1] ?? repoKey;
}

/** The caller's tenants, as `{ id → key }`. One query, reused across every repo
 *  in the batch — and it is also the authorization boundary: a repo can only be
 *  registered into a tenant the caller actually belongs to. */
async function callerTenants(
	db: DojoClient,
	principalId: string
): Promise<Map<string, string>> {
	const mem = await db.from('memberships').select('tenant_id').eq('user_id', principalId);
	if (mem.error) throw new AdminError(500, mem.error.message);
	const ids = (mem.data ?? []).map((m) => (m as { tenant_id: string }).tenant_id);
	if (ids.length === 0) return new Map();

	const t = await db.from('tenants').select('id, key').in('id', ids);
	if (t.error) throw new AdminError(500, t.error.message);
	const out = new Map<string, string>();
	for (const row of (t.data ?? []) as { id: string; key: string }[]) out.set(row.id, row.key);
	return out;
}

/**
 * Register the caller's shared repositories and map each to its tenant.
 *
 * Mapping goes `repo_key → (provider, org) → tenant_connections → tenant`. The
 * connection is matched on `external_slug`, because a git remote carries the
 * org's NAME and never its stable id — which is also why a slug resolving to
 * more than one connection is reported `ambiguous` rather than guessed at. A
 * wrong guess here attaches a private repository to somebody else's dōjō.
 *
 * An unmapped repository gets NO row. `dojo.repositories.tenant_id` is NOT NULL,
 * so there is nowhere to put one — which matches §II.6 exactly: it stays
 * local-only until its org is connected, and is never defaulted to the personal
 * tenant.
 */
export async function registerRepositories(
	db: DojoClient,
	principalId: string,
	repos: RepoInput[]
): Promise<RegisterResult> {
	const tenants = await callerTenants(db, principalId);
	const mapped: MappedRepo[] = [];
	const unmapped: { repo_key: string; reason: UnmappedReason }[] = [];

	for (const repo of repos) {
		const repoKey = (repo.repo_key ?? '').trim().toLowerCase();
		if (!repoKey) continue;

		const ref = forgeRefFromRepoKey(repoKey);
		if (!ref) {
			unmapped.push({ repo_key: repoKey, reason: 'unknown_host' });
			continue;
		}

		const conns = await db
			.from('tenant_connections')
			.select('tenant_id, external_slug')
			.eq('provider', ref.provider);
		if (conns.error) throw new AdminError(500, conns.error.message);

		const hits = ((conns.data ?? []) as { tenant_id: string; external_slug: string }[]).filter(
			(c) => (c.external_slug ?? '').toLowerCase() === ref.org
		);
		if (hits.length === 0) {
			unmapped.push({ repo_key: repoKey, reason: 'no_connection' });
			continue;
		}
		// A slug can only resolve to several tenants if an org was renamed and the
		// name re-registered upstream. Picking one would attach real code to the
		// wrong governance boundary.
		const inMine = hits.filter((h) => tenants.has(h.tenant_id));
		if (inMine.length === 0) {
			unmapped.push({ repo_key: repoKey, reason: 'not_a_member' });
			continue;
		}
		if (inMine.length > 1) {
			unmapped.push({ repo_key: repoKey, reason: 'ambiguous' });
			continue;
		}

		const tenantId = inMine[0].tenant_id;
		const tenantKey = tenants.get(tenantId) as string;

		const existing = await db
			.from('repositories')
			.select('id')
			.eq('tenant_id', tenantId)
			.eq('repo_key', repoKey)
			.maybeSingle();
		if (existing.error) throw new AdminError(500, existing.error.message);

		if (existing.data) {
			mapped.push({
				repo_key: repoKey,
				tenant: tenantKey,
				repo_id: (existing.data as { id: string }).id
			});
			continue;
		}

		const ins = await db
			.from('repositories')
			.insert({
				tenant_id: tenantId,
				repo_key: repoKey,
				remote_url: repo.remote_url ?? null,
				name: (repo.name ?? '').trim() || nameFromKey(repoKey),
				// Stored, not re-derived. `dojo.repositories.provider` is NOT NULL, and
				// keeping the host→provider mapping in this one place is the reason:
				// a second copy in a view or query would be another thing to keep in
				// step with `repo-mapping.ts`.
				provider: ref.provider
			})
			.select('id')
			.single();
		if (ins.error) {
			// A concurrent register won the (tenant_id, repo_key) unique — read it.
			if ((ins.error as { code?: string }).code === '23505') {
				const again = await db
					.from('repositories')
					.select('id')
					.eq('tenant_id', tenantId)
					.eq('repo_key', repoKey)
					.maybeSingle();
				if (again.error) throw new AdminError(500, again.error.message);
				if (again.data) {
					mapped.push({
						repo_key: repoKey,
						tenant: tenantKey,
						repo_id: (again.data as { id: string }).id
					});
					continue;
				}
			}
			throw new AdminError(500, ins.error.message);
		}

		mapped.push({ repo_key: repoKey, tenant: tenantKey, repo_id: (ins.data as { id: string }).id });
	}

	return { mapped, unmapped };
}

/**
 * What the caller may sync this cycle.
 *
 * Reads `dojo.all_my_repositories`, which already joins repository → tenant →
 * membership and carries the owning tenant on every row. That view is the
 * user-plane grain this whole endpoint exists for: a person belongs to several
 * tenants, so asking per tenant would make the caller ask N times AND require it
 * to already know which tenant a repository belongs to — the very thing it is
 * asking. It also means the phase-2 gate arrives as a change to `sync_enabled`
 * in the view, not as a change here.
 *
 * PHASE 1 ALLOWS EVERYTHING REGISTERED. There is no claim, no billing and no
 * seat yet, so there is nothing to deny on — and inventing a denial would be as
 * dishonest as inventing an allowance. `denied` is empty rather than absent, so
 * the daemon's handling of it is exercised from day one and phase 2 changes no
 * shape (§V.5).
 *
 * The plan is an ALLOW-LIST, not a permission check: the daemon syncs the set it
 * is handed and cannot accidentally include a repository it never asked about,
 * which a "may I sync X?" shape permits by omission. Offline therefore degrades
 * to no-sync by construction — no plan, nothing to sync — rather than needing a
 * nullable boolean whose NULL means no.
 */
export async function syncPlan(db: DojoClient, principalId: string): Promise<SyncPlan> {
	const { data, error } = await db
		.from('all_my_repositories')
		.select('repository_id, repo_key, tenant, sync_enabled, denied_reason')
		.eq('principal_id', principalId);
	if (error) throw new AdminError(500, error.message);

	const allowed: MappedRepo[] = [];
	const denied: { repo_key: string; tenant: string; reason: string }[] = [];
	for (const row of (data ?? []) as {
		repository_id: string;
		repo_key: string;
		tenant: string;
		sync_enabled: boolean;
		denied_reason?: string | null;
	}[]) {
		if (row.sync_enabled) {
			allowed.push({ repo_key: row.repo_key, tenant: row.tenant, repo_id: row.repository_id });
		} else {
			// A denial always names itself. "Nothing to sync" is the shape that hid
			// the original defect for two days; `no_seat` and `not_subscribed` are
			// different problems needing different advice.
			denied.push({
				repo_key: row.repo_key,
				tenant: row.tenant,
				reason: row.denied_reason ?? 'not_permitted'
			});
		}
	}
	return { allowed, denied };
}
