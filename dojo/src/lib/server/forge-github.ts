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

/**
 * A forge read that came back non-2xx, carrying the FORGE's status.
 *
 * Separate from the 502 we answer our own caller with. "401 — this credential is
 * dead" and "503 — try again shortly" both reach a caller as a 502, and the
 * difference between them decides whether the user is told to sign in again or
 * to wait. Keeping the forge status only inside the message text meant every
 * consumer had to parse English to recover it, so none did: both collapsed into
 * `forge_unreachable`, whose console copy is "try again in a moment" — advice
 * that can never come true for a revoked grant.
 */
export class ForgeReadError extends AdminError {
	constructor(
		readonly forgeStatus: number,
		message: string
	) {
		super(502, message);
	}
}

async function getJson(url: string, token: string, fetchImpl: typeof fetch): Promise<unknown> {
	const res = await fetchImpl(url, { headers: headers(token) });
	if (!res.ok) throw new ForgeReadError(res.status, `GitHub read failed (${res.status}) for ${url}`);
	return res.json();
}

/** The forge's answer about one repository — `dojo.repositories.visibility`.
 *  Deliberately NOT the same vocabulary as `sensei.repositories.visibility`
 *  (`private | shared`), which records INTENT; this records what the forge says. */
export type ForgeVisibility = 'private' | 'public';

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

/**
 * Whether the forge considers one repository public or private.
 *
 * The single fact `dojo.repositories.visibility` is allowed to come from — it is
 * never inferred from the remote URL or the owner's name (sharing acceptance
 * criterion 6), because both are guesses and this column drives BOTH gates: a
 * wrong `public` hosts private code for free, and a wrong `private` in an org
 * tenant makes it org-MANDATED and shares it with no election by anyone.
 *
 * Three outcomes, kept distinct because they call for different handling:
 *
 *   'public' | 'private'   the forge answered
 *   null                   404 — this token cannot see the repository (no
 *                          access, or renamed upstream). A definite "we do not
 *                          know", so the caller leaves the row uncaptured.
 *   throws ForgeReadError  any other non-2xx — a FAULT, not an answer, and never
 *                          a defaulted visibility. Carries the forge's status so
 *                          a dead credential is distinguishable from an outage.
 *   throws AdminError      a 2xx whose body has no usable `private` field.
 */
export async function fetchGithubRepoVisibility(
	owner: string,
	repo: string,
	token: string,
	fetchImpl: typeof fetch = fetch
): Promise<ForgeVisibility | null> {
	// Encoded because both segments arrive from a stored `repo_key`; a raw slash
	// or space would let a segment address a different API path.
	const url = `${GITHUB_API}/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}`;
	const res = await fetchImpl(url, { headers: headers(token) });
	if (res.status === 404) return null;
	if (!res.ok) throw new ForgeReadError(res.status, `GitHub read failed (${res.status}) for ${url}`);

	const body = (await res.json()) as Record<string, unknown> | null;
	// `!body?.private` would read every unreadable response as "public" — the
	// free-to-host path. A shape we cannot read is a failed read.
	if (typeof body?.private !== 'boolean') {
		throw new AdminError(502, `GitHub returned no visibility for ${owner}/${repo}`);
	}
	return body.private ? 'private' : 'public';
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
