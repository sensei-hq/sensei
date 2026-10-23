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
/** PostgREST's server-side row cap. Mirrors `max_rows` in supabase/config.toml;
 *  a read with no `.range()` never returns more than this, and says nothing
 *  about having truncated. */
export const PGRST_MAX_ROWS = 1000;

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
	/**
	 * Make every operation on this table FAIL with this error.
	 *
	 * Added because the error path is the one that matters and was the one the
	 * fake could not express: a handler that swallows a read failure and returns
	 * an empty set is indistinguishable from a genuinely empty table, and a test
	 * written against this fake would pass either way. Verified by mutation —
	 * replacing `throw` with `return empty` in syncPlan's activation read passed
	 * every test until this existed.
	 */
	error?: { code?: string; message: string };
	/** Make every WRITE (insert, update, delete) on this table fail with this
	 *  error while reads succeed — a store that can be read but not written. */
	writeError?: { code?: string; message: string };
}

type Filter = { op: 'eq' | 'in' | 'is' | 'gte'; column: string; value: unknown };

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
		// ISO timestamps and numbers compare in order as written.
		if (f.op === 'gte') return v != null && String(v) >= String(f.value);
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
	const rpcCalls: { fn: string; args: Record<string, unknown> }[] = [];
	for (const [name, t] of Object.entries(tables)) {
		// A scalar row (a function that returns one value, read with `.single()`)
		// stays the value; spreading it would turn a string into an object.
		state[name] = {
			rows: t.rows.map((r) => (r !== null && typeof r === 'object' ? { ...r } : r)),
			uniques: t.uniques,
			reads: 0,
			error: t.error,
			writeError: t.writeError
		};
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
			// Injected failure wins over everything: a table that cannot be read
			// cannot be written or counted either.
			if (t.error) return { data: null, error: t.error };
			if (pending && t.writeError) return { data: null, error: t.writeError };
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
			const hits = t.rows.filter((r) => matches(r, filters));
			if (sort) {
				const { column, ascending, nullsFirst } = sort;
				hits.sort((a, b) => {
					const x = a[column];
					const y = b[column];
					if (x == null || y == null) return x == null && y == null ? 0 : (x == null) === nullsFirst ? -1 : 1;
					return (String(x) < String(y) ? -1 : String(x) > String(y) ? 1 : 0) * (ascending ? 1 : -1);
				});
			}
			// Slice AFTER filtering, matching PostgREST: the range applies to the
			// result set, not to the table.
			//
			// AN UNRANGED READ IS CAPPED, exactly as the server caps it
			// (`PGRST_DB_MAX_ROWS=1000`, verified on the running instance). Without
			// this the fake happily returns 5000 rows, every pagination test passes
			// against code that does not paginate, and the cap stays a
			// code-reading exercise instead of a tested one.
			return {
				data: range ? hits.slice(range.from, range.to + 1) : hits.slice(0, PGRST_MAX_ROWS),
				error: null
			};
		};

		/** `.range(from, to)` — INCLUSIVE both ends, as PostgREST defines it. Added
		 *  so paginated reads are testable: without it a page loop is untestable
		 *  and the 1000-row cap it exists to defeat stays a code-reading exercise. */
		let range: { from: number; to: number } | null = null;
		/** `.order(column, { ascending, nullsFirst })` — applied before the range, as PostgREST does. */
		let sort: { column: string; ascending: boolean; nullsFirst: boolean } | null = null;

		/** `.select(cols, { count: 'exact', head: true })` — answer the row count. */
		let counted = false;

		const api: Record<string, unknown> = {
			select: (_cols?: string, opts?: { count?: string; head?: boolean }) => {
				counted = opts?.count === 'exact';
				return api;
			},
			order: (column: string, opts?: { ascending?: boolean; nullsFirst?: boolean }) => {
				const ascending = opts?.ascending ?? true;
				sort = { column, ascending, nullsFirst: opts?.nullsFirst ?? !ascending };
				return api;
			},
			range: (from: number, to: number) => {
				range = { from, to };
				return api;
			},
			eq: (column: string, value: unknown) => {
				filters.push({ op: 'eq', column, value });
				return api;
			},
			is: (column: string, value: unknown) => {
				filters.push({ op: 'is', column, value });
				return api;
			},
			gte: (column: string, value: unknown) => {
				filters.push({ op: 'gte', column, value });
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
				if (counted) return Promise.resolve(resolve({ data: null, count: r.error ? null : (r.data?.length ?? 0), error: r.error }));
				return Promise.resolve(resolve({ data: r.error ? null : r.data, error: r.error }));
			}
		};
		return api;
	}

	return {
		from: (table: string) => builder(table),
		/** A set-returning function, read like a table: `rpc('my_repositories')`
		 *  addresses the fixture table `'rpc:my_repositories'`. What the function
		 *  itself returns (e.g. that it pins the caller's principal) is the
		 *  database's to prove, in database/tests; here the fixture IS its result. */
		rpc: (fn: string, args?: Record<string, unknown>) => {
			rpcCalls.push({ fn, args: args ?? {} });
			return builder(`rpc:${fn}`);
		},
		/** Every rpc made, with its arguments, for assertions on what was asked. */
		rpcCalls,
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

/** The dōjō tables provisioning touches, with the constraints that actually
 *  exist in the DDL — those are what make the idempotence assertions real. */
export function provisioningTables(seed: Partial<Record<string, FakeTable>> = {}): Record<string, FakeTable> {
	return {
		identities: {
			rows: [],
			uniques: [{ columns: ['provider', 'subject'] }],
			...seed.identities
		},
		tenants: {
			rows: [],
			uniques: [{ columns: ['key'] }],
			...seed.tenants
		},
		tenant_connections: {
			rows: [],
			uniques: [
				{ columns: ['provider', 'external_id'], where: (r) => r.external_id != null },
				{
					columns: ['provider', 'external_slug'],
					lower: ['external_slug'],
					where: (r) => r.external_id == null
				}
			],
			...seed.tenant_connections
		},
		memberships: {
			rows: [],
			uniques: [{ columns: ['tenant_id', 'user_id'] }],
			...seed.memberships
		},
		repositories: {
			rows: [],
			// The constraint the whole scoping argument rests on: the SAME repository
			// legitimately exists under two tenants, so `repo_key` alone does not
			// identify a row (§8a "the sign-in refresh must be scoped").
			uniques: [{ columns: ['tenant_id', 'repo_key'] }],
			...seed.repositories
		},
		repository_graph_branches: {
			rows: [],
			uniques: [{ columns: ['repository_id', 'branch'] }],
			...seed.repository_graph_branches
		},
		// The per-person forge mirror (#307 A2c); its own rules are proven in
		// database/tests/dojo/repository_roles_mirror_principal.sql.
		'rpc:mirror_principal_repository_roles': {
			rows: [],
			...seed['rpc:mirror_principal_repository_roles']
		},
		// Leaving an org ends the membership provisioning made (#309); its own rules
		// are proven in database/tests/dojo/membership_departure.sql.
		'rpc:reconcile_org_memberships': {
			rows: [],
			...seed['rpc:reconcile_org_memberships']
		},
		// A forge admin signing in claims the org's dōjō (#308 R5a); first-claim-wins
		// and the audit row are proven in database/tests/dojo/forge_unit_claim.sql.
		'rpc:claim_forge_unit': {
			rows: [],
			...seed['rpc:claim_forge_unit']
		},
		// Where an org's repositories file (#308 R5a): read before an org dōjō is
		// created, so an org an admin filed elsewhere as a workspace gets none.
		forge_unit_mappings: {
			rows: [],
			uniques: [{ columns: ['provider', 'external_id'] }],
			...seed.forge_unit_mappings
		}
	};
}
