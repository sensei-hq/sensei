// A tiny in-memory stand-in for the supabase-js query builder, for tests whose
// subject is a SEQUENCE of reads and writes rather than one round-trip.
//
// Why this exists alongside the queue stubs the other specs use: those declare
// "the next terminal returns X", which is fine for a single query but cannot
// express the property that matters most for provisioning — that running it
// TWICE produces the same rows. A queue stub replays whatever it was handed, so
// an implementation that inserted a duplicate tenant on every pass would still
// pass. This one actually stores rows and enforces unique constraints, so
// idempotency is observable.
//
// Test-only. It supports exactly the subset of the builder the code under test
// uses, and throws on anything else rather than silently returning empty — a
// fake that quietly answers a query it does not understand is how a green test
// ends up proving nothing.

export interface FakeRow {
	[column: string]: unknown;
}

/** A unique constraint: the columns that together must not repeat. `where`
 *  narrows it to a partial index (as `tenant_connections` uses). */
export interface FakeUnique {
	columns: string[];
	where?: (row: FakeRow) => boolean;
	/** Compare these columns case-insensitively (the `lower(external_slug)` index). */
	lower?: string[];
}

export interface FakeTable {
	rows: FakeRow[];
	uniques?: FakeUnique[];
	/** How many times this table was addressed. Counted so a test can assert a
	 *  BOUNDED number of round trips — the ingest once issued ~4 subrequests per
	 *  row, which at 500 rows exceeded a Cloudflare Worker's per-invocation
	 *  subrequest cap. Without a counter that invariant is untestable. */
	reads?: number;
}

type Filter = { op: 'eq' | 'in' | 'is'; column: string; value: unknown };

const PG_UNIQUE_VIOLATION = '23505';

let idCounter = 0;
function nextId(prefix: string): string {
	idCounter += 1;
	return `${prefix}-${idCounter}`;
}

/** Reset the id sequence so ids are stable within a test. */
export function resetFakeIds(): void {
	idCounter = 0;
}

function matches(row: FakeRow, filters: Filter[]): boolean {
	return filters.every((f) => {
		const v = row[f.column];
		if (f.op === 'eq') return v === f.value;
		if (f.op === 'is') return f.value === null ? v === null || v === undefined : v === f.value;
		return Array.isArray(f.value) && (f.value as unknown[]).includes(v);
	});
}

function sameKey(a: FakeRow, b: FakeRow, u: FakeUnique): boolean {
	return u.columns.every((c) => {
		const av = a[c];
		const bv = b[c];
		if (u.lower?.includes(c)) {
			return String(av ?? '').toLowerCase() === String(bv ?? '').toLowerCase();
		}
		return av === bv;
	});
}

/**
 * Build a fake `DojoClient` over the given tables.
 *
 * `db.tables` stays readable after the call so a test can assert on the rows
 * that ended up there — which is usually a better assertion than the value the
 * function returned.
 */
export function fakeDojoDb(tables: Record<string, FakeTable>) {
	const state: Record<string, FakeTable> = {};
	for (const [name, t] of Object.entries(tables)) {
		state[name] = { rows: t.rows.map((r) => ({ ...r })), uniques: t.uniques, reads: 0 };
	}

	function builder(table: string) {
		if (!state[table]) {
			throw new Error(`fakeDojoDb: no table "${table}" was declared for this test`);
		}
		state[table].reads = (state[table].reads ?? 0) + 1;
		const filters: Filter[] = [];
		let pending: { op: 'insert' | 'update' | 'delete'; payload?: FakeRow } | null = null;

		const run = (): { data: FakeRow[] | null; error: { code?: string; message: string } | null } => {
			const t = state[table];
			if (pending?.op === 'insert') {
				const row: FakeRow = { id: nextId(table.slice(0, 4)), ...pending.payload };
				for (const u of t.uniques ?? []) {
					if (u.where && !u.where(row)) continue;
					const clash = t.rows.some((r) => (!u.where || u.where(r)) && sameKey(r, row, u));
					if (clash) {
						return {
							data: null,
							error: {
								code: PG_UNIQUE_VIOLATION,
								message: `duplicate key value violates unique constraint on (${u.columns.join(', ')})`
							}
						};
					}
				}
				t.rows.push(row);
				return { data: [row], error: null };
			}
			if (pending?.op === 'update') {
				const hit = t.rows.filter((r) => matches(r, filters));
				for (const r of hit) Object.assign(r, pending.payload);
				return { data: hit, error: null };
			}
			if (pending?.op === 'delete') {
				const hit = t.rows.filter((r) => matches(r, filters));
				state[table].rows = t.rows.filter((r) => !matches(r, filters));
				return { data: hit, error: null };
			}
			return { data: t.rows.filter((r) => matches(r, filters)), error: null };
		};

		const api: Record<string, unknown> = {
			select: () => api,
			order: () => api,
			eq: (column: string, value: unknown) => {
				filters.push({ op: 'eq', column, value });
				return api;
			},
			is: (column: string, value: unknown) => {
				filters.push({ op: 'is', column, value });
				return api;
			},
			in: (column: string, value: unknown) => {
				filters.push({ op: 'in', column, value });
				return api;
			},
			insert: (payload: FakeRow) => {
				pending = { op: 'insert', payload };
				return api;
			},
			update: (payload: FakeRow) => {
				pending = { op: 'update', payload };
				return api;
			},
			delete: () => {
				pending = { op: 'delete' };
				return api;
			},
			maybeSingle: async () => {
				const r = run();
				return { data: r.error ? null : (r.data?.[0] ?? null), error: r.error };
			},
			single: async () => {
				const r = run();
				if (r.error) return { data: null, error: r.error };
				if (!r.data?.length) {
					return { data: null, error: { code: 'PGRST116', message: 'no rows returned' } };
				}
				return { data: r.data[0], error: null };
			},
			then: (resolve: (v: unknown) => unknown) => {
				const r = run();
				return Promise.resolve(resolve({ data: r.error ? null : r.data, error: r.error }));
			}
		};
		return api;
	}

	return {
		from: (table: string) => builder(table),
		/** Re-scope the next `.from()` to another Postgres schema, as supabase-js
		 *  does. Modelled because NOT modelling it hid a real defect: the metric
		 *  catalogue lives in `sensei.metrics`, a bare `.from('metrics')` resolves
		 *  to `dojo.metrics`, and PostgREST answers 500 "Could not find the table
		 *  'dojo.metrics' in the schema cache". The test passed against a fake that
		 *  could not tell the two apart, and the whole batch was lost in
		 *  production.
		 *
		 *  A table is addressed as `<schema>.<table>` once re-scoped, so a fixture
		 *  must register `'sensei.metrics'` to be found — which is what makes the
		 *  missing prefix fail here instead of live. */
		schema: (name: string) => ({
			from: (table: string) => builder(`${name}.${table}`)
		}),
		/** The stored rows, for assertions. */
		tables: state
	};
}
