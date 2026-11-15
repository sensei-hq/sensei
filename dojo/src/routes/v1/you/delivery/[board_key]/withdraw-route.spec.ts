// Route tests for DELETE /v1/you/delivery/{board_key} (tracker-delivery-metrics
// B13, D4; #355): the caller first, the board key, tenant and install checked
// before the database, 204 when something was withdrawn, 404 when nothing was.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), withdrawDelivery: vi.fn() }));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/delivery-ingest', () => ({ withdrawDelivery: mocks.withdrawDelivery }));

const route = await import('./+server');

const key = 'ab'.repeat(32);
const tenant = 'b0000000-0000-4000-8000-00000000052a';
const source = '5a000000-0000-4000-8000-000000000001';

function call(boardKey = key, query = `tenant_id=${tenant}&source=${source}`) {
	const url = new URL(`https://dojo.test/v1/you/delivery/${boardKey}?${query}`);
	return route.DELETE({
		request: new Request(url, { method: 'DELETE', headers: { authorization: 'Bearer t' } }),
		url,
		params: { board_key: boardKey },
		locals: {}
	} as never);
}

beforeEach(() => {
	mocks.resolveCaller.mockReset().mockResolvedValue({ accessToken: 't', principalId: 'p' });
	mocks.withdrawDelivery.mockReset();
});

describe('DELETE /v1/you/delivery/{board_key}', () => {
	it('204 when something was withdrawn, as the caller', async () => {
		mocks.withdrawDelivery.mockResolvedValue(true);
		const res = await call();
		expect(res.status).toBe(204);
		const [client, t, k, s] = mocks.withdrawDelivery.mock.calls[0];
		expect(client).toMatchObject({ client: 'user', token: 't' });
		expect([t, k, s]).toEqual([tenant, key, source]);
	});

	it('404 with its own answer when there was nothing of the caller’s', async () => {
		mocks.withdrawDelivery.mockResolvedValue(false);
		const res = await call();
		expect(res.status).toBe(404);
		expect(await res.json()).toEqual({ withdrawn: false });
	});

	it('400 for a malformed board key, tenant or install, before the database', async () => {
		for (const [k, q] of [
			['nope', `tenant_id=${tenant}&source=${source}`],
			[key, `source=${source}`],
			[key, `tenant_id=${tenant}`],
			[key, `tenant_id=x&source=${source}`]
		]) {
			expect((await call(k, q)).status, `${k} ${q}`).toBe(400);
		}
		expect(mocks.withdrawDelivery).not.toHaveBeenCalled();
	});

	it('the caller first: no session is the auth answer, nothing withdrawn', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('{}', { status: 401 }));
		expect((await call()).status).toBe(401);
		expect(mocks.withdrawDelivery).not.toHaveBeenCalled();
	});

	it('a refusal keeps its status', async () => {
		mocks.withdrawDelivery.mockRejectedValue(new AdminError(403, 'no'));
		expect((await call()).status).toBe(403);
	});
});
