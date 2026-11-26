// Route tests for GET /v1/you/graph/level0 (#268): one branch's L0 over the
// caller's graph read — params validated, auth and read failures carried.
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
	readableRepository: vi.fn(),
	readBranchGraph: vi.fn(),
	level0: vi.fn(),
	captured: {} as { read?: () => Promise<unknown> }
}));

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
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/graph-reads', () => ({
	readableRepository: mocks.readableRepository,
	readBranchGraph: mocks.readBranchGraph
}));
vi.mock('$lib/server/graph-port', () => ({
	createGraphPort: (read: () => Promise<unknown>) => {
		mocks.captured.read = read;
		return { level0: mocks.level0 };
	}
}));

const route = await import('./+server');

function get(query: string) {
	return {
		request: new Request(`http://x/?${query}`, { headers: { authorization: 'Bearer [REDACTED]' } }),
		locals: {},
		url: new URL(`http://x/?${query}`)
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockResolvedValue({ accessToken: 't' });
	mocks.readableRepository.mockResolvedValue('r1');
	mocks.readBranchGraph.mockResolvedValue({ files: [], nodes: [], edges: [] });
	mocks.level0.mockResolvedValue({ ok: true, data: { level: 'module' } });
});

describe('GET /v1/you/graph/level0', () => {
	it('answers the port L0 for the resolved repository and branch', async () => {
		const res = await route.GET(get('repo_key=github.com%2Facme%2Fapi&branch=main&groupBy=folders'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ ok: true, data: { level: 'module' } });
		expect(mocks.readableRepository).toHaveBeenCalledWith(expect.anything(), 'github.com/acme/api');
		await mocks.captured.read?.();
		expect(mocks.readBranchGraph).toHaveBeenCalledWith(expect.anything(), 'r1', 'main');
		expect(mocks.level0).toHaveBeenCalledWith('folders', null);
	});

	it('groupBy defaults to layers; focus and segment pass through', async () => {
		await route.GET(get('repo_key=k&branch=b&segment=L0'));
		expect(mocks.level0).toHaveBeenCalledWith('layers', { kind: 'segment', id: 'L0' });
	});

	it('focus and segment together are 400', async () => {
		const res = await route.GET(get('repo_key=k&branch=b&focus=a&segment=L0'));
		expect(res.status).toBe(400);
	});

	it('a bad groupBy is 400', async () => {
		const res = await route.GET(get('repo_key=k&branch=b&groupBy=nope'));
		expect(res.status).toBe(400);
	});

	it('missing repo_key or branch is 400', async () => {
		expect((await route.GET(get('branch=b'))).status).toBe(400);
		expect((await route.GET(get('repo_key=k'))).status).toBe(400);
	});

	it('an unknown repository is the resolver\u2019s 404', async () => {
		mocks.readableRepository.mockRejectedValueOnce(new AdminError(404, 'unknown_repository: k'));
		const res = await route.GET(get('repo_key=k&branch=b'));
		expect(res.status).toBe(404);
	});
});
