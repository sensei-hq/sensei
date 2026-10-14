// Route tests for /v1/you/graph (#225): POST files a batch of a repository's
// graph slices, GET reads back the hashes the dōjō holds for one branch.
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
	ingestGraph: vi.fn(),
	graphHashes: vi.fn()
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
vi.mock('$lib/server/graph-ingest', () => ({ ingestGraph: mocks.ingestGraph, graphHashes: mocks.graphHashes }));

const route = await import('./+server');

function post(body: unknown, headers: Record<string, string> = {}) {
	return {
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json', ...headers },
			body: JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

function get(query: string) {
	return {
		request: new Request(`http://x/?${query}`, { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL(`http://x/?${query}`)
	} as never;
}

const PUSH = { repo_key: 'github.com/acme/api', commit: { branch: 'main', sha: 'c', committed_at: 't' }, slices: [] };

beforeEach(() => {
	mocks.resolveCaller
		.mockClear()
		.mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: null, db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.ingestGraph.mockClear().mockResolvedValue({ files: 0, refused_older: 0, pruned: 0 });
	mocks.graphHashes.mockClear().mockResolvedValue({ files: {} });
});

describe('POST /v1/you/graph', () => {
	it('files the batch as the signed-in person, through their own client', async () => {
		mocks.ingestGraph.mockResolvedValue({ files: 2, refused_older: 1, pruned: 0 });
		const res = await route.POST(post(PUSH));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ files: 2, refused_older: 1, pruned: 0 });
		expect(mocks.ingestGraph.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		expect(mocks.ingestGraph.mock.calls[0][1]).toEqual(PUSH);
	});

	it('refuses an oversized body before reading it', async () => {
		const res = await route.POST(post(PUSH, { 'content-length': String(64 * 1024 * 1024) }));
		expect(res.status).toBe(413);
		expect(mocks.ingestGraph).not.toHaveBeenCalled();
	});

	it('keeps a refusal as its status, with its reason', async () => {
		mocks.ingestGraph.mockRejectedValue(new AdminError(422, 'branch_not_tracked: x'));
		const res = await route.POST(post(PUSH));
		expect(res.status).toBe(422);
		expect((await res.json()).error).toContain('branch_not_tracked');
	});

	it('answers an unauthenticated caller as resolveCaller does', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('no', { status: 401 }));
		expect((await route.POST(post(PUSH))).status).toBe(401);
		expect(mocks.ingestGraph).not.toHaveBeenCalled();
	});
});

describe('GET /v1/you/graph', () => {
	it('reads one branch\'s hashes for the caller, from the query', async () => {
		mocks.graphHashes.mockResolvedValue({ files: { 'a.ts': 'h' } });
		const res = await route.GET(get('repo_key=github.com%2Facme%2Fapi&branch=main'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ files: { 'a.ts': 'h' } });
		expect(mocks.graphHashes.mock.calls[0]).toEqual([{ client: 'user', token: 'jwt-A' }, 'github.com/acme/api', 'main']);
	});

	it('keeps a refusal as its status', async () => {
		mocks.graphHashes.mockRejectedValue(new AdminError(404, 'unknown_repository'));
		expect((await route.GET(get('repo_key=x&branch=main'))).status).toBe(404);
	});
});
