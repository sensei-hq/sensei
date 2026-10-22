// Route tests for Admin → Integrations (#308 R5a): list the units a token sees,
// claim a unit's dōjō, map a unit. ADMIN floor of the dōjō whose screen it is;
// the token travels in the BODY (never a URL) and is handed to the data layer,
// never echoed; a refusal keeps its status. Data layer mocked — its own logic is
// in integrations-data.spec.ts.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	listIntegrationUnits: vi.fn(),
	claimIntegrationUnit: vi.fn(),
	mapIntegrationUnit: vi.fn(),
	forgeAdapter: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
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

const ev = (body: unknown, provider = 'github') =>
	({
		params: { origin: 'organization', org: 'acme', provider },
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	}) as never;

const SERVICE = { client: 'service' };
const ADAPTER = { provider: 'github' };

beforeEach(() => {
	for (const m of Object.values(mocks)) m.mockReset();
	mocks.resolveTenantAccess.mockResolvedValue({ tenantId: 't1', userId: 'p-admin', role: 'admin', access: 4, accessToken: 'jwt-A' });
	mocks.forgeAdapter.mockReturnValue(ADAPTER);
});

describe('the integrations routes', () => {
	it('every route asks for the admin floor of the dōjō', async () => {
		mocks.listIntegrationUnits.mockResolvedValue({ provider: 'github', units: [], complete: true });
		mocks.claimIntegrationUnit.mockResolvedValue({ dojo: { key: 'organization/acme' } });
		mocks.mapIntegrationUnit.mockResolvedValue({ tenantId: 't1', workspaceId: 'w1' });
		await list.POST(ev({ token: 'gh' }));
		await claim.POST(ev({ token: 'gh', unit_id: '22' }));
		await map.POST(ev({ token: 'gh', unit_id: '22', as: 'workspace' }));
		expect(mocks.resolveTenantAccess).toHaveBeenCalledTimes(3);
		for (const call of mocks.resolveTenantAccess.mock.calls) expect(call[4]).toBe(4);
	});

	it('POST lists the units the body`s token sees, as system work for this dōjō', async () => {
		const view = { provider: 'github', units: [{ id: '22' }], complete: true };
		mocks.listIntegrationUnits.mockResolvedValue(view);
		const res = await list.POST(ev({ token: ' gh-token ' }));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual(view);
		expect(mocks.forgeAdapter).toHaveBeenCalledWith('github');
		expect(mocks.listIntegrationUnits).toHaveBeenCalledWith(SERVICE, 'p-admin', 't1', 'gh-token', ADAPTER);
	});

	it('claim and map hand the unit and target through', async () => {
		mocks.claimIntegrationUnit.mockResolvedValue({ dojo: { key: 'organization/acme' } });
		const c = await claim.POST(ev({ token: 'gh', unit_id: '22' }));
		expect(c.status).toBe(200);
		expect(await c.json()).toEqual({ dojo: { key: 'organization/acme' } });
		expect(mocks.claimIntegrationUnit).toHaveBeenCalledWith(SERVICE, 'p-admin', 'gh', ADAPTER, '22');

		mocks.mapIntegrationUnit.mockResolvedValue({ tenantId: 't1', workspaceId: 'w1' });
		const m = await map.POST(ev({ token: 'gh', unit_id: '33', as: 'workspace', name: 'Beta' }));
		expect(m.status).toBe(200);
		expect(mocks.mapIntegrationUnit).toHaveBeenCalledWith(SERVICE, 'p-admin', 't1', 'gh', ADAPTER, '33', {
			as: 'workspace',
			name: 'Beta'
		});
		await map.POST(ev({ token: 'gh', unit_id: '33', as: 'dojo', name: 'ignored' }));
		expect(mocks.mapIntegrationUnit).toHaveBeenLastCalledWith(SERVICE, 'p-admin', 't1', 'gh', ADAPTER, '33', { as: 'dojo' });
	});

	it('refuses a request with no token, no unit, or no target with 400 — and reads nothing', async () => {
		for (const [route, body] of [
			[list, {}],
			[list, { token: '   ' }],
			[claim, { token: 'gh' }],
			[map, { token: 'gh', unit_id: '22' }],
			[map, { token: 'gh', unit_id: '22', as: 'team' }]
		] as const) {
			const res = await route.POST(ev(body));
			expect(res.status).toBe(400);
		}
		expect(mocks.listIntegrationUnits).not.toHaveBeenCalled();
		expect(mocks.claimIntegrationUnit).not.toHaveBeenCalled();
		expect(mocks.mapIntegrationUnit).not.toHaveBeenCalled();
	});

	it('answers 404 for a provider that is not a forge', async () => {
		const res = await list.POST(ev({ token: 'gh' }, 'myspace'));
		expect(res.status).toBe(404);
		expect(mocks.forgeAdapter).not.toHaveBeenCalled();
	});

	it('a refusal keeps its status and reason, and never echoes the token', async () => {
		mocks.claimIntegrationUnit.mockRejectedValue(new AdminError(403, 'only an admin of acme can claim its dōjō'));
		const res = await claim.POST(ev({ token: 'gh-secret', unit_id: '22' }));
		expect(res.status).toBe(403);
		const text = await res.text();
		expect(JSON.parse(text)).toEqual({ error: 'only an admin of acme can claim its dōjō' });
		expect(text).not.toContain('gh-secret');
	});
});
