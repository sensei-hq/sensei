// Route-level tests for GET /v1/you/projects/{slug}/constitution — the USER-WIDE
// per-project constitution read (F4). Caller resolved from the JWT (no tenant, no
// role floor), read authorized by the user_id filter inside the store. Covers the
// { constitution } envelope, honest null (no federated resolution yet),
// AdminError → status (fail-closed, never a fixture), and 401 propagation.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), getUserProjectConstitution: vi.fn() }));

// The service role vs the caller's own JWT — which client the store got.
vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/projects-data', () => ({
	getUserProjectConstitution: mocks.getUserProjectConstitution,
	AdminError
}));

const route = await import('./+server');

function ev(slug = 'acme/ledger') {
	return {
		params: { slug },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.getUserProjectConstitution.mockClear().mockResolvedValue(null);
});

describe('GET /v1/you/projects/{slug}/constitution', () => {
	it('returns { constitution } scoped to the caller + slug', async () => {
		const constitution = { rules: [{ scope_key: 'organization', title: 'x', enforcement: 'mandatory' }], conflicts: [], locks: 1 };
		mocks.getUserProjectConstitution.mockResolvedValueOnce(constitution);
		const res = await route.GET(ev('acme/ledger'));
		expect(await res.json()).toEqual({ constitution });
		expect(mocks.getUserProjectConstitution.mock.calls[0][1]).toBe('u1');
		expect(mocks.getUserProjectConstitution.mock.calls[0][2]).toBe('acme/ledger');
	});

	it('returns { constitution: null } honestly when none is federated yet', async () => {
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({ constitution: null });
	});

	it('maps an AdminError(500) to 500 (fail-closed, never a fixture)', async () => {
		mocks.getUserProjectConstitution.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});

	it('propagates a 401 without reading', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.getUserProjectConstitution).not.toHaveBeenCalled();
	});
});

// The caller's own projects, read with THEIR JWT: the projects policy decides the
// rows, not only the route's user filter (#236).
describe('runs as the signed-in person (#236)', () => {
	it('reads with the caller\'s JWT, never the service role', async () => {
		await route.GET(ev());
		expect(mocks.getUserProjectConstitution.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'u1', db: { client: 'service' } });
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.getUserProjectConstitution).not.toHaveBeenCalled();
	});
});
