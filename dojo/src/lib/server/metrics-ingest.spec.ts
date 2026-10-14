// The receiving half of the metric push (daemon-sync.md §7).
//
// The WRITE and its gate are the database's: dojo.upsert_repository_metrics
// re-checks every row against the caller's own verdict (sync_enabled), takes the
// tenant from that verdict row, refuses the whole batch on a mismatch, and
// upserts on the seven-column key — proved against a real Postgres in
// database/tests/dojo/metrics_write_access.sql (#236), including re-push
// idempotency and commit_sha-distinct rows. What is left here is this side's
// half: resolving each row to a repository and a metric, rejecting by name what
// it can, and sending the rest in ONE call.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { ingestMetrics } from './metrics-ingest';

/** The caller's own verdict rows (what dojo.my_repositories() returns them): one
 *  permitted repository. Another tenant's repositories never appear — that is
 *  the function's pin, not this fixture's choice. */
function tables(mine: Record<string, unknown>[] = [MINE]): Record<string, FakeTable> {
	return {
		'rpc:my_repositories': { rows: mine },
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
		'rpc:upsert_repository_metrics': { rows: [] }
	};
}
const MINE = { repository_id: 'r-api', tenant_id: 't-acme', repo_key: 'github.com/acme/api', sync_enabled: true };

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

const sent = (db: ReturnType<typeof fakeDojoDb>) =>
	db.rpcCalls.filter((c) => c.fn === 'upsert_repository_metrics').map((c) => c.args.p_rows);

beforeEach(() => resetFakeIds());

describe('ingestMetrics', () => {
	it('sends a permitted row with the caller\'s repository and the catalogue metric — and no tenant', async () => {
		// The tenant is the database's to take from the caller's verdict row; a
		// tenant in the payload would be one more thing a client could forge.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, [row({ commit_sha: 'abc', props: { n: 1 } })]);
		expect(out).toEqual({ accepted: 1, rejected: [] });
		expect(sent(db)).toEqual([
			[
				{
					repository_id: 'r-api',
					metric_id: 'met-commits',
					scope: 'repo',
					commit_sha: 'abc',
					computed_on: '2026-08-27',
					grain: 'daily',
					value: 12,
					props: { n: 1 },
					source: 'measured'
				}
			]
		]);
	});

	it('cannot be used to discover which repo_keys other tenants have registered', async () => {
		// Another tenant's repository and a never-registered one answer the SAME:
		// distinguishing them is an existence oracle — signup is open, so any
		// stranger could probe which private repo_keys other organisations enrol.
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, [
			row({ repo_key: 'github.com/other/secret' }),
			row({ repo_key: 'github.com/never/registered' })
		]);
		expect(out.rejected.map((r) => r.reason)).toEqual(['unknown_repository', 'unknown_repository']);
		expect(sent(db)).toEqual([]);
	});

	it('refuses a repository the plan does not permit — the write RE-DECIDES', async () => {
		// The daemon's local push gate was removed (B2) on the premise that the dōjō
		// re-decides entitlement on every write. Named here; held in the database.
		const db = fakeDojoDb(tables([{ ...MINE, sync_enabled: false }]));
		const out = await ingestMetrics(db as never, [row()]);
		expect(out.rejected).toEqual([{ repo_key: 'github.com/acme/api', metric: 'commits_per_day', reason: 'not_permitted' }]);
		expect(sent(db)).toEqual([]);
	});

	it('refuses a repo_key in two of the caller\'s tenants per row, by name', async () => {
		// Picking one would file real metrics under an arbitrary governance
		// boundary; one ambiguous repository must not take the batch down.
		const db = fakeDojoDb(tables([MINE, { ...MINE, repository_id: 'r-api-2', tenant_id: 't-client' }]));
		const out = await ingestMetrics(db as never, [row()]);
		expect(out.rejected.map((r) => r.reason)).toEqual(['ambiguous']);
	});

	it('refuses an unknown metric key rather than inventing a metric id', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, [row({ metric: 'no-such-metric' })]);
		expect(out.rejected.map((r) => r.reason)).toEqual(['unknown_metric']);
	});

	// #240: a user-scoped row is the caller's — the database stamps the person
	// from the session, so the row carries only its scope, never a principal.
	it('files a user-scoped row with its scope and no principal of its own', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, [row({ scope: 'user' })]);
		expect(out).toEqual({ accepted: 1, rejected: [] });
		const [batch] = sent(db) as Array<Array<Record<string, unknown>>>;
		expect(batch[0].scope).toBe('user');
		expect(batch[0]).not.toHaveProperty('principal_id');
	});

	it('refuses a scope that is neither repo nor user, by name', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, [row({ scope: 'team' })]);
		expect(out.rejected.map((r) => r.reason)).toEqual(['unsupported_scope']);
		expect(sent(db)).toEqual([]);
	});

	it('accepts the good rows of a mixed batch in ONE write and reports only the bad', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestMetrics(db as never, [
			row({ commit_sha: 'a' }),
			row({ metric: 'nope' }),
			row({ commit_sha: 'b', metric: 'churn' })
		]);
		expect(out.accepted).toBe(2);
		expect(out.rejected.map((r) => r.reason)).toEqual(['unknown_metric']);
		expect(sent(db)).toHaveLength(1);
		expect((sent(db)[0] as unknown[]).length).toBe(2);
	});

	it('resolves and writes the batch in a bounded number of round trips, not per row', async () => {
		// The loop once issued ~4 PostgREST subrequests PER ROW inside one
		// Cloudflare Worker invocation: past the per-invocation subrequest cap and
		// the daemon's 30s timeout at 500 rows. Now: one verdict read, one catalogue
		// read, one write — however large the batch.
		const db = fakeDojoDb(tables());
		await ingestMetrics(db as never, Array.from({ length: 50 }, (_, i) => row({ commit_sha: `s${i}` })));
		expect(db.tables['rpc:my_repositories'].reads).toBe(1);
		expect(db.tables.metric_catalogue.reads).toBe(1);
		expect(sent(db)).toHaveLength(1);
	});

	it('writes nothing — and makes no write call — when every row is rejected', async () => {
		const db = fakeDojoDb(tables());
		await ingestMetrics(db as never, [row({ metric: 'nope' })]);
		expect(sent(db)).toEqual([]);
	});

	it('THROWS when the write fails rather than reporting the rows accepted', async () => {
		const t = tables();
		t['rpc:upsert_repository_metrics'].error = { code: '08006', message: 'connection reset' };
		const db = fakeDojoDb(t);
		await expect(ingestMetrics(db as never, [row()])).rejects.toThrow(/connection reset/);
	});
});

describe('ingest permission read — bounded by the BATCH, not the account', () => {
	it('permits a repo that sits past the first 1000 rows of the caller\'s repositories', async () => {
		// PostgREST caps an unbounded read at 1000 and says nothing about
		// truncating. The verdict read is filtered by the pushed keys, so it is
		// proportional to the push rather than to the account.
		const many = Array.from({ length: 1500 }, (_, i) => ({
			...MINE,
			repository_id: `r-${i}`,
			repo_key: `github.com/acme/r${i}`
		}));
		const db = fakeDojoDb(tables(many));
		const out = await ingestMetrics(db as never, [row({ repo_key: 'github.com/acme/r1400' })]);
		expect(out).toEqual({ accepted: 1, rejected: [] });
	});
});
