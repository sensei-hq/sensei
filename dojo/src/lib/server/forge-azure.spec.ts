// The Azure DevOps adapter (#308 R5b, D35/D36): an organization and its projects
// behind the one forge interface, read with the admin's pasted PAT.
//
// Fail closed, as forge-github is: a read that could not be made THROWS, a
// partial member read says `complete: false`, and nothing here may reach a host
// other than Azure DevOps' own API hosts or carry an org name it did not
// validate. The token travels only in the Authorization header.
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import { ForgeReadError } from './forge-github';
import { azureAdapter, azureOrgFromUrl, readAzureConnection, AZURE_REQUIRED_SCOPES } from './forge-azure';
import { fakeFetch, type FakeRoute } from './fake-forge-fetch';
import * as F from './forge-azure.fixtures';

const PAT = 'ztestpat0000000000000000000000000000000000000000AZDOabcd';
const UNIT = { id: F.ORG_ID, login: F.ORG };
const basic = `Basic ${btoa(':' + PAT)}`;

function adminRoutes(over: Record<string, FakeRoute> = {}): Record<string, FakeRoute> {
	return {
		...over,
		'/_apis/connectionData': { body: F.connectionData() },
		'continuationToken=groups-2': { body: F.groupsPage2 },
		'/_apis/graph/groups': { body: F.groupsPage1, headers: { 'content-type': 'application/json', 'x-ms-continuationtoken': 'groups-2' } }
	};
}

describe('azureOrgFromUrl', () => {
	it('reads the organization from both URL forms', () => {
		expect(azureOrgFromUrl('https://dev.azure.com/example-org')).toBe('example-org');
		expect(azureOrgFromUrl('https://dev.azure.com/Example-Org/Some%20Project/_git/api')).toBe('example-org');
		expect(azureOrgFromUrl('  https://example-org.visualstudio.com/  ')).toBe('example-org');
		expect(azureOrgFromUrl('dev.azure.com/example-org')).toBe('example-org');
	});

	it('refuses anything that is not an Azure DevOps organization URL', () => {
		for (const bad of [
			'',
			'http://dev.azure.com/example-org',
			'https://dev.azure.com/',
			'https://dev.azure.com.attacker.net/example-org',
			'https://attacker.net/dev.azure.com/example-org',
			'https://evil.dev.azure.com/example-org',
			'https://dev.azure.com/..',
			'https://dev.azure.com/a%2Fb',
			'https://dev.azure.com/-leading-hyphen',
			'https://dev.azure.com/' + 'a'.repeat(51),
			'https://vs-ssh.visualstudio.com/example-org',
			'https://a.b.visualstudio.com/',
			// userinfo in the URL (built in two parts: the leak guard reads `x@host` as an email)
			'https://user:pw' + '@dev.azure.com/example-org',
			'https://dev.azure.com:8443/example-org'
		]) {
			expect(() => azureOrgFromUrl(bad), bad).toThrow(AdminError);
		}
	});
});

describe('the Azure adapter, before any request', () => {
	it('refuses an organization name that could change the URL', () => {
		for (const bad of ['a/b', '..', 'a?b', 'a#b', 'a b', '', 'a.b', 'x'.repeat(51)]) {
			expect(() => azureAdapter(bad, fakeFetch({}).fn), bad).toThrow(AdminError);
		}
	});

	it('refuses a token that could not be a PAT, without asking Azure', async () => {
		const { fn, calls } = fakeFetch(adminRoutes());
		const a = azureAdapter(F.ORG, fn);
		for (const bad of ['', '   ', 'has space', 'line\nbreak', 'x'.repeat(513)]) {
			await expect(a.listUnits(bad), JSON.stringify(bad)).rejects.toMatchObject({ status: 400 });
		}
		expect(calls).toHaveLength(0);
	});

	it('names the read-only scopes the token needs', () => {
		expect(AZURE_REQUIRED_SCOPES.map((s) => s.scope)).toEqual([
			'vso.project',
			'vso.graph',
			'vso.memberentitlementmanagement',
			'vso.code'
		]);
	});
});

describe('readAzureConnection', () => {
	it('reads the organization id and the token owner, by Basic auth to the org host only', async () => {
		const { fn, calls } = fakeFetch({ '/_apis/connectionData': { body: F.connectionData() } });
		expect(await readAzureConnection(F.ORG, PAT, fn)).toEqual({
			orgId: F.ORG_ID,
			user: { id: F.ME_ID, descriptor: F.ME_DESCRIPTOR, displayName: 'Example Admin', active: true }
		});
		expect(calls[0].url).toBe('https://dev.azure.com/example-org/_apis/connectionData');
		expect(calls[0].url).not.toContain(PAT);
		const init = calls[0].init as RequestInit;
		expect((init.headers as Record<string, string>).Authorization).toBe(basic);
		// A redirect is Azure's answer to a bad token (a sign-in page) — never followed.
		expect(init.redirect).toBe('manual');
	});

	it('throws on a rejected token, keeping the forge status, and never names the token', async () => {
		const { fn } = fakeFetch({ '/_apis/connectionData': { status: 401, body: {} } });
		const err = await readAzureConnection(F.ORG, PAT, fn).catch((e) => e);
		expect(err).toBeInstanceOf(ForgeReadError);
		expect(err.forgeStatus).toBe(401);
		expect(String(err.message)).not.toContain(PAT);
	});

	it('throws on the 203 sign-in page Azure answers a bad token with, and on any non-JSON answer', async () => {
		const page = fakeFetch({ '/_apis/connectionData': { status: 203, body: '<html>', headers: { 'content-type': 'text/html' } } });
		await expect(readAzureConnection(F.ORG, PAT, page.fn)).rejects.toBeInstanceOf(ForgeReadError);
		const html = fakeFetch({ '/_apis/connectionData': { body: '<html>', headers: { 'content-type': 'text/html' } } });
		await expect(readAzureConnection(F.ORG, PAT, html.fn)).rejects.toBeInstanceOf(ForgeReadError);
		const redirect = fakeFetch({ '/_apis/connectionData': { status: 302, body: null } });
		await expect(readAzureConnection(F.ORG, PAT, redirect.fn)).rejects.toBeInstanceOf(ForgeReadError);
	});

	it('throws when the answer has no organization id or no user id — never a guessed one', async () => {
		const noOrg = fakeFetch({ '/_apis/connectionData': { body: F.connectionData({ instanceId: undefined }) } });
		await expect(readAzureConnection(F.ORG, PAT, noOrg.fn)).rejects.toBeInstanceOf(AdminError);
		const noUser = fakeFetch({ '/_apis/connectionData': { body: F.connectionData({}, { id: undefined }) } });
		await expect(readAzureConnection(F.ORG, PAT, noUser.fn)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('connection', () => {
	it('is the adapter\'s organization read: its stable id and the token owner', async () => {
		const { fn, calls } = fakeFetch({ '/_apis/connectionData': { body: F.connectionData() } });
		const conn = await azureAdapter(F.ORG, fn).connection(PAT);
		expect(conn.orgId).toBe(F.ORG_ID);
		expect(conn.user.id).toBe(F.ME_ID);
		expect(calls[0].url).toBe('https://dev.azure.com/example-org/_apis/connectionData');
	});
});

describe('listUnits', () => {
	it('is the bound organization, by its stable id, with the person proven an admin', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.PCA_DESCRIPTOR]) } }));
		expect(await azureAdapter(F.ORG, fn).listUnits(PAT)).toEqual({
			units: [{ id: F.ORG_ID, login: F.ORG, role: 'admin' }],
			complete: true
		});
	});

	it('names a member a member', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.OTHER_GROUP]) }, [`Memberships/${F.OTHER_GROUP}`]: { body: F.memberships(F.OTHER_GROUP, []) } }));
		const read = await azureAdapter(F.ORG, fn).listUnits(PAT);
		expect(read.units[0].role).toBe('member');
	});

	it('is empty — a positive answer — when the token owner is not active in the organization', async () => {
		const { fn } = fakeFetch({ '/_apis/connectionData': { body: F.connectionData({}, { isActive: false }) } });
		expect(await azureAdapter(F.ORG, fn).listUnits(PAT)).toEqual({ units: [], complete: true });
	});
});

describe('proveAdmin (Project Collection Administrators, via the Graph API)', () => {
	it('is true for a direct member, finding the group across pages of groups', async () => {
		const { fn, calls } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.PCA_DESCRIPTOR]) } }));
		expect(await azureAdapter(F.ORG, fn).proveAdmin(PAT, UNIT)).toBe(true);
		const hosts = new Set(calls.map((c) => new URL(c.url).host));
		expect([...hosts].sort()).toEqual(['dev.azure.com', 'vssps.dev.azure.com']);
		expect(calls.some((c) => c.url.includes('continuationToken=groups-2'))).toBe(true);
		for (const c of calls) expect((c.init?.headers as Record<string, string>).Authorization).toBe(basic);
	});

	it('is true through a group that is itself in Project Collection Administrators', async () => {
		const { fn } = fakeFetch(
			adminRoutes({
				[`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.OTHER_GROUP, F.NESTED_GROUP]) },
				[`Memberships/${F.OTHER_GROUP}`]: { body: F.memberships(F.OTHER_GROUP, []) },
				[`Memberships/${F.NESTED_GROUP}`]: { body: F.memberships(F.NESTED_GROUP, [F.PCA_DESCRIPTOR]) }
			})
		);
		expect(await azureAdapter(F.ORG, fn).proveAdmin(PAT, UNIT)).toBe(true);
	});

	it('is false for a member of no admin group', async () => {
		const { fn } = fakeFetch(
			adminRoutes({
				[`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.OTHER_GROUP]) },
				[`Memberships/${F.OTHER_GROUP}`]: { body: F.memberships(F.OTHER_GROUP, []) }
			})
		);
		expect(await azureAdapter(F.ORG, fn).proveAdmin(PAT, UNIT)).toBe(false);
	});

	it('is false when the name now belongs to another organization (a different stable id)', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.PCA_DESCRIPTOR]) } }));
		expect(await azureAdapter(F.ORG, fn).proveAdmin(PAT, { id: 'another-org-id', login: F.ORG })).toBe(false);
	});

	it('throws — never false — when the admin group cannot be found or a read fails', async () => {
		const noGroup = fakeFetch({
			'/_apis/connectionData': { body: F.connectionData() },
			'/_apis/graph/groups': { body: F.groupsPage1 }
		});
		await expect(azureAdapter(F.ORG, noGroup.fn).proveAdmin(PAT, UNIT)).rejects.toBeInstanceOf(AdminError);
		const failed = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { status: 500, body: {} } }));
		await expect(azureAdapter(F.ORG, failed.fn).proveAdmin(PAT, UNIT)).rejects.toBeInstanceOf(ForgeReadError);
	});

	it('throws on a membership graph too deep or cyclic to finish, rather than guessing', async () => {
		const many = Array.from({ length: 200 }, (_, i) => `vssgp.g${i}`);
		const { fn } = fakeFetch(
			adminRoutes({
				[`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, many) },
				'Memberships/vssgp.g': { body: F.memberships('vssgp.g', []) }
			})
		);
		await expect(azureAdapter(F.ORG, fn).proveAdmin(PAT, UNIT)).rejects.toBeInstanceOf(AdminError);
	});

	it('refuses a descriptor from Azure that could change the URL', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, ['../../evil']) } }));
		await expect(azureAdapter(F.ORG, fn).proveAdmin(PAT, UNIT)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('listMembers (user entitlements, paged by continuation token)', () => {
	it('reads every page, keyed by the identity id, and says it was complete', async () => {
		const { fn, calls } = fakeFetch({
			'continuationToken=page-2': { body: F.entitlementsPage2 },
			'/_apis/userentitlements': { body: F.entitlementsPage1 }
		});
		const read = await azureAdapter(F.ORG, fn).listMembers(PAT, UNIT);
		expect(read).toEqual({
			members: [
				{ id: F.ME_ID, login: 'admin@example.com' },
				{ id: '7c5a4b1e-0000-4000-8000-000000000002', login: 'dev@example.com' },
				{ id: '7c5a4b1e-0000-4000-8000-000000000003', login: 'ops@example.com' }
			],
			complete: true
		});
		expect(calls.map((c) => new URL(c.url).host)).toEqual(['vsaex.dev.azure.com', 'vsaex.dev.azure.com']);
	});

	it('reads the documented `items` key as well as the `members` key the service answers with', async () => {
		const { fn } = fakeFetch({
			'/_apis/userentitlements': { body: { items: F.entitlementsPage2.members, continuationToken: null } }
		});
		const read = await azureAdapter(F.ORG, fn).listMembers(PAT, UNIT);
		expect(read.members.map((m) => m.login)).toEqual(['ops@example.com']);
	});

	it('is incomplete when an entry cannot be named', async () => {
		const page = { members: [...F.entitlementsPage2.members, { user: { principalName: 'x@example.com' } }], continuationToken: null };
		const { fn } = fakeFetch({ '/_apis/userentitlements': { body: page } });
		expect((await azureAdapter(F.ORG, fn).listMembers(PAT, UNIT)).complete).toBe(false);
	});

	it('throws on a failed page, a shape it cannot read, or a token that repeats — never a short list', async () => {
		const failed = fakeFetch({
			'continuationToken=page-2': { status: 503, body: {} },
			'/_apis/userentitlements': { body: F.entitlementsPage1 }
		});
		await expect(azureAdapter(F.ORG, failed.fn).listMembers(PAT, UNIT)).rejects.toBeInstanceOf(ForgeReadError);
		const shape = fakeFetch({ '/_apis/userentitlements': { body: { totalCount: 3 } } });
		await expect(azureAdapter(F.ORG, shape.fn).listMembers(PAT, UNIT)).rejects.toBeInstanceOf(AdminError);
		const loop = fakeFetch({ '/_apis/userentitlements': { body: { ...F.entitlementsPage1, continuationToken: 'same' } } });
		await expect(azureAdapter(F.ORG, loop.fn).listMembers(PAT, UNIT)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('listProjects', () => {
	it('reads every page of the organization\'s projects, by stable id', async () => {
		const { fn, calls } = fakeFetch({
			'$skip=100': { body: F.projects(3, 100) },
			'/_apis/projects': { body: F.projects(100) }
		});
		const read = await azureAdapter(F.ORG, fn).listProjects(PAT, UNIT);
		expect(read.complete).toBe(true);
		expect(read.projects).toHaveLength(103);
		expect(read.projects[102]).toEqual({ id: '6ce954b1-ce1f-45d1-b94d-000000000102', name: 'Project 102' });
		expect(calls).toHaveLength(2);
		expect(calls[0].url.startsWith('https://dev.azure.com/example-org/_apis/projects?')).toBe(true);
	});

	it('is incomplete when a project has no id or name, and throws on a failed read', async () => {
		const page = { count: 1, value: [{ name: 'No Id' }] };
		const partial = fakeFetch({ '/_apis/projects': { body: page } });
		expect((await azureAdapter(F.ORG, partial.fn).listProjects(PAT, UNIT)).complete).toBe(false);
		const failed = fakeFetch({ '/_apis/projects': { status: 401, body: {} } });
		await expect(azureAdapter(F.ORG, failed.fn).listProjects(PAT, UNIT)).rejects.toMatchObject({ forgeStatus: 401 });
	});
});
