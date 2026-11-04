// billing build (Design, phase A) — M3: "Who is counted" needs each seat's name,
// and the wire carries only user_id. GET …/billing resolves the names through the
// same identity join the members read uses (resolveDisplayNames, as the caller),
// and never invents one.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller: Record<string, unknown> = { tenantId: 't1', userId: 'admin-uuid', role: 'admin', access: 4, membershipId: 'm1', accessToken: 'jwt-A' };
const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	loadActiveSeatRows: vi.fn(),
	getBillingAccount: vi.fn(),
	resolveDisplayNames: vi.fn()
}));
vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({ client: 'service' }), userDb: (token: string) => ({ client: 'user', token }) }));
vi.mock('$lib/server/caller-db', () => ({ callerDb: (c: { accessToken: string }) => ({ client: 'user', token: c.accessToken }) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) => new Response(JSON.stringify({ error: message }), { status }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/identity-resolve', () => ({ resolveDisplayNames: mocks.resolveDisplayNames }));
vi.mock('$lib/server/billing-data', async (orig) => ({
	...(await orig<typeof import('$lib/server/billing-data')>()),
	loadActiveSeatRows: mocks.loadActiveSeatRows,
	getBillingAccount: mocks.getBillingAccount
}));

const { GET } = await import('./+server');
const ev = () => ({ params: { origin: 'github', org: 'acme' }, request: new Request('http://x/v1/t/github/acme/billing'), locals: {} }) as never;
const seat = (user_id: string, name: string) => ({ user_id, role: 'contributor', namespace_id: 'n-' + name, workspace_name: name, workspace_slug: name, visibility: 'private' as const });

describe('GET …/billing · seat names (billing build, M3)', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mocks.resolveTenantAccess.mockResolvedValue(caller);
		mocks.getBillingAccount.mockResolvedValue(null);
		mocks.loadActiveSeatRows.mockResolvedValue([seat('u1', 'platform-core'), seat('u2', 'mobile-app')]);
		mocks.resolveDisplayNames.mockResolvedValue(new Map([['u1', { display_name: 'Priya K', email: null }]]));
	});

	it.skip('pending: billing-build — names each billable user from the identity join, as the caller, for exactly those users', async () => {
		const res = await GET(ev());
		const body = await res.json();
		expect(mocks.resolveDisplayNames).toHaveBeenCalledWith({ client: 'user', token: 'jwt-A' }, 't1', ['u1', 'u2']);
		expect(body.usage.billable_users.map((u: { user_id: string; display_name: string | null }) => [u.user_id, u.display_name])).toEqual([
			['u1', 'Priya K'],
			['u2', null]
		]);
	});

	it.skip('pending: billing-build — with no billable user, no identity read', async () => {
		mocks.loadActiveSeatRows.mockResolvedValue([]);
		await GET(ev());
		expect(mocks.resolveDisplayNames).not.toHaveBeenCalled();
	});
});
