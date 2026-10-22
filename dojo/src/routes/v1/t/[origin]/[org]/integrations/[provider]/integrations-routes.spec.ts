// Route tests for Admin → Integrations (#308 R5a): list the units the person's
// GitHub sees, claim a unit's dōjō, map a unit. ADMIN floor of the dōjō whose
// screen it is. The GitHub token comes ONLY from the sealed cookie a "Connect
// GitHub" re-authorization set (forge-token-cookie.ts) — never from the request
// body; without it, or with one GitHub no longer honours, the answer is 428
// "connect GitHub", never a read with something else. Data layer mocked — its
// own logic is in integrations-data.spec.ts.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ForgeReadError } from '$lib/server/forge-github';
import { FORGE_TOKEN_COOKIE, sealForgeToken } from '$lib/server/forge-token-cookie';

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

const SECRET = 'route-test-secret';
const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	listIntegrationUnits: vi.fn(),
	claimIntegrationUnit: vi.fn(),
	mapIntegrationUnit: vi.fn(),
	forgeAdapter: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token }),
	serverSecret: () => 'route-test-secret'
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/integrations-data', async (orig) => ({
	...(await orig<typeof import('$lib/server/integrations-data')>()),
	listIntegrationUnits: mocks.listIntegrationUnits,
	claimIntegrationUnit: mocks.claimIntegrationUnit,
	mapIntegrationUnit: mocks.mapIntegrationUnit
}));
vi.mock('$lib/server/forge-adapter', () => ({ forgeAdapter: mocks.forgeAdapter }));
vi.mock('$lib/server/admin-data', async (orig) => ({
	...(await orig<typeof import('$lib/server/admin-data')>()),
	AdminError
}));

const list = await import('./+server');
const claim = await import('./claim/+server');
const map = await import('./map/+server');

const ev = (body: unknown, cookie?: string, provider = 'github') =>
	({
		params: { origin: 'organization', org: 'acme', provider },
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: JSON.stringify(body)
		}),
		cookies: { get: (name: string) => (name === FORGE_TOKEN_COOKIE ? cookie : undefined) },
		locals: {},
		url: new URL('http://x/')
	}) as never;

const SERVICE = { client: 'service' };
const ADAPTER = { provider: 'github' };
let connected: string;

beforeEach(async () => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({
		tenantId: 't1',
		userId: 'p-admin',
		authUserId: 'auth-admin',
		role: 'admin',
		access: 4,
		accessToken: 'jwt-A'
	});
	mocks.forgeAdapter.mockReturnValue(ADAPTER);
	mocks.listIntegrationUnits.mockResolvedValue({ provider: 'github', units: [], complete: true });
	mocks.claimIntegrationUnit.mockResolvedValue({ dojo: { key: 'organization/acme' } });
	mocks.mapIntegrationUnit.mockResolvedValue({ tenantId: 't1', workspaceId: 'w1' });
	connected = await sealForgeToken('gho_connected', 'auth-admin', SECRET);
});

describe('the integrations routes', () => {
	it('every route asks for the admin floor of the dōjō', async () => {
		await list.POST(ev({}, connected));
		await claim.POST(ev({ unit_id: '22' }, connected));
		await map.POST(ev({ unit_id: '22', as: 'workspace' }, connected));
		expect(mocks.resolveTenantAccess).toHaveBeenCalledTimes(3);
		for (const call of mocks.resolveTenantAccess.mock.calls) expect(call[4]).toBe(4);
	});

	it('reads with the token the GitHub connection sealed — as system work for this dōjō', async () => {
		const view = { provider: 'github', units: [{ id: '22' }], complete: true };
		mocks.listIntegrationUnits.mockResolvedValue(view);
		const res = await list.POST(ev({}, connected));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual(view);
		expect(mocks.forgeAdapter).toHaveBeenCalledWith('github');
		expect(mocks.listIntegrationUnits).toHaveBeenCalledWith(SERVICE, 'p-admin', 't1', 'gho_connected', ADAPTER);
	});

	it('is 428 "connect GitHub" without a connection — a token in the body is not one', async () => {
		const another = await sealForgeToken('gho_other', 'auth-someone-else', SECRET);
		for (const [route, body, cookie] of [
			[list, { token: 'pasted' }, undefined],
			[claim, { token: 'pasted', unit_id: '22' }, undefined],
			[map, { token: 'pasted', unit_id: '22', as: 'workspace' }, undefined],
			[list, {}, another],
			[list, {}, 'tampered']
		] as const) {
			const res = await route.POST(ev(body, cookie));
			expect(res.status).toBe(428);
			expect((await res.json()).error).toMatch(/Connect GitHub/);
		}
		expect(mocks.listIntegrationUnits).not.toHaveBeenCalled();
		expect(mocks.claimIntegrationUnit).not.toHaveBeenCalled();
		expect(mocks.mapIntegrationUnit).not.toHaveBeenCalled();
	});

	it('claim and map hand the unit and target through, with the connected token', async () => {
		const c = await claim.POST(ev({ unit_id: '22' }, connected));
		expect(c.status).toBe(200);
		expect(await c.json()).toEqual({ dojo: { key: 'organization/acme' } });
		expect(mocks.claimIntegrationUnit).toHaveBeenCalledWith(SERVICE, 'p-admin', 'gho_connected', ADAPTER, '22');

		const m = await map.POST(ev({ unit_id: '33', as: 'workspace', name: 'Beta' }, connected));
		expect(m.status).toBe(200);
		expect(mocks.mapIntegrationUnit).toHaveBeenCalledWith(SERVICE, 'p-admin', 't1', 'gho_connected', ADAPTER, '33', {
			as: 'workspace',
			name: 'Beta'
		});
		await map.POST(ev({ unit_id: '33', as: 'dojo', name: 'ignored' }, connected));
		expect(mocks.mapIntegrationUnit).toHaveBeenLastCalledWith(SERVICE, 'p-admin', 't1', 'gho_connected', ADAPTER, '33', {
			as: 'dojo'
		});
	});

	it('refuses a claim or map with no unit, or a map with no target, with 400', async () => {
		for (const [route, body] of [
			[claim, {}],
			[map, { unit_id: '22' }],
			[map, { unit_id: '22', as: 'team' }]
		] as const) {
			expect((await route.POST(ev(body, connected))).status).toBe(400);
		}
		expect(mocks.claimIntegrationUnit).not.toHaveBeenCalled();
		expect(mocks.mapIntegrationUnit).not.toHaveBeenCalled();
	});

	it('when GitHub no longer honours the token: 428, and the cookie is cleared', async () => {
		mocks.listIntegrationUnits.mockRejectedValue(new ForgeReadError(401, 'GitHub read failed (401)'));
		const res = await list.POST(ev({}, connected));
		expect(res.status).toBe(428);
		expect(res.headers.get('set-cookie')).toMatch(new RegExp(`^${FORGE_TOKEN_COOKIE}=; .*Max-Age=0`));
	});

	it('answers 404 for a provider that is not a forge', async () => {
		const res = await list.POST(ev({}, connected, 'myspace'));
		expect(res.status).toBe(404);
		expect(mocks.forgeAdapter).not.toHaveBeenCalled();
	});

	it('a refusal keeps its status and reason, and never echoes the token', async () => {
		mocks.claimIntegrationUnit.mockRejectedValue(new AdminError(403, 'only an admin of acme can claim its dōjō'));
		const res = await claim.POST(ev({ unit_id: '22' }, connected));
		expect(res.status).toBe(403);
		const text = await res.text();
		expect(JSON.parse(text)).toEqual({ error: 'only an admin of acme can claim its dōjō' });
		expect(text).not.toContain('gho_connected');
	});
});
