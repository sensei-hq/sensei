// The Azure DevOps adapter (#308 R5b, D35/D36): an organization and its projects
// behind the one forge interface, read with the admin's pasted ACCESS.
//
// Fail closed, as forge-github is: a read that could not be made THROWS, a
// partial member read says `complete: false`, and nothing here may reach a host
// other than Azure DevOps' own API hosts or carry an org name it did not
// validate. The token travels only in the Authorization header.
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import { ForgeReadError } from './forge-github';
import { azureAdapter, listAzureOrganizations, readAzureConnection, AZURE_REQUIRED_SCOPES } from './forge-azure';
import { fakeFetch, type FakeRoute } from './fake-forge-fetch';
import * as F from './forge-azure.fixtures';

// An Entra access token for the Azure DevOps resource (#308 R5b, D42) — a JWT, opaque here.
const ACCESS = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJ6dGVzdCI6InRva2VuIn0.c2lnbmF0dXJl';
const UNIT = { id: F.ORG_ID, login: F.ORG };
const basic = `Bearer ${ACCESS}`;

function adminRoutes(over: Record<string, FakeRoute> = {}): Record<string, FakeRoute> {
	// A test's own routes win over these (a later spread replaces a value in place).
	return {
		'/_apis/connectionData': { body: F.connectionData() },
		'continuationToken=groups-2': { body: F.groupsPage2 },
		'/_apis/graph/groups': { body: F.groupsPage1, headers: { 'content-type': 'application/json', 'x-ms-continuationtoken': 'groups-2' } },
		...over
	};
}

describe('the Azure adapter, before any request', () => {
	it('refuses an organization name that could change the URL', () => {
		for (const bad of ['a/b', '..', 'a?b', 'a#b', 'a b', '', 'a.b', 'x'.repeat(51)]) {
			expect(() => azureAdapter(bad, fakeFetch({}).fn), bad).toThrow(AdminError);
		}
	});

	it('refuses a token that could not be an access token, without asking Azure', async () => {
		const { fn, calls } = fakeFetch(adminRoutes());
		const a = azureAdapter(F.ORG, fn);
		for (const bad of ['', '   ', 'has space', 'line\nbreak', 'x'.repeat(16385)]) {
			await expect(a.listUnits(bad), JSON.stringify(bad)).rejects.toMatchObject({ status: 400 });
		}
		expect(calls).toHaveLength(0);
	});

	it('names the read-only scopes the connect asks for', () => {
		expect(AZURE_REQUIRED_SCOPES.map((s) => s.scope)).toEqual([
			'vso.profile',
			'vso.project',
			'vso.graph',
			'vso.memberentitlementmanagement',
			'vso.code'
		]);
	});
});

describe('readAzureConnection', () => {
	it('reads the organization id and the token owner, with the bearer token, from the org host only', async () => {
		const { fn, calls } = fakeFetch({ '/_apis/connectionData': { body: F.connectionData() } });
		expect(await readAzureConnection(F.ORG, ACCESS, fn)).toEqual({
			orgId: F.ORG_ID,
			user: { id: F.ME_ID, descriptor: F.ME_DESCRIPTOR, displayName: 'Example Admin', active: true }
		});
		expect(calls[0].url).toBe('https://dev.azure.com/example-org/_apis/connectionData');
		expect(calls[0].url).not.toContain(ACCESS);
		const init = calls[0].init as RequestInit;
		expect((init.headers as Record<string, string>).Authorization).toBe(basic);
		// A redirect is Azure's answer to a bad token (a sign-in page) — never followed.
		expect(init.redirect).toBe('manual');
	});

	it('throws on a rejected token, keeping the forge status, and never names the token', async () => {
		const { fn } = fakeFetch({ '/_apis/connectionData': { status: 401, body: {} } });
		const err = await readAzureConnection(F.ORG, ACCESS, fn).catch((e) => e);
		expect(err).toBeInstanceOf(ForgeReadError);
		expect(err.forgeStatus).toBe(401);
		expect(String(err.message)).not.toContain(ACCESS);
	});

	it('throws on the 203 sign-in page Azure answers a bad token with, and on any non-JSON answer', async () => {
		const page = fakeFetch({ '/_apis/connectionData': { status: 203, body: '<html>', headers: { 'content-type': 'text/html' } } });
		await expect(readAzureConnection(F.ORG, ACCESS, page.fn)).rejects.toBeInstanceOf(ForgeReadError);
		const html = fakeFetch({ '/_apis/connectionData': { body: '<html>', headers: { 'content-type': 'text/html' } } });
		await expect(readAzureConnection(F.ORG, ACCESS, html.fn)).rejects.toBeInstanceOf(ForgeReadError);
		const redirect = fakeFetch({ '/_apis/connectionData': { status: 302, body: null } });
		await expect(readAzureConnection(F.ORG, ACCESS, redirect.fn)).rejects.toBeInstanceOf(ForgeReadError);
	});

	it('throws when the answer has no organization id or no user id — never a guessed one', async () => {
		const noOrg = fakeFetch({ '/_apis/connectionData': { body: F.connectionData({ instanceId: undefined }) } });
		await expect(readAzureConnection(F.ORG, ACCESS, noOrg.fn)).rejects.toBeInstanceOf(AdminError);
		const noUser = fakeFetch({ '/_apis/connectionData': { body: F.connectionData({}, { id: undefined }) } });
		await expect(readAzureConnection(F.ORG, ACCESS, noUser.fn)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('connection', () => {
	it('is the adapter\'s organization read: its stable id and the token owner', async () => {
		const { fn, calls } = fakeFetch({ '/_apis/connectionData': { body: F.connectionData() } });
		const conn = await azureAdapter(F.ORG, fn).connection(ACCESS);
		expect(conn.orgId).toBe(F.ORG_ID);
		expect(conn.user.id).toBe(F.ME_ID);
		expect(calls[0].url).toBe('https://dev.azure.com/example-org/_apis/connectionData');
	});
});

describe('owner — the token owner by their Microsoft Entra object id (D45)', () => {
	it('reads the Graph user behind the token and keys it on originId', async () => {
		const { fn, calls } = fakeFetch({
			'/_apis/connectionData': { body: F.connectionData() },
			[`/_apis/graph/users/${F.ME_DESCRIPTOR}`]: { body: F.graphUser() }
		});
		expect(await azureAdapter(F.ORG, fn).owner(ACCESS)).toEqual({ objectId: F.ME_OID, displayName: 'Example Admin' });
		expect(calls[1].url).toBe(`https://vssps.dev.azure.com/example-org/_apis/graph/users/${F.ME_DESCRIPTOR}?api-version=7.1-preview.1`);
	});

	it('refuses an owner who is not a Microsoft Entra user, or has no object id', async () => {
		for (const user of [F.graphUser({ origin: 'msa' }), F.graphUser({ originId: undefined }), F.graphUser({ originId: 'not-a-guid' })]) {
			const { fn } = fakeFetch({
				'/_apis/connectionData': { body: F.connectionData() },
				[`/_apis/graph/users/${F.ME_DESCRIPTOR}`]: { body: user }
			});
			await expect(azureAdapter(F.ORG, fn).owner(ACCESS)).rejects.toBeInstanceOf(AdminError);
		}
	});
});

describe('listUnits', () => {
	it('is the bound organization, by its stable id, with the person proven an admin', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.PCA_DESCRIPTOR]) } }));
		expect(await azureAdapter(F.ORG, fn).listUnits(ACCESS)).toEqual({
			units: [{ id: F.ORG_ID, login: F.ORG, role: 'admin' }],
			complete: true
		});
	});

	it('names a member a member', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.OTHER_GROUP]) }, [`Memberships/${F.OTHER_GROUP}`]: { body: F.memberships(F.OTHER_GROUP, []) } }));
		const read = await azureAdapter(F.ORG, fn).listUnits(ACCESS);
		expect(read.units[0].role).toBe('member');
	});

	it('is empty — a positive answer — when the token owner is not active in the organization', async () => {
		const { fn } = fakeFetch({ '/_apis/connectionData': { body: F.connectionData({}, { isActive: false }) } });
		expect(await azureAdapter(F.ORG, fn).listUnits(ACCESS)).toEqual({ units: [], complete: true });
	});
});

describe('proveAdmin (Project Collection Administrators, via the Graph API)', () => {
	it('is true for a direct member, finding the group across pages of groups', async () => {
		const { fn, calls } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.PCA_DESCRIPTOR]) } }));
		expect(await azureAdapter(F.ORG, fn).proveAdmin(ACCESS, UNIT)).toBe(true);
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
		expect(await azureAdapter(F.ORG, fn).proveAdmin(ACCESS, UNIT)).toBe(true);
	});

	it('is false for a member of no admin group', async () => {
		const { fn } = fakeFetch(
			adminRoutes({
				[`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.OTHER_GROUP]) },
				[`Memberships/${F.OTHER_GROUP}`]: { body: F.memberships(F.OTHER_GROUP, []) }
			})
		);
		expect(await azureAdapter(F.ORG, fn).proveAdmin(ACCESS, UNIT)).toBe(false);
	});

	it('is false when the name now belongs to another organization (a different stable id)', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.PCA_DESCRIPTOR]) } }));
		expect(await azureAdapter(F.ORG, fn).proveAdmin(ACCESS, { id: 'another-org-id', login: F.ORG })).toBe(false);
	});

	it('throws — never false — when the admin group cannot be found or a read fails', async () => {
		const noGroup = fakeFetch({
			'/_apis/connectionData': { body: F.connectionData() },
			'/_apis/graph/groups': { body: F.groupsPage1 }
		});
		await expect(azureAdapter(F.ORG, noGroup.fn).proveAdmin(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);
		const failed = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { status: 500, body: {} } }));
		await expect(azureAdapter(F.ORG, failed.fn).proveAdmin(ACCESS, UNIT)).rejects.toBeInstanceOf(ForgeReadError);
	});

	it('takes the collection\'s own admin group, never a project group of the same name (#308 R5b review)', async () => {
		// A project named like the organization could hold a group whose principal
		// name reads `[org]\Project Collection Administrators`; it sits in the
		// project's scope (Classification/TeamProject), not the collection's.
		const impostor = {
			subjectKind: 'group',
			domain: 'vstfs:///Classification/TeamProject/ca97818a-3c86-4f95-b591-a4263b656b9e',
			principalName: `[${F.ORG}]\\Project Collection Administrators`,
			descriptor: F.OTHER_GROUP
		};
		const proven = fakeFetch(
			adminRoutes({
				'/_apis/graph/groups': { body: { count: 2, value: [impostor, ...F.groupsPage2.value] } },
				[`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, [F.OTHER_GROUP]) },
				[`Memberships/${F.OTHER_GROUP}`]: { body: F.memberships(F.OTHER_GROUP, []) }
			})
		);
		// Membership of the impostor group proves nothing.
		expect(await azureAdapter(F.ORG, proven.fn).proveAdmin(ACCESS, UNIT)).toBe(false);

		const onlyImpostor = fakeFetch(adminRoutes({ '/_apis/graph/groups': { body: { count: 1, value: [impostor] } } }));
		await expect(azureAdapter(F.ORG, onlyImpostor.fn).proveAdmin(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);

		const twice = fakeFetch(adminRoutes({ '/_apis/graph/groups': { body: { count: 2, value: [...F.groupsPage2.value, ...F.groupsPage2.value] } } }));
		await expect(azureAdapter(F.ORG, twice.fn).proveAdmin(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);
	});

	it('throws on a membership graph too deep or cyclic to finish, rather than guessing', async () => {
		const many = Array.from({ length: 200 }, (_, i) => `vssgp.g${i}`);
		const { fn } = fakeFetch(
			adminRoutes({
				[`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, many) },
				'Memberships/vssgp.g': { body: F.memberships('vssgp.g', []) }
			})
		);
		await expect(azureAdapter(F.ORG, fn).proveAdmin(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);
	});

	it('refuses a descriptor from Azure that could change the URL', async () => {
		const { fn } = fakeFetch(adminRoutes({ [`Memberships/${F.ME_DESCRIPTOR}`]: { body: F.memberships(F.ME_DESCRIPTOR, ['../../evil']) } }));
		await expect(azureAdapter(F.ORG, fn).proveAdmin(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('listMembers (user entitlements, paged by continuation token)', () => {
	it('reads every page, keyed by the identity id, and says it was complete', async () => {
		const { fn, calls } = fakeFetch({
			'continuationToken=page-2': { body: F.entitlementsPage2 },
			'/_apis/userentitlements': { body: F.entitlementsPage1 }
		});
		const read = await azureAdapter(F.ORG, fn).listMembers(ACCESS, UNIT);
		// Keyed by the Microsoft Entra object id (D43/D45) — what a Microsoft sign-in
		// (R5c) will also see — never by email.
		expect(read).toEqual({
			members: [
				{ id: F.ME_OID, login: 'admin@example.com' },
				{ id: 'a0e10000-0000-4000-8000-000000000002', login: 'dev@example.com' },
				{ id: 'a0e10000-0000-4000-8000-000000000003', login: 'ops@example.com' }
			],
			complete: true
		});
		expect(calls.map((c) => new URL(c.url).host)).toEqual(['vsaex.dev.azure.com', 'vsaex.dev.azure.com']);
	});

	it('reads the documented `items` key as well as the `members` key the service answers with', async () => {
		const { fn } = fakeFetch({
			'/_apis/userentitlements': { body: { items: F.entitlementsPage2.members, continuationToken: null } }
		});
		const read = await azureAdapter(F.ORG, fn).listMembers(ACCESS, UNIT);
		expect(read.members.map((m) => m.login)).toEqual(['ops@example.com']);
	});

	it('reads a continuation token sent as a one-item array (the shape earlier API versions use)', async () => {
		const { fn, calls } = fakeFetch({
			'continuationToken=page-2': { body: F.entitlementsPage2 },
			'/_apis/userentitlements': { body: { ...F.entitlementsPage1, continuationToken: ['page-2'] } }
		});
		const read = await azureAdapter(F.ORG, fn).listMembers(ACCESS, UNIT);
		expect(calls).toHaveLength(2);
		expect(read.members).toHaveLength(3);
		expect(read.complete).toBe(true);
		const last = fakeFetch({ '/_apis/userentitlements': { body: { ...F.entitlementsPage2, continuationToken: [] } } });
		expect((await azureAdapter(F.ORG, last.fn).listMembers(ACCESS, UNIT)).complete).toBe(true);
	});

	it('a continuation token it cannot read is never taken for the last page — it throws (#308 R5b review)', async () => {
		for (const token of [{ next: 'page-2' }, ['a', 'b'], [7], 42]) {
			const { fn } = fakeFetch({ '/_apis/userentitlements': { body: { ...F.entitlementsPage1, continuationToken: token } } });
			await expect(azureAdapter(F.ORG, fn).listMembers(ACCESS, UNIT), JSON.stringify(token)).rejects.toBeInstanceOf(AdminError);
		}
	});

	it('is incomplete when an Entra entry carries no object id, or an entry cannot be named', async () => {
		const noOid = { members: [...F.entitlementsPage2.members, { id: 'x', user: { principalName: 'x@example.com', origin: 'aad' } }], continuationToken: null };
		const a = fakeFetch({ '/_apis/userentitlements': { body: noOid } });
		expect((await azureAdapter(F.ORG, a.fn).listMembers(ACCESS, UNIT)).complete).toBe(false);
		const page = { members: [...F.entitlementsPage2.members, { user: { principalName: 'x@example.com' } }], continuationToken: null };
		const { fn } = fakeFetch({ '/_apis/userentitlements': { body: page } });
		expect((await azureAdapter(F.ORG, fn).listMembers(ACCESS, UNIT)).complete).toBe(false);
	});

	it('throws on a failed page, a shape it cannot read, or a token that repeats — never a short list', async () => {
		const failed = fakeFetch({
			'continuationToken=page-2': { status: 503, body: {} },
			'/_apis/userentitlements': { body: F.entitlementsPage1 }
		});
		await expect(azureAdapter(F.ORG, failed.fn).listMembers(ACCESS, UNIT)).rejects.toBeInstanceOf(ForgeReadError);
		const shape = fakeFetch({ '/_apis/userentitlements': { body: { totalCount: 3 } } });
		await expect(azureAdapter(F.ORG, shape.fn).listMembers(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);
		const loop = fakeFetch({ '/_apis/userentitlements': { body: { ...F.entitlementsPage1, continuationToken: 'same' } } });
		await expect(azureAdapter(F.ORG, loop.fn).listMembers(ACCESS, UNIT)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('listProjects', () => {
	it('reads every page of the organization\'s projects, by stable id', async () => {
		const { fn, calls } = fakeFetch({
			'$skip=100': { body: F.projects(3, 100) },
			'/_apis/projects': { body: F.projects(100) }
		});
		const read = await azureAdapter(F.ORG, fn).listProjects(ACCESS, UNIT);
		expect(read.complete).toBe(true);
		expect(read.projects).toHaveLength(103);
		expect(read.projects[102]).toEqual({ id: '6ce954b1-ce1f-45d1-b94d-000000000102', name: 'Project 102' });
		expect(calls).toHaveLength(2);
		expect(calls[0].url.startsWith('https://dev.azure.com/example-org/_apis/projects?')).toBe(true);
	});

	it('is incomplete when a project has no id or name, and throws on a failed read', async () => {
		const page = { count: 1, value: [{ name: 'No Id' }] };
		const partial = fakeFetch({ '/_apis/projects': { body: page } });
		expect((await azureAdapter(F.ORG, partial.fn).listProjects(ACCESS, UNIT)).complete).toBe(false);
		const failed = fakeFetch({ '/_apis/projects': { status: 401, body: {} } });
		await expect(azureAdapter(F.ORG, failed.fn).listProjects(ACCESS, UNIT)).rejects.toMatchObject({ forgeStatus: 401 });
	});
});

describe('listAzureOrganizations (Profiles + Accounts, vso.profile)', () => {
	// https://learn.microsoft.com/en-us/rest/api/azure/devops/profile/profiles/get?view=azure-devops-rest-7.1
	// https://learn.microsoft.com/en-us/rest/api/azure/devops/account/accounts/list?view=azure-devops-rest-7.1
	const me = { displayName: 'Example Admin', publicAlias: F.ME_ID, emailAddress: 'admin@example.com', id: F.ME_ID };
	const accounts = {
		count: 3,
		value: [
			{ accountId: '6affcbef-fe5d-44c6-8a29-439b79aaa118', accountUri: 'https://vssps.dev.azure.com/example-org/', accountName: 'Example-Org', properties: {} },
			{ accountId: 'bf83696f-383b-4980-95b5-1f1127640a8a', accountUri: 'https://vssps.dev.azure.com/second-org/', accountName: 'second-org', properties: {} },
			{ accountId: 'aaaaaaaa-0000-4000-8000-000000000000', accountName: 'not a name/../x', properties: {} }
		]
	};

	it('lists the organizations the token\'s owner belongs to, by name — and says when one could not be read', async () => {
		const { fn, calls } = fakeFetch({ '/_apis/profile/profiles/me': { body: me }, '/_apis/accounts': { body: accounts } });
		expect(await listAzureOrganizations(ACCESS, fn)).toEqual({
			organizations: [
				{ id: '6affcbef-fe5d-44c6-8a29-439b79aaa118', name: 'example-org' },
				{ id: 'bf83696f-383b-4980-95b5-1f1127640a8a', name: 'second-org' }
			],
			complete: false
		});
		expect(calls.map((c) => c.url)).toEqual([
			'https://app.vssps.visualstudio.com/_apis/profile/profiles/me?api-version=7.1',
			`https://app.vssps.visualstudio.com/_apis/accounts?memberId=${F.ME_ID}&api-version=7.1`
		]);
		for (const c of calls) expect((c.init?.headers as Record<string, string>).Authorization).toBe(basic);
	});

	it('throws on a failed read or a profile with no id — never an empty list', async () => {
		const failed = fakeFetch({ '/_apis/profile/profiles/me': { status: 401, body: {} } });
		await expect(listAzureOrganizations(ACCESS, failed.fn)).rejects.toBeInstanceOf(ForgeReadError);
		const noId = fakeFetch({ '/_apis/profile/profiles/me': { body: { displayName: 'x' } } });
		await expect(listAzureOrganizations(ACCESS, noId.fn)).rejects.toBeInstanceOf(AdminError);
		const notList = fakeFetch({ '/_apis/profile/profiles/me': { body: me }, '/_apis/accounts': { body: { count: 0 } } });
		await expect(listAzureOrganizations(ACCESS, notList.fn)).rejects.toBeInstanceOf(AdminError);
	});
});
