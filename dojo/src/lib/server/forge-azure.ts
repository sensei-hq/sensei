// Azure DevOps behind the one forge interface (#308 R5b, D36/D42).
//
// A unit is an Azure DevOps ORGANIZATION (its own dōjō); its PROJECTS become
// workspaces inside it. Every read is made with a Microsoft Entra ACCESS token
// for the Azure DevOps resource (azure-entra.ts), sent as a bearer — never in a
// URL, never logged, never in an error message. The dōjō stores only the
// refresh token it came from (D42).
//
// ONE ORGANIZATION PER ADAPTER. `azureAdapter(org)` reads one organization the
// admin picked from `listAzureOrganizations` (the organizations the token's
// owner belongs to); `listUnits` answers whether that owner is active in it.
//
// SAFE BY CONSTRUCTION. Requests reach only Azure DevOps' API hosts
// (AZURE_HOSTS), over https, without following a redirect (Azure answers a bad
// token with a sign-in page). The organization name is validated before it is
// put in a URL; descriptors and continuation tokens Azure hands back are
// validated or encoded before they go back into one.
//
// FAIL CLOSED, as forge-github does: a read that could not be made THROWS
// (ForgeReadError keeps Azure's status, so a dead token — 401 — is told apart
// from an outage); `proveAdmin` never answers false for a read it could not
// make; a member read that could not name an entry says `complete: false`, and
// one that could not finish throws (#309: only a complete read ends anything).
import { AdminError } from './admin-data';
import type { ForgeAdapter, ForgeMember, ForgeMembersRead, ForgeUnit, ForgeUnitRef, ForgeUnitsRead } from './forge-adapter';
import { ForgeReadError } from './forge-github';

/** The only hosts an Azure DevOps read may reach. */
const AZURE_HOSTS = new Set(['app.vssps.visualstudio.com', 'dev.azure.com', 'vssps.dev.azure.com', 'vsaex.dev.azure.com']);

/** An organization name: letters, digits and hyphens, starting with a letter or
 *  digit, at most 50 characters (Azure DevOps' own naming rule). Nothing that
 *  could add a path segment, a query or a host. */
const ORG_NAME = /^[a-z0-9][a-z0-9-]{0,49}$/;

/** A Graph subject descriptor, as Azure returns them (`aad.…`, `vssgp.…`). */
const DESCRIPTOR = /^[A-Za-z0-9][A-Za-z0-9._-]{0,511}$/;

/** At most this many pages of groups are read looking for the admin group. */
const MAX_GROUP_PAGES = 20;
/** At most this many membership lookups while proving the admin (the person's
 *  own groups and the groups those are in). */
const MAX_MEMBERSHIP_LOOKUPS = 100;
/** At most this many pages of user entitlements; a larger organization THROWS
 *  rather than returning a shorter list (#309). */
export const MAX_ENTITLEMENT_PAGES = 200;
/** Projects are read 100 at a time, at most this many pages. */
const PROJECT_PAGE = 100;
const MAX_PROJECT_PAGES = 50;

/** The read scopes the Entra connect asks for — one list, the screen’s (azure-view.ts). */
export { AZURE_REQUIRED_SCOPES } from '../azure-view';

/** The org name, checked, or a 400 — before it is put in any URL. */
function orgName(org: string): string {
	const name = (typeof org === 'string' ? org : '').trim().toLowerCase();
	if (!ORG_NAME.test(name)) throw new AdminError(400, 'that is not an Azure DevOps organization name');
	return name;
}

/** An access token is printable ASCII with no whitespace (an Entra JWT, a few
 *  KB); anything else is refused before it reaches a header. Its value never
 *  appears in the message. */
function checkToken(token: string): void {
	if (typeof token !== 'string' || !/^[\x21-\x7e]{1,16384}$/.test(token)) {
		throw new AdminError(400, 'that is not an Azure DevOps access token');
	}
}

function nonEmpty(v: unknown): string | null {
	return typeof v === 'string' && v.trim() ? v.trim() : null;
}

interface AzureAnswer {
	body: Record<string, unknown>;
	headers: Headers;
}

/**
 * One GET against an Azure DevOps API host, with the Entra access token as a
 * bearer (#308 R5b, D42). Throws
 * ForgeReadError (Azure's status) on anything but a 200 JSON answer — a 203 or a
 * redirect is Azure's sign-in page for a token it will not take.
 */
async function azureGet(url: string, token: string, fetchImpl: typeof fetch): Promise<AzureAnswer> {
	const u = new URL(url);
	if (u.protocol !== 'https:' || !AZURE_HOSTS.has(u.host)) {
		throw new AdminError(500, 'refusing to send an Azure DevOps token to a host that is not Azure DevOps');
	}
	const where = `${u.host}${u.pathname}`;
	const res = await fetchImpl(u.toString(), {
		headers: { Authorization: `Bearer ${token}`, Accept: 'application/json', 'User-Agent': 'sensei-dojo' },
		redirect: 'manual'
	});
	if (res.status !== 200) throw new ForgeReadError(res.status, `Azure DevOps read failed (${res.status}) for ${where}`);
	const type = res.headers?.get('content-type') ?? '';
	if (!type.toLowerCase().includes('json')) {
		throw new ForgeReadError(203, `Azure DevOps answered ${where} with something other than JSON (a sign-in page?)`);
	}
	const body = (await res.json().catch(() => null)) as unknown;
	if (!body || typeof body !== 'object') throw new AdminError(502, `Azure DevOps returned no readable answer for ${where}`);
	return { body: body as Record<string, unknown>, headers: res.headers };
}

/** One Azure DevOps organization the token's owner belongs to. */
export interface AzureOrganization {
	/** The account id the Accounts API names it by. */
	id: string;
	/** Its name — what its URLs carry (lowercased). */
	name: string;
}

/**
 * The organizations the token's owner belongs to — the Profiles API's `me`, then
 * the Accounts API by that member id (both need `vso.profile` and an Entra
 * token —
 * https://learn.microsoft.com/en-us/rest/api/azure/devops/profile/profiles/get?view=azure-devops-rest-7.1,
 * https://learn.microsoft.com/en-us/rest/api/azure/devops/account/accounts/list?view=azure-devops-rest-7.1).
 * An account whose name could not be an organization's is skipped and the read
 * marked incomplete; a failed read throws — never an empty list.
 */
export async function listAzureOrganizations(token: string, fetchImpl: typeof fetch = fetch): Promise<{ organizations: AzureOrganization[]; complete: boolean }> {
	checkToken(token);
	const { body: me } = await azureGet('https://app.vssps.visualstudio.com/_apis/profile/profiles/me?api-version=7.1', token, fetchImpl);
	const memberId = nonEmpty(me.id);
	if (!memberId || !/^[0-9a-fA-F-]{36}$/.test(memberId)) throw new AdminError(502, 'Azure DevOps returned a profile without an id');
	const { body } = await azureGet(
		`https://app.vssps.visualstudio.com/_apis/accounts?memberId=${memberId}&api-version=7.1`,
		token,
		fetchImpl
	);
	if (!Array.isArray(body.value)) throw new AdminError(502, 'Azure DevOps returned organizations that are not a list');
	const organizations: AzureOrganization[] = [];
	let complete = true;
	for (const a of body.value as Record<string, unknown>[]) {
		const id = nonEmpty(a?.accountId);
		const name = (nonEmpty(a?.accountName) ?? '').toLowerCase();
		if (!id || !ORG_NAME.test(name)) {
			complete = false;
			continue;
		}
		organizations.push({ id, name });
	}
	return { organizations, complete };
}

/** A Microsoft Entra object id (a GUID), lowercased, or null. */
function entraObjectId(v: unknown): string | null {
	const id = (nonEmpty(v) ?? '').toLowerCase();
	return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(id) ? id : null;
}

/** The token's owner as the dōjō records them (D45): their Microsoft Entra
 *  object id — the stable id a Microsoft sign-in (R5c) will also see. */
export interface AzureOwner {
	objectId: string;
	displayName: string | null;
}

/**
 * The token's owner, by Microsoft Entra object id: connectionData names the
 * owner's Graph descriptor; Graph Users - Get answers `origin` and `originId`
 * (https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/users/get?view=azure-devops-rest-7.1).
 * An owner who is not an Entra (`aad`) user, or has no object id, is refused:
 * an Entra connect always yields one, so its absence is a read we cannot key.
 */
async function readAzureOwner(name: string, token: string, fetchImpl: typeof fetch): Promise<AzureOwner> {
	const conn = await readAzureConnection(name, token, fetchImpl);
	if (!DESCRIPTOR.test(conn.user.descriptor)) throw new AdminError(502, 'Azure DevOps returned a descriptor this cannot address');
	const { body } = await azureGet(
		`https://vssps.dev.azure.com/${name}/_apis/graph/users/${encodeURIComponent(conn.user.descriptor)}?api-version=7.1-preview.1`,
		token,
		fetchImpl
	);
	const objectId = (nonEmpty(body.origin) ?? '').toLowerCase() === 'aad' ? entraObjectId(body.originId) : null;
	if (!objectId) {
		throw new AdminError(422, 'this Azure DevOps account is not a Microsoft Entra work or school account — personal Microsoft accounts cannot connect');
	}
	return { objectId, displayName: nonEmpty(body.displayName) ?? conn.user.displayName };
}

/** The token's owner as Azure names them in one organization. */
export interface AzureUser {
	/** The identity id ("storage key", VSID) — the user entitlement id. */
	id: string;
	/** The Graph subject descriptor. */
	descriptor: string;
	displayName: string | null;
	/** Whether they hold any group membership in the organization. */
	active: boolean;
}

/** What one connection read says: the organization's stable id and who the token is. */
export interface AzureConnection {
	orgId: string;
	user: AzureUser;
}

/**
 * The organization's stable id and the token's owner — one read of
 * `GET https://dev.azure.com/{org}/_apis/connectionData`.
 *
 * Not in the REST reference: it is the read Microsoft's own client makes on
 * connect (azure-devops-node-api `WebApi.connect()` → `LocationsApi.getConnectionData`),
 * typed as `ConnectionData { authenticatedUser: Identity, instanceId, … }` in
 * https://github.com/microsoft/azure-devops-node-api/blob/master/api/interfaces/LocationsInterfaces.ts
 * (`Identity.id` is the storage key/VSID, `subjectDescriptor` the Graph
 * descriptor, `isActive` "has a membership in any Azure DevOps group in the
 * organization" — IdentitiesInterfaces.ts). The documented alternative, the
 * Profile API, needs a Microsoft Entra token. Spec R5b records the deviation.
 */
export async function readAzureConnection(org: string, token: string, fetchImpl: typeof fetch = fetch): Promise<AzureConnection> {
	const name = orgName(org);
	checkToken(token);
	const { body } = await azureGet(`https://dev.azure.com/${name}/_apis/connectionData`, token, fetchImpl);
	const user = (body.authenticatedUser ?? {}) as Record<string, unknown>;
	const orgId = nonEmpty(body.instanceId);
	const id = nonEmpty(user.id);
	const descriptor = nonEmpty(user.subjectDescriptor);
	if (!orgId || !id || !descriptor) {
		throw new AdminError(502, `Azure DevOps did not say which organization ${name} is or whose token this is`);
	}
	return {
		orgId,
		user: { id, descriptor, displayName: nonEmpty(user.providerDisplayName), active: user.isActive === true }
	};
}

/** The Graph descriptor of the organization's Project Collection Administrators.
 *  https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/groups/list?view=azure-devops-rest-7.1
 *  — paged by the `X-MS-ContinuationToken` response header. The group is named
 *  `[{org}]\\Project Collection Administrators`, and — the check that matters —
 *  sits in the COLLECTION's scope: its `domain` is
 *  `vstfs:///Framework/IdentityDomain/{the organization's id}` ("for VSTS groups
 *  the ScopeId"), where a project group's is `vstfs:///Classification/TeamProject/…`.
 *  A project named like the organization could hold a group with the same
 *  principal name (#308 R5b review), so exactly one collection-scope match is
 *  required. Throws otherwise: every organization has one, so not seeing it — or
 *  seeing two — is a read we cannot trust. */
async function adminGroup(name: string, orgId: string, token: string, fetchImpl: typeof fetch): Promise<string> {
	const wanted = `[${name}]\\project collection administrators`;
	const scope = `vstfs:///framework/identitydomain/${orgId.toLowerCase()}`;
	const found: string[] = [];
	let continuation: string | null = null;
	for (let page = 0; page < MAX_GROUP_PAGES; page++) {
		const q: string = continuation ? `continuationToken=${encodeURIComponent(continuation)}&` : '';
		const { body, headers } = await azureGet(
			`https://vssps.dev.azure.com/${name}/_apis/graph/groups?${q}api-version=7.1-preview.1`,
			token,
			fetchImpl
		);
		if (!Array.isArray(body.value)) throw new AdminError(502, 'Azure DevOps returned groups that are not a list');
		for (const g of body.value as Record<string, unknown>[]) {
			if ((nonEmpty(g.principalName) ?? '').toLowerCase() !== wanted) continue;
			if ((nonEmpty(g.domain) ?? '').toLowerCase() !== scope) continue;
			const d = nonEmpty(g.descriptor);
			if (!d || !DESCRIPTOR.test(d)) throw new AdminError(502, 'Azure DevOps returned an admin group this cannot address');
			found.push(d);
		}
		const next = headers.get('x-ms-continuationtoken');
		if (!next || next === continuation) break;
		continuation = next;
	}
	if (found.length !== 1) {
		throw new AdminError(502, `could not find exactly one Project Collection Administrators group for ${name}`);
	}
	return found[0];
}

/** The groups one subject is directly in.
 *  https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/memberships/list?view=azure-devops-rest-7.1
 *  (direction=up; depth 1 is the only depth Azure supports). */
async function containersOf(name: string, descriptor: string, token: string, fetchImpl: typeof fetch): Promise<string[]> {
	if (!DESCRIPTOR.test(descriptor)) throw new AdminError(502, 'Azure DevOps returned a descriptor this cannot address');
	const { body } = await azureGet(
		`https://vssps.dev.azure.com/${name}/_apis/graph/Memberships/${encodeURIComponent(descriptor)}?direction=up&api-version=7.1-preview.1`,
		token,
		fetchImpl
	);
	if (!Array.isArray(body.value)) throw new AdminError(502, 'Azure DevOps returned memberships that are not a list');
	const out: string[] = [];
	for (const m of body.value as Record<string, unknown>[]) {
		const c = nonEmpty(m.containerDescriptor);
		if (!c) continue;
		if (!DESCRIPTOR.test(c)) throw new AdminError(502, 'Azure DevOps returned a descriptor this cannot address');
		out.push(c);
	}
	return out;
}

/**
 * Whether the token's owner is in the organization's Project Collection
 * Administrators — directly, or through groups that are in it (walked up through
 * the Graph API). True only for the organization we know by its STABLE id: a name
 * that now belongs to another organization proves nothing about ours.
 */
async function proveAzureAdmin(name: string, token: string, unit: ForgeUnitRef, fetchImpl: typeof fetch): Promise<boolean> {
	const conn = await readAzureConnection(name, token, fetchImpl);
	if (conn.orgId !== unit.id || !conn.user.active) return false;
	const admins = await adminGroup(name, conn.orgId, token, fetchImpl);

	const seen = new Set<string>([conn.user.descriptor]);
	const queue = [conn.user.descriptor];
	let lookups = 0;
	while (queue.length) {
		if (++lookups > MAX_MEMBERSHIP_LOOKUPS) {
			throw new AdminError(502, `could not finish reading ${name}'s group memberships; not guessing who administers it`);
		}
		const containers = await containersOf(name, queue.shift() as string, token, fetchImpl);
		if (containers.includes(admins)) return true;
		for (const c of containers) {
			if (!seen.has(c)) {
				seen.add(c);
				queue.push(c);
			}
		}
	}
	return false;
}

/**
 * The next page's continuation token, or null on the last page. 7.1 documents a
 * string (PagedGraphMemberList.continuationToken —
 * https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/members/get?view=azure-devops-rest-7.1);
 * earlier versions send a string array, so a one-string array is read too and an
 * empty one (or null) ends the read. Anything else THROWS (#308 R5b review):
 * reading a token we do not understand as "no more pages" would call page one
 * complete and end everyone after it.
 */
function continuationOf(v: unknown): string | null {
	if (v === null || v === undefined) return null;
	if (typeof v === 'string') return v.trim() || null;
	if (Array.isArray(v)) {
		if (v.length === 0) return null;
		if (v.length === 1 && typeof v[0] === 'string') return v[0].trim() || null;
	}
	throw new AdminError(502, 'Azure DevOps returned a continuation token this cannot read; not reading a partial member list');
}

/** Statuses that are not membership: removed, or access switched off. */
const NOT_A_MEMBER = new Set(['deleted', 'disabled', 'pendingdisabled']);

/**
 * Every member of the organization — its user entitlements, page by page.
 * https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1
 * The reference names the page `items` and `members` — both are read. A page
 * carries `continuationToken`, absent on the last one. A member is keyed by
 * their Microsoft Entra OBJECT ID — `user.originId` when `user.origin` is `aad`
 * (GraphUser: "the unique identifier from the system of origin") — the id a
 * Microsoft sign-in (R5c) also records, so identities never need re-keying
 * (D43: never by email). An Entra entry without one is skipped and the read
 * marked incomplete; a failed page, an unreadable page, a repeated token or more
 * than MAX_ENTITLEMENT_PAGES pages THROW.
 */
async function listAzureMembers(name: string, token: string, fetchImpl: typeof fetch): Promise<ForgeMembersRead> {
	const members: ForgeMember[] = [];
	let complete = true;
	let continuation: string | null = null;
	const tokens = new Set<string>();
	for (let page = 0; page < MAX_ENTITLEMENT_PAGES; page++) {
		const q: string = continuation ? `continuationToken=${encodeURIComponent(continuation)}&` : '';
		const { body } = await azureGet(`https://vsaex.dev.azure.com/${name}/_apis/userentitlements?${q}api-version=7.1`, token, fetchImpl);
		const list = Array.isArray(body.members) ? body.members : Array.isArray(body.items) ? body.items : null;
		if (!list) throw new AdminError(502, 'Azure DevOps returned user entitlements that are not a list');
		for (const entry of list as Record<string, unknown>[]) {
			const user = (entry?.user ?? {}) as Record<string, unknown>;
			const access = (entry?.accessLevel ?? {}) as Record<string, unknown>;
			if (NOT_A_MEMBER.has((nonEmpty(access.status) ?? '').toLowerCase())) continue;
			const origin = (nonEmpty(user.origin) ?? '').toLowerCase();
			const login = nonEmpty(user.principalName) ?? nonEmpty(user.mailAddress);
			if (!origin || !login) {
				complete = false;
				continue;
			}
			// Only a Microsoft Entra user carries the stable id a dōjō identity is
			// keyed on (D43/D45). Anyone else (a GitHub-backed Azure DevOps user)
			// can be matched by no identity either way: not listed, and not a gap.
			if (origin !== 'aad') continue;
			const id = entraObjectId(user.originId);
			if (!id) {
				complete = false;
				continue;
			}
			members.push({ id, login });
		}
		const next = continuationOf(body.continuationToken);
		if (!next) return { members, complete };
		if (tokens.has(next)) throw new AdminError(502, 'Azure DevOps repeated a continuation token; not reading a partial member list');
		tokens.add(next);
		continuation = next;
	}
	throw new AdminError(502, `Azure DevOps reports more than ${MAX_ENTITLEMENT_PAGES} pages of members; not reading a partial list`);
}

/** One Azure DevOps project — a workspace in its organization's dōjō (D36). */
export interface AzureProject {
	/** The project's stable id — the mapping key. */
	id: string;
	name: string;
}

export interface AzureProjectsRead {
	projects: AzureProject[];
	complete: boolean;
}

/**
 * Every project in the organization the token can see.
 * https://learn.microsoft.com/en-us/rest/api/azure/devops/core/projects/list?view=azure-devops-rest-7.1
 * (`$top`/`$skip`; `value: TeamProjectReference[]` with `id` and `name`).
 */
async function listAzureProjects(name: string, token: string, fetchImpl: typeof fetch): Promise<AzureProjectsRead> {
	const projects: AzureProject[] = [];
	let complete = true;
	for (let page = 0; page < MAX_PROJECT_PAGES; page++) {
		const { body } = await azureGet(
			`https://dev.azure.com/${name}/_apis/projects?$top=${PROJECT_PAGE}&$skip=${page * PROJECT_PAGE}&api-version=7.1`,
			token,
			fetchImpl
		);
		if (!Array.isArray(body.value)) throw new AdminError(502, 'Azure DevOps returned projects that are not a list');
		for (const p of body.value as Record<string, unknown>[]) {
			const id = nonEmpty(p?.id);
			const pname = nonEmpty(p?.name);
			if (!id || !pname) {
				complete = false;
				continue;
			}
			projects.push({ id, name: pname });
		}
		if (body.value.length < PROJECT_PAGE) return { projects, complete };
	}
	throw new AdminError(502, `Azure DevOps reports more than ${MAX_PROJECT_PAGES * PROJECT_PAGE} projects; not reading a partial list`);
}

/** The Azure adapter: the forge interface, plus the organization's projects. */
export interface AzureAdapter extends ForgeAdapter {
	provider: 'azure_devops';
	/** The organization this adapter reads. */
	organization: string;
	/** The organization's stable id and the token's owner, read now. */
	connection(token: string): Promise<AzureConnection>;
	/** The token's owner by their Microsoft Entra object id (D45). */
	owner(token: string): Promise<AzureOwner>;
	listProjects(token: string, unit: ForgeUnitRef): Promise<AzureProjectsRead>;
}

/** Azure DevOps for one organization. `fetchImpl` is injected for tests. */
export function azureAdapter(organization: string, fetchImpl: typeof fetch = fetch): AzureAdapter {
	const name = orgName(organization);
	const prove = (token: string, unit: ForgeUnitRef) => {
		checkToken(token);
		return proveAzureAdmin(name, token, unit, fetchImpl);
	};
	return {
		provider: 'azure_devops',
		organization: name,
		connection: (token: string) => readAzureConnection(name, token, fetchImpl),
		owner: (token: string) => {
			checkToken(token);
			return readAzureOwner(name, token, fetchImpl);
		},
		/** The organization, when the token's owner is active in it — with their
		 *  standing proven now. Empty (a positive answer) when they are not. */
		listUnits: async (token: string): Promise<ForgeUnitsRead> => {
			checkToken(token);
			const conn = await readAzureConnection(name, token, fetchImpl);
			if (!conn.user.active) return { units: [], complete: true };
			const unit: ForgeUnit = { id: conn.orgId, login: name, role: 'member' };
			if (await prove(token, unit)) unit.role = 'admin';
			return { units: [unit], complete: true };
		},
		proveAdmin: prove,
		listMembers: (token: string) => {
			checkToken(token);
			return listAzureMembers(name, token, fetchImpl);
		},
		listProjects: (token: string) => {
			checkToken(token);
			return listAzureProjects(name, token, fetchImpl);
		}
	};
}
