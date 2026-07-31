// Route-level tests for GET /v1/t/{origin}/{org}/knowledge — the maintainer
// Knowledge library read. Covers the MAINTAINER auth floor, the envelope,
// KnowledgeError → status mapping, and auth-Response propagation. The
// getKnowledgeLibrary store is mocked (its own logic is in knowledge-data.spec).
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller = { tenantId: 't1', userId: 'maint-uuid', role: 'maintainer', access: 3, membershipId: 'm1' };

class KnowledgeError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	getKnowledgeLibrary: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}), ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 } }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/knowledge-data', () => ({
	getKnowledgeLibrary: mocks.getKnowledgeLibrary,
	KnowledgeError
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

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.getKnowledgeLibrary.mockClear().mockResolvedValue({ retention_days: 90, active: [], pending: [], catalog: [] });
});

describe('GET /knowledge', () => {
	it('returns the library at the MAINTAINER floor', async () => {
		const lib = { retention_days: 30, active: [{ id: 'a1' }], pending: [], catalog: [{ id: 's1' }] };
		mocks.getKnowledgeLibrary.mockResolvedValueOnce(lib);
		const res = await route.GET(ev());
		expect(await res.json()).toEqual(lib);
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(3); // ACCESS.maintainer
	});
	it('maps a KnowledgeError(500) to 500 (fail-closed, never a fixture)', async () => {
		mocks.getKnowledgeLibrary.mockRejectedValueOnce(new KnowledgeError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});
	it('propagates a 403 from the auth floor without reading', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await route.GET(ev())).status).toBe(403);
		expect(mocks.getKnowledgeLibrary).not.toHaveBeenCalled();
	});
});
