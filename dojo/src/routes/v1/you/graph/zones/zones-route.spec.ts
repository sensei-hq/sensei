// Route tests for GET /v1/you/graph/zones (#268): Martin points per module.
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
	zones: vi.fn()
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
	createGraphPort: () => ({ zones: mocks.zones })
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
	mocks.zones.mockResolvedValue({ ok: true, data: { points: [] } });
});

describe('GET /v1/you/graph/zones', () => {
	it('answers the port zones for the resolved repository and branch', async () => {
		const res = await route.GET(get('repo_key=k&branch=b'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ ok: true, data: { points: [] } });
		expect(mocks.zones).toHaveBeenCalledWith();
	});
});
