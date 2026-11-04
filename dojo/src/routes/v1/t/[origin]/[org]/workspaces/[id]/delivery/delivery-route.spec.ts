// Route tests for GET /v1/t/{origin}/{org}/workspaces/{id}/delivery and
// …/delivery/tickets (tracker-delivery-metrics D4, D5; #352). The wiring, not the
// reads (delivery-data.spec.ts, delivery_facts.sql): the caller first, ids checked
// before anything is read, the tenant from the path, the reads as the caller, and
// each refusal keeping its status.
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
	resolveTenantId: vi.fn(),
	getWorkspaceDelivery: vi.fn(),
	getDeliveryTickets: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({ client: 'service' }),
	userDb: (token: string) => ({ client: 'user', token })
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	resolveTenantId: mocks.resolveTenantId,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/delivery-data', () => ({
	getWorkspaceDelivery: mocks.getWorkspaceDelivery,
	getDeliveryTickets: mocks.getDeliveryTickets
}));

const boards = await import('./+server');
const tickets = await import('./tickets/+server');

const USER = { client: 'user', token: 'jwt-A' };
const WID = 'c0000000-0000-4000-8000-0000000000f4';
const BID = 'f2000000-0000-4000-8000-000000000001';

function ev(id: string, query = '') {
	return {
		params: { origin: 'organization', org: 'acme', id },
		request: new Request('http://x/', { headers: { authorization: 'Bearer jwt-A' } }),
		locals: {},
		url: new URL(`http://x/${query}`)
	} as never;
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.resolveCaller.mockResolvedValue({ userId: 'u1', db: { client: 'service' }, accessToken: 'jwt-A' });
	mocks.resolveTenantId.mockResolvedValue('t-acme');
	mocks.getWorkspaceDelivery.mockResolvedValue({ boards: [] });
	mocks.getDeliveryTickets.mockResolvedValue([{ key: 'AUTH-2' }]);
});

describe('GET …/workspaces/{id}/delivery', () => {
	it('reads the addressed tenant’s workspace as the caller', async () => {
		const res = await boards.GET(ev(WID));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ boards: [] });
		expect(mocks.getWorkspaceDelivery).toHaveBeenCalledWith(USER, 't-acme', WID);
	});

	it('a malformed id is 400 before the tenant or anything else is read', async () => {
		const res = await boards.GET(ev('not-a-uuid'));
		expect(res.status).toBe(400);
		expect(mocks.resolveTenantId).not.toHaveBeenCalled();
		expect(mocks.getWorkspaceDelivery).not.toHaveBeenCalled();
	});

	it('the caller first: an unauthenticated request never resolves the tenant', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('no', { status: 401 }));
		const res = await boards.GET(ev(WID));
		expect(res.status).toBe(401);
		expect(mocks.resolveTenantId).not.toHaveBeenCalled();
	});

	it('a refusal keeps its status: 403 not a reader, 404 not this dōjō’s', async () => {
		for (const status of [403, 404]) {
			mocks.getWorkspaceDelivery.mockRejectedValueOnce(new AdminError(status, 'no'));
			expect((await boards.GET(ev(WID))).status).toBe(status);
		}
	});
});

describe('GET …/workspaces/{id}/delivery/tickets', () => {
	it('reads one sprint’s tickets', async () => {
		const res = await tickets.GET(ev(WID, `?board=${BID}&sprint=101`));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ tickets: [{ key: 'AUTH-2' }] });
		expect(mocks.getDeliveryTickets).toHaveBeenCalledWith(USER, 't-acme', WID, BID, { sprint: '101', backlog: false });
	});

	it('reads the backlog', async () => {
		await tickets.GET(ev(WID, `?board=${BID}&backlog=1`));
		expect(mocks.getDeliveryTickets).toHaveBeenCalledWith(USER, 't-acme', WID, BID, { sprint: undefined, backlog: true });
	});

	it('a missing or malformed board is 400; a sprint and the backlog together is 400', async () => {
		for (const q of ['', '?board=x', `?board=${BID}&sprint=101&backlog=1`]) {
			expect((await tickets.GET(ev(WID, q))).status, q).toBe(400);
		}
		expect(mocks.getDeliveryTickets).not.toHaveBeenCalled();
	});
});
