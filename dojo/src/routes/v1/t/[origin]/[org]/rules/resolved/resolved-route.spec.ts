// GET /v1/t/{origin}/{org}/rules/resolved — the rules of packs the caller's
// tenant adopted, for the daemon's get_rules (#236 step 4; decision 2026-10-12:
// tenant-owned namespaces). Wiring only: dojo.tenant_pack_rules decides which
// rules, covered by database/tests/dojo/tenant_pack_rules.sql.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class RulesError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}
const mocks = vi.hoisted(() => ({ resolveTenantCaller: vi.fn(), resolveTenantPackRules: vi.fn() }));
const SESSION = vi.hoisted(() => ({ as: 'session' }));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ as: 'system' }),
	userDb: () => SESSION
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantCaller: mocks.resolveTenantCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/rules-data', async (importOriginal) => {
	const actual = await (importOriginal as () => Promise<Record<string, unknown>>)();
	return { ...actual, resolveTenantPackRules: mocks.resolveTenantPackRules, RulesError };
});

const { GET } = await import('./+server');

const member = { tenantId: 't1', userId: 'p1', membershipId: 'm1', role: 'contributor', access: 1, accessToken: 'jwt' };
function ev(query: string) {
	const url = new URL(`http://x/?${query}`);
	return { params: { origin: 'organization', org: 'acme' }, request: new Request(url), url, locals: {} } as never;
}

beforeEach(() => {
	mocks.resolveTenantCaller.mockReset().mockResolvedValue(member);
	mocks.resolveTenantPackRules.mockReset().mockResolvedValue([{ rule_id: 'r1' }]);
});

describe('GET rules/resolved', () => {
	it('asks as the person for the folder’s namespaces, at the member floor', async () => {
		const res = await GET(ev('ns=organization:acme,technology:react'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ rules: [{ rule_id: 'r1' }] });
		expect(mocks.resolveTenantCaller).toHaveBeenCalledWith('organization', 'acme', expect.any(Request), 0);
		expect(mocks.resolveTenantPackRules).toHaveBeenCalledWith(SESSION, 't1', [
			{ scope_key: 'organization', slug: 'acme' },
			{ scope_key: 'technology', slug: 'react' }
		]);
	});

	it('a caller with no session is 401', async () => {
		const { accessToken: _t, ...device } = member;
		mocks.resolveTenantCaller.mockResolvedValue(device);
		const res = await GET(ev('ns=organization:acme'));
		expect(res.status).toBe(401);
		expect(mocks.resolveTenantPackRules).not.toHaveBeenCalled();
	});

	it('a store error is a 500, never an empty rule list', async () => {
		mocks.resolveTenantPackRules.mockRejectedValue(new RulesError(500, 'boom'));
		const res = await GET(ev('ns=organization:acme'));
		expect(res.status).toBe(500);
	});

	it('propagates the auth refusal', async () => {
		mocks.resolveTenantCaller.mockRejectedValue(new Response('{}', { status: 403 }));
		expect((await GET(ev('ns=organization:acme'))).status).toBe(403);
	});
});
