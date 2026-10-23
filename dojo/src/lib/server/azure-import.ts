// Importing an Azure DevOps organization into the dōjō (#308 R5b, D35/D36).
//
// An organization admin pastes the organization's URL and a personal access
// token. The token is VALIDATED FIRST — it must name the organization (its
// owner active there) and prove its owner a Project Collection Administrator —
// and only then is anything written. A failing token is refused and never
// stored. Then:
//   * the organization → a dōjō: from the PERSONAL dōjō, its own dōjō is made
//     (or joined, as a proven admin) — how an Azure organization's dōjō is first
//     created; from an ORGANIZATION dōjō's Integrations, it connects to that
//     dōjō (one organization known to GitHub and Azure). Claimed through
//     dojo.claim_forge_unit, provider azure_devops;
//   * the token → Supabase Vault (dojo.set_forge_token); the connection keeps
//     only the secret's id and the token's state;
//   * each project → a workspace in that dōjō (dojo.map_forge_unit, keyed by the
//     project's stable id, named `{org}/{project}` — what
//     dojo.register_repository matches an Azure repository's project against).
//
// System work (the server's service-role client): it reads every tenant's
// connections and writes through the SECURITY DEFINER functions that hold the
// rules. The token is used for this request's reads and the Vault write only —
// never returned, logged or put in an error.
import { AdminError, slugify, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';
import { azureOrgFromUrl, type AzureAdapter, type AzureConnection, type AzureProject } from './forge-azure';
import { ForgeReadError } from './forge-github';
import { authMethodFor, claimTenantIfOwner, ensureForgeIdentity, ensureOrgTenant } from './provisioning';

const PROVIDER = 'azure_devops' as const;

/** Where the organization goes: its own dōjō (from the personal dōjō), or the
 *  organization dōjō whose Integrations screen asked. */
export type AzureImportTarget = { as: 'own' } | { as: 'this'; tenantId: string };

/** What an import came to. Never carries the token. */
export interface AzureImportResult {
	dojo: { key: string; created: boolean };
	organization: { id: string; name: string };
	projects: {
		/** Projects the token could read. */
		total: number;
		/** Filed as workspaces of the dōjō (new, or kept and renamed). */
		mapped: number;
		/** Already filed into another dōjō by an admin there — left as they are. */
		elsewhere: number;
		/** Whether the project read saw every project. */
		complete: boolean;
	};
}

function fail(error: { code?: string; message: string } | null): void {
	if (error) throw new AdminError(refusalStatus(error), error.message);
}

/**
 * Validate the token against the organization — the token's owner is active in
 * it and is a Project Collection Administrator — and read its projects. Reads
 * only: nothing is written before this returns. A token Azure rejects throws its
 * ForgeReadError (the route answers "Azure DevOps did not accept that token").
 */
async function validate(adapter: AzureAdapter, token: string) {
	const conn: AzureConnection = await adapter.connection(token);
	const org = adapter.organization;
	if (!conn.user.active) {
		throw new AdminError(403, `this token's account is not a member of ${org} on Azure DevOps`);
	}
	const unit = { id: conn.orgId, login: org };
	if (!(await adapter.proveAdmin(token, unit))) {
		throw new AdminError(403, `only a Project Collection Administrator of ${org} can import it`);
	}
	const read = await adapter.listProjects(token, unit);
	return { conn, unit, read };
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
		const { error } = await db
			.rpc('map_forge_unit', {
				p_principal: principalId,
				p_provider: PROVIDER,
				p_external_id: project.id,
				p_external_slug: `${org}/${project.name}`,
				p_tenant: tenantId,
				p_workspace_slug: slug,
				p_workspace_name: kept ? kept.name : project.name,
				p_proven: true
			})
			.single();
		fail(error);
		mapped++;
	}
	return { mapped, elsewhere };
}

/**
 * Import one Azure DevOps organization with an admin's PAT: validate, then make
 * or connect its dōjō, claim it, store the token in Vault and file its projects
 * as workspaces. See the module header.
 */
export async function importAzureOrganization(
	db: DojoClient,
	principalId: string,
	target: AzureImportTarget,
	token: string,
	adapter: AzureAdapter
): Promise<AzureImportResult> {
	const { conn, unit, read } = await validate(adapter, token);

	// The token's owner, as this person: one Azure account is one person (a 409
	// if another already holds it) — what a member read is matched against.
	await ensureForgeIdentity(db, principalId, PROVIDER, { id: conn.user.id, name: conn.user.displayName, email: null });

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
		p_token: token,
		p_expires_at: null
	});
	fail(stored.error);

	const filed = await mapProjects(db, principalId, dojo.id, unit.login, read.projects);
	return {
		dojo: { key: dojo.key, created: dojo.created },
		organization: { id: unit.id, name: unit.login },
		projects: { total: read.projects.length, ...filed, complete: read.complete }
	};
}

/** What a sync came to. Never carries the token. */
export interface AzureSyncResult {
	organization: { id: string; name: string };
	projects: AzureImportResult['projects'];
}

/**
 * Re-read one of this dōjō's Azure organizations with its STORED token: re-prove
 * the admin and file its projects (new ones get workspaces; renamed ones keep
 * theirs). The token is read from Vault for this request only.
 *
 * A token Azure rejects (401) marks the connection needs_new_token and stops
 * (409) — reads stay stopped until an admin pastes a new one. Any other failed
 * read is thrown as it is: an outage is not a dead token.
 */
export async function syncAzureOrganization(
	db: DojoClient,
	principalId: string,
	tenantId: string,
	orgId: string,
	adapterFor: (organization: string) => AzureAdapter
): Promise<AzureSyncResult> {
	const org = await azureConnectionOf(db, tenantId, orgId);
	const stored = await db.rpc('forge_token', { p_provider: PROVIDER, p_external_id: org.id }).single();
	fail(stored.error);
	const token = stored.data;
	if (typeof token !== 'string' || !token) throw new AdminError(500, `no token could be read for ${org.name}`);

	let checked: Awaited<ReturnType<typeof validate>>;
	try {
		checked = await validate(adapterFor(org.name), token);
	} catch (e) {
		if (e instanceof ForgeReadError && e.forgeStatus === 401) {
			const marked = await db.rpc('mark_forge_token_rejected', { p_provider: PROVIDER, p_external_id: org.id });
			fail(marked.error);
			throw new AdminError(409, `Azure DevOps no longer accepts the stored token for ${org.name}; paste a new one`);
		}
		throw e;
	}
	if (checked.unit.id !== org.id) {
		throw new AdminError(409, `${org.name} now names a different Azure DevOps organization; nothing was changed`);
	}

	const filed = await mapProjects(db, principalId, tenantId, org.name, checked.read.projects);
	return {
		organization: org,
		projects: { total: checked.read.projects.length, ...filed, complete: checked.read.complete }
	};
}

/** One of a dōjō's Azure DevOps organizations, as the screen shows it. */
export interface AzureConnectionView {
	/** The organization's stable id. */
	id: string;
	name: string;
	verifiedAt: string | null;
	/** The stored token: working, rejected by Azure, or none stored. */
	token: 'ok' | 'needs_new_token' | 'none';
	tokenExpiresAt: string | null;
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

/** A connect request: the organization (from its URL) and the token. A 400
 *  without either; the message never repeats the token. */
export async function connectRequest(request: Request): Promise<{ organization: string; token: string }> {
	const body = await bodyOf(request);
	const url = typeof body.organization_url === 'string' ? body.organization_url : '';
	const token = typeof body.token === 'string' ? body.token.trim() : '';
	if (!url.trim()) throw new AdminError(400, 'which organization? (organization_url)');
	if (!token) throw new AdminError(400, 'a personal access token is needed (token)');
	return { organization: azureOrgFromUrl(url), token };
}

/** A request naming one of this dōjō's Azure organizations by its id. */
export async function organizationRequest(request: Request): Promise<{ orgId: string }> {
	const body = await bodyOf(request);
	const orgId = typeof body.organization_id === 'string' ? body.organization_id.trim() : '';
	if (!orgId) throw new AdminError(400, 'which organization? (organization_id)');
	return { orgId };
}
