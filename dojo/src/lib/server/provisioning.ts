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
import { fetchGithubFacts, type ForgeFacts, type ForgeOrg, type ForgeProvider } from './forge-github';

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

/** What a provisioning pass established — the payload the daemon mirrors. */
export interface ProvisionResult {
	/** False when the pass could not read the forge; `reason` says why. Never
	 *  omitted on a partial pass: a silent no-op reading as success is exactly
	 *  how the original bug stayed invisible. */
	synced: boolean;
	reason?: 'no_forge_token' | 'forge_unreachable' | 'no_identity';
	personal: ProvisionedTenant | null;
	tenants: ProvisionedTenant[];
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
	name: string
): Promise<{ id: string; key: string; slug: string }> {
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
	}
	throw new AdminError(409, `could not find a free slug near "${preferredSlug}"`);
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
	const mine = await db.from('memberships').select('tenant_id, role').eq('user_id', principalId);
	if (mine.error) throw new AdminError(500, mine.error.message);
	const tenantIds = (mine.data ?? []).map((m) => (m as { tenant_id: string }).tenant_id);

	if (tenantIds.length > 0) {
		const owned = await db
			.from('tenants')
			.select('id, key, origin')
			.eq('origin', 'personal')
			.in('id', tenantIds);
		if (owned.error) throw new AdminError(500, owned.error.message);
		const existing = (owned.data ?? [])[0] as { id: string; key: string } | undefined;
		if (existing) {
			const role = (mine.data ?? []).find(
				(m) => (m as { tenant_id: string }).tenant_id === existing.id
			) as { role: string } | undefined;
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
	const tenant = await insertTenantWithFreeSlug(db, 'personal', slug, `${displayName}'s Dōjō`);
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
	if (error && (error as { code?: string }).code !== '23505') {
		throw new AdminError(500, error.message);
	}

	const role = await ensureMembership(
		db,
		tenant.id,
		principalId,
		'employer',
		authenticatedVia,
		roleForOrg(org)
	);
	return { id: tenant.id, key: tenant.key, origin: 'organization', role, created: true };
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
	return ensureProvisioned(db, principalId, facts, fallback);
}
