// Route tests for POST /v1/you/delivery (tracker-delivery-metrics D4, #352): the
// caller first, the byte cap before the parse, the shape before the database, the
// caller's own client for the write, and each refusal keeping its status.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), fileDelivery: vi.fn() }));

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
vi.mock('$lib/server/delivery-ingest', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/server/delivery-ingest')>()),
	fileDelivery: mocks.fileDelivery
}));

const route = await import('./+server');

const valid = {
	v: 1,
	tenant_id: 'b0000000-0000-4000-8000-00000000052a',
	repo_keys: ['github.com/acme/api'],
	board_key: 'ab'.repeat(32),
	kind: 'jira',
	board_name: 'Mobile Scrum',
	facts: { history_at: null, pending_tickets: 0, predictability: null, predictability_reason: 'too_few_sprints', sprints: [], tickets: [] },
	effort: null
};

function post(raw: string, headers: Record<string, string> = {}) {
	return {
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json', ...headers },
			body: raw
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockReset().mockResolvedValue({ accessToken: 'jwt', principalId: 'p' });
	mocks.fileDelivery.mockReset().mockResolvedValue({ facts: 'accepted' });
});

describe('POST /v1/you/delivery', () => {
	it('files a valid board with the caller’s own client and answers its outcome', async () => {
		const res = await route.POST(post(JSON.stringify(valid)));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ facts: 'accepted' });
		expect(mocks.fileDelivery.mock.calls[0][0]).toEqual({ client: 'user', token: 'jwt' });
	});

	it('an unauthenticated caller is refused before anything is read', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('no', { status: 401 }));
		const res = await route.POST(post(JSON.stringify(valid)));
		expect(res.status).toBe(401);
		expect(mocks.fileDelivery).not.toHaveBeenCalled();
	});

	it('a declared body over 4 MB is 413 before the parse', async () => {
		const res = await route.POST(post('{', { 'content-length': String(4 * 1024 * 1024 + 1) }));
		expect(res.status).toBe(413);
		expect(mocks.fileDelivery).not.toHaveBeenCalled();
	});

	it('a body over 4 MB with no declared length is 413 too', async () => {
		const big = JSON.stringify({ ...valid, board_name: 'x'.repeat(4 * 1024 * 1024) });
		const res = await route.POST(post(big));
		expect(res.status).toBe(413);
		expect(mocks.fileDelivery).not.toHaveBeenCalled();
	});

	it('a streamed body with no declared length is cut off at 4 MB, not read whole', async () => {
		let pulled = 0;
		let cancelled = false;
		const chunk = new Uint8Array(256 * 1024).fill(0x61);
		const stream = new ReadableStream<Uint8Array>({
			pull(c) {
				if (pulled >= 40 * 1024 * 1024) return c.close();
				pulled += chunk.length;
				c.enqueue(chunk);
			},
			cancel() {
				cancelled = true;
			}
		});
		const request = new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: stream,
			duplex: 'half'
		} as RequestInit);
		const res = await route.POST({ request, locals: {}, url: new URL('http://x/') } as never);
		expect(res.status).toBe(413);
		expect(cancelled).toBe(true);
		expect(pulled).toBeLessThan(6 * 1024 * 1024);
		expect(mocks.fileDelivery).not.toHaveBeenCalled();
	});

	it('the cap counts bytes, not characters: 2.2 million é is over 4 MB', async () => {
		const res = await route.POST(post(JSON.stringify({ ...valid, board_name: 'é'.repeat(2_200_000) })));
		expect(res.status).toBe(413);
	});

	it('exactly 4 MiB is let through to the shape check; one byte more is 413', async () => {
		const at = 'x'.repeat(4 * 1024 * 1024);
		expect((await route.POST(post(at))).status).toBe(400);
		expect((await route.POST(post(at + 'x'))).status).toBe(413);
	});

	it('malformed JSON or a wrong shape is 400 and never reaches the database', async () => {
		for (const raw of ['not json', JSON.stringify({ ...valid, v: 2 })]) {
			const res = await route.POST(post(raw));
			expect(res.status, raw.slice(0, 20)).toBe(400);
		}
		expect(mocks.fileDelivery).not.toHaveBeenCalled();
	});

	it('a refusal keeps its status and message: 403, 404 workspace_not_filed, 409 workspace_ambiguous', async () => {
		for (const [status, message] of [
			[403, 'a board names a repository the caller may not sync in this dōjō'],
			[404, 'workspace_not_filed'],
			[409, 'workspace_ambiguous']
		] as const) {
			mocks.fileDelivery.mockRejectedValueOnce(new AdminError(status, message));
			const res = await route.POST(post(JSON.stringify(valid)));
			expect(res.status).toBe(status);
			expect(await res.json()).toEqual({ error: message });
		}
	});
});
