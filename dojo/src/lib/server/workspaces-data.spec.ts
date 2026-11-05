// Unit tests for the workspaces data layer (`workspaces-data.ts`, #299): the tenant
// list, the caller's RLS-scoped list, the drill-in by id, and a run attaching to
// the shared workspace. Fail-closed on error.
import { describe, it, expect } from 'vitest';
import {
	listOrgWorkspaces,
	listUserWorkspaces,
	getWorkspaceConstitution,
	getWorkspaceMetrics,
	attachRunToWorkspace,
	fileRunUnderWorkspace,
	AdminError,
	type DojoClient
} from './workspaces-data';

function makeListDb(result: { data: unknown; error: unknown }) {
	const captured: { tenantEq?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (_c: string, v: unknown) => {
		captured.tenantEq = v;
		return b;
	};
	b.order = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

// Table-keyed mock for the activity sort: each `.from(table)` chain resolves
// that table's canned result at its `.order()` / `.in()` terminal, recording
// every call so tests can pin the read's narrowing (F3).
function makeActivityDb(tables: Record<string, { data?: unknown; error: unknown }>) {
	let table = '';
	const calls: Array<{ table: string; op: string; args: unknown[] }> = [];
	const b: Record<string, unknown> = {};
	b.from = (t: string) => ((table = t), b);
	b.select = () => b;
	b.eq = (...args: unknown[]) => {
		calls.push({ table, op: 'eq', args });
		return b;
	};
	b.order = (...args: unknown[]) => {
		calls.push({ table, op: 'order', args });
		return Promise.resolve(tables[table] ?? { data: [], error: null });
	};
	b.in = (...args: unknown[]) => {
		calls.push({ table, op: 'in', args });
		return Promise.resolve(tables[table] ?? { data: [], error: null });
	};
	return { db: b as unknown as DojoClient, calls };
}

describe('listOrgWorkspaces', () => {
	it('returns the tenant rows', async () => {
		const rows = [{ id: 'p1' }, { id: 'p2' }];
		const { db, captured } = makeListDb({ data: rows, error: null });
		expect(await listOrgWorkspaces(db, 't1')).toEqual(rows);
		expect(captured.tenantEq).toBe('t1');
	});
	it('keeps API order when no workspace has any activity', async () => {
		const { db } = makeActivityDb({
			workspaces: { data: [{ id: 'w1' }, { id: 'w2' }], error: null },
			workspace_activity: { data: [], error: null }
		});
		expect((await listOrgWorkspaces(db, 't1')).map((w) => w.id)).toEqual(['w1', 'w2']);
	});
	it.skip('pending: L4 workspace-sort — most recently active first, quiet workspaces last, no invented date', async () => {
		const { db, calls } = makeActivityDb({
			workspaces: {
				data: [
					{ id: 'w1', last_run_at: null },
					{ id: 'w2', last_run_at: null },
					{ id: 'w3', last_run_at: null }
				],
				error: null
			},
			// The max lives in the DB (F3): one read, per-workspace activity.
			workspace_activity: {
				data: [
					{ workspace_id: 'w1', last_activity_at: '2026-11-02T10:00:00Z' },
					{ workspace_id: 'w2', last_activity_at: '2026-11-03T10:00:00Z' }
				],
				error: null
			}
		});
		const out = await listOrgWorkspaces(db, 't1');
		expect(out.map((w) => w.id)).toEqual(['w2', 'w1', 'w3']);
		expect(out[2]).toEqual({ id: 'w3', last_run_at: null });
		// F3: the view read is narrowed to the listed ids — a whole-view read
		// would hit max_rows and truncate with no error.
		const activityRead = calls.find((c) => c.table === 'workspace_activity' && c.op === 'in');
		expect(activityRead?.args, 'view read narrowed to listed ids').toEqual([
			'workspace_id',
			['w1', 'w2', 'w3']
		]);
	});
	it.skip('pending: L4 workspace-sort — an activity-read error fails closed, never API order', async () => {
		const { db } = makeActivityDb({
			workspaces: { data: [{ id: 'w1' }], error: null },
			workspace_activity: { data: null, error: { message: 'act boom' } }
		});
		await expect(listOrgWorkspaces(db, 't1')).rejects.toThrow(AdminError);
	});
	it('honest-empty [] when data is null', async () => {
		const { db } = makeListDb({ data: null, error: null });
		expect(await listOrgWorkspaces(db, 't1')).toEqual([]);
	});
	it('fails closed (500) on a query error — never a fixture', async () => {
		const { db } = makeListDb({ data: null, error: { message: 'boom' } });
		await expect(listOrgWorkspaces(db, 't1')).rejects.toBeInstanceOf(AdminError);
	});
});

// #299: a workspace is ONE SHARED item in a dōjō. The caller's list and the
// drill-in read go through RLS (dojo.can_read_workspace — whoever may read one of
// its repositories), never a `user_id` filter: there is no owner column.
function makeUserDb(result: { data: unknown; error: unknown }) {
	const captured: { eqs: [string, unknown][] } = { eqs: [] };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (c: string, v: unknown) => {
		captured.eqs.push([c, v]);
		return b;
	};
	b.order = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('listUserWorkspaces — every workspace the caller may read (RLS), across dōjōs', () => {
	it.skip('pending: L4 workspace-sort — the personal list sorts by activity too, quiet last', async () => {
		const { db, calls } = makeActivityDb({
			workspaces: {
				data: [
					{ id: 'w1', last_run_at: null },
					{ id: 'w2', last_run_at: null }
				],
				error: null
			},
			workspace_activity: {
				data: [{ workspace_id: 'w2', last_activity_at: '2026-11-03T10:00:00Z' }],
				error: null
			}
		});
		const out = await listUserWorkspaces(db);
		expect(out.map((w) => w.id)).toEqual(['w2', 'w1']);
		expect(out[1]).toEqual({ id: 'w1', last_run_at: null });
		const activityRead = calls.find((c) => c.table === 'workspace_activity' && c.op === 'in');
		expect(activityRead?.args, 'view read narrowed to listed ids').toEqual(['workspace_id', ['w1', 'w2']]);
	});
	it.skip('pending: L4 workspace-sort — the personal list fails closed when the activity read errors', async () => {
		const { db } = makeActivityDb({
			workspaces: { data: [{ id: 'w1' }], error: null },
			workspace_activity: { data: null, error: { message: 'act boom' } }
		});
		await expect(listUserWorkspaces(db)).rejects.toThrow(AdminError);
	});
	it('returns the readable rows with their dōjō, with no owner filter', async () => {
		const rows = [{ id: 'p1', tenant_id: 't1' }, { id: 'p2', tenant_id: 't2' }];
		const { db, captured } = makeUserDb({ data: rows, error: null });
		expect(await listUserWorkspaces(db)).toEqual(rows);
		expect(captured.eqs).toEqual([]);
	});
	it('honest-empty [] when data is null (genuine empty, not a mask)', async () => {
		const { db } = makeUserDb({ data: null, error: null });
		expect(await listUserWorkspaces(db)).toEqual([]);
	});
	it('fails closed (AdminError 500) on a query error — never a fabricated list', async () => {
		const { db } = makeUserDb({ data: null, error: { message: 'boom' } });
		await expect(listUserWorkspaces(db)).rejects.toBeInstanceOf(AdminError);
	});
});

function makeSingleDb(result: { data: unknown; error: unknown }) {
	const captured: { eqs: [string, unknown][] } = { eqs: [] };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (c: string, v: unknown) => {
		captured.eqs.push([c, v]);
		return b;
	};
	b.maybeSingle = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('getWorkspaceConstitution — the per-workspace drill-in read, by id', () => {
	// By id: a slug is unique only within one dōjō, so two of the caller's dōjōs
	// can hold the same slug.
	it('returns the stored constitution jsonb for that workspace id', async () => {
		const constitution = { rules: [{ scope_key: 'organization', title: 'x', enforcement: 'mandatory' }], conflicts: [], locks: 1 };
		const { db, captured } = makeSingleDb({ data: { constitution }, error: null });
		expect(await getWorkspaceConstitution(db, 'p1')).toEqual(constitution);
		expect(captured.eqs).toEqual([['id', 'p1']]);
	});
	it('returns null when the workspace has no federated constitution yet (honest-empty)', async () => {
		const { db } = makeSingleDb({ data: { constitution: null }, error: null });
		expect(await getWorkspaceConstitution(db, 'p1')).toBeNull();
	});
	it('returns null when the caller cannot read the workspace (RLS hides it)', async () => {
		const { db } = makeSingleDb({ data: null, error: null });
		expect(await getWorkspaceConstitution(db, 'p1')).toBeNull();
	});
	it('fails closed (AdminError 500) on a query error — never a fabricated constitution', async () => {
		const { db } = makeSingleDb({ data: null, error: { message: 'boom' } });
		await expect(getWorkspaceConstitution(db, 'p1')).rejects.toBeInstanceOf(AdminError);
	});
});

function makeRpcDb(result: { data: unknown; error: unknown }) {
	const captured: { fn?: string; args?: Record<string, unknown> } = {};
	const db = {
		rpc: (fn: string, args: Record<string, unknown>) => {
			captured.fn = fn;
			captured.args = args;
			return Promise.resolve(result);
		}
	};
	return { db: db as unknown as DojoClient, captured };
}

describe('attachRunToWorkspace — a run attaches to the shared workspace, never creates one', () => {
	it('asks the database for the (tenant, slug) workspace and passes the constitution', async () => {
		const { db, captured } = makeRpcDb({ data: 'proj-1', error: null });
		expect(await attachRunToWorkspace(db, 't1', 'sensei', { rules: [] })).toBe('proj-1');
		expect(captured).toEqual({
			fn: 'attach_run_to_workspace',
			args: { p_tenant_id: 't1', p_slug: 'sensei', p_constitution: { rules: [] } }
		});
	});
	it('passes no constitution when the daemon resolved none', async () => {
		const { db, captured } = makeRpcDb({ data: 'proj-1', error: null });
		await attachRunToWorkspace(db, 't1', 'sensei', undefined);
		expect(captured.args?.p_constitution).toBeNull();
	});
	it('returns null when the dōjō has no such workspace yet (the sync has not filed it)', async () => {
		const { db } = makeRpcDb({ data: null, error: null });
		expect(await attachRunToWorkspace(db, 't1', 'sensei', null)).toBeNull();
	});
	it('fails closed (AdminError 500) on an error', async () => {
		const { db } = makeRpcDb({ data: null, error: { message: 'not a member of this dōjō' } });
		await expect(attachRunToWorkspace(db, 't1', 'sensei', null)).rejects.toBeInstanceOf(AdminError);
	});
});


// #252: a relay session carries its owner's workspace, which is what lets a team
// owning that workspace see THAT the run is going (status only).
describe('fileRunUnderWorkspace', () => {
	it('sets the session\'s workspace_id, by the session id', async () => {
		const calls: Array<[string, unknown]> = [];
		const b: Record<string, unknown> = {};
		b.from = (t: string) => (calls.push(['from', t]), b);
		b.update = (v: unknown) => (calls.push(['update', v]), b);
		b.eq = (c: string, v: unknown) => (calls.push(['eq', [c, v]]), Promise.resolve({ error: null }));
		await fileRunUnderWorkspace(b as unknown as DojoClient, 'sess-1', 'proj-1');
		expect(calls).toEqual([
			['from', 'relay_sessions'],
			['update', { workspace_id: 'proj-1' }],
			['eq', ['id', 'sess-1']]
		]);
	});
	it('throws AdminError on a DB error', async () => {
		const b: Record<string, unknown> = {};
		b.from = () => b;
		b.update = () => b;
		b.eq = () => Promise.resolve({ error: { message: 'denied' } });
		await expect(fileRunUnderWorkspace(b as unknown as DojoClient, 's', 'p')).rejects.toBeInstanceOf(AdminError);
	});
});

// WP4: a workspace's metric series — the read behind its charts. The gate (403),
// the tenant scope (404) and the rollup (500 on error, never []) are three
// separate answers, so they are decided in that order.
describe('getWorkspaceMetrics', () => {
	const TODAY = new Date('2026-11-03T12:00:00Z');

	function makeMetricsDb(opts: {
		canRead: boolean;
		inTenant: boolean;
		series: unknown[] | { error: { message: string } };
	}) {
		const calls: string[] = [];
		const orders: Array<{ column: string; nullsFirst?: boolean }> = [];
		const rpc = (fn: string, _args: Record<string, unknown>) => {
			calls.push(`rpc:${fn}`);
			if (fn === 'can_read_workspace') {
				return Promise.resolve({ data: opts.canRead, error: null });
			}
			const chain: Record<string, unknown> = {};
			chain.select = () => chain;
			chain.order = (column: string, o?: { nullsFirst?: boolean }) => {
				orders.push({ column, nullsFirst: o?.nullsFirst });
				return chain;
			};
			chain.range = () => {
				const s = opts.series;
				return Promise.resolve(
					Array.isArray(s) ? { data: s, error: null } : { data: null, error: s.error }
				);
			};
			return chain;
		};
		const from = (table: string) => {
			calls.push(`from:${table}`);
			const chain: Record<string, unknown> = {};
			chain.select = () => chain;
			chain.eq = () => chain;
			chain.maybeSingle = () =>
				Promise.resolve(
					opts.inTenant ? { data: { id: 'w1' }, error: null } : { data: null, error: null }
				);
			return chain;
		};
		return { db: { rpc, from } as unknown as DojoClient, calls, orders };
	}

	const ROWS = [
		{
			repository_id: 'r1',
			repo_key: 'github.com/acme/api',
			metric_key: 'ftr',
			scope: 'repo',
			principal_id: null,
			computed_on: '2026-11-03',
			grain: 'daily',
			value: '0.5',
			props: {}
		},
		{
			repository_id: 'r1',
			repo_key: 'github.com/acme/api',
			metric_key: 'ftr',
			scope: 'pooled',
			principal_id: null,
			computed_on: '2026-11-03',
			grain: 'daily',
			value: 7,
			props: { pooled_rows: 3, numerator: '15', denominator: 6 }
		}
	];

	it('returns the shaped series for a workspace the caller may read', async () => {
		const { db } = makeMetricsDb({ canRead: true, inTenant: true, series: ROWS });
		await expect(getWorkspaceMetrics(db, 't-acme', 'w1', { days: 30, today: TODAY })).resolves.toEqual({
			since: '2026-10-05',
			days: 30,
			rows: [
				{
					repository_id: 'r1',
					repo_key: 'github.com/acme/api',
					metric_key: 'ftr',
					scope: 'repo',
					principal_id: null,
					day: '2026-11-03',
					value: 0.5,
					props: {}
				},
				{
					repository_id: 'r1',
					repo_key: 'github.com/acme/api',
					metric_key: 'ftr',
					scope: 'pooled',
					principal_id: null,
					day: '2026-11-03',
					value: 7,
					props: { pooled_rows: 3, numerator: '15', denominator: 6 }
				}
			]
		});
	});

	it('refuses a caller who may not read the workspace, before touching its rows', async () => {
		// The gate runs first: an outsider learns nothing, not even whether the
		// workspace is in this dōjō.
		const { db, calls } = makeMetricsDb({ canRead: false, inTenant: true, series: ROWS });
		const err = await getWorkspaceMetrics(db, 't-acme', 'w1', { days: 30, today: TODAY }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(403);
		expect(calls).toEqual(['rpc:can_read_workspace']);
	});

	it('answers 404 when the workspace lives in another dōjō', async () => {
		const { db, calls } = makeMetricsDb({ canRead: true, inTenant: false, series: ROWS });
		const err = await getWorkspaceMetrics(db, 't-acme', 'w1', { days: 30, today: TODAY }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(404);
		expect(calls).not.toContain('rpc:workspace_metric_series');
	});

	it('fails closed on a series error — never []', async () => {
		const { db } = makeMetricsDb({
			canRead: true,
			inTenant: true,
			series: { error: { message: 'connection reset' } }
		});
		const err = await getWorkspaceMetrics(db, 't-acme', 'w1', { days: 30, today: TODAY }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(500);
		expect(err.message).toMatch(/connection reset/);
	});

	it('pages the series in a deterministic total order — an unordered OFFSET can drop or repeat rows', async () => {
		// Each page re-runs the rollup with a fresh OFFSET; without an ORDER BY
		// Postgres keeps no row order between runs, so past 1,000 rows a row can
		// land on two pages or on none, with no error. The order is unique per
		// row: within one branch the table's natural key is unique, and a pooled
		// row carries a null principal where a detail row carries a real one.
		const { db, orders } = makeMetricsDb({ canRead: true, inTenant: true, series: ROWS });
		await getWorkspaceMetrics(db, 't-acme', 'w1', { days: 30, today: TODAY });
		expect(orders).toEqual([
			{ column: 'repository_id', nullsFirst: undefined },
			{ column: 'metric_key', nullsFirst: undefined },
			{ column: 'scope', nullsFirst: undefined },
			{ column: 'computed_on', nullsFirst: undefined },
			{ column: 'principal_id', nullsFirst: true }
		]);
	});
});
