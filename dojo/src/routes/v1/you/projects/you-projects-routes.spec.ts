// Route-level tests for GET /v1/you/projects — the USER-WIDE personal projects
// read (every dōjō the caller belongs to). Unlike the tenant-scoped
// /v1/t/{origin}/{org}/projects, there is NO tenant and NO role floor: the
// caller is resolved from the JWT and the read is authorized by a user_id
// filter. Covers the { projects } envelope, AdminError → status (fail-closed,
// never a fixture), and auth-Response (401) propagation. The store is mocked.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), listUserProjects: vi.fn() }));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/projects-data', () => ({ listUserProjects: mocks.listUserProjects, AdminError }));

const route = await import('./+server');

function ev() {
	return {
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', db: {} });
	mocks.listUserProjects.mockClear().mockResolvedValue([]);
});

describe('GET /v1/you/projects', () => {
	it('returns { projects } for the resolved caller (user-wide, no tenant)', async () => {
		mocks.listUserProjects.mockResolvedValueOnce([{ id: 'p1', slug: 'me/x' }]);
		const res = await route.GET(ev());
		expect(await res.json()).toEqual({ projects: [{ id: 'p1', slug: 'me/x' }] });
		// Read is scoped to the JWT-resolved caller, not a path tenant.
		expect(mocks.listUserProjects.mock.calls[0][1]).toBe('u1');
	});

	it('maps an AdminError(500) to 500 (fail-closed, never a fixture)', async () => {
		mocks.listUserProjects.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await route.GET(ev())).status).toBe(500);
	});

	it('propagates a 401 without reading', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await route.GET(ev())).status).toBe(401);
		expect(mocks.listUserProjects).not.toHaveBeenCalled();
	});
});
