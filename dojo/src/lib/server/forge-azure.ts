// Azure DevOps behind the one forge interface (#308 R5b, D35/D36).
//
// A unit is an Azure DevOps ORGANIZATION (its own dōjō); its PROJECTS become
// workspaces inside it. Every read is made with the personal access token an
// organization admin pasted (D35), sent as HTTP Basic auth — never in a URL,
// never logged, never in an error message.
//
// ONE ORGANIZATION PER ADAPTER. An organization-scoped PAT cannot enumerate the
// organizations its owner belongs to: the Profile and Accounts APIs need a
// Microsoft Entra token ("Some APIs, including the Organizations, Profiles, and
// PAT Lifecycle Management APIs, require Microsoft Entra tokens" —
// https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/use-personal-access-tokens-to-authenticate#can-i-use-pats-with-all-azure-devops-rest-apis),
// and global ("all accessible organizations") PATs stop working on 1 Dec 2026.
// So the admin names the organization (its URL) and the adapter is bound to it;
// `listUnits` answers whether the token's owner is in THAT organization.
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
const AZURE_HOSTS = new Set(['dev.azure.com', 'vssps.dev.azure.com', 'vsaex.dev.azure.com']);

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

/** The read-only scopes the PAT needs — one list, the screen's (azure-view.ts). */
export { AZURE_REQUIRED_SCOPES } from '../azure-view';

/**
 * The organization an Azure DevOps URL names — `https://dev.azure.com/{org}…`
 * or `https://{org}.visualstudio.com…` — lowercased, or a 400. Only https, the
 * exact hosts, no credentials and no port: the name goes into API URLs.
 */
export function azureOrgFromUrl(input: string): string {
	const raw = (typeof input === 'string' ? input : '').trim();
	const bad = () => new AdminError(400, 'that is not an Azure DevOps organization URL (https://dev.azure.com/your-organization)');
	if (!raw) throw bad();
	let url: URL;
	try {
		url = new URL(/^[a-z][a-z0-9+.-]*:\/\//i.test(raw) ? raw : `https://${raw}`);
	} catch {
		throw bad();
	}
	if (url.protocol !== 'https:' || url.username || url.password || url.port) throw bad();
	const host = url.hostname.toLowerCase();
	let org: string | undefined;
	if (host === 'dev.azure.com') {
		org = url.pathname.split('/').filter(Boolean)[0];
	} else if (host.endsWith('.visualstudio.com')) {
		const label = host.slice(0, -'.visualstudio.com'.length);
		if (!label.includes('.') && label !== 'vs-ssh' && label !== 'app') org = label;
	}
	const name = (org ?? '').toLowerCase();
	if (!ORG_NAME.test(name)) throw bad();
	return name;
}

/** The org name, checked, or a 400 — before it is put in any URL. */
function orgName(org: string): string {
	const name = (typeof org === 'string' ? org : '').trim().toLowerCase();
	if (!ORG_NAME.test(name)) throw new AdminError(400, 'that is not an Azure DevOps organization name');
	return name;
}

/** A PAT is printable ASCII with no whitespace; anything else is refused before
 *  it reaches a header. Its value never appears in the message. */
function checkToken(token: string): void {
	if (typeof token !== 'string' || !/^[\x21-\x7e]{1,512}$/.test(token)) {
		throw new AdminError(400, 'that is not an Azure DevOps personal access token');
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
 * One GET against an Azure DevOps API host, with the PAT as Basic auth. Throws
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
		headers: { Authorization: `Basic ${btoa(':' + token)}`, Accept: 'application/json', 'User-Agent': 'sensei-dojo' },
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
 *  — paged by the `X-MS-ContinuationToken` response header; the group is named
 *  `[{org}]\Project Collection Administrators`. Throws when it is not found: every
 *  organization has one, so not seeing it is a read we could not make. */
async function adminGroup(name: string, token: string, fetchImpl: typeof fetch): Promise<string> {
	const wanted = `[${name}]\\project collection administrators`;
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
			if ((nonEmpty(g.principalName) ?? '').toLowerCase() === wanted) {
				const d = nonEmpty(g.descriptor);
				if (d && DESCRIPTOR.test(d)) return d;
			}
		}
		const next = headers.get('x-ms-continuationtoken');
		if (!next || next === continuation) break;
		continuation = next;
	}
	throw new AdminError(502, `could not find ${name}'s Project Collection Administrators group`);
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
	const admins = await adminGroup(name, token, fetchImpl);

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

/** Statuses that are not membership: removed, or access switched off. */
const NOT_A_MEMBER = new Set(['deleted', 'disabled', 'pendingdisabled']);

/**
 * Every member of the organization — its user entitlements, page by page.
 * https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/search-user-entitlements?view=azure-devops-rest-7.1
 * The reference names the page `items`; the service answers `members` — both are
 * read. A page carries `continuationToken`, null on the last one. A member is
 * keyed by `id` ("matches the Id of the Identity" — the token owner's
 * connectionData id). An entry with no id is skipped and the read marked
 * incomplete; a failed page, an unreadable page, a repeated token or more than
 * MAX_ENTITLEMENT_PAGES pages THROW.
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
			const id = nonEmpty(entry?.id);
			const login = nonEmpty(user.principalName) ?? nonEmpty(user.mailAddress);
			if (!id || !login) {
				complete = false;
				continue;
			}
			members.push({ id, login });
		}
		const next = nonEmpty(body.continuationToken);
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
