// Route tests for PUT/DELETE /v1/you/repositories/graph-branches.
//
// The wiring, not the tracking logic (that is dojo.set_graph_branch's, covered
// in graph_branches_access.sql): that a track reaches the store with the label,
// that an untrack is a DELETE rather than a PUT of false, that a bad body is
// refused before any write, and that a refusal keeps its status instead of
// collapsing into a 500.
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
	setGraphBranch: vi.fn()
}));

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
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/repositories', () => ({ setGraphBranch: mocks.setGraphBranch }));

const route = await import('./+server');

const USER = { client: 'user', token: 'jwt-A' };
const OUT = {
	repo_key: 'github.com/alice/api',
	branch: 'develop',
	tracked: true,
	is_default: false
};

function put(body: unknown) {
	return route.PUT({
		request: new Request('http://x/v1/you/repositories/graph-branches', {
			method: 'PUT',
			body: JSON.stringify(body),
			headers: { 'content-type': 'application/json' }
		}),
		locals: {}
	} as never);
}

function del(body: unknown) {
	return route.DELETE({
		request: new Request('http://x/v1/you/repositories/graph-branches', {
			method: 'DELETE',
			body: JSON.stringify(body),
			headers: { 'content-type': 'application/json' }
		}),
		locals: {}
	} as never);
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.resolveCaller.mockResolvedValue({ userId: 'p-alice', db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.setGraphBranch.mockResolvedValue(OUT);
});

describe('PUT /v1/you/repositories/graph-branches', () => {
	it('adds_a_tracked_branch_for_a_configurable_repository', async () => {
		const res = await put({
			repo_key: 'github.com/alice/api',
			branch: 'develop',
			label: 'development'
		});
		expect(res.status).toBe(200);
		expect(mocks.setGraphBranch).toHaveBeenCalledWith(
			USER,
			'github.com/alice/api',
			'develop',
			true,
			'development'
		);
		expect(await res.json()).toEqual(OUT);
	});

	it('tracks without a label when none is given', async () => {
		// The label is optional: the forge default tracks with none, and a team
		// may track a branch before deciding what environment it is. Absent stays
		// absent — never an empty string the CHECK would refuse.
		await put({ repo_key: 'github.com/alice/api', branch: 'develop' });
		expect(mocks.setGraphBranch).toHaveBeenCalledWith(
			USER,
			'github.com/alice/api',
			'develop',
			true,
			null
		);
	});

	it('an_empty_branch_is_400', async () => {
		const res = await put({ repo_key: 'github.com/alice/api', branch: '  ' });
		expect(res.status).toBe(400);
		expect(mocks.setGraphBranch).not.toHaveBeenCalled();
	});

	it('an_unknown_label_is_400', async () => {
		// The table's CHECK would refuse it as a 500 (db-refusal has no 23514
		// mapping); the route names the three known labels instead.
		const res = await put({ repo_key: 'github.com/alice/api', branch: 'develop', label: 'nightly' });
		expect(res.status).toBe(400);
		expect(mocks.setGraphBranch).not.toHaveBeenCalled();
	});

	it('refuses a missing repo_key before writing anything', async () => {
		const res = await put({ branch: 'develop' });
		expect(res.status).toBe(400);
		expect(mocks.setGraphBranch).not.toHaveBeenCalled();
	});

	it('refuses_a_member_without_authority_with_403', async () => {
		// A contributor of an organisation: the database knows the repository,
		// the caller may not change what its graph tracks. 403, not 404 — the
		// repository IS theirs to see, just not to configure.
		mocks.setGraphBranch.mockRejectedValue(new AdminError(403, 'you may not change which branches this tracks'));
		const res = await put({ repo_key: 'github.com/alice/api', branch: 'develop' });
		expect(res.status).toBe(403);
		expect((await res.json()).error).toBe('you may not change which branches this tracks');
	});

	it.each([
		[404, 'no repository for this account'],
		[409, 'has no authority yet']
	])('keeps a %i refusal as %i rather than collapsing it to 500', async (status, message) => {
		mocks.setGraphBranch.mockRejectedValue(new AdminError(status, message));
		const res = await put({ repo_key: 'github.com/alice/api', branch: 'develop' });
		expect(res.status).toBe(status);
		expect((await res.json()).error).toBe(message);
	});
});

describe('DELETE /v1/you/repositories/graph-branches', () => {
	it('removes_a_tracked_branch', async () => {
		// Untracking is a DELETE, not a PUT of tracked=false: it deletes the
		// branch's graph, and the method says so.
		mocks.setGraphBranch.mockResolvedValue({ ...OUT, tracked: false });
		const res = await del({ repo_key: 'github.com/alice/api', branch: 'develop' });
		expect(res.status).toBe(200);
		expect(mocks.setGraphBranch).toHaveBeenCalledWith(
			USER,
			'github.com/alice/api',
			'develop',
			false,
			null
		);
	});

	it('an_empty_branch_is_400', async () => {
		const res = await del({ repo_key: 'github.com/alice/api', branch: '' });
		expect(res.status).toBe(400);
		expect(mocks.setGraphBranch).not.toHaveBeenCalled();
	});
});

describe('PUT /v1/you/repositories/graph-branches without a session', () => {
	it('refuses rather than tracking as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p-alice', db: { client: 'service' } });
		const res = await put({ repo_key: 'github.com/alice/api', branch: 'develop' });
		expect(res.status).toBe(401);
		expect(mocks.setGraphBranch).not.toHaveBeenCalled();
	});
});
