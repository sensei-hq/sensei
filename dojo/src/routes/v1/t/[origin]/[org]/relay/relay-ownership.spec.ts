// Relay data is the OWNER's (#249). A user runs tasks on their own machine;
// another member of the tenant must not see their gates, segments or replies,
// and must never answer their approval gates.
//
// These five routes used to scope by `membershipIdsForTenant` — every
// membership in the tenant — at `ACCESS.member`, on a service-role client that
// bypasses RLS. Any member could read every member's pending gates (payloads
// included) and answer them. RLS on these tables (`owns_membership`) already
// said own-rows-only; the routes overrode it.
//
// Each route must now filter on the CALLER's membership and nothing wider.
// A caller with no membership in the tenant is a 403, never a tenant-wide read.
import { describe, it, expect, vi, beforeEach } from 'vitest';

type Terminal = { data: unknown; error: unknown };
type Call = { table: string; op: string; args: unknown[] };

// A chainable, awaitable stub of the supabase-js builder. Every filter call is
// recorded against the table it was made on, so a test can ask "which
// membership did the relay_inbox read filter on?".
function makeDb() {
	const calls: Call[] = [];
	let results: Terminal[] = [];
	let table = '';
	const next = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	const b: Record<string, unknown> = {};
	const rec = (op: string) => (...args: unknown[]) => {
		calls.push({ table, op, args });
		return b;
	};
	b.from = (t: string) => {
		table = t;
		calls.push({ table: t, op: 'from', args: [] });
		return b;
	};
	for (const op of ['select', 'eq', 'in', 'order', 'update', 'insert', 'upsert']) b[op] = rec(op);
	b.maybeSingle = next;
	b.single = next;
	b.then = (ok: (v: Terminal) => unknown, err: (e: unknown) => unknown) => next().then(ok, err);
	return {
		builder: b,
		calls,
		queue: (...r: Terminal[]) => (results = r),
		reset: () => {
			calls.length = 0;
			results = [];
		}
	};
}

const db = makeDb();
const MINE = 'mem-A';
let caller: {
	tenantId: string;
	userId: string;
	role: string;
	access: number;
	membershipId?: string;
	accessToken?: string;
};
const tenantWide = vi.fn(async () => {
	throw new Error('membershipIdsForTenant must not be called: relay data is the owner\'s');
});

// Which client each route read with: the service role, or the CALLER's JWT.
const clients = { service: 0, userTokens: [] as string[] };
vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => {
		clients.service += 1;
		return db.builder;
	},
	userDb: (token: string) => {
		clients.userTokens.push(token);
		return db.builder;
	}
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: vi.fn(async () => caller),
	membershipIdsForTenant: tenantWide,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));

const { GET: gatesGET } = await import('./gates/+server');
const { POST: replyPOST } = await import('./reply/+server');
const { GET: segmentsGET } = await import('./segments/+server');
const { GET: sessionGET } = await import('./session/+server');
const { POST: reviewPOST } = await import('./review/+server');
const { POST: nudgePOST } = await import('./nudge/+server');

function ev(opts: { body?: unknown; query?: string } = {}) {
	const url = new URL(`http://x/${opts.query ? `?${opts.query}` : ''}`);
	const request = new Request(url, opts.body === undefined ? {} : { method: 'POST', body: JSON.stringify(opts.body) });
	return { params: { origin: 'personal', org: 'dev' }, request, locals: {}, url } as never;
}

/** The membership filter a route applied on `table`, and whether it went wide. */
function scopeOn(table: string) {
	const onTable = db.calls.filter((c) => c.table === table);
	return {
		eq: onTable.filter((c) => c.op === 'eq' && c.args[0] === 'membership_id').map((c) => c.args[1]),
		inList: onTable.filter((c) => c.op === 'in' && c.args[0] === 'membership_id')
	};
}

const routes: Array<{ name: string; table: string; run: () => Promise<Response>; queue: Terminal[] }> = [
	{
		name: 'gates GET',
		table: 'relay_inbox',
		run: async () => gatesGET(ev()),
		queue: [{ data: [], error: null }]
	},
	{
		name: 'reply POST',
		table: 'relay_inbox',
		run: async () => replyPOST(ev({ body: { inbox_id: 'gate-of-B', reply: { approve: true } } })),
		queue: [{ data: { id: 'gate-of-B', seq: 3 }, error: null }]
	},
	{
		name: 'segments GET',
		table: 'relay_sessions',
		run: async () => segmentsGET(ev({ query: 'run_id=run-1' })),
		queue: [{ data: null, error: null }]
	},
	{
		name: 'session GET',
		table: 'relay_sessions',
		run: async () => sessionGET(ev()),
		queue: [{ data: [], error: null }]
	},
	{
		name: 'review POST',
		table: 'relay_sessions',
		run: async () => reviewPOST(ev({ body: { run_id: 'run-1', reviews: [] } })),
		queue: [{ data: null, error: null }]
	},
	{
		name: 'nudge POST',
		table: 'relay_sessions',
		run: async () => nudgePOST(ev({ body: { run_id: 'run-1', text: 'focus' } })),
		queue: [{ data: { id: 's1' }, error: null }, { data: { id: 'n1', seq: 1 }, error: null }]
	}
];

beforeEach(() => {
	db.reset();
	tenantWide.mockClear();
	caller = { tenantId: 't1', userId: 'user-A', role: 'member', access: 0, membershipId: MINE, accessToken: 'jwt-A' };
	clients.service = 0;
	clients.userTokens.length = 0;
});

describe('relay routes are scoped to the caller’s own membership (#249)', () => {
	for (const r of routes) {
		it(`${r.name} filters on the caller’s membership, never the tenant’s`, async () => {
			db.queue(...r.queue);
			const res = await r.run();
			expect(res.status).toBeLessThan(500);
			const scope = scopeOn(r.table);
			expect(scope.eq, `${r.name}: membership filter on ${r.table}`).toEqual([MINE]);
			expect(scope.inList, `${r.name}: no tenant-wide membership list`).toEqual([]);
			expect(tenantWide).not.toHaveBeenCalled();
		});

		it(`${r.name} is a 403 for a caller with no membership in the tenant`, async () => {
			caller = { ...caller, membershipId: undefined };
			db.queue(...r.queue);
			const res = await r.run();
			expect(res.status).toBe(403);
			expect(db.calls.filter((c) => c.table.startsWith('relay_')), 'no relay table is touched').toEqual([]);
		});
	}
});

// The relay READS run as the person, under the own-rows RLS that already exists
// on relay_sessions / relay_inbox / relay_segments (#236 step 3). The route's
// membership filter stays, but it is no longer the only thing between one
// member and another's gates.
describe('relay reads and writes run as the caller, under RLS (#236 step 3)', () => {
	const reads = routes.filter((r) =>
		['gates GET', 'segments GET', 'session GET', 'reply POST', 'review POST', 'nudge POST'].includes(
			r.name
		)
	);
	for (const r of reads) {
		it(`${r.name} runs with the caller's JWT, never the service role`, async () => {
			db.queue(...r.queue);
			await r.run();
			expect(clients.userTokens).toEqual(['jwt-A']);
			expect(clients.service).toBe(0);
		});
	}
});
