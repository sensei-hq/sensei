// Route-level tests for GET /v1/t/{origin}/{org}/audit/ledger — the lead
// confidentiality ledger. Covers the LEAD floor, the { entries } envelope,
// AdminError → status, and auth-Response propagation. The store is mocked.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller = { tenantId: 't1', userId: 'lead-uuid', role: 'lead', access: 2, membershipId: 'm1' };

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveTenantAccess: vi.fn(), getClientAuditLedger: vi.fn() }));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}), ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 } }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/client-audit-data', () => ({ getClientAuditLedger: mocks.getClientAuditLedger, AdminError }));

const route = await import('./+server');

function ev() {
	return {
		params: { origin: 'github', org: 'acme' },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.getClientAuditLedger.mockClear().mockResolvedValue([]);
});

describe('GET /audit/ledger', () => {
	it('returns { entries } at the LEAD floor', async () => {
		mocks.getClientAuditLedger.mockResolvedValueOnce([{ id: 1, action: 'publish', client_name: 'Globex' }]);
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({ entries: [{ id: 1, action: 'publish', client_name: 'Globex' }] });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2); // ACCESS.lead
	});
	it('maps an AdminError(500) to 500 (fail-closed)', async () => {
		mocks.getClientAuditLedger.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});
	it('propagates a 403 without reading', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.GET(ev())).status).toBe(403);
		expect(mocks.getClientAuditLedger).not.toHaveBeenCalled();
	});
});
