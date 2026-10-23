// Route tests for connecting Azure DevOps through Microsoft Entra (#308 R5b,
// D42): start (an org dōjō's admin, or the personal dōjō), Microsoft's callback,
// the pending grant's organization list and import, and an org dōjō's status /
// sync / remove-token. No body ever carries a token and no answer returns one.
// Every connect step is rate-limited. A server with no app registration answers
// 503 "not configured". The data and flow layers are mocked — their logic is in
// azure-import.spec.ts, azure-connect.spec.ts and azure-entra.spec.ts.
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
	azureAdapter: vi.fn(),
	listAzureOrganizations: vi.fn(),
	startConnect: vi.fn(),
	finishConnect: vi.fn(),
	openGrant: vi.fn(),
	useGrant: vi.fn(),
	guardAttempt: vi.fn(),
	env: {} as Record<string, string | undefined>
}));

vi.mock('$env/dynamic/private', () => ({ env: mocks.env }));
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
	azureAdapter: mocks.azureAdapter,
	listAzureOrganizations: mocks.listAzureOrganizations
}));
vi.mock('$lib/server/azure-connect', async (orig) => ({
	...(await orig<typeof import('$lib/server/azure-connect')>()),
	startConnect: mocks.startConnect,
	finishConnect: mocks.finishConnect,
	openGrant: mocks.openGrant,
	useGrant: mocks.useGrant
}));
vi.mock('$lib/server/rate-limit', async (orig) => ({
	...(await orig<typeof import('$lib/server/rate-limit')>()),
	guardAttempt: mocks.guardAttempt
}));

const status = await import('./+server');
const connect = await import('./connect/+server');
const sync = await import('./sync/+server');
const token = await import('./token/+server');
const personal = await import('../../../../../you/integrations/azure_devops/connect/+server');
const callback = await import('../../../../../integrations/azure_devops/callback/+server');
const organizations = await import('../../../../../integrations/azure_devops/organizations/+server');
const importRoute = await import('../../../../../integrations/azure_devops/import/+server');

const SERVICE = { client: 'service' };
const CONFIGURED = { AZURE_DEVOPS_CLIENT_ID: 'client-id-0000', AZURE_DEVOPS_CLIENT_SECRET: 'client-secret-0000' };
const GRANT = { refreshToken: 'ztest-refresh-1', writeScopes: ['vso.code_write'], target: { as: 'this', tenantId: 't1' } };

const ev = (body: unknown, method = 'POST', origin = 'organization', url = 'https://dojo.example.com/v1/x', cookies: Record<string, string> = {}) =>
	({
		params: { origin, org: 'acme' },
		request: new Request(url, {
			method,
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: method === 'GET' ? undefined : JSON.stringify(body)
		}),
		cookies: { get: (name: string) => cookies[name] },
		locals: {},
		url: new URL(url)
	}) as never;

const IMPORTED = {
	dojo: { key: 'organization/acme', created: false },
	organization: { id: 'org-1', name: 'example-org' },
	projects: { total: 2, mapped: 2, elsewhere: 0, complete: true },
	members: { complete: true, listed: 3, ended: 0 }
};

beforeEach(() => {
	for (const m of Object.values(mocks)) if (typeof m === 'function' && 'mockReset' in m) m.mockReset();
	for (const k of Object.keys(mocks.env)) delete mocks.env[k];
	Object.assign(mocks.env, CONFIGURED);
	mocks.resolveTenantAccess.mockResolvedValue({ tenantId: 't1', userId: 'p-admin', authUserId: 'auth-admin', role: 'admin', access: 4, accessToken: 'jwt-A' });
	mocks.resolveCaller.mockResolvedValue({ userId: 'p-me', authUserId: 'auth-me', email: 'me@example.com', accessToken: 'jwt', githubId: null });
	mocks.azureAdapter.mockImplementation((org: string) => ({ provider: 'azure_devops', organization: org }));
	mocks.importAzureOrganization.mockResolvedValue(IMPORTED);
	mocks.syncAzureOrganization.mockResolvedValue({ organization: IMPORTED.organization, projects: IMPORTED.projects, members: IMPORTED.members });
	mocks.listAzureConnections.mockResolvedValue([]);
	mocks.removeAzureToken.mockResolvedValue(undefined);
	mocks.guardAttempt.mockResolvedValue(undefined);
	mocks.startConnect.mockResolvedValue({ url: 'https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize?state=s', cookie: 'dojo_azure_state=sealed; Path=/v1' });
	mocks.finishConnect.mockResolvedValue({ location: '/org/t1/integrations?azure=connected', cookies: ['dojo_azure_state=; Max-Age=0', 'dojo_azure_grant=sealed; Path=/v1'] });
	mocks.openGrant.mockResolvedValue(GRANT);
	mocks.useGrant.mockResolvedValue({ accessToken: 'ztest-access', grant: { ...GRANT, refreshToken: 'ztest-refresh-2' }, cookie: 'dojo_azure_grant=resealed; Path=/v1' });
	mocks.listAzureOrganizations.mockResolvedValue({ organizations: [{ id: 'a1', name: 'example-org' }], complete: true });
});

describe('start', () => {
	it("an org dōjō's admin starts the connect into THIS dōjō — rate-limited, the state in a cookie, the Entra URL in the answer", async () => {
		const res = await connect.POST(ev({}));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ url: 'https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize?state=s' });
		expect(res.headers.get('set-cookie')).toContain('dojo_azure_state=sealed');
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4);
		expect(mocks.guardAttempt).toHaveBeenCalledWith(SERVICE, 'p-admin', 'azure_connect');
		const args = mocks.startConnect.mock.calls[0][0];
		expect(args).toMatchObject({ authUserId: 'auth-admin', secret: 'route-test-secret', origin: 'https://dojo.example.com', target: { as: 'this', tenantId: 't1' } });
		expect(args.cfg).toEqual({ clientId: 'client-id-0000', clientSecret: 'client-secret-0000' });
	});

	it('the personal dōjō starts a connect that imports organizations as their own dōjōs', async () => {
		const res = await personal.POST(ev({}));
		expect(res.status).toBe(200);
		expect(mocks.startConnect.mock.calls[0][0]).toMatchObject({ authUserId: 'auth-me', target: { as: 'own' } });
		expect(mocks.guardAttempt).toHaveBeenCalledWith(SERVICE, 'p-me', 'azure_connect');
	});

	it('is a 503 "not configured" without the app registration — nothing started, no attempt spent', async () => {
		delete mocks.env.AZURE_DEVOPS_CLIENT_SECRET;
		for (const route of [connect, personal]) {
			const res = await route.POST(ev({}));
			expect(res.status).toBe(503);
			expect((await res.json()).error).toBe('Azure DevOps is not configured on this dōjō');
		}
		expect(mocks.startConnect).not.toHaveBeenCalled();
		expect(mocks.guardAttempt).not.toHaveBeenCalled();
	});

	it('a personal dōjō cannot start an import into itself through the org route (400)', async () => {
		const res = await connect.POST(ev({}, 'POST', 'personal'));
		expect(res.status).toBe(400);
		expect(mocks.startConnect).not.toHaveBeenCalled();
	});

	it('too many attempts is a 429 before Microsoft is asked', async () => {
		mocks.guardAttempt.mockRejectedValue(new AdminError(429, 'too many attempts — try again in a few minutes'));
		expect((await connect.POST(ev({}))).status).toBe(429);
		expect((await personal.POST(ev({}))).status).toBe(429);
		expect(mocks.startConnect).not.toHaveBeenCalled();
	});
});

describe('callback', () => {
	const url = 'https://dojo.example.com/v1/integrations/azure_devops/callback?code=c&state=s';

	it("finishes the connect for the session's login and redirects back with the grant cookie", async () => {
		const res = await callback.GET(ev(undefined, 'GET', 'organization', url, { dojo_azure_state: 'sealed-state' }));
		expect(res.status).toBe(303);
		expect(res.headers.get('location')).toBe('/org/t1/integrations?azure=connected');
		expect(res.headers.get('set-cookie')).toContain('dojo_azure_grant=sealed');
		const args = mocks.finishConnect.mock.calls[0][0];
		expect(args).toMatchObject({ authUserId: 'auth-me', url, stateCookie: 'sealed-state', secret: 'route-test-secret' });
		// The attempt is counted only once the state has opened and matched — finishConnect calls the guard.
		expect(mocks.guardAttempt).not.toHaveBeenCalled();
		await args.guard();
		expect(mocks.guardAttempt).toHaveBeenCalledWith(SERVICE, 'p-me', 'azure_connect');
	});

	it('answers a top-level navigation with a redirect naming why: no session, not configured', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		let res = await callback.GET(ev(undefined, 'GET', 'organization', url));
		expect(res.headers.get('location')).toBe('/you/dojos?azure_error=signin');

		delete mocks.env.AZURE_DEVOPS_CLIENT_ID;
		res = await callback.GET(ev(undefined, 'GET', 'organization', url));
		expect(res.headers.get('location')).toBe('/you/dojos?azure_error=not_configured');
		Object.assign(mocks.env, CONFIGURED);
		expect(mocks.finishConnect).not.toHaveBeenCalled();
		expect(mocks.guardAttempt).not.toHaveBeenCalled();
	});
});

describe('the pending grant: organizations and import', () => {
	const withGrant = (body: unknown) => ev(body, 'POST', 'organization', 'https://dojo.example.com/v1/integrations/azure_devops/x', { dojo_azure_grant: 'sealed-grant' });

	it('lists the organizations the grant sees, with its write scopes, and re-seals the rotated grant — no token in the answer', async () => {
		const res = await organizations.POST(withGrant({}));
		expect(res.status).toBe(200);
		const text = await res.text();
		expect(JSON.parse(text)).toEqual({
			organizations: [{ id: 'a1', name: 'example-org' }],
			complete: true,
			writeScopes: ['vso.code_write'],
			target: 'this'
		});
		expect(text).not.toContain('ztest');
		expect(res.headers.get('set-cookie')).toContain('dojo_azure_grant=resealed');
		expect(mocks.openGrant).toHaveBeenCalledWith('sealed-grant', 'auth-me', 'route-test-secret');
		expect(mocks.listAzureOrganizations.mock.calls[0][0]).toBe('ztest-access');
		expect(mocks.guardAttempt).toHaveBeenCalledWith(SERVICE, 'p-me', 'azure_connect');
	});

	it('is a 428 "connect first" without a grant that opens for this login', async () => {
		mocks.openGrant.mockResolvedValue(null);
		expect((await organizations.POST(withGrant({}))).status).toBe(428);
		expect((await importRoute.POST(withGrant({ organization: 'example-org' }))).status).toBe(428);
		expect(mocks.useGrant).not.toHaveBeenCalled();
	});

	it('imports the picked organization where the connect was started, storing the rotated refresh token', async () => {
		const res = await importRoute.POST(withGrant({ organization: 'Example-Org' }));
		expect(res.status).toBe(200);
		const text = await res.text();
		expect(JSON.parse(text)).toEqual(IMPORTED);
		expect(text).not.toContain('ztest');
		expect(mocks.azureAdapter).toHaveBeenCalledWith('example-org');
		expect(mocks.importAzureOrganization).toHaveBeenCalledWith(
			SERVICE,
			'p-me',
			{ as: 'this', tenantId: 't1' },
			{ accessToken: 'ztest-access', refreshToken: 'ztest-refresh-2' },
			expect.objectContaining({ organization: 'example-org' })
		);
		expect(res.headers.get('set-cookie')).toContain('dojo_azure_grant=resealed');
	});

	it('an Azure read refused is a 422 naming no token; an outage stays a 502', async () => {
		mocks.importAzureOrganization.mockRejectedValue(new ForgeReadError(401, 'Azure DevOps read failed (401) for dev.azure.com/x'));
		let res = await importRoute.POST(withGrant({ organization: 'example-org' }));
		expect(res.status).toBe(422);
		mocks.importAzureOrganization.mockRejectedValue(new ForgeReadError(503, 'Azure DevOps read failed (503)'));
		res = await importRoute.POST(withGrant({ organization: 'example-org' }));
		expect(res.status).toBe(502);
	});

	it('an import with no organization is a 400 and reaches nothing', async () => {
		expect((await importRoute.POST(withGrant({}))).status).toBe(400);
		expect(mocks.importAzureOrganization).not.toHaveBeenCalled();
	});
});

describe('an org dōjō: status, sync, remove token', () => {
	it('every route asks for the admin floor', async () => {
		await status.GET(ev(undefined, 'GET'));
		await sync.POST(ev({ organization_id: 'org-1' }));
		await token.DELETE(ev({ organization_id: 'org-1' }, 'DELETE'));
		for (const call of mocks.resolveTenantAccess.mock.calls) expect(call[4]).toBe(4);
	});

	it('GET lists the connections, the read scopes, and whether this dōjō can connect', async () => {
		mocks.listAzureConnections.mockResolvedValue([{ id: 'org-1', name: 'example-org', token: 'ok' }]);
		let body = await (await status.GET(ev(undefined, 'GET'))).json();
		expect(body.connections).toEqual([{ id: 'org-1', name: 'example-org', token: 'ok' }]);
		expect(body.scopes.map((s: { scope: string }) => s.scope)).toContain('vso.graph');
		expect(body.configured).toBe(true);
		delete mocks.env.AZURE_DEVOPS_CLIENT_ID;
		body = await (await status.GET(ev(undefined, 'GET'))).json();
		expect(body.configured).toBe(false);
	});

	it('sync refreshes the stored grant through the configured app registration', async () => {
		const res = await sync.POST(ev({ organization_id: 'org-1' }));
		expect(res.status).toBe(200);
		const [db, who, tenant, org, deps] = mocks.syncAzureOrganization.mock.calls[0];
		expect([db, who, tenant, org]).toEqual([SERVICE, 'p-admin', 't1', 'org-1']);
		expect(deps.cfg).toEqual({ clientId: 'client-id-0000', clientSecret: 'client-secret-0000' });
		expect(deps.adapterFor('example-org')).toEqual({ provider: 'azure_devops', organization: 'example-org' });
		expect((await sync.POST(ev({}))).status).toBe(400);
		delete mocks.env.AZURE_DEVOPS_CLIENT_SECRET;
		expect((await sync.POST(ev({ organization_id: 'org-1' }))).status).toBe(503);
	});

	it('DELETE token removes the stored grant of one connected organization', async () => {
		const res = await token.DELETE(ev({ organization_id: 'org-1' }, 'DELETE'));
		expect(res.status).toBe(204);
		expect(mocks.removeAzureToken).toHaveBeenCalledWith(SERVICE, 'p-admin', 't1', 'org-1');
	});
});
