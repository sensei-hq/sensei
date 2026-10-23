// Admin-console API client + types (R10) over the R7 dojo-mind admin backend.
//
// Binds to the real admin endpoints (crates/dojo-mind/src/api.rs :495-955), all
// behind the ADMIN role-floor (`resolve_admin` → 403 for a non-admin):
//   GET    …/members                       → { members: Membership[] }
//   PATCH  …/members/{user_id}/role        → { user_id, role }
//   GET    …/identities                    → { identities: Identity[] }
//   GET    …/policies                      → { policies: Policy[] }
//   POST   …/policies                      → { id, scope_key }
//   PATCH  …/policies/{id}                 → { id }
//   DELETE …/policies/{id}                 → { deleted: true }
//   GET    …/health                        → HealthRollup
//   GET    …/audit?limit=N                 → { events: AuditEvent[] }
//
// Row/response types are DERIVED from the Rust store shapes (store.rs
// list_memberships/list_identities/list_policies/health_rollup/list_audit_events)
// and the api.rs request bodies — not invented. The shared HTTP primitives (base
// URL, bearer header, tenant encoding, error, JSON parse) are reused from
// `dojo-api.ts` — the same core the triage client uses, per DRY.
//
// Kept in a plain module (base URL read once, fetch injectable) so it imports
// cleanly under SSR/prerender and unit tests without a live backend; the screens
// render off an injected/empty fixture until Dev wires a real admin session.

import { DojoApiError, authHeaders, dojoApiUrl, encodeTenant, parseJson } from './dojo-api';
import type { DojoCallOpts } from './dojo-api';
import type { DojoViewerWire } from './viewers-view';
import type { TeamWire } from './teams-view';
import type { BillableUserWire, BillingAccountWire, SeatUsageWire } from './billing-view';
import type { ForgeProvider } from './forge-providers';
import type { IntegrationUnitsView, MapTarget } from './integrations-view';
import type { AzureStatusView, AzureSyncResult } from './azure-view';

// Re-export the shared error under the admin name so admin screens `instanceof`
// against a local symbol without reaching into the shared module directly.
export { DojoApiError, dojoApiUrl };
export const AdminApiError = DojoApiError;

// ── wire types (mirror the Rust store row shapes) ────────────────────────────

/** A Dōjō role — `dojo.member_role` (api.rs `is_member_role`). */
export type MemberRole = 'contributor' | 'maintainer' | 'lead' | 'admin';

/** One membership row — `store.list_memberships` (store.rs:658). */
export interface Membership {
	id: string;
	user_id: string;
	role: string;
	/** membership kind (`human`, `service`, …). */
	kind: string;
	/** how the member authenticated (`sso | github_oauth | device_code | …`). */
	authenticated_via: string;
	sync_status: string;
	attribution_default: string;
	/** ISO-8601, or null when never seen. */
	last_heartbeat_at: string | null;
	/** ISO-8601, or null when active. */
	disabled_at: string | null;
	created_at: string;
	/** WS-1: best display name resolved from the member's `dojo.identities` row,
	 *  or null when they have no named identity (the mapper falls back to a shortId). */
	display_name: string | null;
	email: string | null;
}

/** An identity-provider method — `dojo.auth_method` (api.rs `is_auth_method`). */
export type AuthMethod = 'sso' | 'github_oauth' | 'device_code';

/** One identity mapping — `store.list_identities` (store.rs:792). Keys on
 *  `principal_id`: an identity belongs to a `dojo.principals` row, not to a
 *  Supabase login and not to a tenant (spec dojo-auth-provisioning §VIII.2). */
export interface Identity {
	id: string;
	principal_id: string;
	provider: string;
	subject: string;
	email: string | null;
	display_name: string | null;
	created_at: string;
	last_login_at: string | null;
}

/** An attribution mode — `dojo.attribution_mode` (api.rs `is_attribution_mode`). */
export type AttributionMode = 'named' | 'anonymous';

/** One policy row — `store.list_policies` (store.rs:904). */
export interface Policy {
	id: string;
	scope_key: string;
	attribution_default: string;
	/** free-form confidentiality rule JSON. */
	confidentiality: unknown;
	/** retention window in days, or null for indefinite. */
	retention_days: number | null;
	created_at: string;
	updated_at: string;
}

/** The health strip — `admin_health` (api.rs:917 / store.health_rollup). */
export interface HealthRollup {
	/** distinct members active in the last 5min. */
	connections: number;
	/** `count(*)` of queued triage rows. */
	queue_depth: number;
	/** approved+published+distributed events in the last hour. */
	publish_rate_1h: number;
	/** error events in the last hour. */
	error_rate_1h: number;
	/** Contributions-vs-approvals by week (last 4) for the bar chart — computed by
	 *  the health route over `dojo.audit_events`. Absent on older responses. */
	contrib_vs_approve?: HealthWeek[];
}

/** One week's contributions/approvals pair (mirrors the server `HealthWeek`;
 *  maps 1:1 onto the presentational `KitHealthWeek`). */
export interface HealthWeek {
	wk: string;
	c: number;
	a: number;
}

/** One audit event — `store.list_audit_events` (store.rs:1086). */
export interface AuditEvent {
	id: number;
	/** ISO-8601. */
	ts: string;
	actor_id: string | null;
	action: string;
	target: string | null;
	detail: unknown;
}

// ── request bodies (mirror the api.rs Deserialize structs) ───────────────────

/** `POST …/policies` body — `UpsertPolicy` (api.rs:774). Upserts on `scope_key`. */
export interface UpsertPolicyBody {
	scope_key: string;
	attribution_default?: string;
	confidentiality?: unknown;
	retention_days?: number;
}

/** `PATCH …/policies/{id}` body — `PatchPolicy` (api.rs:829). */
export interface PatchPolicyBody {
	attribution_default?: string;
	confidentiality?: unknown;
	retention_days?: number;
}

// ── client ───────────────────────────────────────────────────────────────────

/** Build the `/v1/t/{tenant}/…` URL for an admin sub-path, tenant-encoded. */
function adminUrl(tenantKey: string, path: string): string {
	return `${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}${path}`;
}

/** A GET returning a JSON envelope, with the bearer header. */
async function getJson<T>(url: string, opts: DojoCallOpts): Promise<T> {
	const f = opts.fetch ?? fetch;
	const res = await f(url, { headers: authHeaders(opts.accessToken) });
	return parseJson<T>(res);
}

/** A body-carrying request (POST/PUT/PATCH/DELETE) with the bearer + JSON headers. */
async function sendJson<T>(
	url: string,
	method: 'POST' | 'PUT' | 'PATCH' | 'DELETE',
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

// viewers (#307 D8) ────────────────────────────────────────────────────────────

const viewerPath = (principalId: string) => `/viewers/${encodeURIComponent(principalId)}`;
const grantPath = (principalId: string, workspaceId: string) =>
	`${viewerPath(principalId)}/workspaces/${encodeURIComponent(workspaceId)}`;

/** `GET …/viewers` — the dōjō's viewers, each with the workspaces granted to them (admin). */
export async function listViewers(tenantKey: string, opts: DojoCallOpts = {}): Promise<DojoViewerWire[]> {
	return (await getJson<{ viewers: DojoViewerWire[] }>(adminUrl(tenantKey, '/viewers'), opts)).viewers;
}

/** `DELETE …/viewers/{id}` — remove a viewer from the dōjō, and their workspace views. */
export async function removeViewer(tenantKey: string, principalId: string, opts: DojoCallOpts = {}): Promise<void> {
	await sendJson(adminUrl(tenantKey, viewerPath(principalId)), 'DELETE', undefined, opts);
}

/** `PUT …/viewers/{id}/workspaces/{workspace}` — grant the view of one of the dōjō's workspaces. */
export async function grantWorkspaceView(
	tenantKey: string,
	principalId: string,
	workspaceId: string,
	opts: DojoCallOpts = {}
): Promise<void> {
	await sendJson(adminUrl(tenantKey, grantPath(principalId, workspaceId)), 'PUT', undefined, opts);
}

/** `DELETE …/viewers/{id}/workspaces/{workspace}` — revoke it. */
export async function revokeWorkspaceView(
	tenantKey: string,
	principalId: string,
	workspaceId: string,
	opts: DojoCallOpts = {}
): Promise<void> {
	await sendJson(adminUrl(tenantKey, grantPath(principalId, workspaceId)), 'DELETE', undefined, opts);
}

// teams (#307 D7) ──────────────────────────────────────────────────────────────

const teamPath = (teamId: string) => `/teams/${encodeURIComponent(teamId)}`;

/** `GET …/teams` — the dōjō's teams with their members. */
export async function listTeams(tenantKey: string, opts: DojoCallOpts = {}): Promise<TeamWire[]> {
	return (await getJson<{ teams: TeamWire[] }>(adminUrl(tenantKey, '/teams'), opts)).teams;
}

export async function createTeam(tenantKey: string, name: string, opts: DojoCallOpts = {}): Promise<void> {
	await sendJson(adminUrl(tenantKey, '/teams'), 'POST', { name }, opts);
}

export async function renameTeam(tenantKey: string, teamId: string, name: string, opts: DojoCallOpts = {}): Promise<void> {
	await sendJson(adminUrl(tenantKey, teamPath(teamId)), 'PATCH', { name }, opts);
}

/** The workspace views a team granted were granted to people, and stay. */
export async function deleteTeam(tenantKey: string, teamId: string, opts: DojoCallOpts = {}): Promise<void> {
	await sendJson(adminUrl(tenantKey, teamPath(teamId)), 'DELETE', undefined, opts);
}

export async function addTeamMember(
	tenantKey: string,
	teamId: string,
	principalId: string,
	opts: DojoCallOpts = {}
): Promise<void> {
	await sendJson(adminUrl(tenantKey, `${teamPath(teamId)}/members/${encodeURIComponent(principalId)}`), 'PUT', undefined, opts);
}

export async function removeTeamMember(
	tenantKey: string,
	teamId: string,
	principalId: string,
	opts: DojoCallOpts = {}
): Promise<void> {
	await sendJson(
		adminUrl(tenantKey, `${teamPath(teamId)}/members/${encodeURIComponent(principalId)}`),
		'DELETE',
		undefined,
		opts
	);
}

/** Grant everyone now on the team the view of one of the dōjō's workspaces. */
export async function grantTeamWorkspaceView(
	tenantKey: string,
	teamId: string,
	workspaceId: string,
	opts: DojoCallOpts = {}
): Promise<void> {
	await sendJson(adminUrl(tenantKey, `${teamPath(teamId)}/workspaces/${encodeURIComponent(workspaceId)}`), 'PUT', undefined, opts);
}

// members ──────────────────────────────────────────────────────────────────────

/** `GET …/members` — the tenant's memberships (admin). */
export async function listMembers(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<Membership[]> {
	const data = await getJson<{ members?: Membership[] }>(adminUrl(tenantKey, '/members'), opts);
	return data.members ?? [];
}

/** `PATCH …/members/{user_id}/role` — override a member's role (admin). */
export async function setMemberRole(
	tenantKey: string,
	userId: string,
	role: string,
	opts: DojoCallOpts = {}
): Promise<{ user_id: string; role: string }> {
	return sendJson(
		adminUrl(tenantKey, `/members/${encodeURIComponent(userId)}/role`),
		'PATCH',
		{ role },
		opts
	);
}

// identities ────────────────────────────────────────────────────────────────────

/** `GET …/identities` — the tenant's identity mappings (admin). */
export async function listIdentities(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<Identity[]> {
	const data = await getJson<{ identities?: Identity[] }>(
		adminUrl(tenantKey, '/identities'),
		opts
	);
	return data.identities ?? [];
}

// policies ──────────────────────────────────────────────────────────────────────

/** `GET …/policies` — the tenant's policy grid (admin). */
export async function listPolicies(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<Policy[]> {
	const data = await getJson<{ policies?: Policy[] }>(adminUrl(tenantKey, '/policies'), opts);
	return data.policies ?? [];
}

/** `POST …/policies` — create or edit-by-scope a policy (admin, upsert on scope). */
export async function upsertPolicy(
	tenantKey: string,
	body: UpsertPolicyBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string; scope_key: string }> {
	return sendJson(adminUrl(tenantKey, '/policies'), 'POST', body, opts);
}

/** `PATCH …/policies/{id}` — update a policy by id (admin). */
export async function patchPolicy(
	tenantKey: string,
	id: string,
	body: PatchPolicyBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string }> {
	return sendJson(adminUrl(tenantKey, `/policies/${encodeURIComponent(id)}`), 'PATCH', body, opts);
}

/** `DELETE …/policies/{id}` — remove a policy (admin). */
export async function deletePolicy(
	tenantKey: string,
	id: string,
	opts: DojoCallOpts = {}
): Promise<{ deleted: boolean }> {
	return sendJson(
		adminUrl(tenantKey, `/policies/${encodeURIComponent(id)}`),
		'DELETE',
		undefined,
		opts
	);
}

// health + audit ────────────────────────────────────────────────────────────────

/** `GET …/health` — the admin health strip (read-only, no audit). */
export async function getHealth(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<HealthRollup> {
	return getJson<HealthRollup>(adminUrl(tenantKey, '/health'), opts);
}

// billing ────────────────────────────────────────────────────────────────────

export type {
	BillableUserWire as BillingBillableUser,
	SeatUsageWire as BillingUsage,
	BillingAccountWire as BillingAccount
} from './billing-view';

/** `GET …/billing` envelope. */
export interface BillingResponse {
	account: BillingAccountWire | null;
	usage: SeatUsageWire;
}

/** `GET …/billing` — the tenant's billing account + LIVE seat usage (admin). */
export async function getBilling(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<BillingResponse> {
	return getJson<BillingResponse>(adminUrl(tenantKey, '/billing'), opts);
}

// integrations (#308 R5a) ───────────────────────────────────────────────────────

const integrationsPath = (provider: ForgeProvider) => `/integrations/${encodeURIComponent(provider)}`;

/** `POST …/integrations/{provider}` — the units the person's connected GitHub
 *  sees. The token is never sent from here: the server reads it from the sealed
 *  cookie a "Connect GitHub" re-authorization set, and answers 428 without one. */
export async function listIntegrationUnits(
	tenantKey: string,
	provider: ForgeProvider,
	opts: DojoCallOpts = {}
): Promise<IntegrationUnitsView> {
	return sendJson<IntegrationUnitsView>(adminUrl(tenantKey, integrationsPath(provider)), 'POST', {}, opts);
}

/** `POST …/integrations/{provider}/claim` — claim a unit's dōjō. */
export async function claimIntegrationUnit(
	tenantKey: string,
	provider: ForgeProvider,
	unitId: string,
	opts: DojoCallOpts = {}
): Promise<{ dojo: { key: string } }> {
	return sendJson(adminUrl(tenantKey, `${integrationsPath(provider)}/claim`), 'POST', { unit_id: unitId }, opts);
}

/** `POST …/integrations/{provider}/map` — its own dōjō, or a workspace here. */
export async function mapIntegrationUnit(
	tenantKey: string,
	provider: ForgeProvider,
	unitId: string,
	target: MapTarget,
	opts: DojoCallOpts = {}
): Promise<{ tenantId: string; workspaceId: string | null }> {
	return sendJson(adminUrl(tenantKey, `${integrationsPath(provider)}/map`), 'POST', { unit_id: unitId, ...target }, opts);
}

// Azure DevOps (#308 R5b) ────────────────────────────────────────────────────────

const azurePath = '/integrations/azure_devops';

/** `GET …/integrations/azure_devops` — this dōjō's Azure DevOps organizations and
 *  their connection state (never a token), the read scopes the connect asks for,
 *  and whether this server can connect at all. */
export async function listAzureConnections(tenantKey: string, opts: DojoCallOpts = {}): Promise<AzureStatusView> {
	return getJson<AzureStatusView>(adminUrl(tenantKey, azurePath), opts);
}

/** `POST …/integrations/azure_devops/connect` — start connecting Azure DevOps
 *  through Microsoft Entra for this dōjō: the URL to send the person to. */
export async function startAzureConnect(tenantKey: string, opts: DojoCallOpts = {}): Promise<{ url: string }> {
	return sendJson(adminUrl(tenantKey, `${azurePath}/connect`), 'POST', {}, opts);
}

/** `POST …/integrations/azure_devops/sync` — re-read with the stored grant. */
export async function syncAzure(tenantKey: string, organizationId: string, opts: DojoCallOpts = {}): Promise<AzureSyncResult> {
	return sendJson(adminUrl(tenantKey, `${azurePath}/sync`), 'POST', { organization_id: organizationId }, opts);
}

/** `DELETE …/integrations/azure_devops/token` — remove the stored grant. */
export async function removeAzureToken(tenantKey: string, organizationId: string, opts: DojoCallOpts = {}): Promise<void> {
	await sendJson(adminUrl(tenantKey, `${azurePath}/token`), 'DELETE', { organization_id: organizationId }, opts);
}

/** `GET …/audit` — the admin audit log, most recent first (read-only). */
export async function listAudit(
	tenantKey: string,
	opts: DojoCallOpts & { limit?: number } = {}
): Promise<AuditEvent[]> {
	const path = opts.limit != null ? `/audit?limit=${encodeURIComponent(opts.limit)}` : '/audit';
	const data = await getJson<{ events?: AuditEvent[] }>(adminUrl(tenantKey, path), opts);
	return data.events ?? [];
}
