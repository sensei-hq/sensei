// Admin → Integrations (#308 R5a, D20/D37): a dōjō comes from a git integration.
//
// The person lists the units (GitHub orgs) their own token can see, and for each:
//   * CLAIM its dōjō — only an admin of the org, proven now with their token;
//   * MAP it — to its own dōjō (an admin of the org), or to a workspace of the
//     dōjō whose screen this is (anyone who administers that dōjō, while the org
//     is unclaimed and no admin of it has placed it).
// Spec: docs/spec/dojo/git-integration-import.md.
//
// The token is the person's GitHub connection (a "Connect GitHub"
// re-authorization, integrations-route.ts), used for the reads of one request and
// never stored, logged or echoed. Every forge fact is re-read at the moment of the act — never taken
// from the client, and never from an earlier listing (§II.5).
//
// This module is system work (the server's service-role client): it reads every
// tenant's connections, which no session may, and the two writes are the
// SECURITY DEFINER functions that hold the rules (dojo.claim_forge_unit,
// dojo.map_forge_unit). It reveals another dōjō only to someone in it.
import { AdminError, slugify, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';
import type { ForgeAdapter, ForgeUnit } from './forge-adapter';
import { isForgeProvider, type ForgeProvider } from '../forge-providers';
import type {
	IntegrationDojo,
	IntegrationMapping,
	IntegrationUnit,
	IntegrationUnitsView,
	MapTarget
} from '../integrations-view';
import { authMethodFor, ensureOrgTenant } from './provisioning';

export type { IntegrationDojo, IntegrationMapping, IntegrationUnit, IntegrationUnitsView, MapTarget };

/** Every row of one read, or a 500 — never `[]` for a failure. */
async function rows<T>(query: PromiseLike<{ data: unknown; error: { message: string } | null }>): Promise<T[]> {
	const { data, error } = await query;
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as T[];
}

const uniq = (xs: (string | null | undefined)[]) => [...new Set(xs.filter((x): x is string => Boolean(x)))];

interface ConnRow {
	tenant_id: string;
	external_id: string;
}
interface MapRow {
	external_id: string;
	tenant_id: string;
	workspace_id: string | null;
	proven_at: string | null;
}
interface TenantRow {
	id: string;
	key: string;
	name: string;
	claimed_at: string | null;
	claimed_by: string | null;
}

/**
 * The units the person's token can see on this forge, each with what it is today
 * and what the person may do with it from this dōjō's screen.
 *
 * Throws on a failed forge read (an outage is never "you are in no orgs") and on
 * any failed table read.
 */
export async function listIntegrationUnits(
	db: DojoClient,
	principalId: string,
	tenantId: string,
	token: string,
	adapter: ForgeAdapter
): Promise<IntegrationUnitsView> {
	const read = await adapter.listUnits(token);
	const provider = adapter.provider;
	const ids = read.units.map((u) => u.id);
	if (ids.length === 0) return { provider, units: [], complete: read.complete };

	const [conns, maps, mine] = await Promise.all([
		rows<ConnRow>(
			db.from('tenant_connections').select('tenant_id, external_id').eq('provider', provider).in('external_id', ids)
		),
		rows<MapRow>(
			db
				.from('forge_unit_mappings')
				.select('external_id, tenant_id, workspace_id, proven_at')
				.eq('provider', provider)
				.in('external_id', ids)
		),
		rows<{ tenant_id: string }>(
			db.from('memberships').select('tenant_id').eq('user_id', principalId).is('disabled_at', null)
		)
	]);
	const readable = new Set([tenantId, ...mine.map((m) => m.tenant_id)]);
	const visibleMaps = maps.filter((m) => readable.has(m.tenant_id));

	const tenantIds = uniq([...conns.map((c) => c.tenant_id), ...visibleMaps.map((m) => m.tenant_id)]);
	const tenants = tenantIds.length
		? await rows<TenantRow>(db.from('tenants').select('id, key, name, claimed_at, claimed_by').in('id', tenantIds))
		: [];
	const claimers = uniq(tenants.map((t) => t.claimed_by));
	const workspaceIds = uniq(visibleMaps.map((m) => m.workspace_id));
	const [principals, workspaces] = await Promise.all([
		claimers.length
			? rows<{ id: string; display_name: string | null }>(db.from('principals').select('id, display_name').in('id', claimers))
			: Promise.resolve([]),
		workspaceIds.length
			? rows<{ id: string; name: string }>(db.from('workspaces').select('id, name').in('id', workspaceIds))
			: Promise.resolve([])
	]);

	const tenantOf = new Map(tenants.map((t) => [t.id, t]));
	const nameOf = new Map(principals.map((p) => [p.id, p.display_name]));
	const workspaceOf = new Map(workspaces.map((w) => [w.id, w.name]));
	const connOf = new Map(conns.map((c) => [c.external_id, c.tenant_id]));
	const mapOf = new Map(maps.map((m) => [m.external_id, m]));

	const units = read.units.map((unit): IntegrationUnit => {
		const own = tenantOf.get(connOf.get(unit.id) ?? '');
		const dojo: IntegrationDojo | null = own
			? {
					key: own.key,
					name: own.name,
					claimed: own.claimed_at != null,
					claimedBy: own.claimed_by ? (nameOf.get(own.claimed_by) ?? null) : null
				}
			: null;
		const m = mapOf.get(unit.id);
		const isAdmin = unit.role === 'admin';
		return {
			provider,
			id: unit.id,
			login: unit.login,
			role: unit.role,
			dojo,
			mapping: m ? mappingView(m, tenantId, readable, tenantOf, workspaceOf) : null,
			canClaim: isAdmin && !dojo?.claimed,
			canMapAsDojo: isAdmin,
			canMapAsWorkspace: isAdmin || (!dojo?.claimed && !m?.proven_at)
		};
	});
	return { provider, units, complete: read.complete };
}

function mappingView(
	m: MapRow,
	tenantId: string,
	readable: Set<string>,
	tenantOf: Map<string, TenantRow>,
	workspaceOf: Map<string, string>
): IntegrationMapping {
	if (!m.workspace_id) return { as: 'dojo' };
	const visible = readable.has(m.tenant_id);
	return {
		as: 'workspace',
		inThisDojo: m.tenant_id === tenantId,
		dojoKey: visible ? (tenantOf.get(m.tenant_id)?.key ?? null) : null,
		workspaceName: visible ? (workspaceOf.get(m.workspace_id) ?? null) : null
	};
}

/** The unit with this id among those the token can see, or a 404. */
async function visibleUnit(adapter: ForgeAdapter, token: string, unitId: string): Promise<ForgeUnit> {
	const { units } = await adapter.listUnits(token);
	const unit = units.find((u) => u.id === unitId);
	if (!unit) throw new AdminError(404, 'that organization is not one your token can see');
	return unit;
}

/** The unit's own dōjō, made if it has none, with the admin in it as admin. */
async function ownDojo(db: DojoClient, principalId: string, provider: ForgeProvider, unit: ForgeUnit) {
	const tenant = await ensureOrgTenant(
		db,
		principalId,
		provider,
		{ ...unit, role: 'admin' },
		authMethodFor(provider),
		{ evenIfMapped: true }
	);
	if (!tenant) throw new AdminError(500, `no dōjō could be made for ${unit.login}`);
	return tenant;
}

/**
 * Claim a unit's dōjō: prove, with the person's own token, that they administer
 * the unit; make its dōjō if it has none; claim it (dojo.claim_forge_unit, via
 * the same path a forge admin's sign-in takes — first claim wins, audited).
 * A member is refused (403) before anything is written; a proof that cannot be
 * read throws and writes nothing.
 */
export async function claimIntegrationUnit(
	db: DojoClient,
	principalId: string,
	token: string,
	adapter: ForgeAdapter,
	unitId: string
): Promise<{ dojo: { key: string } }> {
	const unit = await visibleUnit(adapter, token, unitId);
	if (!(await adapter.proveAdmin(token, unit))) {
		throw new AdminError(403, `only an admin of ${unit.login} can claim its dōjō`);
	}
	const tenant = await ownDojo(db, principalId, adapter.provider, unit);
	return { dojo: { key: tenant.key } };
}

/**
 * Map a unit: to its own dōjō (a proven admin of the unit only — its dōjō is
 * made if it has none), or to a workspace of THIS dōjō, named `name` or after the
 * unit. The admin proof is read now and passed to dojo.map_forge_unit, which
 * enforces who may do what and keeps its refusal's status and reason.
 */
export async function mapIntegrationUnit(
	db: DojoClient,
	principalId: string,
	tenantId: string,
	token: string,
	adapter: ForgeAdapter,
	unitId: string,
	target: MapTarget
): Promise<{ tenantId: string; workspaceId: string | null }> {
	const unit = await visibleUnit(adapter, token, unitId);

	let workspaceName: string | null = null;
	let workspaceSlug: string | null = null;
	if (target.as === 'workspace') {
		workspaceName = (target.name ?? '').trim() || unit.login;
		workspaceSlug = slugify(workspaceName);
		if (!workspaceSlug) throw new AdminError(400, 'a workspace name needs a letter or a digit');
	}

	const proven = await adapter.proveAdmin(token, unit);
	let into = tenantId;
	if (target.as === 'dojo') {
		if (!proven) throw new AdminError(403, `only an admin of ${unit.login} can make it its own dōjō`);
		into = (await ownDojo(db, principalId, adapter.provider, unit)).id;
	}

	const { data, error } = await db
		.rpc('map_forge_unit', {
			p_principal: principalId,
			p_provider: adapter.provider,
			p_external_id: unit.id,
			p_external_slug: unit.login,
			p_tenant: into,
			p_workspace_slug: workspaceSlug,
			p_workspace_name: workspaceName,
			p_proven: proven
		})
		.single();
	if (error) throw new AdminError(refusalStatus(error), error.message);
	const row = data as { tenant_id: string; workspace_id: string | null };
	return { tenantId: row.tenant_id, workspaceId: row.workspace_id };
}

/** The route's `[provider]`, or a 404 — a name that is not a forge addresses
 *  nothing. (A forge with no adapter yet is `forgeAdapter`'s 501.) */
export function providerParam(p: string | undefined): ForgeProvider {
	if (!isForgeProvider(p)) throw new AdminError(404, 'no such git provider');
	return p;
}

/** An Integrations request body, or {} when it is not JSON. It carries no token:
 *  the GitHub token comes only from the connection (integrations-route.ts). */
async function bodyOf(request: Request): Promise<Record<string, unknown>> {
	return ((await request.json().catch(() => ({}))) ?? {}) as Record<string, unknown>;
}

function unitOf(body: Record<string, unknown>): string {
	const unitId = typeof body.unit_id === 'string' ? body.unit_id.trim() : '';
	if (!unitId) throw new AdminError(400, 'which organization? (unit_id)');
	return unitId;
}

/** A claim request: the unit. */
export async function claimRequest(request: Request): Promise<{ unitId: string }> {
	return { unitId: unitOf(await bodyOf(request)) };
}

/** A map request: the unit, and where it goes. */
export async function mapRequest(request: Request): Promise<{ unitId: string; target: MapTarget }> {
	const body = await bodyOf(request);
	const unitId = unitOf(body);
	if (body.as === 'dojo') return { unitId, target: { as: 'dojo' } };
	if (body.as === 'workspace') {
		const name = typeof body.name === 'string' && body.name.trim() ? body.name.trim() : undefined;
		return { unitId, target: name ? { as: 'workspace', name } : { as: 'workspace' } };
	}
	throw new AdminError(400, "map it as 'dojo' or 'workspace'");
}
