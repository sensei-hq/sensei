// Route tests for GET /v1/you/graph/file (#268): one file's symbols with its
// callers and callees.
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
	file: vi.fn()
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
	createGraphPort: () => ({ file: mocks.file })
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
	mocks.file.mockResolvedValue({ ok: true, data: { level: 'symbol' } });
});

describe('GET /v1/you/graph/file', () => {
	it('answers the port L2 for the file path', async () => {
		const res = await route.GET(get('repo_key=k&branch=b&id=shop%2Fcart.ts'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ ok: true, data: { level: 'symbol' } });
		expect(mocks.file).toHaveBeenCalledWith('shop/cart.ts');
	});

	it('missing id is 400', async () => {
		expect((await route.GET(get('repo_key=k&branch=b'))).status).toBe(400);
	});
});
