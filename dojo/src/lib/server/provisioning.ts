// ensureProvisioned — the operation that creates a tenant.
//
// Before this existed there were ZERO inserts into `dojo.tenants` anywhere in
// the app. `syncGithubMemberships` joined only tenants that already existed
// ("never invents a tenant"), so the first user in an org had nothing to join,
// and the personal dōjō that D1 promises every user was never created at all.
// Spec: docs/spec/dojo/dojo-auth-provisioning.md §II.7, §VIII.7 item 2.
//
// One idempotent operation with three callers, so "in sync regardless of where
// it was initiated" is a property of the design rather than two flows kept in
// step by hand. Idempotence rests on constraints that already exist —
// `(provider, subject)` on identities, `(provider, external_id)` on connections,
// `(tenant_id, user_id)` on memberships, `key` on tenants — so concurrent
// sign-ins converge rather than racing (Part I Scenario 22).
//
// TWO THINGS THIS DELIBERATELY DOES NOT DO, both phase 2:
//
//   * It never REMOVES. A pass that fails or reads nothing must never be read as
//     "the user left everything" — that would disable an entire org on a GitHub
//     outage. Only a positively-proved forge list may de-provision (§IV.6), and
//     that machinery does not exist yet.
//   * It never overwrites an existing membership role. `memberships.role` is
//     "usually git-derived, admin-overridable"; re-deriving it on every sign-in
//     would silently undo every override (Part I Scenario 5 keeps existing
//     memberships unchanged).
//
// Claim (`claimed_at` / `claim_state`) is also phase 2 — those columns do not
// exist yet, so every org tenant created here is implicitly unclaimed, which is
// the correct default: none of them has proved ownership.
import { AdminError, slugify, type DojoClient } from './admin-data';
import {
	fetchGithubFacts,
	fetchGithubRepoVisibility,
	type ForgeFacts,
	type ForgeOrg,
	type ForgeProvider,
	type ForgeVisibility
} from './forge-github';
import { forgeRefFromRepoKey } from './repo-mapping';

/** One tenant this pass established the caller's place in. */
export interface ProvisionedTenant {
	id: string;
	key: string;
	origin: 'personal' | 'organization';
	/** The caller's membership role in it. */
	role: string;
	/** True when this pass created the tenant, false when it already existed. */
	created: boolean;
}

/** What one forge-visibility refresh did, in ROWS — so the five buckets add up
 *  to the number of `dojo.repositories` rows the pass considered. Every row that
 *  was NOT written says which of the four reasons applies, rather than being
 *  absent from an "updated N" count that cannot be checked against anything. */
export interface VisibilityRefreshResult {
	/** Rows whose `visibility` + `visibility_captured_at` were written. */
	captured: number;
	/** Rows left alone because the forge answered 404 — this token cannot see the
	 *  repository (no access, or renamed upstream). */
	unavailable: number;
	/** Rows left alone because the forge read failed outright. */
	failed: number;
	/** Rows the per-pass cap left for a later pass. */
	deferred: number;
	/** Rows on a forge this token cannot speak for, or with a key we cannot
	 *  address as `owner/repo`. */
	unsupported: number;
}

/** What a provisioning pass established — the payload the daemon mirrors. */
export interface ProvisionResult {
	/** False when the pass could not read the forge; `reason` says why. Never
	 *  omitted on a partial pass: a silent no-op reading as success is exactly
	 *  how the original bug stayed invisible. */
	synced: boolean;
	reason?: 'no_forge_token' | 'forge_unreachable' | 'no_identity';
	personal: ProvisionedTenant | null;
	tenants: ProvisionedTenant[];
	/** Present only when the forge was actually read — a pass with no token, or
	 *  one that could not reach the forge, captured nothing and says so by
	 *  omission rather than by reporting five honest-looking zeros. */
	visibility?: VisibilityRefreshResult;
}

/** What we know about the caller when there is no forge to ask. */
export interface FallbackIdentity {
	email?: string | null;
	displayName?: string | null;
}

const DOJO_HOST = 'dojo.sensei-hq.org';

/** `dojo.forge_provider` → `dojo.auth_method`. The auth_method enum has no
 *  generic `oauth` label yet (§V.1 puts it in phase 2), so only github has a
 *  valid value — which is why phase 1 provisions github alone. */
function authMethodFor(provider: ForgeProvider): string {
	if (provider === 'github') return 'github_oauth';
	throw new AdminError(
		501,
		`provisioning from ${provider} needs the generic 'oauth' auth_method (phase 2)`
	);
}

/**
 * Record the claim when a forge OWNER/ADMIN signs in (§II.4).
 *
 * An org tenant is created by whoever signs in first, who may be a plain member
 * — so its existence proves nothing about who owns the org. Unclaimed, it may
 * not hold a subscription and can never sync private data.
 *
 * This is not new information: `roleForOrg` already reads `org.role` at every
 * sign-in, so a forge admin signing in IS the proof. No endpoint, no prompt, and
 * re-verified against the forge on every sign-in rather than trusted once.
 *
 * FIRST CLAIM WINS. The `is('claimed_at', null)` on the update is the guard, not
 * the read before it: two admins signing in together would both see NULL, and
 * without it the later write would rewrite `claimed_by` to whoever logged in
 * most recently — destroying the record of who actually established the claim.
 *
 * Errors PROPAGATE. A silently-failed claim leaves the tenant unclaimed, which
 * now refuses every private repository — a denial caused by an error nobody saw.
 */
export async function claimTenantIfOwner(
	db: DojoClient,
	tenantId: string,
	principalId: string,
	role: string
): Promise<void> {
	// Membership is not ownership. Only the forge's own admin/owner standing
	// claims a tenant; a contributor claiming it could subscribe on behalf of an
	// organisation that never agreed.
	if (role !== 'admin') return;

	const { error } = await db
		.from('tenants')
		.update({ claimed_at: new Date().toISOString(), claimed_by: principalId })
		.eq('id', tenantId)
		.is('claimed_at', null);
	if (error) throw new AdminError(500, error.message);
}

/** Forge standing → `dojo.member_role`. An org owner/admin administers the
 *  tenant; everyone else starts as a contributor and is promoted explicitly. */
function roleForOrg(org: ForgeOrg): string {
	return org.role === 'admin' ? 'admin' : 'contributor';
}

function tenantUrl(key: string): string {
	return `${DOJO_HOST}/${key}`;
}

/**
 * Insert a tenant, trying `slug`, `slug-2`, `slug-3`… until the unique on `key`
 * is satisfied. Two different humans can be `jerrythomas` on two forges, and two
 * unrelated orgs can share a name — the second must land somewhere of its own
 * rather than failing or, far worse, joining the first one's dōjō (§II.3).
 */
async function insertTenantWithFreeSlug(
	db: DojoClient,
	origin: 'personal' | 'organization',
	preferredSlug: string,
	name: string,
	/** Consulted on a key collision BEFORE escalating to `-2`. Returning a tenant
	 *  means "that collision is me, racing myself" — adopt it instead of forking.
	 *  Absent for orgs, which are arbitrated by their forge connection instead. */
	adoptIfMine?: (key: string) => Promise<{ id: string; key: string } | null>
): Promise<{ id: string; key: string; slug: string; adopted?: boolean }> {
	for (let attempt = 1; attempt <= 20; attempt += 1) {
		const slug = attempt === 1 ? preferredSlug : `${preferredSlug}-${attempt}`;
		const key = `${origin}/${slug}`;
		const { data, error } = await db
			.from('tenants')
			.insert({ key, origin, slug, name, dojo_url: tenantUrl(key), scope: 'private' })
			.select('id, key')
			.single();
		if (!error) return { id: (data as { id: string }).id, key, slug };
		if ((error as { code?: string }).code !== '23505') throw new AdminError(500, error.message);
		const mine = adoptIfMine ? await adoptIfMine(key) : null;
		if (mine) return { id: mine.id, key: mine.key, slug, adopted: true };
	}
	throw new AdminError(409, `could not find a free slug near "${preferredSlug}"`);
}

/**
 * On a PERSONAL key collision: is the colliding tenant mine, or another human's?
 *
 * `-2` exists for a real reason — two different people can both be `jerrythomas`
 * on two forges, and joining them into one dōjō would be far worse than a
 * duplicate (§II.3). But it is wrong when a pass collides with ITSELF, which is
 * what produced `personal/jerrythomas-2` two milliseconds after
 * `personal/jerrythomas`.
 *
 * The distinguisher is membership. If this principal is ALREADY a member of the
 * colliding tenant, it is mine and I lost a race against myself — adopt it.
 * Otherwise it belongs to someone else and the caller must keep escalating.
 *
 * Returns null rather than throwing: "not mine" is an ordinary answer here, and
 * the caller's next attempt is the correct response to it.
 */
export async function adoptOwnPersonalTenant(
	db: DojoClient,
	key: string,
	principalId: string
): Promise<{ id: string; key: string } | null> {
	const t = await db.from('tenants').select('id, key').eq('key', key).maybeSingle();
	if (t.error) throw new AdminError(500, t.error.message);
	if (!t.data) return null;
	const row = t.data as { id: string; key: string };

	const m = await db
		.from('memberships')
		.select('id')
		.eq('tenant_id', row.id)
		.eq('user_id', principalId)
		.maybeSingle();
	if (m.error) throw new AdminError(500, m.error.message);
	return m.data ? row : null;
}

/**
 * Resolve a LOST race for a forge org.
 *
 * A `23505` from the `tenant_connections` insert is not noise to swallow — it is
 * the definitive statement that this forge org is ALREADY connected to a tenant,
 * i.e. a concurrent pass created it first. Two passes genuinely do run at once:
 * kavach's `onSessionSync` and the console's `POST /v1/you/provision`.
 *
 * Observed live before this existed: nine tenants where five were expected —
 * `senecaglobalinc` AND `senecaglobalinc-2`, two milliseconds apart. The loser
 * had created a tenant, swallowed its own 23505, and left an orphan with no
 * connection and one membership, which the user saw as a duplicate dōjō.
 *
 * So the loser adopts the winner and REMOVES the tenant it just made. That is
 * safe precisely because it is brand new: it has no connection, no repositories
 * and no metrics — a membership may exist and cascades. It is emphatically NOT
 * safe for the winner, so discarding it is refused outright.
 */
export async function adoptConnectedTenant(
	db: DojoClient,
	provider: ForgeProvider,
	externalId: string,
	discardTenantId: string
): Promise<{ id: string; key: string }> {
	const conn = await db
		.from('tenant_connections')
		.select('tenant_id')
		.eq('provider', provider)
		.eq('external_id', externalId)
		.maybeSingle();
	if (conn.error) throw new AdminError(500, conn.error.message);
	if (!conn.data) {
		// The constraint fired with nothing to conflict against, so our model of it
		// is wrong. Returning the tenant we created would silently re-introduce the
		// fork this function exists to prevent.
		throw new AdminError(500, `tenant_connections conflicted for ${provider}:${externalId} but no connection could be read back`);
	}
	const winnerId = (conn.data as { tenant_id: string }).tenant_id;

	const t = await db.from('tenants').select('id, key').eq('id', winnerId).maybeSingle();
	if (t.error) throw new AdminError(500, t.error.message);
	if (!t.data) throw new AdminError(500, 'a tenant_connection points at a missing tenant');
	const winner = t.data as { id: string; key: string };

	// Never discard the winner. If the id we were handed IS the connected tenant
	// we did not lose anything, and deleting it would take the real tenant and
	// every membership on it.
	if (discardTenantId !== winner.id) {
		const del = await db.from('tenants').delete().eq('id', discardTenantId);
		if (del.error) throw new AdminError(500, del.error.message);
	}
	return winner;
}

/** One row of `dojo.memberships`, narrowed to what this module reads. */
interface Membership {
	tenant_id: string;
	role: string;
}

/** Every tenant the caller belongs to. This is the scope of everything a
 *  provisioning pass may touch on their behalf — §VIII.7 item 4 records what a
 *  dropped `tenant_id` filter already cost this codebase once. */
async function myMemberships(db: DojoClient, principalId: string): Promise<Membership[]> {
	const { data, error } = await db
		.from('memberships')
		.select('tenant_id, role')
		.eq('user_id', principalId)
		// ACTIVE memberships only. `dojo.all_my_repositories` and
		// `can_read_repository_metric` both carry this guard; without it a member
		// whose access was REVOKED still scopes writes into that tenant — and
		// `refreshForgeVisibility` writes `visibility`, which decides WHICH AUTHORITY
		// governs sharing for every remaining member. Losing access to a tenant must
		// remove your reach into it, not merely stop new grants.
		.is('disabled_at', null);
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as Membership[];
}

/** Create the membership if it is not already there, and report the role that
 *  ends up in force. An existing row WINS — see the header on overrides. */
async function ensureMembership(
	db: DojoClient,
	tenantId: string,
	principalId: string,
	kind: string,
	authenticatedVia: string,
	role: string
): Promise<string> {
	const existing = await db
		.from('memberships')
		.select('id, role')
		.eq('tenant_id', tenantId)
		.eq('user_id', principalId)
		.maybeSingle();
	if (existing.error) throw new AdminError(500, existing.error.message);
	if (existing.data) return (existing.data as { role: string }).role;

	const { data, error } = await db
		.from('memberships')
		.insert({
			tenant_id: tenantId,
			user_id: principalId,
			kind,
			authenticated_via: authenticatedVia,
			role
		})
		.select('role')
		.single();
	if (error) {
		// Lost a race with a concurrent pass — the constraint did its job.
		if ((error as { code?: string }).code === '23505') return role;
		throw new AdminError(500, error.message);
	}
	return (data as { role: string }).role;
}

/**
 * Record that this forge account belongs to this principal.
 *
 * NOT an upsert on `(provider, subject)`. An upsert would re-point an identity
 * that already belongs to somebody else — silently handing a second principal
 * every membership derived from that account. One GitHub account is one person;
 * a conflict is a 409 that surfaces (it is the shape an accidental Supabase
 * account merge takes), never a quiet reassignment.
 */
async function ensureIdentity(
	db: DojoClient,
	principalId: string,
	facts: ForgeFacts
): Promise<void> {
	const provider = authMethodFor(facts.provider);
	const now = new Date().toISOString();

	const found = await db
		.from('identities')
		.select('id, principal_id')
		.eq('provider', provider)
		.eq('subject', facts.user.id)
		.maybeSingle();
	if (found.error) throw new AdminError(500, found.error.message);

	if (found.data) {
		const row = found.data as { id: string; principal_id: string };
		if (row.principal_id !== principalId) {
			throw new AdminError(
				409,
				'that forge account is already linked to a different person'
			);
		}
		const upd = await db
			.from('identities')
			.update({
				email: facts.user.email,
				display_name: facts.user.name,
				last_login_at: now
			})
			.eq('id', row.id)
			.select('id')
			.maybeSingle();
		if (upd.error) throw new AdminError(500, upd.error.message);
		return;
	}

	const { error } = await db
		.from('identities')
		.insert({
			principal_id: principalId,
			provider,
			subject: facts.user.id,
			email: facts.user.email,
			display_name: facts.user.name,
			last_login_at: now
		})
		.select('id')
		.single();
	if (error) {
		// A concurrent pass inserted it first; the winner is this same principal
		// unless someone else owns the account, which the read above would have
		// caught. Converge rather than fail.
		if ((error as { code?: string }).code === '23505') return;
		throw new AdminError(500, error.message);
	}
}

/**
 * The caller's personal dōjō, created if absent (D1 — always active, no
 * activation step).
 *
 * Found by the caller's existing personal MEMBERSHIP, never by re-deriving the
 * slug: a forge handle is renameable, and re-deriving would create a second
 * personal dōjō and strand the first one's history.
 */
async function ensurePersonalTenant(
	db: DojoClient,
	principalId: string,
	preferredSlug: string,
	displayName: string,
	authenticatedVia: string
): Promise<ProvisionedTenant | null> {
	const mine = await myMemberships(db, principalId);
	const tenantIds = mine.map((m) => m.tenant_id);

	if (tenantIds.length > 0) {
		const owned = await db
			.from('tenants')
			.select('id, key, origin')
			.eq('origin', 'personal')
			.in('id', tenantIds);
		if (owned.error) throw new AdminError(500, owned.error.message);
		const existing = (owned.data ?? [])[0] as { id: string; key: string } | undefined;
		if (existing) {
			const role = mine.find((m) => m.tenant_id === existing.id);
			return {
				id: existing.id,
				key: existing.key,
				origin: 'personal',
				role: role?.role ?? 'admin',
				created: false
			};
		}
	}

	const slug = slugify(preferredSlug);
	if (!slug) return null;
	const tenant = await insertTenantWithFreeSlug(
		db,
		'personal',
		slug,
		`${displayName}'s Dōjō`,
		(key) => adoptOwnPersonalTenant(db, key, principalId)
	);
	const role = await ensureMembership(
		db,
		tenant.id,
		principalId,
		'personal',
		authenticatedVia,
		'admin'
	);
	return { id: tenant.id, key: tenant.key, origin: 'personal', role, created: true };
}

/**
 * The tenant for one forge org, created with its connection if this org has
 * never been seen.
 *
 * Matched on `(provider, external_id)` — the forge's STABLE id. Matching on the
 * slug would fork the tenant when an org is renamed, and would let whoever
 * claims the freed name upstream inherit this tenant's governance (§II.2).
 */
async function ensureOrgTenant(
	db: DojoClient,
	principalId: string,
	provider: ForgeProvider,
	org: ForgeOrg,
	authenticatedVia: string
): Promise<ProvisionedTenant> {
	const conn = await db
		.from('tenant_connections')
		.select('tenant_id')
		.eq('provider', provider)
		.eq('external_id', org.id)
		.maybeSingle();
	if (conn.error) throw new AdminError(500, conn.error.message);

	if (conn.data) {
		const tenantId = (conn.data as { tenant_id: string }).tenant_id;
		const t = await db.from('tenants').select('id, key').eq('id', tenantId).maybeSingle();
		if (t.error) throw new AdminError(500, t.error.message);
		if (!t.data) throw new AdminError(500, 'a tenant_connection points at a missing tenant');
		const row = t.data as { id: string; key: string };
		const role = await ensureMembership(
			db,
			row.id,
			principalId,
			'employer',
			authenticatedVia,
			roleForOrg(org)
		);
		// A tenant may have been created by a plain member long before an owner
		// first signs in, so the claim is attempted on EVERY pass, not only at
		// creation. `claimTenantIfOwner` is a no-op once claimed.
		await claimTenantIfOwner(db, row.id, principalId, roleForOrg(org));
		return { id: row.id, key: row.key, origin: 'organization', role, created: false };
	}

	const tenant = await insertTenantWithFreeSlug(
		db,
		'organization',
		slugify(org.login) || org.login,
		org.login
	);

	// verified_at: this pass read the org from the forge with the user's own
	// token, which is exactly what "org control was proven" means. An unverified
	// connection confers no entitlement (§VI.2), so recording it honestly matters.
	const { error } = await db
		.from('tenant_connections')
		.insert({
			tenant_id: tenant.id,
			provider,
			external_id: org.id,
			external_slug: org.login,
			connected_by: principalId,
			verified_at: new Date().toISOString()
		})
		.select('id')
		.single();
	if (error) {
		if ((error as { code?: string }).code !== '23505') {
			throw new AdminError(500, error.message);
		}
		// WE LOST A CONCURRENT PASS. This 23505 says the forge org is already
		// connected to a tenant — so the one we just created above is redundant.
		// Swallowing this is what produced `senecaglobalinc-2`: an orphan tenant
		// with no connection, plus a membership pointing at it, which the user saw
		// as a duplicate organisation they belong to once.
		const winner = await adoptConnectedTenant(db, provider, org.id, tenant.id);
		const joinedRole = await ensureMembership(
			db,
			winner.id,
			principalId,
			'employer',
			authenticatedVia,
			roleForOrg(org)
		);
		return { id: winner.id, key: winner.key, origin: 'organization', role: joinedRole, created: false };
	}

	const role = await ensureMembership(
		db,
		tenant.id,
		principalId,
		'employer',
		authenticatedVia,
		roleForOrg(org)
	);
	// The creator may themselves be the forge owner, in which case the tenant is
	// claimed the moment it exists rather than waiting for a second sign-in.
	await claimTenantIfOwner(db, tenant.id, principalId, roleForOrg(org));
	return { id: tenant.id, key: tenant.key, origin: 'organization', role, created: true };
}

/**
 * How many repositories one pass will ask the forge about.
 *
 * A Cloudflare Worker has a hard per-invocation subrequest budget, and this
 * codebase has already lost an entire metrics batch to it (see the note on
 * `FakeTable.reads`). So the pass is BOUNDED and reports what it left behind,
 * rather than growing with a user's repository count until sign-in starts
 * failing for whoever has the most repositories.
 */
export const MAX_VISIBILITY_REFRESH_PER_PASS = 40;

/** One `dojo.repositories` row this pass may refresh. */
interface RepoVisibilityRow {
	id: string;
	repo_key: string;
	visibility_captured_at: string | null;
}

/**
 * `owner`/`repo` for a GitHub repository key, or null when the key is not one a
 * GitHub token can be asked about.
 *
 * Which host is which forge stays in `repo-mapping.ts` — re-deciding it here
 * would be a second copy of the rule that says `github.com.attacker.net` is not
 * GitHub. Only the repository segment is read locally, and only for the exact
 * `host/owner/repo` shape the normaliser produces; a longer path is a key we
 * cannot address, and guessing which segment is the repository would ask the
 * forge about a different one.
 */
function githubOwnerRepo(repoKey: string): { owner: string; repo: string } | null {
	const ref = forgeRefFromRepoKey(repoKey);
	if (!ref || ref.provider !== 'github') return null;
	const parts = repoKey.trim().toLowerCase().split('/').filter((p) => p.length > 0);
	if (parts.length !== 3) return null;
	return { owner: ref.org, repo: parts[2] };
}

/** Uncaptured first, then oldest capture first — the rows that cannot sync at
 *  all are served before the ones that merely have an ageing answer. An
 *  unparseable timestamp sorts as uncaptured, because we cannot tell its age. */
function captureAge(row: RepoVisibilityRow): number {
	const at = row.visibility_captured_at ? Date.parse(row.visibility_captured_at) : NaN;
	// A finite sentinel, not -Infinity: comparing two uncaptured rows would then
	// subtract two infinities and hand `sort` a NaN.
	return Number.isNaN(at) ? -1 : at;
}

/**
 * Refresh the forge's visibility answer for repositories the dōjō ALREADY has.
 *
 * This is the capture step §8a resolves the chicken/egg with. `dojo.repositories`
 * rows are created by `registerRepositories` and nothing else, and that caller
 * holds a SUPABASE token — it cannot ask the forge anything. A provider token
 * exists server-side only during the sign-in pass, so capture happens here.
 *
 * THREE PROPERTIES, each from a review finding, each with a test:
 *
 *   1. It REFRESHES, it never INSERTS. Creating a row per visible forge repo was
 *      rejected outright: it would disclose repositories the user never chose to
 *      disclose, turning a sign-in into an inventory upload. The set asked about
 *      is exactly the set already registered.
 *   2. It is SCOPED TO THE SIGNER'S OWN TENANTS. `dojo.repositories` is
 *      `unique (tenant_id, repo_key)`, so the same repository legitimately exists
 *      under two tenants; an update keyed on `repo_key` alone would let user A's
 *      token rewrite tenant B's row. Since visibility decides which AUTHORITY
 *      governs a repository, that would change who governs it for every member of
 *      B. An authorization boundary, not an optimisation.
 *   3. A read that does not succeed writes NOTHING. Never a guessed `private` —
 *      in an org tenant that guess resolves to org-MANDATED and shares the
 *      repository with no election by anyone (§8a, BLOCKING).
 *
 * `visibility` and `visibility_captured_at` are always written together: a value
 * with no timestamp is indistinguishable from the old bad column default, and the
 * view's staleness rule would have nothing to measure.
 */
export async function refreshForgeVisibility(
	db: DojoClient,
	principalId: string,
	token: string,
	fetchImpl: typeof fetch = fetch
): Promise<VisibilityRefreshResult> {
	const out: VisibilityRefreshResult = {
		captured: 0,
		unavailable: 0,
		failed: 0,
		deferred: 0,
		unsupported: 0
	};

	const tenantIds = [...new Set((await myMemberships(db, principalId)).map((m) => m.tenant_id))];
	if (tenantIds.length === 0) return out;

	const found = await db
		.from('repositories')
		.select('id, repo_key, visibility_captured_at')
		.in('tenant_id', tenantIds);
	if (found.error) throw new AdminError(500, found.error.message);

	// One forge read per repository, however many tenants registered it.
	const byRepo = new Map<string, { owner: string; repo: string; rows: RepoVisibilityRow[] }>();
	for (const row of (found.data ?? []) as RepoVisibilityRow[]) {
		const ref = githubOwnerRepo(row.repo_key);
		if (!ref) {
			out.unsupported += 1;
			continue;
		}
		const group = byRepo.get(row.repo_key);
		if (group) group.rows.push(row);
		else byRepo.set(row.repo_key, { ...ref, rows: [row] });
	}

	const groups = [...byRepo.values()].sort(
		(a, b) => Math.min(...a.rows.map(captureAge)) - Math.min(...b.rows.map(captureAge))
	);
	for (const group of groups.slice(MAX_VISIBILITY_REFRESH_PER_PASS)) {
		out.deferred += group.rows.length;
	}

	const capturedAt = new Date().toISOString();
	for (const group of groups.slice(0, MAX_VISIBILITY_REFRESH_PER_PASS)) {
		let visibility: ForgeVisibility | null;
		try {
			visibility = await fetchGithubRepoVisibility(group.owner, group.repo, token, fetchImpl);
		} catch {
			// The forge would not answer. The row keeps whatever it had — including
			// nothing — and the view fails closed on it.
			out.failed += group.rows.length;
			continue;
		}
		if (!visibility) {
			out.unavailable += group.rows.length;
			continue;
		}
		for (const row of group.rows) {
			// By primary key, drawn from the membership-scoped read above — which is
			// what keeps property 2 true of the WRITE and not just of the read.
			const upd = await db
				.from('repositories')
				.update({ visibility, visibility_captured_at: capturedAt })
				.eq('id', row.id);
			if (upd.error) throw new AdminError(500, upd.error.message);
			out.captured += 1;
		}
	}

	if (out.failed || out.unavailable) {
		// Named, not swallowed. These rows will not sync and the reason is not
		// visible anywhere else: the sign-in hook discards its result.
		console.warn('forge-visibility: repositories left uncaptured', {
			principalId,
			failed: out.failed,
			unavailable: out.unavailable
		});
	}
	return out;
}

/**
 * Provision everything one pass can prove, idempotently.
 *
 * `facts` null means no forge token was available. `provider_token` exists only
 * immediately after the OAuth exchange, so this is the ordinary case for any
 * later call — and it MUST report `synced: false` with a reason rather than
 * appearing to succeed (§II.7). The personal dōjō is still ensured, because D1
 * is unconditional; only the org half needs the forge.
 */
export async function ensureProvisioned(
	db: DojoClient,
	principalId: string,
	facts: ForgeFacts | null,
	fallback: FallbackIdentity = {}
): Promise<ProvisionResult> {
	if (!facts) {
		// Name the personal dōjō from the email's local part. A derived slug, not
		// an invented identity — and when there is nothing to derive from we
		// create nothing rather than minting `user-a1b2c3`, which would put a
		// fabricated name in a user-visible URL.
		const local = (fallback.email ?? '').split('@')[0] ?? '';
		const slug = slugify(local);
		if (!slug) {
			return { synced: false, reason: 'no_identity', personal: null, tenants: [] };
		}
		const personal = await ensurePersonalTenant(
			db,
			principalId,
			slug,
			fallback.displayName?.trim() || local,
			'sso'
		);
		return { synced: false, reason: 'no_forge_token', personal, tenants: [] };
	}

	const authenticatedVia = authMethodFor(facts.provider);
	await ensureIdentity(db, principalId, facts);

	const personal = await ensurePersonalTenant(
		db,
		principalId,
		facts.user.login,
		facts.user.name?.trim() || facts.user.login,
		authenticatedVia
	);

	const tenants: ProvisionedTenant[] = [];
	for (const org of facts.orgs) {
		tenants.push(await ensureOrgTenant(db, principalId, facts.provider, org, authenticatedVia));
	}

	return { synced: true, personal, tenants };
}

/**
 * Read the forge (when there is a token) and provision — the composition all
 * three callers of §II.7 share, so "in sync regardless of where it was
 * initiated" stays a property of one function rather than three that drift.
 *
 * The three outcomes are kept DISTINCT on purpose:
 *
 *   synced: true                     the forge was read and everything provisioned
 *   reason: 'no_forge_token'         not a forge sign-in, or the token has expired
 *                                    out of the session (the ordinary later case)
 *   reason: 'forge_unreachable'      we had a token and the forge would not answer
 *
 * Collapsing the last two into one "nothing to sync" is precisely the shape that
 * kept the original defect invisible for two days, and they call for different
 * advice: one says "sign in with GitHub", the other says "try again".
 *
 * A failed forge read provisions NO org tenant — an org invented from an
 * unsuccessful read is a governance boundary conjured out of an outage. The
 * personal dōjō is still ensured either way, because D1 does not depend on any
 * forge.
 */
export async function provisionWithToken(
	db: DojoClient,
	principalId: string,
	providerToken: string | null | undefined,
	fallback: FallbackIdentity = {},
	fetchImpl: typeof fetch = fetch
): Promise<ProvisionResult> {
	if (!providerToken) return ensureProvisioned(db, principalId, null, fallback);

	let facts: ForgeFacts;
	try {
		facts = await fetchGithubFacts(providerToken, fetchImpl);
	} catch {
		const degraded = await ensureProvisioned(db, principalId, null, fallback);
		// Keep `no_identity` if that is what actually stopped us — it is a more
		// specific answer than "the forge was down".
		return degraded.reason === 'no_identity'
			? degraded
			: { ...degraded, reason: 'forge_unreachable' };
	}
	const result = await ensureProvisioned(db, principalId, facts, fallback);
	// AFTER provisioning, deliberately: an org tenant this pass just created is a
	// membership the refresh's scope must include, or the first sign-in of the
	// first employee captures nothing for their employer's repositories.
	const visibility = await refreshForgeVisibility(db, principalId, providerToken, fetchImpl);
	return { ...result, visibility };
}
