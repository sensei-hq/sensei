// The ACTIVATION write path — a tenant switching a catalogue metric off for one
// repository.
//
// The rule that carries the design: enabling DELETES the row. `absence = enabled`
// is the invariant the whole feature rests on (a metric added to the catalogue
// later must be on everywhere), so storing `enabled = true` would quietly make
// the table authoritative for "on" as well as "off" — and then a new metric would
// arrive off for every repository no row happens to mention.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { setMetricActivation } from './metric-activation';

const ALICE = 'p-alice';

/** The view row `all_my_repositories` yields — the authorization source. */
const viewRow = (over: Record<string, unknown> = {}) => ({
	repository_id: 'r1',
	tenant_id: 't-acme',
	repo_key: 'github.com/acme/api',
	tenant: 'organization/acme',
	principal_id: ALICE,
	configurable_by_me: true,
	...over
});

function tables(rows: Record<string, unknown>[] = [viewRow()]): Record<string, FakeTable> {
	return {
		all_my_repositories: { rows },
		metric_catalogue: { rows: [{ id: 'm-ftr', key: 'ftr' }, { id: 'm-churn', key: 'churn_rate' }] },
		metric_activations: {
			rows: [],
			uniques: [{ columns: ['tenant_id', 'repository_id', 'metric_id'] }]
		}
	};
}

beforeEach(() => resetFakeIds());

describe('setMetricActivation', () => {
	it('writes one row when a metric is switched OFF', async () => {
		const db = fakeDojoDb(tables());
		const out = await setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false);
		expect(out).toMatchObject({ repo_key: 'github.com/acme/api', metric: 'ftr', enabled: false });
		expect(db.tables.metric_activations.rows).toHaveLength(1);
		expect(db.tables.metric_activations.rows[0]).toMatchObject({
			tenant_id: 't-acme',
			repository_id: 'r1',
			metric_id: 'm-ftr',
			enabled: false
		});
	});

	it('DELETES the row when a metric is switched back on', async () => {
		// Not `enabled = true`. Absence is what "enabled" means, and a stored true
		// would make this table authoritative for on as well as off — so a metric
		// catalogued tomorrow would arrive OFF for every repository with no row.
		const db = fakeDojoDb(tables());
		await setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false);
		expect(db.tables.metric_activations.rows).toHaveLength(1);

		const out = await setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', true);
		expect(out).toMatchObject({ enabled: true });
		expect(db.tables.metric_activations.rows).toHaveLength(0);
	});

	it('is idempotent — switching off twice leaves one row', async () => {
		const db = fakeDojoDb(tables());
		await setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false);
		await setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false);
		expect(db.tables.metric_activations.rows).toHaveLength(1);
	});

	it('enabling something never disabled is a no-op, not an error', async () => {
		// The UI may send the current state back. Refusing would make a harmless
		// double-click look like a failure.
		const db = fakeDojoDb(tables());
		const out = await setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', true);
		expect(out).toMatchObject({ enabled: true });
		expect(db.tables.metric_activations.rows).toHaveLength(0);
	});

	it('refuses a repository the caller cannot configure', async () => {
		// `configurable_by_me` is the view's own verdict — admin of that tenant.
		// Deactivation stops a computation for everyone sharing the repository, so
		// it is not a per-member preference.
		const db = fakeDojoDb(tables([viewRow({ configurable_by_me: false })]));
		await expect(
			setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false)
		).rejects.toThrow(/may not/i);
		expect(db.tables.metric_activations.rows).toHaveLength(0);
	});

	it('refuses a repository that is not the callers at all', async () => {
		// The view is per-principal, so scoping IS the authorization — there is no
		// second membership check to forget.
		const db = fakeDojoDb(tables([viewRow({ principal_id: 'p-bob' })]));
		await expect(
			setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false)
		).rejects.toThrow(/no repository/i);
	});

	it('refuses a metric the catalogue does not have', async () => {
		// Storing an unknown key would write a deactivation that matches nothing
		// forever — invisible, and impossible to undo through this API.
		const db = fakeDojoDb(tables());
		await expect(
			setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'not_a_metric', false)
		).rejects.toThrow(/unknown metric/i);
	});

	it('propagates a read failure rather than reporting success', async () => {
		const db = fakeDojoDb({
			...tables(),
			all_my_repositories: { rows: [], error: { message: 'connection reset' } }
		});
		await expect(
			setMetricActivation(db as never, ALICE, 'github.com/acme/api', 'ftr', false)
		).rejects.toThrow(/connection reset/);
	});
});
