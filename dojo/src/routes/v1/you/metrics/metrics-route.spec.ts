// Route tests for POST /v1/you/metrics — the third leg of the daemon cycle, and
// the only one that WRITES. It shipped with no test while all six sibling
// /v1/you/* routes had one, so the body guard, the row cap and the
// Response-vs-AdminError split were all unexercised.
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
	ingestMetrics: vi.fn(),
	readMyMetrics: vi.fn()
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
vi.mock('$lib/server/metrics-ingest', () => ({ ingestMetrics: mocks.ingestMetrics }));
vi.mock('$lib/server/my-metrics-data', () => ({ readMyMetrics: mocks.readMyMetrics }));

const route = await import('./+server');

function post(body: unknown, raw?: string) {
	return {
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: raw ?? JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

function row(over: Record<string, unknown> = {}) {
	return {
		repo_key: 'github.com/acme/api',
		metric: 'commits_per_day',
		scope: 'repo',
		computed_on: '2026-08-27',
		grain: 'daily',
		value: 1,
		...over
	};
}

beforeEach(() => {
	mocks.resolveCaller
		.mockClear()
		.mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: null, db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.ingestMetrics.mockClear().mockResolvedValue({ accepted: 0, rejected: [] });
});

describe('POST /v1/you/metrics', () => {
	it('stores the batch as the signed-in person', async () => {
		// The caller's JWT carries who they are, and the database resolves their
		// principal from it — no id to pass, and none to get wrong (#236).
		mocks.ingestMetrics.mockResolvedValue({ accepted: 1, rejected: [] });
		const res = await route.POST(post({ metrics: [row()] }));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ accepted: 1, rejected: [] });
		expect(mocks.ingestMetrics).toHaveBeenCalledOnce();
		expect(mocks.ingestMetrics.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt-A' });
		expect(mocks.ingestMetrics.mock.calls[0]).toHaveLength(2);
	});

	it('400s when `metrics` is not an array, without attempting the write', async () => {
		for (const body of [{}, { metrics: 'nope' }, { metrics: null }]) {
			const res = await route.POST(post(body));
			expect(res.status).toBe(400);
		}
		expect(mocks.ingestMetrics).not.toHaveBeenCalled();
	});

	it('400s on a body that is not JSON at all', async () => {
		// `request.json()` throws; without the catch this would be a 500, which the
		// daemon classifies as Unreachable and retries forever.
		const res = await route.POST(post(null, '{not json'));
		expect(res.status).toBe(400);
		expect(mocks.ingestMetrics).not.toHaveBeenCalled();
	});

	it('413s on a batch over the cap rather than attempting it', async () => {
		// The comment on MAX_ROWS says an unbounded body "is not a decision a client
		// gets to make". Nothing enforced it.
		const res = await route.POST(post({ metrics: Array.from({ length: 1001 }, () => row()) }));
		expect(res.status).toBe(413);
		expect(mocks.ingestMetrics).not.toHaveBeenCalled();
	});

	it('propagates a 401 from the resolver instead of turning it into a 500', async () => {
		// `resolveCaller` refuses by THROWING a Response. Losing the re-raise makes
		// every unauthenticated push a 500 — which the daemon reads as a transient
		// outage and retries forever instead of telling the user to sign in.
		mocks.resolveCaller.mockRejectedValue(
			new Response(JSON.stringify({ error: 'unauthorized' }), { status: 401 })
		);
		const res = await route.POST(post({ metrics: [row()] }));
		expect(res.status).toBe(401);
	});

	it('maps an AdminError to its own status', async () => {
		mocks.ingestMetrics.mockRejectedValue(new AdminError(500, 'db exploded'));
		const res = await route.POST(post({ metrics: [row()] }));
		expect(res.status).toBe(500);
	});
});

describe('POST /v1/you/metrics without a session', () => {
	it('refuses rather than storing as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await route.POST(post({ metrics: [row()] }))).status).toBe(401);
		expect(mocks.ingestMetrics).not.toHaveBeenCalled();
	});
});

function get(query = '') {
	return {
		request: new Request(`http://x/v1/you/metrics${query}`, { headers: { authorization: 'Bearer jwt' } }),
		locals: {},
		url: new URL(`http://x/v1/you/metrics${query}`)
	} as never;
}

describe('GET /v1/you/metrics', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mocks.resolveCaller.mockResolvedValue({ userId: 'p-alice', db: { client: 'service' }, accessToken: 'jwt-A' });
		mocks.readMyMetrics.mockResolvedValue({ since: '2026-09-23', days: 30, series: {} });
	});

	it('reads the caller\'s own series, as the caller, over a month by default', async () => {
		const res = await route.GET(get());
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ since: '2026-09-23', days: 30, series: {} });
		const [db, opts] = mocks.readMyMetrics.mock.calls[0];
		expect(db).toEqual({ client: 'user', token: 'jwt-A' });
		expect(opts.days).toBe(30);
	});

	it('takes the window from ?days=', async () => {
		await route.GET(get('?days=7'));
		expect(mocks.readMyMetrics.mock.calls[0][1].days).toBe(7);
	});

	it.each(['0', '91', 'abc', '7.5'])('400s on ?days=%s, without reading', async (days) => {
		expect((await route.GET(get(`?days=${days}`))).status).toBe(400);
		expect(mocks.readMyMetrics).not.toHaveBeenCalled();
	});

	it('maps an AdminError to its own status', async () => {
		mocks.readMyMetrics.mockRejectedValueOnce(new AdminError(403, 'no'));
		expect((await route.GET(get())).status).toBe(403);
	});

	it('refuses without a session rather than reading as the service role', async () => {
		mocks.resolveCaller.mockResolvedValueOnce({ userId: 'p1', db: { client: 'service' } });
		expect((await route.GET(get())).status).toBe(401);
		expect(mocks.readMyMetrics).not.toHaveBeenCalled();
	});
});
