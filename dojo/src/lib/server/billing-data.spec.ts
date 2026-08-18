// Unit tests for the billing store logic (`billing-data.ts`). Exercises:
//   • summarizeSeatUsage — the pure per-seat count: PRIVATE-only, deduped per
//     user across projects, public-only users excluded, breakdown sorted.
//   • loadActiveSeatRows — active-seat query (eq tenant, is ended_at null) then
//     the cross-schema (sensei) namespace join, orphan-seat drop, empty short-cut.
//   • getBillingAccount — the maybeSingle fetch (row or null).
//   • refreshSeatsUsed — the billing_accounts upsert shape + onConflict.
//   • errors surface as BillingError(500).
// A chainable supabase-js stub (no live DB), like the sibling route specs.
import { describe, it, expect } from 'vitest';
import {
	summarizeSeatUsage,
	loadActiveSeatRows,
	getBillingAccount,
	refreshSeatsUsed,
	resolveProjectNamespaceId,
	openOrRefreshSeat,
	closeSeat,
	BillingError,
	type SeatRow,
	type DojoClient
} from './billing-data';

type Terminal = { data: unknown; error: unknown };
interface Call {
	schema?: string;
	table?: string;
	op?: string;
	payload?: unknown;
	onConflict?: string;
	select?: string;
	filters: [string, string, unknown][];
}
function makeDb() {
	const calls: Call[] = [];
	let results: Terminal[] = [];
	let pendingSchema: string | undefined;
	let cur: Call;
	const b: Record<string, unknown> = {};
	b.schema = (s: string) => {
		pendingSchema = s;
		return b;
	};
	b.from = (t: string) => {
		cur = { schema: pendingSchema, table: t, filters: [] };
		pendingSchema = undefined;
		calls.push(cur);
		return b;
	};
	b.select = (s?: string) => {
		if (cur) cur.select = s;
		return b;
	};
	b.eq = (c: string, v: unknown) => {
		cur.filters.push(['eq', c, v]);
		return b;
	};
	b.is = (c: string, v: unknown) => {
		cur.filters.push(['is', c, v]);
		return b;
	};
	b.in = (c: string, v: unknown) => {
		cur.filters.push(['in', c, v]);
		return b;
	};
	b.insert = (p: unknown) => {
		cur.op = 'insert';
		cur.payload = p;
		return b;
	};
	b.update = (p: unknown) => {
		cur.op = 'update';
		cur.payload = p;
		return b;
	};
	b.upsert = (p: unknown, opts?: { onConflict?: string }) => {
		cur.op = 'upsert';
		cur.payload = p;
		cur.onConflict = opts?.onConflict;
		return b;
	};
	b.single = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	b.maybeSingle = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	b.then = (resolve: (v: Terminal) => unknown) => resolve(results.shift() ?? { data: [], error: null });
	return {
		client: b as unknown as DojoClient,
		calls,
		queue(...r: Terminal[]) {
			results = r;
		}
	};
}

const seat = (user_id: string, visibility: 'private' | 'public', project: string, role = 'contributor'): SeatRow => ({
	user_id,
	role,
	namespace_id: `ns-${project}`,
	project_name: project,
	project_slug: project.toLowerCase(),
	visibility
});

describe('summarizeSeatUsage', () => {
	it('is empty for no seats', () => {
		expect(summarizeSeatUsage([])).toEqual({ seats_used: 0, total_active_seats: 0, billable_users: [] });
	});

	it('counts unique users on private projects, deduped across projects', () => {
		// A: private P1 · B: private P1 + public P2 · C: public P2 only
		const rows = [
			seat('userB', 'private', 'P1', 'maintainer'),
			seat('userB', 'public', 'P2'),
			seat('userA', 'private', 'P1'),
			seat('userC', 'public', 'P2')
		];
		const u = summarizeSeatUsage(rows);
		expect(u.seats_used).toBe(2); // A and B; C is public-only
		expect(u.total_active_seats).toBe(4);
		// sorted by user_id; C absent
		expect(u.billable_users.map((x) => x.user_id)).toEqual(['userA', 'userB']);
		// B's breakdown lists only the PRIVATE project, not the public one
		expect(u.billable_users[1].projects).toEqual([{ name: 'P1', slug: 'p1', role: 'maintainer' }]);
	});

	it('counts a user once even across multiple private projects', () => {
		const rows = [seat('u', 'private', 'P1'), seat('u', 'private', 'P2')];
		const u = summarizeSeatUsage(rows);
		expect(u.seats_used).toBe(1);
		expect(u.billable_users[0].projects).toHaveLength(2);
	});

	it('excludes everyone when all projects are public', () => {
		expect(summarizeSeatUsage([seat('u', 'public', 'P1')]).seats_used).toBe(0);
	});
});

describe('loadActiveSeatRows', () => {
	it('queries active seats then joins namespaces, dropping orphans', async () => {
		const db = makeDb();
		db.queue(
			{
				data: [
					{ user_id: 'u1', role: 'contributor', namespace_id: 'n1' },
					{ user_id: 'u2', role: 'lead', namespace_id: 'n2' },
					{ user_id: 'u3', role: 'contributor', namespace_id: 'gone' }
				],
				error: null
			},
			{
				data: [
					{ id: 'n1', name: 'Alpha', slug: 'alpha', visibility: 'private' },
					{ id: 'n2', name: 'Beta', slug: 'beta', visibility: 'public' }
				],
				error: null
			}
		);
		const rows = await loadActiveSeatRows(db.client, 'tenant-1');

		// seats query: dojo schema, filtered by tenant + active
		expect(db.calls[0]).toMatchObject({ schema: undefined, table: 'seats' });
		expect(db.calls[0].filters).toEqual([
			['eq', 'tenant_id', 'tenant-1'],
			['is', 'ended_at', null]
		]);
		// namespaces query: base (dojo) schema via the dojo.namespaces view, filtered by the deduped ids
		expect(db.calls[1]).toMatchObject({ schema: undefined, table: 'namespaces' });
		expect(db.calls[1].filters).toEqual([['in', 'id', ['n1', 'n2', 'gone']]]);
		// join result: orphan (namespace 'gone') dropped
		expect(rows).toEqual([
			{ user_id: 'u1', role: 'contributor', namespace_id: 'n1', project_name: 'Alpha', project_slug: 'alpha', visibility: 'private' },
			{ user_id: 'u2', role: 'lead', namespace_id: 'n2', project_name: 'Beta', project_slug: 'beta', visibility: 'public' }
		]);
	});

	it('short-circuits with no namespace query when there are no seats', async () => {
		const db = makeDb();
		db.queue({ data: [], error: null });
		const rows = await loadActiveSeatRows(db.client, 'tenant-1');
		expect(rows).toEqual([]);
		expect(db.calls).toHaveLength(1); // only the seats query ran
	});

	it('throws BillingError(500) when the seats query errors', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'boom' } });
		await expect(loadActiveSeatRows(db.client, 'tenant-1')).rejects.toBeInstanceOf(BillingError);
	});
});

describe('getBillingAccount', () => {
	it('returns the row when present', async () => {
		const db = makeDb();
		const acct = { plan: 'team', status: 'active', seats_included: 5, seats_used: 2, seats_computed_at: null, period_start: null, period_end: null };
		db.queue({ data: acct, error: null });
		expect(await getBillingAccount(db.client, 't')).toEqual(acct);
		expect(db.calls[0]).toMatchObject({ table: 'billing_accounts' });
	});

	it('returns null when the tenant has no account', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null });
		expect(await getBillingAccount(db.client, 't')).toBeNull();
	});
});

describe('refreshSeatsUsed', () => {
	it('upserts the cached seat count on tenant_id', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null });
		await refreshSeatsUsed(db.client, 'tenant-9', 3, '2026-07-25T00:00:00Z');
		expect(db.calls[0]).toMatchObject({
			table: 'billing_accounts',
			op: 'upsert',
			onConflict: 'tenant_id',
			payload: { tenant_id: 'tenant-9', seats_used: 3, seats_computed_at: '2026-07-25T00:00:00Z' }
		});
	});

	it('throws BillingError(500) on upsert failure', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'nope' } });
		await expect(refreshSeatsUsed(db.client, 't', 1, 'now')).rejects.toBeInstanceOf(BillingError);
	});
});

describe('resolveProjectNamespaceId', () => {
	it('looks up a project-scope namespace by slug via the dojo.namespaces view', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'ns-1' }, error: null });
		const id = await resolveProjectNamespaceId(db.client, 'my-proj');
		expect(id).toBe('ns-1');
		expect(db.calls[0]).toMatchObject({ schema: undefined, table: 'namespaces' });
		expect(db.calls[0].filters).toEqual([
			['eq', 'scope_key', 'project'],
			['eq', 'slug', 'my-proj']
		]);
	});

	it('returns null for an unknown slug', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null });
		expect(await resolveProjectNamespaceId(db.client, 'nope')).toBeNull();
	});
});

describe('openOrRefreshSeat', () => {
	it('bumps last_active_at when an active seat already exists', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'seat-1' }, error: null }, { data: null, error: null });
		const r = await openOrRefreshSeat(db.client, {
			tenantId: 't',
			userId: 'u',
			namespaceId: 'n',
			nowIso: '2026-07-25T00:00:00Z'
		});
		expect(r).toEqual({ opened: false, id: 'seat-1' });
		// read filtered to the ACTIVE seat, then update by id
		expect(db.calls[0].filters).toEqual([
			['eq', 'user_id', 'u'],
			['eq', 'namespace_id', 'n'],
			['is', 'ended_at', null]
		]);
		expect(db.calls[1]).toMatchObject({
			op: 'update',
			payload: { last_active_at: '2026-07-25T00:00:00Z' }
		});
	});

	it('opens a fresh seat when none is active', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null }, { data: { id: 'seat-2' }, error: null });
		const r = await openOrRefreshSeat(db.client, {
			tenantId: 't',
			userId: 'u',
			namespaceId: 'n',
			role: 'maintainer',
			nowIso: 'now'
		});
		expect(r).toEqual({ opened: true, id: 'seat-2' });
		expect(db.calls[1]).toMatchObject({
			op: 'insert',
			payload: { tenant_id: 't', user_id: 'u', namespace_id: 'n', role: 'maintainer', last_active_at: 'now' }
		});
	});

	it('defaults role to contributor', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null }, { data: { id: 'seat-3' }, error: null });
		await openOrRefreshSeat(db.client, { tenantId: 't', userId: 'u', namespaceId: 'n', nowIso: 'now' });
		expect((db.calls[1].payload as { role: string }).role).toBe('contributor');
	});
});

describe('closeSeat', () => {
	it('sets ended_at on the active seat and reports it closed', async () => {
		const db = makeDb();
		db.queue({ data: [{ id: 'seat-1' }], error: null });
		const r = await closeSeat(db.client, { userId: 'u', namespaceId: 'n', nowIso: 'end' });
		expect(r).toEqual({ closed: true });
		expect(db.calls[0]).toMatchObject({ op: 'update', payload: { ended_at: 'end' } });
		expect(db.calls[0].filters).toEqual([
			['eq', 'user_id', 'u'],
			['eq', 'namespace_id', 'n'],
			['is', 'ended_at', null]
		]);
	});

	it('reports not-closed when there was no active seat', async () => {
		const db = makeDb();
		db.queue({ data: [], error: null });
		expect(await closeSeat(db.client, { userId: 'u', namespaceId: 'n', nowIso: 'end' })).toEqual({
			closed: false
		});
	});
});
