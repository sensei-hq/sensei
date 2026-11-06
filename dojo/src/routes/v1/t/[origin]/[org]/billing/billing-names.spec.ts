// billing build (Design) — M3: "Who is counted" needs each seat's name,
// and the wire carries only user_id. GET …/billing resolves the names through the
// same identity join the members read uses (resolveDisplayNames), and never invents one.
// Names are read through dojoDb() — the access matrix's resolveDisplayNames exception:
// callers cannot read `principals`, so a caller-scoped read 500s for any unnamed
// seat-holder (L9, develop 91dc59735; L0 ruling r3). The SEATS stay the caller's read.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller: Record<string, unknown> = { tenantId: 't1', userId: 'admin-uuid', role: 'admin', access: 4, membershipId: 'm1', accessToken: 'jwt-A' };
// The identity resolver's error (identity-resolve.ts throws AdminError(500)); hoisted for the mock.
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
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
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
		mocks.loadActiveSeatRows.mockResolvedValue([seat('u1', 'ws-alpha'), seat('u2', 'ws-gamma'), seat('u3', 'ws-delta')]);
		// u1 named; u2 an email only; u3 nobody named (no entry) — an ordinary dōjō.
		mocks.resolveDisplayNames.mockResolvedValue(
			new Map([
				['u1', { display_name: 'Alex Doe', email: null }],
				['u2', { display_name: null, email: 'sam@example.com' }]
			])
		);
	});

	it('names each billable user through dojoDb(), for exactly those users; the seats stay the caller\'s read', async () => {
		const res = await GET(ev());
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(mocks.resolveDisplayNames).toHaveBeenCalledTimes(1);
		expect(mocks.resolveDisplayNames).toHaveBeenCalledWith({ client: 'service' }, 't1', ['u1', 'u2', 'u3']);
		// Kills a route that does everything as service: the seat rows are read as the caller.
		expect(mocks.loadActiveSeatRows.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		const users = (body as { usage: { billable_users: { user_id: string; display_name: string | null; email: string | null }[] } }).usage.billable_users;
		expect(users.map((u) => [u.user_id, u.display_name, u.email])).toEqual([
			['u1', 'Alex Doe', null],
			['u2', null, 'sam@example.com'],
			['u3', null, null]
		]);
	});

	it('with no billable user, no identity read', async () => {
		mocks.loadActiveSeatRows.mockResolvedValue([]);
		await GET(ev());
		expect(mocks.resolveDisplayNames).not.toHaveBeenCalled();
	});

	// M1 (r1): a failed name read is an error, never a 200 of null names (D83).
	it('a failed identity read answers 500 with the error, and no seat list', async () => {
		mocks.resolveDisplayNames.mockRejectedValue(new AdminError(500, 'identities: boom'));
		const res = await GET(ev());
		expect(res.status).toBe(500);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body).toEqual({ error: 'identities: boom' });
		expect('usage' in body).toBe(false);
	});
});
