// Route-level integration tests for the relay push Worker routes (P4.3):
//   POST relay/push/subscribe → upserts dojo.push_subscriptions
//   POST relay/push/prefs     → upserts dojo.notification_prefs
//
// Exercises the subscribe→store round-trip (insert vs update branching), the
// stored write shape ({endpoint,p256dh,auth}, enabled, last_seen, keyed by
// user_id+endpoint), and the defensive 400s — WITHOUT a live Worker/DB by
// mocking the two server deps (resolveTenantAccess + dojoDb). This is the
// "route unit test" round-trip the P4.3 plan asks for.
import { describe, it, expect, vi, beforeEach } from 'vitest';

// A chainable stub over the supabase-js query builder we use in the routes.
// Each terminal (`.maybeSingle()` / `.single()`) resolves whatever the test
// queued; `.insert`/`.update`/`.upsert` capture their argument for assertions.
type Terminal = { data: unknown; error: unknown };
function makeDb() {
	const captured: { table?: string; op?: string; payload?: unknown; filters: [string, unknown][] } = {
		filters: []
	};
	let nextResults: Terminal[] = [];
	const builder: Record<string, unknown> = {};
	const chain = () => builder;
	builder.from = (t: string) => {
		captured.table = t;
		return builder;
	};
	builder.select = chain;
	builder.eq = (col: string, val: unknown) => {
		captured.filters.push([col, val]);
		return builder;
	};
	builder.insert = (p: unknown) => {
		captured.op = 'insert';
		captured.payload = p;
		return builder;
	};
	builder.update = (p: unknown) => {
		captured.op = 'update';
		captured.payload = p;
		return builder;
	};
	builder.upsert = (p: unknown) => {
		captured.op = 'upsert';
		captured.payload = p;
		return builder;
	};
	builder.maybeSingle = () => Promise.resolve(nextResults.shift() ?? { data: null, error: null });
	builder.single = () => Promise.resolve(nextResults.shift() ?? { data: null, error: null });
	return {
		builder,
		captured,
		queue(...results: Terminal[]) {
			nextResults = results;
		}
	};
}

const db = makeDb();
const access = { userId: 'user-1', membershipId: 'mem-1', tenantId: 't1', role: 'member', access: 0 };

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => db.builder,
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: vi.fn(async () => access),
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));

// Import the handlers AFTER the mocks are registered.
const { POST: subscribePOST } = await import('./subscribe/+server');
const { POST: prefsPOST } = await import('./prefs/+server');

function req(body: unknown): Request {
	return new Request('http://x/', { method: 'POST', body: JSON.stringify(body) });
}
// The routes only read params/request/locals off the event; the rest is unused.
function ev(body: unknown) {
	return { params: { origin: 'personal', org: 'jerry' }, request: req(body), locals: {} } as never;
}

beforeEach(() => {
	db.captured.table = undefined;
	db.captured.op = undefined;
	db.captured.payload = undefined;
	db.captured.filters = [];
});

describe('relay/push/subscribe', () => {
	const good = { endpoint: 'https://push.example/abc', keys: { p256dh: 'PK', auth: 'AK' }, platform: 'web' };

	it('inserts a new push_subscriptions row when none exists for (user, endpoint)', async () => {
		db.queue({ data: null, error: null }, { data: { id: 'sub-new' }, error: null });
		const res = await subscribePOST(ev(good));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ id: 'sub-new' });
		expect(db.captured.table).toBe('push_subscriptions');
		expect(db.captured.op).toBe('insert');
		const p = db.captured.payload as Record<string, unknown>;
		expect(p.user_id).toBe('user-1');
		expect(p.membership_id).toBe('mem-1');
		expect(p.platform).toBe('web');
		expect(p.enabled).toBe(true);
		expect(p.web_push).toEqual({ endpoint: good.endpoint, p256dh: 'PK', auth: 'AK' });
		expect(typeof p.last_seen).toBe('string');
	});

	it('updates in place when a row already exists (keyed by user_id + endpoint jsonb)', async () => {
		db.queue({ data: { id: 'sub-existing' }, error: null }, { data: { id: 'sub-existing' }, error: null });
		const res = await subscribePOST(ev(good));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ id: 'sub-existing' });
		expect(db.captured.op).toBe('update');
		// The existence check filtered by user_id and the endpoint jsonb path.
		expect(db.captured.filters).toContainEqual(['user_id', 'user-1']);
		expect(db.captured.filters).toContainEqual(['web_push->>endpoint', good.endpoint]);
		const p = db.captured.payload as Record<string, unknown>;
		expect(p.enabled).toBe(true);
		expect(p.web_push).toEqual({ endpoint: good.endpoint, p256dh: 'PK', auth: 'AK' });
	});

	it('400s a missing endpoint', async () => {
		const res = await subscribePOST(ev({ keys: { p256dh: 'PK', auth: 'AK' } }));
		expect(res.status).toBe(400);
		expect((await res.json()).error).toMatch(/endpoint/);
	});

	it('400s missing keys', async () => {
		const res = await subscribePOST(ev({ endpoint: 'https://push.example/abc' }));
		expect(res.status).toBe(400);
		expect((await res.json()).error).toMatch(/p256dh|auth/);
	});

	it('400s a non-https endpoint (SSRF guard)', async () => {
		const res = await subscribePOST(
			ev({ endpoint: 'http://169.254.169.254/latest/meta-data', keys: { p256dh: 'PK', auth: 'AK' } })
		);
		expect(res.status).toBe(400);
		expect((await res.json()).error).toMatch(/https/);
	});

	it('400s an unparseable endpoint URL', async () => {
		const res = await subscribePOST(ev({ endpoint: 'not a url', keys: { p256dh: 'PK', auth: 'AK' } }));
		expect(res.status).toBe(400);
		expect((await res.json()).error).toMatch(/valid URL/);
	});
});

describe('relay/push/prefs', () => {
	it('whitelists + merges the events opt-ins onto existing prefs', async () => {
		db.queue({ data: { events: { approvals: true } }, error: null }, { data: { user_id: 'user-1' }, error: null });
		const res = await prefsPOST(ev({ events: { stalls: true, crashed: true, bogus: true } }));
		expect(res.status).toBe(200);
		expect(db.captured.table).toBe('notification_prefs');
		expect(db.captured.op).toBe('upsert');
		const p = db.captured.payload as Record<string, unknown>;
		expect(p.user_id).toBe('user-1');
		// merged with existing approvals:true; bogus key dropped by the whitelist.
		expect(p.events).toEqual({ approvals: true, stalls: true, crashed: true });
	});

	it('400s when no recognised events are supplied', async () => {
		const res = await prefsPOST(ev({ events: { bogus: true } }));
		expect(res.status).toBe(400);
		expect((await res.json()).error).toMatch(/events/);
	});
});
