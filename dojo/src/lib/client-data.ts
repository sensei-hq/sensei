// The dōjō console's API client + wire types for the `/v1` reads and writes the
// org and personal screens call: the knowledge library, workspaces,
// contributions, my repositories and their elections, rule packs, provisioning,
// invites and a workspace's constitution. The shared HTTP primitives (base URL,
// bearer header, tenant encoding, error, JSON parse) are reused from
// `dojo-api.ts` — the same core the triage + admin clients use, per DRY.
//
// Kept in a plain module (base URL read once, fetch injectable) so it imports
// cleanly under SSR/prerender and unit tests without a live backend.

import { DojoApiError, authHeaders, dojoApiUrl, encodeTenant, parseJson } from './dojo-api';
import type { RelayConstitution } from './constitution-map';
import type { ContributionRow, DownstreamRow } from './contributions-map';
import type { DojoCallOpts } from './dojo-api';
import type { AzureImportResult } from './azure-view';

// Re-export the shared error under the client name so screens
// `instanceof` against a local symbol without reaching into the shared module.
export { DojoApiError, dojoApiUrl };
export const ClientApiError = DojoApiError;

// ── client ───────────────────────────────────────────────────────────────────

/** Build the `/v1/t/{tenant}/…` URL for a tenant sub-path, tenant-encoded. */
function clientUrl(tenantKey: string, path: string): string {
	return `${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}${path}`;
}

/** User-wide (non-tenant-scoped) `/v1/you` path — the caller's own cross-dōjō
 *  reads, authorized by the JWT alone (no `{origin}/{org}` segment). Mirrors the
 *  `/you` personal-zone UI namespace (vs `clientUrl`'s tenant `/v1/t`). */
function youUrl(path: string): string {
	return `${dojoApiUrl}/v1/you${path}`;
}

/** A GET returning a JSON envelope, with the bearer header. */
async function getJson<T>(url: string, opts: DojoCallOpts): Promise<T> {
	const f = opts.fetch ?? fetch;
	const res = await f(url, { headers: authHeaders(opts.accessToken) });
	return parseJson<T>(res);
}

/** A body-carrying request (POST/PATCH/DELETE) with the bearer + JSON headers. */
async function sendJson<T>(
	url: string,
	method: 'POST' | 'PATCH' | 'DELETE',
	body: unknown,
	opts: DojoCallOpts
): Promise<T> {
	const f = opts.fetch ?? fetch;
	const res = await f(url, {
		method,
		headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	return parseJson<T>(res);
}

/** One published artifact row in the Knowledge library (mirrors the server
 *  `knowledge-data.ts` KnowledgeArtifact). `scope` is `dojo.artifacts.scope` jsonb. */
export interface KnowledgeArtifactWire {
	id: string;
	kind: string;
	title: string;
	scope: unknown;
	adopted_count: number;
	created_at: string;
}

/** `GET …/knowledge` — the tenant's published library, partitioned. */
export interface KnowledgeLibraryWire {
	retention_days: number | null;
	active: KnowledgeArtifactWire[];
	pending: KnowledgeArtifactWire[];
	catalog: KnowledgeArtifactWire[];
}

/** `GET …/knowledge` — the maintainer library over `dojo.artifacts` (read-only). */
export async function getKnowledge(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<KnowledgeLibraryWire> {
	const data = await getJson<Partial<KnowledgeLibraryWire>>(clientUrl(tenantKey, '/knowledge'), opts);
	return {
		retention_days: data.retention_days ?? null,
		active: data.active ?? [],
		pending: data.pending ?? [],
		catalog: data.catalog ?? []
	};
}

/** One workspace row (mirrors the server `WorkspaceRow`): a shared workspace in one
 *  dōjō (#299). `slug` is unique within that dōjō; `id` is the drill-in key. */
export interface WorkspaceRow {
	id: string;
	/** The dōjō the workspace lives in. */
	tenant_id: string;
	slug: string;
	name: string;
	classification: string;
	phase: string;
	last_run_at: string | null;
	runs_week: number;
}

/** `GET …/workspaces` — the org's workspaces (dojo.workspaces, tenant-scoped). */
export async function listWorkspaces(tenantKey: string, opts: DojoCallOpts = {}): Promise<WorkspaceRow[]> {
	const data = await getJson<{ workspaces?: WorkspaceRow[] }>(clientUrl(tenantKey, '/workspaces'), opts);
	return data.workspaces ?? [];
}

/** `GET /v1/you/workspaces` — every workspace the caller may read, across every
 *  dōjō they belong to. Backs the personal `/you/workspaces` list. Not
 *  tenant-scoped — the server reads with the caller's JWT and RLS decides (#299). Honest-empty on a
 *  missing key; a non-2xx throws `ClientApiError` (never []-as-success). */
export async function listUserWorkspaces(opts: DojoCallOpts = {}): Promise<WorkspaceRow[]> {
	const data = await getJson<{ workspaces?: WorkspaceRow[] }>(youUrl('/workspaces'), opts);
	return data.workspaces ?? [];
}

/** `GET /v1/you/contributions` — the caller's contributions across every dōjō:
 *  `mine` (artifacts they contributed) + `downstream` (approved for them). Both
 *  honest-empty on a missing key (never []-as-success masking a failure — a
 *  non-2xx throws `ClientApiError`). */
export async function listContributions(
	opts: DojoCallOpts = {}
): Promise<{ mine: ContributionRow[]; downstream: DownstreamRow[] }> {
	const data = await getJson<{ mine?: ContributionRow[]; downstream?: DownstreamRow[] }>(
		youUrl('/contributions'),
		opts
	);
	return { mine: data.mine ?? [], downstream: data.downstream ?? [] };
}

/** `POST /v1/you/contributions/adopt` — Pin (adopt) an approved-for-you artifact
 *  by its id. Throws `ClientApiError` on a non-2xx. */
export async function adoptContribution(artifactId: string, opts: DojoCallOpts = {}): Promise<void> {
	await sendJson(youUrl('/contributions/adopt'), 'POST', { artifactId }, opts);
}


/** One repository as it crosses the /v1/you wire, with the sharing verdict.
 *  Mirrors `MyRepository` in `$lib/server/repositories`; declared here so the
 *  loader and the screen never import a `$lib/server/*` module. */
export interface MyRepoWire {
	repository_id: string;
	repo_key: string;
	name: string | null;
	tenant: string;
	owning_org: string | null;
	forge_visibility: 'public' | 'private' | null;
	authority: 'user' | 'organization' | null;
	may_share: boolean;
	elected: boolean;
	sync_enabled: boolean;
	configurable_by_me: boolean;
	reason_code: string | null;
	reason: string | null;
	remedy: string | null;
	reason_actor: string | null;
	last_synced_at: string | null;
	metric_rows: number | null;
}

/** `GET /v1/you/repositories` — every repository the caller can see, with the
 *  sharing verdict and its remedy. A non-2xx throws `ClientApiError`; the caller
 *  renders an ERROR state rather than an empty list, because "no repositories"
 *  and "could not ask" are different answers. */
export async function listMyRepos(opts: DojoCallOpts = {}): Promise<MyRepoWire[]> {
	const data = await getJson<{ repositories?: MyRepoWire[] }>(youUrl('/repositories'), opts);
	return data.repositories ?? [];
}

/** `PATCH /v1/you/repositories/election` — turn sharing on or off for ONE
 *  repository. Returns the verdict re-read from the view, so a caller learns
 *  that (say) entitlement still refuses rather than assuming success. */
export async function setRepoElection(
	repoKey: string,
	elected: boolean,
	opts: DojoCallOpts = {}
): Promise<{ repo_key: string; elected: boolean; sync_enabled: boolean; reason_code: string | null }> {
	return sendJson(
		youUrl('/repositories/election'),
		'PATCH',
		{ repo_key: repoKey, elected },
		opts
	) as Promise<{
		repo_key: string;
		elected: boolean;
		sync_enabled: boolean;
		reason_code: string | null;
	}>;
}

/** One global library pack as it crosses the /v1/you wire (browse). The wire
 *  boundary lives here (client-reachable) so the map + loader never import a
 *  `$lib/server/*` module. Produced by `rulepacks-data.shapeLibraryPacks`. */
export interface LibraryPackWire {
	slug: string;
	kanji: string | null;
	name: string;
	by: string;
	note: string;
	rules: string[];
}

/** `GET /v1/you/rule-packs` — the global rule-pack library (curated, adoptable) +
 *  the slugs the caller has adopted. Honest-empty on a missing key (a non-2xx
 *  throws `ClientApiError`). */
export async function listLibraryPacks(
	opts: DojoCallOpts = {}
): Promise<{ packs: LibraryPackWire[]; adopted: string[] }> {
	const data = await getJson<{ packs?: LibraryPackWire[]; adopted?: string[] }>(
		youUrl('/rule-packs'),
		opts
	);
	return { packs: data.packs ?? [], adopted: data.adopted ?? [] };
}

/** `POST /v1/you/rule-packs/{slug}/adopt` — adopt (or drop) a library pack for the
 *  caller's user-scoped namespace. Throws `ClientApiError` on a non-2xx (e.g. 404
 *  unknown slug). */
export async function setPackAdoption(
	slug: string,
	adopt: boolean,
	opts: DojoCallOpts = {}
): Promise<void> {
	await sendJson(youUrl(`/rule-packs/${encodeTenant(slug)}/adopt`), 'POST', { adopt }, opts);
}

/** One tenant a provisioning pass established the caller's place in. */
export interface ProvisionedTenant {
	id: string;
	key: string;
	origin: 'personal' | 'organization';
	role: string;
	/** True when this pass CREATED the tenant, false when it already existed. */
	created: boolean;
}

/** What `POST /v1/you/provision` returns — the daemon mirrors this too. */
export interface ProvisionResult {
	synced: boolean;
	/** Present when `synced` is false. Each case calls for different advice,
	 *  which is why they are not collapsed into one "nothing happened":
	 *  connect GitHub, wait, or sign in again. */
	reason?: 'no_forge_token' | 'forge_unreachable' | 'forge_token_rejected' | 'no_identity';
	personal: ProvisionedTenant | null;
	tenants: ProvisionedTenant[];
}

/** `POST /v1/you/provision` — establish the caller's personal dōjō and a tenant
 *  for each org their forge token proves, idempotently. Supersedes the old
 *  `/github/sync` join-only call, which could never create the tenant the first
 *  user in an org needed to join. `/github/sync` still routes here. */
export async function provisionFromForge(opts: DojoCallOpts = {}): Promise<ProvisionResult> {
	return sendJson(youUrl('/provision'), 'POST', {}, opts);
}

/** `POST /v1/you/integrations/azure_devops/connect` — the personal dōjō's import
 *  of an Azure DevOps organization as its own dōjō (#308 R5b). The one request
 *  that carries the PAT; the server validates it before storing it, and never
 *  sends it back. */
export async function importAzureAsOwnDojo(
	organizationUrl: string,
	token: string,
	opts: DojoCallOpts = {}
): Promise<AzureImportResult> {
	return sendJson(youUrl('/integrations/azure_devops/connect'), 'POST', { organization_url: organizationUrl, token }, opts);
}

/** `POST /v1/t/{tenant}/invites` — an admin issues a magic-link invite (F3b).
 *  Returns the invite incl. the single-use `token` for the accept link. Throws
 *  `ClientApiError` on a non-2xx (403 below admin, 400 invalid). */
export async function createInvite(
	tenantKey: string,
	input: { email: string; role?: string; kind: string; access?: 'member' | 'viewer' },
	opts: DojoCallOpts = {}
): Promise<{ id: string; token: string; email: string; role: string; expires_at: string }> {
	return sendJson(clientUrl(tenantKey, '/invites'), 'POST', input, opts);
}

/** `POST /v1/you/invites/accept` — the invitee redeems an invite by its token
 *  (F3b). Returns `{ tenant_id, role, access }` on success — a viewer invite
 *  (#307 D8) has `role: null` and `access: 'viewer'`. Throws `ClientApiError` on a
 *  gate rejection (403 email-mismatch, 410 expired, 409 used, 404 unknown). */
export async function acceptInvite(
	token: string,
	opts: DojoCallOpts = {}
): Promise<{ tenant_id: string; role: string | null; access: 'member' | 'viewer' }> {
	return sendJson(youUrl('/invites/accept'), 'POST', { token }, opts);
}

/** `GET /v1/you/workspaces/{id}/constitution` — the daemon-resolved constitution
 *  for one workspace (F4 drill-in), by workspace id: a slug is unique only within one
 *  dōjō (#299). `null` when none is federated yet or the caller cannot read the
 *  workspace (honest-empty). */
export async function getWorkspaceConstitution(
	workspaceId: string,
	opts: DojoCallOpts = {}
): Promise<RelayConstitution | null> {
	const data = await getJson<{ constitution?: RelayConstitution | null }>(
		youUrl(`/workspaces/${encodeURIComponent(workspaceId)}/constitution`),
		opts
	);
	return data.constitution ?? null;
}
