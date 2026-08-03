// Route tests for the user-wide contributions endpoints (F5): GET
// /v1/you/contributions ({ mine, downstream }) and POST …/adopt (Pin). Caller
// resolved from the JWT; fail-closed → status; 401 propagates. Store mocked.
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
	userMembershipIds: vi.fn(),
	listUserContributions: vi.fn(),
	listUserDownstream: vi.fn(),
	adoptDownstream: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/contributions-data', () => ({
	userMembershipIds: mocks.userMembershipIds,
	listUserContributions: mocks.listUserContributions,
	listUserDownstream: mocks.listUserDownstream,
	adoptDownstream: mocks.adoptDownstream,
	AdminError
}));

const getRoute = await import('./+server');
const adoptRoute = await import('./adopt/+server');

function ev(body?: unknown) {
	return {
		request: new Request('http://x/', {
			method: body ? 'POST' : 'GET',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: body ? JSON.stringify(body) : undefined
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', db: {} });
	mocks.userMembershipIds.mockClear().mockResolvedValue(['m1']);
	mocks.listUserContributions.mockClear().mockResolvedValue([]);
	mocks.listUserDownstream.mockClear().mockResolvedValue([]);
	mocks.adoptDownstream.mockClear().mockResolvedValue(undefined);
});

describe('GET /v1/you/contributions', () => {
	it('returns { mine, downstream } for the caller', async () => {
		mocks.listUserContributions.mockResolvedValueOnce([{ title: 'a' }]);
		mocks.listUserDownstream.mockResolvedValueOnce([{ title: 'b' }]);
		const res = await getRoute.GET(ev());
		expect(await res.json()).toEqual({ mine: [{ title: 'a' }], downstream: [{ title: 'b' }] });
		expect(mocks.listUserContributions.mock.calls[0][1]).toBe('u1');
	});
	it('maps AdminError(500) → 500; propagates 401 without reading', async () => {
		mocks.listUserContributions.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await getRoute.GET(ev())).status).toBe(500);
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await getRoute.GET(ev())).status).toBe(401);
	});
});

describe('POST /v1/you/contributions/adopt', () => {
	it('pins the artifact for the caller and returns ok', async () => {
		const res = await adoptRoute.POST(ev({ artifactId: 'art-1' }));
		expect(await res.json()).toEqual({ ok: true });
		expect(mocks.adoptDownstream.mock.calls[0][2]).toBe('art-1');
	});
	it('400 when artifactId is missing (no write)', async () => {
		expect((await adoptRoute.POST(ev({}))).status).toBe(400);
		expect(mocks.adoptDownstream).not.toHaveBeenCalled();
	});
	it('maps an AdminError(403 no membership) → 403', async () => {
		mocks.adoptDownstream.mockRejectedValueOnce(new AdminError(403, 'no membership'));
		expect((await adoptRoute.POST(ev({ artifactId: 'a' }))).status).toBe(403);
	});
});
