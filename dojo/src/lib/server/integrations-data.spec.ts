// Admin → Integrations (#308 R5a): the GitHub orgs a person can see, what each
// is today (its own dōjō, claimed or not; or a workspace of a dōjō), and the two
// acts — claim an org's dōjō, and map an org to its own dōjō or to a workspace
// of this one. Every forge fact is read with the person's OWN token at the
// moment of the act; the database functions enforce who may do what
// (database/tests/dojo/forge_unit_claim.sql, forge_unit_mapping.sql). What is
// proven here is this side's half: what it reads, what it asks, what it refuses
// before writing anything.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, provisioningTables, type FakeTable } from './fake-dojo-db';
import { AdminError } from './admin-data';
import { ForgeReadError } from './forge-github';
import type { ForgeAdapter, ForgeUnit } from './forge-adapter';
import { listIntegrationUnits, claimIntegrationUnit, mapIntegrationUnit } from './integrations-data';

const ME = 'p-me';
const HERE = 't-here'; // the dōjō whose Integrations screen this is
const ACME: ForgeUnit = { id: '22', login: 'acme', role: 'admin' };
const BETA: ForgeUnit = { id: '33', login: 'beta', role: 'member' };

function adapter(units: ForgeUnit[], admins: string[] = [], complete = true) {
	const proved: string[] = [];
	const a: ForgeAdapter = {
		provider: 'github',
		listUnits: async () => ({ units, complete }),
		proveAdmin: async (_t, unit) => {
			proved.push(unit.id);
			return admins.includes(unit.id);
		},
		listMembers: async () => ({ members: [], complete: true })
	};
	return { a, proved };
}

function db(seed: Partial<Record<string, FakeTable>> = {}) {
	return fakeDojoDb({
		...provisioningTables(seed),
		tenants: {
			rows: [{ id: HERE, key: 'organization/here', origin: 'organization', slug: 'here', name: 'Here' }],
			uniques: [{ columns: ['key'] }],
			...seed.tenants
		},
		memberships: {
			rows: [{ id: 'm-here', tenant_id: HERE, user_id: ME, role: 'admin', disabled_at: null }],
			uniques: [{ columns: ['tenant_id', 'user_id'] }],
			...seed.memberships
		},
		principals: { rows: [{ id: 'p-owner', display_name: 'Owner Person' }], ...seed.principals },
		workspaces: { rows: [], ...seed.workspaces },
		'rpc:map_forge_unit': { rows: [{ mapping_id: 'fm-1', tenant_id: HERE, workspace_id: 'w-1' }], ...seed['rpc:map_forge_unit'] }
	});
}

beforeEach(() => resetFakeIds());

describe('listIntegrationUnits', () => {
	it('lists every org the token sees, with its own dōjō and whether it is claimed', async () => {
		const d = db({
			tenants: {
				rows: [
					{ id: HERE, key: 'organization/here', origin: 'organization', slug: 'here', name: 'Here' },
					{ id: 't-acme', key: 'organization/acme', origin: 'organization', slug: 'acme', name: 'acme', claimed_at: '2026-10-01T00:00:00Z', claimed_by: 'p-owner' },
					{ id: 't-beta', key: 'organization/beta', origin: 'organization', slug: 'beta', name: 'beta', claimed_at: null, claimed_by: null }
				]
			},
			tenant_connections: {
				rows: [
					{ id: 'c1', tenant_id: 't-acme', provider: 'github', external_id: '22', external_slug: 'acme' },
					{ id: 'c2', tenant_id: 't-beta', provider: 'github', external_id: '33', external_slug: 'beta' }
				]
			}
		});
		const out = await listIntegrationUnits(d as never, ME, HERE, 'tok', adapter([ACME, BETA]).a);
		expect(out.complete).toBe(true);
		expect(out.units).toEqual([
			{
				provider: 'github',
				id: '22',
				login: 'acme',
				role: 'admin',
				dojo: { key: 'organization/acme', name: 'acme', claimed: true, claimedBy: 'Owner Person' },
				mapping: null,
				canClaim: false,
				canMapAsDojo: true,
				canMapAsWorkspace: true
			},
			{
				provider: 'github',
				id: '33',
				login: 'beta',
				role: 'member',
				dojo: { key: 'organization/beta', name: 'beta', claimed: false, claimedBy: null },
				mapping: null,
				canClaim: false,
				canMapAsDojo: false,
				canMapAsWorkspace: true
			}
		]);
	});

	it('an admin may claim an unclaimed dōjō — or one not made yet; a member never', async () => {
		const out = await listIntegrationUnits(db() as never, ME, HERE, 'tok', adapter([ACME, BETA]).a);
		expect(out.units.map((u) => [u.login, u.dojo, u.canClaim])).toEqual([
			['acme', null, true],
			['beta', null, false]
		]);
	});

	it('a member cannot file a CLAIMED org, nor one an admin of it already placed', async () => {
		const d = db({
			tenants: {
				rows: [
					{ id: HERE, key: 'organization/here', origin: 'organization', slug: 'here', name: 'Here' },
					{ id: 't-beta', key: 'organization/beta', origin: 'organization', slug: 'beta', name: 'beta', claimed_at: '2026-10-01T00:00:00Z', claimed_by: null }
				]
			},
			tenant_connections: { rows: [{ id: 'c2', tenant_id: 't-beta', provider: 'github', external_id: '33', external_slug: 'beta' }] }
		});
		const claimed = await listIntegrationUnits(d as never, ME, HERE, 'tok', adapter([BETA]).a);
		expect(claimed.units[0].canMapAsWorkspace).toBe(false);

		const placed = db({
			forge_unit_mappings: {
				rows: [{ id: 'fm', provider: 'github', external_id: '33', external_slug: 'beta', tenant_id: HERE, workspace_id: null, proven_at: '2026-10-01T00:00:00Z' }]
			}
		});
		const out = await listIntegrationUnits(placed as never, ME, HERE, 'tok', adapter([BETA]).a);
		expect(out.units[0].canMapAsWorkspace).toBe(false);
	});

	it('shows a workspace mapping into this dōjō by name, and one into a dōjō the person is not in without naming it', async () => {
		const d = db({
			tenants: {
				rows: [
					{ id: HERE, key: 'organization/here', origin: 'organization', slug: 'here', name: 'Here' },
					{ id: 't-secret', key: 'organization/secret', origin: 'organization', slug: 'secret', name: 'Secret' }
				]
			},
			forge_unit_mappings: {
				rows: [
					{ id: 'fm1', provider: 'github', external_id: '22', external_slug: 'acme', tenant_id: HERE, workspace_id: 'w-acme', proven_at: null },
					{ id: 'fm2', provider: 'github', external_id: '33', external_slug: 'beta', tenant_id: 't-secret', workspace_id: 'w-beta', proven_at: null }
				]
			},
			workspaces: {
				rows: [
					{ id: 'w-acme', tenant_id: HERE, name: 'Acme' },
					{ id: 'w-beta', tenant_id: 't-secret', name: 'Beta' }
				]
			}
		});
		const out = await listIntegrationUnits(d as never, ME, HERE, 'tok', adapter([ACME, BETA]).a);
		expect(out.units[0].mapping).toEqual({ as: 'workspace', inThisDojo: true, dojoKey: 'organization/here', workspaceName: 'Acme' });
		// Another dōjō's key and workspace are not the caller's to read.
		expect(out.units[1].mapping).toEqual({ as: 'workspace', inThisDojo: false, dojoKey: null, workspaceName: null });
	});

	it('passes an incomplete forge read through as incomplete', async () => {
		const out = await listIntegrationUnits(db() as never, ME, HERE, 'tok', adapter([ACME], [], false).a);
		expect(out.complete).toBe(false);
	});

	it('fails closed: a failed forge read or a failed table read throws, never an empty list', async () => {
		const broken: ForgeAdapter = {
			...adapter([]).a,
			listUnits: async () => {
				throw new ForgeReadError(401, 'dead token');
			}
		};
		await expect(listIntegrationUnits(db() as never, ME, HERE, 'tok', broken)).rejects.toBeInstanceOf(ForgeReadError);
		const d = db({ forge_unit_mappings: { rows: [], error: { message: 'mappings down' } } });
		await expect(listIntegrationUnits(d as never, ME, HERE, 'tok', adapter([ACME]).a)).rejects.toThrow('mappings down');
	});
});

describe('claimIntegrationUnit', () => {
	it('proves the admin with the person`s token, makes the dōjō if absent, and claims it', async () => {
		const d = db();
		const { a, proved } = adapter([ACME], ['22']);
		const out = await claimIntegrationUnit(d as never, ME, 'tok', a, '22');
		expect(proved).toEqual(['22']);
		expect(out.dojo.key).toBe('organization/acme');
		const conn = d.tables.tenant_connections.rows.find((c) => c.external_id === '22');
		expect(conn).toBeDefined();
		expect(d.tables.memberships.rows.find((m) => m.tenant_id === conn!.tenant_id && m.user_id === ME)?.role).toBe('admin');
		expect(d.rpcCalls.filter((c) => c.fn === 'claim_forge_unit')).toEqual([
			{ fn: 'claim_forge_unit', args: { p_principal: ME, p_provider: 'github', p_external_id: '22' } }
		]);
	});

	it('claims an org`s dōjo even when the org is mapped as a workspace elsewhere — the claim is the admin`s', async () => {
		const d = db({
			forge_unit_mappings: {
				rows: [{ id: 'fm', provider: 'github', external_id: '22', external_slug: 'acme', tenant_id: HERE, workspace_id: 'w', proven_at: null }]
			}
		});
		const out = await claimIntegrationUnit(d as never, ME, 'tok', adapter([ACME], ['22']).a, '22');
		expect(out.dojo.key).toBe('organization/acme');
	});

	it('refuses a member with 403 and writes nothing', async () => {
		const d = db();
		const err = await claimIntegrationUnit(d as never, ME, 'tok', adapter([BETA]).a, '33').catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(403);
		expect(d.tables.tenant_connections.rows).toEqual([]);
		expect(d.rpcCalls).toEqual([]);
	});

	it('refuses an org the token cannot see with 404', async () => {
		const err = await claimIntegrationUnit(db() as never, ME, 'tok', adapter([ACME], ['22']).a, '99').catch((e) => e);
		expect(err.status).toBe(404);
	});

	it('writes nothing when the admin proof cannot be read', async () => {
		const d = db();
		const a: ForgeAdapter = {
			...adapter([ACME]).a,
			proveAdmin: async () => {
				throw new ForgeReadError(403, 'org restricts third-party access');
			}
		};
		await expect(claimIntegrationUnit(d as never, ME, 'tok', a, '22')).rejects.toBeInstanceOf(ForgeReadError);
		expect(d.tables.tenant_connections.rows).toEqual([]);
	});
});

describe('mapIntegrationUnit', () => {
	const maps = (d: ReturnType<typeof db>) => d.rpcCalls.filter((c) => c.fn === 'map_forge_unit');

	it('files an org as a workspace of THIS dōjō, named after the org, saying whether the admin was proven', async () => {
		const d = db();
		const { a, proved } = adapter([BETA]);
		const out = await mapIntegrationUnit(d as never, ME, HERE, 'tok', a, '33', { as: 'workspace' });
		expect(proved).toEqual(['33']);
		expect(maps(d)).toEqual([
			{
				fn: 'map_forge_unit',
				args: {
					p_principal: ME,
					p_provider: 'github',
					p_external_id: '33',
					p_external_slug: 'beta',
					p_tenant: HERE,
					p_workspace_slug: 'beta',
					p_workspace_name: 'beta',
					p_proven: false
				}
			}
		]);
		expect(out).toEqual({ tenantId: HERE, workspaceId: 'w-1' });
	});

	it('takes a workspace name, and derives the slug from it', async () => {
		const d = db();
		await mapIntegrationUnit(d as never, ME, HERE, 'tok', adapter([ACME], ['22']).a, '22', { as: 'workspace', name: 'Acme Web' });
		expect(maps(d)[0].args).toMatchObject({ p_workspace_slug: 'acme-web', p_workspace_name: 'Acme Web', p_proven: true });
	});

	it('makes an org its own dōjō only for a proven admin — creating the dōjō if the org has none', async () => {
		const d = db();
		await mapIntegrationUnit(d as never, ME, HERE, 'tok', adapter([ACME], ['22']).a, '22', { as: 'dojo' });
		const own = d.tables.tenant_connections.rows.find((c) => c.external_id === '22')?.tenant_id;
		expect(own).toBeDefined();
		expect(maps(d)[0].args).toMatchObject({
			p_tenant: own,
			p_workspace_slug: null,
			p_workspace_name: null,
			p_proven: true
		});

		const member = db();
		const err = await mapIntegrationUnit(member as never, ME, HERE, 'tok', adapter([BETA]).a, '33', { as: 'dojo' }).catch((e) => e);
		expect(err.status).toBe(403);
		expect(member.rpcCalls).toEqual([]);
		expect(member.tables.tenant_connections.rows).toEqual([]);
	});

	it('keeps the database`s refusal as its status and reason', async () => {
		const d = db({ 'rpc:map_forge_unit': { rows: [], error: { code: '42501', message: 'only an admin of the organization can move a claimed organization`s repositories' } } });
		const err = await mapIntegrationUnit(d as never, ME, HERE, 'tok', adapter([BETA]).a, '33', { as: 'workspace' }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(403);
		expect(err.message).toMatch(/claimed organization/);
	});

	it('refuses an org the token cannot see with 404, before asking anything else', async () => {
		const { a, proved } = adapter([ACME], ['22']);
		const err = await mapIntegrationUnit(db() as never, ME, HERE, 'tok', a, '99', { as: 'workspace' }).catch((e) => e);
		expect(err.status).toBe(404);
		expect(proved).toEqual([]);
	});

	it('refuses a workspace name that leaves nothing to slug', async () => {
		const err = await mapIntegrationUnit(db() as never, ME, HERE, 'tok', adapter([ACME], ['22']).a, '22', { as: 'workspace', name: '***' }).catch((e) => e);
		expect(err.status).toBe(400);
	});
});
