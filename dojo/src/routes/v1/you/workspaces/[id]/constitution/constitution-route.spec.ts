// Route-level tests for GET /v1/you/workspaces/{id}/constitution — the per-workspace
// constitution read (F4), by workspace id (#299: a slug is unique only within one
// dōjō). Caller resolved from the JWT; the read runs with the caller's client, so
// RLS (dojo.can_read_workspace) authorizes it. Covers the
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

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), getWorkspaceConstitution: vi.fn() }));

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
vi.mock('$lib/server/workspaces-data', () => ({
	getWorkspaceConstitution: mocks.getWorkspaceConstitution,
	AdminError
}));

const route = await import('./+server');

function ev(id = 'proj-1') {
	return {
		params: { id },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.getWorkspaceConstitution.mockClear().mockResolvedValue(null);
});

describe('GET /v1/you/workspaces/{id}/constitution', () => {
	it('returns { constitution } for that workspace id', async () => {
		const constitution = { rules: [{ scope_key: 'organization', title: 'x', enforcement: 'mandatory' }], conflicts: [], locks: 1 };
		mocks.getWorkspaceConstitution.mockResolvedValueOnce(constitution);
		const res = await route.GET(ev('proj-1'));
		expect(await res.json()).toEqual({ constitution });
		expect(mocks.getWorkspaceConstitution.mock.calls[0][1]).toBe('proj-1');
	});

	it('returns { constitution: null } honestly when none is federated yet', async () => {
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({ constitution: null });
	});

	it('maps an AdminError(500) to 500 (fail-closed, never a fixture)', async () => {
		mocks.getWorkspaceConstitution.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});

	it('propagates a 401 without reading', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.getWorkspaceConstitution).not.toHaveBeenCalled();
	});
});

// Read with the caller's JWT: the workspaces policy decides what they may read (#236, #299).
describe('runs as the signed-in person (#236)', () => {
	it('reads with the caller\'s JWT, never the service role', async () => {
		await route.GET(ev());
		expect(mocks.getWorkspaceConstitution.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'u1', db: { client: 'service' } });
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.getWorkspaceConstitution).not.toHaveBeenCalled();
	});
});
