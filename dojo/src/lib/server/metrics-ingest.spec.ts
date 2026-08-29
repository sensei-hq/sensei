// The receiving half of the metric push (daemon-sync.md §7). Runs against
// `fakeDojoDb` so the unique key is real and a re-push is observably an update
// rather than a second row.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { ingestMetrics } from './metrics-ingest';

const ALICE = 'p-alice';

/** Alice is in `t-acme` and has one registered repository there. `t-other` is
 *  someone else's tenant with its own repository. */
function tables(): Record<string, FakeTable> {
	return {
		memberships: {
			rows: [
				{ id: 'm1', tenant_id: 't-acme', user_id: ALICE },
				{ id: 'm2', tenant_id: 't-other', user_id: 'p-bob' }
			]
		},
		repositories: {
			rows: [
				{ id: 'r-api', tenant_id: 't-acme', repo_key: 'github.com/acme/api', name: 'api' },
				{ id: 'r-secret', tenant_id: 't-other', repo_key: 'github.com/other/secret', name: 'secret' }
			]
		},
		// `dojo.metric_catalogue`, the sanctioned view over `sensei.metrics` —
		// named exactly as the code addresses it. A fixture called `metrics` would
		// let a bare `.from('metrics')` pass here and 500 in production, which is
		// what happened twice.
		metric_catalogue: {
			rows: [
				{ id: 'met-commits', key: 'commits_per_day' },
				{ id: 'met-churn', key: 'churn' }
			]
		},
		// The gate the ingest reads. Modelled in the base fixture because every test
		// that expects a row to be STORED needs the plan to permit it — the view is
		// now part of the write path, not just the read.
		all_my_repositories: {
			rows: [
				{
					repository_id: 'r-api',
					repo_key: 'github.com/acme/api',
					principal_id: ALICE,
					sync_enabled: true,
					refused_by: null,
					reason_code: null
				}
			]
		},
		repository_metrics: {
			rows: [],
			uniques: [
				{
					columns: [
						'metric_id',
						'repository_id',
						'scope',
						'principal_id',
						'commit_sha',
						'computed_on',
						'grain'
					]
				}
			]
		}
	};
}

function row(over: Record<string, unknown> = {}) {
	return {
		repo_key: 'github.com/acme/api',
		metric: 'commits_per_day',
		scope: 'repo',
		computed_on: '2026-08-27',
		grain: 'daily',
		value: 12,
		...over
	};
}

beforeEach(() => resetFakeIds());

describe('ingestMetrics', () => {
	it('stores a repo-scoped metric against the tenant that owns the repository', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [row()]);
		expect(out.rejected).toEqual([]);
		expect(out.accepted).toBe(1);
		expect(db.tables.repository_metrics.rows[0]).toMatchObject({
			tenant_id: 't-acme',
			repository_id: 'r-api',
			// Resolved from the KEY the daemon sent. Metric uuids are not
			// guaranteed identical across installs; the key is the stable identity.
			metric_id: 'met-commits',
			scope: 'repo',
			computed_on: '2026-08-27',
			grain: 'daily',
			value: 12
		});
	});

	it('refuses a repository the caller is not a member of, even though the plan is the gate', async () => {
		// The plan is an allow-list the daemon acts on, but entitlement is
		// RE-DECIDED here. A daemon that ignores its plan must gain nothing.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [
			row({ repo_key: 'github.com/other/secret' })
		]);
		expect(out.accepted).toBe(0);
		expect(out.rejected).toHaveLength(1);
		expect(db.tables.repository_metrics.rows).toHaveLength(0);
	});

	it('cannot be used to discover which repo_keys other tenants have registered', async () => {
		// An EXISTENCE ORACLE. Signup is open, so any stranger could post up to
		// MAX_ROWS probe rows and learn which private repo_keys are enrolled by
		// other organisations — 1000 probes per request, no membership needed.
		//
		// The reason must therefore be IDENTICAL for "registered in a tenant you are
		// not in" and "registered nowhere". The old code's comment claimed the
		// opposite ("conflating the two would leak…") — it is the DISTINCTION that
		// leaks, and it was pinned by the test above.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [
			// registered, but only in t-other, which Alice is not in
			row({ repo_key: 'github.com/other/secret' }),
			// registered nowhere at all
			row({ repo_key: 'github.com/other/does-not-exist' })
		]);
		expect(out.accepted).toBe(0);
		expect(out.rejected).toHaveLength(2);
		expect(out.rejected[0].reason).toBe(out.rejected[1].reason);
	});

	it('refuses a repo_key it has never registered rather than dropping it', async () => {
		// If this reject were removed the row would be SILENTLY dropped, and then —
		// because `rejected` would be empty — the daemon would mark it `shared_at`
		// and never re-push it. The daemon's watermark logic depends on `rejected`
		// being complete.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [
			row({ repo_key: 'github.com/acme/ghost' })
		]);
		expect(out.accepted).toBe(0);
		expect(out.rejected).toEqual([
			{ repo_key: 'github.com/acme/ghost', metric: 'commits_per_day', reason: 'unknown_repository' }
		]);
		expect(db.tables.repository_metrics.rows).toHaveLength(0);
	});

	it('refuses an unknown metric key rather than inventing a metric id', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [row({ metric: 'not_a_real_metric' })]);
		expect(out.rejected[0].reason).toBe('unknown_metric');
		expect(db.tables.repository_metrics.rows).toHaveLength(0);
	});

	it('refuses a user-scoped row loudly instead of dropping it', async () => {
		// `dojo.repository_metrics.principal_id` is a PRINCIPAL, never a git
		// email — commit trailers are unverified and would let anyone attribute
		// work to a colleague. `personas.principal_id` is the bridge and is NULL
		// until the persona is linked, so there is nothing honest to attribute a
		// user-scoped row to yet. Rejecting names the gap; silently skipping
		// would look like a successful push that moved 4,050 fewer rows.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [row({ scope: 'user' })]);
		expect(out.accepted).toBe(0);
		expect(out.rejected[0].reason).toBe('unsupported_scope');
		expect(db.tables.repository_metrics.rows).toHaveLength(0);
	});

	it('is idempotent — re-pushing the same day updates the value, it does not duplicate', async () => {
		// The daemon re-pushes whenever a row is recomputed, so this is the
		// normal path, not an edge case.
		const db = fakeDojoDb(tables());
		await ingestMetrics(db as never, ALICE, [row({ value: 12 })]);
		const out = await ingestMetrics(db as never, ALICE, [row({ value: 19 })]);
		expect(out.accepted).toBe(1);
		expect(db.tables.repository_metrics.rows).toHaveLength(1);
		expect(db.tables.repository_metrics.rows[0].value).toBe(19);
	});

	it('keeps rows that differ only by commit_sha as separate rows', async () => {
		// The destination unique key is 7 columns including commit_sha; the lookup
		// used 4. Two same-day rows for different commits therefore overwrote each
		// other while BOTH were counted accepted — and the daemon then marked both
		// shared_at, so the lost value was never re-sent. `quality.rs` writes
		// exactly this shape: one repo-scoped row per commit, re-scanned each pass.
		// 6 groups / 34 rows in the live database would be destroyed.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [
			row({ commit_sha: 'aaa', value: 0.1 }),
			row({ commit_sha: 'bbb', value: 0.2 })
		]);
		expect(out.rejected).toEqual([]);
		expect(out.accepted).toBe(2);
		expect(db.tables.repository_metrics.rows).toHaveLength(2);
		expect(db.tables.repository_metrics.rows.map((r) => r.value).sort()).toEqual([0.1, 0.2]);
	});

	it('a repo_key registered under two tenants is rejected per row, not thrown', async () => {
		// `dojo.repositories` is unique (tenant_id, repo_key) and its DDL says so
		// deliberately: "ONE ROW PER (repo_key, tenant), not one globally. A
		// consultant legitimately has the same repository under two clients."
		// An unscoped `.maybeSingle()` then returns PGRST116 for two rows, which
		// threw a 500 and killed the WHOLE batch — permanently, since nothing gets
		// marked shared and the identical batch retries every cadence.
		const t = tables();
		t.memberships.rows.push({ id: 'm3', tenant_id: 't-consult', user_id: ALICE });
		t.repositories.rows.push({
			id: 'r-api-consult',
			tenant_id: 't-consult',
			repo_key: 'github.com/acme/api',
			name: 'api'
		});
		const db = fakeDojoDb(t);
		const out = await ingestMetrics(db as never, ALICE, [
			row(),
			row({ metric: 'churn', repo_key: 'github.com/acme/api', value: 3 })
		]);
		// Ambiguous is a per-row refusal naming itself; the batch survives.
		expect(out.rejected.every((r) => r.reason === 'ambiguous')).toBe(true);
		expect(out.rejected).toHaveLength(2);
	});

	it('resolves the batch in a bounded number of reads, not one set per row', async () => {
		// The loop issued ~4 PostgREST subrequests PER ROW inside one Cloudflare
		// Worker invocation: at the daemon's 500 rows that is ~2001 sequential round
		// trips, past Cloudflare's per-invocation subrequest cap and past the
		// daemon's 30s timeout. A failure there is not clean — the Worker has
		// already committed a prefix, the daemon marks nothing shared, and the
		// identical window retries every cadence.
		//
		// The resolve reads (memberships, repositories, metric_catalogue) must stay
		// CONSTANT as the batch grows; only the per-row existing-check and write may
		// scale.
		const count = (db: ReturnType<typeof fakeDojoDb>) =>
			(t: string) => db.tables[t]?.reads ?? 0;

		const small = fakeDojoDb(tables());
		await ingestMetrics(small as never, ALICE, [row({ commit_sha: 'a' })]);
		const big = fakeDojoDb(tables());
		await ingestMetrics(
			big as never,
			ALICE,
			Array.from({ length: 20 }, (_, i) => row({ commit_sha: `s${i}` }))
		);

		for (const t of ['memberships', 'repositories', 'metric_catalogue']) {
			expect(count(big)(t), `${t} must be read a bounded number of times`).toBe(
				count(small)(t)
			);
		}
	});

	it('refuses a repository the plan does not permit — the write RE-DECIDES', async () => {
		// The daemon's local push gate was removed (B2) on the stated premise that
		// "the dōjō re-decides entitlement on every write". That premise was FALSE:
		// this function checked membership and nothing else, so dropping the daemon
		// gate removed the only gate. A member of the tenant could push metrics for a
		// repository that is unelected, unsubscribed, or whose forge visibility was
		// never captured.
		const t = tables();
		t.all_my_repositories = {
			rows: [
				{
					repository_id: 'r-api',
					repo_key: 'github.com/acme/api',
					principal_id: ALICE,
					sync_enabled: false,
					refused_by: 'election',
					reason_code: 'not_elected_user'
				}
			]
		};
		const db = fakeDojoDb(t);
		const out = await ingestMetrics(db as never, ALICE, [row()]);
		expect(out.accepted).toBe(0);
		expect(out.rejected[0].reason).toBe('not_permitted');
		expect(db.tables.repository_metrics.rows).toHaveLength(0);
	});

	it('accepts a repository the plan permits', async () => {
		const t = tables();
		t.all_my_repositories = {
			rows: [
				{
					repository_id: 'r-api',
					repo_key: 'github.com/acme/api',
					principal_id: ALICE,
					sync_enabled: true,
					refused_by: null,
					reason_code: null
				}
			]
		};
		const db = fakeDojoDb(t);
		const out = await ingestMetrics(db as never, ALICE, [row()]);
		expect(out.rejected).toEqual([]);
		expect(out.accepted).toBe(1);
	});

	it('accepts the good rows in a mixed batch and reports only the bad ones', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, ALICE, [
			row(),
			row({ metric: 'churn', value: 3 }),
			row({ repo_key: 'github.com/other/secret' })
		]);
		expect(out.accepted).toBe(2);
		expect(out.rejected).toHaveLength(1);
		expect(db.tables.repository_metrics.rows).toHaveLength(2);
	});
});

// The 1000-row cap, on the gate that decides whether a push is stored.
//
// `PGRST_DB_MAX_ROWS=1000` (verified live). The permission read had no `.range()`
// and no `.in()`, so it fetched EVERY row of `all_my_repositories` for the
// caller and silently kept the first 1000. A user past that ceiling would have
// legitimate pushes refused as `not_permitted` — a denial naming the wrong
// reason, for a repository they are entitled to sync.
//
// The fix is not only paging: the ingest needs the repos IN THIS BATCH, so it
// filters by them. That makes the read proportional to the push rather than to
// the account, which is also why it stops being a ceiling at all.
describe('ingest permission read — bounded by the BATCH, not the account', () => {
	it('permits a repo that sits past the first 1000 rows of the view', async () => {
		const t = tables();
		// 1200 unrelated permitted repos ahead of the one being pushed.
		const filler = Array.from({ length: 1200 }, (_, i) => ({
			repository_id: `r-filler-${i}`,
			repo_key: `github.com/acme/filler-${i}`,
			principal_id: ALICE,
			sync_enabled: true,
			refused_by: null,
			reason_code: null
		}));
		t.all_my_repositories.rows = [...filler, ...t.all_my_repositories.rows];

		const db = fakeDojoDb(t);
		const out = await ingestMetrics(db as never, ALICE, [
			{
				repo_key: 'github.com/acme/api',
				metric: 'commits_per_day',
				scope: 'repo',
				grain: 'day',
				computed_on: '2026-08-29',
				value: 3
			}
		]);
		expect(out.accepted).toBe(1);
		expect(out.rejected).toEqual([]);
	});
});
