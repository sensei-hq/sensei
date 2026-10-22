// Route tests for GET/POST /v1/t/{origin}/{org}/billing — the tenant's per-seat
// billing (admin floor). The billing-data store is mocked (its logic is in
// billing-data.spec); this proves the handler wiring: the admin floor, which
// client each read and write runs on (#236), and error mapping.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class BillingError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const caller: Record<string, unknown> = {
	tenantId: 't1',
	userId: 'admin-uuid',
	role: 'admin',
	access: 4,
	membershipId: 'm1',
	accessToken: 'jwt-A'
};

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	loadActiveSeatRows: vi.fn(),
	summarizeSeatUsage: vi.fn(),
	getBillingAccount: vi.fn(),
	refreshSeatsUsed: vi.fn()
}));

// The service role vs the caller's own JWT — which client each store call got.
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
vi.mock('$lib/server/billing-data', () => ({
	loadActiveSeatRows: mocks.loadActiveSeatRows,
	summarizeSeatUsage: mocks.summarizeSeatUsage,
	getBillingAccount: mocks.getBillingAccount,
	refreshSeatsUsed: mocks.refreshSeatsUsed,
	BillingError
}));

const route = await import('./+server');

function ev() {
	return {
		params: { origin: 'github', org: 'acme' },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

const USER = { client: 'user', token: 'jwt-A' };
const SERVICE = { client: 'service' };

beforeEach(() => {
	caller.accessToken = 'jwt-A';
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.loadActiveSeatRows.mockClear().mockResolvedValue([]);
	mocks.summarizeSeatUsage.mockClear().mockReturnValue({ seats_used: 2, total_active_seats: 3, billable_users: [] });
	mocks.getBillingAccount.mockClear().mockResolvedValue({ plan: 'team' });
	mocks.refreshSeatsUsed.mockClear().mockResolvedValue(undefined);
});

describe('GET /billing', () => {
	it('returns { account, usage } at the admin floor', async () => {
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({
			account: { plan: 'team' },
			usage: { seats_used: 2, total_active_seats: 3, billable_users: [] }
		});
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4); // ACCESS.admin
	});
	// Seats and the account are read as the signed-in admin (the policies decide);
	// the seats' workspace names are a system lookup — no session reads
	// sensei.namespaces (#236).
	it('reads seats and the account as the caller, workspace names as the system', async () => {
		await route.GET(ev());
		expect(mocks.loadActiveSeatRows.mock.calls[0]).toEqual([USER, 't1', SERVICE]);
		expect(mocks.getBillingAccount.mock.calls[0][0]).toEqual(USER);
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		caller.accessToken = undefined;
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.loadActiveSeatRows).not.toHaveBeenCalled();
	});
	it('maps a BillingError(500) to 500', async () => {
		mocks.getBillingAccount.mockRejectedValueOnce(new BillingError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});
});

describe('POST /billing (refresh seats_used)', () => {
	// seats_used is a money-facing cache derived from the seats: the system
	// recomputes and writes it after the admin gate; no session writes it.
	it('recomputes and persists as the system, after the admin gate', async () => {
		const res = await route.POST(ev());
		expect(await res.json()).toEqual({ seats_used: 2 });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(4); // ACCESS.admin
		expect(mocks.loadActiveSeatRows.mock.calls[0]).toEqual([SERVICE, 't1', SERVICE]);
		expect(mocks.refreshSeatsUsed.mock.calls[0][0]).toEqual(SERVICE);
		expect(mocks.refreshSeatsUsed.mock.calls[0][2]).toBe(2);
	});
	it('propagates a 403 (below admin) without refreshing', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.POST(ev())).status).toBe(403);
		expect(mocks.refreshSeatsUsed).not.toHaveBeenCalled();
	});
});
