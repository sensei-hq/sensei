// Route tests for GET /v1/t/{origin}/{org}/workspaces/{id}/metrics.
//
// The wiring, not the series logic (that is dojo.workspace_metric_series',
// covered in repository_metric_series.sql): that the tenant comes from the
// path, the window from `?days=`, the read runs as the caller, and a refusal
// keeps its status instead of collapsing into a 500.
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
	resolveCaller: vi.fn(),
	resolveTenantId: vi.fn(),
	getWorkspaceMetrics: vi.fn()
}));

// The service role vs the caller's own JWT — which client the store got.
vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	resolveTenantId: mocks.resolveTenantId,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
// One mock class in both modules, so the REAL windowDays (unmocked below)
// throws what the route catches.
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/workspaces-data', () => ({
	getWorkspaceMetrics: mocks.getWorkspaceMetrics,
	AdminError
}));

const route = await import('./+server');

const USER = { client: 'user', token: 'jwt-A' };
const OUT = { since: '2026-10-05', days: 30, rows: [] };
// A well-formed workspace id — the route refuses anything else with 400 before reading.
const WID = 'c0000000-0000-4000-8000-0000000000f4';

function ev(id = WID, days: string | null = null) {
	return {
		params: { origin: 'organization', org: 'acme', id },
		request: new Request('http://x/', { headers: { authorization: 'Bearer [REDACTED]' } }),
		locals: {},
		url: new URL(days === null ? 'http://x/' : `http://x/?days=${days}`)
	} as never;
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.resolveCaller.mockResolvedValue({ userId: 'u1', db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.resolveTenantId.mockResolvedValue('t-acme');
	mocks.getWorkspaceMetrics.mockResolvedValue(OUT);
});

describe('GET /v1/t/{origin}/{org}/workspaces/{id}/metrics', () => {
	it('returns the workspace series for the addressed tenant and workspace', async () => {
		const res = await route.GET(ev());
		expect(res.status).toBe(200);
		expect(mocks.resolveTenantId).toHaveBeenCalledWith({ client: 'service' }, 'organization', 'acme');
		expect(mocks.getWorkspaceMetrics).toHaveBeenCalledWith(USER, 't-acme', WID, {
			days: 30,
			today: expect.any(Date)
		});
		expect(await res.json()).toEqual(OUT);
	});

	it('narrows the window from ?days=', async () => {
		await route.GET(ev(WID, '7'));
		expect(mocks.getWorkspaceMetrics).toHaveBeenCalledWith(USER, 't-acme', WID, {
			days: 7,
			today: expect.any(Date)
		});
	});

	it('refuses a bad window with 400 before reading anything', async () => {
		const res = await route.GET(ev(WID, 'abc'));
		expect(res.status).toBe(400);
		expect(mocks.getWorkspaceMetrics).not.toHaveBeenCalled();
	});

	it.each([
		[403, 'you may not read this workspace’s metrics'],
		[404, 'no such workspace in this dōjō']
	])('keeps a %i refusal as %i rather than collapsing it to 500', async (status, message) => {
		mocks.getWorkspaceMetrics.mockRejectedValue(new AdminError(status, message));
		const res = await route.GET(ev());
		expect(res.status).toBe(status);
		expect((await res.json()).error).toBe(message);
	});

	it('fails closed with 500 on a series error — never []', async () => {
		mocks.getWorkspaceMetrics.mockRejectedValue(new AdminError(500, 'connection reset'));
		const res = await route.GET(ev());
		expect(res.status).toBe(500);
	});

	it('propagates an unknown tenant as 404 without reading', async () => {
		mocks.resolveTenantId.mockRejectedValueOnce(
			new Response(JSON.stringify({ error: 'no such tenant' }), { status: 404 })
		);
		const res = await route.GET(ev());
		expect(res.status).toBe(404);
		expect(mocks.getWorkspaceMetrics).not.toHaveBeenCalled();
	});

	it('refuses a malformed workspace id with 400 — a non-uuid is the caller’s input, not a server failure', async () => {
		// Without this the id reaches can_read_workspace, which raises 22P02,
		// and db-refusal turns that into a 500 "server failure".
		const res = await route.GET(ev('not-a-uuid'));
		expect(res.status).toBe(400);
		expect(mocks.getWorkspaceMetrics).not.toHaveBeenCalled();
	});

	it('authenticates before resolving the tenant — an anonymous request learns nothing about which slugs exist', async () => {
		// Resolving the tenant first would answer 404 for an unknown org and
		// 401 for a real one, so anyone could probe which dōjō slugs exist.
		// The house pattern (resolveTenantAccess) authenticates first.
		// Nothing queued on resolveTenantId: a queued rejection the route never
		// reaches would leak into the next test (clearAllMocks drains calls,
		// not once-queues). The not-called assertion is the regression.
		mocks.resolveCaller.mockRejectedValueOnce(
			new Response(JSON.stringify({ error: 'unauthenticated' }), { status: 401 })
		);
		const res = await route.GET(ev());
		expect(res.status).toBe(401);
		expect(mocks.resolveTenantId).not.toHaveBeenCalled();
		expect(mocks.getWorkspaceMetrics).not.toHaveBeenCalled();
	});
});

describe('GET workspaces/{id}/metrics without a session', () => {
	it('refuses rather than reading as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'u1', db: { client: 'service' } });
		const res = await route.GET(ev());
		expect(res.status).toBe(401);
		expect(mocks.getWorkspaceMetrics).not.toHaveBeenCalled();
	});
});
