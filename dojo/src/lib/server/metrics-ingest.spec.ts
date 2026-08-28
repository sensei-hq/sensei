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
		metrics: {
			rows: [
				{ id: 'met-commits', key: 'commits_per_day' },
				{ id: 'met-churn', key: 'churn' }
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
		expect(out.rejected).toEqual([
			{ repo_key: 'github.com/other/secret', metric: 'commits_per_day', reason: 'not_permitted' }
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
