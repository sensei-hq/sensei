// Reading the facts a forge will vouch for, with the USER'S OWN token.
//
// Provisioning must never trust a client's claim about its own entitlements
// (§II.5, and §IV.8's design consequence: the daemon holds the token, the dōjō
// makes the decisions). So the daemon sends the token and the dōjō reads the
// org list itself — it never accepts a list of orgs from a caller.
//
// Fail closed everywhere. A non-2xx throws; a shape we do not recognise is
// skipped rather than guessed at. The one thing this module must never do is
// return a plausible org list built from anything but a live, successful read —
// an invented org becomes a tenant, and a tenant is a governance boundary.
//
// Supersedes `fetchGithubOrgLogins` in github-sync-data.ts, which returned only
// logins: provisioning needs the forge's STABLE id for `tenant_connections`
// (§II.2 — a slug can be renamed and re-registered upstream, so keying on it
// would let a squatter inherit a tenant's governance) and the caller's role.
import { AdminError } from './admin-data';

/** `dojo.forge_provider`. Phase 1 provisions `github` only — `dojo.auth_method`
 *  has no generic `oauth` label yet, so an identity on another forge has nothing
 *  valid to record (spec §VIII.6). The type is here because the model is
 *  forge-agnostic by design (D6) and the shape must not have to change later. */
export type ForgeProvider = 'github' | 'gitlab' | 'bitbucket' | 'azure_devops';

/** The person, as the forge knows them. */
export interface ForgeUser {
	/** The forge's stable user id, as text — `dojo.identities.subject`. Never the
	 *  login, which is renameable. */
	id: string;
	/** The forge's handle. Used only to propose a personal-dōjō slug. */
	login: string;
	name: string | null;
	email: string | null;
}

/** One organisation the forge confirms the caller belongs to. */
export interface ForgeOrg {
	/** The forge's stable org id — `tenant_connections.external_id`. */
	id: string;
	/** The forge's name for the org — `tenant_connections.external_slug`. */
	login: string;
	/** The caller's standing in the org, as the forge reports it. */
	role: 'admin' | 'member';
}

/** Everything one provisioning pass learned from one forge. `orgs` is only ever
 *  a POSITIVELY read list — see `ensureProvisioned` for why that distinction
 *  matters (§IV.6: a failed read must never be read as "the user left"). */
export interface ForgeFacts {
	provider: ForgeProvider;
	user: ForgeUser;
	orgs: ForgeOrg[];
}

const GITHUB_API = 'https://api.github.com';

function headers(token: string): HeadersInit {
	return {
		Authorization: `Bearer ${token}`,
		Accept: 'application/vnd.github+json',
		'X-GitHub-Api-Version': '2022-11-28',
		'User-Agent': 'sensei-dojo'
	};
}

/** A stable id as text. GitHub returns numbers; Azure returns GUIDs. Rejects
 *  anything else rather than coercing — `String(undefined)` is `"undefined"`,
 *  which would happily become a unique connection key. */
function idText(v: unknown): string | null {
	if (typeof v === 'number' && Number.isFinite(v)) return String(v);
	if (typeof v === 'string' && v.trim()) return v.trim();
	return null;
}

function nonEmpty(v: unknown): string | null {
	return typeof v === 'string' && v.trim() ? v.trim() : null;
}

async function getJson(url: string, token: string, fetchImpl: typeof fetch): Promise<unknown> {
	const res = await fetchImpl(url, { headers: headers(token) });
	if (!res.ok) throw new AdminError(502, `GitHub read failed (${res.status}) for ${url}`);
	return res.json();
}

/** The authenticated GitHub user. `fetchImpl` is injected for tests. */
export async function fetchGithubUser(
	token: string,
	fetchImpl: typeof fetch = fetch
): Promise<ForgeUser> {
	const body = (await getJson(`${GITHUB_API}/user`, token, fetchImpl)) as Record<string, unknown>;
	const id = idText(body?.id);
	const login = nonEmpty(body?.login);
	// No id or no login means we cannot key an identity or propose a slug. That
	// is a failed read, not a user without a name.
	if (!id || !login) throw new AdminError(502, 'GitHub returned a user without an id or login');
	return { id, login, name: nonEmpty(body?.name), email: nonEmpty(body?.email) };
}

/**
 * The organisations GitHub confirms the caller is an ACTIVE member of.
 *
 * Uses `/user/memberships/orgs` rather than `/user/orgs` because it carries both
 * the org's stable id and the caller's `role` in one read, and because it
 * exposes `state` — a **pending** invitation is not membership, and provisioning
 * a tenant from one would hand someone a governance boundary they were merely
 * invited to.
 *
 * An entry missing an id or login is SKIPPED, not defaulted: a connection is a
 * claim of identity, and a guessed one is worse than none (§F7).
 */
export async function fetchGithubOrgs(
	token: string,
	fetchImpl: typeof fetch = fetch
): Promise<ForgeOrg[]> {
	const body = await getJson(
		`${GITHUB_API}/user/memberships/orgs?per_page=100&state=active`,
		token,
		fetchImpl
	);
	if (!Array.isArray(body)) return [];
	const orgs: ForgeOrg[] = [];
	for (const entry of body) {
		const m = entry as Record<string, unknown>;
		if (nonEmpty(m.state) !== 'active') continue;
		const org = (m.organization ?? {}) as Record<string, unknown>;
		const id = idText(org.id);
		const login = nonEmpty(org.login);
		if (!id || !login) continue;
		orgs.push({ id, login, role: nonEmpty(m.role) === 'admin' ? 'admin' : 'member' });
	}
	return orgs;
}

/** Both reads for one provisioning pass. Throws (never returns a partial) so a
 *  caller can only ever act on a complete, positively-read picture. */
export async function fetchGithubFacts(
	token: string,
	fetchImpl: typeof fetch = fetch
): Promise<ForgeFacts> {
	const [user, orgs] = await Promise.all([
		fetchGithubUser(token, fetchImpl),
		fetchGithubOrgs(token, fetchImpl)
	]);
	return { provider: 'github', user, orgs };
}
