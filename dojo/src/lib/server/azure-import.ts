// Importing an Azure DevOps organization into the dōjō (#308 R5b, D36/D42).
//
// An organization admin connects Azure DevOps through Microsoft Entra
// (azure-connect.ts) and picks an organization. The grant is VALIDATED FIRST —
// its owner is active in the organization and a Project Collection
// Administrator, and the projects can be read — and only then is anything
// written. Then:
//   * the organization → a dōjō: from the PERSONAL dōjō, its own dōjō is made
//     (or joined, as a proven admin) — how an Azure organization's dōjō is first
//     created; from an ORGANIZATION dōjō's Integrations, it connects to that
//     dōjō (one organization known to GitHub and Azure). Claimed through
//     dojo.claim_forge_unit, provider azure_devops;
//   * the owner's Azure identity is recorded by their Microsoft Entra object id
//     (D43/D45) — what a Microsoft sign-in (R5c) will also see;
//   * the REFRESH token → Supabase Vault (dojo.set_forge_token); the connection
//     keeps only the secret's id and the token's state. Access tokens are used
//     for one request and never stored;
//   * each project → a workspace in that dōjō (dojo.map_forge_unit, keyed by the
//     project's stable id, named `{org}/{project}` — what
//     dojo.register_repository matches an Azure repository's project against);
//   * the members are read and #309's rule applied (followMembers).
//
// System work (the server's service-role client): it reads every tenant's
// connections and writes through the SECURITY DEFINER functions that hold the
// rules. No token is ever returned, logged or put in an error.
import { AdminError, slugify, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';
import { AzureGrantRejected, refreshGrant, type AzureEntraConfig } from './azure-entra';
import type { AzureAdapter, AzureConnection, AzureProject } from './forge-azure';
import { ForgeReadError } from './forge-github';
import type { AzureConnectionView, AzureImportResult, AzureMembersRead, AzureSyncResult } from '../azure-view';

export type { AzureConnectionView, AzureImportResult, AzureMembersRead, AzureSyncResult };
import { authMethodFor, claimTenantIfOwner, ensureForgeIdentity, ensureOrgTenant } from './provisioning';

const PROVIDER = 'azure_devops' as const;

/** Where the organization goes: its own dōjō (from the personal dōjō), or the
 *  organization dōjō whose Integrations screen asked. */
export type AzureImportTarget = { as: 'own' } | { as: 'this'; tenantId: string };

function fail(error: { code?: string; message: string } | null): void {
	if (error) throw new AdminError(refusalStatus(error), error.message);
}

/** A connected grant for one request: the access token to read with, and the
 *  refresh token to keep. */
export interface AzureGrantTokens {
	accessToken: string;
	refreshToken: string;
}

/**
 * Validate the grant against the organization — its owner is active in it and
 * is a Project Collection Administrator — and read its projects. Reads only:
 * nothing is written before this returns. A read Azure refuses throws its
 * ForgeReadError.
 */
async function validate(adapter: AzureAdapter, token: string) {
	const { conn, unit } = await proveGrant(adapter, token);
	const read = await adapter.listProjects(token, unit);
	return { conn, unit, read };
}

/**
 * Prove the grant still speaks for an admin of the organization: its owner is
 * active in it and a Project Collection Administrator — now. Every path that
 * writes or ENDS anything from a grant runs it first (import, Sync, the
 * scheduled member read): a demoted owner may still read user entitlements but
 * see only part of the organization, and a "complete" read from them would end
 * everyone they cannot see (#308 R5b review).
 */
export async function proveGrant(adapter: AzureAdapter, token: string) {
	const conn: AzureConnection = await adapter.connection(token);
	const org = adapter.organization;
	if (!conn.user.active) {
		throw new AdminError(403, `your Azure DevOps account is not a member of ${org}`);
	}
	const unit = { id: conn.orgId, login: org };
	if (!(await adapter.proveAdmin(token, unit))) {
		throw new AdminError(403, `only a Project Collection Administrator of ${org} can connect it`);
	}
	return { conn, unit };
}

/** A 403 unless the person is an ACTIVE admin of the dōjō. */
async function requireActiveAdmin(db: DojoClient, principalId: string, tenantId: string): Promise<void> {
	const { data, error } = await db
		.from('memberships')
		.select('role, disabled_at')
		.eq('tenant_id', tenantId)
		.eq('user_id', principalId)
		.maybeSingle();
	fail(error);
	const m = data as { role: string; disabled_at: string | null } | null;
	if (!m || m.disabled_at || m.role !== 'admin') {
		throw new AdminError(403, 'only an admin of this dōjō can connect an Azure DevOps organization to it');
	}
}

/** The organization dōjō whose screen asked, connected to the Azure
 *  organization (made now if not yet) — or a 409 when another dōjō holds it. */
async function connectHere(
	db: DojoClient,
	principalId: string,
	tenantId: string,
	unit: { id: string; login: string }
): Promise<{ id: string; key: string }> {
	const tenant = await db.from('tenants').select('id, key, origin').eq('id', tenantId).maybeSingle();
	fail(tenant.error);
	const row = tenant.data as { id: string; key: string; origin: string } | null;
	if (!row) throw new AdminError(404, 'no such dōjō');
	if (row.origin !== 'organization') {
		throw new AdminError(400, 'an Azure DevOps organization is imported into an organization’s dōjō — from your personal dōjō, import it as its own');
	}

	const existing = await db
		.from('tenant_connections')
		.select('tenant_id')
		.eq('provider', PROVIDER)
		.eq('external_id', unit.id)
		.maybeSingle();
	fail(existing.error);
	const holder = (existing.data as { tenant_id: string } | null)?.tenant_id;
	if (holder && holder !== tenantId) {
		throw new AdminError(409, `${unit.login} is already connected to another dōjō`);
	}
	if (!holder) {
		const { error } = await db
			.from('tenant_connections')
			.insert({
				tenant_id: tenantId,
				provider: PROVIDER,
				external_id: unit.id,
				external_slug: unit.login,
				connected_by: principalId,
				verified_at: new Date().toISOString()
			})
			.select('id')
			.single();
		if (error && (error as { code?: string }).code === '23505') {
			throw new AdminError(409, `${unit.login} is already connected to another dōjō`);
		}
		fail(error);
	}
	await claimTenantIfOwner(db, PROVIDER, unit.id, principalId, 'admin');
	return { id: row.id, key: row.key };
}

/** A workspace slug for a project — its name, slugified, made unique among the
 *  slugs already taken in the dōjō; a name with no letter or digit takes the
 *  project id's head. */
function freeSlug(project: AzureProject, taken: Set<string>): string {
	const base = slugify(project.name) || `project-${slugify(project.id).slice(0, 8)}`;
	let slug = base;
	for (let n = 2; taken.has(slug); n++) slug = `${base}-${n}`;
	taken.add(slug);
	return slug;
}

interface MapRow {
	external_id: string;
	tenant_id: string;
	workspace_id: string | null;
}

/**
 * File each project as a workspace of the dōjō (dojo.map_forge_unit, proven).
 * A project already filed here keeps its workspace (and takes its new name, so
 * a project renamed upstream still matches); one an admin filed into another
 * dōjō is left there; a new one gets a workspace of its own name.
 */
async function mapProjects(
	db: DojoClient,
	principalId: string,
	tenantId: string,
	org: string,
	projects: AzureProject[]
): Promise<{ mapped: number; elsewhere: number }> {
	if (projects.length === 0) return { mapped: 0, elsewhere: 0 };
	const [maps, spaces] = await Promise.all([
		db
			.from('forge_unit_mappings')
			.select('external_id, tenant_id, workspace_id')
			.eq('provider', PROVIDER)
			.in('external_id', projects.map((p) => p.id)),
		db.from('workspaces').select('id, slug, name').eq('tenant_id', tenantId)
	]);
	fail(maps.error);
	fail(spaces.error);
	const mapOf = new Map(((maps.data ?? []) as MapRow[]).map((m) => [m.external_id, m]));
	const workspaces = (spaces.data ?? []) as { id: string; slug: string; name: string }[];
	const workspaceOf = new Map(workspaces.map((w) => [w.id, w]));
	const taken = new Set(workspaces.map((w) => w.slug));

	let mapped = 0;
	let elsewhere = 0;
	for (const project of projects) {
		const prev = mapOf.get(project.id);
		if (prev && prev.tenant_id !== tenantId) {
			elsewhere++;
			continue;
		}
		const kept = prev?.workspace_id ? workspaceOf.get(prev.workspace_id) : undefined;
		const slug = kept ? kept.slug : freeSlug(project, taken);
		// p_keep_elsewhere: the database itself leaves a project another dōjō holds
		// where it is, even if it moved there after the read above.
		const { data, error } = await db
			.rpc('map_forge_unit', {
				p_principal: principalId,
				p_provider: PROVIDER,
				p_external_id: project.id,
				p_external_slug: `${org}/${project.name}`,
				p_tenant: tenantId,
				p_workspace_slug: slug,
				p_workspace_name: kept ? kept.name : project.name,
				p_proven: true,
				p_keep_elsewhere: true
			})
			.single();
		fail(error);
		const filedIn = (data as { tenant_id?: unknown } | null)?.tenant_id;
		if (typeof filedIn !== 'string') throw new AdminError(500, `map_forge_unit did not say where ${project.name} is filed`);
		if (filedIn === tenantId) mapped++;
		else elsewhere++;
	}
	return { mapped, elsewhere };
}

/**
 * Follow the organization's member list (#309's rule, from the organization's
 * side): read every member with the token; a COMPLETE read that lists anyone
 * hands their forge ids to dojo.reconcile_unit_members, which ends the
 * provisioned memberships of people it no longer lists, restores those who
 * came back, keeps the list as the connection's member set and joins the listed
 * people who already signed in (#308 R5c — what a Microsoft sign-in provisions
 * from). A partial read, an empty one or a failed one ends, joins and keeps
 * nothing — the failure is said in the answer, and the import or sync that
 * asked still stands.
 */
export async function followMembers(
	db: DojoClient,
	tenantId: string,
	adapter: AzureAdapter,
	token: string,
	unit: { id: string; login: string }
): Promise<AzureMembersRead> {
	let read: Awaited<ReturnType<AzureAdapter['listMembers']>>;
	try {
		read = await adapter.listMembers(token, unit);
	} catch (e) {
		const status = e instanceof ForgeReadError ? ` (${e.forgeStatus})` : '';
		return { error: `Azure DevOps could not list the members${status}; nobody was ended` };
	}
	const listed = read.members.length;
	if (!read.complete || listed === 0) return { complete: false, listed, ended: 0, joined: 0 };
	const { data, error } = await db
		.rpc('reconcile_unit_members', {
			p_tenant: tenantId,
			p_provider: PROVIDER,
			p_unit: unit.id,
			p_auth: authMethodFor(PROVIDER),
			p_subjects: read.members.map((m) => m.id)
		})
		.single();
	fail(error);
	const row = data as { ended?: unknown; joined?: unknown } | null;
	if (typeof row?.ended !== 'number' || typeof row.joined !== 'number') {
		throw new AdminError(500, 'reconcile_unit_members did not say how many memberships it ended and joined');
	}
	return { complete: true, listed, ended: row.ended, joined: row.joined };
}

/**
 * Import one Azure DevOps organization with an admin's Entra grant: validate, then make
 * or connect its dōjō, claim it, store the token in Vault and file its projects
 * as workspaces. See the module header.
 */
export async function importAzureOrganization(
	db: DojoClient,
	principalId: string,
	target: AzureImportTarget,
	grant: AzureGrantTokens,
	adapter: AzureAdapter
): Promise<AzureImportResult> {
	const token = grant.accessToken;
	const { unit, read } = await validate(adapter, token);
	const owner = await adapter.owner(token);
	// The connect was started from this dōjō's admin screen minutes ago; the
	// import is a separate request, so the admin floor is checked again here.
	if (target.as === 'this') await requireActiveAdmin(db, principalId, target.tenantId);

	// The grant's owner, as this person, by Microsoft Entra object id (D43/D45):
	// one Azure account is one person (a 409 if another already holds it) — what
	// a member read is matched against, and what a Microsoft sign-in will see.
	await ensureForgeIdentity(db, principalId, PROVIDER, { id: owner.objectId, name: owner.displayName, email: null });

	let dojo: { id: string; key: string; created: boolean };
	if (target.as === 'own') {
		const t = await ensureOrgTenant(db, principalId, PROVIDER, { ...unit, role: 'admin' }, authMethodFor(PROVIDER), {
			evenIfMapped: true
		});
		if (!t) throw new AdminError(500, `no dōjō could be made for ${unit.login}`);
		dojo = { id: t.id, key: t.key, created: t.created };
	} else {
		dojo = { ...(await connectHere(db, principalId, target.tenantId, unit)), created: false };
	}

	const stored = await db.rpc('set_forge_token', {
		p_principal: principalId,
		p_provider: PROVIDER,
		p_external_id: unit.id,
		p_token: grant.refreshToken,
		p_expires_at: null
	});
	fail(stored.error);

	const filed = await mapProjects(db, principalId, dojo.id, unit.login, read.projects);
	const members = await followMembers(db, dojo.id, adapter, token, unit);
	return {
		dojo: { key: dojo.key, created: dojo.created },
		organization: { id: unit.id, name: unit.login },
		projects: { total: read.projects.length, ...filed, complete: read.complete },
		members
	};
}

/** What reading with a stored grant needs: the app registration, and fetch. */
export interface AzureStoredGrantDeps {
	cfg: AzureEntraConfig;
	fetchImpl?: typeof fetch;
}

/**
 * A fresh access token from a connection's STORED grant (#308 R5b, D42): the
 * refresh token is read from Vault, refreshed with Entra, and the rotated one
 * stored back in place (dojo.rotate_forge_token, unaudited) — both
 * compare-and-set on the token refreshed, so a sync racing a reconnect never
 * touches the newer grant. An `invalid_grant`
 * — revoked, expired, consent withdrawn — marks the connection needs_new_token
 * and is a 409 "reconnect": reads stop and no membership is ended (#309). A
 * connection already marked is refused by the database (409).
 */
export async function accessFromStoredGrant(db: DojoClient, orgId: string, deps: AzureStoredGrantDeps): Promise<string> {
	const stored = await db.rpc('forge_token', { p_provider: PROVIDER, p_external_id: orgId }).single();
	fail(stored.error);
	if (typeof stored.data !== 'string' || !stored.data) throw new AdminError(500, 'no grant could be read for this connection');
	let tokens;
	try {
		tokens = await refreshGrant(deps.cfg, stored.data, deps.fetchImpl ?? fetch);
	} catch (e) {
		if (e instanceof AzureGrantRejected) {
			// Compare-and-set: marks only if the stored grant is still the one refused.
			const marked = await db.rpc('mark_forge_token_rejected', { p_provider: PROVIDER, p_external_id: orgId, p_rejected: stored.data });
			fail(marked.error);
		}
		throw e;
	}
	// Compare-and-set: a no-op when an admin reconnected meanwhile (the stored
	// grant is newer than the one refreshed here).
	const rotated = await db.rpc('rotate_forge_token', {
		p_provider: PROVIDER,
		p_external_id: orgId,
		p_expected: stored.data,
		p_token: tokens.refreshToken
	});
	fail(rotated.error);
	return tokens.accessToken;
}

/**
 * Re-read one of this dōjō's Azure organizations with its STORED grant: re-prove
 * the admin, file its projects (new ones get workspaces; renamed ones keep
 * theirs), and follow its members.
 */
export async function syncAzureOrganization(
	db: DojoClient,
	principalId: string,
	tenantId: string,
	orgId: string,
	deps: AzureStoredGrantDeps & { adapterFor: (organization: string) => AzureAdapter }
): Promise<AzureSyncResult> {
	const org = await azureConnectionOf(db, tenantId, orgId);
	const token = await accessFromStoredGrant(db, org.id, deps);
	const adapter = deps.adapterFor(org.name);
	const checked = await validate(adapter, token);
	if (checked.unit.id !== org.id) {
		throw new AdminError(409, `${org.name} now names a different Azure DevOps organization; nothing was changed`);
	}
	const filed = await mapProjects(db, principalId, tenantId, org.name, checked.read.projects);
	const members = await followMembers(db, tenantId, adapter, token, checked.unit);
	return {
		organization: org,
		projects: { total: checked.read.projects.length, ...filed, complete: checked.read.complete },
		members
	};
}

/** This dōjō's Azure DevOps organizations. Throws on a failed read. */
export async function listAzureConnections(db: DojoClient, tenantId: string): Promise<AzureConnectionView[]> {
	const { data, error } = await db
		.from('tenant_connections')
		.select('external_id, external_slug, verified_at, token_secret_id, token_state, token_expires_at')
		.eq('tenant_id', tenantId)
		.eq('provider', PROVIDER);
	fail(error);
	return ((data ?? []) as Record<string, unknown>[])
		.filter((r) => typeof r.external_id === 'string')
		.map((r) => ({
			id: r.external_id as string,
			name: r.external_slug as string,
			verifiedAt: (r.verified_at as string | null) ?? null,
			token: r.token_secret_id ? ((r.token_state as 'ok' | 'needs_new_token' | null) ?? 'ok') : 'none',
			tokenExpiresAt: (r.token_expires_at as string | null) ?? null
		}));
}

/** The connection of one of this dōjō's Azure organizations, or a 404. */
export async function azureConnectionOf(db: DojoClient, tenantId: string, orgId: string): Promise<{ id: string; name: string }> {
	const { data, error } = await db
		.from('tenant_connections')
		.select('external_id, external_slug')
		.eq('tenant_id', tenantId)
		.eq('provider', PROVIDER)
		.eq('external_id', orgId)
		.maybeSingle();
	fail(error);
	const row = data as { external_id: string; external_slug: string } | null;
	if (!row) throw new AdminError(404, 'this dōjō is not connected to that Azure DevOps organization');
	return { id: row.external_id, name: row.external_slug };
}

/** Remove the stored token of one of this dōjō's Azure organizations. */
export async function removeAzureToken(db: DojoClient, principalId: string, tenantId: string, orgId: string): Promise<void> {
	await azureConnectionOf(db, tenantId, orgId);
	const { error } = await db.rpc('remove_forge_token', {
		p_principal: principalId,
		p_provider: PROVIDER,
		p_external_id: orgId
	});
	fail(error);
}

async function bodyOf(request: Request): Promise<Record<string, unknown>> {
	return ((await request.json().catch(() => ({}))) ?? {}) as Record<string, unknown>;
}

/** An import request: the organization (by the name the organization list gave). */
export async function importRequest(request: Request): Promise<{ organization: string }> {
	const body = await bodyOf(request);
	const organization = typeof body.organization === 'string' ? body.organization.trim().toLowerCase() : '';
	if (!organization) throw new AdminError(400, 'which organization? (organization)');
	return { organization };
}

/** A request naming one of this dōjō's Azure organizations by its id. */
export async function organizationRequest(request: Request): Promise<{ orgId: string }> {
	const body = await bodyOf(request);
	const orgId = typeof body.organization_id === 'string' ? body.organization_id.trim() : '';
	if (!orgId) throw new AdminError(400, 'which organization? (organization_id)');
	return { orgId };
}
