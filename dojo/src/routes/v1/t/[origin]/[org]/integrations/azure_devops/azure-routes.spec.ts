// Route tests for Azure DevOps on Admin → Integrations (#308 R5b, D35/D36), and
// the personal dōjō's import (/v1/you/integrations/azure_devops/connect). ADMIN
// floor of the screen's dōjō; the PAT arrives in the connect body only, is handed
// to the import and NEVER comes back — not in an answer, not in an error. A token
// Azure will not take is a 422 that names no token; an outage is a 502. The data
// layer is mocked — its own logic is in azure-import.spec.ts.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ForgeReadError } from '$lib/server/forge-github';

const { AdminError } = vi.hoisted(() => ({
	AdminError: class AdminError extends Error {
		constructor(
			readonly status: number,
			message: string
		) {
			super(message);
		}
	}
}));

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	resolveCaller: vi.fn(),
	importAzureOrganization: vi.fn(),
	syncAzureOrganization: vi.fn(),
	listAzureConnections: vi.fn(),
	removeAzureToken: vi.fn(),
	azureAdapter: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token }),
	serverSecret: () => 'route-test-secret'
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/admin-data', async (orig) => ({
	...(await orig<typeof import('$lib/server/admin-data')>()),
	AdminError
}));
vi.mock('$lib/server/azure-import', async (orig) => ({
	...(await orig<typeof import('$lib/server/azure-import')>()),
	importAzureOrganization: mocks.importAzureOrganization,
	syncAzureOrganization: mocks.syncAzureOrganization,
	listAzureConnections: mocks.listAzureConnections,
	removeAzureToken: mocks.removeAzureToken
}));
vi.mock('$lib/server/forge-azure', async (orig) => ({
	...(await orig<typeof import('$lib/server/forge-azure')>()),
	azureAdapter: mocks.azureAdapter
}));

const status = await import('./+server');
const connect = await import('./connect/+server');
const sync = await import('./sync/+server');
const token = await import('./token/+server');
const personal = await import('../../../../../you/integrations/azure_devops/connect/+server');

const PAT = 'ztestpatAZDO0000secret';
const SERVICE = { client: 'service' };
const URL_ = 'https://dev.azure.com/example-org';

const ev = (body: unknown, method = 'POST', origin = 'organization') =>
	({
		params: { origin, org: 'acme' },
		request: new Request('http://x/', {
			method,
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: method === 'GET' ? undefined : JSON.stringify(body)
		}),
		cookies: { get: () => undefined },
		locals: {},
		url: new URL('http://x/')
	}) as never;

const IMPORTED = {
	dojo: { key: 'organization/acme', created: false },
	organization: { id: 'org-1', name: 'example-org' },
	projects: { total: 2, mapped: 2, elsewhere: 0, complete: true },
	members: { complete: true, listed: 3, ended: 0 }
};

beforeEach(() => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({ tenantId: 't1', userId: 'p-admin', role: 'admin', access: 4, accessToken: 'jwt-A' });
	mocks.resolveCaller.mockResolvedValue({ userId: 'p-me', authUserId: 'a-me', email: 'me@example.com', accessToken: 'jwt', githubId: null });
	mocks.azureAdapter.mockImplementation((org: string) => ({ provider: 'azure_devops', organization: org }));
	mocks.importAzureOrganization.mockResolvedValue(IMPORTED);
	mocks.syncAzureOrganization.mockResolvedValue({ organization: IMPORTED.organization, projects: IMPORTED.projects });
	mocks.listAzureConnections.mockResolvedValue([]);
	mocks.removeAzureToken.mockResolvedValue(undefined);
});

describe('Azure DevOps on an organization dōjō', () => {
	it('every route asks for the admin floor of the dōjō', async () => {
		await status.GET(ev(undefined, 'GET'));
		await connect.POST(ev({ organization_url: URL_, token: PAT }));
		await sync.POST(ev({ organization_id: 'org-1' }));
		await token.DELETE(ev({ organization_id: 'org-1' }, 'DELETE'));
		expect(mocks.resolveTenantAccess).toHaveBeenCalledTimes(4);
		for (const call of mocks.resolveTenantAccess.mock.calls) expect(call[4]).toBe(4);
	});

	it('GET lists the connections and the read-only scopes the token needs', async () => {
		mocks.listAzureConnections.mockResolvedValue([{ id: 'org-1', name: 'example-org', token: 'ok' }]);
		const res = await status.GET(ev(undefined, 'GET'));
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(body.connections).toEqual([{ id: 'org-1', name: 'example-org', token: 'ok' }]);
		expect(body.scopes.map((s: { scope: string }) => s.scope)).toContain('vso.graph');
		expect(mocks.listAzureConnections).toHaveBeenCalledWith(SERVICE, 't1');
	});

	it('connect imports into THIS dōjō with the pasted token, bound to the named organization', async () => {
		const res = await connect.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(200);
		expect(mocks.azureAdapter).toHaveBeenCalledWith('example-org');
		expect(mocks.importAzureOrganization).toHaveBeenCalledWith(
			SERVICE,
			'p-admin',
			{ as: 'this', tenantId: 't1' },
			PAT,
			expect.objectContaining({ organization: 'example-org' })
		);
		const text = await res.text();
		expect(JSON.parse(text)).toEqual(IMPORTED);
		expect(text).not.toContain(PAT);
	});

	it('a token Azure will not take is a 422 naming no token; an outage a 502', async () => {
		mocks.importAzureOrganization.mockRejectedValue(new ForgeReadError(401, 'Azure DevOps read failed (401) for dev.azure.com/x'));
		let res = await connect.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(422);
		let text = await res.text();
		expect(text).toMatch(/did not accept that token/);
		expect(text).not.toContain(PAT);

		mocks.importAzureOrganization.mockRejectedValue(new ForgeReadError(203, 'sign-in page'));
		expect((await connect.POST(ev({ organization_url: URL_, token: PAT }))).status).toBe(422);

		mocks.importAzureOrganization.mockRejectedValue(new ForgeReadError(503, 'Azure DevOps read failed (503)'));
		res = await connect.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(502);
		text = await res.text();
		expect(text).not.toContain(PAT);
	});

	it('a refusal keeps its status and reason', async () => {
		mocks.importAzureOrganization.mockRejectedValue(new AdminError(403, 'only a Project Collection Administrator of example-org can import it'));
		const res = await connect.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(403);
		expect((await res.json()).error).toMatch(/Project Collection Administrator/);
	});

	it('a connect without an organization URL or a token, or with a URL that is not Azure, is a 400 and reaches nothing', async () => {
		for (const body of [{ token: PAT }, { organization_url: URL_ }, { organization_url: 'https://example.com/org', token: PAT }]) {
			const res = await connect.POST(ev(body));
			expect(res.status).toBe(400);
			expect(await res.text()).not.toContain(PAT);
		}
		expect(mocks.importAzureOrganization).not.toHaveBeenCalled();
		expect(mocks.azureAdapter).not.toHaveBeenCalled();
	});

	it('sync re-reads one connected organization with its stored token; the body carries none', async () => {
		const res = await sync.POST(ev({ organization_id: 'org-1', token: 'ignored' }));
		expect(res.status).toBe(200);
		const [db, who, tenant, org, adapterFor] = mocks.syncAzureOrganization.mock.calls[0];
		expect([db, who, tenant, org]).toEqual([SERVICE, 'p-admin', 't1', 'org-1']);
		expect(adapterFor('example-org')).toEqual({ provider: 'azure_devops', organization: 'example-org' });
		expect((await sync.POST(ev({}))).status).toBe(400);
	});

	it('DELETE token removes the stored token of one connected organization', async () => {
		const res = await token.DELETE(ev({ organization_id: 'org-1' }, 'DELETE'));
		expect(res.status).toBe(204);
		expect(mocks.removeAzureToken).toHaveBeenCalledWith(SERVICE, 'p-admin', 't1', 'org-1');
	});
});

describe('the personal dōjō\'s import', () => {
	it('imports as the organization\'s own dōjō, for the signed-in person', async () => {
		const res = await personal.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(200);
		expect(mocks.importAzureOrganization).toHaveBeenCalledWith(
			SERVICE,
			'p-me',
			{ as: 'own' },
			PAT,
			expect.objectContaining({ organization: 'example-org' })
		);
		expect(await res.text()).not.toContain(PAT);
	});

	it('refuses an anonymous caller', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response(JSON.stringify({ error: 'sign in' }), { status: 401 }));
		const res = await personal.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(401);
		expect(mocks.importAzureOrganization).not.toHaveBeenCalled();
	});

	it('maps a token Azure will not take to 422, like the organization route', async () => {
		mocks.importAzureOrganization.mockRejectedValue(new ForgeReadError(401, 'Azure DevOps read failed (401)'));
		const res = await personal.POST(ev({ organization_url: URL_, token: PAT }));
		expect(res.status).toBe(422);
	});
});
