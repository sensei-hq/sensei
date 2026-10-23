// Importing an Azure DevOps organization (#308 R5b, D35/D36): the organization
// becomes a dōjō (or joins the org dōjō whose screen asked) and each project a
// workspace in it; the PAT is stored in Vault only after it lists the
// organization and proves the admin. A failing token is refused and nothing is
// written. The database functions hold their own rules (forge_token_vault.sql,
// forge_unit_claim.sql, forge_unit_mapping.sql, azure_project_filing.sql); this
// proves the server's half — what it reads, what it refuses, what it asks.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, provisioningTables, type FakeTable } from './fake-dojo-db';
import { AdminError } from './admin-data';
import { ForgeReadError } from './forge-github';
import type { AzureAdapter, AzureProject } from './forge-azure';
import { importAzureOrganization, syncAzureOrganization, connectRequest, listAzureConnections, removeAzureToken } from './azure-import';

const ME = 'p-me';
const HERE = 't-here';
const ORG = 'example-org';
const ORG_ID = 'org-guid-1';
const PAT = 'ztestpatAZDO0000';
const ME_AZ = 'az-user-1';

const P1: AzureProject = { id: 'proj-1', name: 'Mobile Apps' };
const P2: AzureProject = { id: 'proj-2', name: 'mobile-apps' };
const P3: AzureProject = { id: 'proj-3', name: 'Platform' };

interface Opts {
	admin?: boolean;
	active?: boolean;
	projects?: AzureProject[];
	complete?: boolean;
	reject?: number;
	members?: { id: string; login: string }[];
	membersComplete?: boolean;
	membersFail?: number;
}

function fakeAzure(o: Opts = {}) {
	const asked: string[] = [];
	const a: AzureAdapter = {
		provider: 'azure_devops',
		organization: ORG,
		connection: async (token) => {
			asked.push(`connection:${token === PAT}`);
			if (o.reject) throw new ForgeReadError(o.reject, 'Azure DevOps read failed');
			return { orgId: ORG_ID, user: { id: ME_AZ, descriptor: 'aad.me', displayName: 'Example Admin', active: o.active ?? true } };
		},
		listUnits: async () => ({ units: [], complete: true }),
		proveAdmin: async (_t, unit) => {
			asked.push(`prove:${unit.id}`);
			return o.admin ?? true;
		},
		listMembers: async () => {
			asked.push('members');
			if (o.membersFail) throw new ForgeReadError(o.membersFail, 'Azure DevOps read failed');
			return { members: o.members ?? [{ id: ME_AZ, login: 'admin@example.com' }, { id: 'az-user-2', login: 'dev@example.com' }], complete: o.membersComplete ?? true };
		},
		listProjects: async () => {
			asked.push('projects');
			return { projects: o.projects ?? [P1, P3], complete: o.complete ?? true };
		}
	};
	return { a, asked };
}

function db(seed: Partial<Record<string, FakeTable>> = {}) {
	return fakeDojoDb({
		...provisioningTables(seed),
		tenants: {
			rows: [
				{ id: HERE, key: 'organization/here', origin: 'organization', slug: 'here', name: 'Here' },
				{ id: 't-mine', key: 'personal/me', origin: 'personal', slug: 'me', name: 'Me' }
			],
			uniques: [{ columns: ['key'] }],
			...seed.tenants
		},
		memberships: {
			rows: [{ id: 'm-here', tenant_id: HERE, user_id: ME, role: 'admin', disabled_at: null }],
			uniques: [{ columns: ['tenant_id', 'user_id'] }],
			...seed.memberships
		},
		workspaces: { rows: [], ...seed.workspaces },
		'rpc:set_forge_token': { rows: [], ...seed['rpc:set_forge_token'] },
		'rpc:remove_forge_token': { rows: [], ...seed['rpc:remove_forge_token'] },
		'rpc:forge_token': { rows: [PAT as never], ...seed['rpc:forge_token'] },
		'rpc:mark_forge_token_rejected': { rows: [], ...seed['rpc:mark_forge_token_rejected'] },
		'rpc:map_forge_unit': { rows: [{ mapping_id: 'fm-1', tenant_id: HERE, workspace_id: 'w-1' }], ...seed['rpc:map_forge_unit'] },
		'rpc:reconcile_unit_members': { rows: [1 as never], ...seed['rpc:reconcile_unit_members'] }
	});
}

const writes = (d: ReturnType<typeof db>) => d.rpcCalls.map((c) => c.fn);

beforeEach(() => resetFakeIds());

describe('importAzureOrganization — from the personal dōjō (how an Azure organization\'s dōjō is first made)', () => {
	it('makes the organization\'s dōjō, claims it, stores the token and files each project as a workspace', async () => {
		// The fake numbers rows in order (resetFakeIds): the new org dōjō is `tena-2`,
		// and map_forge_unit answers that it filed there.
		const d = db({ 'rpc:map_forge_unit': { rows: [{ mapping_id: 'fm-1', tenant_id: 'tena-2', workspace_id: 'w-1' }] } });
		const { a } = fakeAzure();
		const out = await importAzureOrganization(d as never, ME, { as: 'own' }, PAT, a);

		const org = d.tables.tenants.rows.find((t) => t.origin === 'organization' && t.id !== HERE);
		expect(org).toMatchObject({ key: 'organization/example-org', name: ORG });
		expect(d.tables.tenant_connections.rows).toEqual([
			expect.objectContaining({ tenant_id: org?.id, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG })
		]);
		expect(d.tables.memberships.rows).toContainEqual(
			expect.objectContaining({
				tenant_id: org?.id,
				user_id: ME,
				role: 'admin',
				authenticated_via: 'azure_devops',
				source: 'provisioned',
				source_provider: 'azure_devops',
				source_unit: ORG_ID
			})
		);
		// The Azure account is recorded as this person's — what a member read keys on.
		expect(d.tables.identities.rows).toEqual([
			expect.objectContaining({ principal_id: ME, provider: 'azure_devops', subject: ME_AZ })
		]);

		const calls = d.rpcCalls;
		expect(calls.find((c) => c.fn === 'claim_forge_unit')?.args).toEqual({
			p_principal: ME,
			p_provider: 'azure_devops',
			p_external_id: ORG_ID
		});
		expect(calls.find((c) => c.fn === 'set_forge_token')?.args).toEqual({
			p_principal: ME,
			p_provider: 'azure_devops',
			p_external_id: ORG_ID,
			p_token: PAT,
			p_expires_at: null
		});
		const maps = calls.filter((c) => c.fn === 'map_forge_unit').map((c) => c.args);
		expect(maps).toEqual([
			{
				p_principal: ME,
				p_provider: 'azure_devops',
				p_external_id: 'proj-1',
				p_external_slug: 'example-org/Mobile Apps',
				p_tenant: org?.id,
				p_workspace_slug: 'mobile-apps',
				p_workspace_name: 'Mobile Apps',
				p_proven: true,
				p_keep_elsewhere: true
			},
			expect.objectContaining({ p_external_id: 'proj-3', p_external_slug: 'example-org/Platform', p_workspace_slug: 'platform' })
		]);

		expect(out).toEqual({
			dojo: { key: 'organization/example-org', created: true },
			organization: { id: ORG_ID, name: ORG },
			projects: { total: 2, mapped: 2, elsewhere: 0, complete: true },
			members: { complete: true, listed: 2, ended: 1 }
		});
		// The answer never carries the token.
		expect(JSON.stringify(out)).not.toContain(PAT);
	});

	it('the token is stored only after it lists the organization and proves the admin', async () => {
		const d = db();
		const { a, asked } = fakeAzure();
		await importAzureOrganization(d as never, ME, { as: 'own' }, PAT, a);
		expect(asked.slice(0, 3)).toEqual(['connection:true', `prove:${ORG_ID}`, 'projects']);
		const order = writes(d);
		expect(order.indexOf('set_forge_token')).toBeGreaterThan(order.indexOf('claim_forge_unit'));
	});

	it('refuses a member who is not a Project Collection Administrator — and writes nothing', async () => {
		const d = db();
		const err = await importAzureOrganization(d as never, ME, { as: 'own' }, PAT, fakeAzure({ admin: false }).a).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(403);
		expect(d.rpcCalls).toEqual([]);
		expect(d.tables.tenant_connections.rows).toEqual([]);
		expect(d.tables.identities.rows).toEqual([]);
		expect(d.tables.tenants.rows).toHaveLength(2);
	});

	it('refuses a token whose owner is not in the organization — and writes nothing', async () => {
		const d = db();
		const err = await importAzureOrganization(d as never, ME, { as: 'own' }, PAT, fakeAzure({ active: false }).a).catch((e) => e);
		expect(err.status).toBe(403);
		expect(d.rpcCalls).toEqual([]);
	});

	it('a token Azure rejects is refused (the forge status kept) and never stored', async () => {
		const d = db();
		const err = await importAzureOrganization(d as never, ME, { as: 'own' }, PAT, fakeAzure({ reject: 401 }).a).catch((e) => e);
		expect(err).toBeInstanceOf(ForgeReadError);
		expect(err.forgeStatus).toBe(401);
		expect(d.rpcCalls).toEqual([]);
		expect(d.tables.tenants.rows).toHaveLength(2);
	});

	it('joins an organization dōjō that already exists, as a proven admin', async () => {
		const d = db({
			tenants: {
				rows: [
					{ id: HERE, key: 'organization/here', origin: 'organization', slug: 'here', name: 'Here' },
					{ id: 't-az', key: 'organization/example-org', origin: 'organization', slug: 'example-org', name: ORG }
				]
			},
			tenant_connections: {
				rows: [{ id: 'c-az', tenant_id: 't-az', provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG }]
			}
		});
		const out = await importAzureOrganization(d as never, ME, { as: 'own' }, PAT, fakeAzure().a);
		expect(out.dojo).toEqual({ key: 'organization/example-org', created: false });
		expect(d.tables.memberships.rows).toContainEqual(expect.objectContaining({ tenant_id: 't-az', user_id: ME, role: 'admin' }));
	});
});

describe('importAzureOrganization — from an organization dōjō\'s Integrations', () => {
	it('connects the organization to THIS dōjō: one dōjō, known to GitHub and Azure', async () => {
		const d = db();
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure().a);
		expect(d.tables.tenant_connections.rows).toEqual([
			expect.objectContaining({ tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG, connected_by: ME })
		]);
		expect(d.tables.tenant_connections.rows[0].verified_at).toBeTruthy();
		expect(d.rpcCalls.filter((c) => c.fn === 'map_forge_unit').every((c) => c.args.p_tenant === HERE)).toBe(true);
		expect(out.dojo).toEqual({ key: 'organization/here', created: false });
	});

	it('refuses an organization another dōjō is connected to (409), before storing anything', async () => {
		const d = db({
			tenant_connections: {
				rows: [{ id: 'c-az', tenant_id: 't-other', provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG }]
			}
		});
		const err = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure().a).catch((e) => e);
		expect(err.status).toBe(409);
		expect(writes(d)).toEqual([]);
	});

	it('keeps a project mapped elsewhere, keeps the workspace of one mapped here, and never repeats a slug', async () => {
		const d = db({
			forge_unit_mappings: {
				rows: [
					{ provider: 'azure_devops', external_id: 'proj-1', external_slug: 'example-org/Old Name', tenant_id: HERE, workspace_id: 'w-kept' },
					{ provider: 'azure_devops', external_id: 'proj-3', external_slug: 'example-org/Platform', tenant_id: 't-other', workspace_id: 'w-away' }
				]
			},
			workspaces: { rows: [{ id: 'w-kept', tenant_id: HERE, slug: 'apps', name: 'Apps' }] }
		});
		const out = await importAzureOrganization(
			d as never,
			ME,
			{ as: 'this', tenantId: HERE },
			PAT,
			fakeAzure({ projects: [P1, P2, P3, { id: 'proj-4', name: 'Mobile Apps!' }] }).a
		);
		const maps = d.rpcCalls.filter((c) => c.fn === 'map_forge_unit').map((c) => c.args);
		expect(maps.map((m) => [m.p_external_id, m.p_external_slug, m.p_workspace_slug, m.p_workspace_name])).toEqual([
			// Renamed upstream: the same workspace, under the project's new name.
			['proj-1', 'example-org/Mobile Apps', 'apps', 'Apps'],
			['proj-2', 'example-org/mobile-apps', 'mobile-apps', 'mobile-apps'],
			['proj-4', 'example-org/Mobile Apps!', 'mobile-apps-2', 'Mobile Apps!']
		]);
		expect(out.projects).toEqual({ total: 4, mapped: 3, elsewhere: 1, complete: true });
	});

	it('counts a project the database kept in another dōjō (a re-map that raced the import)', async () => {
		const d = db({ 'rpc:map_forge_unit': { rows: [{ mapping_id: 'fm-9', tenant_id: 't-other', workspace_id: 'w-away' }] } });
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure({ projects: [P1] }).a);
		expect(d.rpcCalls.find((c) => c.fn === 'map_forge_unit')?.args.p_keep_elsewhere).toBe(true);
		expect(out.projects).toEqual({ total: 1, mapped: 0, elsewhere: 1, complete: true });
	});

	it('says when the project read was not complete', async () => {
		const d = db();
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure({ complete: false }).a);
		expect(out.projects.complete).toBe(false);
	});
});

describe('connectRequest', () => {
	it('reads the organization URL and the token, trimmed', async () => {
		const req = new Request('http://x/', {
			method: 'POST',
			body: JSON.stringify({ organization_url: ' https://dev.azure.com/example-org ', token: ` ${PAT} ` })
		});
		expect(await connectRequest(req)).toEqual({ organization: ORG, token: PAT });
	});

	it('is a 400 without either, naming neither value', async () => {
		for (const body of [{}, { organization_url: 'https://dev.azure.com/example-org' }, { token: PAT }, { organization_url: 'https://example.com/x', token: PAT }]) {
			const err = await connectRequest(new Request('http://x/', { method: 'POST', body: JSON.stringify(body) })).catch((e) => e);
			expect(err).toBeInstanceOf(AdminError);
			expect(err.status).toBe(400);
			expect(String(err.message)).not.toContain(PAT);
		}
	});
});

describe('listAzureConnections', () => {
	it('lists this dōjō\'s Azure organizations and their token state — never a secret id or a token', async () => {
		const d = db({
			tenant_connections: {
				rows: [
					{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG, verified_at: '2026-10-22T00:00:00Z', token_secret_id: 'sec-1', token_state: 'needs_new_token', token_expires_at: null },
					{ id: 'c-gh', tenant_id: HERE, provider: 'github', external_id: '22', external_slug: 'acme' }
				]
			}
		});
		expect(await listAzureConnections(d as never, HERE)).toEqual([
			{ id: ORG_ID, name: ORG, verifiedAt: '2026-10-22T00:00:00Z', token: 'needs_new_token', tokenExpiresAt: null }
		]);
	});

	it('says a connection without a stored token has none', async () => {
		const d = db({
			tenant_connections: { rows: [{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG, token_secret_id: null, token_state: null }] }
		});
		expect((await listAzureConnections(d as never, HERE))[0].token).toBe('none');
	});

	it('throws on a failed read — never an empty list', async () => {
		const d = db({ tenant_connections: { rows: [], error: { message: 'boom' } } });
		await expect(listAzureConnections(d as never, HERE)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('removeAzureToken', () => {
	it('removes the token of one of this dōjō\'s Azure organizations, as the admin', async () => {
		const d = db({
			tenant_connections: { rows: [{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG }] }
		});
		await removeAzureToken(d as never, ME, HERE, ORG_ID);
		expect(d.rpcCalls).toEqual([{ fn: 'remove_forge_token', args: { p_principal: ME, p_provider: 'azure_devops', p_external_id: ORG_ID } }]);
	});

	it('is a 404 for an organization this dōjō is not connected to', async () => {
		const d = db();
		const err = await removeAzureToken(d as never, ME, HERE, ORG_ID).catch((e) => e);
		expect(err.status).toBe(404);
		expect(d.rpcCalls).toEqual([]);
	});
});

describe('syncAzureOrganization — re-read with the stored token', () => {
	const connected = () =>
		db({
			tenant_connections: {
				rows: [{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG, token_secret_id: 'sec-1', token_state: 'ok' }]
			}
		});

	it('reads the token from Vault, re-proves the admin and files the projects — without storing the token again', async () => {
		const d = connected();
		const { a, asked } = fakeAzure();
		const out = await syncAzureOrganization(d as never, ME, HERE, ORG_ID, (org) => {
			expect(org).toBe(ORG);
			return a;
		});
		expect(asked).toContain(`prove:${ORG_ID}`);
		expect(d.rpcCalls[0]).toEqual({ fn: 'forge_token', args: { p_provider: 'azure_devops', p_external_id: ORG_ID } });
		expect(writes(d)).not.toContain('set_forge_token');
		expect(out.projects).toEqual({ total: 2, mapped: 2, elsewhere: 0, complete: true });
		expect(out.members).toEqual({ complete: true, listed: 2, ended: 1 });
		expect(JSON.stringify(out)).not.toContain(PAT);
	});

	it('marks a token Azure rejects as needing a new one, and stops (409)', async () => {
		const d = connected();
		const err = await syncAzureOrganization(d as never, ME, HERE, ORG_ID, () => fakeAzure({ reject: 401 }).a).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(409);
		expect(d.rpcCalls.map((c) => c.fn)).toEqual(['forge_token', 'mark_forge_token_rejected']);
	});

	it('keeps an outage an outage: no mark, the read error thrown', async () => {
		const d = connected();
		const err = await syncAzureOrganization(d as never, ME, HERE, ORG_ID, () => fakeAzure({ reject: 503 }).a).catch((e) => e);
		expect(err).toBeInstanceOf(ForgeReadError);
		expect(writes(d)).toEqual(['forge_token']);
	});

	it('keeps the database\'s refusal of a token already marked (409) or missing (404)', async () => {
		const marked = db({
			tenant_connections: { rows: [{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG }] },
			'rpc:forge_token': { rows: [], error: { code: '55000', message: 'the stored token was rejected; an admin must replace it' } }
		});
		const err = await syncAzureOrganization(marked as never, ME, HERE, ORG_ID, () => fakeAzure().a).catch((e) => e);
		expect(err.status).toBe(409);
	});

	it('refuses when the organization name now names another organization', async () => {
		const d = db({
			tenant_connections: { rows: [{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: 'org-guid-OLD', external_slug: ORG, token_secret_id: 's' }] }
		});
		const err = await syncAzureOrganization(d as never, ME, HERE, 'org-guid-OLD', () => fakeAzure().a).catch((e) => e);
		expect(err.status).toBe(409);
		expect(writes(d)).not.toContain('map_forge_unit');
	});

	it('is a 404 for an organization this dōjō is not connected to', async () => {
		const err = await syncAzureOrganization(db() as never, ME, HERE, ORG_ID, () => fakeAzure().a).catch((e) => e);
		expect(err.status).toBe(404);
	});
});

describe('members — #309\'s rule, read from the organization\'s side', () => {
	it('a complete read ends the provisioned members it no longer lists (the database decides who)', async () => {
		const d = db();
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure().a);
		const call = d.rpcCalls.find((c) => c.fn === 'reconcile_unit_members');
		expect(call?.args).toEqual({
			p_tenant: HERE,
			p_provider: 'azure_devops',
			p_unit: ORG_ID,
			p_auth: 'azure_devops',
			p_subjects: [ME_AZ, 'az-user-2']
		});
		expect(out.members).toEqual({ complete: true, listed: 2, ended: 1 });
	});

	it('a partial read ends nobody — and says so', async () => {
		const d = db();
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure({ membersComplete: false }).a);
		expect(d.rpcCalls.map((c) => c.fn)).not.toContain('reconcile_unit_members');
		expect(out.members).toEqual({ complete: false, listed: 2, ended: 0 });
	});

	it('a failed member read ends nobody and is reported — the import itself stands', async () => {
		const d = db();
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure({ membersFail: 503 }).a);
		expect(d.rpcCalls.map((c) => c.fn)).not.toContain('reconcile_unit_members');
		expect(d.rpcCalls.map((c) => c.fn)).toContain('set_forge_token');
		expect(out.members).toEqual({ error: 'Azure DevOps could not list the members (503); nobody was ended' });
	});

	it('a read that lists nobody is not a complete answer — nobody is ended', async () => {
		const d = db();
		const out = await importAzureOrganization(d as never, ME, { as: 'this', tenantId: HERE }, PAT, fakeAzure({ members: [] }).a);
		expect(d.rpcCalls.map((c) => c.fn)).not.toContain('reconcile_unit_members');
		expect(out.members).toEqual({ complete: false, listed: 0, ended: 0 });
	});

	it('sync reads the members too, with the stored token', async () => {
		const d = db({
			tenant_connections: {
				rows: [{ id: 'c-az', tenant_id: HERE, provider: 'azure_devops', external_id: ORG_ID, external_slug: ORG, token_secret_id: 's', token_state: 'ok' }]
			}
		});
		const { a, asked } = fakeAzure();
		const out = await syncAzureOrganization(d as never, ME, HERE, ORG_ID, () => a);
		expect(asked).toContain('members');
		expect(out.members).toEqual({ complete: true, listed: 2, ended: 1 });
	});
});
