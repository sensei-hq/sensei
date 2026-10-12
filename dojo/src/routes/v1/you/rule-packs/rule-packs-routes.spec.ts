// Route tests for the rule-pack library and adoption (/v1/you/rule-packs):
//   GET  …/rule-packs            → { packs, adopted }
//   POST …/rule-packs/{slug}/adopt → { adopted }
// Both run as the signed-in person (#236): the library is a global read, the
// adoptions are the caller's own, and adopting writes their own namespace — the
// database derives whose from the session. The store is mocked (its logic is in
// rulepacks-data.spec; the database's in database/tests/dojo/rule_packs_access.sql).
import { describe, it, expect, vi, beforeEach } from 'vitest';

class RulePacksError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveCaller: vi.fn(),
	listLibraryPacks: vi.fn(),
	listAdoptedPackSlugs: vi.fn(),
	setPackAdoption: vi.fn()
}));

// The service role vs the caller's own JWT — which client each store call got.
vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/rulepacks-data', () => ({
	listLibraryPacks: mocks.listLibraryPacks,
	listAdoptedPackSlugs: mocks.listAdoptedPackSlugs,
	setPackAdoption: mocks.setPackAdoption,
	RulePacksError
}));

const library = await import('./+server');
const adopt = await import('./[slug]/adopt/+server');

const USER = { client: 'user', token: 'jwt-A' };

function ev(body?: unknown) {
	return {
		params: { slug: 'clean-code' },
		request: new Request('http://x/', {
			method: body === undefined ? 'GET' : 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller
		.mockClear()
		.mockResolvedValue({ userId: 'p1', email: 'ada@example.com', db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.listLibraryPacks.mockClear().mockResolvedValue([{ slug: 'clean-code' }]);
	mocks.listAdoptedPackSlugs.mockClear().mockResolvedValue(['clean-code']);
	mocks.setPackAdoption.mockClear().mockResolvedValue(true);
});

describe('GET /v1/you/rule-packs', () => {
	it('returns the library and the caller\'s adoptions, both read with their JWT', async () => {
		const res = await library.GET(ev());
		expect(await res.json()).toEqual({ packs: [{ slug: 'clean-code' }], adopted: ['clean-code'] });
		expect(mocks.listLibraryPacks.mock.calls[0]).toEqual([USER]);
		expect(mocks.listAdoptedPackSlugs.mock.calls[0]).toEqual([USER]);
	});
	it('refuses a caller with no session rather than reading as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await library.GET(ev())).status).toBe(401);
		expect(mocks.listLibraryPacks).not.toHaveBeenCalled();
	});
});

describe('POST /v1/you/rule-packs/{slug}/adopt', () => {
	it('adopts as the caller — no user id leaves the route', async () => {
		const res = await adopt.POST(ev({}));
		expect(await res.json()).toEqual({ adopted: true });
		expect(mocks.setPackAdoption.mock.calls[0]).toEqual([USER, 'clean-code', 'ada@example.com', true]);
	});
	it('drops on { adopt: false }', async () => {
		await adopt.POST(ev({ adopt: false }));
		expect(mocks.setPackAdoption.mock.calls[0][3]).toBe(false);
	});
	it('404s an unknown pack', async () => {
		mocks.setPackAdoption.mockResolvedValueOnce(false);
		expect((await adopt.POST(ev({}))).status).toBe(404);
	});
	it('refuses a caller with no session rather than adopting as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await adopt.POST(ev({}))).status).toBe(401);
		expect(mocks.setPackAdoption).not.toHaveBeenCalled();
	});
});
