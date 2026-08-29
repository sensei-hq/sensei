// Route tests for PATCH /v1/you/repositories/election.
//
// The wiring, not the election logic (that is elections.spec.ts): that the
// caller's PRINCIPAL id is what the election is keyed on, that a bad body is
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
	setElection: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/elections', () => ({ setElection: mocks.setElection }));

const route = await import('./+server');

const DB = { marker: 'db' };
const OUT = {
	repo_key: 'github.com/alice/api',
	elected: true,
	authority: 'user',
	sync_enabled: true,
	reason_code: null
};

function patch(body: unknown) {
	return route.PATCH({
		request: new Request('http://x/v1/you/repositories/election', {
			method: 'PATCH',
			body: JSON.stringify(body),
			headers: { 'content-type': 'application/json' }
		}),
		locals: {}
	} as never);
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.resolveCaller.mockResolvedValue({ userId: 'p-alice', db: DB });
	mocks.setElection.mockResolvedValue(OUT);
});

describe('PATCH /v1/you/repositories/election', () => {
	it('elects on behalf of the resolved PRINCIPAL, not anything in the body', async () => {
		// The body cannot name the principal. If it could, one signed-in user could
		// elect on another's behalf — and under a USER authority the election IS the
		// consent.
		const res = await patch({
			repo_key: 'github.com/alice/api',
			elected: true,
			principal_id: 'p-mallory'
		});
		expect(res.status).toBe(200);
		expect(mocks.setElection).toHaveBeenCalledWith(DB, 'p-alice', 'github.com/alice/api', true);
		expect(await res.json()).toEqual(OUT);
	});

	it('passes elected=false through rather than treating it as absent', async () => {
		// `if (!body.elected)` would make turning sharing OFF indistinguishable from
		// omitting the field — a 400 on the one operation a user most needs to work.
		await patch({ repo_key: 'github.com/alice/api', elected: false });
		expect(mocks.setElection).toHaveBeenCalledWith(DB, 'p-alice', 'github.com/alice/api', false);
	});

	it('refuses a non-boolean `elected` instead of coercing it', async () => {
		// "false" is truthy. Coercion here would turn sharing ON for a client that
		// asked to turn it off — the wrong direction to fail for a disclosure toggle.
		const res = await patch({ repo_key: 'github.com/alice/api', elected: 'false' });
		expect(res.status).toBe(400);
		expect(mocks.setElection).not.toHaveBeenCalled();
	});

	it('refuses a missing repo_key before writing anything', async () => {
		const res = await patch({ elected: true });
		expect(res.status).toBe(400);
		expect(mocks.setElection).not.toHaveBeenCalled();
	});

	it.each([
		[403, 'you may not change sharing'],
		[404, 'no repository for this account'],
		[409, 'has no authority yet']
	])('keeps a %i refusal as %i rather than collapsing it to 500', async (status, message) => {
		mocks.setElection.mockRejectedValue(new AdminError(status, message));
		const res = await patch({ repo_key: 'github.com/alice/api', elected: true });
		expect(res.status).toBe(status);
		expect((await res.json()).error).toBe(message);
	});
});
