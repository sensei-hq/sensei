// Route tests for GET /v1/you/graph/schema (#268): one branch's schema as
// dbd's SchemaModel v3 for the rokkit ER views.
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
	readBranchSchema: vi.fn(),
	assembleSchemaModel: vi.fn()
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
	readBranchSchema: mocks.readBranchSchema
}));
vi.mock('$lib/server/graph-schema', () => ({
	assembleSchemaModel: mocks.assembleSchemaModel
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
	mocks.readBranchSchema.mockResolvedValue({ entities: [{ fqn: 'e' }], deps: [] });
	mocks.assembleSchemaModel.mockReturnValue({ version: 3, tables: [] });
});

describe('GET /v1/you/graph/schema', () => {
	it('assembles the branch schema into the v3 payload', async () => {
		const res = await route.GET(get('repo_key=k&branch=b'));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ version: 3, tables: [] });
		expect(mocks.readBranchSchema).toHaveBeenCalledWith(expect.anything(), 'r1', 'b');
		// enum_values is not synced: the route supplies the named empty.
		expect(mocks.assembleSchemaModel).toHaveBeenCalledWith([{ fqn: 'e', enum_values: [] }], []);
	});

	it('missing repo_key or branch is 400', async () => {
		expect((await route.GET(get('branch=b'))).status).toBe(400);
	});
});
